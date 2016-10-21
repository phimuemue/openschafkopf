pub mod suspicion;
pub mod handiterators;

use primitives::*;
use rules::*;
use game::*;
use ai::suspicion::*;
use ai::handiterators::*;

use rand;
use std::collections::HashMap;
use std::iter::FromIterator;
use std::fs;
use std::mem;

pub trait TAi {
    fn rank_rules(&self, hand_fixed: &SFullHand, eplayerindex_first: EPlayerIndex, eplayerindex_rank: EPlayerIndex, rules: &TRules, n_tests: usize) -> f64;
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

fn suspicion_from_hands_respecting_stich_current(
    rules: &TRules,
    ahand: [SHand; 4],
    mut vecstich_complete_mut: &mut Vec<SStich>,
    stich_current: &SStich,
    n_branches: usize
) -> SSuspicion {
    assert_ahand_same_size(&ahand);
    let n_stich_complete = vecstich_complete_mut.len();
    let susp = SSuspicion::new(
        stich_current.first_playerindex(),
        ahand,
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
    assert!(susp.suspicion_transitions().len() <= susp.count_leaves());
    if let Err(_) = susp.print_suspicion(8, 0, rules, vecstich_complete_mut, Some(stich_current.clone()), &mut fs::File::create(&"suspicion.txt").unwrap()) {
        // TODO: what shall be done on error?
    }
    susp
}

fn possible_payouts(rules: &TRules, susp: &SSuspicion, mut vecstich_complete_payout: &mut Vec<SStich>, eplayerindex_fixed: EPlayerIndex) -> Vec<(SCard, isize)> { // TODO Rust: return iterator
    susp.suspicion_transitions().iter()
        .map(|susptrans| {
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
    fn rank_rules (&self, hand_fixed: &SFullHand, eplayerindex_first: EPlayerIndex, eplayerindex_rank: EPlayerIndex, rules: &TRules, n_tests: usize) -> f64 {
        // TODO: adjust interface to get whole game
        SAiSimulating{}.rank_rules(hand_fixed, eplayerindex_first, eplayerindex_rank, rules, n_tests)
    }

    fn internal_suggest_card(&self, game: &SGame) -> SCard {
        let mut vecstich_complete_mut = game.m_vecstich.iter()
            .filter(|stich| stich.size()==4)
            .cloned()
            .collect::<Vec<_>>();
        let stich_current = game.current_stich().clone();
        assert!(stich_current.size()<4);
        let susp = suspicion_from_hands_respecting_stich_current(
            game.m_rules,
            create_playerindexmap(|eplayerindex|
                SHand::new_from_vec(
                    stich_current.get(eplayerindex).cloned().into_iter()
                        .chain(game.m_ahand[eplayerindex].cards().iter().cloned())
                        .collect()
                )
            ),
            &mut vecstich_complete_mut,
            &stich_current,
            /*n_branches*/2
        );
        possible_payouts(game.m_rules, &susp, &mut vecstich_complete_mut, stich_current.current_playerindex().unwrap()).into_iter()
            .max_by_key(|&(_card, n_payout)| n_payout)
            .unwrap()
            .0
    }
}

pub struct SAiSimulating {}
fn is_compatible_with_game_so_far(ahand: &[SHand; 4], game: &SGame) -> bool {
    let ref stich_current = game.current_stich();
    assert!(stich_current.size()<4);
    // hands must contain respective cards from stich_current...
    stich_current.iter()
        .all(|(eplayerindex, card)| ahand[eplayerindex].contains(*card))
    // ... and must not contain other cards preventing farbe/trumpf frei
    && {
        let mut vecstich_complete_and_current_stich = game.completed_stichs().iter().cloned().collect::<Vec<_>>();
        vecstich_complete_and_current_stich.push(SStich::new(stich_current.first_playerindex()));
        stich_current.iter()
            .all(|(eplayerindex, card_played)| {
                let b_valid = game.m_rules.card_is_allowed(
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
        for stich in game.completed_stichs().iter().rev() {
            for eplayerindex in 0..4 {
                ahand_simulate[eplayerindex].cards_mut().push(stich[eplayerindex]);
            }
        }
        assert_ahand_same_size(&ahand_simulate);
        game.m_rules.playerindex().map_or(true, |eplayerindex_active|
            game.m_rules.can_be_played(&SFullHand::new(&ahand_simulate[eplayerindex_active]))
        )
        && {
            let mut b_valid_up_to_now = true;
            let mut vecstich_simulate = Vec::new();
            'loopstich: for stich in game.completed_stichs().iter() {
                vecstich_simulate.push(SStich::new(stich.m_eplayerindex_first));
                for (eplayerindex, card) in stich.iter() {
                    if game.m_rules.card_is_allowed(
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

fn determine_best_card<HandsIterator>(game: &SGame, eplayerindex_fixed: EPlayerIndex, itahand: HandsIterator) -> SCard
    where HandsIterator: Iterator<Item=[SHand; 4]>
{
    let mut vecstich_complete_mut = game.completed_stichs().iter().cloned().collect::<Vec<_>>();
    let veccard_allowed_fixed = game.m_rules.all_allowed_cards(&game.m_vecstich, &game.m_ahand[eplayerindex_fixed]);
    let mapcardpayout = itahand
        .map(|ahand| suspicion_from_hands_respecting_stich_current(
            game.m_rules,
            ahand,
            &mut vecstich_complete_mut,
            &game.current_stich(),
            /*n_branches*/1
        ))
        .fold(
            // aggregate n_payout per card in some way
            HashMap::from_iter(
                veccard_allowed_fixed.iter()
                    .map(|card| (card.clone(), 0)) // TODO Option<isize> more convenient?
            ),
            |mut mapcardpayout: HashMap<SCard, isize>, susp| {
                for (card, n_payout) in possible_payouts(game.m_rules, &susp, &mut game.completed_stichs().iter().cloned().collect(), eplayerindex_fixed) {
                    let n_payout_acc = mapcardpayout[&card];
                    *mapcardpayout.get_mut(&card).unwrap() = n_payout_acc + n_payout;
                }
                mapcardpayout
            }
        );
    veccard_allowed_fixed.into_iter()
        .max_by_key(|card| mapcardpayout[card])
        .unwrap()
        .clone()
}

impl TAi for SAiSimulating {
    fn rank_rules (&self, hand_fixed: &SFullHand, eplayerindex_first: EPlayerIndex, eplayerindex_rank: EPlayerIndex, rules: &TRules, n_tests: usize) -> f64 {
        forever_rand_hands(/*vecstich*/&Vec::new(), hand_fixed.get().clone(), eplayerindex_rank)
            .take(n_tests)
            .map(|ahand| {
                SSuspicion::new(
                    eplayerindex_first,
                    ahand,
                    rules,
                    &mut Vec::new(),
                    |_vecstich_complete, vecstich_successor| {
                        assert!(!vecstich_successor.is_empty());
                        random_sample_from_vec(vecstich_successor, 1);
                    }
                ).min_reachable_payout(rules, &mut Vec::new(), None, eplayerindex_rank)
            })
            .sum::<isize>() as f64
            / n_tests as f64
    }

    fn internal_suggest_card(&self, game: &SGame) -> SCard {
        let n_tests = 10;
        let ref stich_current = game.current_stich();
        assert!(stich_current.size()<4);
        let eplayerindex_fixed = stich_current.current_playerindex().unwrap();
        let ref hand_fixed = game.m_ahand[eplayerindex_fixed];
        assert!(!hand_fixed.cards().is_empty());
        if hand_fixed.cards().len()<=2 {
            determine_best_card(
                game,
                eplayerindex_fixed,
                all_possible_hands(game.completed_stichs(), hand_fixed.clone(), eplayerindex_fixed)
                    .filter(|ahand| is_compatible_with_game_so_far(ahand, game))
            )
        } else {
            determine_best_card(
                game,
                eplayerindex_fixed,
                forever_rand_hands(game.completed_stichs(), hand_fixed.clone(), eplayerindex_fixed)
                    .filter(|ahand| is_compatible_with_game_so_far(ahand, game))
                    .take(n_tests)
            )
        }
    }
}

#[test]
fn test_is_compatible_with_game_so_far() {
    use rules::rulesrufspiel::*;
    use util::cardvectorparser;
    use game;
    enum VTestAction {
        PlayStich(&'static str),
        AssertFrei(EPlayerIndex, VTrumpfOrFarbe),
        AssertNotFrei(EPlayerIndex, VTrumpfOrFarbe),
    }
    let test_game = |astr_hand: [&'static str; 4], rules: &TRules, eplayerindex_first, vectestaction: Vec<VTestAction>| {
        let mut game = game::SGame {
            m_doublings : SDoublings::new(eplayerindex_first),
            m_ahand : create_playerindexmap(|eplayerindex| {
                SHand::new_from_vec(cardvectorparser::parse_cards(astr_hand[eplayerindex]).unwrap())
            }),
            m_rules : rules,
            m_vecstich : vec![SStich::new(eplayerindex_first)],
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
                .filter(|ahand| is_compatible_with_game_so_far(ahand, &game))
                .take(100)
            {
                for eplayerindex in 0..4 {
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
        &SRulesRufspiel {m_eplayerindex: 1, m_efarbe: EFarbe::Gras},
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
        &SRulesRufspiel {m_eplayerindex: 0, m_efarbe: EFarbe::Schelln},
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
