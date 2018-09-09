use primitives::*;
use rules::*;
use itertools::Itertools;
use util::*;
use std::{
    fs,
    io::Write,
    fmt,
};
use rand::{
    self,
    Rng,
};
use arrayvec::ArrayVec;

pub fn assert_ahand_same_size(ahand: &EnumMap<EPlayerIndex, SHand>) {
    assert!(ahand.iter().map(|hand| hand.cards().len()).all_equal());
}

pub fn hand_size_internal(ahand: &EnumMap<EPlayerIndex, SHand>) -> usize {
    assert_ahand_same_size(ahand);
    ahand[EPlayerIndex::EPI0].cards().len()
}

pub trait TForEachSnapshot {
    type Output;
    fn pruned_output(&mut self, vecstich: &mut SVecStichPushPop, ahand: &EnumMap<EPlayerIndex, SHand>) -> Option<Self::Output>;
    fn begin_snapshot(&mut self, slcstich: SCompletedStichs, ahand: &EnumMap<EPlayerIndex, SHand>, slcstich_successor: &[SStich]);
    fn end_snapshot<ItTplStichOutput: Iterator<Item=(SStich, Self::Output)>>(
        &mut self,
        ittplstichoutput: ItTplStichOutput,
    ) -> Self::Output;
}

pub struct SForEachSnapshotHTMLVisualizer<'rules, 'foreachsnapshot, ForEachSnapshot: TForEachSnapshot + 'foreachsnapshot> {
    file_output: fs::File,
    rules: &'rules TRules,
    foreachsnapshot: &'foreachsnapshot mut ForEachSnapshot
}
impl<'rules, 'foreachsnapshot, ForEachSnapshot: TForEachSnapshot> SForEachSnapshotHTMLVisualizer<'rules, 'foreachsnapshot, ForEachSnapshot> {
    pub fn new(file_output: fs::File, rules: &'rules TRules, foreachsnapshot: &'foreachsnapshot mut ForEachSnapshot) -> Self {
        let mut foreachsnapshothtmlvisualizer = SForEachSnapshotHTMLVisualizer{file_output, rules, foreachsnapshot};
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

    fn player_table<T, FnPerPlayer>(fn_per_player: FnPerPlayer) -> String
        where
            FnPerPlayer: Fn(EPlayerIndex) -> T,
            T: fmt::Display,
    {
        format!(
            "<table>
              <tr><td align=\"center\" colspan=\"2\"><br>{}<br></td></tr>
              <tr><td>{}</td><td>{}</td></tr>
              <tr><td align=\"center\" colspan=\"2\">{}</td></tr>
            </table>\n",
            fn_per_player(EPlayerIndex::EPI2),
            fn_per_player(EPlayerIndex::EPI1),
            fn_per_player(EPlayerIndex::EPI3),
            fn_per_player(EPlayerIndex::EPI0),
        )
    }

    fn end_snapshot_internal(&mut self) {
        self.write_all(b"</ul>\n");
        self.write_all(b"</li>\n");
    }

    fn begin_snapshot_internal(&mut self, slcstich: SCompletedStichs, ahand: &EnumMap<EPlayerIndex, SHand>, slcstich_successor: &[SStich]) {
        let str_item_id = format!("{}{}",
            slcstich.get().len(),
            rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).collect::<String>(), // we simply assume no collisions here
        );
        self.write_all(format!("<li><<input type=\"checkbox\" id=\"{}\" />>\n", str_item_id).as_bytes());
        self.write_all(format!("<label for=\"{}\">{} direct successors<table><tr>\n",
            str_item_id,
            slcstich_successor.len(),
        ).as_bytes());
        EKurzLang::from_cards_per_player(slcstich.get().len()+hand_size_internal(ahand));
        for stich in slcstich.get().iter() {
            self.write_all(b"<td>\n");
            self.write_all(Self::player_table(|epi| Self::output_card(stich[epi], epi==stich.first_playerindex())).as_bytes());
            self.write_all(b"</td>\n");
        }
        let str_table_hands = format!(
            "<td>{}</td> <td>TODO min_reachable_payout</td>\n", // TODO how to output min_reachable_payout?
            Self::player_table(|epi| {
                let mut veccard = ahand[epi].cards().clone();
                self.rules.sort_cards_first_trumpf_then_farbe(veccard.as_mut_slice());
                veccard.into_iter()
                    .map(|card| Self::output_card(card, /*b_border*/false))
                    .join("")
            }),
            // Self::player_table(|epi| self.min_reachable_payout(
            //     self.rules,
            //     &mut slcstich.clone(),
            //     epi,
            //     /*tpln_stoss_doubling*/(0, 0), // dummy values
            //     /*n_stock*/0, // dummy value
            // ).1),
        );
        self.write_all(str_table_hands.as_bytes());
        self.write_all(b"</tr></table></label>\n");
        self.write_all(b"<ul>\n");
    }
}
impl<'rules, 'foreachsnapshot, ForEachSnapshot: TForEachSnapshot> TForEachSnapshot for SForEachSnapshotHTMLVisualizer<'rules, 'foreachsnapshot, ForEachSnapshot> {
    type Output = ForEachSnapshot::Output;

