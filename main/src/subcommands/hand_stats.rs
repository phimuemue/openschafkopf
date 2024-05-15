use openschafkopf_util::*;
use itertools::Itertools;
use super::common_given_game::*;
use failure::*;
use as_num::*;
use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
};

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
    impl Display for STotalOrderedFloat {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            self.0.fmt(f)
        }
    }
    impl Eq for STotalOrderedFloat {}
    impl PartialEq for STotalOrderedFloat {
        fn eq(&self, other: &Self) -> bool {
            self.0.total_cmp(&other.0)==Ordering::Equal
        }
    }
    impl PartialOrd for STotalOrderedFloat {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.0.total_cmp(&other.0))
        }
    }
    impl Ord for STotalOrderedFloat {
        fn cmp(&self, other: &Self) -> Ordering {
            self.0.total_cmp(&other.0)
        }
    }
    impl Hash for STotalOrderedFloat {
        fn hash<H: Hasher>(&self, state: &mut H) {
            self.0.to_bits().hash(state)
        }
    }
    #[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
    enum VInspectionResult<Number, Unknown> {
        Number(Number),
        Array(Vec<VInspectionResult<Number, Unknown>>),
        Unknown(Unknown),
    }
    #[derive(Hash, Eq, PartialEq, Ord, PartialOrd)]
    struct SUndefined;
    impl Display for SUndefined {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            write!(formatter, "\u{22a5}")
        }
    }
    impl <Number, Unknown> VInspectionResult<Number, Unknown> {
        fn map_numbers_remove_unknown<Number2>(&self, fn_number: &impl Fn(&Number)->Number2) -> VInspectionResult<Number2, SUndefined> {
            match self {
                VInspectionResult::Number(number) => VInspectionResult::Number(fn_number(number)),
                VInspectionResult::Unknown(_unknown) => VInspectionResult::Unknown(SUndefined),
                VInspectionResult::Array(vecinspectionresult) => VInspectionResult::Array(
                    vecinspectionresult.iter()
                        .map(|inspectionresult| inspectionresult.map_numbers_remove_unknown(fn_number))
                        .collect()
                ),
            }
        }
    }
    impl VInspectionResult<f64, SUndefined> {
        fn accumulate_weighted_sum(&mut self, inspectionresult: &Self, f_percentage: f64) {
            use VInspectionResult::*;
            match (self, inspectionresult) {
                (Number(ref mut number_self), Number(number_rhs)) => {
                    *number_self += number_rhs * f_percentage;
                },
                (Array(ref mut vecinspectionresult_self), Array(vecinspectionresult_rhs)) => {
                    itertools::zip_eq(vecinspectionresult_self, vecinspectionresult_rhs)
                        .for_each(|(lhs, rhs)| lhs.accumulate_weighted_sum(rhs, f_percentage));
                },
                (/*TODO why is slf needed?*/slf, _) => {
                    *slf = VInspectionResult::Unknown(SUndefined);
                },
            }
        }
    }
    impl VInspectionResult<VIntFloat, String> {
        fn new(dynamic: rhai::Dynamic) -> Self {
            if let Ok(n) = dynamic.as_int() {
                VInspectionResult::Number(VIntFloat::Int(n))
            } else if let Ok(f) = dynamic.as_float() {
                VInspectionResult::Number(VIntFloat::Float(STotalOrderedFloat(f)))
            } else if dynamic.is_array() {
                VInspectionResult::Array(
                    unwrap!(dynamic.into_array()).into_iter()
                        .map(VInspectionResult::new)
                        .collect()
                )
            } else {
                VInspectionResult::Unknown(dynamic.to_string())
            }
        }
    }
    impl<Number: Display, Unknown: Display> Display for VInspectionResult<Number, Unknown> {
        fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            match self {
                VInspectionResult::Number(n) => n.fmt(f),
                VInspectionResult::Unknown(unknown) => unknown.fmt(f),
                VInspectionResult::Array(vecinspectionresult) => {
                    // TODO itertools: Could join respect formatting width, etc?
                    write!(f, "[")?;
                    let mut b_first = true;
                    for inspectionresult in vecinspectionresult.iter() {
                        if !assign_neq(&mut b_first, false) {
                            write!(f, ", ")?;
                        }
                        inspectionresult.fmt(f)?;
                    }
                    write!(f, "]")
                },
            }
        }
    }
    #[derive(/*TODO? Hash by numeric value?*/Hash, Eq, PartialEq)]
    enum VIntFloat { // TODO distinction even useful?
        Int(rhai::INT),
        Float(STotalOrderedFloat),
    }
    impl Display for VIntFloat {
        fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
            match self {
                VIntFloat::Int(n) => n.fmt(formatter),
                VIntFloat::Float(f) => f.fmt(formatter),
            }
        }
    }
    impl VIntFloat {
        fn to_total_ordered_float(&self) -> STotalOrderedFloat {
            match self {
                VIntFloat::Int(n) => STotalOrderedFloat(n.as_num::<f64>()),
                VIntFloat::Float(STotalOrderedFloat(f)) => STotalOrderedFloat(*f),
            }
        }
    }
    impl PartialOrd for VIntFloat {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for VIntFloat {
        fn cmp(&self, other: &Self) -> Ordering {
            // Order by numerical value
            Ord::cmp(&self.to_total_ordered_float(), &other.to_total_ordered_float())
        }
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
                            |resdynamic| resdynamic
                                .map(VInspectionResult::new)
                                .map_err(|err| format!("Error: {:?}", err)),
                        ),
                    ).or_insert(0) += 1;
                }
                n_ahand_total += 1;
            }
            let percentage = |n_count: usize| n_count.as_num::<f64>()/n_ahand_total.as_num::<f64>();
            for (mapresinspectionresultn, constraint) in vectplmapresinspectionresultnconstraint {
                if b_verbose || 1<vecconstraint.len() {
                    println!("{}", constraint);
                }
                let mut oresinspectionresult_weighted_sum = None;
                for (resinspectionresult, n_count) in mapresinspectionresultn.into_iter()
                    .sorted_unstable_by(|lhs, rhs| Ord::cmp(&lhs.0, &rhs.0))
                {
                    let str_result_or_err = match resinspectionresult {
                        Ok(inspectionresult) => {
                            if let Ok(ref mut inspectionresult_weighted_sum) = oresinspectionresult_weighted_sum.get_or_insert_with(||
                                Ok(inspectionresult.map_numbers_remove_unknown(&|_| 0.,)) // Determine structure, initialize numbers with 0
                            ) {
                                inspectionresult_weighted_sum.accumulate_weighted_sum(
                                    &inspectionresult.map_numbers_remove_unknown(&|number| number.to_total_ordered_float().0),
                                    percentage(n_count),
                                );
                            }
                            format!("{}", inspectionresult)
                        },
                        Err(str_err) => {
                            oresinspectionresult_weighted_sum = Some(Err(())); // Do not show weighted sum if there are errors.
                            str_err
                        },
                    };
                    println!("{} {} ({:.2}%)", str_result_or_err, n_count, percentage(n_count)*100.);
                }
                if let Some(Ok(inspectionresult_weighted_sum))=oresinspectionresult_weighted_sum {
                    println!("-----");
                    println!("\u{2300} {:.4}", inspectionresult_weighted_sum);
                }
            }
            Ok(())
        }
    )
}
