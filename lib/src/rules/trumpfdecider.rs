use crate::primitives::*;
use crate::rules::*;
use crate::util::*;
use std::{
    cmp::Ordering,
    marker::PhantomData,
    num::NonZeroUsize,
};
use arrayvec::ArrayVec;

pub trait TTrumpfDecider : Sync + 'static + Clone + fmt::Debug + Send {
    fn trumpforfarbe(&self, card: ECard) -> VTrumpfOrFarbe;

    type ItCardTrumpf<'slf>: Iterator<Item=ECard>+'slf;
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf<'_>);
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
pub struct STrumpfDeciderNoTrumpf<CompareFarbcards> {
    phantom: PhantomData<CompareFarbcards>
}
impl<CompareFarbcards: TCompareFarbcards> TTrumpfDecider for STrumpfDeciderNoTrumpf<CompareFarbcards> {
    fn trumpforfarbe(&self, card: ECard) -> VTrumpfOrFarbe {
        VTrumpfOrFarbe::Farbe(card.farbe())
    }
    type ItCardTrumpf<'slf> = std::iter::Empty<ECard>;
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf<'_>) {
        std::iter::empty()
    }
    fn compare_cards(&self, card_fst: ECard, card_snd: ECard) -> Option<Ordering> {
        if_then_some!(
            card_fst.farbe()==card_snd.farbe(),
            CompareFarbcards::compare_farbcards(card_fst, card_snd)
        )
    }
}

#[derive(Clone, Debug)]
struct STrumpfDeciderInternal {
    slcschlag: &'static [ESchlag],
    oefarbe: Option<EFarbe>,
    mapschlagoi_higher_is_stronger: EnumMap<ESchlag, Option<NonZeroUsize>>,
}

impl STrumpfDeciderInternal {
    fn new_with_custom_ace_to_7_ordering(slcschlag_trumpf: &'static [ESchlag], oefarbe: impl Into<Option<EFarbe>>, itschlag_no_trumpf: impl IntoIterator<Item=ESchlag>) -> Self {
        let mut mapschlagoi_higher_is_stronger = ESchlag::map_from_fn(|_eschlag| None);
        for (i_schlag_no_trumpf, schlag_no_trumpf) in itschlag_no_trumpf.into_iter().enumerate() {
            assert!(mapschlagoi_higher_is_stronger[schlag_no_trumpf].is_none());
            mapschlagoi_higher_is_stronger[schlag_no_trumpf] = verify!(NonZeroUsize::new(ESchlag::SIZE - i_schlag_no_trumpf));
        }
        for schlag in ESchlag::values() {
            assert_eq!(mapschlagoi_higher_is_stronger[schlag].is_some(), !slcschlag_trumpf.contains(&schlag));
        }
        Self{
            slcschlag: slcschlag_trumpf,
            oefarbe: oefarbe.into(),
            mapschlagoi_higher_is_stronger,
        }
    }
}

#[derive(Clone, Debug)]
pub struct STrumpfDeciderSchlag {
    trumpfdecider: STrumpfDeciderInternal,
    veccard_trumpf_in_descending_order: ArrayVec<ECard, {ECard::SIZE}>,
}

