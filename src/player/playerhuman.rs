use card::*;
use hand::*;
use player::*;
use rules::*;
use rules::ruleset::*;
use game::*;
use skui;
use ai::*;

use std::sync::mpsc;
use std::io::Read;

pub struct SPlayerHuman<'ai> {
    pub m_ai : &'ai TAi,
}

impl<'ai> TPlayer for SPlayerHuman<'ai> {
    fn take_control(&mut self, gamestate: &SGameState, txcard: mpsc::Sender<SCard>) {
        skui::print_vecstich(&gamestate.m_vecstich);
        let ref hand = gamestate.m_ahand[gamestate.which_player_can_do_something().unwrap()];
        let veccard_allowed = gamestate.m_rules.all_allowed_cards(&gamestate.m_vecstich, &hand);
        match txcard.send(
            skui::ask_for_alternative(
                &format!("Your cards: {}", hand),
                &hand.cards(),
                skui::choose_card_from_hand_key_bindings(),
                |card| {veccard_allowed.iter().any(|card_allowed| card_allowed==card)},
                |card| card.to_string(),
                |_card, i_card| {
                    skui::print_hand(hand.cards(), Some(i_card));
                    skui::print_game_info(gamestate);
                },
                || {Some(self.m_ai.suggest_card(gamestate))}
            ).clone()
        ) {
            Ok(_) => (),
            Err(_) => unimplemented!(), // we possibly want to be able to deal with "blocked" plays (timeout etc.)
        }
    }

    fn ask_for_game<'rules>(&self, hand: &SHand, vecgameannouncement : &Vec<SGameAnnouncement>, ruleset: &'rules SRuleSet) -> Option<&'rules TRules> {
        skui::print_game_announcements(vecgameannouncement);
        *skui::ask_for_alternative(
            &format!("Your cards: {}. What do you want to play?", hand),
            &Some(None).into_iter() // TODO is there no singleton iterator?
                .chain(
                    ruleset.allowed_rules().iter()
                        .filter(|rules| rules.can_be_played(hand))
                        .map(|rules| Some(rules.as_ref()))
                )
                .collect::<Vec<_>>(),
            skui::choose_alternative_from_list_key_bindings(),
            |_orules| {true},
            |orules| match orules {
                &None => "Nothing".to_string(),
                &Some(ref rules) => rules.to_string()
            },
            |orules, _i_orules| {
                let mut veccard = hand.cards().clone();
                if let Some(rules)=orules.as_ref() {
                    rules.sort_cards_first_trumpf_then_farbe(veccard.as_mut_slice());
                }
                skui::print_hand(&veccard, None);
            },
            ||{None}
        )
    }
}
