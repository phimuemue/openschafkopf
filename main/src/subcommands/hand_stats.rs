use openschafkopf_util::*;
use itertools::Itertools;
use super::common_given_game::*;
use failure::*;
use as_num::*;

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
    #[derive(Clone, Copy)]
    struct STotalOrderedFloat(rhai::FLOAT); // TODO good idea?
    impl Eq for STotalOrderedFloat {}
    impl PartialEq for STotalOrderedFloat {
        fn eq(&self, other: &STotalOrderedFloat) -> bool {
            self.0.total_cmp(&other.0)==std::cmp::Ordering::Equal
        }
    }
    impl std::hash::Hash for STotalOrderedFloat {
        fn hash<H>(&self, state: &mut H)
            where H: std::hash::Hasher
        {
            self.0.to_bits().hash(state)
        }
    }
    #[derive(Hash, Eq, PartialEq)]
    enum VInspectionResult<Number> {
        Number(Number),
        Unknown(String),
    }
    #[derive(Hash, Eq, PartialEq)]
    enum VIntFloat<Float> {
        Int(rhai::INT),
        Float(Float),
    }
    with_common_args(
        clapmatches,
        |itahand, rules, _stichseq, _ahand_fixed_with_holes, _epi_position, _expensifiers, b_verbose| {
            let mut vectplmapresinspectionresultnconstraint = vecconstraint
                .iter()
                .map(|constraint| (std::collections::HashMap::new(), constraint))
                .collect::<Vec<_>>();
            let mut n_ahand_total = 0;
            for ahand in itahand {
                // assert_eq!(ahand[epi_position], hand_fixed);
                for (mapresinspectionresultn, constraint) in vectplmapresinspectionresultnconstraint.iter_mut() {
                    *mapresinspectionresultn.entry(
                        constraint.internal_eval(
                            &ahand,
                            rules.clone(),
                            |resinspectionresult| resinspectionresult
                                .map(|dynamic| {
                                    if let Ok(n) = dynamic.as_int() {
                                        VInspectionResult::Number(VIntFloat::Int(n))
                                    } else if let Ok(f) = dynamic.as_float() {
                                        VInspectionResult::Number(VIntFloat::Float(STotalOrderedFloat(f)))
                                    } else {
                                        VInspectionResult::Unknown(dynamic.to_string())
                                    }
                                })
                                .map_err(|err| format!("Error: {:?}", err)),
                        ),
                    ).or_insert(0) += 1;
                }
                n_ahand_total += 1;
            }
            for (mapresinspectionresultn, constraint) in vectplmapresinspectionresultnconstraint {
                if b_verbose || 1<vecconstraint.len() {
                    println!("{}", constraint);
                }
                let mut of_weighted_sum = Some(0.);
                for (resstr, n_count, f_percentage) in mapresinspectionresultn.into_iter()
                    .map(|(resinspectionresult, n_count)| {
                        let f_percentage = n_count.as_num::<f64>()/n_ahand_total.as_num::<f64>();
                        (
                            resinspectionresult.map(|inspectionresult| match inspectionresult {
                                VInspectionResult::Number(number) => {
                                    let (str_result, f_summand) = match number {
                                        VIntFloat::Int(n) => (format!("{}", n), n.as_num::<f64>()),
                                        VIntFloat::Float(STotalOrderedFloat(f)) => (format!("{}", f), f.into()),
                                    };
                                    if let Some(ref mut f_weighted_sum) = of_weighted_sum {
                                        *f_weighted_sum += f_summand * f_percentage;
                                    };
                                    str_result
                                },
                                VInspectionResult::Unknown(str_unknown) => {
                                    of_weighted_sum = None;
                                    str_unknown
                                },
                            }),
                            n_count,
                            f_percentage,
                        )
                    })
                    .sorted_unstable_by(|lhs, rhs| lhs.0.cmp(&rhs.0))
                {
                    println!("{} {} ({:.2}%)", resstr.unwrap_or_else(|err| err), n_count, f_percentage*100.);
                }
                if let Some(f_weighted_sum)=of_weighted_sum {
                    println!("-----");
                    println!("\u{2300} {:.2}", f_weighted_sum);
                }
            }
            Ok(())
        }
    )
}
