use crate::primitives::*;
use crate::rules::*;
use crate::game::{SGame, SStichSequence, stoss_and_doublings};
use itertools::Itertools;
use crate::util::*;
use std::{
    fs,
    io::Write,
    fmt,
};
use rand::{
    self,
    Rng,
};

pub trait TForEachSnapshot {
    type Output;
    fn final_output(&self, slcstich: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output;
    fn pruned_output(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> Option<Self::Output>;
    fn combine_outputs<ItTplCardOutput: Iterator<Item=(SCard, Self::Output)>>(
        &self,
        epi_self: EPlayerIndex,
        epi_card: EPlayerIndex,
        ittplcardoutput: ItTplCardOutput,
    ) -> Self::Output;
}

trait TSnapshotVisualizer {
    fn begin_snapshot(&mut self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>);
    fn end_snapshot<Output: fmt::Debug>(&mut self, output: &Output);
}

pub struct SForEachSnapshotHTMLVisualizer<'rules> {
    file_output: fs::File,
    rules: &'rules dyn TRules,
    epi: EPlayerIndex,
}
impl<'rules> SForEachSnapshotHTMLVisualizer<'rules> {
    pub fn new(file_output: fs::File, rules: &'rules dyn TRules, epi: EPlayerIndex) -> Self {
        let mut foreachsnapshothtmlvisualizer = SForEachSnapshotHTMLVisualizer{file_output, rules, epi};
        foreachsnapshothtmlvisualizer.write_all(
            b"<style>
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
        verify!(self.file_output.write_all(buf)).unwrap() // TODO error handling
    }

    fn output_card(card: SCard, b_border: bool) -> String {
        let (n_width, n_height) = (336 / ESchlag::SIZE.as_num::<isize>(), 232 / EFarbe::SIZE.as_num::<isize>());
        format!(
            "<div style=\"
                margin: 0;
                padding: 0;
                width:{};
                height:{};
                display:inline-block;
                background-image:url(https://www.sauspiel.de/images/redesign/cards/by/card-icons@2x.png);
                background-position-x:{}px;
                background-position-y:{}px;
                border:{};
            \"></div>",
            n_width,
            n_height,
            // Do not use Enum::to_usize. Sauspiel's representation does not necessarily match ours.
            -n_width * match card.schlag() {
                ESchlag::Ass => 0,
                ESchlag::Zehn => 1,
                ESchlag::Koenig => 2,
                ESchlag::Ober => 3,
                ESchlag::Unter => 4,
                ESchlag::S9 => 5,
                ESchlag::S8 => 6,
                ESchlag::S7 => 7,
            },
            -n_height * match card.farbe() {
                EFarbe::Eichel => 0,
                EFarbe::Gras => 1,
                EFarbe::Herz => 2,
                EFarbe::Schelln => 3,
            },
            if b_border {"solid"} else {"none"},
        )
    }

    fn player_table<T: fmt::Display>(epi_self: EPlayerIndex, fn_per_player: impl Fn(EPlayerIndex)->Option<T>) -> String {
        let fn_per_player_internal = move |epi: EPlayerIndex| {
            fn_per_player(epi.wrapping_add(epi_self.to_usize()))
                .map_or("".to_string(), |t| t.to_string())
        };
        format!(
            "<table>
              <tr><td align=\"center\" colspan=\"2\"><br>{}<br></td></tr>
              <tr><td>{}</td><td>{}</td></tr>
              <tr><td align=\"center\" colspan=\"2\">{}</td></tr>
            </table>\n",
            fn_per_player_internal(EPlayerIndex::EPI2),
            fn_per_player_internal(EPlayerIndex::EPI1),
            fn_per_player_internal(EPlayerIndex::EPI3),
            fn_per_player_internal(EPlayerIndex::EPI0),
        )
    }
}

impl TSnapshotVisualizer for SForEachSnapshotHTMLVisualizer<'_> {
    fn begin_snapshot(&mut self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>) {
        let str_item_id = format!("{}{}",
            stichseq.count_played_cards(),
            rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).collect::<String>(), // we simply assume no collisions here
        );
        self.write_all(format!("<li><<input type=\"checkbox\" id=\"{}\" />>\n", str_item_id).as_bytes());
        self.write_all(format!("<label for=\"{}\">{} direct successors<table><tr>\n",
            str_item_id,
            "TODO", // slccard_allowed.len(),
        ).as_bytes());
        //TODO assert!(ahand_vecstich_card_count_is_compatible(slcstich, ahand, ekurzlang));
        for stich in stichseq.visible_stichs() {
            self.write_all(b"<td>\n");
            let epi_0 = self.epi;
            self.write_all(Self::player_table(epi_0, |epi| stich.get(epi).map(|card| Self::output_card(*card, epi==stich.first_playerindex()))).as_bytes());
            self.write_all(b"</td>\n");
        }
        let str_table_hands = format!(
            "<td>{}</td>\n",
            Self::player_table(self.epi, |epi| {
                let mut veccard = ahand[epi].cards().clone();
                self.rules.sort_cards_first_trumpf_then_farbe(veccard.as_mut_slice());
                Some(veccard.into_iter()
                    .map(|card| Self::output_card(card, /*b_border*/false))
                    .join(""))
            }),
        );
        self.write_all(str_table_hands.as_bytes());
        self.write_all(b"</tr></table></label>\n");
        self.write_all(b"<ul>\n");
    }

