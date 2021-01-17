pub mod analyze;
pub mod cli;
pub mod rank_rules;
pub mod suggest_card;
pub mod websocket;

use crate::util::*;
use crate::rules::ruleset::SRuleSet;
use crate::ai::SAi;
use crate::primitives::hand::SHand;

fn clap_arg(str_long: &'static str, str_default: &'static str) -> clap::Arg<'static, 'static> {
    clap::Arg::with_name(str_long)
        .long(str_long)
        .default_value(str_default)
}

pub fn get_ruleset(clapmatches: &clap::ArgMatches) -> Result<SRuleSet, Error> {
    SRuleSet::from_file(std::path::Path::new(unwrap!(clapmatches.value_of("ruleset"))))
}

pub fn ai(subcommand_matches: &clap::ArgMatches) -> SAi {
    match unwrap!(subcommand_matches.value_of("ai")) {
        "cheating" => SAi::new_cheating(/*n_rank_rules_samples*/50, /*n_suggest_card_branches*/2),
        "simulating" => 
            SAi::new_simulating(
                /*n_rank_rules_samples*/50,
                /*n_suggest_card_branches*/2,
                /*n_suggest_card_samples*/10,
            ),
        _ => {
            println!("Warning: AI not recognized. Defaulting to 'cheating'");
            SAi::new_cheating(/*n_rank_rules_samples*/50, /*n_suggest_card_branches*/2)
        }
    }
}

pub fn str_to_hand(str_hand: &str) -> Result<SHand, Error> {
    Ok(SHand::new_from_vec(crate::primitives::cardvector::parse_cards(str_hand).ok_or_else(||format_err!("Could not parse hand."))?))
}
