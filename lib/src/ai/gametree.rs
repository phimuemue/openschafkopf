use crate::game::SGameGeneric;
use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use itertools::Itertools;
use rand::{self, Rng};
use std::{borrow::Borrow, cmp::Ordering, fmt::Debug, fs, io::{BufWriter, Write}, ops::ControlFlow, convert::Infallible};
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


pub fn visualizer_factory<'rules>(path: std::path::PathBuf, rules: &'rules SRules, epi: EPlayerIndex) -> impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, Option<ECard>) -> SForEachSnapshotHTMLVisualizer<'rules> {
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
                        format!("_{card}")
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

pub struct SForEachSnapshotHTMLVisualizer<'rules> {
    file_output: BufWriter<fs::File>,
    rules: &'rules SRules,
    epi: EPlayerIndex,
}
impl<'rules> SForEachSnapshotHTMLVisualizer<'rules> {
    fn new(file_output: fs::File, rules: &'rules SRules, epi: EPlayerIndex) -> Self {
        let mut foreachsnapshothtmlvisualizer = SForEachSnapshotHTMLVisualizer{file_output: BufWriter::with_capacity(16*1024*1024, file_output), rules, epi};
        foreachsnapshothtmlvisualizer.write_all(
            &"<link rel=\"stylesheet\" type=\"text/css\" href=\"../css.css\">
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

    fn write_all(&mut self, t: &impl std::fmt::Display) {
        let _ = verify_or_error!(write!(self.file_output, "{}", t));
    }
}

pub fn output_card(card: ECard, b_border: bool) -> html_generator::HtmlElement<impl html_generator::AttributeOrChild> {
    use html_generator::*;
    div(class(format!("card-image {}{}",
        card,
        if b_border {" border"} else {""},
    )))
}

pub fn player_table<HtmlAttributesAndChildrenPerPlayer: html_generator::AttributeOrChild>(epi_self: EPlayerIndex, fn_per_player: impl Fn(EPlayerIndex)->HtmlAttributesAndChildrenPerPlayer) -> html_generator::HtmlElement<impl html_generator::AttributeOrChild> {
    use html_generator::*;
    let table_cell = |ostr_colspan: Option<&'static str>, epi: EPlayerIndex| {
        td((
            (ostr_colspan.map(colspan), attributes::style("text-align: center;")),
            fn_per_player(epi.wrapping_add(epi_self.to_usize())),
        ))
    };
    table((
        class("player-table"),
        tr(table_cell(/*ostr_colspan*/Some("2"), EPlayerIndex::EPI2)),
        tr((
            table_cell(/*ostr_colspan*/None, EPlayerIndex::EPI1),
            table_cell(/*ostr_colspan*/None, EPlayerIndex::EPI3),
        )),
        tr(table_cell(/*ostr_colspan*/Some("2"), EPlayerIndex::EPI0)),
    ))
}

pub fn player_table_stichseq<'a, HtmlAttributeOrChildCard: html_generator::AttributeOrChild>(epi_self: EPlayerIndex, stichseq: &'a SStichSequence, fn_output_card: &'a dyn Fn(ECard, bool/*b_highlight*/)->HtmlAttributeOrChildCard) -> impl html_generator::AttributeOrChild + use<'a, HtmlAttributeOrChildCard> {
    use html_generator::*;
    html_iter(stichseq.visible_stichs().iter().map(move |stich| {
        td(player_table(epi_self, |epi| {
            stich.get(epi).map(|card| {
                fn_output_card(*card, /*b_border*/epi==stich.first_playerindex())
            })
        }))
    }))
}

pub fn player_table_ahand<'a, HtmlAttributeOrChildCard: html_generator::AttributeOrChild>(epi_self: EPlayerIndex, ahand: &'a EnumMap<EPlayerIndex, SHand>, rules: &'a SRules, fn_border: impl Fn(ECard)->bool + Clone, fn_output_card: &'a dyn Fn(ECard, bool/*b_highlight*/)->HtmlAttributeOrChildCard) -> html_generator::HtmlElement<impl html_generator::AttributeOrChild> {
    use html_generator::*;
    td(player_table(epi_self, move |epi| {
        let mut veccard = ahand[epi].cards().clone();
        rules.sort_cards(&mut veccard);
        let fn_border = fn_border.clone(); // TODO really required?
        html_generator::html_iter(veccard.into_iter()
            .map(move |card| fn_output_card(card, fn_border(card)))
        )
    }))
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
                        veccard_epi[rand::rng().random_range(0..veccard_epi.len())]
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
pub struct SMinReachablePayoutBase<'rules, Pruner, TplStrategies, AlphaBetaPruner> {
    pub(super) rules: &'rules SRules,
    pub(super) epi: EPlayerIndex,
    expensifiers: SExpensifiers, // TODO could this borrow?
    alphabetapruner: AlphaBetaPruner,
    phantom: std::marker::PhantomData<(Pruner, TplStrategies)>,
}
impl<'rules, Pruner, TplStrategies, AlphaBetaPruner> SMinReachablePayoutBase<'rules, Pruner, TplStrategies, AlphaBetaPruner> {
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

macro_rules! define_and_impl_perminmaxstrategies{([$(($IsSome:ident, $emmstrategy:ident, $ident_strategy:ident))*][$($ident_strategy_cmp:ident)*]) => {
    #[derive(Clone, Copy)]
    pub enum EMinMaxStrategy {
        $($emmstrategy,)*
    }

    pub trait TTplStrategies : Clone + PartialEq + std::fmt::Debug + 'static + Serialize {
        $(type $IsSome: TIsSome;)*

        fn maxmin_for_pruner(permmstrategy: &SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, Self>, epi_self: EPlayerIndex) -> isize;
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct SPerMinMaxStrategyGeneric<T, TplStrategies: TTplStrategies> {
        $(/*TODO pub a good idea?*/pub $ident_strategy: StaticOption<$emmstrategy<T>, TplStrategies::$IsSome>,)*
    }


    impl<T, TplStrategies: TTplStrategies> SPerMinMaxStrategyGeneric<T, TplStrategies> {
        pub fn new(t: T) -> Self
            where T: Clone
        {
            Self {
                $($ident_strategy: StaticOption::<_, TplStrategies::$IsSome>::new_with(|| $emmstrategy::new(t.clone())),)* // TODO can we avoid one clone call?
            }
        }

        pub fn map<R>(&self, mut f: impl FnMut(&T)->R) -> SPerMinMaxStrategyGeneric<R, TplStrategies> {
            let Self{ $($ident_strategy,)* } = self;
            SPerMinMaxStrategyGeneric{
                $($ident_strategy: $ident_strategy.as_ref().map(|t| $emmstrategy::new(f(&t.0))),)*
            }
        }

        pub fn modify_with_other<T1>(
            &mut self,
            other: &SPerMinMaxStrategyGeneric<T1, TplStrategies>, 
            mut fn_modify_element: impl FnMut(&mut T, &T1),
        ) {
            $(
                self.$ident_strategy.as_mut().tuple_2(other.$ident_strategy.as_ref())
                    .map(|(t, t1)| fn_modify_element(&mut t.0, &t1.0)); // TODO good idea to use "map" to call a function?
            )*
        }

        pub fn via_accessors(&self) -> Vec<(EMinMaxStrategy, &T)> {
            [$((EMinMaxStrategy::$emmstrategy, self.$ident_strategy.as_ref().map(|t| &t.0).into_option()),)*]
                .into_iter()
                .filter_map(|(emmstrategy, ot)| ot.map(|t| (emmstrategy, t)))
                .collect()
        }

        pub fn accessors() -> &'static [(EMinMaxStrategy, fn(&Self)->Option<&T>)] { // TODO is there a better alternative?
            &[$((EMinMaxStrategy::$emmstrategy, (|slf: &Self| slf.$ident_strategy.as_ref().into_option().map(|t| &t.0)) as fn(&Self) -> Option<&T>),)*]
        }
        pub fn compare_canonical<PayoutStatsPayload: Ord+Copy>(&self, other: &Self, fn_loss_or_win: impl Fn(isize, PayoutStatsPayload)->std::cmp::Ordering) -> std::cmp::Ordering where T: Borrow<SPayoutStats<PayoutStatsPayload>> {
            use std::cmp::Ordering::*;
            fn compare_fractions((numerator_lhs, denominator_lhs): (u128, u128), (numerator_rhs, denominator_rhs): (u128, u128)) -> std::cmp::Ordering {
                u128::cmp(&(numerator_lhs * denominator_rhs), &(denominator_lhs * numerator_rhs))
            }
            Equal
                $(.then_with(|| self.$ident_strategy_cmp.as_ref().tuple_2(other.$ident_strategy_cmp.as_ref())
                    .map_or_else(/*default*/|| Equal, |(lhs, rhs)| {
                        let payoutstats_lhs = lhs.0.borrow();
                        let payoutstats_rhs = rhs.0.borrow();
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
                    })
                ))*
        }
    }

    impl<TplStrategies: TTplStrategies> SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies> {
        fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex) {
            let Self{$($ident_strategy,)*} = other;
            $(
                self.$ident_strategy.as_mut().tuple_2($ident_strategy)
                    .map(|(lhs, rhs)| lhs.assign_minmax_self(rhs, epi_self)); // TODO good idea to use "map" to call a function?
            )*
        }

        fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, epi_card: EPlayerIndex) {
            let Self{$($ident_strategy,)*} = other;
            $(
                self.$ident_strategy.as_mut().tuple_2($ident_strategy)
                    .map(|(lhs, rhs)| lhs.assign_minmax_other(rhs, epi_self, epi_card)); // TODO good idea to use "map" to call a function?
            )*
        }
    }

    impl<TplStrategies: TTplStrategies> TSnapshotVisualizer<SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>> for SForEachSnapshotHTMLVisualizer<'_> {
        fn begin_snapshot(&mut self, ahand: &EnumMap<EPlayerIndex, SHand>, stichseq: &SStichSequence) {
            let str_item_id = format!("{}{}",
                stichseq.count_played_cards(),
                rand::rng().sample_iter(&rand::distr::Alphanumeric).take(16).join(""), // we simply assume no collisions here TODO uuid
            );
            self.write_all(&format!("<li><<input type=\"checkbox\" id=\"{str_item_id}\" />>\n"));
            self.write_all(&format!("<label for=\"{}\">{} direct successors<table><tr>\n",
                str_item_id,
                "TODO", // slccard_allowed.len(),
            ));
            assert!(crate::ai::ahand_vecstich_card_count_is_compatible(ahand, stichseq));
            self.write_all(&html_generator::html_display_children(player_table_stichseq(self.epi, stichseq, &output_card)));
            self.write_all(&player_table_ahand(self.epi, ahand, self.rules, /*fn_border*/|_card| false, &output_card));
            self.write_all(&"</tr></table></label>\n");
            self.write_all(&"<ul>\n");
        }

        fn end_snapshot(&mut self, minmax: &SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>) {
            self.write_all(&"</ul>\n");
            self.write_all(&"</li>\n");
            self.write_all(&player_table(self.epi, |epi| {
                Some(
                    minmax.via_accessors().into_iter()
                        .map(|(_emmstrategy, an_payout)| an_payout[epi])
                        .join("/")
                )
            }).to_string());
        }
    }
}}
define_and_impl_perminmaxstrategies!(
    [
        (IsSomeMinMin, MinMin, ominmin)
        (IsSomeMaxMin, MaxMin, omaxmin)
        (IsSomeMaxSelfishMin, MaxSelfishMin, omaxselfishmin)
        (IsSomeMaxSelfishMax, MaxSelfishMax, omaxselfishmax)
        (IsSomeMaxMax, Max, omaxmax)
    ]
    [omaxselfishmin omaxselfishmax omaxmin omaxmax ominmin]
);

