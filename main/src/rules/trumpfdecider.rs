use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use std::{cmp::Ordering, marker::PhantomData};

pub trait TTrumpfDecider : Sync + 'static + Clone + fmt::Debug + Send {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe;

    type ItCardTrumpf: Iterator<Item=SCard>;
    fn trumpfs_in_descending_order() -> return_impl!(Self::ItCardTrumpf);
    fn compare_cards(card_fst: SCard, card_snd: SCard) -> Option<Ordering>;
}

pub trait TCompareFarbcards : Sync + 'static + Clone + fmt::Debug + Send {
    fn compare_farbcards(card_fst: SCard, card_snd: SCard) -> Ordering;
}
#[derive(Clone, Debug)]
pub struct SCompareFarbcardsSimple;
impl TCompareFarbcards for SCompareFarbcardsSimple {
    fn compare_farbcards(card_fst: SCard, card_snd: SCard) -> Ordering {
        let get_schlag_value = |card: SCard| { match card.schlag() {
            ESchlag::S7 => 0,
            ESchlag::S8 => 1,
            ESchlag::S9 => 2,
            ESchlag::Unter => 3,
            ESchlag::Ober => 4,
            ESchlag::Koenig => 5,
            ESchlag::Zehn => 6,
            ESchlag::Ass => 7,
        } };
        get_schlag_value(card_fst).cmp(&get_schlag_value(card_snd))
    }
}

#[derive(Clone, Debug)]
pub struct STrumpfDeciderNoTrumpf<CompareFarbcards> {
    phantom: PhantomData<CompareFarbcards>
}
impl<CompareFarbcards: TCompareFarbcards> TTrumpfDecider for STrumpfDeciderNoTrumpf<CompareFarbcards> {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        VTrumpfOrFarbe::Farbe(card.farbe())
    }
    type ItCardTrumpf = std::iter::Empty<SCard>;
    fn trumpfs_in_descending_order() -> return_impl!(Self::ItCardTrumpf) {
        std::iter::empty()
    }
    fn compare_cards(card_fst: SCard, card_snd: SCard) -> Option<Ordering> {
        if_then_some!(
            card_fst.farbe()==card_snd.farbe(),
            CompareFarbcards::compare_farbcards(card_fst, card_snd)
        )
    }
}

#[derive(Clone, Debug)]
pub struct STrumpfDeciderSchlag<StaticSchlag, DeciderSec> {
    phantom: PhantomData<(StaticSchlag, DeciderSec)>,
}
fn static_schlag<StaticSchlag: TStaticValue<ESchlag>>(card: &SCard) -> bool {
    StaticSchlag::VALUE!=card.schlag()
}
impl<StaticSchlag: TStaticValue<ESchlag>, DeciderSec: TTrumpfDecider> TTrumpfDecider for STrumpfDeciderSchlag<StaticSchlag, DeciderSec> {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        if StaticSchlag::VALUE == card.schlag() {
            VTrumpfOrFarbe::Trumpf
        } else {
            DeciderSec::trumpforfarbe(card)
        }
    }
    type ItCardTrumpf = Box<dyn Iterator<Item=SCard>>; // TODO concrete type
    fn trumpfs_in_descending_order() -> return_impl!(Self::ItCardTrumpf) {
        Box::new(
            EFarbe::values()
                .map(|efarbe| SCard::new(efarbe, StaticSchlag::VALUE))
                .chain(
                    DeciderSec::trumpfs_in_descending_order()
                        .filter(static_schlag::<StaticSchlag>)
                )
        )
    }
    fn compare_cards(card_fst: SCard, card_snd: SCard) -> Option<Ordering> {
        match (StaticSchlag::VALUE==card_fst.schlag(), StaticSchlag::VALUE==card_snd.schlag()) {
            (true, true) => {
                static_assert!(assert(EFarbe::Eichel < EFarbe::Gras, "Farb-Sorting can't be used here"));
                static_assert!(assert(EFarbe::Gras < EFarbe::Herz, "Farb-Sorting can't be used here"));
                static_assert!(assert(EFarbe::Herz < EFarbe::Schelln, "Farb-Sorting can't be used here"));
                Some(card_snd.farbe().cmp(&card_fst.farbe()))
            },
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => DeciderSec::compare_cards(card_fst, card_snd),
        }
    }
}

impl<StaticFarbe: TStaticValue<EFarbe>> TTrumpfDecider for StaticFarbe {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        if StaticFarbe::VALUE == card.farbe() {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }
    #[allow(clippy::type_complexity)] // covered by the fact that return_impl should go away
    type ItCardTrumpf = std::iter::Map<std::iter::Map<std::ops::Range<usize>, fn(usize) -> ESchlag>, fn(ESchlag) -> SCard>;
    fn trumpfs_in_descending_order() -> return_impl!(Self::ItCardTrumpf) {
        ESchlag::values()
            .map(|eschlag| SCard::new(StaticFarbe::VALUE, eschlag))
    }
    fn compare_cards(card_fst: SCard, card_snd: SCard) -> Option<Ordering> {
        match (StaticFarbe::VALUE==card_fst.farbe(), StaticFarbe::VALUE==card_snd.farbe()) {
            (true, true) => Some(SCompareFarbcardsSimple::compare_farbcards(card_fst, card_snd)),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => STrumpfDeciderNoTrumpf::<SCompareFarbcardsSimple>::compare_cards(card_fst, card_snd),
        }
    }
}

macro_rules! impl_rules_trumpf {() => {
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe {
        <Self as TRulesNoObj>::TrumpfDecider::trumpforfarbe(card)
    }
    fn compare_cards(&self, card_fst: SCard, card_snd: SCard) -> Option<Ordering> {
        <Self as TRulesNoObj>::TrumpfDecider::compare_cards(card_fst, card_snd)
    }
}}

macro_rules! impl_rules_trumpf_noobj{($trumpfdecider: ty) => {
    type TrumpfDecider = $trumpfdecider;
}}
