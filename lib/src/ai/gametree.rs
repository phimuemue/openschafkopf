use crate::game::SGameGeneric;
use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use itertools::Itertools;
use rand::{self, Rng};
use std::{borrow::Borrow, cmp::Ordering, fmt::{self, Debug}, fs, io::{BufWriter, Write}, ops::ControlFlow, convert::Infallible};
use super::{SPayoutStats, cardspartition::*};
use serde::Serialize;

pub trait TForEachSnapshot {
    type Output;
    type InfoFromParent;
    fn initial_info_from_parent() -> Self::InfoFromParent;
    fn final_output(&self, stichseq: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output;
    fn pruned_output(&self, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), rulestatecache: &SRuleStateCache) -> Option<Self::Output>;
    fn combine_outputs(
        &self,
        epi_card: EPlayerIndex,
        infofromparent: Self::InfoFromParent,
        itcard_allowed: impl Iterator<Item=ECard>,
        fn_card_to_output: impl FnMut(ECard, Self::InfoFromParent) -> Self::Output,
    ) -> Self::Output;
}

pub trait TSnapshotVisualizer<Output> {
    fn begin_snapshot(&mut self, ahand: &EnumMap<EPlayerIndex, SHand>, stichseq: &SStichSequence);
    fn end_snapshot(&mut self, output: &Output);
}


pub fn visualizer_factory<'rules, MinMaxStrategiesHK>(path: std::path::PathBuf, rules: &'rules SRules, epi: EPlayerIndex) -> impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, Option<ECard>) -> SForEachSnapshotHTMLVisualizer<'rules, MinMaxStrategiesHK> {
    unwrap!(std::fs::create_dir_all(&path));
    unwrap!(crate::game_analysis::generate_html_auxiliary_files(&path));
    move |i_ahand, ahand, ocard| {
        let path_abs = path.join(
            std::path::Path::new(&format!("{}", chrono::Local::now().format("%Y%m%d%H%M%S")))
                .join(format!("{}_{}{}.html",
                    i_ahand,
                    ahand.iter()
                        .map(|hand| hand.cards().iter().join(""))
                        .join("_"),
                    if let Some(card)=ocard {
                        format!("_{}", card)
                    } else {
                        "".to_owned()
                    }
                )),
        );
        unwrap!(std::fs::create_dir_all(unwrap!(path_abs.parent())));
        SForEachSnapshotHTMLVisualizer::new(
            unwrap!(std::fs::File::create(path_abs)),
            rules,
            epi
        )
    }
}

pub struct SForEachSnapshotHTMLVisualizer<'rules, MinMaxStrategiesHK> {
    file_output: BufWriter<fs::File>,
    rules: &'rules SRules,
    epi: EPlayerIndex,
    phantom: std::marker::PhantomData<MinMaxStrategiesHK>,
}
impl<'rules, MinMaxStrategiesHK> SForEachSnapshotHTMLVisualizer<'rules, MinMaxStrategiesHK> {
    fn new(file_output: fs::File, rules: &'rules SRules, epi: EPlayerIndex) -> Self {
        let mut foreachsnapshothtmlvisualizer = SForEachSnapshotHTMLVisualizer{file_output: BufWriter::with_capacity(16*1024*1024, file_output), rules, epi, phantom: std::marker::PhantomData};
        foreachsnapshothtmlvisualizer.write_all(
            b"<link rel=\"stylesheet\" type=\"text/css\" href=\"../css.css\">
            <style>
            input + label + ul {
                display: none;
            }
            input:checked + label + ul {
                display: block;
            }
            </style>"
        );
        foreachsnapshothtmlvisualizer
    }

    fn write_all(&mut self, buf: &[u8]) {
        if let Err(err) = self.file_output.write_all(buf) {
            error!("Error writing file: {}", err);
        }
    }
}

pub fn output_card(card: ECard, b_border: bool) -> String {
    format!(r#"<div class="card-image {}{}"></div>"#,
        card,
        if b_border {" border"} else {""},
    )
}

pub fn player_table<T: fmt::Display>(epi_self: EPlayerIndex, fn_per_player: impl Fn(EPlayerIndex)->Option<T>) -> String {
    let fn_per_player_internal = move |epi: EPlayerIndex| {
        fn_per_player(epi.wrapping_add(epi_self.to_usize()))
            .map_or("".to_string(), |t| t.to_string())
    };
    format!(
        "<table class=\"player-table\">
          <tr><td colspan=\"2\" style=\"text-align: center;\">{}</td></tr>
          <tr><td style=\"text-align: center;\">{}</td><td style=\"text-align: center;\">{}</td></tr>
          <tr><td colspan=\"2\" style=\"text-align: center;\">{}</td></tr>
        </table>\n",
        fn_per_player_internal(EPlayerIndex::EPI2),
        fn_per_player_internal(EPlayerIndex::EPI1),
        fn_per_player_internal(EPlayerIndex::EPI3),
        fn_per_player_internal(EPlayerIndex::EPI0),
    )
}

pub fn player_table_stichseq(epi_self: EPlayerIndex, stichseq: &SStichSequence, fn_output_card: &dyn Fn(ECard, bool/*b_highlight*/)->String) -> String {
    format!("{}", stichseq.visible_stichs().iter().map(|stich| {
        format!("<td>{}</td>", player_table(epi_self, |epi| {
            stich.get(epi).map(|card| {
                fn_output_card(*card, /*b_border*/epi==stich.first_playerindex())
            })
        }))
    }).format("\n"))
}