macro_rules! impl_perminmaxstrategy{(
    $struct:ident {
        $IsSomeMinMin:ident,
        $IsSomeMaxMin:ident,
        $IsSomeMaxSelfishMin:ident,
        $IsSomeMaxSelfishMax:ident,
        $IsSomeMaxMax:ident,
    }
    $struct_tpl_strategies:ident
    $ident_strategy_maxmin_for_pruner:ident
) => {
    #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
    pub struct $struct_tpl_strategies;
    impl TTplStrategies for $struct_tpl_strategies {
        type IsSomeMinMin = $IsSomeMaxMin;
        type IsSomeMaxMin = $IsSomeMaxMin;
        type IsSomeMaxSelfishMin = $IsSomeMaxSelfishMin;
        type IsSomeMaxSelfishMax = $IsSomeMaxSelfishMax;
        type IsSomeMaxMax = $IsSomeMaxMax;

        fn maxmin_for_pruner(permmstrategy: &SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, Self>, epi_self: EPlayerIndex) -> isize {
            permmstrategy.$ident_strategy_maxmin_for_pruner.as_ref().unwrap_static_some().0[epi_self]
        }
    }
    pub type $struct<T> = SPerMinMaxStrategyGeneric<
        T,
        $struct_tpl_strategies,
    >;
}}

