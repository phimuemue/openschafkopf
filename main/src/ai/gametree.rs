use crate::game::{stoss_and_doublings, SGame, SStichSequence};
use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use itertools::Itertools;
use rand::{self, Rng};
use std::{cmp::Ordering, fmt, fs, io::{BufWriter, Write}};
use super::cardspartition::*;

pub trait TForEachSnapshot {
    type Output;
    fn final_output(&self, slcstich: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output;
    fn pruned_output(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> Option<Self::Output>;
    fn combine_outputs<ItTplCardOutput: Iterator<Item=(SCard, Self::Output)>>(
        &self,
        epi_card: EPlayerIndex,
        ittplcardoutput: ItTplCardOutput,
    ) -> Self::Output;
}

pub trait TSnapshotVisualizer<Output> {
    fn begin_snapshot(&mut self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>);
    fn end_snapshot(&mut self, output: &Output);
}


pub fn visualizer_factory<'rules>(path: std::path::PathBuf, rules: &'rules dyn TRules, epi: EPlayerIndex) -> impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, SCard) -> SForEachSnapshotHTMLVisualizer<'rules> {
    unwrap!(std::fs::create_dir_all(&path));
    unwrap!(crate::game_analysis::generate_html_auxiliary_files(&path));
    move |i_ahand, ahand, card| {
        let path_abs = path.join(
            &std::path::Path::new(&format!("{}", chrono::Local::now().format("%Y%m%d%H%M%S")))
                .join(format!("{}_{}_{}.html",
                    i_ahand,
                    ahand.iter()
                        .map(|hand| hand.cards().iter().join(""))
                        .join("_"),
                    card
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

pub fn output_card(card: SCard, b_border: bool) -> String {
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

pub fn player_table_ahand(epi_self: EPlayerIndex, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules, fn_border: impl Fn(SCard)->bool) -> String {
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
    fn begin_snapshot(&mut self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>) {
        let str_item_id = format!("{}{}",
            stichseq.count_played_cards(),
            rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).join(""), // we simply assume no collisions here TODO uuid
        );
        self.write_all(format!("<li><<input type=\"checkbox\" id=\"{}\" />>\n", str_item_id).as_bytes());
        self.write_all(format!("<label for=\"{}\">{} direct successors<table><tr>\n",
            str_item_id,
            "TODO", // slccard_allowed.len(),
        ).as_bytes());
        assert!(crate::ai::ahand_vecstich_card_count_is_compatible(stichseq, ahand));
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
    pub fn factory() -> impl Fn(usize, &EnumMap<EPlayerIndex, SHand>, SCard)->Self + std::marker::Sync {
        |_,_,_| Self
    }
}
impl<Output> TSnapshotVisualizer<Output> for SNoVisualization {
    fn begin_snapshot(&mut self, _stichseq: &SStichSequence, _ahand: &EnumMap<EPlayerIndex, SHand>) {}
    fn end_snapshot(&mut self, _output: &Output) {}
}

pub trait TFilterAllowedCards {
    type UnregisterStich;
    fn register_stich(&mut self, stich: &SStich) -> Self::UnregisterStich;
    fn unregister_stich(&mut self, unregisterstich: Self::UnregisterStich);
    fn filter_allowed_cards(&self, stichseq: &SStichSequence, veccard: &mut SHandVector);
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
    fn register_stich(&mut self, _stich: &SStich) -> Self::UnregisterStich {}
    fn unregister_stich(&mut self, _unregisterstich: Self::UnregisterStich) {}
    fn filter_allowed_cards(&self, _stichseq: &SStichSequence, _veccard: &mut SHandVector) {}
}

pub fn explore_snapshots<
    ForEachSnapshot,
    FilterAllowedCards: TFilterAllowedCards,
    OFilterAllowedCards: Into<Option<FilterAllowedCards>>,
>(
    ahand: &mut EnumMap<EPlayerIndex, SHand>,
    rules: &dyn TRules,
    stichseq: &mut SStichSequence,
    fn_make_filter: &impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>)->OFilterAllowedCards,
    foreachsnapshot: &ForEachSnapshot,
    snapshotvisualizer: &mut impl TSnapshotVisualizer<ForEachSnapshot::Output>,
) -> ForEachSnapshot::Output 
    where
        ForEachSnapshot: TForEachSnapshot,
{
    macro_rules! forward{($func_filter_allowed_cards:expr,) => {
        explore_snapshots_internal(
            ahand,
            rules,
            &mut SRuleStateCache::new(
                stichseq,
                ahand,
                |stich| rules.winner_index(stich),
            ),
            stichseq,
            $func_filter_allowed_cards,
            foreachsnapshot,
            snapshotvisualizer,
        )
    }}
    cartesian_match!(forward,
        match (fn_make_filter(stichseq, ahand).into()) {
            Some(mut func_filter_allowed_cards) => (&mut func_filter_allowed_cards),
            None => (/*func_filter_allowed_cards*/&mut SNoFilter),
        },
    )
}

fn explore_snapshots_internal<ForEachSnapshot>(
    ahand: &mut EnumMap<EPlayerIndex, SHand>,
    rules: &dyn TRules,
    rulestatecache: &mut SRuleStateCache,
    stichseq: &mut SStichSequence,
    func_filter_allowed_cards: &mut impl TFilterAllowedCards,
    foreachsnapshot: &ForEachSnapshot,
    snapshotvisualizer: &mut impl TSnapshotVisualizer<ForEachSnapshot::Output>,
) -> ForEachSnapshot::Output 
    where
        ForEachSnapshot: TForEachSnapshot,
{
    snapshotvisualizer.begin_snapshot(stichseq, ahand);
    let epi_current = unwrap!(stichseq.current_stich().current_playerindex());
    let output = if debug_verify_eq!(
        ahand[epi_current].cards().len() <= 1,
        ahand.iter().all(|hand| hand.cards().len() <= 1)
    ) {
        macro_rules! for_each_allowed_card{
            (($i_offset_0: expr, $($i_offset: expr,)*), $stichseq: expr) => {{
                let epi = epi_current.wrapping_add($i_offset_0);
                let card = debug_verify_eq!(
                    ahand[epi].cards(),
                    &rules.all_allowed_cards($stichseq, &ahand[epi])
                )[0];
                //ahand[epi].play_card(card); // not necessary
                let output = $stichseq.zugeben_and_restore(
                    card,
                    rules,
                    |stichseq| {for_each_allowed_card!(($($i_offset,)*), stichseq)}
                );
                //ahand[epi].add_card(card); // not necessary
                output
            }};
            ((), $stichseq: expr) => {{
                let unregisterstich = rulestatecache.register_stich(
                    unwrap!($stichseq.completed_stichs().last()),
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
        foreachsnapshot.pruned_output(stichseq, ahand, rulestatecache).unwrap_or_else(|| {
            let mut veccard_allowed = rules.all_allowed_cards(stichseq, &ahand[epi_current]);
            func_filter_allowed_cards.filter_allowed_cards(stichseq, &mut veccard_allowed);
            // TODO? use equivalent card optimization
            foreachsnapshot.combine_outputs(
                epi_current,
                veccard_allowed.into_iter().map(|card| {
                    ahand[epi_current].play_card(card);
                    let output = stichseq.zugeben_and_restore(card, rules, |stichseq| {
                        macro_rules! next_step {($func_filter_allowed_cards:expr) => {explore_snapshots_internal(
                            ahand,
                            rules,
                            rulestatecache,
                            stichseq,
                            $func_filter_allowed_cards,
                            foreachsnapshot,
                            snapshotvisualizer,
                        )}}
                        if stichseq.current_stich().is_empty() {
                            let stich = unwrap!(stichseq.completed_stichs().last());
                            let unregisterstich_cache = rulestatecache.register_stich(
                                stich,
                                stichseq.current_stich().first_playerindex(),
                            );
                            let output = if func_filter_allowed_cards.continue_with_filter(stichseq) {
                                let unregisterstich_filter = func_filter_allowed_cards.register_stich(stich);
                                let output = next_step!(func_filter_allowed_cards);
                                func_filter_allowed_cards.unregister_stich(unregisterstich_filter);
                                output
                            } else {
                                next_step!(&mut SNoFilter)
                            };
                            rulestatecache.unregister_stich(unregisterstich_cache);
                            output
                        } else {
                            next_step!(func_filter_allowed_cards)
                        }
                    });
                    ahand[epi_current].add_card(card);
                    (card, output)
                })
            )
        })
    };
    snapshotvisualizer.end_snapshot(&output);
    output
}

#[derive(Clone, new)]
pub struct SMinReachablePayoutBase<'rules, Pruner> {
    rules: &'rules dyn TRules,
    epi: EPlayerIndex,
    tpln_stoss_doubling: (usize, usize),
    n_stock: isize,
    phantom: std::marker::PhantomData<Pruner>,
}
impl<'rules, Pruner> SMinReachablePayoutBase<'rules, Pruner> {
    pub fn new_from_game(game: &'rules SGame) -> Self {
        Self::new(
            game.rules.as_ref(),
            unwrap!(game.current_playable_stich().current_playerindex()),
            /*tpln_stoss_doubling*/stoss_and_doublings(&game.vecstoss, &game.doublings),
            game.n_stock,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SPerMinMaxStrategy<T>(pub EnumMap<EMinMaxStrategy, T>);

pub type SMinMax = SPerMinMaxStrategy<EnumMap<EPlayerIndex, isize>>;

impl SMinMax {
    fn new_final(an_payout: EnumMap<EPlayerIndex, isize>) -> Self {
        Self(EMinMaxStrategy::map_from_fn(|_| an_payout.explicit_clone()))
    }
}

impl<Pruner: TPruner> TForEachSnapshot for SMinReachablePayoutBase<'_, Pruner> {
    type Output = SMinMax;

    fn final_output(&self, slcstich: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output {
        SMinMax::new_final(self.rules.payout(
            slcstich,
            self.tpln_stoss_doubling,
            self.n_stock,
            rulestatecache,
            /*b_test_points_as_payout*/if_dbg_else!({true}{()}),
        ))
    }

    fn pruned_output(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> Option<Self::Output> {
        Pruner::pruned_output(self, stichseq, ahand, rulestatecache)
    }

    fn combine_outputs<ItTplCardOutput: Iterator<Item=(SCard, Self::Output)>>(
        &self,
        epi_card: EPlayerIndex,
        ittplcardoutput: ItTplCardOutput,
    ) -> Self::Output {
        let itminmax = ittplcardoutput.map(|(_card, minmax)| minmax);
        unwrap!(if self.epi==epi_card {
            itminmax.reduce(mutate_return!(|minmax_acc, minmax| {
                assign_min_by_key(
                    &mut minmax_acc.0[EMinMaxStrategy::MinMin],
                    minmax.0[EMinMaxStrategy::MinMin].explicit_clone(),
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
                        minmax.0[emmstrategy].explicit_clone(),
                        |an_payout| an_payout[self.epi],
                    );
                }
            }))
        } else {
            // other players may play inconveniently for epi_stich
            itminmax.reduce(mutate_return!(|minmax_acc, minmax| {
                assign_min_by_key(
                    &mut minmax_acc.0[EMinMaxStrategy::MinMin],
                    minmax.0[EMinMaxStrategy::MinMin].explicit_clone(),
                    |an_payout| an_payout[self.epi],
                );
                assign_min_by_key(
                    &mut minmax_acc.0[EMinMaxStrategy::Min],
                    minmax.0[EMinMaxStrategy::Min].explicit_clone(),
                    |an_payout| an_payout[self.epi],
                );
                assign_better(
                    &mut minmax_acc.0[EMinMaxStrategy::SelfishMin],
                    minmax.0[EMinMaxStrategy::SelfishMin].explicit_clone(),
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
                    minmax.0[EMinMaxStrategy::SelfishMax].explicit_clone(),
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
                    minmax.0[EMinMaxStrategy::Max].explicit_clone(),
                    |an_payout| an_payout[self.epi],
                );

            }))
        })
    }
}

pub type SMinReachablePayout<'rules> = SMinReachablePayoutBase<'rules, SPrunerNothing>;
pub type SMinReachablePayoutLowerBoundViaHint<'rules> = SMinReachablePayoutBase<'rules, SPrunerViaHint>;

pub trait TPruner : Sized {
    fn pruned_output(params: &SMinReachablePayoutBase<'_, Self>, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> Option<SMinMax>;
}

pub struct SPrunerNothing;
impl TPruner for SPrunerNothing {
    fn pruned_output(_params: &SMinReachablePayoutBase<'_, Self>, _stichseq: &SStichSequence, _ahand: &EnumMap<EPlayerIndex, SHand>, _rulestatecache: &SRuleStateCache) -> Option<SMinMax> {
        None
    }
}

pub struct SPrunerViaHint;
impl TPruner for SPrunerViaHint {
    fn pruned_output(params: &SMinReachablePayoutBase<'_, Self>, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> Option<SMinMax> {
        let mapepion_payout = params.rules.payouthints(stichseq, ahand, params.tpln_stoss_doubling, params.n_stock, rulestatecache)
            .map(|intvlon_payout| intvlon_payout[ELoHi::Lo]);
        if_then_some!(
            mapepion_payout.iter().all(Option::is_some) && 0<unwrap!(mapepion_payout[params.epi]),
            SMinMax::new_final(mapepion_payout.map(|opayout| unwrap!(opayout)))
        )
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct SFilterEquivalentCards {
    cardspartition: SCardsPartition,
    epi_fixed: EPlayerIndex,
    n_until_stichseq_len: usize,
}

impl TFilterAllowedCards for SFilterEquivalentCards {
    type UnregisterStich = EnumMap<EPlayerIndex, SRemoved>;
    fn register_stich(&mut self, stich: &SStich) -> Self::UnregisterStich {
        assert!(stich.is_full());
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
                debug_assert_eq!(veccard as &[SCard], &veccard_out as &[SCard]);
            }
        }
        *veccard = unwrap!((&veccard_out as &[SCard]).try_into());
    }
    fn continue_with_filter(&self, stichseq: &SStichSequence) -> bool {
        stichseq.completed_stichs().len()<=self.n_until_stichseq_len
    }
}

pub fn equivalent_cards_filter(
    n_until_stichseq_len: usize,
    epi_fixed: EPlayerIndex,
    cardspartition: SCardsPartition,
) -> impl Fn(&SStichSequence, &EnumMap<EPlayerIndex, SHand>)->SFilterEquivalentCards {
    move |stichseq, _ahand| {
        let mut filterequivalentcards = SFilterEquivalentCards {
            cardspartition: cardspartition.clone(),
            epi_fixed,
            n_until_stichseq_len,
        };
        for stich in stichseq.completed_stichs() {
            filterequivalentcards.register_stich(stich);
        }
        filterequivalentcards
    }
}
