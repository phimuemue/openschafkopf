use openschafkopf_util::*;
use as_num::*;
use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
};

#[derive(Debug, Clone, Copy)]
pub struct STotalOrderedFloat(pub rhai::FLOAT); // TODO good idea?
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
        Some(self.cmp(other))
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

#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub enum VInspectionResult<Number, Unknown> {
    RecognizableAsNumber(Number),
    Array(Vec<VInspectionResult<Number, Unknown>>),
    Unknown(Unknown),
}
#[derive(Debug, Hash, Eq, PartialEq, Ord, PartialOrd)]
pub struct SUndefined;
impl Display for SUndefined {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(formatter, "\u{22a5}")
    }
}
impl <Number, Unknown> VInspectionResult<Number, Unknown> {
    pub fn map_numbers_remove_unknown<Number2>(&self, fn_number: &impl Fn(&Number)->Number2) -> VInspectionResult<Number2, SUndefined> {
        match self {
            VInspectionResult::RecognizableAsNumber(number) => VInspectionResult::RecognizableAsNumber(fn_number(number)),
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
    pub fn accumulate_weighted_sum(&mut self, inspectionresult: &Self, f_percentage: f64) {
        match_same_variants!(match (&mut *self, inspectionresult) {
            VInspectionResult::RecognizableAsNumber(number_self), (number_rhs) => {
                *number_self += number_rhs * f_percentage;
            },
            VInspectionResult::Array(vecinspectionresult_self), (vecinspectionresult_rhs) => {
                if vecinspectionresult_self.len()==vecinspectionresult_rhs.len() {
                    itertools::zip_eq(vecinspectionresult_self, vecinspectionresult_rhs)
                        .for_each(|(lhs, rhs)| lhs.accumulate_weighted_sum(rhs, f_percentage));
                } else {
                    *self = VInspectionResult::Unknown(SUndefined);
                }
            },
            VInspectionResult::Unknown(SUndefined), (SUndefined) => {
                // No need to update self
            },
            _ => {
                *self = VInspectionResult::Unknown(SUndefined);
            },
        })
    }
}
impl VInspectionResult<VRecognizableAsNumber, String> {
    pub fn new(dynamic: rhai::Dynamic) -> Self {
        if let Ok(b) = dynamic.as_bool() {
            VInspectionResult::RecognizableAsNumber(VRecognizableAsNumber::Bool(b))
        } else if let Ok(n) = dynamic.as_int() {
            VInspectionResult::RecognizableAsNumber(VRecognizableAsNumber::Int(n))
        } else if let Ok(f) = dynamic.as_float() {
            VInspectionResult::RecognizableAsNumber(VRecognizableAsNumber::Float(STotalOrderedFloat(f)))
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
            VInspectionResult::RecognizableAsNumber(n) => n.fmt(f),
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
#[derive(/*TODO? Hash by numeric value?*/Hash, Eq, PartialEq, Debug)]
pub enum VRecognizableAsNumber { // TODO distinction even useful?
    Int(rhai::INT),
    Float(STotalOrderedFloat),
    Bool(bool),
}
impl Display for VRecognizableAsNumber {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            VRecognizableAsNumber::Int(n) => n.fmt(formatter),
            VRecognizableAsNumber::Float(f) => f.fmt(formatter),
            VRecognizableAsNumber::Bool(b) => b.fmt(formatter),
        }
    }
}
impl VRecognizableAsNumber {
    pub fn to_total_ordered_float(&self) -> STotalOrderedFloat {
        STotalOrderedFloat(match self {
            VRecognizableAsNumber::Int(n) => n.as_num::<f64>(),
            VRecognizableAsNumber::Float(STotalOrderedFloat(f)) => *f,
            VRecognizableAsNumber::Bool(b) => usize::from(*b).as_num::<f64>(),
        })
    }
}
impl PartialOrd for VRecognizableAsNumber {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for VRecognizableAsNumber {
    fn cmp(&self, other: &Self) -> Ordering {
        // Order by numerical value
        Ord::cmp(&self.to_total_ordered_float(), &other.to_total_ordered_float())
    }
}

