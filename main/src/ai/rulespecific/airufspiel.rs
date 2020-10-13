use crate::ai::rulespecific::*;
use crate::game::*;
use crate::primitives::*;
use crate::rules::{card_points::points_card, rulesrufspiel::*, *};
use crate::util::*;

#[derive(new)]
pub struct SAIRufspiel<'rules> {
    rules : &'rules SRulesRufspiel,
}

impl TRuleSpecificAI for SAIRufspiel<'_> {
    fn suggest_card(&self, game: &SGame) -> Option<SCard> {
        let epi = unwrap!(game.which_player_can_do_something()).0;
        let rules = self.rules;
        // suchen
        if epi!=rules.active_playerindex() && game.stichseq.no_card_played() {
            let hand = &game.ahand[epi];
            if !hand.contains(rules.rufsau()) {
                let veccard_ruffarbe : Vec<_> = hand.cards().iter().copied()
                    .filter(|&card| rules.trumpforfarbe(card)==rules.trumpforfarbe(rules.rufsau()))
                    .collect();
                match (veccard_ruffarbe.len(), game.kurzlang()) {
                    (0, _kurzlang) => return None,
                    (1, _kurzlang) => return Some(veccard_ruffarbe[0]),
                    (2, EKurzLang::Kurz) => return verify!(veccard_ruffarbe.into_iter().max_by_key(|&card| points_card(card))),
                    (2, EKurzLang::Lang) => return verify!(veccard_ruffarbe.into_iter().min_by_key(|&card| points_card(card))),
                    (3, EKurzLang::Lang) | (4, EKurzLang::Lang) => return verify!(veccard_ruffarbe.into_iter().max_by_key(|&card| points_card(card))),
                    _ => panic!("Found too many ruffarbe cards"),
                }
            }
        }
        None
    }
}
