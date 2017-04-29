use primitives::*;
use rules::*;
use itertools::Itertools;
use util::*;

use std::fs;
use std::io::Write;
use std::io;
use std::fmt;

pub struct SSuspicionTransition {
    stich : SStich,
    susp : SSuspicion,
}

pub fn assert_ahand_same_size(ahand: &EnumMap<EPlayerIndex, SHand>) {
    let n_len_hand = ahand[EPlayerIndex::EPI0].cards().len();
    assert!(ahand.iter().all(|hand| hand.cards().len()==n_len_hand)); // TODO itertools: use all_equal
}


pub fn push_pop_vecstich<Func, R>(vecstich: &mut Vec<SStich>, stich: SStich, func: Func) -> R
    where Func: FnOnce(&mut Vec<SStich>) -> R
{
    let n_stich = vecstich.len();
    assert!(vecstich.iter().all(|stich| stich.size()==4));
    vecstich.push(stich);
    let r = func(vecstich);
    vecstich.pop().expect("vecstich unexpectedly empty");
    assert!(vecstich.iter().all(|stich| stich.size()==4));
    assert_eq!(n_stich, vecstich.len());
    r
}

impl SSuspicionTransition {
    pub fn stich(&self) -> &SStich {
        &self.stich
    }

    pub fn suspicion(&self) -> &SSuspicion {
        &self.susp
    }
}

pub struct SSuspicion {
    vecsusptrans : Vec<SSuspicionTransition>,
    ahand : EnumMap<EPlayerIndex, SHand>,
}

impl SSuspicion {

    pub fn suspicion_transitions(&self) -> &[SSuspicionTransition] {
        &self.vecsusptrans
    }

