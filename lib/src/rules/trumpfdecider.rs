use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use std::{cmp::Ordering, marker::PhantomData};

pub trait TTrumpfDecider : Sync + 'static + Clone + fmt::Debug + Send {
    fn trumpforfarbe(&self, card: ECard) -> VTrumpfOrFarbe;

    type ItCardTrumpf: Iterator<Item=ECard>;
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf);
    fn compare_cards(&self, card_fst: ECard, card_snd: ECard) -> Option<Ordering>;

    fn equivalent_when_on_same_hand(&self, ) -> EnumMap<VTrumpfOrFarbe, Vec<ECard>> {
        let mut maptrumpforfarbeveccard = VTrumpfOrFarbe::map_from_fn(|_trumpforfarbe| Vec::new());
        for card in <ECard as PlainEnum>::values() {
            maptrumpforfarbeveccard[self.trumpforfarbe(card)].push(card);
        }
        for veccard in maptrumpforfarbeveccard.iter_mut() {
            veccard.sort_unstable_by(|card_lhs, card_rhs|
                unwrap!(self.compare_cards(*card_lhs, *card_rhs)).reverse()
            );
        }
        maptrumpforfarbeveccard
    }

    fn sort_cards_first_trumpf_then_farbe(&self, slccard: &mut [ECard]) {
        slccard.sort_unstable_by(|&card_lhs, &card_rhs| {
            match self.compare_cards(card_lhs, card_rhs) {
                Some(ord) => ord.reverse(),
                None => {
                    assert_eq!(VTrumpfOrFarbe::Farbe(card_lhs.farbe()), self.trumpforfarbe(card_lhs));
                    assert_eq!(VTrumpfOrFarbe::Farbe(card_rhs.farbe()), self.trumpforfarbe(card_rhs));
                    card_lhs.farbe().cmp(&card_rhs.farbe())
                },
            }
        });
    }
}

pub trait TCompareFarbcards : Sync + 'static + Clone + fmt::Debug + Send {
    fn compare_farbcards(card_fst: ECard, card_snd: ECard) -> Ordering;
}
#[derive(Clone, Debug, Default)]
pub struct SCompareFarbcardsSimple;
impl TCompareFarbcards for SCompareFarbcardsSimple {
    fn compare_farbcards(card_fst: ECard, card_snd: ECard) -> Ordering {
        let get_schlag_value = |card: ECard| { match card.schlag() {
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
    fn trumpforfarbe(&self, card: ECard) -> VTrumpfOrFarbe {
        VTrumpfOrFarbe::Farbe(card.farbe())
    }
    type ItCardTrumpf = std::iter::Empty<ECard>;
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf) {
        std::iter::empty()
    }
    fn compare_cards(&self, card_fst: ECard, card_snd: ECard) -> Option<Ordering> {
        if_then_some!(
            card_fst.farbe()==card_snd.farbe(),
            CompareFarbcards::compare_farbcards(card_fst, card_snd)
        )
    }
}

#[derive(Clone, Debug, Default, new)]
pub struct STrumpfDeciderSchlag<Schlag, DeciderSec> {
    schlag: Schlag,
    trumpfdecider_sec: DeciderSec,
}

impl<Schlag: TStaticOrDynamicValue<ESchlag>+Send+fmt::Debug+Copy+Sync+'static, DeciderSec: TTrumpfDecider> TTrumpfDecider for STrumpfDeciderSchlag<Schlag, DeciderSec> {
    fn trumpforfarbe(&self, card: ECard) -> VTrumpfOrFarbe {
        if self.schlag.value() == card.schlag() {
            VTrumpfOrFarbe::Trumpf
        } else {
            self.trumpfdecider_sec.trumpforfarbe(card)
        }
    }
    type ItCardTrumpf = Box<dyn Iterator<Item=ECard>>; // TODO concrete type
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf) {
        let eschlag = self.schlag.value();
        Box::new(
            EFarbe::values()
                .map(move |efarbe| ECard::new(efarbe, eschlag))
                .chain(
                    self.trumpfdecider_sec.trumpfs_in_descending_order()
                        .filter(move |card| eschlag!=card.schlag())
                )
        )
    }
    fn compare_cards(&self, card_fst: ECard, card_snd: ECard) -> Option<Ordering> {
        match (self.schlag.value()==card_fst.schlag(), self.schlag.value()==card_snd.schlag()) {
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
    fn trumpforfarbe(&self, card: ECard) -> VTrumpfOrFarbe {
        if (*self).value() == card.farbe() {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }
    type ItCardTrumpf = Box<dyn Iterator<Item=ECard>>; // TODO concrete type
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf) {
        let efarbe = (*self).value();
        Box::new(
            ESchlag::values()
                .map(move |eschlag| ECard::new(efarbe, eschlag))
        )
    }
    fn compare_cards(&self, card_fst: ECard, card_snd: ECard) -> Option<Ordering> {
        match ((*self).value()==card_fst.farbe(), (*self).value()==card_snd.farbe()) {
            (true, true) => Some(SCompareFarbcardsSimple::compare_farbcards(card_fst, card_snd)),
            (true, false) => Some(Ordering::Greater),
            (false, true) => Some(Ordering::Less),
            (false, false) => STrumpfDeciderNoTrumpf::<SCompareFarbcardsSimple>{phantom: PhantomData}.compare_cards(card_fst, card_snd),
        }
    }
}

macro_rules! impl_rules_trumpf {() => {
    fn trumpforfarbe(&self, card: ECard) -> VTrumpfOrFarbe {
        self.trumpfdecider.trumpforfarbe(card)
    }
    fn compare_cards(&self, card_fst: ECard, card_snd: ECard) -> Option<Ordering> {
        self.trumpfdecider.compare_cards(card_fst, card_snd)
    }
    fn sort_cards_first_trumpf_then_farbe(&self, slccard: &mut [ECard]) {
        self.trumpfdecider.sort_cards_first_trumpf_then_farbe(slccard)
    }
}}

#[test]
fn test_equivalent_when_on_same_hand_trumpfdecider() {
    type TrumpfDecider = STrumpfDeciderSchlag<
        SStaticSchlagOber, STrumpfDeciderSchlag<
        SStaticSchlagUnter, SStaticFarbeHerz>>;
    let maptrumpforfarbeveccard = TrumpfDecider::default().equivalent_when_on_same_hand();
    fn assert_eq_cards(slccard_lhs: &[ECard], slccard_rhs: &[ECard]) {
        assert_eq!(slccard_lhs, slccard_rhs);
    }
    use crate::primitives::ECard::*;
    assert_eq_cards(&maptrumpforfarbeveccard[VTrumpfOrFarbe::Trumpf], &[EO, GO, HO, SO, EU, GU, HU, SU, HA, HZ, HK, H9, H8, H7]);
    assert_eq_cards(&maptrumpforfarbeveccard[VTrumpfOrFarbe::Farbe(EFarbe::Eichel)], &[EA, EZ, EK, E9, E8, E7]);
    assert_eq_cards(&maptrumpforfarbeveccard[VTrumpfOrFarbe::Farbe(EFarbe::Gras)], &[GA, GZ, GK, G9, G8, G7]);
    assert_eq_cards(&maptrumpforfarbeveccard[VTrumpfOrFarbe::Farbe(EFarbe::Herz)], &[]);
    assert_eq_cards(&maptrumpforfarbeveccard[VTrumpfOrFarbe::Farbe(EFarbe::Schelln)], &[SA, SZ, SK, S9, S8, S7]);
}