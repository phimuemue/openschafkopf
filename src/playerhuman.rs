use stich::*;
use card::*;
use hand::*;
use player::*;
use gamestate::*;
use rules::*;
use ruleset::*;
use skui;

use std::sync::mpsc;
use std::io::Read;
use std::rc::Rc;

pub struct CPlayerHuman;

impl CPlayer for CPlayerHuman {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<CCard>) {
        let eplayerindex = gamestate.which_player_can_do_something().unwrap();
        skui::print_vecstich(&gamestate.m_vecstich);
        let ref hand = gamestate.m_ahand[eplayerindex];
        txcard.send(
            skui::ask_for_alternative(
                &format!("Your cards: {}", hand),
                &gamestate.m_rules.all_allowed_cards(
                    &gamestate.m_vecstich,
                    &hand
                ),
                |card| card.to_string(),
                |_card, _i_card| {
                    // beware: i_card is not a valid index into hand!
                    skui::print_hand(hand, None)
                }
            )
        );
    }

    fn ask_for_game(&self, eplayerindex: EPlayerIndex, hand: &CHand) -> Option<Rc<TRules>> {
        let vecorules = Some(None).into_iter() // TODO is there no singleton iterator?
            .chain(
                ruleset_default(eplayerindex).allowed_rules().iter()
                    .filter(|rules| rules.can_be_played(hand))
                    .map(|rules| Some(rules.clone()))
            )
            .collect();
        skui::ask_for_alternative(
            &format!("Your cards: {}. What do you want to play?", hand),
            &vecorules,
            |orules| match orules {
                &None => "Nothing".to_string(),
                &Some(ref rules) => rules.to_string()
            },
            |orules, _i_orules| {
                let mut veccard = hand.cards().clone();
                if let Some(rules)=orules.as_ref() {
                    veccard.sort_by(|&card_lhs, &card_rhs| {
                        match(rules.trumpf_or_farbe(card_lhs), rules.trumpf_or_farbe(card_rhs)) {
                            (VTrumpfOrFarbe::Farbe(efarbe_lhs), VTrumpfOrFarbe::Farbe(efarbe_rhs)) => {
                                if efarbe_lhs==efarbe_rhs {
                                    rules.compare_in_stich_farbe(card_lhs, card_rhs)
                                } else {
                                    efarbe_lhs.cmp(&efarbe_rhs)
                                }
                            }
                            (_, _) => { // at least one of them is trumpf
                                rules.compare_in_stich(card_lhs, card_rhs)
                            }
                        }
                    }.reverse());
                }
                skui::print_hand(&CHand::new_from_vec(veccard), None);
            }
        )
    }
}
