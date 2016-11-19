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

fn read_sololike<PayoutDecider>(str_l: &str, eplayerindex: EPlayerIndex, i_prioindex_offset: isize, str_rulename_suffix: &str) -> Option<SRuleGroup>
    where PayoutDecider: TPayoutDecider,
          PayoutDecider: Sync,
          PayoutDecider: 'static,
{
    let internal_rulename = |str_rulename| {
        format!("{}{}", str_rulename, str_rulename_suffix)
    };
    let priority = |i_prioindex| {
        // TODO introduce different kind of priority for Tout
        i_prioindex + i_prioindex_offset
    };
    macro_rules! generate_sololike_farbe {
        ($eplayerindex: ident, $trumpfdecider: ident, $i_prioindex: expr, $rulename: expr) => {
            vec! [
                sololike::<$trumpfdecider<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, PayoutDecider> ($eplayerindex, $i_prioindex, &format!("Eichel-{}", $rulename)),
                sololike::<$trumpfdecider<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, PayoutDecider>   ($eplayerindex, $i_prioindex, &format!("Gras-{}", $rulename)),
                sololike::<$trumpfdecider<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, PayoutDecider>   ($eplayerindex, $i_prioindex, &format!("Herz-{}", $rulename)),
                sololike::<$trumpfdecider<STrumpfDeciderFarbe<SFarbeDesignatorSchelln>>, PayoutDecider>($eplayerindex, $i_prioindex, &format!("Schelln-{}", $rulename)),
            ]
        }
    }
    if str_l=="solo" {
	// TODO Sie
        let str_rulename = internal_rulename("Solo");
        create_rulegroup(&str_rulename, generate_sololike_farbe!(eplayerindex, SCoreSolo, /*i_prioindex*/priority(0), &str_rulename))
    } else if str_l=="farbwenz" {
        create_rulegroup(&internal_rulename("Farbwenz"), generate_sololike_farbe!(eplayerindex, SCoreGenericWenz, /*i_prioindex*/priority(2), &internal_rulename("Wenz")))
    } else if str_l=="wenz" {
        let str_rulename = internal_rulename("Wenz");
        create_rulegroup(&str_rulename, vec![sololike::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, PayoutDecider>(eplayerindex, /*i_prioindex*/priority(1),&str_rulename)])
    } else if str_l=="farbgeier" {
        create_rulegroup(&internal_rulename("Farbgeier"), generate_sololike_farbe!(eplayerindex, SCoreGenericGeier, /*i_prioindex*/priority(4), &internal_rulename("Geier")))
    } else if str_l=="geier" {
        let str_rulename = internal_rulename("Geier");
        create_rulegroup(&str_rulename, vec![sololike::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, PayoutDecider>(eplayerindex, /*i_prioindex*/priority(3),&str_rulename)])
    } else {
        None
    }
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
            let mut vecrulegroup = Vec::new();
            for rulegroup in vecstr_rule_name.iter()
                .filter_map(|str_l| {
                    if str_l=="rufspiel" {
                        create_rulegroup(
                            "Rufspiel", 
                            EFarbe::values()
                                .filter(|efarbe| EFarbe::Herz!=*efarbe)
                                .map(|efarbe| Box::new(SRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: efarbe}) as Box<TActivelyPlayableRules>)
                                .collect()
                        )
                    } else {
                        None
                    }
                })
            {
                vecrulegroup.push(rulegroup);
            }
            for rulegroup in vecstr_rule_name.iter().filter_map(|str_l| read_sololike::<SPayoutDeciderPointBased>(str_l, eplayerindex, /*i_prioindex_offset*/100, "")) {
                vecrulegroup.push(rulegroup);
            }
            for rulegroup in vecstr_rule_name.iter().filter_map(|str_l| read_sololike::<SPayoutDeciderTout>(str_l, eplayerindex, /*i_prioindex_offset*/0, " Tout")) {
                vecrulegroup.push(rulegroup);
            }
            vecrulegroup
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

