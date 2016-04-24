pub mod suspicion;

use card::*;
use hand::*;
use rules::*;
use game::*;
use stich::*;
use ai::suspicion::*;

use rand::{self, Rng};
use std::collections::HashMap;
use std::iter::FromIterator;

pub trait TAi {
    fn rank_rules(&self, hand_fixed: &SHand, eplayerindex_fixed: EPlayerIndex, rules: &TRules, n_tests: usize) -> f64;
    fn suggest_card(&self, gamestate: &SGameState) -> SCard;
}

pub fn random_sample_from_vec(vecstich: &mut Vec<SStich>, n_size: usize) {
    let vecstich_sample = rand::sample(&mut rand::thread_rng(), vecstich.iter().cloned(), n_size);
    // TODO can't we just assign to vecstich?
    vecstich.clear();
    for stich in vecstich_sample.into_iter() {
        vecstich.push(stich.clone())
    }
}

pub fn unplayed_cards(vecstich: &Vec<SStich>, hand_fixed: &SHand) -> Vec<Option<SCard>> {
    SCard::all_values().into_iter()
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
    m_eplayerindex_fixed : EPlayerIndex,
    m_ahand: [SHand; 4],
}

impl Iterator for SForeverRandHands {
    type Item = [SHand; 4];
    fn next(&mut self) -> Option<[SHand; 4]> {
        let n_len_hand = self.m_ahand[0].cards().len();
        assert!(self.m_ahand.iter().all(|hand| hand.cards().len()==n_len_hand));
        let mut rng = rand::thread_rng();
        for i_card in 0..3*n_len_hand {
            let i_rand = rng.gen_range(0, 3*n_len_hand - i_card);
            let ((eplayerindex_swap, i_hand_swap), (eplayerindex_rand, i_hand_rand)) = {
                let convert_to_idxs = |i_rand| {
                    // eplayerindex_fixed==0 => 8..31 valid
                    // eplayerindex_fixed==1 => 0..7, 16..31 valid
                    // eplayerindex_fixed==2 => 0..15, 24..31 valid
                    // eplayerindex_fixed==3 => 0..23  valid
                    let i_valid = {
                        if i_rand < self.m_eplayerindex_fixed*n_len_hand {
                            i_rand
                        } else {
                            i_rand + n_len_hand
                        }
                    };
                    (i_valid/n_len_hand, i_valid%n_len_hand)
                };
                (convert_to_idxs(i_card), convert_to_idxs(i_rand))
            };
            {
                let assert_valid = |eplayerindex, i_hand| {
                    assert!(eplayerindex<4);
                    assert!(i_hand<n_len_hand);
                    assert!(eplayerindex!=self.m_eplayerindex_fixed);
                };
                assert_valid(eplayerindex_swap, i_hand_swap);
                assert_valid(eplayerindex_rand, i_hand_rand);
            }
            let card_swap = self.m_ahand[eplayerindex_swap].cards()[i_hand_swap];
            let card_rand = self.m_ahand[eplayerindex_rand].cards()[i_hand_rand];
            *self.m_ahand[eplayerindex_swap].cards_mut().get_mut(i_hand_swap).unwrap() = card_rand;
            *self.m_ahand[eplayerindex_rand].cards_mut().get_mut(i_hand_rand).unwrap() = card_swap;
        }
        Some(create_playerindexmap(|eplayerindex| self.m_ahand[eplayerindex].clone()))
    }
}

fn forever_rand_hands(vecstich: &Vec<SStich>, hand_fixed: SHand, eplayerindex_fixed: EPlayerIndex) -> SForeverRandHands {
    SForeverRandHands {
        m_eplayerindex_fixed : eplayerindex_fixed,
        m_ahand : {
            let mut vecocard = unplayed_cards(vecstich, &hand_fixed);
            assert!(vecocard.iter().filter(|ocard| ocard.is_some()).count()>=3*hand_fixed.cards().len());
            let n_size = hand_fixed.cards().len();
            create_playerindexmap(|eplayerindex| {
                if eplayerindex==eplayerindex_fixed {
                    hand_fixed.clone()
                } else {
                    random_hand(n_size, &mut vecocard)
                }
            })
        }
    }
}

fn suspicion_from_hands_respecting_stich_current(
    rules: &TRules,
    ahand: [SHand; 4],
    mut vecstich_complete_mut: &mut Vec<SStich>,
    stich_current: &SStich,
    n_branches: usize
) -> SSuspicion {
    let n_hand_len = ahand[0].cards().len();
    assert!(ahand.iter().all(|hand| hand.cards().len()==n_hand_len));
    let mut susp = SSuspicion::new_from_raw(stich_current.first_player_index(), ahand);
    let n_stich_complete = vecstich_complete_mut.len();
    susp.compute_successors(
        rules,
        &mut vecstich_complete_mut,
        &|vecstich_complete_successor: &Vec<SStich>, vecstich_successor: &mut Vec<SStich>| {
            assert!(!vecstich_successor.is_empty());
            if vecstich_complete_successor.len()==n_stich_complete {
                vecstich_successor.retain(|stich_successor| {
                    assert!(stich_successor.size()==4);
                    stich_current.equal_up_to_size(stich_successor, stich_current.size())
                });
                assert!(!vecstich_successor.is_empty());
            } else if n_stich_complete < 6 {
                // TODO: maybe keep more than one successor stich
                random_sample_from_vec(vecstich_successor, n_branches);
            } else {
                // if vecstich_complete_successor>=6, we hope that we can compute everything
            }
        }
    );
    assert!(susp.suspicion_tranitions().len() <= susp.count_leaves());
    susp
}

