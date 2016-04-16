pub mod suspicion;

use card::*;
use hand::*;
use rules::*;
use game::*;
use stich::*;
use ai::suspicion::*;

use rand;
use std::collections::HashMap;
use std::iter::FromIterator;

pub fn rank_rules (hand_fixed: &CHand, eplayerindex_fixed: EPlayerIndex, rules: &TRules, n_tests: usize) -> f64 {
    (0..n_tests)
        .map(|_i_test| {
            let mut vecocard = unplayed_cards(&Vec::new(), hand_fixed);
            let mut susp = SSuspicion::new_from_raw(
                eplayerindex_fixed,
                create_playerindexmap(|eplayerindex| {
                    if eplayerindex_fixed==eplayerindex {
                        hand_fixed.clone()
                    } else {
                        random_hand(8, &mut vecocard)
                    }
                })
            );
            susp.compute_successors(rules, &mut Vec::new(), &|_vecstich_complete, vecstich_successor| {
                assert!(!vecstich_successor.is_empty());
                random_sample_from_vec(vecstich_successor, 1);
            });
            susp.min_reachable_payout(rules, &mut Vec::new(), None, eplayerindex_fixed)
        })
        .fold(0, |n_payout_acc, n_payout| n_payout_acc+n_payout) as f64
        / n_tests as f64
}

pub fn random_sample_from_vec(vecstich: &mut Vec<CStich>, n_size: usize) {
    let vecstich_sample = rand::sample(&mut rand::thread_rng(), vecstich.iter().cloned(), n_size);
    // TODO can't we just assign to vecstich?
    vecstich.clear();
    for stich in vecstich_sample.into_iter() {
        vecstich.push(stich.clone())
    }
}

pub fn unplayed_cards(vecstich: &Vec<CStich>, hand_fixed: &CHand) -> Vec<Option<CCard>> {
    CCard::all_values().into_iter()
        .map(|card| 
             if 
                 hand_fixed.contains(card)
                 || vecstich.iter().any(|stich|
                    stich.indices_and_cards().any(|(_eplayerindex, card_played)|
                        card_played==card
                    )
                 )
             {
                 None
             } else {
                 Some(card)
             }
        )
        .collect()
}

struct SForeverRandHands {
    m_vecocard : Vec<Option<CCard>>,
    m_eplayerindex_fixed : EPlayerIndex,
    m_hand_fixed : CHand,
}

impl Iterator for SForeverRandHands {
    type Item = [CHand; 4];
    fn next(&mut self) -> Option<[CHand; 4]> {
        let mut vecocard = self.m_vecocard.clone();
        Some(create_playerindexmap(|eplayerindex| {
            if self.m_eplayerindex_fixed==eplayerindex {
                self.m_hand_fixed.clone()
            } else {
                random_hand(self.m_hand_fixed.cards().len(), &mut vecocard)
            }
        }))
    }
}

fn forever_rand_hands(vecstich: &Vec<CStich>, hand_fixed: CHand, eplayerindex_fixed: EPlayerIndex) -> SForeverRandHands {
    SForeverRandHands {
        m_vecocard : {
            let vecocard = unplayed_cards(vecstich, &hand_fixed);
            assert!(vecocard.iter().filter(|ocard| ocard.is_some()).count()>=3*hand_fixed.cards().len());
            vecocard
        },
        m_eplayerindex_fixed : eplayerindex_fixed,
        m_hand_fixed : hand_fixed,
    }
}

pub fn suggest_card(gamestate: &SGameState) -> CCard {
    let n_tests = 100;
    let mut vecstich_complete_mut = gamestate.m_vecstich.iter()
        .filter(|stich| stich.size()==4)
        .cloned()
        .collect::<Vec<_>>();
    let vecstich_complete_immutable = vecstich_complete_mut.clone();
    let stich_current = gamestate.m_vecstich.last().unwrap().clone();
    assert!(stich_current.size()<4);
    let eplayerindex_fixed = stich_current.current_player_index();
    let ref hand_fixed = gamestate.m_ahand[eplayerindex_fixed];
    let mapcardpayout = forever_rand_hands(&vecstich_complete_immutable, hand_fixed.clone(), eplayerindex_fixed)
        .filter(|ahand| {
            // hands must contain respective cards from stich_current...
            stich_current.indices_and_cards()
                .all(|(eplayerindex, card)| ahand[eplayerindex].contains(card))
            // ... and must not contain other cards preventing farbe/trumpf frei
            && {
                let mut vecstich_complete_and_current_stich = vecstich_complete_immutable.clone();
                vecstich_complete_and_current_stich.push(CStich::new(stich_current.first_player_index()));
                stich_current.indices_and_cards()
                    .all(|(eplayerindex, card_played)| {
                        let b_valid = gamestate.m_rules.card_is_allowed(
                            &vecstich_complete_and_current_stich,
                            &ahand[eplayerindex],
                            card_played
                        );
                        vecstich_complete_and_current_stich.last_mut().unwrap().zugeben(card_played);
                        b_valid
                    })
            }
        })
        .take(n_tests)
        .map(|ahand| {
            let mut susp = SSuspicion::new_from_raw(stich_current.first_player_index(), ahand);
            susp.compute_successors(
                gamestate.m_rules,
                &mut vecstich_complete_mut,
                &|vecstich_complete_successor: &Vec<CStich>, vecstich_successor: &mut Vec<CStich>| {
                    assert!(!vecstich_successor.is_empty());
                    if vecstich_complete_successor.len()==vecstich_complete_immutable.len() {
                        assert!(vecstich_complete_successor.iter().eq(vecstich_complete_immutable.iter()));
                        vecstich_successor.retain(|stich_successor| {
                            assert!(stich_successor.size()==4);
                            stich_current.equal_up_to_size(stich_successor, stich_current.size())
                        });
                        assert!(!vecstich_successor.is_empty());
                    } else if vecstich_complete_successor.len() < 6 {
                        // TODO: maybe keep more than one successor stich
                        random_sample_from_vec(vecstich_successor, 1);
                    } else {
                        // if vecstich_complete_successor>=6, we hope that we can compute everything
                    }
                }
            );
            susp
        })
        .fold(
            // aggregate n_payout per card in some way
            HashMap::from_iter(
                gamestate.m_rules.all_allowed_cards(&gamestate.m_vecstich, hand_fixed).into_iter()
                    .map(|card| (card, 0)) // TODO Option<isize> more convenient?
            ),
            |mut mapcardpayout: HashMap<CCard, isize>, susp| {
                for susptrans in susp.suspicion_tranitions() {
                    let mut vecstich_complete_payout = vecstich_complete_immutable.clone();
                    let n_payout = push_pop_vecstich(&mut vecstich_complete_payout, susptrans.stich().clone(), |mut vecstich_complete_payout| {
                        susptrans.suspicion().min_reachable_payout(
                            gamestate.m_rules,
                            &mut vecstich_complete_payout,
                            None,
                            eplayerindex_fixed
                        )
                    });
                    let card = susptrans.stich()[eplayerindex_fixed];
                    let n_payout_acc = mapcardpayout[&card];
                    *mapcardpayout.get_mut(&card).unwrap() = n_payout_acc + n_payout;
                }
                mapcardpayout
            }
        );
    assert!(!hand_fixed.cards().is_empty());
    gamestate.m_rules.all_allowed_cards(&gamestate.m_vecstich, hand_fixed).into_iter()
        .max_by_key(|card| mapcardpayout[card])
        .unwrap()
        .clone()
}
