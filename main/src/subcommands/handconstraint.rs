use openschafkopf_lib::{
    primitives::*,
    rules::{
        *,
        trumpfdecider::SLaufendeCount,
    },
};
use openschafkopf_util::*;
use plain_enum::{PlainEnum, EnumMap};
use as_num::*;
use std::{
    cmp::Ordering,
    fmt::{Display, Formatter},
    hash::{Hash, Hasher},
};

#[derive(Debug)]
pub struct SConstraint {
    engine: rhai::Engine,
    ast: rhai::AST,
    str_display: String,
}

type SRhaiUsize = i64; // TODO good idea?
type SRhaiEPlayerIndex = i64; // TODO good idea?

#[derive(Clone)]
struct SContext {
    stichseq: SStichSequence, // TODO how expensive is this?
    ahand: EnumMap<EPlayerIndex, SHand>,
    rules: SRules,
}

impl SContext {
    fn internal_count(&self, epi: EPlayerIndex, fn_pred: impl Fn(ECard)->bool) -> SRhaiUsize {
        self.ahand[epi]
            .cards()
            .iter()
            .copied()
            .filter(|card| fn_pred(*card))
            .count()
            .as_num::<SRhaiUsize>()
    }

    fn count(&self, i_epi: SRhaiUsize, fn_pred: impl Fn(ECard)->bool) -> SRhaiUsize {
        self.internal_count(unwrap!(EPlayerIndex::checked_from_usize(i_epi.as_num::<usize>())), fn_pred)
    }

    fn count_enummap(&self, fn_pred: impl Fn(&Self, ECard)->bool) -> rhai::Array {
        EPlayerIndex::map_from_fn(|epi| self.internal_count(epi, |card| fn_pred(self, card)))
            .into_raw()
            .into_iter()
            .map(rhai::Dynamic::from)
            .collect()
    }

    fn who_has_card_internal(&self, card: ECard) -> Option<EPlayerIndex> {
        EPlayerIndex::values().find(|&epi| self.ahand[epi].contains(card))
    }

    fn who_has_card(&self, card: ECard) -> SRhaiEPlayerIndex/*or -1*/ {
        self.who_has_card_internal(card)
            .map(|epi| epi.to_usize().as_num::<SRhaiEPlayerIndex>())
            .unwrap_or(-1)
    }
}

impl SConstraint {
    pub fn internal_eval(
        &self,
        stichseq: &SStichSequence,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        rules: SRules,
    ) -> Result<rhai::Dynamic, Box<rhai::EvalAltResult>> {
        self.engine.call_fn(
            &mut rhai::Scope::new(),
            &self.ast,
            "inspect",
            (SContext{stichseq: stichseq.clone(), ahand: ahand.clone(), rules},),
        )
    }
    pub fn eval(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rules: SRules) -> bool {
        match self.internal_eval(stichseq, ahand, rules) {
            Ok(dynamic) => {
                if let Ok(n) = dynamic.as_int() {
                    0 != n
                } else if let Ok(b) = dynamic.as_bool() {
                    b
                } else {
                    eprintln!("Unknown result data type. Interpreted as false.");
                    false
                }
            },
            Err(e) => {
                eprintln!("Error evaluating script ({e:?}).");
                false
            }
        }
    }
}

impl std::fmt::Display for SConstraint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "{}", self.str_display)
    }
}

