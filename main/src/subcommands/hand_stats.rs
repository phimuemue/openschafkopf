use crate::util::*;
use itertools::Itertools;
use super::common_given_game::*;

pub fn subcommand(str_subcommand: &'static str) -> clap::Command {
    subcommand_given_game(str_subcommand, "Statistics about hands that could be dealt.")
        .arg(clap::Arg::new("inspect")
            .long("inspect")
            .takes_value(true)
            .multiple_occurrences(true)
            .help("Describes inspection target")
            .long_help("Describes what the software will inspect. Example: \"EA(0)\" checks if player 0 has Eichel-Ass, \"T(2)\" counts the trumpf cards held by player 2. (Players are numbere from 0 to 3, where 0 is the player to open the first stich (1, 2, 3 follow accordingly).)") // TODO improve docs.
        )
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    let vecconstraint = unwrap!(clapmatches.values_of("inspect"))
        .map(|str_inspect| /*-> Result<_, Error>*/ {
            str_inspect.parse::<SConstraint>()
                .map_err(|_| format_err!("Cannot parse inspection target."))
        })
        .collect::<Result<Vec<_>,_>>()?;
    with_common_args(
        clapmatches,
        |itahand, rules, _stichseq, _ahand_fixed_with_holes, _epi_position, b_verbose| {
            let mut vecmapostrn = vecconstraint
                .iter()
                .map(|_constraint| std::collections::HashMap::new())
                .collect::<Vec<_>>();
            for ahand in itahand {
                // assert_eq!(ahand[epi_position], hand_fixed);
                for (mapostrn, constraint) in vecmapostrn.iter_mut()
                    .zip_eq(vecconstraint.iter())
                {
                    *mapostrn.entry(
                        constraint.internal_eval(
                            &ahand,
                            rules,
                            |resdynamic| resdynamic.ok().map(|dynamic| dynamic.to_string()),
                        ),
                    ).or_insert(0) += 1;
                }
            }
            vecmapostrn
                .into_iter()
                .zip_eq(vecconstraint.iter())
                .for_each(|(mapostrn, constraint)| {
                    if b_verbose || 1<vecconstraint.len() {
                        println!("{}", constraint);
                    }
                    for (ostr, n_count) in mapostrn.into_iter()
                        .sorted_unstable_by(|lhs, rhs| lhs.0.cmp(&rhs.0))
                    {
                        println!("{} {}", ostr.unwrap_or_else(|| "<Error>".into()), n_count);
                    }
                        
                });
            Ok(())
        }
    )
}
