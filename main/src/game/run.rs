use super::*;
use crate::player::*;
use crate::rules::{
    TActivelyPlayableRulesBoxClone, // TODO improve trait-object behaviour
};
use crate::skui;
use std::sync::mpsc;

pub struct SAtTable {
    pub player: Box<dyn TPlayer>,
    pub n_money: isize,
}

pub fn run_simple_game_loop(aplayer: EnumMap<EPlayerIndex, Box<dyn TPlayer>>, n_games: usize, ruleset: SRuleSet) -> ([SAtTable; 4], isize) {
    let mut aattable = aplayer.map_into(|player| SAtTable{player, n_money:0});
    let mut n_stock = 0;
    for _i_game in 0..n_games {
        fn communicate_via_channel<T: std::fmt::Debug>(f: impl FnOnce(mpsc::Sender<T>)) -> T {
            let (txt, rxt) = mpsc::channel::<T>();
            f(txt);
            unwrap!(rxt.recv())
        }
        let mut dealcards = SDealCards::new(ruleset.clone(), n_stock);
        while let Some(epi) = dealcards.which_player_can_do_something() {
            unwrap!(dealcards.announce_doubling(
                epi,
                /*b_doubling*/communicate_via_channel(|txb_doubling| {
                    aattable[epi].player.ask_for_doubling(
                        dealcards.first_hand_for(epi),
                        txb_doubling
                    );
                })
            ));
        }
        let mut gamepreparations = unwrap!(dealcards.finish());
        while let Some(epi) = gamepreparations.which_player_can_do_something() {
            info!("Asking player {} for game", epi);
            unwrap!(gamepreparations.announce_game(
                epi,
                communicate_via_channel(|txorules| {
                    aattable[epi].player.ask_for_game(
                        epi,
                        gamepreparations.fullhand(epi),
                        &gamepreparations.gameannouncements,
                        &gamepreparations.ruleset.avecrulegroup[epi],
                        stoss_and_doublings(/*vecstoss*/&[], &gamepreparations.doublings),
                        gamepreparations.n_stock,
                        None,
                        txorules
                    );
                }).map(TActivelyPlayableRulesBoxClone::box_clone)
            ));
        }
        info!("Asked players if they want to play. Determining rules");
        let stockorgame = match unwrap!(gamepreparations.finish()) {
            VGamePreparationsFinish::DetermineRules(mut determinerules) => {
                while let Some((epi, vecrulegroup_steigered))=determinerules.which_player_can_do_something() {
                    if let Some(rules) = communicate_via_channel(|txorules| {
                        aattable[epi].player.ask_for_game(
                            epi,
                            determinerules.fullhand(epi),
                            /*gameannouncements*/&SPlayersInRound::new(SStaticEPI0{}),
                            &vecrulegroup_steigered,
                            stoss_and_doublings(/*vecstoss*/&[], &determinerules.doublings),
                            determinerules.n_stock,
                            Some(determinerules.currently_offered_prio()),
                            txorules
                        );
                    }).map(TActivelyPlayableRulesBoxClone::box_clone) {
                        unwrap!(determinerules.announce_game(epi, rules));
                    } else {
                        unwrap!(determinerules.resign(epi));
                    }
                }
                VStockOrT::OrT(unwrap!(determinerules.finish()))
            },
            VGamePreparationsFinish::DirectGame(game) => {
                VStockOrT::OrT(game)
            },
            VGamePreparationsFinish::Stock(gameresult) => {
                VStockOrT::Stock(gameresult)
            }
        };
        let gameresult = match stockorgame {
            VStockOrT::OrT(mut game) => {
                while let Some(gameaction)=game.which_player_can_do_something() {
                    if !gameaction.1.is_empty() {
                        if let Some(epi_stoss) = gameaction.1.iter()
                            .find(|epi| {
                                communicate_via_channel(|txb_stoss| {
                                    aattable[**epi].player.ask_for_stoss(
                                        **epi,
                                        &game.doublings,
                                        game.rules.as_ref(),
                                        &game.ahand[**epi],
                                        &game.vecstoss,
                                        game.n_stock,
                                        txb_stoss,
                                    );
                                })
                            })
                        {
                            unwrap!(game.stoss(*epi_stoss));
                            continue;
                        }
                    }
                    unwrap!(game.zugeben(
                        communicate_via_channel(|txcard| {
                            aattable[gameaction.0].player.ask_for_card(
                                &game,
                                txcard,
                            );
                        }),
                        gameaction.0
                    ));
                }
                unwrap!(game.finish())
            },
            VStockOrT::Stock(gameresult) => gameresult,
        };
        gameresult.apply_payout(&mut n_stock, |epi, n_payout| {
            aattable[epi].n_money += n_payout;
        });
        assert_eq!(n_stock + aattable.iter().map(|attable| attable.n_money).sum::<isize>(), 0);
        skui::print_account_balance(&aattable.map(|attable| attable.n_money), n_stock);
        aattable.as_raw_mut().rotate_left(1);
    }
    (aattable.into_raw(), n_stock)
}

