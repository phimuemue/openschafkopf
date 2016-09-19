use primitives::*;
use rules::*;
use std::cmp::Ordering;
use std::marker::PhantomData;

pub trait TTrumpfDecider {
    fn trumpf_or_farbe(card: SCard) -> VTrumpfOrFarbe {
        if Self::is_trumpf(card) {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }

    fn better_trumpf(card_fst: SCard, card_snd: SCard) -> SCard {
        assert!(Self::is_trumpf(card_fst));
        assert!(Self::is_trumpf(card_snd));
        if Ordering::Less==Self::compare_trumpfcards_solo(card_fst, card_snd) {
            card_snd
        } else {
            card_fst
        }
    }

    fn is_trumpf(card: SCard) -> bool;
    fn trumpfs_in_descending_order(mut veceschlag: Vec<ESchlag>) -> Vec<SCard>;
    fn compare_trumpfcards_solo(card_fst: SCard, card_snd: SCard) -> Ordering;
    fn count_laufende(vecstich: &Vec<SStich>, ab_winner: &[bool; 4]) -> isize {
        let veccard_trumpf = Self::trumpfs_in_descending_order(Vec::new());
        let mapcardeplayerindex = SCardMap::<EPlayerIndex>::new_from_pairs(
            vecstich.iter().flat_map(|stich| stich.indices_and_cards())
        );
        let laufende_relevant = |card: &SCard| {
            ab_winner[mapcardeplayerindex[*card]]
        };
        let b_might_have_lauf = laufende_relevant(&veccard_trumpf[0]);
        veccard_trumpf.iter()
            .take_while(|card| b_might_have_lauf==laufende_relevant(card))
            .count() as isize
    }
}

pub struct STrumpfDeciderNoTrumpf {}
impl TTrumpfDecider for STrumpfDeciderNoTrumpf {
    fn is_trumpf(_card: SCard) -> bool {
        false
    }
    fn trumpfs_in_descending_order(mut _veceschlag: Vec<ESchlag>) -> Vec<SCard> {
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
    fn trumpfs_in_descending_order(mut veceschlag: Vec<ESchlag>) -> Vec<SCard> {
        let mut veccard_trumpf : Vec<_> = EFarbe::all_values()
            .map(|efarbe| SCard::new(efarbe, SchlagDesignator::schlag()))
            .collect();
        veceschlag.push(SchlagDesignator::schlag());
        let mut veccard_trumpf_sec = DeciderSec::trumpfs_in_descending_order(veceschlag);
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

pub struct STrumpfDeciderFarbe<FarbeDesignator> {
    m_farbedesignator: PhantomData<FarbeDesignator>,
}
impl<FarbeDesignator> TTrumpfDecider for STrumpfDeciderFarbe<FarbeDesignator> 
    where FarbeDesignator: TFarbeDesignator,
{
    fn is_trumpf(card: SCard) -> bool {
        FarbeDesignator::farbe() == card.farbe()
    }
    fn trumpfs_in_descending_order(veceschlag: Vec<ESchlag>) -> Vec<SCard> {
        ESchlag::all_values()
            .filter(|eschlag| !veceschlag.iter().any(|&eschlag_done| eschlag_done==*eschlag))
            .map(|eschlag| SCard::new(FarbeDesignator::farbe(), eschlag))
            .collect()
    }
    fn compare_trumpfcards_solo(card_fst: SCard, card_snd: SCard) -> Ordering {
        assert!(Self::is_trumpf(card_fst));
        assert!(Self::is_trumpf(card_snd));
        compare_farbcards_same_color(card_fst, card_snd)
    }
}

