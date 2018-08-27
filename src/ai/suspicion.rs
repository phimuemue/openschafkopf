use primitives::*;
use rules::*;
use itertools::Itertools;
use util::*;
use std::{
    fs,
    io::Write,
    io,
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

impl SSuspicion {

    pub fn suspicion_transitions(&self) -> &[SSuspicionTransition] {
        &self.vecsusptrans
    }


    // TODO Maybe something like payout_hint is useful to prune suspicion tree
    pub fn new<FuncFilterSuccessors>(
        ahand: EnumMap<EPlayerIndex, SHand>,
        rules: &TRules,
        vecstich: &mut Vec<SStich>,
        stich_current_model: &SStich,
        func_filter_successors: &FuncFilterSuccessors,
    ) -> Self 
        where FuncFilterSuccessors : Fn(&[SStich] /*vecstich_complete*/, &mut Vec<SStich>/*vecstich_successor*/)
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
        let vecsusptrans = vecstich_successor.into_iter()
            .map(|stich| {
                let epi_first_susp = rules.winner_index(&stich);
                push_pop_vecstich(vecstich, stich.clone(), |vecstich| SSuspicionTransition {
                    stich : stich.clone(),
                    susp : SSuspicion::new(
                        EPlayerIndex::map_from_fn(|epi| {
                            ahand[epi].new_from_hand(stich[epi])
                        }),
                        rules,
                        vecstich,
                        &SStich::new(epi_first_susp),
                        func_filter_successors
                    ),
                })
            })
            .collect();
        SSuspicion {
            vecsusptrans,
            ahand,
        }
    }

    pub fn count_leaves(&self) -> usize {
        if self.vecsusptrans.is_empty() {
            1
        } else {
            self.vecsusptrans.iter()
                .map(|susptrans| susptrans.susp.count_leaves())
                .sum()
        }
    }

    fn hand_size(&self) -> usize {
        hand_size_internal(&self.ahand)
    }

    pub fn print_suspicion(
        &self,
        n_level_end: usize,
        n_level: usize,
        rules: &TRules,
        vecstich: &mut Vec<SStich>,
        mut file_output: &mut fs::File,
    ) -> io::Result<()> {
        if n_level < n_level_end {
            let str_item_id = format!("{}{}",
                n_level,
                rand::thread_rng().sample_iter(&rand::distributions::Alphanumeric).take(16).collect::<String>(), // we simply assume no collisions here
            );
            file_output.write_all(format!("<li><<input type=\"checkbox\" id=\"{}\" />>\n", str_item_id).as_bytes())?;
            file_output.write_all(format!("<label for=\"{}\">{} direct successors<table><tr>\n",
                str_item_id,
                self.vecsusptrans.len(),
            ).as_bytes())?;
            EKurzLang::from_cards_per_player(vecstich.len()+self.hand_size());
            let output_card = |card: SCard, b_border| {
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
            };
            fn player_table<T, FnPerPlayer>(fn_per_player: FnPerPlayer) -> String
                where FnPerPlayer: Fn(EPlayerIndex) -> T,
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
            for stich in vecstich.iter() {
                file_output.write_all(b"<td>\n")?;
                file_output.write_all(player_table(|epi| output_card(stich[epi], epi==stich.first_playerindex())).as_bytes())?;
                file_output.write_all(b"</td>\n")?;
            }
            file_output.write_all(format!(
                "<td>{}</td> <td>{}</td>\n",
                player_table(|epi| {
                    let mut veccard = self.ahand[epi].cards().clone();
                    rules.sort_cards_first_trumpf_then_farbe(veccard.as_mut_slice());
                    veccard.into_iter()
                        .map(|card| output_card(card, /*b_border*/false))
                        .join("")
                }),
                player_table(|epi| self.min_reachable_payout(
                    rules,
                    &mut vecstich.clone(),
                    epi,
                    /*tpln_stoss_doubling*/(0, 0), // dummy values
                    /*n_stock*/0, // dummy value
                ).1),
            ).as_bytes())?;
            file_output.write_all(b"</tr></table></label>\n")?;
            if 1<self.hand_size() {
                file_output.write_all(b"<ul>\n")?;
                for susptrans in &self.vecsusptrans {
                    push_pop_vecstich(vecstich, susptrans.stich.clone(), |vecstich| {
                        susptrans.susp.print_suspicion(n_level_end, n_level+1, rules, vecstich, &mut file_output)
                    })?;
                }
                file_output.write_all(b"</ul>\n")?;
            } else {
                assert_eq!(1, self.vecsusptrans.len());
            }
            file_output.write_all(b"</li>\n")
        } else {
            Ok(())
        }
    }

    pub fn min_reachable_payout(
        &self,
        rules: &TRules,
        vecstich: &mut Vec<SStich>,
        epi: EPlayerIndex,
        tpln_stoss_doubling: (usize, usize),
        n_stock: isize,
    ) -> (SCard, isize) {
        let vecstich_backup = vecstich.clone();
        assert!(vecstich.iter().all(|stich| stich.size()==4));
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
        let tplcardpayout = verify!(self.vecsusptrans.iter()
            .map(|susptrans| {
                assert_eq!(susptrans.stich.size(), 4);
                push_pop_vecstich(vecstich, susptrans.stich.clone(), |vecstich| {
                    (susptrans, susptrans.susp.min_reachable_payout(rules, vecstich, epi, tpln_stoss_doubling, n_stock))
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
            .unwrap();
        assert!(vecstich_backup.iter().zip(vecstich.iter()).all(|(s1,s2)|s1.size()==s2.size()));
        assert!(self.ahand[epi].cards().contains(&tplcardpayout.0));
        tplcardpayout
    }

}