pub fn player_table_ahand(epi_self: EPlayerIndex, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &SRules, fn_border: impl Fn(ECard)->bool, fn_output_card: &dyn Fn(ECard, bool/*b_highlight*/)->String) -> String {
    format!(
        "<td>{}</td>\n",
        player_table(epi_self, |epi| {
            let mut veccard = ahand[epi].cards().clone();
            rules.sort_cards_first_trumpf_then_farbe(&mut veccard);
            Some(veccard.into_iter()
                .map(|card| fn_output_card(card, fn_border(card)))
                .join(""))
        }),
    )
}

impl<
    MinMaxStrategies: TMinMaxStrategies + TGenericArgs1<Arg0=EnumMap<EPlayerIndex, isize>>,
>
    TSnapshotVisualizer<MinMaxStrategies> for SForEachSnapshotHTMLVisualizer<'_, MinMaxStrategies::HigherKinded>
{
    fn begin_snapshot(&mut self, ahand: &EnumMap<EPlayerIndex, SHand>, stichseq: &SStichSequence) {
        let str_item_id = format!("{}{}",
            stichseq.count_played_cards(),
            rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).join(""), // we simply assume no collisions here TODO uuid
        );
        self.write_all(format!("<li><<input type=\"checkbox\" id=\"{}\" />>\n", str_item_id).as_bytes());
        self.write_all(format!("<label for=\"{}\">{} direct successors<table><tr>\n",
            str_item_id,
            "TODO", // slccard_allowed.len(),
        ).as_bytes());
        assert!(crate::ai::ahand_vecstich_card_count_is_compatible(ahand, stichseq));
        self.write_all(player_table_stichseq(self.epi, stichseq, &output_card).as_bytes());
        self.write_all(player_table_ahand(self.epi, ahand, self.rules, /*fn_border*/|_card| false, &output_card).as_bytes());
        self.write_all(b"</tr></table></label>\n");
        self.write_all(b"<ul>\n");
    }

    fn end_snapshot(&mut self, minmax: &MinMaxStrategies) {
        self.write_all(b"</ul>\n");
        self.write_all(b"</li>\n");
        self.write_all(player_table(self.epi, |epi| {
            Some(
                minmax.via_accessors().into_iter()
                    .map(|(_emmstrategy, an_payout)| an_payout[epi])
                    .join("/")
            )
        }).as_bytes());
    }
}

pub struct SNoVisualization;
impl SNoVisualization {
    pub fn factory() -> impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, Option<ECard>)->Self + std::marker::Sync {
        |_,_,_| Self
    }
}
impl<Output> TSnapshotVisualizer<Output> for SNoVisualization {
    fn begin_snapshot(&mut self, _ahand: &EnumMap<EPlayerIndex, SHand>, _stichseq: &SStichSequence) {}
    fn end_snapshot(&mut self, _output: &Output) {}
}

pub trait TFilterAllowedCards {
    type UnregisterStich;
    fn register_stich(&mut self, ahand: &mut EnumMap<EPlayerIndex, SHand>, stichseq: &mut SStichSequence) -> Self::UnregisterStich;
    fn unregister_stich(&mut self, unregisterstich: Self::UnregisterStich);
    fn filter_allowed_cards(&self, stichseq: &SStichSequence, veccard: &mut SHandVector);
    fn continue_with_filter(&self, _stichseq: &SStichSequence) -> bool {
        true
    }
}

#[derive(new)]
pub struct SFilterOnePerWinnerIndex<'rules> {
    oepi_unfiltered: Option<EPlayerIndex>,
    rules: &'rules SRules,
}

impl TFilterAllowedCards for SFilterOnePerWinnerIndex<'_> {
    type UnregisterStich = ();
    fn register_stich(&mut self, _ahand: &mut EnumMap<EPlayerIndex, SHand>, _stichseq: &mut SStichSequence) -> Self::UnregisterStich {
    }
    fn unregister_stich(&mut self, _unregisterstich: Self::UnregisterStich) {
    }
    fn filter_allowed_cards(&self, stichseq: &SStichSequence, veccard: &mut SHandVector) {
        let stich = stichseq.current_stich();
        if self.oepi_unfiltered!=verify!(stich.current_playerindex()) {
            // TODO all this could be done more efficiently (sampling the mutual true/false iterator items separately, avoiding allocating Vec)
            let mut mapepiveccard = EPlayerIndex::map_from_fn(|_epi| Vec::new());
            let mut stich = stich.clone();
            for &card in veccard.iter() {
                stich.push(card);
                mapepiveccard[self.rules.preliminary_winner_index(&stich)].push(card);
                stich.undo_most_recent();
            }
            *veccard = mapepiveccard.into_raw().into_iter()
                .filter_map(|veccard_epi| {
                    if_then_some!(!veccard_epi.is_empty(),
                        veccard_epi[rand::thread_rng().gen_range(0..veccard_epi.len())]
                    )
                })
                .collect()
        }
    }
    fn continue_with_filter(&self, _stichseq: &SStichSequence) -> bool {
        true
    }
}

pub struct SNoFilter;
impl SNoFilter {
    pub fn factory() -> impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>)->Self {
        |_,_| SNoFilter
    }
}
impl TFilterAllowedCards for SNoFilter {
    type UnregisterStich = ();
    fn register_stich(&mut self, _ahand: &mut EnumMap<EPlayerIndex, SHand>, _stichseq: &mut SStichSequence) -> Self::UnregisterStich {}
    fn unregister_stich(&mut self, _unregisterstich: Self::UnregisterStich) {}
    fn filter_allowed_cards(&self, _stichseq: &SStichSequence, _veccard: &mut SHandVector) {}
}

