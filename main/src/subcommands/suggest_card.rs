use crate::ai::{*, handiterators::*, suspicion::*};
use crate::game::SStichSequence;
use crate::primitives::*;
use crate::util::*;

plain_enum_mod!(moderemainingcards, ERemainingCards {_1, _2, _3, _4, _5, _6, _7, _8,});

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
    let epi_fixed = determinebestcard.epi_fixed;
    let eremainingcards = debug_verify!(ERemainingCards::checked_from_usize(remaining_cards_per_hand(determinebestcard.stichseq)[epi_fixed] - 1)).unwrap();
    let determinebestcardresult = { // we are interested in payout => single-card-optimization useless
        macro_rules! forward{(($itahand: expr), ($func_filter_allowed_cards: expr, $foreachsnapshot: ident,),) => {{ // TODORUST generic closures
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
        use ERemainingCards::*;
        cartesian_match!(
            forward,
            match (eremainingcards) {
                _1|_2|_3|_4 => (all_possible_hands(determinebestcard.stichseq, determinebestcard.hand_fixed.clone(), epi_fixed, determinebestcard.rules)),
                _5|_6|_7|_8 => (forever_rand_hands(determinebestcard.stichseq, determinebestcard.hand_fixed.clone(), epi_fixed, determinebestcard.rules)
                    .take(n_suggest_card_samples)),
            },
            match (eremainingcards) {
                _1|_2|_3 => (
                    &|_,_| (/*no filtering*/),
                    SMinReachablePayout,
                ),
                _4 => (
                    &|_,_| (/*no filtering*/),
                    SMinReachablePayoutLowerBoundViaHint,
                ),
                _5|_6|_7|_8 => (
                    &branching_factor(|_stichseq| (1, n_suggest_card_branches+1)),
                    SMinReachablePayoutLowerBoundViaHint,
                ),
            },
        )
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
