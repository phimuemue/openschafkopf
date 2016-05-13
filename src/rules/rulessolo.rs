use card::*;
use stich::*;
use hand::*;
use rules::*;
use std::fmt;
use std::cmp::Ordering;
use std::marker::PhantomData;

pub trait TTrumpfDecider {
    fn is_trumpf(card: SCard) -> bool;
    fn trumpfs_in_descending_order(mut veceschlag: Vec<ESchlag>, mut vecefarbe: Vec<EFarbe>) -> Vec<SCard>;
    fn compare_trumpfcards_solo(card_fst: SCard, card_snd: SCard) -> Ordering;
}

pub struct STrumpfDeciderNoTrumpf {}
impl TTrumpfDecider for STrumpfDeciderNoTrumpf {
    fn is_trumpf(_card: SCard) -> bool {
        false
    }
    fn trumpfs_in_descending_order(mut _veceschlag: Vec<ESchlag>, mut _vecefarbe: Vec<EFarbe>) -> Vec<SCard> {
        Vec::new()
    }
    fn compare_trumpfcards_solo(_card_fst: SCard, _card_snd: SCard) -> Ordering {
        panic!("STrumpfDeciderNoTrumpf::compare_trumpfcards_solo called")
    }
}

pub trait TSchlagDesignator {fn schlag() -> ESchlag;}
pub struct SSchlagDesignatorOber {}
pub struct SSchlagDesignatorUnter {}
impl TSchlagDesignator for SSchlagDesignatorOber { fn schlag() -> ESchlag {ESchlag::Ober} }
impl TSchlagDesignator for SSchlagDesignatorUnter { fn schlag() -> ESchlag {ESchlag::Unter} }

pub struct STrumpfDeciderSchlag<SchlagDesignator, DeciderSec> {
    m_schlagdesignator: PhantomData<SchlagDesignator>,
    m_decidersec: PhantomData<DeciderSec>,
}
impl<SchlagDesignator, DeciderSec> TTrumpfDecider for STrumpfDeciderSchlag<SchlagDesignator, DeciderSec> 
    where DeciderSec: TTrumpfDecider,
          SchlagDesignator: TSchlagDesignator,
{
    fn is_trumpf(card: SCard) -> bool {
        SchlagDesignator::schlag() == card.schlag() || DeciderSec::is_trumpf(card)
    }
    fn trumpfs_in_descending_order(mut veceschlag: Vec<ESchlag>, vecefarbe: Vec<EFarbe>) -> Vec<SCard> {
        let mut veccard_trumpf : Vec<_> = EFarbe::all_values().iter()
            .filter(|efarbe| !vecefarbe.iter().any(|&efarbe_done| efarbe_done==**efarbe))
            .map(|&efarbe| SCard::new(efarbe, SchlagDesignator::schlag()))
            .collect();
        veceschlag.push(SchlagDesignator::schlag());
        let mut veccard_trumpf_sec = DeciderSec::trumpfs_in_descending_order(veceschlag, vecefarbe);
        veccard_trumpf.append(&mut veccard_trumpf_sec);
        veccard_trumpf
    }
    fn compare_trumpfcards_solo(card_fst: SCard, card_snd: SCard) -> Ordering {
        match (SchlagDesignator::schlag()==card_fst.schlag(), SchlagDesignator::schlag()==card_snd.schlag()) {
            (true, true) => {
                // TODO static_assert not available in rust, right?
                assert!(EFarbe::Eichel < EFarbe::Gras, "Farb-Sorting can't be used here");
                assert!(EFarbe::Gras < EFarbe::Herz, "Farb-Sorting can't be used here");
                assert!(EFarbe::Herz < EFarbe::Schelln, "Farb-Sorting can't be used here");
                if card_snd.farbe() < card_fst.farbe() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
            },
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => DeciderSec::compare_trumpfcards_solo(card_fst, card_snd),
        }
    }
}

pub trait TFarbeDesignator {fn farbe() -> EFarbe;}
pub struct SFarbeDesignatorEichel {}
impl TFarbeDesignator for SFarbeDesignatorEichel { fn farbe() -> EFarbe {EFarbe::Eichel} }
pub struct SFarbeDesignatorGras {}
impl TFarbeDesignator for SFarbeDesignatorGras { fn farbe() -> EFarbe {EFarbe::Gras} }
pub struct SFarbeDesignatorHerz {}
impl TFarbeDesignator for SFarbeDesignatorHerz { fn farbe() -> EFarbe {EFarbe::Herz} }
pub struct SFarbeDesignatorSchelln {}
impl TFarbeDesignator for SFarbeDesignatorSchelln { fn farbe() -> EFarbe {EFarbe::Schelln} }

pub struct STrumpfDeciderFarbe<FarbeDesignator, DeciderSec> {
    m_farbedesignator: PhantomData<FarbeDesignator>,
    m_decidersec: PhantomData<DeciderSec>,
}
impl<FarbeDesignator, DeciderSec> TTrumpfDecider for STrumpfDeciderFarbe<FarbeDesignator, DeciderSec> 
    where DeciderSec: TTrumpfDecider,
          FarbeDesignator: TFarbeDesignator,
{
    fn is_trumpf(card: SCard) -> bool {
        FarbeDesignator::farbe() == card.farbe() || DeciderSec::is_trumpf(card)
    }
    fn trumpfs_in_descending_order(veceschlag: Vec<ESchlag>, mut vecefarbe: Vec<EFarbe>) -> Vec<SCard> {
        let mut veccard_trumpf : Vec<_> = ESchlag::all_values().iter()
            .filter(|eschlag| !veceschlag.iter().any(|&eschlag_done| eschlag_done==**eschlag))
            .map(|&eschlag| SCard::new(FarbeDesignator::farbe(), eschlag))
            .collect();
        vecefarbe.push(FarbeDesignator::farbe());
        let mut veccard_trumpf_sec = DeciderSec::trumpfs_in_descending_order(veceschlag, vecefarbe);
        veccard_trumpf.append(&mut veccard_trumpf_sec);
        veccard_trumpf
    }
    fn compare_trumpfcards_solo(card_fst: SCard, card_snd: SCard) -> Ordering {
        match (FarbeDesignator::farbe()==card_fst.farbe(), FarbeDesignator::farbe()==card_snd.farbe()) {
            (true, true) => {compare_farbcards_same_color(card_fst, card_snd)},
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => DeciderSec::compare_trumpfcards_solo(card_fst, card_snd),
        }
    }
}

