use crate::ai::{*, handiterators::*, suspicion::*};
use crate::game::SStichSequence;
use crate::primitives::*;
use crate::util::*;

pub fn suggest_card(
    str_rules_with_epi: &str,
    hand_fixed: &SHand,
    slccard_as_played: &[SCard],
) -> Result<(), Error> {
    // TODO check that everything is ok (no duplicate cards, cards are allowed, current stich not full, etc.)
    let rules = crate::rules::parser::parse_rule_description_simple(str_rules_with_epi)?;
    let rules = rules.as_ref();
    let stichseq = SStichSequence::new_from_cards(
        /*ekurzlang*/EKurzLang::checked_from_cards_per_player(
            /*n_stichs_complete*/slccard_as_played.len() / EPlayerIndex::SIZE
                + hand_fixed.cards().len()
        )
            .ok_or_else(|| format_err!("Cannot determine ekurzlang from {} and {:?}.", hand_fixed, slccard_as_played))?,
        slccard_as_played.iter().copied(),
        rules
    );
    let determinebestcard =  SDetermineBestCard::new(
        rules,
        &stichseq,
        &hand_fixed,
    );
    let n_suggest_card_samples = 50; // TODO? make customizable
    let n_suggest_card_branches = 2; // TODO? make customizable
    let determinebestcardresult = { // we are interested in payout => single-card-optimization useless
        macro_rules! forward_step_2{($itahand: expr, $func_filter_allowed_cards: expr, $foreachsnapshot: ident,) => {{ // TODORUST generic closures
            determine_best_card(
                &determinebestcard,
                $itahand,
                $func_filter_allowed_cards,
                &$foreachsnapshot::new(
                    determinebestcard.rules,
                    determinebestcard.epi_fixed,
                    /*tpln_stoss_doubling*/(0, 0), // TODO? make customizable
                    /*n_stock*/0, // TODO? make customizable
                ),
                |minmax_acc, minmax| {
                    minmax_acc.assign_min_by_key(&minmax, determinebestcard.epi_fixed);
                },
                /*opath_out_dir*/None, // TODO? make customizable
            )
        }}}
        macro_rules! forward_step_1{($itahand: expr) => { // TODORUST generic closures
            // TODORUST exhaustive_integer_patterns for isize/usize
            // https://github.com/rust-lang/rfcs/pull/2591/commits/46135303146c660f3c5d34484e0ede6295c8f4e7#diff-8fe9cb03c196455367c9e539ea1964e8R70
            match /*n_remaining_cards_on_hand*/remaining_cards_per_hand(determinebestcard.stichseq)[determinebestcard.epi_fixed] {
                1|2|3 => forward_step_2!(
                    $itahand,
                    &|_,_| (/*no filtering*/),
                    SMinReachablePayout,
                ),
                4 => forward_step_2!(
                    $itahand,
                    &|_,_| (/*no filtering*/),
                    SMinReachablePayoutLowerBoundViaHint,
                ),
                5|6|7|8 => forward_step_2!(
                    $itahand,
                    &branching_factor(|_stichseq| {
                        (1, n_suggest_card_branches+1)
                    }),
                    SMinReachablePayoutLowerBoundViaHint,
                ),
                n_remaining_cards_on_hand => panic!("internal_suggest_card called with {} cards on hand", n_remaining_cards_on_hand),
            }


        }}
        let epi_fixed = determinebestcard.epi_fixed;
        match /*n_remaining_cards_on_hand*/remaining_cards_per_hand(determinebestcard.stichseq)[epi_fixed] {
            1|2|3|4 => forward_step_1!(
                all_possible_hands(determinebestcard.stichseq, determinebestcard.hand_fixed.clone(), epi_fixed, determinebestcard.rules)
            ),
            5|6|7|8 => forward_step_1!(
                forever_rand_hands(determinebestcard.stichseq, determinebestcard.hand_fixed.clone(), epi_fixed, determinebestcard.rules)
                    .take(n_suggest_card_samples)
            ),
            n_remaining_cards_on_hand => panic!("internal_suggest_card called with {} cards on hand", n_remaining_cards_on_hand),
        }
    };
    // TODO interface should probably output payout interval per card
    let epi = debug_verify!(stichseq.current_stich().current_playerindex()).unwrap();
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