    fn pruned_output(&mut self, vecstich: &mut SVecStichPushPop, ahand: &EnumMap<EPlayerIndex, SHand>) -> Option<Self::Output> {
        if let Some(output) = self.foreachsnapshot.pruned_output(vecstich, ahand) {
            self.begin_snapshot_internal(vecstich.get(), ahand, &Vec::new());
            self.end_snapshot_internal();
            Some(output)
        } else {
            None
        }
    }

    fn begin_snapshot(&mut self, slcstich: SCompletedStichs, ahand: &EnumMap<EPlayerIndex, SHand>, slcstich_successor: &[SStich]) {
        self.begin_snapshot_internal(slcstich, ahand, slcstich_successor);
        self.foreachsnapshot.begin_snapshot(slcstich, ahand, slcstich_successor)
    }

    fn end_snapshot<ItTplStichOutput: Iterator<Item=(SStich, Self::Output)>>(
        &mut self,
        ittplstichoutput: ItTplStichOutput,
    ) -> Self::Output {
        self.end_snapshot_internal();
        self.foreachsnapshot.end_snapshot(ittplstichoutput)
    }
}

pub fn explore_snapshots<FuncFilterSuccessors, ForEachSnapshot>(
    ahand: EnumMap<EPlayerIndex, SHand>,
    rules: &TRules,
    vecstich: &mut SVecStichPushPop,
    stich_current_model: &SStich,
    func_filter_successors: &FuncFilterSuccessors,
    foreachsnapshot: &mut ForEachSnapshot,
    ostr_file_out: Option<&str>,
) -> ForEachSnapshot::Output 
    where
        FuncFilterSuccessors : Fn(&[SStich] /*vecstich_complete*/, &mut Vec<SStich>/*vecstich_successor*/),
        ForEachSnapshot: TForEachSnapshot,
{
    if let Some(str_file_out) = ostr_file_out {
        explore_snapshots_internal(
            ahand,
            rules,
            vecstich,
            stich_current_model,
            func_filter_successors,
            &mut SForEachSnapshotHTMLVisualizer::new(
                verify!(fs::File::create(format!("{}.html", str_file_out))).unwrap(),
                rules,
                foreachsnapshot,
            ),
        )
    } else {
        explore_snapshots_internal(
            ahand,
            rules,
            vecstich,
            stich_current_model,
            func_filter_successors,
            foreachsnapshot,
        )
    }
}

