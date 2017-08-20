use primitives::*;
use game::*;
use rules::*;
use rules::rulesrufspiel::*;
use rules::card_points::points_card;
use ai::rulespecific::*;

#[derive(new)]
pub struct SAIRufspiel<'rules> {
    rules : &'rules SRulesRufspiel,
}

impl<'rules> TRuleSpecificAI for SAIRufspiel<'rules> {
    fn suggest_card(&self, game: &SGame) -> Option<SCard> {
        let epi = game.which_player_can_do_something().unwrap().0;
        let rules = self.rules;
        // suchen
        if epi!=rules.playerindex().unwrap() && 1==game.vecstich.len() && 0==game.current_stich().size() {
            let hand = &game.ahand[epi];
            if !hand.contains(rules.rufsau()) {
                let veccard_ruffarbe : Vec<_> = hand.cards().iter().cloned()
                    .filter(|&card| rules.trumpforfarbe(card)==rules.trumpforfarbe(rules.rufsau()))
                    .collect();
                match veccard_ruffarbe.len() {
                    0 => return None,
                    1 | 2 => return veccard_ruffarbe.into_iter().min_by_key(|&card| points_card(card)),
                    3 | 4 => return veccard_ruffarbe.into_iter().max_by_key(|&card| points_card(card)),
                    _ => panic!("Found too many ruffarbe cards"),
                }
            }
        }
        None
    }
}
