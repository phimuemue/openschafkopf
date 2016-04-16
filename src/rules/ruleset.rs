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

pub struct SRuleSet {
    m_vecrules : Vec<Box<TRules>>,
}

impl SRuleSet {
    pub fn allowed_rules(&self) -> &Vec<Box<TRules>> {
        &self.m_vecrules
    }
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
        let mut vecrules = Vec::<Box<TRules>>::new();
        assert!(path.exists()); 
        let file = match File::open(&path) {
            Err(why) => panic!("Could not open {}: {}", path.display(), Error::description(&why)),
            Ok(file) => file,
        };
        for str_l in BufReader::new(&file).lines().map(|str| str.unwrap()) {
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
        SRuleSet { m_vecrules : vecrules }
    })
}

