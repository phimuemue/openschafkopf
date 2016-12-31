pub mod suspicion;
pub mod handiterators;

use primitives::*;
use rules::*;
use game::*;
use ai::suspicion::*;
use ai::handiterators::*;

use rand;
use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::fs;
use std::mem;
use crossbeam;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicIsize, Ordering};
use std::cmp;

pub trait TAi {
    fn rank_rules(&self, hand_fixed: &SFullHand, eplayerindex_first: EPlayerIndex, eplayerindex_rank: EPlayerIndex, rules: &TRules, n_stock: isize) -> f64;
    fn suggest_card(&self, game: &SGame) -> SCard {
        let veccard_allowed = game.m_rules.all_allowed_cards(
            &game.m_vecstich,
            &game.m_ahand[game.which_player_can_do_something().unwrap()]
        );
        assert!(1<=veccard_allowed.len());
        if 1==veccard_allowed.len() {
            veccard_allowed[0]
        } else {
            self.internal_suggest_card(game)
        }
    }
    fn internal_suggest_card(&self, game: &SGame) -> SCard;
}

pub fn random_sample_from_vec(vecstich: &mut Vec<SStich>, n_size: usize) {
    let mut vecstich_sample = rand::sample(&mut rand::thread_rng(), vecstich.iter().cloned(), n_size);
    mem::swap(vecstich, &mut vecstich_sample);
}

pub fn unplayed_cards(vecstich: &[SStich], hand_fixed: &SHand) -> Vec<SCard> {
    SCard::values().into_iter()
        .filter(|card| 
             !hand_fixed.contains(*card)
             && !vecstich.iter().any(|stich|
                stich.iter().any(|(_eplayerindex, card_played)|
                    card_played==card
                )
             )
        )
        .collect()
}

#[test]
fn test_unplayed_cards() {
    use util::cardvectorparser;
    let vecstich = ["g7 g8 ga g9", "s8 ho s7 s9", "h7 hk hu su", "eo go hz h8", "e9 ek e8 ea", "sa eu so ha"].into_iter()
        .map(|str_stich| {
            let mut stich = SStich::new(/*eplayerindex should not be relevant*/0);
            for card in cardvectorparser::parse_cards::<Vec<_>>(str_stich).unwrap() {
                stich.push(card.clone());
            }
            stich
        })
        .collect::<Vec<_>>();
    let veccard_unplayed = unplayed_cards(
        &vecstich,
        &SHand::new_from_vec(cardvectorparser::parse_cards("gk sk").unwrap())
    );
    let veccard_unplayed_check = cardvectorparser::parse_cards::<Vec<_>>("gz e7 sz h9 ez gu").unwrap();
    assert_eq!(veccard_unplayed.len(), veccard_unplayed_check.len());
    assert!(veccard_unplayed.iter().all(|card| veccard_unplayed_check.contains(card)));
    assert!(veccard_unplayed_check.iter().all(|card| veccard_unplayed.contains(card)));
}

pub struct SAiCheating {
    m_n_rank_rules_samples: usize,
}

impl SAiCheating {
    pub fn new(n_rank_rules_samples: usize) -> SAiCheating {
        SAiCheating {
            m_n_rank_rules_samples: n_rank_rules_samples,
        }
    }
}

impl TAi for SAiCheating {
    fn rank_rules (&self, hand_fixed: &SFullHand, eplayerindex_first: EPlayerIndex, eplayerindex_rank: EPlayerIndex, rules: &TRules, n_stock: isize) -> f64 {
        // TODO: adjust interface to get whole game
        SAiSimulating::new(
            /*n_suggest_card_branches*/2,
            /*n_suggest_card_samples*/10,
            self.m_n_rank_rules_samples,
        ).rank_rules(hand_fixed, eplayerindex_first, eplayerindex_rank, rules, n_stock)
    }

    fn internal_suggest_card(&self, game: &SGame) -> SCard {
        determine_best_card(
            game,
            Some(create_playerindexmap(|eplayerindex|
                SHand::new_from_vec(
                    game.current_stich().get(eplayerindex).cloned().into_iter()
                        .chain(game.m_ahand[eplayerindex].cards().iter().cloned())
                        .collect()
                )
            )).into_iter(),
            /*n_branches*/1,
        )
    }
}

