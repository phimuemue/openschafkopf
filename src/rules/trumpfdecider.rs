use primitives::*;
use rules::*;
use std::{
    cmp::Ordering,
    marker::PhantomData,
};
use util::*;

pub trait TTrumpfDecider : Sync + 'static + Clone + fmt::Debug {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe;

    fn trumpfs_in_descending_order(veceschlag: Vec<ESchlag>) -> return_impl!(Vec<SCard>);
    fn compare_trumpf(card_fst: SCard, card_snd: SCard) -> Ordering;
    fn count_laufende(gamefinishedstiche: &SGameFinishedStiche, ab_winner: &EnumMap<EPlayerIndex, bool>) -> usize {
        let veccard_trumpf = Self::trumpfs_in_descending_order(Vec::new());
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
        let laufende_relevant = |card: &SCard| {
            ab_winner[mapcardepi[*card]]
        };
        let b_might_have_lauf = laufende_relevant(&veccard_trumpf[0]);
        veccard_trumpf.iter()
            .filter(|&card| ekurzlang.supports_card(*card))
            .take_while(|card| b_might_have_lauf==laufende_relevant(card))
            .count()
    }
}

#[derive(Clone, Debug)]
pub struct STrumpfDeciderNoTrumpf {}
impl TTrumpfDecider for STrumpfDeciderNoTrumpf {
    fn trumpforfarbe(card: SCard) -> VTrumpfOrFarbe {
        VTrumpfOrFarbe::Farbe(card.farbe())
    }
    fn trumpfs_in_descending_order(mut _veceschlag: Vec<ESchlag>) -> return_impl!(Vec<SCard>) {
        Vec::new()
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
    fn trumpfs_in_descending_order(mut veceschlag: Vec<ESchlag>) -> return_impl!(Vec<SCard>) {
        let mut veccard_trumpf : Vec<_> = EFarbe::values()
            .map(|efarbe| SCard::new(efarbe, StaticSchlag::VALUE))
            .collect();
        veceschlag.push(StaticSchlag::VALUE);
        let mut veccard_trumpf_sec = DeciderSec::trumpfs_in_descending_order(veceschlag);
        veccard_trumpf.append(&mut veccard_trumpf_sec);
        veccard_trumpf
    }
    fn compare_trumpf(card_fst: SCard, card_snd: SCard) -> Ordering {
        match (StaticSchlag::VALUE==card_fst.schlag(), StaticSchlag::VALUE==card_snd.schlag()) {
            (true, true) => {
                static_assert!(assert(EFarbe::Eichel < EFarbe::Gras, "Farb-Sorting can't be used here"));
                static_assert!(assert(EFarbe::Gras < EFarbe::Herz, "Farb-Sorting can't be used here"));
                static_assert!(assert(EFarbe::Herz < EFarbe::Schelln, "Farb-Sorting can't be used here"));
                if card_snd.farbe() < card_fst.farbe() {
                    Ordering::Less
                } else {
                    Ordering::Greater
                }
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
    fn trumpfs_in_descending_order(veceschlag: Vec<ESchlag>) -> return_impl!(Vec<SCard>) {
        ESchlag::values()
            .filter(|eschlag| !veceschlag.iter().any(|&eschlag_done| eschlag_done==*eschlag))
            .map(|eschlag| SCard::new(StaticFarbe::VALUE, eschlag))
            .collect()
    }
    fn compare_trumpf(card_fst: SCard, card_snd: SCard) -> Ordering {
        assert!(Self::trumpforfarbe(card_fst).is_trumpf());
        assert!(Self::trumpforfarbe(card_snd).is_trumpf());
        compare_farbcards_same_color(card_fst, card_snd)
    }
}

macro_rules! impl_rules_trumpf {($trumpfdecider: ident) => {
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe {
        $trumpfdecider::trumpforfarbe(card)
    }
    fn compare_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        $trumpfdecider::compare_trumpf(card_fst, card_snd)
    }
    fn count_laufende(&self, gamefinishedstiche: &SGameFinishedStiche, ab_winner: &EnumMap<EPlayerIndex, bool>) -> usize {
        $trumpfdecider::count_laufende(gamefinishedstiche, ab_winner)
    }
}}