pub trait TSnapshotCache<T> { // TODO? could this be implemented via TForEachSnapshot
    fn get(&self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache) -> Option<T>;
    fn put(&mut self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache, t: &T); // borrow to avoid unconditional copy - TODO good idea?
    fn continue_with_cache(&self, _stichseq: &SStichSequence) -> bool {
        true
    }
}
pub struct SSnapshotCacheNone;
impl SSnapshotCacheNone {
    pub fn factory() -> impl Fn(&SRuleStateCacheFixed) -> Self {
        |_| Self
    }
}
impl<T> TSnapshotCache<T> for SSnapshotCacheNone {
    fn get(&self, _stichseq: &SStichSequence, _rulestatecache: &SRuleStateCache) -> Option<T> {
        None
    }
    fn put(&mut self, _stichseq: &SStichSequence, _rulestatecache: &SRuleStateCache, _t: &T) {
    }
}

impl<T> TSnapshotCache<T> for Box<dyn TSnapshotCache<T>> {
    fn get(&self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache) -> Option<T> {
        self.as_ref().get(stichseq, rulestatecache)
    }
    fn put(&mut self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache, t: &T) {
        self.as_mut().put(stichseq, rulestatecache, t)
    }
    fn continue_with_cache(&self, stichseq: &SStichSequence) -> bool {
        self.as_ref().continue_with_cache(stichseq)
    }
}

pub fn explore_snapshots<
    ForEachSnapshot,
    FilterAllowedCards: TFilterAllowedCards,
    OFilterAllowedCards: Into<Option<FilterAllowedCards>>,
    SnapshotCache: TSnapshotCache<ForEachSnapshot::Output>,
    OSnapshotCache: Into<Option<SnapshotCache>>,
>(
    (ahand, stichseq): (&mut EnumMap<EPlayerIndex, SHand>, &mut SStichSequence),
    rules: &SRules,
    fn_make_filter: &impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>)->OFilterAllowedCards,
    foreachsnapshot: &ForEachSnapshot,
    fn_snapshotcache: &impl Fn(&SRuleStateCacheFixed) -> OSnapshotCache,
    snapshotvisualizer: &mut impl TSnapshotVisualizer<ForEachSnapshot::Output>,
) -> ForEachSnapshot::Output 
    where
        ForEachSnapshot: TForEachSnapshot,
{
    macro_rules! forward{($func_filter_allowed_cards:expr, $snapshotcache:expr,) => {{
        let mut rulestatecache = SRuleStateCache::new(
            (ahand, stichseq),
            rules,
        );
        explore_snapshots_internal(
            (ahand, stichseq),
            rules,
            &mut rulestatecache,
            $func_filter_allowed_cards,
            foreachsnapshot,
            ForEachSnapshot::initial_info_from_parent(),
            $snapshotcache,
            snapshotvisualizer,
        )
    }}}
    cartesian_match!(forward,
        match (fn_make_filter(stichseq, ahand).into()) {
            Some(mut func_filter_allowed_cards) => (&mut func_filter_allowed_cards),
            None => (/*func_filter_allowed_cards*/&mut SNoFilter),
        },
        match(fn_snapshotcache(&SRuleStateCacheFixed::new(ahand, stichseq)).into()) {
            Some(mut snapshotcache) => (&mut snapshotcache),
            None => (&mut SSnapshotCacheNone),
        },
    )
}

