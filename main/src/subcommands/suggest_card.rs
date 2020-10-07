use crate::ai::{*, handiterators::*, suspicion::*};
use crate::game::SStichSequence;
use crate::primitives::*;
use crate::util::*;

plain_enum_mod!(moderemainingcards, ERemainingCards {_1, _2, _3, _4, _5, _6, _7, _8,});

pub fn suggest_card(
    str_rules_with_epi: &str,
    hand_fixed: &SHand,
    slccard_as_played: &[SCard],
    otpln_branching_factor: Option<(usize, usize)>,
    ostr_itahand: Option<&str>,
    b_verbose: bool,
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
    let epi_fixed = determinebestcard.epi_fixed;
    let eremainingcards = debug_verify!(ERemainingCards::checked_from_usize(remaining_cards_per_hand(&stichseq)[epi_fixed] - 1)).unwrap();
    let determinebestcardresult = { // we are interested in payout => single-card-optimization useless
        macro_rules! forward{(($itahand: expr), ($func_filter_allowed_cards: expr), ($foreachsnapshot: ident),) => {{ // TODORUST generic closures
            determine_best_card(
                &determinebestcard,
                $itahand
                    .inspect(|ahand| {
                        if b_verbose { // TODO? dispatch statically
                            // TODO make output pretty
                            for hand in ahand.iter() {
                                print!("{} | ", hand);
                            }
                            println!("");
                        }
                    }),
                $func_filter_allowed_cards,
                &$foreachsnapshot::new(
                    rules,
                    epi_fixed,
                    /*tpln_stoss_doubling*/(0, 0), // TODO? make customizable
                    /*n_stock*/0, // TODO? make customizable
                ),
                |minmax_acc, minmax| {
                    minmax_acc.assign_min_by_key(&minmax, epi_fixed);
                },
                /*opath_out_dir*/None, // TODO? make customizable
            )
        }}}
        enum VChooseItAhand {
            All,
            Sample(usize),
        };
        use VChooseItAhand::*;
        let oiteratehands = if_then_some!(let Some(str_itahand)=ostr_itahand,
            if "all"==str_itahand.to_lowercase() {
                All
            } else {
                Sample(str_itahand.parse()?)
            }
        );
        use ERemainingCards::*;
        cartesian_match!(
            forward,
            match ((oiteratehands, eremainingcards)) {
                (Some(All), _)|(None, _1)|(None, _2)|(None, _3)|(None, _4) => (all_possible_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)),
                (Some(Sample(n_samples)), _) => (forever_rand_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                    .take(n_samples)),
                (None, _5)|(None, _6)|(None, _7)|(None, _8) => (forever_rand_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                    .take(/*n_suggest_card_samples*/50)),
            },
            match ((otpln_branching_factor, eremainingcards)) {
                (Some((n_lo, n_hi)), _) => (&branching_factor(move |_stichseq| {
                    let n_lo = n_lo.max(1);
                    (n_lo, (n_hi.max(n_lo+1)))
                })),
                (None,_1)|(None,_2)|(None,_3)|(None,_4) => (&|_,_| (/*no filtering*/)),
                (None,_5)|(None,_6)|(None,_7)|(None,_8) => (&branching_factor(|_stichseq| (1, 3))),
            },
            match (eremainingcards) {
                _1|_2|_3 => (SMinReachablePayout),
                _4|_5|_6|_7|_8 => (SMinReachablePayoutLowerBoundViaHint),
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
