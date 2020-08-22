use crate::primitives::*;
use crate::rules::{
    payoutdecider::*, rulesbettel::*, rulesramsch::*, rulesrufspiel::*, rulessolo::*, *,
};
use crate::util::*;
use std::{fs::File, io::prelude::*, path::Path};

#[derive(Debug, Clone)]
pub struct SRuleGroup {
    pub str_name : String,
    pub vecorules : Vec<Option<Box<dyn TActivelyPlayableRules>>>,
}

impl SRuleGroup {
    pub fn with_higher_prio_than(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<SRuleGroup> {
        let vecorules_steigered = self.vecorules.iter()
            .filter_map(|orules| match orules.as_ref().map(|rules| rules.with_higher_prio_than(prio, ebid)) {
                None => Some(None), // allow playing nothing
                Some(None) => None, // steigern was impossible
                Some(Some(rules_steigered)) => Some(Some(rules_steigered)), // steigern successful
            })
            .collect::<Vec<_>>();
        if !vecorules_steigered.is_empty() {
            Some(SRuleGroup {
                str_name: self.str_name.clone(),
                vecorules: vecorules_steigered,
            })
        } else {
            None
        }
    }

    pub fn allowed_rules<'retval, 'hand : 'retval, 'rules : 'retval>(&'rules self, hand: SFullHand<'hand>) -> impl Clone + Iterator<Item=Option<&'rules dyn TActivelyPlayableRules>> + 'retval {
        self.vecorules.iter().map(|orules| orules.as_ref().map(|rules| rules.as_ref()))
            .filter(move |orules| orules.map_or(true, |rules| rules.can_be_played(hand)))
    }
}

#[derive(Debug, Clone)]
pub enum VStockOrT<Stock, T> {
    Stock(Stock), // number must be positive, but use isize since it is essentially a payment
    OrT(T),
}

#[derive(Debug, Clone)]
pub enum EDoublingScope {
    Games,
    GamesAndStock,
}

#[derive(Clone, new, Debug)]
pub struct SStossParams {
    pub n_stoss_max : usize,
}

#[derive(new, Debug, Clone)]
pub struct SRuleSet {
    pub avecrulegroup : EnumMap<EPlayerIndex, Vec<SRuleGroup>>,
    pub stockorramsch : VStockOrT</*n_stock*/isize, Box<dyn TRules>>,
    pub oedoublingscope : Option<EDoublingScope>,
    pub ostossparams : Option<SStossParams>,
    pub ekurzlang : EKurzLang,
}

