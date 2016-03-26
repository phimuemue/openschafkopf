use card::*;
use stich::*;
use rules::*;
use rulesrufspiel::*;
use rulessolo::*;

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
    for efarbe in EFarbe::all_values().iter().filter(|&efarbe| EFarbe::Herz!=*efarbe) {
        vecrules.push(Box::new(CRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: *efarbe}));
    }
    for efarbe in EFarbe::all_values().iter() {
        vecrules.push(Box::new(CRulesSolo{m_eplayerindex: eplayerindex, m_efarbe: *efarbe}));
    }
    SRuleSet { m_vecrules : vecrules }
}

