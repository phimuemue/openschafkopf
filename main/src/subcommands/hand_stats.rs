use crate::ai::*;
use crate::primitives::*;
use crate::util::*;

use super::common_given_game::*;

pub fn subcommand(str_subcommand: &'static str) -> clap::Command {
    subcommand_given_game(str_subcommand, "Statistics about hands that could be dealt.")
        .arg(clap::Arg::new("inspect")
            .long("inspect")
            .takes_value(true)
            .multiple_occurrences(true)
            .help("Describes inspection target")
            .long_help("Describes what the software will inspect. Example: \"EA(0)\" checks if player 0 has Eichel-Ass, \"T(2)\" counts the trumpf cards held by player 2. (Players are numbere from 0 to 3, where 0 is the player to open the first stich (1, 2, 3 follow accordingly).)")
        )
}

pub fn run(clapmatches: &clap::ArgMatches) -> Result<(), Error> {
    struct SWithCommonArgs<'argmatches> {
        clapmatches: &'argmatches clap::ArgMatches,
    }
    impl<'argmatches> TWithCommonArgs for SWithCommonArgs<'argmatches> {
        fn call<'rules>(
            self,
            itahand: Box<dyn Iterator<Item=EnumMap<EPlayerIndex, SHand>>+Send+'rules>,
            _eremainingcards: ERemainingCards,
            determinebestcard: SDetermineBestCard,
            _b_verbose: bool,
        ) -> Result<(), Error> {
            let clapmatches = self.clapmatches;
            let vecconstraint = unwrap!(clapmatches.values_of("inspect"))
                .map(|str_inspect| /*-> Result<_, Error>*/ {
                    str_inspect.parse::<VConstraint>()
                        .map_err(|_| format_err!("Cannot parse inspection target."))
                })
                .collect::<Result<Vec<_>,_>>()?;
            for constraint in vecconstraint.iter() {
                println!("{:?}", constraint);
            }
            #[derive(PartialOrd, Ord, Hash, PartialEq, Eq)]
            enum VInspectValue {
                Usize(usize),
                Bool(bool),
            }
            let mut mapvecinspectvaluen = std::collections::HashMap::<Vec<_>,_>::new();
            for ahand in itahand {
                *mapvecinspectvaluen
                    .entry(
                        vecconstraint.iter()
                            .map(|constraint| 
                                constraint.internal_eval(&ahand, determinebestcard.rules, VInspectValue::Bool, VInspectValue::Usize),
                            )
                            .collect()
                    )
                    .or_insert(0) += 1;
            }
            let mut vectplvecinspectvaluen = mapvecinspectvaluen.into_iter().collect::<Vec<_>>();
            vectplvecinspectvaluen.sort_unstable_by(|lhs, rhs| lhs.0.cmp(&rhs.0));
            for (vecinspectvalue, n_count) in vectplvecinspectvaluen {
                for inspectvalue in vecinspectvalue {
                    print!(
                        "{} ",
                        match inspectvalue {
                            VInspectValue::Usize(n_val) => format!("{}", n_val),
                            VInspectValue::Bool(b_val) => format!("{}", b_val),
                        }
                    );
                }
                println!("{}", n_count);
            }
            Ok(())
        }
    }
    with_common_args(clapmatches, SWithCommonArgs{clapmatches})
}
