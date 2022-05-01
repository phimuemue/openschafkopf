use crate::primitives::*;
use crate::rules::ruleset::*;
use crate::util::*;

pub fn subcommand(str_subcommand: &'static str) -> clap::Command {
    use super::clap_arg;
    clap::Command::new(str_subcommand)
        .about("Estimate strength of own hand")
        .arg(clap_arg("ruleset", "rulesets/default.toml"))
        .arg(clap_arg("ai", "cheating"))
        .arg(clap_arg("hand", ""))
        .arg(clap_arg("position", "0"))
        // TODO align arguments with suggest-card
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let ruleset = super::get_ruleset(clapmatches)?;
    let hand = super::str_to_hand(clapmatches.value_of("hand").ok_or_else(||format_err!("No hand given as parameter."))?)?;
    let hand = Some(hand).filter(|hand| hand.cards().len()==ruleset.ekurzlang.cards_per_player()).ok_or_else(||format_err!("Could not convert hand to a full hand of cards"))?;
    let hand = SFullHand::new(hand.cards(), ruleset.ekurzlang);
    let epi = clapmatches.value_of_t("position").unwrap_or(EPlayerIndex::EPI0);
    let ai = super::ai(clapmatches);
    println!("Hand: {}", SDisplayCardSlice(hand.get()));
    let mut vectplrulesf = allowed_rules(&ruleset.avecrulegroup[epi], hand)
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
    vectplrulesf.sort_unstable_by(|tplrulesf_lhs, tplrulesf_rhs| unwrap!(tplrulesf_rhs.1.partial_cmp(&tplrulesf_lhs.1)));
    for (rules, f_avg_payout) in vectplrulesf {
        println!("{}: {}", rules, f_avg_payout);
    }
    Ok(())
}