impl STrumpfDeciderSchlag {
    pub fn new_with_custom_ace_to_7_ordering(slcschlag_trumpf: &'static [ESchlag], oefarbe: impl Into<Option<EFarbe>>, itschlag_no_trumpf: impl IntoIterator<Item=ESchlag>) -> Self {
        let trumpfdecider = STrumpfDeciderInternal::new_with_custom_ace_to_7_ordering(slcschlag_trumpf, oefarbe, itschlag_no_trumpf);
        let mut veccard_trumpf_in_descending_order : ArrayVec::<ECard, {ECard::SIZE}> = <ECard as PlainEnum>::values()
            .filter(|&card| trumpfdecider.trumpforfarbe(card).is_trumpf())
            .collect();
        veccard_trumpf_in_descending_order.sort_unstable_by(|&card_lhs, &card_rhs| {
            verify_ne!(
                verify_eq!(
                    unwrap!(trumpfdecider.compare_cards(card_lhs, card_rhs)),
                    unwrap!(trumpfdecider.compare_cards(card_rhs, card_lhs)).reverse()
                ),
                Ordering::Equal
            ).reverse()
        });
        Self {
            trumpfdecider,
            veccard_trumpf_in_descending_order,
        }
    }
    pub fn new(slcschlag_trumpf: &'static [ESchlag], oefarbe: Option<EFarbe>) -> Self {
        Self::new_with_custom_ace_to_7_ordering(
            slcschlag_trumpf,
            oefarbe,
            [ESchlag::Ass, ESchlag::Zehn, ESchlag::Koenig, ESchlag::Ober, ESchlag::Unter, ESchlag::S9, ESchlag::S8, ESchlag::S7]
                .into_iter()
                .filter(|schlag| !slcschlag_trumpf.contains(schlag)),
        )
    }
}

impl STrumpfDeciderInternal {
    fn trumpforfarbe(&self, card: ECard) -> VTrumpfOrFarbe {
        if self.slcschlag.contains(&card.schlag()) {
            VTrumpfOrFarbe::Trumpf
        } else if self.oefarbe == Some(card.farbe()) {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }
    fn compare_cards(&self, card_fst: ECard, card_snd: ECard) -> Option<Ordering> {
        let find_schlag = |schlag_card| self.slcschlag.iter().position(|&schlag_trumpf| schlag_trumpf==schlag_card);
        match (find_schlag(card_fst.schlag()), find_schlag(card_snd.schlag())) {
            (Some(i_fst), Some(i_snd)) => Some({
                match i_fst.cmp(&i_snd) {
                    Ordering::Less => Ordering::Greater,
                    Ordering::Greater => Ordering::Less,
                    Ordering::Equal => {
                        static_assert!(assert(EFarbe::Eichel < EFarbe::Gras, "Farb-Sorting can't be used here"));
                        static_assert!(assert(EFarbe::Gras < EFarbe::Herz, "Farb-Sorting can't be used here"));
                        static_assert!(assert(EFarbe::Herz < EFarbe::Schelln, "Farb-Sorting can't be used here"));
                        card_snd.farbe().cmp(&card_fst.farbe())
                    },
                }
            }),
            (Some(_i_fst), None) => Some(Ordering::Greater),
            (None, Some(_i_snd)) => Some(Ordering::Less),
            (None, None) => {
                let compare_schlag = || {
                    assert_eq!(card_fst.farbe(), card_snd.farbe());
                    let get_index = |card: ECard| self.mapschlagoi_higher_is_stronger[card.schlag()];
                    get_index(card_fst).cmp(&get_index(card_snd))
                };
                match (self.oefarbe==Some(card_fst.farbe()), self.oefarbe==Some(card_snd.farbe())) {
                    (true, true) => Some(compare_schlag()),
                    (true, false) => Some(Ordering::Greater),
                    (false, true) => Some(Ordering::Less),
                    (false, false) => if_then_some!(card_fst.farbe()==card_snd.farbe(), compare_schlag()),
                }
            }
        }
    }
}

impl TTrumpfDecider for STrumpfDeciderSchlag {
    fn trumpforfarbe(&self, card: ECard) -> VTrumpfOrFarbe {
        self.trumpfdecider.trumpforfarbe(card)
    }
    type ItCardTrumpf<'slf> = Box<dyn Iterator<Item=ECard>+'slf>; // TODO concrete type
    fn trumpfs_in_descending_order(&self, ) -> return_impl!(Self::ItCardTrumpf<'_>) {
        Box::new(self.veccard_trumpf_in_descending_order.iter().copied())
    }
    fn compare_cards(&self, card_fst: ECard, card_snd: ECard) -> Option<Ordering> {
        self.trumpfdecider.compare_cards(card_fst, card_snd)
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
    let maptrumpforfarbeveccard = STrumpfDeciderSchlag::new(&[ESchlag::Ober, ESchlag::Unter], Some(EFarbe::Herz)).equivalent_when_on_same_hand();
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
