fn kartenverteilung(ctx, n_relevant_cards) {
    let i_relevant_card = 0;
    let an_count = [0, 0, 0, 0];
    for eschlag in [schlag::Ass, schlag::Zehn, schlag::Koenig, schlag::Ober, schlag::Unter, schlag::Neun, schlag::Acht, schlag::Sieben] {
        for efarbe in [farbe::Eichel, farbe::Gras, farbe::Herz] {
            if i_relevant_card<n_relevant_cards {
                i_relevant_card += 1;
                an_count[ctx.who_has_card(card::new_card(efarbe, eschlag))] += 1;
            }
        }
    }
    an_count.shift();
    an_count.sort();
    an_count.reverse();
    an_count
}