    pub fn new<FuncFilterSuccessors>(
        epi_first: EPlayerIndex,
        ahand: EnumMap<EPlayerIndex, SHand>,
        rules: &TRules,
        vecstich: &mut Vec<SStich>,
        func_filter_successors: &FuncFilterSuccessors,
    ) -> Self 
        where FuncFilterSuccessors : Fn(&[SStich] /*vecstich_complete*/, &mut Vec<SStich>/*vecstich_successor*/)
    {
        let mut vecstich_successor : Vec<SStich> = Vec::new();
        push_pop_vecstich(vecstich, SStich::new(epi_first), |vecstich| {
            let offset_to_playerindex = move |i_offset: usize| {epi_first.wrapping_add(i_offset)};
            macro_rules! traverse_valid_cards {($i_offset : expr, $func: expr) => {
                // TODO use equivalent card optimization
                for card in rules.all_allowed_cards(vecstich, &ahand[offset_to_playerindex($i_offset)]) {
                    vecstich.last_mut().unwrap().push(card);
                    assert_eq!(card, vecstich.last().unwrap()[offset_to_playerindex($i_offset)]);
                    $func;
                    vecstich.last_mut().unwrap().undo_most_recent();
                }
            };};
            traverse_valid_cards!(0, { // TODO: more efficient to explicitly handle first card?
                traverse_valid_cards!(1, {
                    traverse_valid_cards!(2, {
                        traverse_valid_cards!(3, {
                            vecstich_successor.push(vecstich.last().unwrap().clone());
                        } );
                    } );
                } );
            } );
        });
        if !vecstich_successor.is_empty() {
            func_filter_successors(vecstich, &mut vecstich_successor);
            assert!(!vecstich_successor.is_empty());
        }
        let vecsusptrans = vecstich_successor.into_iter()
            .map(|stich| {
                let epi_first_susp = rules.winner_index(&stich);
                push_pop_vecstich(vecstich, stich.clone(), |vecstich| SSuspicionTransition {
                    stich : stich.clone(),
                    susp : SSuspicion::new(
                        epi_first_susp,
                        EPlayerIndex::map_from_fn(|epi| {
                            ahand[epi].new_from_hand(stich[epi])
                        }),
                        rules,
                        vecstich,
                        func_filter_successors
                    ),
                })
            })
            .collect();
        SSuspicion {
            vecsusptrans: vecsusptrans,
            ahand : ahand,
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
        assert_ahand_same_size(&self.ahand);
        self.ahand[EPlayerIndex::EPI0].cards().len()
    }

    fn player_table<T, FnPerPlayer>(fn_per_player: FnPerPlayer) -> String // TODO Rust generic closures
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

    pub fn print_suspicion(
        &self,
        n_level_end: usize,
        n_level: usize,
        rules: &TRules,
        vecstich: &mut Vec<SStich>, // TODO use vecstich or omit parameter
        mut file_output: &mut fs::File,
    ) -> io::Result<()> {
        if n_level < n_level_end {
            file_output.write_all(b"<li>\n")?;
            file_output.write_all(b"<table><tr>\n")?;
            assert_eq!(vecstich.len()+self.hand_size(), 8);
            let output_card = |card: SCard, b_border| {
                let (n_width, n_height) = (336 / ESchlag::ubound_usize().as_num::<isize>(), 232 / EFarbe::ubound_usize().as_num::<isize>());
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
            for stich in vecstich.iter() {
                file_output.write_all(b"<td>\n")?;
                file_output.write_all(Self::player_table(|epi| output_card(stich[epi], epi==stich.first_playerindex())).as_bytes())?;
                file_output.write_all(b"</td>\n")?;
            }
            file_output.write_all(format!(
                "<td>{}</td> <td>{}</td>\n",
                Self::player_table(|epi| {
                    let mut veccard = self.ahand[epi].cards().clone();
                    rules.sort_cards_first_trumpf_then_farbe(veccard.as_mut_slice());
                    veccard.into_iter()
                        .map(|card| output_card(card, /*b_border*/false))
                        .join("")
                }),
                Self::player_table(|_epi| "payout TODO"),
            ).as_bytes())?;
            file_output.write_all(b"</tr></table>\n")?;
            if !&self.vecsusptrans.is_empty() {
                file_output.write_all(b"<ul>\n")?;
                for susptrans in &self.vecsusptrans {
                    push_pop_vecstich(vecstich, susptrans.stich.clone(), |vecstich| {
                        susptrans.susp.print_suspicion(n_level_end, (n_level+1), rules, vecstich, &mut file_output)
                    })?;
                }
                file_output.write_all(b"</ul>\n")?;
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
        ostich_given: Option<SStich>,
        epi: EPlayerIndex,
        n_stoss: usize,
        n_doubling: usize,
        n_stock: isize,
    ) -> isize {
        let vecstich_backup = vecstich.clone();
        assert!(ostich_given.as_ref().map_or(true, |stich| stich.size() < 4));
        assert!(vecstich.iter().all(|stich| stich.size()==4));
        assert_eq!(vecstich.len()+self.hand_size(), 8);
        if 0==self.hand_size() {
            return rules.payout(&SGameFinishedStiche::new(vecstich), n_stoss, n_doubling, n_stock).get_player(epi);
        }
        let n_payout = self.vecsusptrans.iter()
            .filter(|susptrans| { // only consider successors compatible with current stich_given so far
                assert_eq!(susptrans.susp.hand_size()+1, self.hand_size());
                ostich_given.as_ref().map_or(true, |stich_given| {
                    stich_given.iter()
                        .zip(susptrans.stich.iter())
                        .all(|((i_current_stich, card_current_stich), (i_susp_stich, card_susp_stich))| {
                            assert_eq!(i_current_stich, i_susp_stich);
                            card_current_stich==card_susp_stich
                        })
                })
            })
            .map(|susptrans| {
                assert_eq!(susptrans.stich.size(), 4);
                push_pop_vecstich(vecstich, susptrans.stich.clone(), |vecstich| {
                    (susptrans, susptrans.susp.min_reachable_payout(rules, vecstich, None, epi, n_stoss, n_doubling, n_stock))
                })
            })
            .group_by(|&(susptrans, _n_payout)| { // other players may play inconveniently for epi...
                susptrans.stich.iter()
                    .take_while(|&(epi_stich, _card)| epi_stich != epi)
                    .map(|(_epi, card)| card)
                    .collect::<Vec<_>>()
            })
            .into_iter()
            .map(|(_stich_key_before_epi, grpsusptransn_before_epi)| {
                grpsusptransn_before_epi.into_iter()
                    .group_by(|&(susptrans, _n_payout)| susptrans.stich[epi])
                    .into_iter()
                    .map(|(_stich_key_epi, grpsusptransn_epi)| {
                        // in this group, we need the worst case if other players play badly
                        grpsusptransn_epi.into_iter().min_by_key(|&(_susptrans, n_payout)| n_payout).unwrap()
                    })
                    .max_by_key(|&(_susptrans, n_payout)| n_payout)
                    .unwrap()
            })
            .min_by_key(|&(_susptrans, n_payout)| n_payout)
            .unwrap()
            .1;
        assert!(vecstich_backup.iter().zip(vecstich.iter()).all(|(s1,s2)|s1.size()==s2.size()));
        n_payout
    }

}

