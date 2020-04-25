use crate::ai::*;
use crate::primitives::*;
use crate::rules::ruleset::*;

pub fn rank_rules(ruleset: &SRuleSet, hand: SFullHand, epi: EPlayerIndex, ai: &SAi) {
    for rules in allowed_rules(&ruleset.avecrulegroup[epi], hand)
        .filter_map(|orules| orules) // do not rank None
    {
        println!("Hand: {}", hand.get());
        println!("{}: {}",
            rules,
            ai.rank_rules(
                hand,
                EPlayerIndex::EPI0,
                epi,
                rules.upcast(),
                /*tpln_stoss_doubling*/(0, 0), // assume no stoss, no doublings in subcommand rank-rules
                /*n_stock*/0, // assume no stock in subcommand rank-rules
            )
        );
    }
}
