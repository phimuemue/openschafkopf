use crate::primitives::*;
use crate::util::*;
use crate::rules::*;

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
    ahand: EnumMap<EPlayerIndex, SHand>,
    rules: Box<dyn TRules>,
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
}

impl SConstraint {
    pub fn internal_eval<R>(
        &self,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        rules: &dyn TRules,
        fn_eval: impl Fn(Result<rhai::Dynamic, Box<rhai::EvalAltResult>>)->R,
    ) -> R {
        fn_eval(self.engine.call_fn(
            &mut rhai::Scope::new(),
            &self.ast,
            "inspect",
            (SContext{ahand: ahand.clone(), rules: rules.box_clone()},)
        ))
    }
    pub fn eval(&self, ahand: &EnumMap<EPlayerIndex, SHand>, rules: &dyn TRules) -> bool {
        self.internal_eval(ahand, rules, |resdynamic| {
            match resdynamic {
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
                    eprintln!("Error evaluating script ({:?}).", e);
                    false
                }
            }
        })
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
        engine.set_strict_variables(true);
        engine
            .register_type::<SContext>();
        fn register_count_fn(
            engine: &mut rhai::Engine,
            str_name: &str,
            fn_pred: impl Fn(&SContext, ECard)->bool + Clone + Send + Sync + 'static,
        ) {
            let fn_pred_clone = fn_pred.clone();
            engine.register_fn(str_name, move |ctx: SContext, i_epi: SRhaiUsize| {
                ctx.count(i_epi, |card| fn_pred_clone(&ctx, card))
            });
            engine.register_fn(str_name, move |ctx: SContext| -> rhai::Array {
                EPlayerIndex::map_from_fn(|epi| ctx.internal_count(epi, |card| fn_pred(&ctx, card)))
                    .into_raw()
                    .into_iter()
                    .map(rhai::Dynamic::from)
                    .collect()
            });
        }
        for (str_trumpforfarbe, trumpforfarbe) in [
            ("trumpf", VTrumpfOrFarbe::Trumpf),
            ("eichel", VTrumpfOrFarbe::Farbe(EFarbe::Eichel)),
            ("gras", VTrumpfOrFarbe::Farbe(EFarbe::Gras)),
            ("herz", VTrumpfOrFarbe::Farbe(EFarbe::Herz)),
            ("schelln", VTrumpfOrFarbe::Farbe(EFarbe::Schelln)),
        ] {
            register_count_fn(&mut engine, str_trumpforfarbe, move |ctx, card| {
                ctx.rules.trumpforfarbe(card)==trumpforfarbe
            });
        }
        for (str_schlag, eschlag) in [
            ("sieben", ESchlag::S7),
            ("acht", ESchlag::S8),
            ("neun", ESchlag::S9),
            ("zehn", ESchlag::Zehn),
            ("unter", ESchlag::Unter),
            ("ober", ESchlag::Ober),
            ("koenig", ESchlag::Koenig),
            ("ass", ESchlag::Ass),
        ] {
            register_count_fn(&mut engine, str_schlag, move |_ctx, card| {
                card.schlag()==eschlag
            });
        }
        for card_for_fn in <ECard as PlainEnum>::values() {
            register_count_fn(&mut engine, &card_for_fn.to_string().to_lowercase(), move |_ctx, card_hand| {
                card_hand==card_for_fn
            });
            register_count_fn(&mut engine, &card_for_fn.to_string().to_uppercase(), move |_ctx, card_hand| {
                card_hand==card_for_fn
            });
            engine.register_fn(format!("who_has_{}", card_for_fn.to_string().to_lowercase()), move |ctx: SContext| -> SRhaiEPlayerIndex {
                unwrap!(EPlayerIndex::values().find(|&epi| ctx.ahand[epi].contains(card_for_fn)))
                    .to_usize()
                    .as_num::<SRhaiEPlayerIndex>()
            });
        }
        engine
            .register_type::<EPlayerIndex>()
            .register_fn("to_string", EPlayerIndex::to_string)
        ;
        engine.compile(format!("fn inspect(ctx) {{ {} }}", str_in))
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

