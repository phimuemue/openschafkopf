use primitives::*;
use player::*;
use rules::{
    *,
    ruleset::*,
};
use game::*;
use skui;
use ai::*;
use util::*;
use std::{
    sync::mpsc,
    fs,
};

pub struct SPlayerHuman {
    pub ai : Box<TAi>,
}

fn choose_ruleset_or_rules<'t, T, FnFormat, FnChoose>(
    hand: &SHand,
    vect : &'t [T],
    fn_format: FnFormat,
    fn_choose: FnChoose,
    opairepiprio: &Option<(EPlayerIndex, VGameAnnouncementPriority)>,
) -> &'t T
    where FnFormat: Fn(&T) -> String,
          FnChoose: Fn(usize) -> Option<&'t TActivelyPlayableRules>
{
    skui::ask_for_alternative(
        vect,
        &skui::choose_alternative_from_list_key_bindings(),
        |_ot| {true},
        |ncwin, i_ot_chosen, _ot_suggest| {
            assert!(_ot_suggest.is_none());
            skui::wprintln(ncwin, &format!("Your cards: {}. What do you want to play?", hand));
            if let Some(ref pairepiprio) = *opairepiprio {
                skui::wprintln(ncwin, &format!("{} offers {:?}", pairepiprio.0, pairepiprio.1)); // TODO improve output here
            }
            for (i_t, t) in vect.iter().enumerate() {
                skui::wprintln(ncwin, &format!("{} {} ({})",
                    if i_t==i_ot_chosen {"*"} else {" "},
                    fn_format(t),
                    i_t
                ));
            }
            let mut veccard = hand.cards().clone();
            if let Some(rules)=fn_choose(i_ot_chosen) {
                rules.sort_cards_first_trumpf_then_farbe(veccard.as_mut_slice());
            }
            skui::print_hand(&veccard, None);
        },
        || {None}
    )
}

impl TPlayer for SPlayerHuman {
    fn ask_for_doubling(
        &self,
        veccard: &[SCard],
        txb_doubling: mpsc::Sender<bool>,
    ) {
        let vecb_doubling = vec![false, true];
        txb_doubling.send(*skui::ask_for_alternative(
            &vecb_doubling,
            &skui::choose_alternative_from_list_key_bindings(),
            |_| true, // all alternatives allowed
            |ncwin, i_b_doubling_chosen, ob_doubling_suggest| {
                assert!(ob_doubling_suggest.is_none());
                // TODO show who else already doubled
                skui::print_hand(veccard, None);
                for (i_b_doubling, b_doubling) in vecb_doubling.iter().enumerate() {
                    skui::wprintln(ncwin, &format!("{} {}",
                        if i_b_doubling==i_b_doubling_chosen {"*"} else {" "},
                        if *b_doubling {"Doubling"} else {"No Doubling"},
                    ));
                }
            },
            || None, // TODO implement suggestions
        )).unwrap()
    }

    fn ask_for_card(&self, game: &SGame, txcard: mpsc::Sender<SCard>) {
        skui::print_vecstich(verify!(game.current_stich().current_playerindex()).unwrap(), &game.vecstich);
        let hand = {
            let mut hand = game.ahand[game.which_player_can_do_something().unwrap().0].clone();
            game.rules.sort_cards_first_trumpf_then_farbe(hand.cards_mut());
            hand
        };
        let veccard_allowed = game.rules.all_allowed_cards(&game.vecstich, &hand);
        if txcard.send(
            *skui::ask_for_alternative(
                hand.cards(),
                &skui::choose_card_from_hand_key_bindings(),
                |card| {veccard_allowed.iter().any(|card_allowed| card_allowed==card)},
                |ncwin, i_card_chosen, ocard_suggest| {
                    if let Some(card) = *ocard_suggest {
                        skui::wprintln(ncwin, &format!("AI: {}", card));
                    }
                    skui::print_hand(hand.cards(), Some(i_card_chosen));
                    skui::print_game_info(game.rules.as_ref(), &game.doublings, &game.vecstoss);
                },
                || Some(self.ai.suggest_card(game, /*ofile_output*/Some(fs::File::create(&"suspicion.html").unwrap())))
            )
        ).is_err() {
            unimplemented!() // we possibly want to be able to deal with "blocked" plays (timeout etc.)
        }
    }

