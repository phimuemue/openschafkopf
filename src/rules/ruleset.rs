use primitives::*;
use rules::*;
use rules::rulesrufspiel::*;
use rules::rulessolo::*;
use rules::rulesramsch::*;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::io::BufReader;

pub struct SRuleGroup {
    pub m_str_name : String,
    pub m_vecrules : Vec<Box<TActivelyPlayableRules>>,
}

pub struct SRuleSet {
    pub m_avecrulegroup : [Vec<SRuleGroup>; 4],
    pub m_orulesramsch : Option<Box<TRules>>,
}

pub fn allowed_rules(vecrulegroup: &Vec<SRuleGroup>) -> Vec<&TActivelyPlayableRules> {
    vecrulegroup.iter().flat_map(|rulegroup| rulegroup.m_vecrules.iter().map(|rules| rules.as_ref())).collect()
}

pub fn create_rulegroup (str_name: &str, vecrules: Vec<Box<TActivelyPlayableRules>>) -> Option<SRuleGroup> {
    Some(SRuleGroup{
        m_str_name: str_name.to_string(),
        m_vecrules: vecrules
    })
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
    let vecstr_rule_name = {
        assert!(path.exists()); 
        let file = match File::open(&path) {
            Err(why) => panic!("Could not open {}: {}", path.display(), Error::description(&why)),
            Ok(file) => file,
        };
        BufReader::new(&file).lines().map(|str| str.unwrap()).collect::<Vec<_>>()
    };
    SRuleSet {
        m_avecrulegroup : create_playerindexmap(|eplayerindex| {
            vecstr_rule_name.iter()
                .filter_map(|str_l| {
                    println!("allowing {} for {}", str_l, eplayerindex);
                    if str_l=="rufspiel" {
                        create_rulegroup(
                            "Rufspiel", 
                            EFarbe::values()
                                .filter(|efarbe| EFarbe::Herz!=*efarbe)
                                .map(|efarbe| Box::new(SRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: efarbe}) as Box<TActivelyPlayableRules>)
                                .collect()
                        )
                    } else if str_l=="solo" {
                        create_rulegroup("Solo", all_rulessolo(eplayerindex, /*i_prioindex*/0, "Solo"))
                    } else if str_l=="farbwenz" {
                        create_rulegroup("Farbwenz", all_rulesfarbwenz(eplayerindex, /*i_prioindex*/2, "Wenz"))
                    } else if str_l=="wenz" {
                        create_rulegroup("Wenz", all_ruleswenz(eplayerindex, /*i_prioindex*/1, "Wenz"))
                    } else if str_l=="farbgeier" {
                        create_rulegroup("Farbgeier", all_rulesfarbgeier(eplayerindex, /*i_prioindex*/4, "Geier"))
                    } else if str_l=="geier" {
                        create_rulegroup("Geier", all_rulesgeier(eplayerindex, /*i_prioindex*/3, "Geier"))
                    } else {
                        None
                    }
                })
                .collect()
        }),
        m_orulesramsch : { 
            if vecstr_rule_name.contains(&"ramsch".to_string()) {
                Some(Box::new(SRulesRamsch{}))
            } else {
                None
            }
        },
    }
}

