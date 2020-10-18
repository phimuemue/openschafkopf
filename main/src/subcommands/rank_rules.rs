use crate::primitives::*;
use crate::rules::ruleset::*;
use crate::util::*;

pub fn rank_rules(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let ruleset = super::get_ruleset(clapmatches)?;
    let hand = super::str_to_hand(&clapmatches.value_of("hand").ok_or_else(||format_err!("No hand given as parameter."))?)?;
    let hand = Some(hand).filter(|hand| hand.cards().len()==ruleset.ekurzlang.cards_per_player()).ok_or_else(||format_err!("Could not convert hand to a full hand of cards"))?;
    let hand = SFullHand::new(&hand, ruleset.ekurzlang);
    use clap::value_t;
    let epi = value_t!(clapmatches.value_of("position"), EPlayerIndex).unwrap_or(EPlayerIndex::EPI0);
    let ai = super::ai(clapmatches);
    println!("Hand: {}", hand.get());
    let mut vecpairrulesf = allowed_rules(&ruleset.avecrulegroup[epi], hand)
        .filter_map(|orules| orules.map(|rules| { // do not rank None
            (
                rules,
                ai.rank_rules(
                    hand,
                    epi,
                    rules.upcast(),
                    /*tpln_stoss_doubling*/(0, 0), // assume no stoss, no doublings in subcommand rank-rules
                    /*n_stock*/0, // assume no stock in subcommand rank-rules
                ),
            )
        }))
        .collect::<Vec<_>>();
    vecpairrulesf.sort_unstable_by(|pairrulesf_lhs, pairrulesf_rhs| unwrap!(pairrulesf_rhs.1.partial_cmp(&pairrulesf_lhs.1)));
    for (rules, f_avg_payout) in vecpairrulesf {
        println!("{}: {}", rules, f_avg_payout);
    }
    Ok(())
}