// TODO Maybe something like payout_hint is useful to prune suspicion tree
fn explore_snapshots_internal<FuncFilterSuccessors, ForEachSnapshot>(
    ahand: EnumMap<EPlayerIndex, SHand>,
    rules: &TRules,
    vecstich: &mut SVecStichPushPop,
    stich_current_model: &SStich,
    func_filter_successors: &FuncFilterSuccessors,
    foreachsnapshot: &mut ForEachSnapshot,
) -> ForEachSnapshot::Output 
    where
        FuncFilterSuccessors : Fn(&[SStich] /*vecstich_complete*/, &mut Vec<SStich>/*vecstich_successor*/),
        ForEachSnapshot: TForEachSnapshot,
{
    if let Some(output) = foreachsnapshot.pruned_output(vecstich, &ahand) {
        output
    } else {
        let mut vecstich_successor : Vec<SStich> = Vec::new();
        assert!(1<hand_size_internal(&ahand)); // otherwise, pruned_output must return Some(output)
        {
            let epi_first = stich_current_model.first_playerindex();
            vecstich.push_pop(SStich::new(epi_first), |mut vecstich| {
                let offset_to_playerindex = move |i_offset: usize| {epi_first.wrapping_add(i_offset)};
                macro_rules! traverse_valid_cards {($i_offset : expr, $func: expr) => {
                    // TODO use equivalent card optimization
                    for card in rules.all_allowed_cards(vecstich.get(), &ahand[offset_to_playerindex($i_offset)]) {
                        current_stich_mut(vecstich.get_mut()).push(card);
                        assert_eq!(card, current_stich(vecstich.get())[offset_to_playerindex($i_offset)]);
                        $func;
                        current_stich_mut(vecstich.get_mut()).undo_most_recent();
                    }
                };};
                // It seems that this nested loop is the ai's bottleneck
                // because it is currently designed to be generic for all rules.
                // It may be more performant to have TRules::all_possible_stichs
                // so that we can implement rule-specific optimizations.
                traverse_valid_cards!(0, {
                    traverse_valid_cards!(1, {
                        traverse_valid_cards!(2, {
                            traverse_valid_cards!(3, {
                                let stich_current = current_stich(vecstich.get());
                                if stich_current.equal_up_to_size(stich_current_model, stich_current_model.size()) {
                                    vecstich_successor.push(stich_current.clone());
                                }
                            } );
                        } );
                    } );
                } );
            });
            assert!(!vecstich_successor.is_empty());
            func_filter_successors(vecstich.get().get(), &mut vecstich_successor);
            assert!(!vecstich_successor.is_empty());
        }
        foreachsnapshot.begin_snapshot(vecstich.get(), &ahand, &vecstich_successor);
        let vectplstichoutput = vecstich_successor.into_iter()
            .map(|stich| {
                let output_successor = vecstich.push_pop(stich.clone(), |vecstich| {
                    explore_snapshots_internal(
                        EPlayerIndex::map_from_fn(|epi| {
                            ahand[epi].new_from_hand(stich[epi])
                        }),
                        rules,
                        &mut vecstich.to_pushpop(),
                        &SStich::new(rules.winner_index(&stich)),
                        func_filter_successors,
                        foreachsnapshot,
                    )
                });
                (stich, output_successor)
            })
            .collect::<Vec<_>>();
        foreachsnapshot.end_snapshot(vectplstichoutput.into_iter())
    }
}

pub fn min_reachable_payout<FuncFilterSuccessors>(
    ahand: EnumMap<EPlayerIndex, SHand>,
    rules: &TRules,
    vecstich: &mut SVecStichPushPop,
    stich_current_model: &SStich,
    func_filter_successors: &FuncFilterSuccessors,
    epi: EPlayerIndex,
    tpln_stoss_doubling: (usize, usize),
    n_stock: isize,
    ostr_file_out: Option<&str>,
) -> (SCard, isize)
    where
        FuncFilterSuccessors : Fn(&[SStich] /*vecstich_complete*/, &mut Vec<SStich>/*vecstich_successor*/),
{
    let ahand_backup = ahand.clone();
    assert!(vecstich.get().get().iter().all(|stich| stich.size()==4));
    let (card, n_payout) = explore_snapshots(
        ahand,
        rules,
        vecstich,
        stich_current_model,
        func_filter_successors,
        &mut SMinReachablePayout{
            rules,
            epi,
            tpln_stoss_doubling,
            n_stock,
        },
        ostr_file_out,
    );
    assert!(ahand_backup[epi].cards().contains(&card));
    (card, n_payout)
}

