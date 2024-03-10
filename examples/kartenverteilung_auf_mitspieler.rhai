fn kartenverteilung(ctx, n_relevant_cards) {
    let acard = [
        card::EA, card::GA, card::HA,
        card::EZ, card::GZ, card::HZ,
        card::EK, card::GK, card::HK,
        card::EO, card::GO, card::HO,
        card::EU, card::GU, card::HU,
        card::E9, card::G9, card::H9,
        card::E8, card::G8, card::H8,
        card::E7, card::G7, card::H7,
    ];
    let an_count = [0, 0, 0, 0];
    for card in acard.extract(0..n_relevant_cards) {
        an_count[ctx.who_has_card(card)] += 1;
    }
    an_count.shift();
    an_count.sort();
    an_count.reverse();
    an_count
}