pub fn is_compatible_with_game_so_far(
    ahand: &SPlayerIndexMap<SHand>,
    rules: &TRules,
    vecstich: &[SStich],
) -> bool {
    let stich_current = current_stich(vecstich);
    assert!(stich_current.size()<4);
    // hands must contain respective cards from stich_current...
    stich_current.iter()
        .all(|(eplayerindex, card)| ahand[eplayerindex].contains(*card))
    // ... and must not contain other cards preventing farbe/trumpf frei
    && {
        let mut vecstich_complete_and_current_stich = completed_stichs(vecstich).iter().cloned().collect::<Vec<_>>();
        vecstich_complete_and_current_stich.push(SStich::new(stich_current.first_playerindex()));
        stich_current.iter()
            .all(|(eplayerindex, card_played)| {
                let b_valid = rules.card_is_allowed(
                    &vecstich_complete_and_current_stich,
                    &ahand[eplayerindex],
                    *card_played
                );
                vecstich_complete_and_current_stich.last_mut().unwrap().push(*card_played);
                b_valid
            })
    }
    && {
        assert_ahand_same_size(ahand);
        let mut ahand_simulate = create_playerindexmap(|eplayerindex| {
            ahand[eplayerindex].clone()
        });
        for stich in completed_stichs(vecstich).iter().rev() {
            for eplayerindex in eplayerindex_values() {
                ahand_simulate[eplayerindex].cards_mut().push(stich[eplayerindex]);
            }
        }
        assert_ahand_same_size(&ahand_simulate);
        rules.playerindex().map_or(true, |eplayerindex_active|
            rules.can_be_played(&SFullHand::new(&ahand_simulate[eplayerindex_active]))
        )
        && {
            let mut b_valid_up_to_now = true;
            let mut vecstich_simulate = Vec::new();
            'loopstich: for stich in completed_stichs(vecstich).iter() {
                vecstich_simulate.push(SStich::new(stich.m_eplayerindex_first));
                for (eplayerindex, card) in stich.iter() {
                    if rules.card_is_allowed(
                        &vecstich_simulate,
                        &ahand_simulate[eplayerindex],
                        *card
                    ) {
                        assert!(ahand_simulate[eplayerindex].contains(*card));
                        ahand_simulate[eplayerindex].play_card(*card);
                        vecstich_simulate.last_mut().unwrap().push(*card);
                    } else {
                        b_valid_up_to_now = false;
                        break 'loopstich;
                    }
                }
            }
            b_valid_up_to_now
        }
    }
}

