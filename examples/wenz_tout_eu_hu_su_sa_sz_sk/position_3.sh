#!/bin/bash

N_SIMULATE_HANDS=1000000

target/release/openschafkopf hand-stats \
    --rules "wenz tout von 3" \
    --position rulesannouncer \
    --hand "eu hu su sa sz sk" \
    --simulate-hands $N_SIMULATE_HANDS \
    --inspect '
        import "examples/longest_farbe.rhai" as longest_farbe;
        let player_schelln_frei_and_trumpf = |epi| {
            // Note: This condition is always false for the active player.
            ctx.schelln(epi)==0&&ctx.trumpf(epi)>0
        };
        let player_1_or_2_schelln_frei_and_trumpf = if
            player_schelln_frei_and_trumpf.call(1)
            || player_schelln_frei_and_trumpf.call(2)
        {
            1
        } else {
            0
        };
        let a = longest_farbe::longest_farben_no_ass(ctx,0);
        let prob_first_player_plays_schelln = if a.contains(farbe::Schelln) {
            1./a.len()
        } else {
            0
        };
        [
            a.to_string(),
            player_1_or_2_schelln_frei_and_trumpf,
            prob_first_player_plays_schelln,
            prob_first_player_plays_schelln * player_1_or_2_schelln_frei_and_trumpf,
        ]
    '

