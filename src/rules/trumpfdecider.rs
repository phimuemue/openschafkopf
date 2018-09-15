use primitives::*;
use rules::*;
use std::{
    cmp::Ordering,
    marker::PhantomData,
};
use util::*;

pub trait TTrumpfDecider : Sync + 'static + Clone + fmt::Debug {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe;

    fn trumpfs_in_descending_order() -> return_impl!(Box<Iterator<Item=SCard>>);
    fn compare_trumpf(card_fst: SCard, card_snd: SCard) -> Ordering;
    fn count_laufende<PlayerParties: TPlayerParties>(gamefinishedstiche: SGameFinishedStiche, playerparties: &PlayerParties) -> usize {
        #[cfg(debug_assertions)]
        let mut mapcardb_used = SCard::map_from_fn(|_card| false);
        let mapcardepi = {
            let mut mapcardepi = SCard::map_from_fn(|_card| EPlayerIndex::EPI0);
            for (epi, card) in gamefinishedstiche.get().iter().flat_map(|stich| stich.iter()) {
                #[cfg(debug_assertions)] {
                    mapcardb_used[*card] = true;
                }
                mapcardepi[*card] = epi;
            }
            mapcardepi
        };
        let ekurzlang = EKurzLang::from_cards_per_player(gamefinishedstiche.get().len());
        #[cfg(debug_assertions)]
        assert!(SCard::values(ekurzlang).all(|card| mapcardb_used[card]));
        let laufende_relevant = |card: SCard| {
            playerparties.is_primary_party(mapcardepi[card])
        };
        let mut itcard_trumpf_descending = Self::trumpfs_in_descending_order();
        let b_might_have_lauf = laufende_relevant(verify!(itcard_trumpf_descending.nth(0)).unwrap());
        itcard_trumpf_descending
            .filter(|card| ekurzlang.supports_card(*card))
            .take_while(|card| b_might_have_lauf==laufende_relevant(*card))
            .count()
            + 1 // consumed by nth(0)
    }
}

#[derive(Clone, Debug)]
pub struct STrumpfDeciderNoTrumpf {}
impl TTrumpfDecider for STrumpfDeciderNoTrumpf {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        VTrumpfOrFarbe::Farbe(card.farbe())
    }
    fn trumpfs_in_descending_order() -> return_impl!(Box<Iterator<Item=SCard>>) {
        Box::new(None.into_iter())
    }
    fn compare_trumpf(_card_fst: SCard, _card_snd: SCard) -> Ordering {
        panic!("STrumpfDeciderNoTrumpf::compare_trumpf called")
    }
}

#[derive(Clone, Debug)]
pub struct STrumpfDeciderSchlag<StaticSchlag, DeciderSec> {
    staticschlag: PhantomData<StaticSchlag>,
    decidersec: PhantomData<DeciderSec>,
}
impl<StaticSchlag, DeciderSec> TTrumpfDecider for STrumpfDeciderSchlag<StaticSchlag, DeciderSec> 
    where DeciderSec: TTrumpfDecider,
          StaticSchlag: TStaticValue<ESchlag>,
{
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        if StaticSchlag::VALUE == card.schlag() {
            VTrumpfOrFarbe::Trumpf
        } else {
            DeciderSec::trumpforfarbe(card)
        }
    }
    fn trumpfs_in_descending_order() -> return_impl!(Box<Iterator<Item=SCard>>) {
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
    staticfarbe: PhantomData<StaticFarbe>,
}
impl<StaticFarbe> TTrumpfDecider for STrumpfDeciderFarbe<StaticFarbe> 
    where StaticFarbe: TStaticValue<EFarbe>,
{
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        if StaticFarbe::VALUE == card.farbe() {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(card.farbe())
        }
    }
    fn trumpfs_in_descending_order() -> return_impl!(Box<Iterator<Item=SCard>>) {
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
