use card::*;
use stich::*;
use rules::*;
use rulesrufspiel::*;
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
    SRuleSet {
        m_vecrules : vec![
            // TODO: can I somehow collect this?
            Rc::new(CRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: efarbeEICHEL}),
            Rc::new(CRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: efarbeGRAS}),
            Rc::new(CRulesRufspiel{m_eplayerindex: eplayerindex, m_efarbe: efarbeSCHELLN}),
        ]
    }
}