struct SMinReachablePayout<'rules> {
    rules: &'rules TRules,
    epi: EPlayerIndex,
    tpln_stoss_doubling: (usize, usize),
    n_stock: isize,
}

impl<'rules> TForEachSnapshot for SMinReachablePayout<'rules> {
    type Output = (SCard, isize);

    fn pruned_output(&mut self, vecstich: &mut SVecStichPushPop, ahand: &EnumMap<EPlayerIndex, SHand>) -> Option<Self::Output> {
        if 1==hand_size_internal(ahand) {
            assert!(!vecstich.get().get().is_empty());
            let epi_first = self.rules.winner_index(current_stich(vecstich.get().get()));
            return vecstich.push_pop(
                SStich::new_full(
                    epi_first,
                    EPlayerIndex::map_from_fn(|epi_stich|
                        ahand[epi_first.wrapping_add(epi_stich.to_usize())].cards()[0]
                    ).into_raw(),
                ),
                |vecstich| {
                    Some((
                        ahand[self.epi].cards()[0],
                        self.rules.payout(
                            SGameFinishedStiche::new(
                                vecstich.get(),
                                EKurzLang::from_cards_per_player(vecstich.get().len())
                            ),
                            self.tpln_stoss_doubling,
                            self.n_stock,
                        ).get_player(self.epi),
                    ))
                },
            );
        } else {
            assert!(0 < hand_size_internal(ahand));
            None
        }
    }

    fn begin_snapshot(&mut self, _slcstich: SCompletedStichs, _ahand: &EnumMap<EPlayerIndex, SHand>, _slcstich_successor: &[SStich]) {
    }

    fn end_snapshot<ItTplStichOutput: Iterator<Item=(SStich, Self::Output)>>(
        &mut self,
        ittplstichoutput: ItTplStichOutput,
    ) -> Self::Output {
        verify!(ittplstichoutput
            .map(|(stich, output)| {
                assert_eq!(stich.size(), 4);
                (stich, output)
            })
            .group_by(|&(ref stich, _n_payout)| { // other players may play inconveniently for epi...
                type SStichKeyBeforeEpi = ArrayVec<[SCard; 4]>;
                static_assert!(debug_assert(stich.size() <= SStichKeyBeforeEpi::new().capacity()));
                stich.iter()
                    .take_while(|&(epi_stich, _card)| epi_stich != self.epi)
                    .map(|(_epi, card)| *card)
                    .collect::<SStichKeyBeforeEpi>()
            })
            .into_iter()
            .map(|(_stich_key_before_epi, grpstichn_before_epi)| {
                verify!(grpstichn_before_epi
                    .group_by(|&(ref stich, _n_payout)| stich[self.epi])
                    .into_iter()
                    .map(|(_stich_key_epi, grpstichn_epi)| {
                        // in this group, we need the worst case if other players play badly
                        verify!(grpstichn_epi.min_by_key(|&(ref _stich, (_card, n_payout))| n_payout)).unwrap()
                    })
                    .max_by_key(|&(ref _stich, (_card, n_payout))| n_payout))
                    .unwrap()
            })
            .min_by_key(|&(ref _stich, (_card, n_payout))| n_payout)
            .map(|(stich, (_card, n_payout))| (stich[self.epi], n_payout)))
            .unwrap()
    }
}