pub fn allowed_rules<'retval, 'hand : 'retval, 'rules : 'retval>(vecrulegroup: &'rules [SRuleGroup], hand: SFullHand<'hand>) -> impl Clone + Iterator<Item=Option<&'rules (dyn TActivelyPlayableRules + 'rules)>> + 'retval {
    vecrulegroup.iter()
        .flat_map(move |rulegroup| rulegroup.allowed_rules(hand))
}

impl SRuleSet {
    pub fn from_string(str_toml: &str) -> Result<SRuleSet, Error> {
        let tomltbl = str_toml.parse::<toml::Value>()?;
        let read_int = |tomlval: &toml::Value, str_key: &str| -> Result<i64, Error> {
            if let Some(n) = tomlval.get(str_key).and_then(|tomlval| tomlval.as_integer()) {
                if 0<=n {
                    Ok(n)
                } else {
                    bail!(format!("Found {} with invalid value {}. Must be at least 0.", str_key, n));
                }
            } else {
                bail!(format!("Could not find {}.", str_key));
            }
        };
        let fallback = |str_not_found: &str, str_fallback: &str| {
            info!("SRuleSet: Did not find {}. Falling back to {}.", str_not_found, str_fallback);
            read_int(&tomltbl, str_fallback)
        };
        // TODORULES "Der Alte muss"
        // TODORULES Kreuzspiel as alternative to Ramsch
        let stockorramsch = match (tomltbl.get("ramsch"), tomltbl.get("stock")) {
            (Some(_), Some(_)) => {
                // TODORULES Better alternatives? Allow stock *and* ramsch at the same time?
                bail!("Currently, having both Ramsch and Stock is not supported.")
            },
            (Some(val_ramsch), None) => {
                let durchmarsch = (match val_ramsch.get("durchmarsch") {
                    None => Ok(VDurchmarsch::None),
                    Some(&toml::Value::String(ref str_durchmarsch)) if "all"==str_durchmarsch => {
                        Ok(VDurchmarsch::All)
                    },
                    Some(&toml::Value::Integer(n_durchmarsch)) if 61<=n_durchmarsch && n_durchmarsch<=120 => {
                        Ok(VDurchmarsch::AtLeast(n_durchmarsch.as_num()))
                    },
                    _ => bail!("Invalid value for ramsch.durchmarsch. \"All\" or a number in [61; 120] is supported.")
                } as Result<_, Error>)?;
                read_int(val_ramsch, "price").map(|n_price|
                    VStockOrT::OrT(Box::new(
                        SRulesRamsch::new(n_price.as_num(), durchmarsch)
                    ) as Box<dyn TRules>)
                )
            },
            (None, Some(val_stock)) => {
                read_int(val_stock, "price").or_else(|_err| fallback("stock.price", "base-price")).map(|n_price| VStockOrT::Stock(n_price.as_num()))
            },
            (None, None) => {
                Ok(VStockOrT::Stock(0)) // represent "no stock" by using a zero stock payment
            }
        }?;
        let mut avecrulegroup = EPlayerIndex::map_from_fn(|_epi| Vec::new());
        for epi in EPlayerIndex::values() {
            let vecrulegroup = &mut avecrulegroup[epi];
            macro_rules! create_rulegroup {($str_rule_name_file: expr, $str_base_price_fallback: expr, $str_group_name: expr, $fn_rules: expr) => {
                if let Some(tomlval_game) = tomltbl.get($str_rule_name_file) {
                    let n_payout_extra = read_int(tomlval_game, "extra").or_else(|_err| fallback(&format!("{}.extra", $str_rule_name_file), "base-price"))?;
                    let n_payout_base = read_int(tomlval_game, "price").or_else(|_err| fallback(&format!("{}.price", $str_rule_name_file), $str_base_price_fallback))?;
                    let n_lauf_lbound = read_int(tomlval_game, "lauf-min").or_else(|_err| fallback(&format!("{}.lauf-min", $str_rule_name_file), "lauf-min"))?;
                    Ok(vecrulegroup.push(SRuleGroup{
                        str_name: $str_group_name.to_string(),
                        vecorules: ($fn_rules(SPayoutDeciderParams::new(
                            n_payout_base.as_num(),
                            /*n_payout_schneider_schwarz*/n_payout_extra.as_num(),
                            SLaufendeParams::new(
                                /*n_payout_per_lauf*/n_payout_extra.as_num(),
                                n_lauf_lbound.as_num(),
                            ),
                        ))),
                    })) as Result<_, Error>
                } else {
                    Ok(())
                }
            }};
            macro_rules! create_rulegroup_sololike {($str_rule_name_file: expr, $str_group_name: expr, $fn_rules: expr) => {
                create_rulegroup!($str_rule_name_file, "solo-price", $str_group_name, $fn_rules)
            }};
            vecrulegroup.push(SRuleGroup{
                str_name: "Nothing".to_string(),
                vecorules: vec![None],
            });
            create_rulegroup!(
                "rufspiel",
                "base-price",
                "Rufspiel", 
                |payoutparams: SPayoutDeciderParams| {
                    EFarbe::values()
                        .filter(|efarbe| EFarbe::Herz!=*efarbe)
                        .map(|efarbe| Some(Box::new(SRulesRufspiel::new(
                            epi,
                            efarbe,
                            payoutparams.clone(),
                        )) as Box<dyn TActivelyPlayableRules>))
                        .collect()
                }
            )?;
            macro_rules! read_sololike {($payoutdecider: ty, $fn_prio: expr, $str_rulename_suffix: expr) => {{
                type PayoutDecider = $payoutdecider;
                let internal_rulename = |str_rulename| {
                    format!("{}{}", str_rulename, $str_rulename_suffix)
                };
                macro_rules! vecrules{($itoefarbe: expr, $esololike: expr, $i_prioindex: expr) => {
                    |payoutparams: SPayoutDeciderParams| {
                        $itoefarbe
                            .map(|oefarbe| {
                                Some(sololike(
                                    epi,
                                    oefarbe,
                                    $esololike,
                                    PayoutDecider::new(payoutparams.clone(), $i_prioindex),
                                ))
                            })
                            .collect()
                    }
                }}
                create_rulegroup_sololike!(
                    "solo",
                    &internal_rulename("Solo"),
                    vecrules!(EFarbe::values(), ESoloLike::Solo, $fn_prio(0))
                )?;
                create_rulegroup_sololike!(
                    "wenz",
                    &internal_rulename("Wenz"),
                    vecrules!(std::iter::once(None), ESoloLike::Wenz, $fn_prio(-1))
                )?;
                create_rulegroup_sololike!(
                    "farbwenz",
                    &internal_rulename("Farbwenz"),
                    vecrules!(EFarbe::values(), ESoloLike::Wenz, $fn_prio(-2))
                )?;
                create_rulegroup_sololike!(
                    "geier",
                    &internal_rulename("Geier"),
                    vecrules!(std::iter::once(None), ESoloLike::Geier, $fn_prio(-3))
                )?;
                create_rulegroup_sololike!(
                    "farbgeier",
                    &internal_rulename("Farbgeier"),
                    vecrules!(EFarbe::values(), ESoloLike::Geier, $fn_prio(-4))
                )?;
            }}}
            if let Some(tomlval_steigern) = tomltbl.get("steigern") {
                let n_step = if let Some(n_step) = tomlval_steigern.get("step").and_then(|tomlval| tomlval.as_integer()) {
                    if 0<n_step {
                        n_step.as_num::<isize>()
                    } else {
                        info!("SRuleSet: Negative steigern.steps not permitted.");
                        10
                    }
                } else {
                    info!("SRuleSet: steigern.steps not specified");
                    10
                };
                read_sololike!(SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike>, |_i_prio| VGameAnnouncementPrioritySoloLike::SoloSteigern{n_points_to_win: 61, n_step}, "");
            } else {
                read_sololike!(SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike>, VGameAnnouncementPrioritySoloLike::SoloSimple, "");
            }
            read_sololike!(SPayoutDeciderTout, |x|x, " Tout");
            create_rulegroup_sololike!(
                "solo",
                "Sie",
                &|payoutparams| vec![Some(sololike(
                    epi,
                    /*oefarbe*/None,
                    ESoloLike::Solo,
                    SPayoutDeciderSie::new(payoutparams),
                ))]
            )?;
            { // Bettel
                let str_rule_name_file = "bettel";
                if let Some(tomlval_bettel) = tomltbl.get(str_rule_name_file) {
                    let n_payout_base = read_int(tomlval_bettel, "price")
                        .or_else(|_err| 
                            fallback(&format!("{}.price", str_rule_name_file), /*str_base_price_fallback*/"base-price")
                        )?;
                    fn push_bettel<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich>(vecrulegroup: &mut Vec<SRuleGroup>, epi: EPlayerIndex, n_payout_base: isize) {
                        vecrulegroup.push(SRuleGroup{
                            str_name: "Bettel".to_string(),
                            vecorules: vec![Some(Box::new(SRulesBettel::<BettelAllAllowedCardsWithinStich>::new(
                                epi,
                                /*i_prio, large negative number to make less important than any sololike*/-999_999,
                                n_payout_base.as_num::<isize>(),
                            )) as Box<dyn TActivelyPlayableRules>)],
                        });
                    }
                    if Some(true) == tomlval_bettel.get("stichzwang").and_then(|tomlval| tomlval.as_bool()) {
                        push_bettel::<SBettelAllAllowedCardsWithinStichStichzwang>(vecrulegroup, epi, n_payout_base.as_num::<isize>());
                    } else {
                        push_bettel::<SBettelAllAllowedCardsWithinStichNormal>(vecrulegroup, epi, n_payout_base.as_num::<isize>());
                    }
                }
            }
        }
        Ok(SRuleSet::new(
            avecrulegroup,
            stockorramsch,
            tomltbl.get("doubling").map(|tomlval_doubling | {
                if let Some(str_doubling_stock)=tomlval_doubling.get("stock").and_then(|tomlval| tomlval.as_str()) {
                    if "yes"==str_doubling_stock {
                        EDoublingScope::GamesAndStock
                    } else {
                        if "no"!=str_doubling_stock {
                            info!("SRuleSet: doubling.stock has invalid value '{}'. Falling back to 'no'", str_doubling_stock);
                        }
                        EDoublingScope::Games
                    }
                } else {
                    info!("SRuleSet: doubling.stock not specified; falling back to 'stock=yes'");
                    EDoublingScope::GamesAndStock
                }
            }),
            tomltbl.get("stoss").map(|tomlval_stoss| {
                let n_stoss_max_default = 4;
                SStossParams::new(
                    tomlval_stoss.get("max")
                        .and_then(|tomlval| tomlval.as_integer())
                        .map_or(n_stoss_max_default, |n_stoss_max| {
                            if n_stoss_max<=0 {
                                info!("SRuleSet: stoss.max less than 0. Defaulting to {}.", n_stoss_max_default);
                                n_stoss_max_default
                            } else {
                                n_stoss_max.as_num::<usize>()
                            }
                        })
                )
            }),
            match tomltbl.get("deck").and_then(|tomlval_kurzlang| tomlval_kurzlang.as_str()) {
                Some("kurz") => EKurzLang::Kurz,
                None | Some("lang") => EKurzLang::Lang,
                Some(str_kurzlang) => {
                    info!("SRuleSet: {} is not a valid value for 'deck' (supported values: kurz, lang). Defaulting to 'lang'", str_kurzlang);
                    EKurzLang::Lang
                },
            },
        ))
    }

    pub fn from_file(path: &Path) -> Result<SRuleSet, Error> {
        // TODO? ruleset creation wizard
        Self::from_string(&via_out_param_result(|str_toml| File::open(&path)?.read_to_string(str_toml))?.0)
    }
}

