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
use std::io::BufReader;
use std::collections::HashSet;

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

pub fn read_ruleset(path: &Path) -> SRuleSet {
    if !path.exists() {
        println!("File {} not found. Creating it.", path.display());
        let mut file = match File::create(&path) {
            Err(why) => panic!("Could not create {}: {}", path.display(), Error::description(&why)),
            Ok(file) => file,
        };
        // TODO: make creation of ruleset file adjustable
        for str_rules in [
            "rufspiel",
            "solo",
            "farbwenz",
            "wenz",
            "farbgeier",
            "geier",
        ].iter() {
            file.write_all(str_rules.as_bytes()).unwrap();
        }
    }
    let setstr_rule_name = {
        assert!(path.exists()); 
        let file = match File::open(&path) {
            Err(why) => panic!("Could not open {}: {}", path.display(), Error::description(&why)),
            Ok(file) => file,
        };
        BufReader::new(&file).lines().map(|str| str.unwrap()).collect::<HashSet<_>>()
    };
    SRuleSet {
        m_avecrulegroup : create_playerindexmap(|eplayerindex| {
            let mut vecrulegroup = Vec::new();
            {
                let mut create_rulegroup = |str_rule_name_file: &str, str_group_name: &str, vecrules| {
                    if setstr_rule_name.contains(&str_rule_name_file.to_string()) {
                        vecrulegroup.push(SRuleGroup{
                            m_str_name: str_group_name.to_string(),
                            m_vecrules: vecrules
                        });
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
            if setstr_rule_name.contains("ramsch") {
                assert!(!setstr_rule_name.contains("stock"));
                VStockOrT::OrT(Box::new(SRulesRamsch{}) as Box<TRules>)
            } else if setstr_rule_name.contains("stock") {
                VStockOrT::Stock(10) // TODO make adjustable
            } else {
                VStockOrT::Stock(0) // represent "no stock" by using a zero stock payment
            }
        }
    }
}

