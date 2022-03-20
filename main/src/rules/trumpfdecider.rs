use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use std::{cmp::Ordering, marker::PhantomData};

pub trait TTrumpfDecider : Sync + 'static + Clone + fmt::Debug + Send {
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe;

    type ItCardTrumpf: Iterator<Item=SCard>;
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf);
    fn compare_cards(&self, card_fst: SCard, card_snd: SCard) -> Option<Ordering>;

    fn equivalent_when_on_same_hand(&self, ) -> (EnumMap<EFarbe, Vec<SCard>>, /*veccard_trumpf*/Vec<SCard>) {
        let mut mapefarbeveccard = EFarbe::map_from_fn(|_efarbe| Vec::new());
        let mut veccard_trumpf = Vec::new();
        for card in <SCard as TPlainEnum>::values() {
            match self.trumpforfarbe(card) {
                VTrumpfOrFarbe::Trumpf => veccard_trumpf.push(card),
                VTrumpfOrFarbe::Farbe(efarbe) => mapefarbeveccard[efarbe].push(card),
            }
        }
        for veccard in mapefarbeveccard.iter_mut() {
            veccard.sort_unstable_by(|card_lhs, card_rhs|
                unwrap!(self.compare_cards(*card_lhs, *card_rhs)).reverse()
            );
        }
        veccard_trumpf.sort_unstable_by(|card_lhs, card_rhs|
            unwrap!(self.compare_cards(*card_lhs, *card_rhs)).reverse()
        );
        (mapefarbeveccard, veccard_trumpf)
    }
}

pub trait TCompareFarbcards : Sync + 'static + Clone + fmt::Debug + Send {
    fn compare_farbcards(card_fst: SCard, card_snd: SCard) -> Ordering;
}
#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Debug, Default)]
pub struct STrumpfDeciderNoTrumpf<CompareFarbcards> {
    phantom: PhantomData<CompareFarbcards>
}
impl<CompareFarbcards: TCompareFarbcards> TTrumpfDecider for STrumpfDeciderNoTrumpf<CompareFarbcards> {
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe {
        VTrumpfOrFarbe::Farbe(card.farbe())
    }
    type ItCardTrumpf = std::iter::Empty<SCard>;
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf) {
        std::iter::empty()
    }
    fn compare_cards(&self, card_fst: SCard, card_snd: SCard) -> Option<Ordering> {
        if_then_some!(
            card_fst.farbe()==card_snd.farbe(),
            CompareFarbcards::compare_farbcards(card_fst, card_snd)
        )
    }
}

#[derive(Clone, Debug, Default)]
pub struct STrumpfDeciderSchlag<StaticSchlag, DeciderSec> {
    phantom: PhantomData<StaticSchlag>,
    trumpfdecider_sec: DeciderSec,
}
fn static_schlag<StaticSchlag: TStaticValue<ESchlag>>(card: &SCard) -> bool {
    StaticSchlag::VALUE!=card.schlag()
}
impl<StaticSchlag: TStaticValue<ESchlag>, DeciderSec: TTrumpfDecider> TTrumpfDecider for STrumpfDeciderSchlag<StaticSchlag, DeciderSec> {
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe {
        if StaticSchlag::VALUE == card.schlag() {
            VTrumpfOrFarbe::Trumpf
        } else {
            self.trumpfdecider_sec.trumpforfarbe(card)
        }
    }
    type ItCardTrumpf = Box<dyn Iterator<Item=SCard>>; // TODO concrete type
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf) {
        Box::new(
            EFarbe::values()
                .map(|efarbe| SCard::new(efarbe, StaticSchlag::VALUE))
                .chain(
                    self.trumpfdecider_sec.trumpfs_in_descending_order()
                        .filter(static_schlag::<StaticSchlag>)
                )
        )
    }
    fn compare_cards(&self, card_fst: SCard, card_snd: SCard) -> Option<Ordering> {
        match (StaticSchlag::VALUE==card_fst.schlag(), StaticSchlag::VALUE==card_snd.schlag()) {
            (true, true) => {
                static_assert!(assert(EFarbe::Eichel < EFarbe::Gras, "Farb-Sorting can't be used here"));
                static_assert!(assert(EFarbe::Gras < EFarbe::Herz, "Farb-Sorting can't be used here"));
                static_assert!(assert(EFarbe::Herz < EFarbe::Schelln, "Farb-Sorting can't be used here"));
                Some(card_snd.farbe().cmp(&card_fst.farbe()))
            },
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => self.trumpfdecider_sec.compare_cards(card_fst, card_snd),
        }
    }
}

impl<Farbe: TStaticOrDynamicValue<EFarbe> + Send + fmt::Debug + Copy + Sync + 'static> TTrumpfDecider for Farbe {
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe {
        if (*self).value() == card.farbe() {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }
    type ItCardTrumpf = Box<dyn Iterator<Item=SCard>>; // TODO concrete type
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf) {
        let efarbe = (*self).value();
        Box::new(
            ESchlag::values()
                .map(move |eschlag| SCard::new(efarbe, eschlag))
        )
    }
    fn compare_cards(&self, card_fst: SCard, card_snd: SCard) -> Option<Ordering> {
        match ((*self).value()==card_fst.farbe(), (*self).value()==card_snd.farbe()) {
            (true, true) => Some(SCompareFarbcardsSimple::compare_farbcards(card_fst, card_snd)),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => STrumpfDeciderNoTrumpf::<SCompareFarbcardsSimple>{phantom: PhantomData}.compare_cards(card_fst, card_snd),
        }
    }
}

macro_rules! impl_rules_trumpf {() => {
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe {
        self.trumpfdecider.trumpforfarbe(card)
    }
    fn compare_cards(&self, card_fst: SCard, card_snd: SCard) -> Option<Ordering> {
        self.trumpfdecider.compare_cards(card_fst, card_snd)
    }
}}

macro_rules! impl_rules_trumpf_noobj{($trumpfdecider: ty) => {
    type TrumpfDecider = $trumpfdecider;
    fn trumpfdecider(&self) -> &Self::TrumpfDecider {
        &self.trumpfdecider
    }
}}

#[test]
fn test_equivalent_when_on_same_hand_trumpfdecider() {
    type TrumpfDecider = STrumpfDeciderSchlag<
        SStaticSchlagOber, STrumpfDeciderSchlag<
        SStaticSchlagUnter, SStaticFarbeHerz>>;
    let (mapefarbeveccard, veccard_trumpf) = TrumpfDecider::default().equivalent_when_on_same_hand();
    fn assert_eq_cards(slccard_lhs: &[SCard], slccard_rhs: &[SCard]) {
        assert_eq!(slccard_lhs, slccard_rhs);
    }
    use crate::primitives::card_values::*;
    assert_eq_cards(&veccard_trumpf, &[EO, GO, HO, SO, EU, GU, HU, SU, HA, HZ, HK, H9, H8, H7]);
    assert_eq_cards(&mapefarbeveccard[EFarbe::Eichel], &[EA, EZ, EK, E9, E8, E7]);
    assert_eq_cards(&mapefarbeveccard[EFarbe::Gras], &[GA, GZ, GK, G9, G8, G7]);
    assert_eq_cards(&mapefarbeveccard[EFarbe::Herz], &[]);
    assert_eq_cards(&mapefarbeveccard[EFarbe::Schelln], &[SA, SZ, SK, S9, S8, S7]);
}
