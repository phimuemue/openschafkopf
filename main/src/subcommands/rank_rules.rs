use crate::ai::*;
use crate::primitives::*;
use crate::rules::ruleset::*;
use crate::util::*;

pub fn rank_rules(ruleset: &SRuleSet, hand: SFullHand, epi: EPlayerIndex, ai: &SAi) {
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
    vecpairrulesf.sort_unstable_by(|pairrulesf_lhs, pairrulesf_rhs| debug_verify!(pairrulesf_rhs.1.partial_cmp(&pairrulesf_lhs.1)).unwrap());
    for (rules, f_avg_payout) in vecpairrulesf {
        println!("{}: {}", rules, f_avg_payout);
    }
}