fn possible_payouts(rules: &TRules, susp: &SSuspicion, vecstich_complete_immutable: &Vec<SStich>, eplayerindex_fixed: EPlayerIndex) -> Vec<(SCard, isize)> { // TODO Rust: return iterator
    susp.suspicion_tranitions().iter()
        .map(|susptrans| {
            let mut vecstich_complete_payout = vecstich_complete_immutable.clone();
            let n_payout = push_pop_vecstich(&mut vecstich_complete_payout, susptrans.stich().clone(), |mut vecstich_complete_payout| {
                susptrans.suspicion().min_reachable_payout(
                    rules,
                    &mut vecstich_complete_payout,
                    None,
                    eplayerindex_fixed
                )
            });
            (susptrans.stich()[eplayerindex_fixed], n_payout)
        })
        .collect()
}

pub struct SAiCheating {}

impl TAi for SAiCheating {
    fn rank_rules (&self, hand_fixed: &SHand, eplayerindex_fixed: EPlayerIndex, rules: &TRules, n_tests: usize) -> f64 {
        // TODO: adjust interface to get whole gamestate
        SAiSimulating{}.rank_rules(hand_fixed, eplayerindex_fixed, rules, n_tests)
    }

    fn suggest_card(&self, gamestate: &SGameState) -> SCard {
        let mut vecstich_complete_mut = gamestate.m_vecstich.iter()
            .filter(|stich| stich.size()==4)
            .cloned()
            .collect::<Vec<_>>();
        let stich_current = gamestate.m_vecstich.last().unwrap().clone();
        assert!(stich_current.size()<4);
        let susp = suspicion_from_hands_respecting_stich_current(
            gamestate.m_rules,
            create_playerindexmap(|eplayerindex|
                SHand::new_from_vec(
                    stich_current.get(eplayerindex).into_iter()
                        .chain(gamestate.m_ahand[eplayerindex].cards().iter().cloned())
                        .collect()
                )
            ),
            &mut vecstich_complete_mut,
            &stich_current,
            /*n_branches*/2
        );
        possible_payouts(gamestate.m_rules, &susp, &vecstich_complete_mut, stich_current.current_player_index()).into_iter()
            .max_by_key(|&(_card, n_payout)| n_payout)
            .unwrap()
            .0
    }
}

pub struct SAiSimulating {}

impl TAi for SAiSimulating {
    fn rank_rules (&self, hand_fixed: &SHand, eplayerindex_fixed: EPlayerIndex, rules: &TRules, n_tests: usize) -> f64 {
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

    fn suggest_card(&self, gamestate: &SGameState) -> SCard {
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
        let veccard_allowed_fixed = gamestate.m_rules.all_allowed_cards(&gamestate.m_vecstich, hand_fixed);
        let mapcardpayout = forever_rand_hands(&vecstich_complete_immutable, hand_fixed.clone(), eplayerindex_fixed)
            .filter(|ahand| {
                // hands must contain respective cards from stich_current...
                stich_current.indices_and_cards()
                    .all(|(eplayerindex, card)| ahand[eplayerindex].contains(card))
                // ... and must not contain other cards preventing farbe/trumpf frei
                && {
                    let mut vecstich_complete_and_current_stich = vecstich_complete_immutable.clone();
                    vecstich_complete_and_current_stich.push(SStich::new(stich_current.first_player_index()));
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
            .map(|ahand| suspicion_from_hands_respecting_stich_current(
                gamestate.m_rules,
                ahand,
                &mut vecstich_complete_mut,
                &stich_current,
                /*n_branches*/1
            ))
            .fold(
                // aggregate n_payout per card in some way
                HashMap::from_iter(
                    veccard_allowed_fixed.iter()
                        .map(|card| (card.clone(), 0)) // TODO Option<isize> more convenient?
                ),
                |mut mapcardpayout: HashMap<SCard, isize>, susp| {
                    for (card, n_payout) in possible_payouts(gamestate.m_rules, &susp, &vecstich_complete_immutable, eplayerindex_fixed) {
                        let n_payout_acc = mapcardpayout[&card];
                        *mapcardpayout.get_mut(&card).unwrap() = n_payout_acc + n_payout;
                    }
                    mapcardpayout
                }
            );
        assert!(!hand_fixed.cards().is_empty());
        veccard_allowed_fixed.into_iter()
            .max_by_key(|card| mapcardpayout[card])
            .unwrap()
            .clone()
    }
}