    fn end_snapshot<Output: fmt::Debug>(&mut self, output: &Output) {
        self.write_all(b"</ul>\n");
        self.write_all(b"</li>\n");
        self.write_all(format!("<p>{:?}</p>\n", output).as_bytes());
    }
}

pub fn explore_snapshots<ForEachSnapshot>(
    epi_self: EPlayerIndex,
    ahand: &mut EnumMap<EPlayerIndex, SHand>,
    rules: &dyn TRules,
    stichseq: &mut SStichSequence,
    func_filter_allowed_cards: &impl Fn(&SStichSequence, &mut SHandVector),
    foreachsnapshot: &ForEachSnapshot,
    ostr_file_out: Option<&str>,
) -> ForEachSnapshot::Output 
    where
        ForEachSnapshot: TForEachSnapshot,
        ForEachSnapshot::Output: fmt::Debug,
{
    macro_rules! forward_to_internal{($snapshotvisualizer: expr) => {
        explore_snapshots_internal(
            epi_self,
            ahand,
            rules,
            &mut SRuleStateCache::new(
                stichseq,
                ahand,
                |stich| rules.winner_index(stich),
            ),
            stichseq,
            func_filter_allowed_cards,
            foreachsnapshot,
            $snapshotvisualizer,
        )
    }}
    if let Some(str_file_out) = ostr_file_out {
        forward_to_internal!(&mut SForEachSnapshotHTMLVisualizer::new(
            verify!(fs::File::create(format!("{}.html", str_file_out))).unwrap(),
            rules,
            epi_self,
        ))
    } else {
        struct SNoVisualization;
        impl TSnapshotVisualizer for SNoVisualization {
            fn begin_snapshot(&mut self, _stichseq: &SStichSequence, _ahand: &EnumMap<EPlayerIndex, SHand>) {}
            fn end_snapshot<Output: fmt::Debug>(&mut self, _output: &Output) {}
        }
        forward_to_internal!(&mut SNoVisualization{})
    }
}

// TODO Maybe something like payout_hint is useful to prune suspicion tree
fn explore_snapshots_internal<ForEachSnapshot>(
    epi_self: EPlayerIndex,
    ahand: &mut EnumMap<EPlayerIndex, SHand>,
    rules: &dyn TRules,
    rulestatecache: &mut SRuleStateCache,
    stichseq: &mut SStichSequence,
    func_filter_allowed_cards: &impl Fn(&SStichSequence, &mut SHandVector),
    foreachsnapshot: &ForEachSnapshot,
    snapshotvisualizer: &mut impl TSnapshotVisualizer,
) -> ForEachSnapshot::Output 
    where
        ForEachSnapshot: TForEachSnapshot,
        ForEachSnapshot::Output : fmt::Debug,
{
    snapshotvisualizer.begin_snapshot(stichseq, &ahand);
    let output = if ahand.iter().all(|hand| hand.cards().is_empty()) {
        foreachsnapshot.final_output(
            SStichSequenceGameFinished::new(stichseq),
            rulestatecache,
        )
    } else {
        foreachsnapshot.pruned_output(stichseq, &ahand, rulestatecache).unwrap_or_else(|| {
            let epi_current = verify!(stichseq.current_stich().current_playerindex()).unwrap();
            let mut veccard_allowed = rules.all_allowed_cards(stichseq, &ahand[epi_current]);
            func_filter_allowed_cards(stichseq, &mut veccard_allowed);
            // TODO? use equivalent card optimization
            foreachsnapshot.combine_outputs(
                epi_self,
                epi_current,
                veccard_allowed.into_iter().map(|card| {
                    ahand[epi_current].play_card(card);
                    let output = stichseq.zugeben_and_restore(card, rules, |stichseq| {
                        macro_rules! next_step {() => {explore_snapshots_internal(
                            epi_self,
                            ahand,
                            rules,
                            rulestatecache,
                            stichseq,
                            func_filter_allowed_cards,
                            foreachsnapshot,
                            snapshotvisualizer,
                        )}};
                        if stichseq.current_stich().is_empty() {
                            let unregisterstich = rulestatecache.register_stich(
                                verify!(stichseq.completed_stichs().last()).unwrap(),
                                stichseq.current_stich().first_playerindex(),
                            );
                            let output = next_step!();
                            rulestatecache.unregister_stich(unregisterstich);
                            output
                        } else {
                            next_step!()
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

fn end_snapshot_minmax<ItTplCardNPayout: Iterator<Item=(SCard, isize)>>(epi_self: EPlayerIndex, epi_card: EPlayerIndex, ittplcardn_payout: ItTplCardNPayout) -> isize {
    verify!(if epi_self==epi_card {
        ittplcardn_payout.max_by_key(|&(_card, n_payout)| n_payout)
    } else {
        ittplcardn_payout.min_by_key(|&(_card, n_payout)| n_payout) // other players may play inconveniently for epi_stich
    }).unwrap().1
}

#[derive(new, Clone)]
pub struct SMinReachablePayoutParams<'rules> {
    rules: &'rules dyn TRules,
    epi: EPlayerIndex,
    tpln_stoss_doubling: (usize, usize),
    n_stock: isize,
}

impl<'rules> SMinReachablePayoutParams<'rules> {
    pub fn new_from_game(game: &'rules SGame) -> Self {
        SMinReachablePayoutParams::new(
            game.rules.as_ref(),
            verify!(game.current_playable_stich().current_playerindex()).unwrap(),
            /*tpln_stoss_doubling*/stoss_and_doublings(&game.vecstoss, &game.doublings),
            game.n_stock,
        )
    }
}

#[derive(Clone)]
pub struct SMinReachablePayout<'rules>(pub SMinReachablePayoutParams<'rules>);

impl TForEachSnapshot for SMinReachablePayout<'_> {
    type Output = isize;

    fn final_output(&self, slcstich: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output {
        self.0.rules.payout_with_cache(slcstich, self.0.tpln_stoss_doubling, self.0.n_stock, rulestatecache, self.0.epi)
    }

    fn pruned_output(&self, _stichseq: &SStichSequence, _ahand: &EnumMap<EPlayerIndex, SHand>, _rulestatecache: &SRuleStateCache) -> Option<Self::Output> {
        None
    }

    fn combine_outputs<ItTplCardOutput: Iterator<Item=(SCard, Self::Output)>>(
        &self,
        epi_self: EPlayerIndex,
        epi_card: EPlayerIndex,
        ittplcardoutput: ItTplCardOutput,
    ) -> Self::Output {
        end_snapshot_minmax(epi_self, epi_card, ittplcardoutput)
    }
}

#[derive(Clone)]
pub struct SMinReachablePayoutLowerBoundViaHint<'rules>(pub SMinReachablePayoutParams<'rules>);

impl TForEachSnapshot for SMinReachablePayoutLowerBoundViaHint<'_> {
    type Output = isize;

    fn final_output(&self, slcstich: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> Self::Output {
        self.0.rules.payout_with_cache(slcstich, self.0.tpln_stoss_doubling, self.0.n_stock, rulestatecache, self.0.epi)
    }

    fn pruned_output(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> Option<Self::Output> {
        self.0.rules.payouthints(stichseq, ahand, rulestatecache, self.0.epi)
            .lower_bound()
            .clone() // TODO really needed?
            .and_then(|payoutinfo| {
                let n_payout = payoutinfo.payout_including_stock(self.0.n_stock, self.0.tpln_stoss_doubling);
                if_then_option!(0<n_payout, n_payout)
            })
    }

    fn combine_outputs<ItTplCardOutput: Iterator<Item=(SCard, Self::Output)>>(
        &self,
        epi_self: EPlayerIndex,
        epi_card: EPlayerIndex,
        ittplcardoutput: ItTplCardOutput,
    ) -> Self::Output {
        end_snapshot_minmax(epi_self, epi_card, ittplcardoutput)
    }
}
