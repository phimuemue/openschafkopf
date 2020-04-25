use crate::ai::*;
use crate::game::*;
use crate::player::*;
use crate::primitives::*;
use crate::rules::{ruleset::*, *};
use crate::skui;
use crate::util::*;
use std::sync::mpsc;

pub struct SPlayerHuman {
    pub ai : SAi,
}

fn choose_ruleset_or_rules<'t, T>(
    hand: &SHand,
    vect : &'t [T],
    fn_format: impl Fn(&T)->String,
    fn_choose: impl Fn(usize)->Option<&'t dyn TActivelyPlayableRules>,
    opairepiprio: &Option<(EPlayerIndex, VGameAnnouncementPriority)>,
) -> &'t T {
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
        let ab_doubling = [false, true];
        debug_verify!(txb_doubling.send(*skui::ask_for_alternative(
            &ab_doubling,
            &skui::choose_alternative_from_list_key_bindings(),
            |_| true, // all alternatives allowed
            |ncwin, i_b_doubling_chosen, ob_doubling_suggest| {
                assert!(ob_doubling_suggest.is_none());
                // TODO show who else already doubled
                skui::print_hand(veccard, None);
                for (i_b_doubling, b_doubling) in ab_doubling.iter().enumerate() {
                    skui::wprintln(ncwin, &format!("{} {}",
                        if i_b_doubling==i_b_doubling_chosen {"*"} else {" "},
                        if *b_doubling {"Doubling"} else {"No Doubling"},
                    ));
                }
            },
            || None, // TODO implement suggestions
        ))).unwrap()
    }

    fn ask_for_card(&self, game: &SGame, txcard: mpsc::Sender<SCard>) {
        skui::print_stichseq(debug_verify!(game.current_playable_stich().current_playerindex()).unwrap(), &game.stichseq);
        let hand = {
            let mut veccard = game.ahand[debug_verify!(game.which_player_can_do_something()).unwrap().0].cards().clone();
            game.rules.sort_cards_first_trumpf_then_farbe(&mut veccard);
            SHand::new_from_vec(veccard)
        };
        let veccard_allowed = game.rules.all_allowed_cards(&game.stichseq, &hand);
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
                || Some(self.ai.suggest_card(
                    game,
                    /*opath_out_dir*/Some(&std::path::Path::new("suspicion"))
                ))
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
        _tpln_stoss_doubling: (usize, usize),
        _n_stock: isize,
        opairepiprio: Option<(EPlayerIndex, VGameAnnouncementPriority)>,
        txorules: mpsc::Sender<Option<&'rules dyn TActivelyPlayableRules>>,
    ) {
        skui::print_game_announcements(epi, gameannouncements);
        let vecrulegroup : Vec<&SRuleGroup> = vecrulegroup.iter()
            .filter(|rulegroup| 0 < rulegroup.allowed_rules(hand).count())
            .collect();
        loop {
            let vecoorules : Vec<Option<Option<&dyn TActivelyPlayableRules>>> = std::iter::once(None) // stands for "back"
                .chain(
                    choose_ruleset_or_rules(
                        hand.get(),
                        &vecrulegroup,
                        |rulegroup| rulegroup.str_name.clone(),
                        |i_rulegroup_chosen| vecrulegroup[i_rulegroup_chosen].vecorules[0].as_ref().map(|rules| rules.as_ref()),
                        &opairepiprio,
                    )
                        .allowed_rules(hand)
                        .map(Some)
                )
                .collect();
            if let Some(orules) = *choose_ruleset_or_rules(
                hand.get(),
                &vecoorules,
                |oorules| match *oorules {
                    None => "Back".to_string(),
                    Some(None) => "Nothing".to_string(),
                    Some(Some(rules)) => rules.to_string()
                },
                |i_oorules_chosen| vecoorules[i_oorules_chosen].and_then(|orules| orules),
                &opairepiprio,
            ) {
                debug_verify!(txorules.send(orules)).unwrap();
                return;
            }
        }
    }

    fn ask_for_stoss(
        &self,
        _epi: EPlayerIndex,
        doublings: &SDoublings,
        rules: &dyn TRules,
        hand: &SHand,
        vecstoss: &[SStoss],
        _n_stock: isize,
        txb: mpsc::Sender<bool>,
    ) {
        let ab_stoss = [false, true];
        debug_verify!(txb.send(*skui::ask_for_alternative(
            &ab_stoss,
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
                for (i_b_stoss, b_stoss) in ab_stoss.iter().enumerate() {
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
        ))).unwrap()
    }
}