    fn ask_for_game<'rules>(
        &self,
        epi: EPlayerIndex,
        hand: SFullHand,
        gameannouncements : &SGameAnnouncements,
        vecrulegroup: &'rules [SRuleGroup],
        _n_stock: isize,
        opairepiprio: Option<(EPlayerIndex, VGameAnnouncementPriority)>,
        txorules: mpsc::Sender<Option<&'rules TActivelyPlayableRules>>,
    ) {
        skui::print_game_announcements(epi, gameannouncements);
        let vecorulegroup : Vec<Option<&SRuleGroup>> = Some(None).into_iter()
            .chain(
                vecrulegroup.iter()
                    .filter(|rulegroup| rulegroup.vecrules.iter()
                        .any(|rules| rules.can_be_played(hand))
                    )
                    .map(Some)
            )
            .collect();
        while let Some(rulegroup) = *choose_ruleset_or_rules(
            hand.get(),
            &vecorulegroup,
            |orulegroup : &Option<&SRuleGroup>| match *orulegroup {
                None => "Nothing".to_string(),
                Some(rulegroup) => rulegroup.str_name.clone(),
            },
            |i_orulegroup_chosen| vecorulegroup[i_orulegroup_chosen].map(|rulegroup| rulegroup.vecrules[0].as_ref()),
            &opairepiprio,
        )
        {
            let vecorules : Vec<Option<&TActivelyPlayableRules>> = Some(None).into_iter()
                .chain(
                    rulegroup.vecrules.iter()
                        .filter(|rules| rules.can_be_played(hand))
                        .map(|rules| Some(rules.as_ref()))
                )
                .collect();
            if let Some(rules) = *choose_ruleset_or_rules(
                hand.get(),
                &vecorules,
                |orules : &Option<&TActivelyPlayableRules>| match *orules {
                    None => "Back".to_string(),
                    Some(rules) => rules.to_string()
                },
                |i_orules_chosen| vecorules[i_orules_chosen],
                &opairepiprio,
            ) {
                txorules.send(Some(rules)).unwrap();
                return;
            }
        }
        txorules.send(None).unwrap();
    }

    fn ask_for_stoss(
        &self,
        _epi: EPlayerIndex,
        doublings: &SDoublings,
        rules: &TRules,
        hand: &SHand,
        vecstoss: &[SStoss],
        _n_stock: isize,
        txb: mpsc::Sender<bool>,
    ) {
        let vecb_stoss = vec![false, true];
        txb.send(*skui::ask_for_alternative(
            &vecb_stoss,
            &skui::choose_alternative_from_list_key_bindings(),
            |_| true, // all alternatives allowed
            |ncwin, i_b_stoss_chosen, ob_stoss_suggest| {
                assert!(ob_stoss_suggest.is_none());
                skui::print_game_info(rules, doublings, vecstoss);
                {
                    let mut veccard = hand.cards().clone();
                    rules.sort_cards_first_trumpf_then_farbe(veccard.as_mut_slice());
                    skui::print_hand(&veccard, None);
                }
                for (i_b_stoss, b_stoss) in vecb_stoss.iter().enumerate() {
                    skui::wprintln(ncwin, &format!("{} {} {}",
                        if i_b_stoss==i_b_stoss_chosen {"*"} else {" "},
                        if *b_stoss {"Give"} else {"No"},
                        { match vecstoss.len() {
                            0 => "Kontra",
                            1 => "Re",
                            2 => "Sup",
                            3 => "Hirsch",
                            _ => panic!() // currently only quadruple stoss supported
                        } },
                    ));
                }
            },
            || None, // TODO implement suggestions
        )).unwrap()
    }
}