fn explore_snapshots_internal<ForEachSnapshot>(
    (ahand, stichseq): (&mut EnumMap<EPlayerIndex, SHand>, &mut SStichSequence),
    rules: &SRules,
    rulestatecache: &mut SRuleStateCache,
    func_filter_allowed_cards: &mut impl TFilterAllowedCards,
    foreachsnapshot: &ForEachSnapshot,
    infofromparent: ForEachSnapshot::InfoFromParent,
    snapshotcache: &mut impl TSnapshotCache<ForEachSnapshot::Output>,
    snapshotvisualizer: &mut impl TSnapshotVisualizer<ForEachSnapshot::Output>,
) -> ForEachSnapshot::Output 
    where
        ForEachSnapshot: TForEachSnapshot,
{
    snapshotvisualizer.begin_snapshot(ahand, stichseq);
    let epi_current = unwrap!(stichseq.current_stich().current_playerindex());
    let output = if debug_verify_eq!(
        ahand[epi_current].cards().len() <= 1,
        ahand.iter().all(|hand| hand.cards().len() <= 1)
    ) {
        // TODO? use snapshotcache here?
        macro_rules! for_each_allowed_card{
            (($i_offset_0: expr, $($i_offset: expr,)*), $stichseq: expr) => {{
                let epi = epi_current.wrapping_add($i_offset_0);
                let card = debug_verify_eq!(
                    ahand[epi].cards(),
                    &rules.all_allowed_cards($stichseq, &ahand[epi])
                )[0];
                $stichseq.zugeben_and_restore/*zugeben_and_restore_with_hands not necessary; leave ahand untouched*/(
                    card,
                    rules,
                    |stichseq| {for_each_allowed_card!(($($i_offset,)*), stichseq)}
                )
            }};
            ((), $stichseq: expr) => {{
                let unregisterstich = rulestatecache.register_stich(
                    unwrap!($stichseq.last_completed_stich()),
                    $stichseq.current_stich().first_playerindex(),
                );
                let output = foreachsnapshot.final_output(
                    SStichSequenceGameFinished::new($stichseq),
                    rulestatecache,
                );
                rulestatecache.unregister_stich(unregisterstich);
                output
            }};
        }
        match stichseq.current_stich().size() {
            0 => for_each_allowed_card!((0, 1, 2, 3,), stichseq),
            1 => for_each_allowed_card!((0, 1, 2,), stichseq),
            2 => for_each_allowed_card!((0, 1,), stichseq),
            3 => for_each_allowed_card!((0,), stichseq),
            n_stich_size => {
                assert_eq!(n_stich_size, verify_eq!(EPlayerIndex::SIZE, 4));
                for_each_allowed_card!((), stichseq)
            },
        }
    } else {
        foreachsnapshot./*TODO fold pruned_output into combine_outputs*/pruned_output((ahand, stichseq), rulestatecache).unwrap_or_else(|| {
            let mut veccard_allowed = rules.all_allowed_cards(stichseq, &ahand[epi_current]);
            func_filter_allowed_cards.filter_allowed_cards(stichseq, &mut veccard_allowed);
            // TODO? use equivalent card optimization
            foreachsnapshot.combine_outputs(
                epi_current,
                infofromparent,
                veccard_allowed.into_iter(),
                /*fn_card_to_output*/|card, infofromparent| {
                    stichseq.zugeben_and_restore_with_hands(ahand, epi_current, card, rules, |ahand, stichseq| {
                        macro_rules! next_step {($func_filter_allowed_cards:expr, $snapshotcache:expr) => {explore_snapshots_internal(
                            (ahand, stichseq),
                            rules,
                            rulestatecache,
                            $func_filter_allowed_cards,
                            foreachsnapshot,
                            infofromparent,
                            $snapshotcache,
                            snapshotvisualizer,
                        )}}
                        if stichseq.current_stich().is_empty() {
                            let unregisterstich_cache = rulestatecache.register_stich(
                                unwrap!(stichseq.last_completed_stich()),
                                stichseq.current_stich().first_playerindex(),
                            );
                            let output = if let Some(output) = snapshotcache.get(stichseq, rulestatecache) {
                                output
                            } else {
                                macro_rules! fwd{(($fn_before:expr, $fn_after:expr, $func_filter_allowed_cards:expr), ($snapshotcache:expr),) => {{
                                    let input_to_after = $fn_before;
                                    let output = next_step!($func_filter_allowed_cards, $snapshotcache);
                                    #[allow(clippy::redundant_closure_call)]
                                    $fn_after(input_to_after);
                                    output
                                }}}
                                let output = cartesian_match!(fwd,
                                    match (func_filter_allowed_cards.continue_with_filter(stichseq)) {
                                        true => (
                                            func_filter_allowed_cards.register_stich(ahand, stichseq),
                                            |unregisterstich_filter| func_filter_allowed_cards.unregister_stich(unregisterstich_filter),
                                            func_filter_allowed_cards
                                        ),
                                        false => ((), |()|(), &mut SNoFilter),
                                    },
                                    match (snapshotcache.continue_with_cache(stichseq)) {
                                        true => (snapshotcache),
                                        false => (&mut SSnapshotCacheNone),
                                    },
                                );
                                snapshotcache.put(stichseq, rulestatecache, &output);
                                output
                            };
                            rulestatecache.unregister_stich(unregisterstich_cache);
                            output
                        } else {
                            next_step!(func_filter_allowed_cards, snapshotcache)
                        }
                    })
                },
            )
        })
    };
    snapshotvisualizer.end_snapshot(&output);
    output
}

