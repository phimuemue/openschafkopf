fn longest_farben(ctx, epi, fn_filter) {
    let n_longest = 0;
    let vecefarbe = [];
    for efarbe in [farbe::Eichel, farbe::Gras, farbe::Herz, farbe::Schelln] {
        let n_farbe = ctx.trumpforfarbe(trumpforfarbe::farbe(efarbe), epi);
        if n_farbe > 0 && fn_filter.call(efarbe) {
            if n_longest < n_farbe {
                n_longest = n_farbe;
                vecefarbe = [efarbe];
            } else if n_longest==n_farbe {
                vecefarbe.push(efarbe);
            }
        }
    }
    vecefarbe
}

fn longest_farben(ctx, epi) {
    longest_farben(ctx, epi, |efarbe| true)
}

fn longest_farben_no_ass(ctx, epi) {
    longest_farben(ctx, epi, |efarbe|
        ctx.who_has_card(card::new_card(efarbe, schlag::Ass))!=epi
    )
}
