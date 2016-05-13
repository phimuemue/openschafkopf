use card::*;
use stich::*;
use rules::*;
use rules::rulesrufspiel::*;
use rules::rulessolo::*;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::io::BufReader;

pub struct SRuleGroup {
    pub m_str_name : String,
    pub m_vecrules : Vec<Box<TRules>>,
}

pub struct SRuleSet {
    pub m_vecrulegroup : Vec<SRuleGroup>,
}

impl SRuleSet {
    pub fn allowed_rules(&self) -> Vec<&TRules> {
        self.m_vecrulegroup.iter().flat_map(|rulegroup| rulegroup.m_vecrules.iter().map(|rules| rules.as_ref())).collect()
    }
}

pub fn create_rulegroup<ItRules> (str_name: &str, itrules: ItRules) -> Option<SRuleGroup> 
    where ItRules: Iterator<Item=Box<TRules>>,
{
    Some(SRuleGroup{
        m_str_name: str_name.to_string(),
        m_vecrules: itrules.collect()
    })
}

pub fn read_ruleset(path: &Path) -> [SRuleSet; 4] {
    if !path.exists() {
        println!("File {} not found. Creating it.", path.display());
        let mut file = match File::create(&path) {
            Err(why) => panic!("Could not create {}: {}", path.display(), Error::description(&why)),
            Ok(file) => file,
        };
        // TODO: make creation of ruleset file adjustable
        file.write_all(b"rufspiel\n").unwrap();
        file.write_all(b"solo\n").unwrap();
    }
    create_playerindexmap(|eplayerindex| {
        assert!(path.exists()); 
        let file = match File::open(&path) {
            Err(why) => panic!("Could not open {}: {}", path.display(), Error::description(&why)),
            Ok(file) => file,
        };
        SRuleSet {m_vecrulegroup : BufReader::new(&file).lines()
            .map(|str| str.unwrap())
            .filter_map(|str_l| {
                println!("allowing rule: {}", str_l);
                if str_l=="rufspiel" {
                    create_rulegroup(
                        "Rufspiel", 
                        EFarbe::all_values().iter()
                            .filter(|&efarbe| EFarbe::Herz!=*efarbe)
                            .map(|&efarbe| Box::new(SRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: efarbe}) as Box<TRules>)
                    )
                } else if str_l=="solo" {
                    create_rulegroup(
                        "Solo",
                        EFarbe::all_values().iter()
                            .map(|&efarbe| Box::new(SRulesSolo{m_eplayerindex: eplayerindex, m_efarbe: efarbe}) as Box<TRules>)
                    )
                } else {
                    println!("{} is not a valid rule descriptor", str_l);
                    None
                }
            })
            .collect()
        }
    })
}

