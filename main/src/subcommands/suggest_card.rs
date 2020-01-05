use crate::util::*;
use crate::primitives::*;
use crate::ai::suspicion::*;
use crate::game::SStichSequence;

pub fn suggest_card(
    str_epi_first: &str,
    str_rules_with_epi: &str,
    hand_fixed: &SHand,
    slccard_as_played: &[SCard],
) -> Result<(), Error> {
    // TODO check that everything is ok (no duplicate cards, cards are allowed, current stich not full, etc.)
    let fn_str_to_epi = |str_epi: &str| EPlayerIndex::checked_from_usize(str_epi.parse()?)
        .ok_or_else(|| format_err!("Cannot convert {} to EPlayerIndex.", str_epi));
    let rules = crate::rules::parser::parse_rule_description(
        str_rules_with_epi,
        (/*n_tarif_extra*/10, /*n_tarif_ruf*/20, /*n_tarif_solo*/50), // TODO make adjustable
        fn_str_to_epi,
    )?;
    let rules = rules.as_ref();
    let ekurzlang = EKurzLang::checked_from_cards_per_player(
        /*n_stichs_complete*/slccard_as_played.len() / EPlayerIndex::SIZE
            + hand_fixed.cards().len()
    )
        .ok_or_else(|| format_err!("Cannot determine ekurzlang from {} and {:?}.", hand_fixed, slccard_as_played))?;
    let stichseq = slccard_as_played.iter()
        .fold_mutating(
            SStichSequence::new(
                fn_str_to_epi(str_epi_first)?,
                ekurzlang,
            ),
            |stichseq, card| {
                stichseq.zugeben(*card, rules);
            }
        );
    let epi = debug_verify!(stichseq.current_stich().current_playerindex()).unwrap();
    let determinebestcardresult = crate::ai::SAi::suggest_card_simulating( // should not distinguish for SingleAllowed (we want to know expected payout anyway)
        rules,
        &stichseq,
        epi,
        hand_fixed,
        /*n_suggest_card_samples*/50, // TODO? make customizable
        /*n_suggest_card_branches*/2, // TODO? make customizable
        /*tpln_stoss_doubling*/(0, 0), // TODO
        /*n_stock*/0, // TODO
        /*opath_out_dir*/None,
    );
    // TODO interface should probably output payout interval per card
    let mut veccardminmax = determinebestcardresult.cards_and_ts().collect::<Vec<_>>();
    veccardminmax.sort_unstable_by_key(|&(_card, minmax)| minmax.values_for(epi).into_raw());
    veccardminmax.reverse(); // descending
    for (card, minmax) in veccardminmax {
        println!("{}: {}/{}",
            card,
            minmax.aan_payout[EMinMaxStrategy::OthersMin][epi],
            minmax.aan_payout[EMinMaxStrategy::MaxPerEpi][epi],
        );
    }
    Ok(())
}
