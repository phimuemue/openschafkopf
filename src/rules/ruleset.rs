extern crate toml;

use primitives::*;
use rules::*;
use rules::rulesrufspiel::*;
use rules::rulessolo::*;
use rules::rulesramsch::*;
use rules::trumpfdecider::*;
use rules::payoutdecider::*;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub struct SRuleGroup {
    pub m_str_name : String,
    pub m_vecrules : Vec<Box<TActivelyPlayableRules>>,
}

pub enum VStockOrT<T> {
    Stock(/*n_stock*/isize), // number must be positive, but use isize since it is essentially a payment
    OrT(T),
}

pub struct SRuleSet {
    pub m_avecrulegroup : SPlayerIndexMap<Vec<SRuleGroup>>,
    pub m_stockorramsch : VStockOrT<Box<TRules>>,
}

pub fn allowed_rules(vecrulegroup: &Vec<SRuleGroup>) -> Vec<&TActivelyPlayableRules> {
    vecrulegroup.iter().flat_map(|rulegroup| rulegroup.m_vecrules.iter().map(|rules| rules.as_ref())).collect()
}

impl SRuleSet {
    pub fn from_string(str_toml: &str) -> SRuleSet {
        let tomltbl : toml::Value = str_toml.parse().unwrap(); // TODO error handling
        SRuleSet {
            m_avecrulegroup : create_playerindexmap(|eplayerindex| {
                let mut vecrulegroup = Vec::new();
                {
                    let mut create_rulegroup = |str_rule_name_file: &str, str_group_name: &str, vecrules| {
                        if let Some(tomlval_active_rules) = tomltbl.lookup("activerules") {
                            if tomlval_active_rules.lookup(str_rule_name_file).is_some() {
                                vecrulegroup.push(SRuleGroup{
                                    m_str_name: str_group_name.to_string(),
                                    m_vecrules: vecrules
                                });
                            }
                        }
                    };
                    create_rulegroup(
                        "rufspiel",
                        "Rufspiel", 
                        EFarbe::values()
                            .filter(|efarbe| EFarbe::Herz!=*efarbe)
                            .map(|efarbe| Box::new(SRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: efarbe}) as Box<TActivelyPlayableRules>)
                            .collect()
                    );
                    macro_rules! read_sololike {
                        ($payoutdecider: ident, $fn_prio: expr, $str_rulename_suffix: expr) => {
                            let internal_rulename = |str_rulename| {
                                format!("{}{}", str_rulename, $str_rulename_suffix)
                            };
                            macro_rules! generate_sololike_farbe {
                                ($eplayerindex: ident, $trumpfdecider: ident, $i_prioindex: expr, $rulename: expr) => {
                                    vec! [
                                        sololike::<$trumpfdecider<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, $payoutdecider> ($eplayerindex, $i_prioindex, &format!("Eichel-{}", $rulename)),
                                        sololike::<$trumpfdecider<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, $payoutdecider>   ($eplayerindex, $i_prioindex, &format!("Gras-{}", $rulename)),
                                        sololike::<$trumpfdecider<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, $payoutdecider>   ($eplayerindex, $i_prioindex, &format!("Herz-{}", $rulename)),
                                        sololike::<$trumpfdecider<STrumpfDeciderFarbe<SFarbeDesignatorSchelln>>, $payoutdecider>($eplayerindex, $i_prioindex, &format!("Schelln-{}", $rulename)),
                                    ]
                                }
                            }
                            let str_rulename = internal_rulename("Solo");
                            create_rulegroup("solo", &str_rulename, generate_sololike_farbe!(eplayerindex, SCoreSolo, $fn_prio(0), &str_rulename));
                            let str_rulename = internal_rulename("Wenz");
                            create_rulegroup("wenz", &str_rulename, vec![sololike::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, $payoutdecider>(eplayerindex, $fn_prio(-1),&str_rulename)]);
                            create_rulegroup("farbwenz", &internal_rulename("Farbwenz"), generate_sololike_farbe!(eplayerindex, SCoreGenericWenz, $fn_prio(-2), &internal_rulename("Wenz")));
                            let str_rulename = internal_rulename("Geier");
                            create_rulegroup("geier", &str_rulename, vec![sololike::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, $payoutdecider>(eplayerindex, $fn_prio(-3),&str_rulename)]);
                            create_rulegroup("farbgeier", &internal_rulename("Farbgeier"), generate_sololike_farbe!(eplayerindex, SCoreGenericGeier, $fn_prio(-4), &internal_rulename("Geier")));
                        }
                    }
                    read_sololike!(SPayoutDeciderPointBased, |i_prioindex| VGameAnnouncementPriority::SoloLikeSimple(i_prioindex), "");
                    read_sololike!(SPayoutDeciderTout, |i_prioindex| VGameAnnouncementPriority::SoloTout(i_prioindex), " Tout");
                    create_rulegroup("solo", "Sie", vec![sololike::<SCoreSolo<STrumpfDeciderNoTrumpf>, SPayoutDeciderSie>(eplayerindex, VGameAnnouncementPriority::SoloSie ,&"Sie")]);
                }
                vecrulegroup
            }),
            m_stockorramsch : {
                if tomltbl.lookup("noactive.ramsch").is_some() {
                    assert!(tomltbl.lookup("noactive.stock").is_none()); // TODO what to do in those cases? Better option to model alternatives? Allow stock *and+ ramsch at the same time?
                    VStockOrT::OrT(Box::new(SRulesRamsch{}) as Box<TRules>)
                } else if tomltbl.lookup("noactive.stock").is_some() {
                    VStockOrT::Stock(10) // TODO make adjustable
                } else {
                    VStockOrT::Stock(0) // represent "no stock" by using a zero stock payment
                }
            }
        }
    }

    pub fn from_file(path: &Path) -> SRuleSet {
        if !path.exists() {
            println!("File {} not found. Creating it.", path.display());
            let mut file = match File::create(&path) {
                Err(why) => panic!("Could not create {}: {}", path.display(), Error::description(&why)),
                Ok(file) => file,
            };
            // TODO: make creation of ruleset file adjustable
            for str_rules in &[
                "[activerules.rufspiel]",
                "[activerules.solo]",
                "[activerules.farbwenz]",
                "[activerules.wenz]",
                "[activerules.farbgeier]",
                "[activerules.geier]",
            ] {
                file.write_all(str_rules.as_bytes()).unwrap();
            }
        }
        assert!(path.exists()); 
        let mut file = match File::open(&path) {
            Err(why) => panic!("Could not open {}: {}", path.display(), Error::description(&why)),
            Ok(file) => file,
        };
        let mut str_toml = String::new();
        file.read_to_string(&mut str_toml).unwrap(); // TODO error handling
        Self::from_string(&str_toml)
    }
}