fn determine_best_card<HandsIterator>(game: &SGame, itahand: HandsIterator, n_branches: usize) -> SCard
    where HandsIterator: Iterator<Item=SPlayerIndexMap<SHand>>
{
    let stich_current = game.current_stich();
    let eplayerindex_fixed = stich_current.current_playerindex().unwrap();
    let vecsusp = Arc::new(Mutex::new(Vec::new()));
    crossbeam::scope(|scope| {
        for ahand in itahand {
            let vecsusp = vecsusp.clone();
            scope.spawn(move || {
                assert_ahand_same_size(&ahand);
                let mut vecstich_complete_mut = game.completed_stichs().iter().cloned().collect::<Vec<_>>();
                let n_stich_complete = vecstich_complete_mut.len();
                let susp = SSuspicion::new(
                    stich_current.first_playerindex(),
                    ahand,
                    game.m_rules,
                    &mut vecstich_complete_mut,
                    &|vecstich_complete_successor: &[SStich], vecstich_successor: &mut Vec<SStich>| {
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
                assert!(susp.suspicion_transitions().len() <= susp.count_leaves());
                if susp.print_suspicion(8, 0, game.m_rules, &mut vecstich_complete_mut, Some((*stich_current).clone()), &mut fs::File::create(&"suspicion.txt").unwrap()).is_err() {
                    // TODO: what shall be done on error?
                }
                vecsusp.lock().unwrap().push(susp);
            });
        }
    });
    let veccard_allowed_fixed = game.m_rules.all_allowed_cards(&game.m_vecstich, &game.m_ahand[eplayerindex_fixed]);
    let mapcardpayout = vecsusp.lock().unwrap().iter()
        .fold(
            // aggregate n_payout per card in some way
            HashMap::new(),
            |mut mapcardpayout: HashMap<SCard, isize>, susp| {
                let mut vecstich_complete_payout = game.completed_stichs().iter().cloned().collect();
                for (card, n_payout) in susp.suspicion_transitions().iter()
                    .map(|susptrans| {
                        let n_payout = push_pop_vecstich(&mut vecstich_complete_payout, susptrans.stich().clone(), |mut vecstich_complete_payout| {
                            susptrans.suspicion().min_reachable_payout(
                                game.m_rules,
                                &mut vecstich_complete_payout,
                                None,
                                eplayerindex_fixed,
                                /*n_stoss*/ game.m_vecstoss.len(),
                                /*n_doubling*/ game.m_doublings.iter().filter(|&(_eplayerindex, &b_doubling)| b_doubling).count(),
                                game.m_n_stock,
                            )
                        });
                        (susptrans.stich()[eplayerindex_fixed], n_payout)
                    })
                {
                    match mapcardpayout.entry(card) {
                        Entry::Occupied(mut occentry) => {
                            let n_payout_acc = *occentry.get();
                            occentry.insert(cmp::min(n_payout_acc, n_payout));
                        }
                        Entry::Vacant(vacentry) => {
                            vacentry.insert(n_payout);
                        }
                    }
                    assert!(!mapcardpayout.is_empty());
                }
                mapcardpayout
            }
        );
    veccard_allowed_fixed.into_iter()
        .max_by_key(|card| mapcardpayout[card])
        .unwrap()
}

pub struct SAiSimulating {
    m_n_suggest_card_branches: usize,
    m_n_suggest_card_samples: usize,
    m_n_rank_rules_samples: usize,
}
impl SAiSimulating {
    pub fn new(n_suggest_card_branches: usize, n_suggest_card_samples: usize, n_rank_rules_samples: usize) -> SAiSimulating {
        SAiSimulating {
            m_n_suggest_card_branches: n_suggest_card_branches,
            m_n_suggest_card_samples: n_suggest_card_samples,
            m_n_rank_rules_samples: n_rank_rules_samples,
        }
    }
}
impl TAi for SAiSimulating {
    fn rank_rules (&self, hand_fixed: &SFullHand, eplayerindex_first: EPlayerIndex, eplayerindex_rank: EPlayerIndex, rules: &TRules, n_stock: isize) -> f64 {
        let n_payout_sum = Arc::new(AtomicIsize::new(0));
        crossbeam::scope(|scope| {
            for ahand in forever_rand_hands(/*vecstich*/&Vec::new(), hand_fixed.get().clone(), eplayerindex_rank).take(self.m_n_rank_rules_samples) {
                let n_payout_sum = n_payout_sum.clone();
                scope.spawn(move || {
                    let n_payout = 
                        SSuspicion::new(
                            eplayerindex_first,
                            ahand,
                            rules,
                            &mut Vec::new(),
                            |_vecstich_complete, vecstich_successor| {
                                assert!(!vecstich_successor.is_empty());
                                random_sample_from_vec(vecstich_successor, 1);
                            }
                        ).min_reachable_payout(
                            rules,
                            &mut Vec::new(),
                            None,
                            eplayerindex_rank,
                            /*n_stoss*/ 0, // TODO do we need n_stoss from somewhere?
                            /*n_doubling*/ 0, // TODO do we need n_doubling from somewhere?
                            n_stock,
                        )
                    ;
                    n_payout_sum.fetch_add(n_payout, Ordering::SeqCst);
                });
            }
        });
        let n_payout_sum = n_payout_sum.load(Ordering::SeqCst);
        (n_payout_sum as f64) / (self.m_n_rank_rules_samples as f64)
    }

    fn internal_suggest_card(&self, game: &SGame) -> SCard {
        let stich_current = game.current_stich();
        assert!(stich_current.size()<4);
        let eplayerindex_fixed = stich_current.current_playerindex().unwrap();
        let hand_fixed = &game.m_ahand[eplayerindex_fixed];
        assert!(!hand_fixed.cards().is_empty());
        if hand_fixed.cards().len()<=2 {
            determine_best_card(
                game,
                all_possible_hands(game.completed_stichs(), hand_fixed.clone(), eplayerindex_fixed)
                    .filter(|ahand| is_compatible_with_game_so_far(ahand, game.m_rules, &game.m_vecstich)),
                self.m_n_suggest_card_branches,
            )
        } else {
            determine_best_card(
                game,
                forever_rand_hands(game.completed_stichs(), hand_fixed.clone(), eplayerindex_fixed)
                    .filter(|ahand| is_compatible_with_game_so_far(ahand, game.m_rules, &game.m_vecstich))
                    .take(self.m_n_suggest_card_samples),
                self.m_n_suggest_card_branches,
            )
        }
    }
}

#[test]
fn test_is_compatible_with_game_so_far() {
    use rules::rulesrufspiel::*;
    use rules::payoutdecider::*;
    use util::cardvectorparser;
    use game;
    enum VTestAction {
        PlayStich(&'static str),
        AssertFrei(EPlayerIndex, VTrumpfOrFarbe),
        AssertNotFrei(EPlayerIndex, VTrumpfOrFarbe),
    }
    let test_game = |astr_hand: SPlayerIndexMap<&'static str>, rules: &TRules, eplayerindex_first, vectestaction: Vec<VTestAction>| {
        let mut game = game::SGame {
            m_doublings : SDoublings::new(eplayerindex_first),
            m_ahand : create_playerindexmap(|eplayerindex| {
                SHand::new_from_vec(cardvectorparser::parse_cards(astr_hand[eplayerindex]).unwrap())
            }),
            m_rules : rules,
            m_vecstich : vec![SStich::new(eplayerindex_first)],
            m_n_stock: 0,
            m_vecstoss : vec![], // TODO implement tests for SStoss
        };
        let mut vecpaireplayerindextrumpforfarbe_frei = Vec::new();
        for testaction in vectestaction {
            let mut oassertnotfrei = None;
            match testaction {
                VTestAction::PlayStich(str_stich) => {
                    for card in cardvectorparser::parse_cards::<Vec<_>>(str_stich).unwrap() {
                        let eplayerindex = game.which_player_can_do_something().unwrap();
                        game.zugeben(card, eplayerindex).unwrap();
                    }
                },
                VTestAction::AssertFrei(eplayerindex, trumpforfarbe) => {
                    vecpaireplayerindextrumpforfarbe_frei.push((eplayerindex, trumpforfarbe));
                },
                VTestAction::AssertNotFrei(eplayerindex, trumpforfarbe) => {
                    oassertnotfrei = Some((eplayerindex, trumpforfarbe));
                }
            }
            for ahand in forever_rand_hands(
                game.completed_stichs(),
                game.m_ahand[game.which_player_can_do_something().unwrap()].clone(),
                game.which_player_can_do_something().unwrap()
            )
                .filter(|ahand| is_compatible_with_game_so_far(ahand, game.m_rules, &game.m_vecstich))
                .take(100)
            {
                for eplayerindex in eplayerindex_values() {
                    println!("{}: {}", eplayerindex, ahand[eplayerindex]);
                }
                for &(eplayerindex, ref trumpforfarbe) in vecpaireplayerindextrumpforfarbe_frei.iter() {
                    assert!(!ahand[eplayerindex].contains_pred(|card| *trumpforfarbe==game.m_rules.trumpforfarbe(*card)));
                }
                if let Some((eplayerindex_not_frei, ref trumpforfarbe))=oassertnotfrei {
                    assert!(ahand[eplayerindex_not_frei].contains_pred(|card| *trumpforfarbe==game.m_rules.trumpforfarbe(*card)));
                }
            }
        }
    };
    test_game(
        ["h8 su g7 s7 gu eo gk s9", "eu h7 g8 sa ho sz hk hz", "h9 e7 ga gz g9 e9 ek ea", "hu ha so s8 go e8 sk ez"],
        &SRulesRufspiel {m_eplayerindex: 1, m_efarbe: EFarbe::Gras, m_n_payout_base: 20, m_laufendeparams: SLaufendeParams::new(10, 3)},
        /*eplayerindex_first*/ 2,
        vec![
            VTestAction::AssertNotFrei(1, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::PlayStich("h9 hu h8 eu"),
            VTestAction::AssertNotFrei(1, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::PlayStich("h7 e7 ha su"),
            VTestAction::AssertNotFrei(1, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::AssertFrei(2, VTrumpfOrFarbe::Trumpf),
            VTestAction::PlayStich("g7 g8 ga so"),
            VTestAction::AssertFrei(3, VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            VTestAction::PlayStich("s8 s7 sa gz"),
            VTestAction::AssertFrei(2, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            // Remaining stichs: "ho g9 go gu" "e8 eo sz e9" "gk hk ek sk" "hz ea ez s9"
        ]
    );
    test_game(
        ["sz ga hk g8 ea e8 g9 e7", "s7 gz h7 ho g7 sa s8 s9", "e9 ek gu go gk su sk hu", "so ez eo h9 hz h8 ha eu"],
        &SRulesRufspiel {m_eplayerindex: 0, m_efarbe: EFarbe::Schelln, m_n_payout_base: 20, m_laufendeparams: SLaufendeParams::new(10, 3)},
        /*eplayerindex_first*/ 1,
        vec![
            VTestAction::AssertNotFrei(0, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            VTestAction::PlayStich("s9 sk hz sz"),
            VTestAction::AssertFrei(0, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            VTestAction::AssertFrei(2, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
            VTestAction::AssertFrei(3, VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
        ]
    );
}