#[derive(Clone)]
pub struct SMinReachablePayoutBase<'rules, Pruner, MinMaxStrategiesHK, AlphaBetaPruner> {
    pub(super) rules: &'rules SRules,
    pub(super) epi: EPlayerIndex,
    expensifiers: SExpensifiers, // TODO could this borrow?
    alphabetapruner: AlphaBetaPruner,
    phantom: std::marker::PhantomData<(Pruner, MinMaxStrategiesHK)>,
}
impl<'rules, Pruner, MinMaxStrategiesHK, AlphaBetaPruner> SMinReachablePayoutBase<'rules, Pruner, MinMaxStrategiesHK, AlphaBetaPruner> {
    pub fn new_with_pruner(rules: &'rules SRules, epi: EPlayerIndex, expensifiers: SExpensifiers, alphabetapruner: AlphaBetaPruner) -> Self {
        Self {
            rules,
            epi,
            expensifiers,
            alphabetapruner,
            phantom: Default::default(),
        }
    }
    pub fn new(rules: &'rules SRules, epi: EPlayerIndex, expensifiers: SExpensifiers) -> Self
        where
            AlphaBetaPruner: Default,
    {
        Self::new_with_pruner(rules, epi, expensifiers, /*alphabetapruner*/Default::default())
    }
    pub fn new_from_game<Ruleset>(game: &'rules SGameGeneric<Ruleset, (), ()>) -> Self
        where
            AlphaBetaPruner: Default,
    {
        Self::new(
            &game.rules,
            unwrap!(game.current_playable_stich().current_playerindex()),
            game.expensifiers.clone(),
        )
    }
}

#[derive(Clone, Copy)]
pub enum EMinMaxStrategy {
    MinMin,
    Min,
    SelfishMin,
    SelfishMax,
    Max,
}

macro_rules! impl_perminmaxstrategy{(
    $struct:ident {$($emmstrategy:ident $ident_strategy:ident,)*}
    $struct_higher_kinded:ident
    [$($ident_strategy_cmp:ident)+]
    $ident_strategy_maxmin_for_pruner:ident
) => {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct $struct<T> {
        // TODO? pub a good idea?
        $(pub $ident_strategy: $emmstrategy<T>,)*
    }

    #[derive(Serialize)] // TODO this should not be needed
    pub struct $struct_higher_kinded;
    impl TMinMaxStrategiesHigherKinded for $struct_higher_kinded {
        type Type<R> = $struct<R>;
    }

    impl<T> TGenericArgs1 for $struct<T> {
        type Arg0 = T;
    }

    impl<T> TMinMaxStrategies for $struct<T> {
        type HigherKinded = $struct_higher_kinded;
        fn new(t: T) -> Self
            where T: Clone
        {
            Self {
                $($ident_strategy: $emmstrategy::new(t.clone()),)* // TODO can we avoid one clone call?
            }
        }

        fn map<R>(&self, mut f: impl FnMut(&T)->R) -> $struct<R> {
            let Self{
                $(ref $ident_strategy,)*
            } = self;
            $struct{
                $($ident_strategy: $emmstrategy::new(f(&$ident_strategy.0)),)*
            }
        }

        fn modify_with_other<T1>(
            &mut self,
            other: &$struct<T1>, 
            mut fn_modify_element: impl FnMut(&mut T, &T1),
        ) {
            let $struct{
                $($ident_strategy,)*
            } = other;
            $(fn_modify_element(&mut self.$ident_strategy.0, &$ident_strategy.0);)*
        }

        fn via_accessors(&self) -> Vec<(EMinMaxStrategy, &T)> {
            [$((EMinMaxStrategy::$emmstrategy, &self.$ident_strategy.0),)*].into()
        }

        fn accessors() -> &'static [(EMinMaxStrategy, fn(&Self)->&T)] { // TODO is there a better alternative?
            use EMinMaxStrategy::*;
            &[
                $(($emmstrategy, (|slf: &Self| &slf.$ident_strategy.0) as fn(&Self) -> &T),)*
            ]
        }
        fn compare_canonical<PayoutStatsPayload: Ord+Debug+Copy>(&self, other: &Self, fn_loss_or_win: impl Fn(isize, PayoutStatsPayload)->std::cmp::Ordering) -> std::cmp::Ordering where T: Borrow<SPayoutStats<PayoutStatsPayload>> {
            use std::cmp::Ordering::*;
            fn compare_fractions((numerator_lhs, denominator_lhs): (u128, u128), (numerator_rhs, denominator_rhs): (u128, u128)) -> std::cmp::Ordering {
                u128::cmp(&(numerator_lhs * denominator_rhs), &(denominator_lhs * numerator_rhs))
            }
            Equal
                $(.then_with(|| {
                    let payoutstats_lhs = self.$ident_strategy_cmp.0.borrow();
                    let payoutstats_rhs = other.$ident_strategy_cmp.0.borrow();
                    let mapordn_lhs = payoutstats_lhs.counts(&fn_loss_or_win).map_into(|n| n.as_num::<u128>());
                    let mapordn_rhs = payoutstats_rhs.counts(&fn_loss_or_win).map_into(|n| n.as_num::<u128>());
                    let compare_winning_probability_internal = |ord| compare_fractions(
                        (mapordn_lhs[ord], mapordn_lhs.iter().sum()),
                        (mapordn_rhs[ord], mapordn_rhs.iter().sum()),
                    );
                    compare_winning_probability_internal(Greater)
                        .then_with(|| compare_winning_probability_internal(Less).reverse())
                        .then_with(|| match unwrap!(payoutstats_lhs.avg().partial_cmp(&payoutstats_rhs.avg())) {
                            Greater => Greater,
                            Less => Less,
                            Equal => payoutstats_lhs.max().cmp(&payoutstats_rhs.max()),
                        })
                }))+
        }
    }
    impl TMinMaxStrategiesInternal<$struct_higher_kinded> for $struct<EnumMap<EPlayerIndex, isize>> {
        fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex) {
            let $struct{
                $($ident_strategy,)*
            } = other;
            $(self.$ident_strategy.assign_minmax_self($ident_strategy, epi_self);)*
        }

        fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, epi_card: EPlayerIndex) {
            let $struct{
                $($ident_strategy,)*
            } = other;
            $(self.$ident_strategy.assign_minmax_other($ident_strategy, epi_self, epi_card);)*
        }

        fn maxmin_for_pruner(&self, epi_self: EPlayerIndex) -> isize {
            self.$ident_strategy_maxmin_for_pruner.0[epi_self]
        }
    }
}}

