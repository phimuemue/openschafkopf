extern crate toml;

use primitives::*;
use rules::*;
use rules::rulesrufspiel::*;
use rules::rulessolo::*;
use rules::rulesramsch::*;
use rules::trumpfdecider::*;
use rules::payoutdecider::*;
use util::*;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use errors::*;

pub struct SRuleGroup {
    pub m_str_name : String,
    pub m_vecrules : Vec<Box<TActivelyPlayableRules>>,
}

pub enum VStockOrT<T> {
    Stock(/*n_stock*/isize), // number must be positive, but use isize since it is essentially a payment
    OrT(T),
}

pub enum EDoublingScope {
    Games,
    GamesAndStock,
}

#[derive(new)]
pub struct SRuleSet {
    pub m_avecrulegroup : EnumMap<EPlayerIndex, Vec<SRuleGroup>>,
    pub m_stockorramsch : VStockOrT<Box<TRules>>,
    pub m_oedoublingscope : Option<EDoublingScope>,
}

pub fn allowed_rules(vecrulegroup: &[SRuleGroup]) -> Vec<&TActivelyPlayableRules> {
    vecrulegroup.iter().flat_map(|rulegroup| rulegroup.m_vecrules.iter().map(|rules| rules.as_ref())).collect()
}

impl SRuleSet {
    pub fn from_string(str_toml: &str) -> Result<SRuleSet> {
        let tomltbl = toml::Parser::new(str_toml).parse().map(toml::Value::Table).ok_or("Parsing error.")?;
        let read_int = |str_key: &str| -> Result<i64> {
            if let Some(n) = tomltbl.lookup(str_key).and_then(|tomlval| tomlval.as_integer()) {
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
            read_int(str_fallback)
        };
        let stockorramsch = match (tomltbl.lookup("ramsch").is_some(), tomltbl.lookup("stock").is_some()) {
            (true, true) => {
                // TODO rules: Better alternatives? Allow stock *and* ramsch at the same time?
                bail!("Currently, having both Ramsch and Stock is not supported.")
            },
            (true, false) => {
                let durchmarsch = (match tomltbl.lookup("ramsch.durchmarsch") {
                    None => Ok(VDurchmarsch::None),
                    Some(&toml::Value::String(ref str_durchmarsch)) if "all"==str_durchmarsch => {
                        Ok(VDurchmarsch::All)
                    },
                    Some(&toml::Value::Integer(n_durchmarsch)) if 61<=n_durchmarsch && n_durchmarsch<=120 => {
                        Ok(VDurchmarsch::AtLeast(n_durchmarsch.as_num()))
                    },
                    _ => bail!("Invalid value for ramsch.durchmarsch. \"All\" or a number in [61; 120] is supported.")
                } as Result<_>)?;
                read_int("ramsch.price").map(|n_price|
                    VStockOrT::OrT(Box::new(SRulesRamsch{
                        m_n_price: n_price.as_num(),
                        m_durchmarsch: durchmarsch,
                    }) as Box<TRules>)
                )
            },
            (false, true) => {
                read_int("stock.price").or_else(|_err| fallback("stock.price", "base-price")).map(|n_price| VStockOrT::Stock(n_price.as_num()))
            },
            (false, false) => {
                Ok(VStockOrT::Stock(0)) // represent "no stock" by using a zero stock payment
            }
        }?;
        let mut avecrulegroup = EPlayerIndex::map_from_fn(|_epi| Vec::new());
        for epi in EPlayerIndex::values() {
            let vecrulegroup = &mut avecrulegroup[epi];
            let payoutparams_active = |str_game: &str, str_base_price_fallback: &str| -> Result<SPayoutDeciderParams> {
                let n_payout_extra = read_int(&format!("{}.extra", str_game)).or_else(|_err| fallback(&format!("{}.extra", str_game), "base-price"))?;
                let n_payout_base = read_int(&format!("{}.price", str_game)).or_else(|_err| fallback(&format!("{}.price", str_game), str_base_price_fallback))?;
                let n_lauf_lbound = read_int(&format!("{}.lauf-price", str_game)).or_else(|_err| fallback(&format!("{}.lauf-price", str_game), "lauf-min"))?;
                Ok(SPayoutDeciderParams::new(
                    n_payout_base.as_num(),
                    /*n_payout_schneider_schwarz*/n_payout_extra.as_num(),
                    SLaufendeParams::new(
                        /*n_payout_per_lauf*/n_payout_extra.as_num(),
                        n_lauf_lbound.as_num(),
                    ),
                ))
            };
            macro_rules! create_rulegroup {($str_rule_name_file: expr, $str_base_price_fallback: expr, $str_group_name: expr, $fn_rules: expr) => {
                if tomltbl.lookup($str_rule_name_file).is_some() {
                    let payoutparams = payoutparams_active($str_rule_name_file, $str_base_price_fallback)?;
                    Ok(vecrulegroup.push(SRuleGroup{
                        m_str_name: $str_group_name.to_string(),
                        m_vecrules: ($fn_rules(payoutparams.clone())),
                    })) as Result<_>
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
            macro_rules! read_sololike {
                ($payoutdecider: ident, $fn_prio: expr, $str_rulename_suffix: expr) => {
                    let internal_rulename = |str_rulename| {
                        format!("{}{}", str_rulename, $str_rulename_suffix)
                    };
                    macro_rules! generate_sololike_farbe {
                        ($trumpfdecider: ident, $i_prioindex: expr, $rulename: expr, $payoutparams: expr) => {{
                            macro_rules! internal_generate_sololike_farbe {
                                ($farbedesignator: ident) => {
                                    sololike::<$trumpfdecider<STrumpfDeciderFarbe<$farbedesignator>>, $payoutdecider> (epi, $i_prioindex, &format!("{}-{}", $farbedesignator::farbe(), $rulename), $payoutparams)
                                }
                            }
                            vec! [
                                internal_generate_sololike_farbe!(SFarbeDesignatorEichel),
                                internal_generate_sololike_farbe!(SFarbeDesignatorGras),
                                internal_generate_sololike_farbe!(SFarbeDesignatorHerz),
                                internal_generate_sololike_farbe!(SFarbeDesignatorSchelln),
                            ]
                        }}
                    }
                    let str_rulename = internal_rulename("Solo");
                    create_rulegroup_sololike!(
                        "solo",
                        &str_rulename,
                        |payoutparams: SPayoutDeciderParams| generate_sololike_farbe!(SCoreSolo, $fn_prio(0), &str_rulename, payoutparams.clone())
                    )?;
                    let str_rulename = internal_rulename("Wenz");
                    create_rulegroup_sololike!(
                        "wenz",
                        &str_rulename,
                        |payoutparams| vec![sololike::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, $payoutdecider>(epi, $fn_prio(-1),&str_rulename, payoutparams)]
                    )?;
                    create_rulegroup_sololike!(
                        "farbwenz",
                        &internal_rulename("Farbwenz"),
                        |payoutparams: SPayoutDeciderParams| generate_sololike_farbe!(SCoreGenericWenz, $fn_prio(-2), &internal_rulename("Wenz"), payoutparams.clone())
                    )?;
                    let str_rulename = internal_rulename("Geier");
                    create_rulegroup_sololike!(
                        "geier",
                        &str_rulename,
                        |payoutparams| vec![sololike::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, $payoutdecider>(epi, $fn_prio(-3),&str_rulename, payoutparams)]
                    )?;
                    create_rulegroup_sololike!(
                        "farbgeier",
                        &internal_rulename("Farbgeier"),
                        |payoutparams: SPayoutDeciderParams| generate_sololike_farbe!(SCoreGenericGeier, $fn_prio(-4), &internal_rulename("Geier"), payoutparams.clone())
                    )?;
                }
            }
            read_sololike!(SPayoutDeciderPointBased, VGameAnnouncementPriority::SoloLikeSimple, "");
            read_sololike!(SPayoutDeciderTout, VGameAnnouncementPriority::SoloTout, " Tout");
            create_rulegroup_sololike!(
                "solo",
                "Sie",
                &|payoutparams| vec![sololike::<SCoreSolo<STrumpfDeciderNoTrumpf>, SPayoutDeciderSie>(epi, VGameAnnouncementPriority::SoloSie ,&"Sie", payoutparams)]
            )?;
        }
        Ok(SRuleSet::new(
            avecrulegroup,
            stockorramsch,
            tomltbl.lookup("doubling").map(|tomlval_doubling | {
                if let Some(str_doubling_stock)=tomlval_doubling.lookup("stock").and_then(|tomlval| tomlval.as_str()) {
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
            })
        ))
    }

    pub fn from_file(path: &Path) -> Result<SRuleSet> {
        // TODO? ruleset creation wizard
        let mut file = File::open(&path)?;
        let mut str_toml = String::new();
        let _n_bytes = file.read_to_string(&mut str_toml)?;
        Self::from_string(&str_toml)
    }
}

