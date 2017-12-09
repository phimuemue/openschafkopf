use primitives::*;
use rules::*;
use rules::rulesrufspiel::*;
use rules::rulessolo::*;
use rules::rulesbettel::*;
use rules::rulesramsch::*;
use rules::trumpfdecider::*;
use rules::payoutdecider::*;
use util::*;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use toml;

#[derive(Debug)]
pub struct SRuleGroup {
    pub str_name : String,
    pub vecrules : Vec<Box<TActivelyPlayableRules>>,
}

impl SRuleGroup {
    pub fn with_higher_prio_than(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<SRuleGroup> {
        let vecrules_steigered = self.vecrules.iter()
            .filter_map(|rules| rules.with_higher_prio_than(prio, ebid))
            .collect::<Vec<_>>();
        if 0<vecrules_steigered.len() {
            Some(SRuleGroup {
                str_name: self.str_name.clone(),
                vecrules: vecrules_steigered,
            })
        } else {
            None
        }
    }
}

#[derive(Debug)]
pub enum VStockOrT<T> {
    Stock(/*n_stock*/isize), // number must be positive, but use isize since it is essentially a payment
    OrT(T),
}

#[derive(Debug)]
pub enum EDoublingScope {
    Games,
    GamesAndStock,
}

#[derive(Clone, new, Debug)]
pub struct SStossParams {
    pub n_stoss_max : usize,
}

#[derive(new, Debug)]
pub struct SRuleSet {
    pub avecrulegroup : EnumMap<EPlayerIndex, Vec<SRuleGroup>>,
    pub stockorramsch : VStockOrT<Box<TRules>>,
    pub oedoublingscope : Option<EDoublingScope>,
    pub ostossparams : Option<SStossParams>,
    pub ekurzlang : EKurzLang,
}

pub fn allowed_rules(vecrulegroup: &[SRuleGroup]) -> Vec<&TActivelyPlayableRules> {
    vecrulegroup.iter().flat_map(|rulegroup| rulegroup.vecrules.iter().map(|rules| rules.as_ref())).collect()
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
            println!("Did not find {}. Falling back to {}.", str_not_found, str_fallback);
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
                    ) as Box<TRules>)
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
                    let n_payout_extra = read_int(tomlval_game, ("extra")).or_else(|_err| fallback(&format!("{}.extra", $str_rule_name_file), "base-price"))?;
                    let n_payout_base = read_int(tomlval_game, ("price")).or_else(|_err| fallback(&format!("{}.price", $str_rule_name_file), $str_base_price_fallback))?;
                    let n_lauf_lbound = read_int(tomlval_game, ("lauf-min")).or_else(|_err| fallback(&format!("{}.lauf-min", $str_rule_name_file), "lauf-min"))?;
                    Ok(vecrulegroup.push(SRuleGroup{
                        str_name: $str_group_name.to_string(),
                        vecrules: ($fn_rules(SPayoutDeciderParams::new(
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
            create_rulegroup!(
                "rufspiel",
                "base-price",
                "Rufspiel", 
                |payoutparams: SPayoutDeciderParams| {
                    EFarbe::values()
                        .filter(|efarbe| EFarbe::Herz!=*efarbe)
                        .map(|efarbe| Box::new(SRulesRufspiel::new(
                            epi,
                            efarbe,
                            payoutparams.clone(),
                        )) as Box<TActivelyPlayableRules>)
                        .collect()
                }
            )?;
            macro_rules! read_sololike {($payoutdecider: ident, $fn_prio: expr, $str_rulename_suffix: expr) => {
                let internal_rulename = |str_rulename| {
                    format!("{}{}", str_rulename, $str_rulename_suffix)
                };
                macro_rules! vecrules_farbe {($trumpfdecider: ident, $i_prioindex: expr, $rulename: expr) => {
                    |payoutparams: SPayoutDeciderParams| {
                        macro_rules! internal_generate_sololike_farbe {($farbedesignator: ident) => {
                            sololike::<$trumpfdecider<STrumpfDeciderFarbe<$farbedesignator>>, $payoutdecider> (epi, $i_prioindex, &format!("{}-{}", $farbedesignator::FARBE, $rulename), payoutparams.clone())
                        }}
                        vec! [
                            internal_generate_sololike_farbe!(SFarbeDesignatorEichel),
                            internal_generate_sololike_farbe!(SFarbeDesignatorGras),
                            internal_generate_sololike_farbe!(SFarbeDesignatorHerz),
                            internal_generate_sololike_farbe!(SFarbeDesignatorSchelln),
                        ]
                    }
                }}
                macro_rules! vecrules_farblos {($trumpfdecider: ident, $i_prioindex: expr, $rulename: expr) => {
                    |payoutparams| vec![sololike::<$trumpfdecider<STrumpfDeciderNoTrumpf>, $payoutdecider>(epi, $i_prioindex, $rulename, payoutparams)]
                }}
                let str_rulename = internal_rulename("Solo");
                create_rulegroup_sololike!(
                    "solo",
                    &str_rulename,
                    vecrules_farbe!(SCoreSolo, $fn_prio(0), &str_rulename)
                )?;
                let str_rulename = internal_rulename("Wenz");
                create_rulegroup_sololike!(
                    "wenz",
                    &str_rulename,
                    vecrules_farblos!(SCoreGenericWenz, $fn_prio(-1), &str_rulename)
                )?;
                create_rulegroup_sololike!(
                    "farbwenz",
                    &internal_rulename("Farbwenz"),
                    vecrules_farbe!(SCoreGenericWenz, $fn_prio(-2), &internal_rulename("Wenz"))
                )?;
                let str_rulename = internal_rulename("Geier");
                create_rulegroup_sololike!(
                    "geier",
                    &str_rulename,
                    vecrules_farblos!(SCoreGenericGeier, $fn_prio(-3), &str_rulename)
                )?;
                create_rulegroup_sololike!(
                    "farbgeier",
                    &internal_rulename("Farbgeier"),
                    vecrules_farbe!(SCoreGenericGeier, $fn_prio(-4), &internal_rulename("Geier"))
                )?;
            }}
            if tomltbl.get("steigern").is_some() {
                read_sololike!(SPayoutDeciderPointBased, |_i_prio| VGameAnnouncementPriority::SoloLikeSteigern(61), "");
            } else {
                read_sololike!(SPayoutDeciderPointBased, VGameAnnouncementPriority::SoloLikeSimple, "");
            }
            read_sololike!(SPayoutDeciderTout, |x|x, " Tout");
            create_rulegroup_sololike!(
                "solo",
                "Sie",
                &|payoutparams| vec![sololike::<SCoreSolo<STrumpfDeciderNoTrumpf>, SPayoutDeciderSie>(epi, /*prioparams*/() ,"Sie", payoutparams)]
            )?;
            { // Bettel
                let str_rule_name_file = "bettel";
                if let Some(tomlval_bettel) = tomltbl.get(str_rule_name_file) {
                    let n_payout_base = read_int(tomlval_bettel, ("price"))
                        .or_else(|_err| 
                            fallback(&format!("{}.price", str_rule_name_file), /*str_base_price_fallback*/"base-price")
                        )?;
                    fn push_bettel<BettelAllAllowedCardsWithinStich>(vecrulegroup: &mut Vec<SRuleGroup>, epi: EPlayerIndex, n_payout_base: isize)
                        where BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich,
                    {
                        vecrulegroup.push(SRuleGroup{
                            str_name: "Bettel".to_string(),
                            vecrules: vec![Box::new(SRulesBettel::<BettelAllAllowedCardsWithinStich>::new(
                                epi,
                                /*i_prio, large negative number to make less important than any sololike*/-999_999,
                                n_payout_base.as_num::<isize>(),
                            )) as Box<TActivelyPlayableRules>],
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
                            println!("doubling.stock has invalid value '{}'. Falling back to 'no'", str_doubling_stock);
                        }
                        EDoublingScope::Games
                    }
                } else {
                    println!("doubling.stock not specified; falling back to 'stock=yes'");
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
                                println!("stoss.max less than 0. Defaulting to {}.", n_stoss_max_default);
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
                    println!("{} is not a valid value for 'deck' (supported values: kurz, lang). Defaulting to 'lang'", str_kurzlang);
                    EKurzLang::Lang
                },
            },
        ))
    }

    pub fn from_file(path: &Path) -> Result<SRuleSet, Error> {
        // TODO? ruleset creation wizard
        let mut file = File::open(&path)?;
        let mut str_toml = String::new();
        let _n_bytes = file.read_to_string(&mut str_toml)?;
        Self::from_string(&str_toml)
    }
}

