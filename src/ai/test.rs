use game_loop_cli;
use primitives::*;
use rules::ruleset::*;
use player::{
    TPlayer,
    playerrandom::SPlayerRandom,
};
use util::*;
use handiterators::*;
use ai::is_compatible_with_game_so_far;

#[test]
fn detect_expensive_all_possible_hands() {
    game_loop_cli(
        &EPlayerIndex::map_from_fn(|_epi| Box::new(SPlayerRandom::new(
            /*fn_check_ask_for_card*/|game| {
                let vecstich = game.completed_stichs();
                if game.kurzlang().cards_per_player() - 4 < vecstich.len() {
                    let epi_fixed = verify!(game.current_stich().current_playerindex()).unwrap();
                    let n_count_possible_hands = all_possible_hands(
                        vecstich,
                        game.ahand[epi_fixed].clone(),
                        epi_fixed,
                    )
                        .filter(|ahand| is_compatible_with_game_so_far(ahand, game.rules.as_ref(), &game.vecstich))
                        .count();
                    assert!(n_count_possible_hands < /*arbitrary bound, hoping that exploration for lower values can be done efficiently*/3000,
                        "n_count_possible_hands: {}, rules: {}\nahand: {:#?}\nvecstich:{:?}",
                        n_count_possible_hands,
                        game.rules,
                        game.ahand.iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>(),
                        game.vecstich.iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>(),
                    )
                }
            },
        )) as Box<TPlayer>),
        /*n_games*/400,
        &verify!(SRuleSet::from_string(
            r"
            base-price=10
            solo-price=50
            lauf-min=3
            [rufspiel]
            [solo]
            [wenz]
            lauf-min=2
            [stoss]
            max=3
            ",
        )).unwrap()
    );
}