impl std::str::FromStr for SConstraint {
    type Err = Error;
    fn from_str(str_in: &str) -> Result<Self, Self::Err> {
        let mut engine = rhai::Engine::new();
        let mut module_card = rhai::Module::new();
        let mut module_farbe = rhai::Module::new();
        let mut module_schlag = rhai::Module::new();
        let mut module_trumpforfarbe = rhai::Module::new();
        engine.set_strict_variables(true);
        engine
            .register_type::<SContext>()
            .register_type::<ECard>()
            .register_type::<EFarbe>()
            .register_type::<ESchlag>()
            .register_type::<VTrumpfOrFarbe>();
        fn register_output_fn<T: std::fmt::Display+std::fmt::Debug+Sync+Send+Clone+'static>(engine: &mut rhai::Engine) {
            engine
                .register_fn("to_string", |t: &mut T| t.to_string())
                .register_fn("to_debug", |t: &mut T| format!("{t:?}"));
        }
        fn register_equality_operators<T: Eq+Sync+Send+Clone+'static>(engine: &mut rhai::Engine) {
            engine.register_fn("==", |lhs: &mut T, rhs: T| {
                lhs == &rhs
            });
            engine.register_fn("!=", |lhs: &mut T, rhs: T| {
                lhs != &rhs
            });
        }
        register_output_fn::<ECard>(&mut engine);
        register_equality_operators::<ECard>(&mut engine);
        register_output_fn::<EFarbe>(&mut engine);
        register_equality_operators::<EFarbe>(&mut engine);
        register_output_fn::<ESchlag>(&mut engine);
        register_equality_operators::<ESchlag>(&mut engine);
        register_output_fn::<VTrumpfOrFarbe>(&mut engine);
        register_equality_operators::<VTrumpfOrFarbe>(&mut engine);
        fn register_count_fn(
            engine: &mut rhai::Engine,
            str_name: &str,
            fn_pred: impl Fn(&SContext, ECard)->bool + Clone + Send + Sync + 'static,
        ) {
            let fn_pred_clone = fn_pred.clone();
            engine.register_fn(str_name, move |ctx: SContext, i_epi: SRhaiUsize| {
                ctx.count(i_epi, |card| fn_pred_clone(&ctx, card))
            });
            engine.register_fn(str_name, move |ctx: SContext| {
                ctx.count_enummap(&fn_pred)
            });
        }
        fn register_parametrized_count_fn<T: Send+Sync+Clone+'static>(
            engine: &mut rhai::Engine,
            str_name: &str,
            fn_pred: impl Fn(&SContext, T, ECard)->bool + Clone + Send + Sync + 'static,
        ) {
            let fn_pred_clone = fn_pred.clone();
            engine.register_fn(str_name, move |ctx: SContext, t: T, i_epi: SRhaiUsize| {
                ctx.count(i_epi, |card| fn_pred_clone(&ctx, t.clone(), card))
            });
            engine.register_fn(str_name, move |ctx: SContext, t: T| {
                ctx.count_enummap(|ctx, card| fn_pred(ctx, t.clone(), card))
            });
        }
        let mut register_trumpforfarbe = |str_trumpforfarbe: &str, trumpforfarbe| {
            register_count_fn(&mut engine, str_trumpforfarbe, move |ctx, card| {
                ctx.rules.trumpforfarbe(card)==trumpforfarbe
            });
        };
        register_trumpforfarbe("trumpf", VTrumpfOrFarbe::Trumpf);
        module_trumpforfarbe.set_var("Trumpf", VTrumpfOrFarbe::Trumpf); 
        for (str_farbe_capitalized, efarbe) in [
            ("Eichel", EFarbe::Eichel),
            ("Gras", EFarbe::Gras),
            ("Herz", EFarbe::Herz),
            ("Schelln", EFarbe::Schelln),
        ] {
            register_trumpforfarbe(&str_farbe_capitalized.to_ascii_lowercase(), VTrumpfOrFarbe::Farbe(efarbe));
            module_farbe.set_var(str_farbe_capitalized, efarbe); 
            module_trumpforfarbe.set_var(str_farbe_capitalized, VTrumpfOrFarbe::Farbe(efarbe)); 

        }
        rhai::FuncRegistration::new("farbe")
            .with_namespace(rhai::FnNamespace::Internal)
            .with_purity(true)
            .with_volatility(false)
            .set_into_module(&mut module_trumpforfarbe, VTrumpfOrFarbe::Farbe);
        register_parametrized_count_fn(&mut engine, "trumpforfarbe", |ctx, trumpforfarbe, card| {
            ctx.rules.trumpforfarbe(card)==trumpforfarbe
        });
        for (str_schlag_capitalized, eschlag) in [
            ("Sieben", ESchlag::S7),
            ("Acht", ESchlag::S8),
            ("Neun", ESchlag::S9),
            ("Zehn", ESchlag::Zehn),
            ("Unter", ESchlag::Unter),
            ("Ober", ESchlag::Ober),
            ("Koenig", ESchlag::Koenig),
            ("Ass", ESchlag::Ass),
        ] {
            register_count_fn(&mut engine, &str_schlag_capitalized.to_ascii_lowercase(), move |_ctx, card| {
                card.schlag()==eschlag
            });
            module_schlag.set_var(str_schlag_capitalized, eschlag);
        }
        register_parametrized_count_fn(&mut engine, "schlag", |_ctx, eschlag, card| {
            card.schlag()==eschlag
        });
        rhai::FuncRegistration::new("new_card")
            .with_namespace(rhai::FnNamespace::Internal)
            .with_purity(true)
            .with_volatility(false)
            .set_into_module(&mut module_card, ECard::new);
        for card_for_fn in <ECard as PlainEnum>::values() {
            let str_card_lower = card_for_fn.to_string().to_lowercase();
            for str_card in [&str_card_lower, &str_card_lower.to_uppercase()] {
                module_card.set_var(str_card, card_for_fn);
                register_count_fn(&mut engine, str_card, move |_ctx, card_hand| {
                    card_hand==card_for_fn
                });
            }
            engine.register_fn(format!("who_has_{str_card_lower}"), move |ctx: SContext| -> SRhaiEPlayerIndex {
                ctx.who_has_card(card_for_fn)
            });
        }
        register_parametrized_count_fn(&mut engine, "card", |_ctx, card_queried, card| {
            card==card_queried
        });
        engine.register_fn("who_has_card", |ctx: SContext, card: ECard| ctx.who_has_card(card));
        engine
            .register_fn("hand_to_string", |ctx: SContext, i_epi: SRhaiUsize| -> String {
                format!("{}",
                    SDisplayCardSlice::new(
                        ctx.ahand[unwrap!(EPlayerIndex::checked_from_usize(i_epi.as_num::<usize>()))].cards().to_owned(),
                        &ctx.rules,
                    )
                )
            });
        engine
            .register_fn("laufende", |ctx: SContext| {
                if let Some(SLaufendeCount{n_laufende, b_primary_party}) = ctx.rules.count_laufende(
                    ctx.stichseq.kurzlang(),
                    /*fn_who_has_card*/|card| {
                        unwrap!(
                            ctx.who_has_card_internal(card)
                                .or_else(|| ctx.stichseq.visible_cards()
                                    .find(|&(_epi, card_visible)| card_visible==&card)
                                    .map(|(epi, _card)| epi)
                                )
                        )
                    },
                ) {
                    rhai::Dynamic::from(n_laufende.as_num::<rhai::INT>().neg_if(!b_primary_party))
                } else {
                    rhai::Dynamic::from("Rules do not support Laufende")
                }
            });
        engine
            .register_type::<EPlayerIndex>()
            .register_fn("to_string", EPlayerIndex::to_string)
        ;
        engine.register_static_module("card", module_card.into());
        engine.register_static_module("farbe", module_farbe.into());
        engine.register_static_module("schlag", module_schlag.into());
        engine.register_static_module("trumpforfarbe", module_trumpforfarbe.into());
        engine.compile(format!("fn inspect(ctx) {{ {str_in} }}"))
            .or_else(|_err|
                str_in.parse()
                    .map_err(|err| format_err!("Cannot parse path: {:?}", err))
                    .and_then(|path| engine.compile_file(path)
                        .map_err(|err| format_err!("Cannot compile file: {:?}", err))
                    )
            )
            .map(|ast| SConstraint{
                engine,
                ast,
                str_display: str_in.to_string(),
            })
    }
}

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

