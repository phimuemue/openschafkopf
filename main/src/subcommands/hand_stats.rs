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
            .long_help("Describes what the software will inspect. Example: \"ctx.ea(0)\" checks if player 0 has Eichel-Ass, \"ctx.trumpf(2)\" counts the trumpf cards held by player 2. (Players are numbere from 0 to 3, where 0 is the player to open the first stich (1, 2, 3 follow accordingly).)") // TODO improve docs.
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
            #[derive(PartialOrd, Ord, Hash, PartialEq, Eq)]
            enum VInspectValue {
                Usize(usize),
                Bool(bool),
                Str(String), // For now, all "special" things are represented as strings. TODO? good idea?
                Error,
            }
            let mut vecmapinspectvaluen = vecconstraint
                .iter()
                .map(|_constraint| std::collections::HashMap::new())
                .collect::<Vec<_>>();
            for ahand in itahand {
                // assert_eq!(ahand[epi_position], hand_fixed);
                for (mapinspectvaluen, constraint) in vecmapinspectvaluen.iter_mut()
                    .zip_eq(vecconstraint.iter())
                {
                    *mapinspectvaluen.entry(
                        constraint.internal_eval(
                            &ahand,
                            rules,
                            VInspectValue::Bool,
                            VInspectValue::Usize,
                            |odynamic| {
                                if let Some(dynamic)=odynamic {
                                    VInspectValue::Str(dynamic.to_string())
                                } else {
                                    VInspectValue::Error
                                }
                            },
                        ),
                    ).or_insert(0) += 1;
                }
            }
            vecmapinspectvaluen
                .into_iter()
                .zip_eq(vecconstraint.iter())
                .for_each(|(mapinspectvaluen, constraint)| {
                    if b_verbose || 1<vecconstraint.len() {
                        println!("{}", constraint);
                    }
                    let mut vectplinspectvaluen = mapinspectvaluen.into_iter().collect::<Vec<_>>();
                    vectplinspectvaluen.sort_unstable_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
                    for (inspectvalue, n_count) in vectplinspectvaluen {
                        print!(
                            "{} ",
                            match inspectvalue {
                                VInspectValue::Usize(n_val) => format!("{}", n_val),
                                VInspectValue::Bool(b_val) => format!("{}", b_val),
                                VInspectValue::Str(str_val) => str_val,
                                VInspectValue::Error => "<Error>".into(),
                            }
                        );
                        println!("{}", n_count);
                    }
                        
                });
            Ok(())
        }
    )
}
