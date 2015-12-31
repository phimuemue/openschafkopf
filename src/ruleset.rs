use card::*;
use stich::*;
use rules::*;
use rulesrufspiel::*;
use rulessolo::*;
use std::rc::Rc;

pub struct SRuleSet {
    m_vecrules : Vec<Rc<TRules>>,
}

impl SRuleSet {
    pub fn allowed_rules(&self) -> &Vec<Rc<TRules>> {
        &self.m_vecrules
    }
}

pub fn ruleset_default(eplayerindex: EPlayerIndex) -> SRuleSet {
    let mut vecrules = Vec::<Rc<TRules>>::new();
    for efarbe in EFarbe::all_values().iter().filter(|&efarbe| efarbeHERZ!=*efarbe) {
        vecrules.push(Rc::new(CRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: *efarbe}));
    }
    for efarbe in EFarbe::all_values().iter() {
        vecrules.push(Rc::new(CRulesSolo{m_eplayerindex: eplayerindex, m_efarbe: *efarbe}));
    }
    SRuleSet { m_vecrules : vecrules }
}