pub struct SRulesActiveSinglePlay<ActiveSinglePlayCore> 
    where ActiveSinglePlayCore: TTrumpfDecider,
{
    pub m_str_name: String,
    pub m_eplayerindex : EPlayerIndex, // TODO should be static
    pub m_core : PhantomData<ActiveSinglePlayCore>,
}

impl<ActiveSinglePlayCore> fmt::Display for SRulesActiveSinglePlay<ActiveSinglePlayCore> 
    where ActiveSinglePlayCore: TTrumpfDecider,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.m_str_name)
    }
}

impl<ActiveSinglePlayCore> TRules for SRulesActiveSinglePlay<ActiveSinglePlayCore> 
    where ActiveSinglePlayCore: TTrumpfDecider,
{
    fn trumpf_or_farbe(&self, card: SCard) -> VTrumpfOrFarbe {
        if ActiveSinglePlayCore::is_trumpf(card) {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_eplayerindex)
    }

    fn is_winner(&self, eplayerindex: EPlayerIndex, vecstich: &Vec<SStich>) -> bool {
        assert!(8==vecstich.len());
        if eplayerindex==self.m_eplayerindex {
            self.points_per_player(vecstich)[self.m_eplayerindex]>=61
        } else {
            self.points_per_player(vecstich)[self.m_eplayerindex]<=60
        }
    }

    fn payout(&self, vecstich: &Vec<SStich>) -> [isize; 4] {
        let ab_winner = create_playerindexmap(|eplayerindex| self.is_winner(eplayerindex, vecstich));
        let n_laufende = count_laufende_from_veccard_trumpf(
            vecstich,
            &ActiveSinglePlayCore::trumpfs_in_descending_order(Vec::new(), Vec::new()),
            &ab_winner
        );
        create_playerindexmap(|eplayerindex| {
            (/*n_payout_solo*/ 50
             + {if n_laufende<3 {0} else {n_laufende}} * 10
            ) * {
                if ab_winner[eplayerindex] {
                    1
                } else {
                    -1
                }
            } * {
                if self.m_eplayerindex==eplayerindex {
                    3
                } else {
                    1
                }
            }
        } )
    }

    fn all_allowed_cards_first_in_stich(&self, _vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        let card_first = vecstich.last().unwrap().first_card();
        let veccard_allowed : SHandVector = hand.cards().iter()
            .filter(|&&card| self.trumpf_or_farbe(card)==self.trumpf_or_farbe(card_first))
            .cloned()
            .collect();
        if veccard_allowed.is_empty() {
            hand.cards().clone()
        } else {
            veccard_allowed
        }
    }

    fn compare_in_stich_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        ActiveSinglePlayCore::compare_trumpfcards_solo(card_fst, card_snd)
    }
}

macro_rules! generate_sololike {
    ($eplayerindex: ident, $coretype: ty, $rulename: expr) => {
        Box::new(SRulesActiveSinglePlay::<$coretype> {
            m_eplayerindex: $eplayerindex,
            m_core: PhantomData::<$coretype>,
            m_str_name: $rulename.to_string(),
        }) as Box<TRules>
    }
}

macro_rules! generate_sololike_farbe {
    ($eplayerindex: ident, $coretype: ident, $rulename: expr) => {
        vec! [
            generate_sololike!($eplayerindex, $coretype<SFarbeDesignatorEichel>, format!("Eichel-{}", $rulename)),
            generate_sololike!($eplayerindex, $coretype<SFarbeDesignatorGras>, format!("Gras-{}", $rulename)),
            generate_sololike!($eplayerindex, $coretype<SFarbeDesignatorHerz>, format!("Herz-{}", $rulename)),
            generate_sololike!($eplayerindex, $coretype<SFarbeDesignatorSchelln>, format!("Schelln-{}", $rulename)),
        ]
    }
}

pub type SCoreSolo<FarbeDesignator> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, STrumpfDeciderFarbe<
    FarbeDesignator, STrumpfDeciderNoTrumpf>>>;

pub fn all_rulessolo(eplayerindex: EPlayerIndex) -> Vec<Box<TRules>> {
    generate_sololike_farbe!(eplayerindex, SCoreSolo, "Solo")
}

pub type SCoreFarbwenz<FarbeDesignator> = STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, STrumpfDeciderFarbe<
    FarbeDesignator, STrumpfDeciderNoTrumpf>>;

pub fn all_rulesfarbwenz(eplayerindex: EPlayerIndex) -> Vec<Box<TRules>> {
    generate_sololike_farbe!(eplayerindex, SCoreFarbwenz, "Wenz")
}

pub fn all_ruleswenz(eplayerindex: EPlayerIndex) -> Vec<Box<TRules>> {
    vec![generate_sololike!(eplayerindex, STrumpfDeciderSchlag<SSchlagDesignatorUnter,STrumpfDeciderNoTrumpf>, "Wenz")]
}