// Field nomenclature: self-strategy, followed by others-strategy
impl_perminmaxstrategy!(
    SPerMinMaxStrategy {
        /*IsSomeMinMin*/SIsSomeTrue,
        /*IsSomeMaxMin*/SIsSomeTrue,
        /*IsSomeMaxSelfishMin*/SIsSomeTrue,
        /*IsSomeMaxSelfishMax*/SIsSomeTrue,
        /*IsSomeMaxMax*/SIsSomeTrue,
    }
    STplStrategiesAll
    omaxmin // TODO not sensible to prune according to only one strategy
);
impl_perminmaxstrategy!(
    SMaxMinMaxSelfishMin {
        /*IsSomeMinMin*/SIsSomeFalse,
        /*IsSomeMaxMin*/SIsSomeTrue,
        /*IsSomeMaxSelfishMin*/SIsSomeTrue,
        /*IsSomeMaxSelfishMax*/SIsSomeFalse,
        /*IsSomeMaxMax*/SIsSomeFalse,
    }
    STplStrategiesOnlyMaxSelfishMinMaxMin
    omaxmin // TODO not sensible to prune according to only one strategy
);
impl_perminmaxstrategy!(
    SMaxMinStrategy {
        /*IsSomeMinMin*/SIsSomeFalse,
        /*IsSomeMaxMin*/SIsSomeTrue,
        /*IsSomeMaxSelfishMin*/SIsSomeFalse,
        /*IsSomeMaxSelfishMax*/SIsSomeFalse,
        /*IsSomeMaxMax*/SIsSomeFalse,
    }
    STplStrategiesOnlyMaxMin
    omaxmin
);
impl_perminmaxstrategy!(
    SMaxSelfishMinStrategy {
        /*IsSomeMinMin*/SIsSomeFalse,
        /*IsSomeMaxMin*/SIsSomeFalse,
        /*IsSomeMaxSelfishMin*/SIsSomeTrue,
        /*IsSomeMaxSelfishMax*/SIsSomeFalse,
        /*IsSomeMaxMax*/SIsSomeFalse,
    }
    STplStrategiesOnlyMaxSelfishMin
    omaxselfishmin
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
        assign_lt_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
    fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, _epi_card: EPlayerIndex) {
        assign_lt_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaxMin<T>(pub T);
impl<T> MaxMin<T> {
    fn new(t: T) -> Self {
        Self(t)
    }
}
impl MaxMin<EnumMap<EPlayerIndex, isize>> {
    fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex) {
        assign_gt_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
    fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, _epi_card: EPlayerIndex) {
        assign_lt_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MaxSelfishMin<T>(pub T);
impl<T> MaxSelfishMin<T> {
    fn new(t: T) -> Self {
        Self(t)
    }
}
impl MaxSelfishMin<EnumMap<EPlayerIndex, isize>> {
    fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex) {
        assign_gt_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
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
pub struct MaxSelfishMax<T>(pub T);
impl<T> MaxSelfishMax<T> {
    fn new(t: T) -> Self {
        Self(t)
    }
}
impl MaxSelfishMax<EnumMap<EPlayerIndex, isize>> {
    fn assign_minmax_self(&mut self, other: Self, epi_self: EPlayerIndex) {
        assign_gt_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
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
        assign_gt_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
    fn assign_minmax_other(&mut self, other: Self, epi_self: EPlayerIndex, _epi_card: EPlayerIndex) {
        assign_gt_by_key(&mut self.0, other.0, |an_payout| an_payout[epi_self]);
    }
}

pub trait TAlphaBetaPruner {
    type InfoFromParent: Clone;
    fn initial_info_from_parent() -> Self::InfoFromParent;
    type BreakType;
    fn is_prunable_self<TplStrategies: TTplStrategies>(&self, minmax: &SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>, infofromparent: Self::InfoFromParent, epi_self: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent>;
    fn is_prunable_other<TplStrategies: TTplStrategies>(&self, minmax: &SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>, infofromparent: Self::InfoFromParent, epi_self: EPlayerIndex, epi_card: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent>;
}

#[derive(Default)]
pub struct SAlphaBetaPrunerNone;
impl TAlphaBetaPruner for SAlphaBetaPrunerNone {
    type InfoFromParent = ();
    fn initial_info_from_parent() -> Self::InfoFromParent {
    }
    type BreakType = Infallible;
    fn is_prunable_self<TplStrategies: TTplStrategies>(&self, _minmax: &SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>, _infofromparent: Self::InfoFromParent, _epi_self: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent> {
        ControlFlow::Continue(())
    }
    fn is_prunable_other<TplStrategies: TTplStrategies>(&self, _minmax: &SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>, _infofromparent: Self::InfoFromParent, _epi_self: EPlayerIndex, _epi_card: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent> {
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
    fn is_prunable_self<TplStrategies: TTplStrategies>(&self, minmax: &SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>, mut infofromparent: Self::InfoFromParent, epi_self: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent> {
        assert_eq!(self.mapepilohi[epi_self], ELoHi::Hi);
        let n_payout_for_pruner = TplStrategies::maxmin_for_pruner(minmax, epi_self);
        if n_payout_for_pruner >= infofromparent[ELoHi::Hi] {
            // I'm maximizing myself, but if my parent will minimize against what's already in there, I do not need to investigate any further
            ControlFlow::Break(())
        } else {
            assign_gt(&mut infofromparent[ELoHi::Lo], n_payout_for_pruner);
            ControlFlow::Continue(infofromparent)
        }
    }
    fn is_prunable_other<TplStrategies: TTplStrategies>(&self, minmax: &SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>, mut infofromparent: Self::InfoFromParent, epi_self: EPlayerIndex, epi_card: EPlayerIndex) -> ControlFlow<Self::BreakType, /*Continue*/Self::InfoFromParent> {
        match self.mapepilohi[epi_card] {
            ELoHi::Hi => {
                self.is_prunable_self(
                    minmax,
                    infofromparent,
                    epi_self, // TODO we should asser that we could also pass epi_card here.
                )
            },
            ELoHi::Lo => {
                let n_payout_for_pruner = TplStrategies::maxmin_for_pruner(minmax, epi_self);
                if n_payout_for_pruner <= infofromparent[ELoHi::Lo] {
                    // I'm minimizing myself, but if my parent will maximize against what's already in there, I do not need to investigate any further
                    ControlFlow::Break(())
                } else {
                    assign_lt(&mut infofromparent[ELoHi::Hi], n_payout_for_pruner);
                    ControlFlow::Continue(infofromparent)
                }
            }
        }
    }
}

impl<Pruner: TPruner, TplStrategies: TTplStrategies, AlphaBetaPruner: TAlphaBetaPruner> TForEachSnapshot for SMinReachablePayoutBase<'_, Pruner, TplStrategies, AlphaBetaPruner> {
    type Output = SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>;
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

pub type SGenericMinReachablePayout<'rules, TplStrategies, AlphaBetaPruner> = SMinReachablePayoutBase<'rules, SPrunerNothing, TplStrategies, AlphaBetaPruner>;
pub type SMinReachablePayout<'rules> = SMinReachablePayoutBase<'rules, SPrunerNothing, STplStrategiesAll, SAlphaBetaPrunerNone>;
pub type SGenericMinReachablePayoutLowerBoundViaHint<'rules, TplStrategies, AlphaBetaPruner> = SMinReachablePayoutBase<'rules, SPrunerViaHint, TplStrategies, AlphaBetaPruner>;
pub type SMinReachablePayoutLowerBoundViaHint<'rules> = SMinReachablePayoutBase<'rules, SPrunerViaHint, STplStrategiesAll, SAlphaBetaPrunerNone>;

pub trait TPruner : Sized {
    fn pruned_output<TplStrategies: TTplStrategies, AlphaBetaPruner>(params: &SMinReachablePayoutBase<'_, Self, TplStrategies, AlphaBetaPruner>, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), rulestatecache: &SRuleStateCache) -> Option<SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>>;
}

pub struct SPrunerNothing;
impl TPruner for SPrunerNothing {
    fn pruned_output<TplStrategies: TTplStrategies, AlphaBetaPruner>(_params: &SMinReachablePayoutBase<'_, Self, TplStrategies, AlphaBetaPruner>, _tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), _rulestatecache: &SRuleStateCache) -> Option<SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>> {
        None
    }
}

pub struct SPrunerViaHint;
impl TPruner for SPrunerViaHint {
    fn pruned_output<TplStrategies: TTplStrategies, AlphaBetaPruner>(params: &SMinReachablePayoutBase<'_, Self, TplStrategies, AlphaBetaPruner>, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), rulestatecache: &SRuleStateCache) -> Option<SPerMinMaxStrategyGeneric<EnumMap<EPlayerIndex, isize>, TplStrategies>> {
        let mapepion_payout = params.rules.payouthints(tplahandstichseq, &params.expensifiers, rulestatecache)
            .map(|intvlon_payout| {
                intvlon_payout[ELoHi::Lo].filter(|n_payout| 0<*n_payout)
                    .or_else(|| intvlon_payout[ELoHi::Hi].filter(|n_payout| *n_payout<0))
            });
        if_then_some!(
            mapepion_payout.iter().all(Option::is_some),
            SPerMinMaxStrategyGeneric::<EnumMap<EPlayerIndex, isize>, _>::new(mapepion_payout.map(|opayout| unwrap!(opayout)))
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
