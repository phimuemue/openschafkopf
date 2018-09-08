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

pub struct SSuspicionTransition {
    stich : SStich,
    susp : SSuspicion,
}

pub fn assert_ahand_same_size(ahand: &EnumMap<EPlayerIndex, SHand>) {
    assert!(ahand.iter().map(|hand| hand.cards().len()).all_equal());
}

pub fn hand_size_internal(ahand: &EnumMap<EPlayerIndex, SHand>) -> usize {
    assert_ahand_same_size(ahand);
    ahand[EPlayerIndex::EPI0].cards().len()
}

pub fn push_pop_vecstich<Func, R>(vecstich: &mut Vec<SStich>, stich: SStich, func: Func) -> R
    where Func: FnOnce(&mut Vec<SStich>) -> R
{
    let n_stich = vecstich.len();
    assert!(vecstich.iter().all(|stich| stich.size()==4));
    vecstich.push(stich);
    let r = func(vecstich);
    verify!(vecstich.pop()).unwrap();
    assert!(vecstich.iter().all(|stich| stich.size()==4));
    assert_eq!(n_stich, vecstich.len());
    r
}

pub struct SSuspicion {
    vecsusptrans : Vec<SSuspicionTransition>,
    ahand : EnumMap<EPlayerIndex, SHand>,
}

pub trait TForEachSnapshot {
    fn begin_snapshot(&mut self, slcstich: SCompletedStichs, ahand: &EnumMap<EPlayerIndex, SHand>, slcstich_successor: &[SStich]);
    fn end_snapshot(&mut self, slcstich: SCompletedStichs, susp: &SSuspicion, slcstich_successor: &[SStich]);
}

pub struct SForEachSnapshotNoop;
impl TForEachSnapshot for SForEachSnapshotNoop {
    fn begin_snapshot(&mut self, _slcstich: SCompletedStichs, _ahand: &EnumMap<EPlayerIndex, SHand>, _slcstich_successor: &[SStich]) {}
    fn end_snapshot(&mut self, _slcstich: SCompletedStichs, _susp: &SSuspicion, _slcstich_successor: &[SStich]) {}
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
}
impl<'rules, 'foreachsnapshot, ForEachSnapshot: TForEachSnapshot> TForEachSnapshot for SForEachSnapshotHTMLVisualizer<'rules, 'foreachsnapshot, ForEachSnapshot> {
    fn begin_snapshot(&mut self, slcstich: SCompletedStichs, ahand: &EnumMap<EPlayerIndex, SHand>, slcstich_successor: &[SStich]) {
        self.foreachsnapshot.begin_snapshot(slcstich, ahand, slcstich_successor);
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

    fn end_snapshot(&mut self, slcstich: SCompletedStichs, susp: &SSuspicion, slcstich_successor: &[SStich]) {
        self.foreachsnapshot.end_snapshot(slcstich, susp, slcstich_successor);
        self.write_all(b"</ul>\n");
        self.write_all(b"</li>\n");
    }
}

impl SSuspicion {
    pub fn new<FuncFilterSuccessors, ForEachSnapshot>(
        ahand: EnumMap<EPlayerIndex, SHand>,
        rules: &TRules,
        vecstich: &mut Vec<SStich>,
        stich_current_model: &SStich,
        func_filter_successors: &FuncFilterSuccessors,
        foreachsnapshot: &mut ForEachSnapshot,
        ostr_file_out: Option<&str>,
    ) -> Self 
        where
            FuncFilterSuccessors : Fn(&[SStich] /*vecstich_complete*/, &mut Vec<SStich>/*vecstich_successor*/),
            ForEachSnapshot: TForEachSnapshot,
    {
        if let Some(str_file_out) = ostr_file_out {
            Self::new_internal(
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
            Self::new_internal(
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
    pub fn new_internal<FuncFilterSuccessors, ForEachSnapshot>(
        ahand: EnumMap<EPlayerIndex, SHand>,
        rules: &TRules,
        vecstich: &mut Vec<SStich>,
        stich_current_model: &SStich,
        func_filter_successors: &FuncFilterSuccessors,
        foreachsnapshot: &mut ForEachSnapshot,
    ) -> Self 
        where
            FuncFilterSuccessors : Fn(&[SStich] /*vecstich_complete*/, &mut Vec<SStich>/*vecstich_successor*/),
            ForEachSnapshot: TForEachSnapshot,
    {
        SCompletedStichs::new(vecstich);
        let mut vecstich_successor : Vec<SStich> = Vec::new();
        if 1<hand_size_internal(&ahand) {
            let epi_first = stich_current_model.first_playerindex();
            push_pop_vecstich(vecstich, SStich::new(epi_first), |vecstich| {
                let offset_to_playerindex = move |i_offset: usize| {epi_first.wrapping_add(i_offset)};
                macro_rules! traverse_valid_cards {($i_offset : expr, $func: expr) => {
                    // TODO use equivalent card optimization
                    for card in rules.all_allowed_cards(vecstich, &ahand[offset_to_playerindex($i_offset)]) {
                        current_stich_mut(vecstich).push(card);
                        assert_eq!(card, current_stich(vecstich)[offset_to_playerindex($i_offset)]);
                        $func;
                        current_stich_mut(vecstich).undo_most_recent();
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
                                let stich_current = current_stich(vecstich);
                                if stich_current.equal_up_to_size(stich_current_model, stich_current_model.size()) {
                                    vecstich_successor.push(stich_current.clone());
                                }
                            } );
                        } );
                    } );
                } );
            });
            assert!(!vecstich_successor.is_empty());
            func_filter_successors(vecstich, &mut vecstich_successor);
            assert!(!vecstich_successor.is_empty());
        }
        foreachsnapshot.begin_snapshot(SCompletedStichs::new(vecstich), &ahand, &vecstich_successor);
        let vecsusptrans = vecstich_successor.iter()
            .map(|stich| {
                let epi_first_susp = rules.winner_index(stich);
                push_pop_vecstich(vecstich, stich.clone(), |vecstich| SSuspicionTransition {
                    stich : stich.clone(),
                    susp : SSuspicion::new_internal(
                        EPlayerIndex::map_from_fn(|epi| {
                            ahand[epi].new_from_hand(stich[epi])
                        }),
                        rules,
                        vecstich,
                        &SStich::new(epi_first_susp),
                        func_filter_successors,
                        foreachsnapshot,
                    ),
                })
            })
            .collect();
        let susp = SSuspicion {
            vecsusptrans,
            ahand,
        };
        foreachsnapshot.end_snapshot(SCompletedStichs::new(vecstich), &susp, &vecstich_successor);
        susp
    }

