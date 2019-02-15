use crate::primitives::*;
use crate::rules::*;
use std::{
    cmp::Ordering,
    marker::PhantomData,
};
use crate::util::*;

pub trait TTrumpfDecider : Sync + 'static + Clone + fmt::Debug {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe;

    fn trumpfs_in_descending_order() -> return_impl!(Box<dyn Iterator<Item=SCard>>);
    fn compare_trumpf(card_fst: SCard, card_snd: SCard) -> Ordering;
}

#[derive(Clone, Debug)]
pub struct STrumpfDeciderNoTrumpf {}
impl TTrumpfDecider for STrumpfDeciderNoTrumpf {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        VTrumpfOrFarbe::Farbe(card.farbe())
    }
    fn trumpfs_in_descending_order() -> return_impl!(Box<dyn Iterator<Item=SCard>>) {
        Box::new(None.into_iter())
    }
    fn compare_trumpf(_card_fst: SCard, _card_snd: SCard) -> Ordering {
        panic!("STrumpfDeciderNoTrumpf::compare_trumpf called")
    }
}

#[derive(Clone, Debug)]
pub struct STrumpfDeciderSchlag<StaticSchlag, DeciderSec> {
    phantom: PhantomData<(StaticSchlag, DeciderSec)>,
}
impl<StaticSchlag: TStaticValue<ESchlag>, DeciderSec: TTrumpfDecider> TTrumpfDecider for STrumpfDeciderSchlag<StaticSchlag, DeciderSec> {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        if StaticSchlag::VALUE == card.schlag() {
            VTrumpfOrFarbe::Trumpf
        } else {
            DeciderSec::trumpforfarbe(card)
        }
    }
    fn trumpfs_in_descending_order() -> return_impl!(Box<dyn Iterator<Item=SCard>>) {
        Box::new(
            EFarbe::values()
                .map(|efarbe| SCard::new(efarbe, StaticSchlag::VALUE))
                .chain(
                    DeciderSec::trumpfs_in_descending_order()
                        .filter(|card| StaticSchlag::VALUE!=card.schlag())
                )
        )
    }
    fn compare_trumpf(card_fst: SCard, card_snd: SCard) -> Ordering {
        match (StaticSchlag::VALUE==card_fst.schlag(), StaticSchlag::VALUE==card_snd.schlag()) {
            (true, true) => {
                static_assert!(assert(EFarbe::Eichel < EFarbe::Gras, "Farb-Sorting can't be used here"));
                static_assert!(assert(EFarbe::Gras < EFarbe::Herz, "Farb-Sorting can't be used here"));
                static_assert!(assert(EFarbe::Herz < EFarbe::Schelln, "Farb-Sorting can't be used here"));
                card_snd.farbe().cmp(&card_fst.farbe())
            },
            (true, false) => Ordering::Greater,
            (false, true) => Ordering::Less,
            (false, false) => DeciderSec::compare_trumpf(card_fst, card_snd),
        }
    }
}

#[derive(Clone, Debug)]
pub struct STrumpfDeciderFarbe<StaticFarbe> {
    phantom: PhantomData<StaticFarbe>,
}
impl<StaticFarbe: TStaticValue<EFarbe>> TTrumpfDecider for STrumpfDeciderFarbe<StaticFarbe> {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        if StaticFarbe::VALUE == card.farbe() {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }
    fn trumpfs_in_descending_order() -> return_impl!(Box<dyn Iterator<Item=SCard>>) {
        Box::new(
            ESchlag::values()
                .map(|eschlag| SCard::new(StaticFarbe::VALUE, eschlag))
        )
    }
    fn compare_trumpf(card_fst: SCard, card_snd: SCard) -> Ordering {
        assert!(Self::trumpforfarbe(card_fst).is_trumpf());
        assert!(Self::trumpforfarbe(card_snd).is_trumpf());
        compare_farbcards_same_color(card_fst, card_snd)
    }
}

macro_rules! impl_rules_trumpf {() => {
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe {
        <Self as TRulesNoObj>::TrumpfDecider::trumpforfarbe(card)
    }
    fn compare_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        <Self as TRulesNoObj>::TrumpfDecider::compare_trumpf(card_fst, card_snd)
    }
}}

macro_rules! impl_rules_trumpf_noobj{($trumpfdecider: ident) => {
    type TrumpfDecider = $trumpfdecider;
}}
