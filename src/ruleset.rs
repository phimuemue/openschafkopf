use card::*;
use stich::*;
use rules::*;
use rulesrufspiel::*;
use rulessolo::*;

use std::error::Error;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::io::BufReader;

pub struct SRuleSet {
    m_vecrules : Vec<Box<TRules>>,
}

impl SRuleSet {
    pub fn allowed_rules(&self) -> &Vec<Box<TRules>> {
        &self.m_vecrules
    }
}

pub fn ruleset_default(eplayerindex: EPlayerIndex) -> SRuleSet {
    let mut vecrules = Vec::<Box<TRules>>::new();
    let path = Path::new(".schafkopfruleset");
    if path.exists() {
        let file = match File::open(&path) {
            Err(why) => panic!("Could not open {}: {}", path.display(), Error::description(&why)),
            Ok(file) => file,
        };
        for str_line in BufReader::new(&file).lines() {
            let str_l : String = str_line.unwrap();
            println!("allowing rule: {}", str_l);
            if str_l=="rufspiel" {
                for efarbe in EFarbe::all_values().iter().filter(|&efarbe| EFarbe::Herz!=*efarbe) {
                    vecrules.push(Box::new(CRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: *efarbe}));
                }
            } else if str_l=="solo" {
                for efarbe in EFarbe::all_values().iter() {
                    vecrules.push(Box::new(CRulesSolo{m_eplayerindex: eplayerindex, m_efarbe: *efarbe}));
                }
            } else {
                println!("{} is not a valid rule descriptor", str_l);
            }
        }
    } else {
        println!("File not found. Creating it.");
        unimplemented!();
    }
    SRuleSet { m_vecrules : vecrules }
}

