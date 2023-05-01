use crate::game::SGame;
use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use itertools::Itertools;
use rand::{self, Rng};
use std::{cmp::Ordering, fmt, fs, io::{BufWriter, Write}};
use super::cardspartition::*;

pub trait TForEachSnapshot {
    type Output;
    type InfoFromParent;
    fn final_output(&self, stichseq: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output;
    fn pruned_output(&self, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), rulestatecache: &SRuleStateCache) -> Option<Self::Output>;
    fn combine_outputs(
        &self,
        epi_card: EPlayerIndex,
        oinfofromparent: Option<Self::InfoFromParent>,
        veccard: SHandVector, // TODO? &[ECard] better?
        fn_card_to_output: impl FnMut(ECard, Option<Self::InfoFromParent>)->Self::Output,
    ) -> Self::Output;
}

pub trait TSnapshotVisualizer<Output> {
    fn begin_snapshot(&mut self, ahand: &EnumMap<EPlayerIndex, SHand>, stichseq: &SStichSequence);
    fn end_snapshot(&mut self, output: &Output);
}


pub fn visualizer_factory<'rules>(path: std::path::PathBuf, rules: &'rules dyn TRules, epi: EPlayerIndex) -> impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, Option<ECard>) -> SForEachSnapshotHTMLVisualizer<'rules> {
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

pub struct SForEachSnapshotHTMLVisualizer<'rules> {
    file_output: BufWriter<fs::File>,
    rules: &'rules dyn TRules,
    epi: EPlayerIndex,
}
impl<'rules> SForEachSnapshotHTMLVisualizer<'rules> {
    fn new(file_output: fs::File, rules: &'rules dyn TRules, epi: EPlayerIndex) -> Self {
        let mut foreachsnapshothtmlvisualizer = SForEachSnapshotHTMLVisualizer{file_output: BufWriter::with_capacity(16*1024*1024, file_output), rules, epi};
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
          <tr><td colspan=\"2\">{}</td></tr>
          <tr><td>{}</td><td>{}</td></tr>
          <tr><td colspan=\"2\">{}</td></tr>
        </table>\n",
        fn_per_player_internal(EPlayerIndex::EPI2),
        fn_per_player_internal(EPlayerIndex::EPI1),
        fn_per_player_internal(EPlayerIndex::EPI3),
        fn_per_player_internal(EPlayerIndex::EPI0),
    )
}

pub fn player_table_stichseq(epi_self: EPlayerIndex, stichseq: &SStichSequence) -> String {
    format!("{}", stichseq.visible_stichs().iter().map(|stich| {
        format!("<td>{}</td>", player_table(epi_self, |epi| {
            stich.get(epi).map(|card| {
                output_card(*card, /*b_border*/epi==stich.first_playerindex())
            })
        }))
    }).format("\n"))
}

pub fn player_table_ahand(epi_self: EPlayerIndex, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules, fn_border: impl Fn(ECard)->bool) -> String {
    format!(
        "<td>{}</td>\n",
        player_table(epi_self, |epi| {
            let mut veccard = ahand[epi].cards().clone();
            rules.sort_cards_first_trumpf_then_farbe(&mut veccard);
            Some(veccard.into_iter()
                .map(|card| output_card(card, fn_border(card)))
                .join(""))
        }),
    )
}

