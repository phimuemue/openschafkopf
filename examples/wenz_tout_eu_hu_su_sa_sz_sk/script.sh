#!/bin/bash

N_SIMULATE_HANDS=1000000
EPI_POSITION=3

target/release/openschafkopf hand-stats \
    --rules "wenz tout von $EPI_POSITION" \
    --position $EPI_POSITION \
    --hand "eu hu su sa sz sk" \
    --simulate-hands $N_SIMULATE_HANDS \
    --inspect '
        import "examples/longest_farbe.rhai" as longest_farbe;
        let epi_wenz_tout = ctx.who_has_eu(); // TODO Can we use EPI_POSITION in here somehow?
        let is_farbe_dangerous = |epi, farbe| {
            let trumpforfarbe = trumpforfarbe::farbe(farbe);
            (
                ctx.trumpforfarbe(trumpforfarbe, epi_wenz_tout)>0 // epi_wenz_tout must follow farbe => always dangerous
                || epi > epi_wenz_tout // Otherwise => only dangerous if another player may top the Wenz player
            )
            && ctx.trumpforfarbe(trumpforfarbe, epi)==0 && ctx.trumpf(epi)>0
        };
        let afarbe_longest = longest_farbe::longest_farben_no_ass(ctx,0);
        if afarbe_longest.is_empty() {
            // If all longest farben with Ass, play any longest farbe, hoping another player is frei
            afarbe_longest = longest_farbe::longest_farben(ctx,0);
        }
        let prob_first_player_plays_a_farbe_where_another_player_is_frei = 0;
        for farbe in afarbe_longest {
            if
                is_farbe_dangerous.call(1, farbe)
                || is_farbe_dangerous.call(2, farbe)
                || is_farbe_dangerous.call(3, farbe)
            {
                prob_first_player_plays_a_farbe_where_another_player_is_frei +=
                    (1./afarbe_longest.len());
            }
        }
        [
            // afarbe_longest.to_string(), // Uncomment for debugging
            1-prob_first_player_plays_a_farbe_where_another_player_is_frei,
        ]
    '

