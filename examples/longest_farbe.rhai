fn longest_farben(ctx, epi) {
    let vecefarbe = [farbe::Eichel, farbe::Gras, farbe::Herz, farbe::Schelln];
    let n_longest = 0;
    for farbe in vecefarbe {
        n_longest = max(n_longest, ctx.trumpforfarbe(trumpforfarbe::farbe(farbe), epi));
    }
    if 0 < n_longest {
        vecefarbe.filter(|farbe|
            ctx.trumpforfarbe(trumpforfarbe::farbe(farbe), epi)==n_longest
        )
    } else {
        []
    }
}

fn longest_farben_no_ass(ctx, epi) {
    longest_farben(ctx, epi)
        .filter(|farbe|
            ctx.who_has_card(card::new_card(farbe, schlag::Ass))!=epi
        )
}
