pub mod analyze;
pub mod cli;
pub mod rank_rules;
pub mod suggest_card;
pub mod websocket;

use crate::util::*;
use crate::rules::ruleset::SRuleSet;

pub fn get_ruleset(clapmatches: &clap::ArgMatches) -> Result<SRuleSet, Error> {
    SRuleSet::from_file(std::path::Path::new(unwrap!(clapmatches.value_of("ruleset"))))
}