    fn hand_size(&self) -> usize {
        hand_size_internal(&self.ahand)
    }

    pub fn min_reachable_payout<FuncFilterSuccessors>(
        ahand: EnumMap<EPlayerIndex, SHand>,
        rules: &TRules,
        vecstich: &mut Vec<SStich>,
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
        let vecstich_backup = vecstich.clone();
        assert!(vecstich.iter().all(|stich| stich.size()==4));
        let susp = SSuspicion::new(
            ahand,
            rules,
            vecstich,
            stich_current_model,
            func_filter_successors,
            &mut SForEachSnapshotNoop{},
            ostr_file_out,
        );
        assert_eq!(vecstich, &vecstich_backup);
        let (card, n_payout) = susp.min_reachable_payout_internal(rules, vecstich, epi, tpln_stoss_doubling, n_stock);
        assert!(vecstich_backup.iter().zip(vecstich.iter()).all(|(s1,s2)|s1.size()==s2.size()));
        assert!(susp.ahand[epi].cards().contains(&card));
        (card, n_payout)
    }

    fn min_reachable_payout_internal(
        &self,
        rules: &TRules,
        vecstich: &mut Vec<SStich>,
        epi: EPlayerIndex,
        tpln_stoss_doubling: (usize, usize),
        n_stock: isize,
    ) -> (SCard, isize) {
        if 1==self.hand_size() {
            assert!(!vecstich.is_empty());
            let epi_first = rules.winner_index(current_stich(vecstich));
            return push_pop_vecstich(
                vecstich,
                SStich::new_full(
                    epi_first,
                    EPlayerIndex::map_from_fn(|epi_stich|
                        self.ahand[epi_first.wrapping_add(epi_stich.to_usize())].cards()[0]
                    ).into_raw(),
                ),
                |vecstich| {
                    (
                        self.ahand[epi].cards()[0],
                        rules.payout(
                            SGameFinishedStiche::new(
                                vecstich,
                                EKurzLang::from_cards_per_player(vecstich.len())
                            ),
                            tpln_stoss_doubling,
                            n_stock,
                        ).get_player(epi),
                    )
                },
            );
        }
        verify!(self.vecsusptrans.iter()
            .map(|susptrans| {
                assert_eq!(susptrans.stich.size(), 4);
                push_pop_vecstich(vecstich, susptrans.stich.clone(), |vecstich| {
                    (susptrans, susptrans.susp.min_reachable_payout_internal(rules, vecstich, epi, tpln_stoss_doubling, n_stock))
                })
            })
            .group_by(|&(susptrans, _n_payout)| { // other players may play inconveniently for epi...
                type SStichKeyBeforeEpi = ArrayVec<[SCard; 4]>;
                static_assert!(debug_assert(susptrans.stich.size() <= SStichKeyBeforeEpi::new().capacity()));
                susptrans.stich.iter()
                    .take_while(|&(epi_stich, _card)| epi_stich != epi)
                    .map(|(_epi, card)| *card)
                    .collect::<SStichKeyBeforeEpi>()
            })
            .into_iter()
            .map(|(_stich_key_before_epi, grpsusptransn_before_epi)| {
                verify!(grpsusptransn_before_epi
                    .group_by(|&(susptrans, _n_payout)| susptrans.stich[epi])
                    .into_iter()
                    .map(|(_stich_key_epi, grpsusptransn_epi)| {
                        // in this group, we need the worst case if other players play badly
                        verify!(grpsusptransn_epi.min_by_key(|&(_susptrans, (_card, n_payout))| n_payout)).unwrap()
                    })
                    .max_by_key(|&(_susptrans, (_card, n_payout))| n_payout))
                    .unwrap()
            })
            .min_by_key(|&(_susptrans, (_card, n_payout))| n_payout)
            .map(|(susptrans, (_card, n_payout))| (susptrans.stich[epi], n_payout)))
            .unwrap()
    }

}

