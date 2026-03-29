use openschafkopf_util::*;
use openschafkopf_lib::rules::ruleset::SRuleSet;
use failure::Error;

pub fn clap_arg(str_long: &'static str, str_default: &'static str) -> clap::Arg<'static> {
    clap::Arg::new(str_long)
        .long(str_long)
        .default_value(str_default)
}

pub fn ai_arg() -> clap::Arg<'static> {
    clap_arg("ai", "cheating")
        .help("Describes whether AI has access to all players' cards")
        .long_help("Describes whether the AI plays fair or has access to all players' cards.")
        .possible_values(["cheating", "simulating"]) // TODO custom validator?
}

pub fn ruleset_arg() -> clap::Arg<'static> {
    clap_arg("ruleset", "rulesets/default.toml")
        .help("Path to a ruleset TOML file.")
        .long_help("The TOML ruleset file describes the set of available rules, prices, what to do if no one wants to play, whether stoss and doubling is allowed.")
}

pub fn get_ruleset(clapmatches: &clap::ArgMatches) -> Result<SRuleSet, Error> {
    SRuleSet::from_file(std::path::Path::new(unwrap!(clapmatches.value_of("ruleset"))))
}

