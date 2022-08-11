use crate::primitives::*;
use crate::rules::ruleset::*;
use crate::util::*;
use crate::ai::gametree::SPerMinMaxStrategy;
use crate::game_analysis::determine_best_card_table::{internal_table, print_payoutstats};

pub fn subcommand(str_subcommand: &'static str) -> clap::Command {
    use super::clap_arg;
    use super::shared_args::*;
    clap::Command::new(str_subcommand)
        .about("Estimate strength of own hand")
        .arg(ruleset_arg())
        .arg(ai_arg())
        .arg(clap_arg("hand", "")
            .help("The cards on someone's hand")
            .long_help("The cards on someone's hand. Must be complete.")
        )
        .arg(clap_arg("position", "0")
            .help("Position of the player")
            .long_help("Position of the player. Players are numbere from 0 to 3, where 0 is the player to open the first stich (1, 2, 3 follow accordingly).")
        )
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
    let (vecoutputline, mapemmstrategyaformatinfo) = internal_table(
        allowed_rules(&ruleset.avecrulegroup[epi], hand)
            .filter_map(|orules| orules.map(|rules| { // do not rank None
                (
                    rules,
                    SPerMinMaxStrategy(ai.rank_rules(
                        hand,
                        epi,
                        rules.upcast(),
                        /*tpln_stoss_doubling*/(0, 0), // assume no stoss, no doublings in subcommand rank-rules
                        /*n_stock*/0, // assume no stock in subcommand rank-rules
                    ))
                )
            }))
            .collect::<Vec<_>>(),
        /*b_group*/false,
        /*fn_human_readable_payout*/&|f_payout| f_payout,
    );
    print_payoutstats(
        /*b_verbose*/false, // TODO make customizable
        &vecoutputline,
        &mapemmstrategyaformatinfo,
    );
    Ok(())
}