// Field nomenclature: self-strategy, followed by others-strategy
impl_perminmaxstrategy!(
    SPerMinMaxStrategy {
        MinMin minmin,
        Min maxmin,
        SelfishMin maxselfishmin,
        SelfishMax maxselfishmax,
        Max maxmax,
    }
    SPerMinMaxStrategyHigherKinded
    [maxselfishmin maxselfishmax maxmin maxmax minmin]
    maxmin
);
impl_perminmaxstrategy!(
    SMaxMinMaxSelfishMin {
        Min maxmin,
        SelfishMin maxselfishmin,
    }
    SMaxMinMaxSelfishMinHigherKinded
    [maxmin maxselfishmin]
    maxmin
);
impl_perminmaxstrategy!(
    SMaxMinStrategy {
        Min maxmin,
    }
    SMaxMinStrategyHigherKinded
    [maxmin]
    maxmin
);
impl_perminmaxstrategy!(
    SMaxSelfishMinStrategy {
        SelfishMin maxselfishmin,
    }
    SMaxSelfishMinStrategyHigherKinded
    [maxselfishmin]
    maxselfishmin
);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MinMin<T>(pub T); // TODO this does not require a whole EnumMap - entry for epi_self is sufficient.
impl<T> MinMin<T> {
    fn new(t: T) -> Self {
        Self(t)
    }
}
impl MinMin<EnumMap<EPlayerIndex, isize>> {
    fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex) {
        assign_min_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
    fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, _epi_card: EPlayerIndex) {
        assign_min_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Min<T>(pub T);
impl<T> Min<T> {
    fn new(t: T) -> Self {
        Self(t)
    }
}
impl Min<EnumMap<EPlayerIndex, isize>> {
    fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex) {
        assign_max_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
    fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, _epi_card: EPlayerIndex) {
        assign_min_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelfishMin<T>(pub T);
impl<T> SelfishMin<T> {
    fn new(t: T) -> Self {
        Self(t)
    }
}
impl SelfishMin<EnumMap<EPlayerIndex, isize>> {
    fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex) {
        assign_max_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
    fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, epi_card: EPlayerIndex) {
        assign_better(&mut self.0, other.0, |an_payout_lhs, an_payout_rhs| {
            match an_payout_lhs[epi_card].cmp(&an_payout_rhs[epi_card]) {
                Ordering::Less => false,
                Ordering::Equal => an_payout_lhs[epi_self] < an_payout_rhs[epi_self],
                Ordering::Greater => true,
            }
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SelfishMax<T>(pub T);
impl<T> SelfishMax<T> {
    fn new(t: T) -> Self {
        Self(t)
    }
}
impl SelfishMax<EnumMap<EPlayerIndex, isize>> {
    fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex) {
        assign_max_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
    fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, epi_card: EPlayerIndex) {
        assign_better(&mut self.0, other.0, |an_payout_lhs, an_payout_rhs| {
            match an_payout_lhs[epi_card].cmp(&an_payout_rhs[epi_card]) {
                Ordering::Less => false,
                Ordering::Equal => an_payout_lhs[epi_self] > an_payout_rhs[epi_self],
                Ordering::Greater => true,
            }
        });
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Max<T>(pub T);
impl<T> Max<T> {
    fn new(t: T) -> Self {
        Self(t)
    }
}
impl Max<EnumMap<EPlayerIndex, isize>> {
    fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex) {
        assign_max_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
    fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, _epi_card: EPlayerIndex) {
        assign_max_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
}

pub trait TMinMaxStrategiesHigherKinded : Sized + 'static + Sync {
    type Type<R>: TMinMaxStrategies<HigherKinded=Self> + TGenericArgs1<Arg0=R>;
}

pub trait TGenericArgs1 {
    type Arg0;
}

pub trait TMinMaxStrategies : TGenericArgs1 {
    type HigherKinded: TMinMaxStrategiesHigherKinded;
    fn new(t: <Self as TGenericArgs1>::Arg0) -> Self where <Self as TGenericArgs1>::Arg0: Clone;
    fn map<R>(&self, f: impl FnMut(&<Self as TGenericArgs1>::Arg0)->R) -> <Self::HigherKinded as TMinMaxStrategiesHigherKinded>::Type<R>;
    fn modify_with_other<T1>(
        &mut self,
        other: &<Self::HigherKinded as TMinMaxStrategiesHigherKinded>::Type<T1>, 
        fn_modify_element: impl FnMut(&mut <Self as TGenericArgs1>::Arg0, &T1),
    );
    fn via_accessors(&self) -> Vec<(EMinMaxStrategy, &<Self as TGenericArgs1>::Arg0)>
        where
            <Self as TGenericArgs1>::Arg0: 'static; // TODO why is this needed?
    fn accessors() -> &'static [(EMinMaxStrategy, fn(&Self)->&<Self as TGenericArgs1>::Arg0)]; // TODO is there a better alternative?
    fn compare_canonical<PayoutStatsPayload: Ord+Debug+Copy>(&self, other: &Self, fn_loss_or_win: impl Fn(isize, PayoutStatsPayload)->std::cmp::Ordering) -> std::cmp::Ordering where <Self as TGenericArgs1>::Arg0: Borrow<SPayoutStats<PayoutStatsPayload>>;
}
pub trait TMinMaxStrategiesInternal<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded> :
    TMinMaxStrategies<HigherKinded=MinMaxStrategiesHK>
    + TGenericArgs1<Arg0=EnumMap<EPlayerIndex, isize>>
{
    fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex);
    fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, epi_card: EPlayerIndex);
    fn maxmin_for_pruner(&self, epi_self: EPlayerIndex) -> isize;
}

pub trait TAlphaBetaPruner {
    type InfoFromParent: Clone;
    fn initial_info_from_parent() -> Self::InfoFromParent;
    type BreakType;
    fn is_prunable_self<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded, MinMaxStrategies: TMinMaxStrategiesInternal<MinMaxStrategiesHK>>(&self, minmax: &MinMaxStrategies, infofromparent: Self::InfoFromParent, epi_self: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent>;
    fn is_prunable_other<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded, MinMaxStrategies: TMinMaxStrategiesInternal<MinMaxStrategiesHK>>(&self, minmax: &MinMaxStrategies, infofromparent: Self::InfoFromParent, epi_self: EPlayerIndex, epi_card: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent>;
}

#[derive(Default)]
pub struct SAlphaBetaPrunerNone;
impl TAlphaBetaPruner for SAlphaBetaPrunerNone {
    type InfoFromParent = ();
    fn initial_info_from_parent() -> Self::InfoFromParent {
    }
    type BreakType = Infallible;
    fn is_prunable_self<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded, MinMaxStrategies: TMinMaxStrategiesInternal<MinMaxStrategiesHK>>(&self, _minmax: &MinMaxStrategies, _infofromparent: Self::InfoFromParent, _epi_self: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent> {
        ControlFlow::Continue(())
    }
    fn is_prunable_other<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded, MinMaxStrategies: TMinMaxStrategiesInternal<MinMaxStrategiesHK>>(&self, _minmax: &MinMaxStrategies, _infofromparent: Self::InfoFromParent, _epi_self: EPlayerIndex, _epi_card: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent> {
        ControlFlow::Continue(())
    }
}

#[derive(new)]
pub struct SAlphaBetaPruner {
    mapepilohi: EnumMap<EPlayerIndex, ELoHi>, // Does epi minimize (Lo) or maximize (Hi)?
}
impl TAlphaBetaPruner for SAlphaBetaPruner {
    type InfoFromParent = EnumMap<ELoHi, isize>;
    fn initial_info_from_parent() -> Self::InfoFromParent {
        ELoHi::map_from_raw([isize::MIN, isize::MAX])
    }
    type BreakType = ();
    fn is_prunable_self<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded, MinMaxStrategies: TMinMaxStrategiesInternal<MinMaxStrategiesHK>>(&self, minmax: &MinMaxStrategies, mut infofromparent: Self::InfoFromParent, epi_self: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent> {
        assert_eq!(self.mapepilohi[epi_self], ELoHi::Hi);
        let n_payout_for_pruner = minmax.maxmin_for_pruner(epi_self);
        if n_payout_for_pruner >= infofromparent[ELoHi::Hi] {
            // I'm maximizing myself, but if my parent will minimize against what's already in there, I do not need to investigate any further
            ControlFlow::Break(())
        } else {
            assign_max(&mut infofromparent[ELoHi::Lo], n_payout_for_pruner);
            ControlFlow::Continue(infofromparent)
        }
    }
    fn is_prunable_other<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded, MinMaxStrategies: TMinMaxStrategiesInternal<MinMaxStrategiesHK>>(&self, minmax: &MinMaxStrategies, mut infofromparent: Self::InfoFromParent, epi_self: EPlayerIndex, epi_card: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent> {
        match self.mapepilohi[epi_card] {
            ELoHi::Hi => {
                self.is_prunable_self(
                    minmax,
                    infofromparent,
                    epi_self, // TODO we should asser that we could also pass epi_card here.
                )
            },
            ELoHi::Lo => {
                let n_payout_for_pruner = minmax.maxmin_for_pruner(epi_self);
                if n_payout_for_pruner <= infofromparent[ELoHi::Lo] {
                    // I'm minimizing myself, but if my parent will maximize against what's already in there, I do not need to investigate any further
                    ControlFlow::Break(())
                } else {
                    assign_min(&mut infofromparent[ELoHi::Hi], n_payout_for_pruner);
                    ControlFlow::Continue(infofromparent)
                }
            }
        }
    }
}

impl<Pruner: TPruner, MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded, AlphaBetaPruner: TAlphaBetaPruner> TForEachSnapshot for SMinReachablePayoutBase<'_, Pruner, MinMaxStrategiesHK, AlphaBetaPruner>
    where
        MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: TMinMaxStrategiesInternal<MinMaxStrategiesHK>,
{
    type Output = MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>;
    type InfoFromParent = AlphaBetaPruner::InfoFromParent;

    fn initial_info_from_parent() -> Self::InfoFromParent {
        AlphaBetaPruner::initial_info_from_parent()
    }

    fn final_output(&self, stichseq: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output {
        Self::Output::new(self.rules.payout(
            stichseq,
            &self.expensifiers,
            rulestatecache,
            dbg_argument!(/*b_test_points_as_payout*/true),
        ))
    }

    fn pruned_output(&self, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), rulestatecache: &SRuleStateCache) -> Option<Self::Output> {
        Pruner::pruned_output(self, tplahandstichseq, rulestatecache)
    }

    fn combine_outputs(
        &self,
        epi_card: EPlayerIndex,
        mut infofromparent: Self::InfoFromParent,
        mut itcard_allowed: impl Iterator<Item=ECard>,
        mut fn_card_to_output: impl FnMut(ECard, Self::InfoFromParent) -> Self::Output,
    ) -> Self::Output {
        let mut minmax_acc = fn_card_to_output(
            unwrap!(itcard_allowed.next()),
            infofromparent.clone(),
        );
        if self.epi==epi_card {
            for card_allowed in itcard_allowed {
                match self.alphabetapruner.is_prunable_self(&minmax_acc, infofromparent.clone(), self.epi) {
                    ControlFlow::Break(_) => {
                        break;
                    },
                    ControlFlow::Continue(infofromparent_new) => {
                        infofromparent = infofromparent_new;
                        minmax_acc.assign_minmax_self(fn_card_to_output(card_allowed, infofromparent.clone()), self.epi);
                    },
                }
            }
        } else {
            // other players may play inconveniently for epi_stich
            for card_allowed in itcard_allowed {
                match self.alphabetapruner.is_prunable_other(&minmax_acc, infofromparent.clone(), self.epi, epi_card) {
                    ControlFlow::Break(_) => {
                        break;
                    },
                    ControlFlow::Continue(infofromparent_new) => {
                        infofromparent = infofromparent_new;
                        minmax_acc.assign_minmax_other(fn_card_to_output(card_allowed, infofromparent.clone()), self.epi, epi_card);
                    },
                }
            }
        }
        minmax_acc
    }
}

pub type SGenericMinReachablePayout<'rules, MinMaxStrategiesHK, AlphaBetaPruner> = SMinReachablePayoutBase<'rules, SPrunerNothing, MinMaxStrategiesHK, AlphaBetaPruner>;
pub type SMinReachablePayout<'rules> = SMinReachablePayoutBase<'rules, SPrunerNothing, SPerMinMaxStrategyHigherKinded, SAlphaBetaPrunerNone>;
pub type SGenericMinReachablePayoutLowerBoundViaHint<'rules, MinMaxStrategiesHK, AlphaBetaPruner> = SMinReachablePayoutBase<'rules, SPrunerViaHint, MinMaxStrategiesHK, AlphaBetaPruner>;
pub type SMinReachablePayoutLowerBoundViaHint<'rules> = SMinReachablePayoutBase<'rules, SPrunerViaHint, SPerMinMaxStrategyHigherKinded, SAlphaBetaPrunerNone>;

pub trait TPruner : Sized {
    fn pruned_output<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded, AlphaBetaPruner>(params: &SMinReachablePayoutBase<'_, Self, MinMaxStrategiesHK, AlphaBetaPruner>, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), rulestatecache: &SRuleStateCache) -> Option<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>>;
}

pub struct SPrunerNothing;
impl TPruner for SPrunerNothing {
    fn pruned_output<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded, AlphaBetaPruner>(_params: &SMinReachablePayoutBase<'_, Self, MinMaxStrategiesHK, AlphaBetaPruner>, _tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), _rulestatecache: &SRuleStateCache) -> Option<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>> {
        None
    }
}

pub struct SPrunerViaHint;
impl TPruner for SPrunerViaHint {
    fn pruned_output<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded, AlphaBetaPruner>(params: &SMinReachablePayoutBase<'_, Self, MinMaxStrategiesHK, AlphaBetaPruner>, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), rulestatecache: &SRuleStateCache) -> Option<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>> {
        let mapepion_payout = params.rules.payouthints(tplahandstichseq, &params.expensifiers, rulestatecache)
            .map(|intvlon_payout| {
                intvlon_payout[ELoHi::Lo].filter(|n_payout| 0<*n_payout)
                    .or_else(|| intvlon_payout[ELoHi::Hi].filter(|n_payout| *n_payout<0))
            });
        if_then_some!(
            mapepion_payout.iter().all(Option::is_some),
            MinMaxStrategiesHK::Type::<EnumMap<EPlayerIndex, isize>>::new(mapepion_payout.map(|opayout| unwrap!(opayout)))
        )
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct SFilterEquivalentCards {
    cardspartition: SCardsPartition,
    n_until_stichseq_len: usize,
}

impl SFilterEquivalentCards {
    fn internal_register_stich(&mut self, stich: SFullStich<&SStich>) -> <Self as TFilterAllowedCards>::UnregisterStich {
        #[cfg(debug_assertions)] let self_original = self.clone();
        // TODO Can we use EPlayerIndex::map_from_fn? (Unsure about evaluation order.)
        let mut remove_from_chain = |epi| self.cardspartition.remove_from_chain(stich[epi]);
        let removed_0 = remove_from_chain(EPlayerIndex::EPI0);
        let removed_1 = remove_from_chain(EPlayerIndex::EPI1);
        let removed_2 = remove_from_chain(EPlayerIndex::EPI2);
        let removed_3 = remove_from_chain(EPlayerIndex::EPI3);
        let unregisterstich = EPlayerIndex::map_from_raw([removed_0, removed_1, removed_2, removed_3]);
        #[cfg(debug_assertions)] {
            let mut self_clone = self.clone();
            self_clone.unregister_stich(unregisterstich.clone());
            assert_eq!(self_original, self_clone);
        }
        unregisterstich
    }
}

impl TFilterAllowedCards for SFilterEquivalentCards {
    type UnregisterStich = EnumMap<EPlayerIndex, SRemoved>;
    fn register_stich(&mut self, _ahand: &mut EnumMap<EPlayerIndex, SHand>, stichseq: &mut SStichSequence) -> Self::UnregisterStich {
        debug_assert!(stichseq.current_stich().is_empty());
        self.internal_register_stich(unwrap!(stichseq.last_completed_stich()))
    }
    fn unregister_stich(&mut self, unregisterstich: Self::UnregisterStich) {
        for removed in unregisterstich.into_raw().into_iter().rev() {
            self.cardspartition.readd(removed);
        }
    }
    fn filter_allowed_cards(&self, _stichseq: &SStichSequence, veccard: &mut SHandVector) {
        // TODO assert that we actually have the correct cardspartition
        // for (_epi, card) in stichseq.completed_cards() {
        //     cardspartition.remove_from_chain(*card);
        // }
        // assert_eq!(cardspartition, self.cardspartition);
        // 
        // example: First stich was SO GU H8 E7
        // then, we have chains EO-GO-HO EU-HU-SU H9-H7, E9-E8, G9-G8-G7, S9-S8-S7
        // => If some cards from veccard form a contiguous sequence within cardspartition, we only need to propagate one of the cards.
        let mut veccard_out = Vec::new(); // TODO use SHandVector
        for card_allowed in veccard.iter() {
            let card_first_in_chain = self.cardspartition
                .prev_while_contained(*card_allowed, veccard);
            veccard_out.push(card_first_in_chain);
        }
        veccard_out.sort_unstable_by_key(|card| card.to_usize());
        veccard_out.dedup();
        if veccard.len()!=veccard_out.len() {
            // println!("Found equivalent cards:\n stichseq: {}\n veccard_in : {:?}\n veccard_out: {:?}",
            //     stichseq,
            //     veccard,
            //     veccard_out,
            // )
        } else {
            #[cfg(debug_assertions)] {
                veccard.sort_unstable_by_key(|card| card.to_usize());
                debug_assert_eq!(veccard as &[ECard], &veccard_out as &[ECard]);
            }
        }
        *veccard = unwrap!((&veccard_out as &[ECard]).try_into());
    }
    fn continue_with_filter(&self, stichseq: &SStichSequence) -> bool {
        stichseq.completed_stichs().len()<=self.n_until_stichseq_len
    }
}

pub fn equivalent_cards_filter(
    n_until_stichseq_len: usize,
    cardspartition: SCardsPartition,
) -> impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>)->SFilterEquivalentCards {
    move |stichseq, _ahand| {
        let mut filterequivalentcards = SFilterEquivalentCards {
            cardspartition: cardspartition.clone(),
            n_until_stichseq_len,
        };
        for stich in stichseq.completed_stichs() {
            filterequivalentcards.internal_register_stich(SFullStich::new(stich));
        }
        filterequivalentcards
    }
}
