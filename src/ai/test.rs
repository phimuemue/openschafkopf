use game_loop_cli;
use primitives::*;
use rules::ruleset::*;
use player::{
    TPlayer,
    playerrandom::SPlayerRandom,
};
use util::*;
use handiterators::*;
use ai::{
    is_compatible_with_game_so_far,
    suspicion::*,
};

#[test]
fn detect_expensive_all_possible_hands() {
    game_loop_cli(
        &EPlayerIndex::map_from_fn(|_epi| Box::new(SPlayerRandom::new(
            /*fn_check_ask_for_card*/|game| {
                let vecstich = game.completed_stichs();
                if game.kurzlang().cards_per_player() - 4 < vecstich.get().len() {
                    let epi_fixed = verify!(game.current_stich().current_playerindex()).unwrap();
                    let vecahand = all_possible_hands(
                        vecstich,
                        game.ahand[epi_fixed].clone(),
                        epi_fixed,
                    )
                        .filter(|ahand| is_compatible_with_game_so_far(ahand, game.rules.as_ref(), &game.vecstich))
                        .collect::<Vec<_>>();
                    let assert_bound = |n, n_detect| {
                        assert!(n < n_detect,
                            "n: {}\nrules: {}\nahand: {:#?}\nvecstich:{:?}",
                            n,
                            game.rules,
                            game.ahand.iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>(),
                            game.vecstich.iter()
                                .map(ToString::to_string)
                                .collect::<Vec<_>>(),
                        );
                    };
                    assert_bound(vecahand.len(), 2000);
                    for ahand in vecahand {
                        assert_bound(
                            SSuspicion::new(
                                game.current_stich().first_playerindex(),
                                ahand,
                                game.rules.as_ref(),
                                &mut game.completed_stichs().get().to_vec(),
                                &|_vecstich_complete, _vecstich_successor| {/*no filtering*/},
                            ).count_leaves(),
                            2000,
                        );
                    }
                }
            },
        )) as Box<TPlayer>),
        /*n_games*/4,
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