impl TSnapshotVisualizer<SMinMax> for SForEachSnapshotHTMLVisualizer<'_> {
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
        self.write_all(player_table_stichseq(self.epi, stichseq).as_bytes());
        self.write_all(player_table_ahand(self.epi, ahand, self.rules, /*fn_border*/|_card| false).as_bytes());
        self.write_all(b"</tr></table></label>\n");
        self.write_all(b"<ul>\n");
    }

    fn end_snapshot(&mut self, minmax: &SMinMax) {
        self.write_all(b"</ul>\n");
        self.write_all(b"</li>\n");
        self.write_all(player_table(self.epi, |epi| {
            Some(
                minmax.0.iter()
                    .map(|an_payout| an_payout[epi])
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
    rules: &'rules dyn TRules,
}

impl<'rules> TFilterAllowedCards for SFilterOnePerWinnerIndex<'rules> {
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
    rules: &dyn TRules,
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
            /*oinfofromparent*/None,
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
    rules: &dyn TRules,
    rulestatecache: &mut SRuleStateCache,
    func_filter_allowed_cards: &mut impl TFilterAllowedCards,
    foreachsnapshot: &ForEachSnapshot,
    oinfofromparent: Option<ForEachSnapshot::InfoFromParent>,
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
                assert_eq!(n_stich_size, 4);
                for_each_allowed_card!((), stichseq)
            },
        }
    } else {
        foreachsnapshot.pruned_output((ahand, stichseq), rulestatecache).unwrap_or_else(|| {
            let mut veccard_allowed = rules.all_allowed_cards(stichseq, &ahand[epi_current]);
            func_filter_allowed_cards.filter_allowed_cards(stichseq, &mut veccard_allowed);
            // TODO? use equivalent card optimization
            foreachsnapshot.combine_outputs(
                epi_current,
                oinfofromparent,
                veccard_allowed,
                |card, oinfofromparent| {
                    stichseq.zugeben_and_restore_with_hands(ahand, epi_current, card, rules, |ahand, stichseq| {
                        macro_rules! next_step {($func_filter_allowed_cards:expr, $snapshotcache:expr) => {explore_snapshots_internal(
                            (ahand, stichseq),
                            rules,
                            rulestatecache,
                            $func_filter_allowed_cards,
                            foreachsnapshot,
                            oinfofromparent,
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
pub struct SMinReachablePayoutBase<'rules, Pruner, AlphaBetaPruner> {
    rules: &'rules dyn TRules,
    epi: EPlayerIndex,
    expensifiers: SExpensifiers, // TODO could this borrow?
    phantom: std::marker::PhantomData<Pruner>,
    alphabetapruner: AlphaBetaPruner,
}
impl<'rules, Pruner, AlphaBetaPruner> SMinReachablePayoutBase<'rules, Pruner, AlphaBetaPruner> {
    pub fn internal_new(rules: &'rules dyn TRules, epi: EPlayerIndex, expensifiers: SExpensifiers, alphabetapruner: AlphaBetaPruner) -> Self {
        Self {
            rules,
            epi,
            expensifiers,
            phantom: Default::default(),
            alphabetapruner,
        }
    }
}
impl<'rules, Pruner> SMinReachablePayoutBase<'rules, Pruner, SAlphaBetaPrunerNone> {
    pub fn new(rules: &'rules dyn TRules, epi: EPlayerIndex, expensifiers: SExpensifiers) -> Self {
        Self::internal_new(
            rules,
            epi,
            expensifiers,
            SAlphaBetaPrunerNone,
        )
    }
    pub fn new_from_game(game: &'rules SGame) -> Self {
        Self::new(
            game.rules.as_ref(),
            unwrap!(game.current_playable_stich().current_playerindex()),
            game.expensifiers.clone(),
        )
    }
}

plain_enum_mod!(modeminmaxstrategy, EMinMaxStrategy {
    MinMin,
    Min,
    SelfishMin,
    SelfishMax,
    Max,
});

// TODO(performance) offer possibility to constrain oneself to one value of EMinMaxStrategy (reduced run time by ~20%-25% in some tests)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SPerMinMaxStrategy<T>(pub EnumMap<EMinMaxStrategy, T>);

// TODO(performance) storing a whole EnumMap for each EMinMaxStrategy is unnecessary, and slows down the program
pub type SMinMax = SPerMinMaxStrategy<EnumMap<EPlayerIndex, isize>>;

impl SMinMax {
    fn new_final(an_payout: EnumMap<EPlayerIndex, isize>) -> Self {
        Self(EMinMaxStrategy::map_from_fn(|_| an_payout.explicit_clone()))
    }
}

pub trait TAlphaBetaPruner {
    type InfoFromParent;
    fn info_from_parent(&self, tplelohiepi_self: (ELoHi, EPlayerIndex), minmax: &SMinMax) -> Self::InfoFromParent;
    fn alpha_beta_prune(&self, oinfofromparent: &Option<Self::InfoFromParent>, tplelohiepi_self: (ELoHi, EPlayerIndex), minmax: &SMinMax) -> bool;
}

pub struct SAlphaBetaPrunerMin; // TODO also introduce SAlphaBetaPrunerSelfishMin
impl TAlphaBetaPruner for SAlphaBetaPrunerMin {
    type InfoFromParent = (ELoHi, isize/*payout for SMinReachablePayoutBase::epi*/);
    fn info_from_parent(&self, (elohi_self, epi_self): (ELoHi, EPlayerIndex), minmax: &SMinMax) -> Self::InfoFromParent {
        (elohi_self, minmax.0[EMinMaxStrategy::Min][epi_self])
    }
    fn alpha_beta_prune(&self, oinfofromparent: &Option<Self::InfoFromParent>, (elohi_self, epi_self): (ELoHi, EPlayerIndex), minmax: &SMinMax) -> bool {
        if let Some((elohi_parent, n_payout_epi_self_parent)) = oinfofromparent {
            if elohi_parent!=&elohi_self // contradicts this step's goal => Alpha-Beta-Pruning may be possible
                && match elohi_self {
                    ELoHi::Hi => minmax.0[EMinMaxStrategy::Min][epi_self] >= *n_payout_epi_self_parent, // parent's *minimization* will surely *not* be affected by the following possibilities, as they only are *maximized* further
                    ELoHi::Lo => minmax.0[EMinMaxStrategy::Min][epi_self] <= *n_payout_epi_self_parent, // parent's *maximization* will surely *not* be affected by the following possibilities, as they only are *minimized* further
                }
            {
                return true;
            }
        }
        false
    }
}

pub struct SAlphaBetaPrunerNone;
impl TAlphaBetaPruner for SAlphaBetaPrunerNone {
    type InfoFromParent = (); // TODO avoid Some(()) for this case
    fn info_from_parent(&self, _tplelohiepi_self: (ELoHi, EPlayerIndex), _minmax: &SMinMax) -> Self::InfoFromParent {
    }
    fn alpha_beta_prune(&self, _oinfofromparent: &Option<Self::InfoFromParent>, _tplelohiepi_self: (ELoHi, EPlayerIndex), _minmax: &SMinMax) -> bool {
        false
    }
}

impl<Pruner: TPruner, AlphaBetaPruner: TAlphaBetaPruner> TForEachSnapshot for SMinReachablePayoutBase<'_, Pruner, AlphaBetaPruner> {
    type Output = SMinMax;
    type InfoFromParent = AlphaBetaPruner::InfoFromParent;

    fn final_output(&self, stichseq: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output {
        SMinMax::new_final(self.rules.payout(
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
        oinfofromparent: Option<Self::InfoFromParent>,
        veccard: SHandVector, // TODO? &[ECard] better?
        mut fn_card_to_output: impl FnMut(ECard, Option<Self::InfoFromParent>)->Self::Output,
    ) -> Self::Output {
        let mut itcard = veccard.into_iter();
        let mut minmax_acc = fn_card_to_output(unwrap!(itcard.next()), /*oinfofromparent*/None); // first branch must always be investigated
        if self.epi==epi_card {
            for card in itcard {
                let elohi_self = ELoHi::Hi; // this step maximizes for EMinMaxStrategy::Min
                let minmax = fn_card_to_output(card, Some(self.alphabetapruner.info_from_parent((elohi_self, self.epi), &minmax_acc)));
                assign_min_by_key(
                    &mut minmax_acc.0[EMinMaxStrategy::MinMin],
                    minmax.0[EMinMaxStrategy::MinMin],
                    |an_payout| an_payout[self.epi],
                );
                for emmstrategy in [
                    // EMinMaxStrategy::MinMin done above
                    EMinMaxStrategy::Min,
                    EMinMaxStrategy::SelfishMin,
                    EMinMaxStrategy::SelfishMax,
                    EMinMaxStrategy::Max,
                ] {
                    assign_max_by_key(
                        &mut minmax_acc.0[emmstrategy],
                        minmax.0[emmstrategy],
                        |an_payout| an_payout[self.epi],
                    );
                }
                if self.alphabetapruner.alpha_beta_prune(&oinfofromparent, (elohi_self, self.epi), &minmax_acc) {
                    break;
                }
            }
        } else {
            // other players may play inconveniently for epi_stich
            for card in itcard {
                let elohi_self = ELoHi::Lo; // this step minimizes for EMinMaxStrategy::Min
                let minmax = fn_card_to_output(card, Some(self.alphabetapruner.info_from_parent((elohi_self, self.epi), &minmax_acc)));
                assign_min_by_key(
                    &mut minmax_acc.0[EMinMaxStrategy::MinMin],
                    minmax.0[EMinMaxStrategy::MinMin],
                    |an_payout| an_payout[self.epi],
                );
                assign_min_by_key(
                    &mut minmax_acc.0[EMinMaxStrategy::Min],
                    minmax.0[EMinMaxStrategy::Min],
                    |an_payout| an_payout[self.epi],
                );
                assign_better(
                    &mut minmax_acc.0[EMinMaxStrategy::SelfishMin],
                    minmax.0[EMinMaxStrategy::SelfishMin],
                    |an_payout_lhs, an_payout_rhs| {
                        match an_payout_lhs[epi_card].cmp(&an_payout_rhs[epi_card]) {
                            Ordering::Less => false,
                            Ordering::Equal => an_payout_lhs[self.epi] < an_payout_rhs[self.epi],
                            Ordering::Greater => true,
                        }
                    },
                );
                assign_better(
                    &mut minmax_acc.0[EMinMaxStrategy::SelfishMax],
                    minmax.0[EMinMaxStrategy::SelfishMax],
                    |an_payout_lhs, an_payout_rhs| {
                        match an_payout_lhs[epi_card].cmp(&an_payout_rhs[epi_card]) {
                            Ordering::Less => false,
                            Ordering::Equal => an_payout_lhs[self.epi] > an_payout_rhs[self.epi],
                            Ordering::Greater => true,
                        }
                    },
                );
                assign_max_by_key(
                    &mut minmax_acc.0[EMinMaxStrategy::Max],
                    minmax.0[EMinMaxStrategy::Max],
                    |an_payout| an_payout[self.epi],
                );
                if self.alphabetapruner.alpha_beta_prune(&oinfofromparent, (elohi_self, self.epi), &minmax_acc) {
                    break;
                }
            }
        }
        minmax_acc
    }
}

pub type SMinReachablePayout<'rules> = SMinReachablePayoutBase<'rules, SPrunerNothing, SAlphaBetaPrunerNone>;
pub type SMinReachablePayoutLowerBoundViaHint<'rules> = SMinReachablePayoutBase<'rules, SPrunerViaHint, SAlphaBetaPrunerNone>;

pub trait TPruner : Sized {
    fn pruned_output<AlphaBetaPruner>(params: &SMinReachablePayoutBase<'_, Self, AlphaBetaPruner>, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), rulestatecache: &SRuleStateCache) -> Option<SMinMax>;
}

pub struct SPrunerNothing;
impl TPruner for SPrunerNothing {
    fn pruned_output<AlphaBetaPruner>(_params: &SMinReachablePayoutBase<'_, Self, AlphaBetaPruner>, _tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), _rulestatecache: &SRuleStateCache) -> Option<SMinMax> {
        None
    }
}

pub struct SPrunerViaHint;
impl TPruner for SPrunerViaHint {
    fn pruned_output<AlphaBetaPruner>(params: &SMinReachablePayoutBase<'_, Self, AlphaBetaPruner>, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), rulestatecache: &SRuleStateCache) -> Option<SMinMax> {
        let mapepion_payout = params.rules.payouthints(tplahandstichseq, &params.expensifiers, rulestatecache)
            .map(|intvlon_payout| {
                intvlon_payout[ELoHi::Lo].filter(|n_payout| 0<*n_payout)
                    .or_else(|| intvlon_payout[ELoHi::Hi].filter(|n_payout| *n_payout<0))
            });
        if_then_some!(
            mapepion_payout.iter().all(Option::is_some),
            SMinMax::new_final(mapepion_payout.map(|opayout| unwrap!(opayout)))
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
