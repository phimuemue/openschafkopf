#[macro_use]
pub mod trumpfdecider;
#[macro_use]
pub mod singleplay;
pub mod rulesrufspiel;
// TODORULES implement Hochzeit
pub mod rulessolo;
pub mod rulesramsch;
pub mod rulesbettel;
pub mod ruleset;
pub mod payoutdecider;
pub mod card_points;

#[cfg(test)]
pub mod tests;

use primitives::*;
use std::{
    cmp::Ordering,
    fmt,
};
use util::*;
use ai::rulespecific::*;

pub fn current_stich_mut(slcstich: &mut [SStich]) -> &mut SStich {
    verify!(slcstich.last_mut()).unwrap()
}

pub fn current_stich(slcstich: &[SStich]) -> &SStich {
    verify!(slcstich.last()).unwrap()
}

pub fn completed_stichs(slcstich: &[SStich]) -> SCompletedStichs {
    assert!(current_stich(slcstich).size()<4);
    assert_eq!(slcstich[0..slcstich.len()-1].len(), slcstich.len()-1);
    assert!(slcstich[0..slcstich.len()-1].iter().all(|stich| stich.size()==4));
    SCompletedStichs::new(&slcstich[0..slcstich.len()-1])
}

#[derive(PartialEq, Eq, Debug)]
pub enum VTrumpfOrFarbe {
    Trumpf,
    Farbe (EFarbe),
}

impl VTrumpfOrFarbe {
    pub fn is_trumpf(&self) -> bool {
        match *self {
            VTrumpfOrFarbe::Trumpf => true,
            VTrumpfOrFarbe::Farbe(_efarbe) => false,
        }
    }
}

#[derive(Debug)]
pub struct SStoss {
    pub epi : EPlayerIndex,
}

fn all_allowed_cards_within_stich_distinguish_farbe_frei<Rules, Result, FnFarbeFrei, FnFarbeNotFrei>(
    rules: &Rules,
    slcstich: &[SStich],
    hand: &SHand,
    fn_farbe_frei: FnFarbeFrei,
    fn_farbe_not_frei: FnFarbeNotFrei,
) -> Result
    where 
        Rules: TRules + ?Sized,
        FnFarbeFrei: Fn() -> Result,
        FnFarbeNotFrei: Fn(SHandVector) -> Result,
{
    assert!(!slcstich.is_empty());
    let trumpforfarbe_first = rules.trumpforfarbe(*current_stich(slcstich).first());
    let veccard_same_farbe : SHandVector = hand.cards().iter().cloned()
        .filter(|&card| rules.trumpforfarbe(card)==trumpforfarbe_first)
        .collect();
    if veccard_same_farbe.is_empty() {
        fn_farbe_frei()
    } else {
        fn_farbe_not_frei(veccard_same_farbe)
    }
}

#[derive(Eq, PartialEq)]
pub enum EStockAction {
    Ignore,
    TakeHalf,
    GiveHalf,
}

#[derive(Eq, PartialEq, new)]
pub struct SPayoutInfo {
    n_payout: isize,
    estockaction: EStockAction,
}

impl SPayoutInfo {
    fn payout_including_stock(&self, n_stock: isize) -> isize {
        assert_eq!(n_stock%2, 0);
        assert!(self.estockaction!=EStockAction::TakeHalf || 0<self.n_payout);
        assert!(self.estockaction!=EStockAction::GiveHalf || self.n_payout<0);
        self.n_payout + match self.estockaction {
            EStockAction::Ignore => 0,
            EStockAction::TakeHalf => n_stock/2,
            EStockAction::GiveHalf => -n_stock/2,
        }
    }
}

pub trait TRules : fmt::Display + TAsRules + Sync + fmt::Debug {
    box_clone_require!(TRules);

    // TTrumpfDecider
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe;
    fn compare_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering;
    fn count_laufende(&self, gamefinishedstiche: SGameFinishedStiche, ab_winner: &EnumMap<EPlayerIndex, bool>) -> usize;

    fn playerindex(&self) -> Option<EPlayerIndex>;

    fn can_be_played(&self, _hand: SFullHand) -> bool {
        true // probably, only Rufspiel is prevented in some cases
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool;

    fn payout(&self, gamefinishedstiche: SGameFinishedStiche, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> SAccountBalance {
        let apayoutinfo = self.payoutinfos(gamefinishedstiche, tpln_stoss_doubling);
        assert!({
            let count_stockaction = |estockaction| {
                apayoutinfo.iter().filter(|payoutinfo| estockaction==payoutinfo.estockaction).count()
            };
            count_stockaction(EStockAction::TakeHalf)==0 || count_stockaction(EStockAction::GiveHalf)==0
        });
        assert_eq!(n_stock%2, 0);
        let an_payout = apayoutinfo.map(|payoutinfo| payoutinfo.payout_including_stock(n_stock));
        let n_stock = -an_payout.iter().sum::<isize>();
        SAccountBalance::new(an_payout, n_stock)
    }

    fn payoutinfos(&self, gamefinishedstiche: SGameFinishedStiche, tpln_stoss_doubling: (usize, usize)) -> EnumMap<EPlayerIndex, SPayoutInfo>;

    fn all_allowed_cards(&self, slcstich: &[SStich], hand: &SHand) -> SHandVector {
        assert!(!slcstich.is_empty());
        assert!(current_stich(slcstich).size()<4);
        if 0==current_stich(slcstich).size() {
            self.all_allowed_cards_first_in_stich(slcstich, hand)
        } else {
            self.all_allowed_cards_within_stich(slcstich, hand)
        }
    }

    fn all_allowed_cards_first_in_stich(&self, _vecstich: &[SStich], hand: &SHand) -> SHandVector {
        // probably in most cases, every card can be played
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, slcstich: &[SStich], hand: &SHand) -> SHandVector {
        // probably in most cases, only the first card of the current stich is decisive
        all_allowed_cards_within_stich_distinguish_farbe_frei(
            self,
            slcstich,
            hand,
            /*fn_farbe_frei*/|| hand.cards().clone(),
            /*fn_farbe_not_frei*/|veccard_same_farbe| veccard_same_farbe
        )
    }

    fn card_is_allowed(&self, slcstich: &[SStich], hand: &SHand, card: SCard) -> bool {
        self.all_allowed_cards(slcstich, hand).into_iter()
            .any(|card_iterated| card_iterated==card)
    }

    fn winner_index(&self, stich: &SStich) -> EPlayerIndex {
        let mut epi_best = stich.epi_first;
        for (epi, card) in stich.iter().skip(1) {
            if Ordering::Less==self.compare_in_stich(stich[epi_best], *card) {
                epi_best = epi;
            }
        }
        epi_best
    }

    fn compare_in_stich_same_farbe(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        assert_eq!(self.trumpforfarbe(card_fst), self.trumpforfarbe(card_snd));
        assert_eq!(card_fst.farbe(), card_snd.farbe());
        compare_farbcards_same_color(card_fst, card_snd)
    }

    fn compare_in_stich(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        assert_ne!(card_fst, card_snd);
        match (self.trumpforfarbe(card_fst), self.trumpforfarbe(card_snd)) {
            (VTrumpfOrFarbe::Trumpf, VTrumpfOrFarbe::Farbe(_)) => Ordering::Greater,
            (VTrumpfOrFarbe::Farbe(_), VTrumpfOrFarbe::Trumpf) => Ordering::Less,
            (VTrumpfOrFarbe::Trumpf, VTrumpfOrFarbe::Trumpf) => self.compare_trumpf(card_fst, card_snd),
            (VTrumpfOrFarbe::Farbe(efarbe_fst), VTrumpfOrFarbe::Farbe(efarbe_snd)) => {
                if efarbe_fst != efarbe_snd {
                    Ordering::Greater
                } else {
                    self.compare_in_stich_same_farbe(card_fst, card_snd)
                }
            }
        }
    }

    fn sort_cards_first_trumpf_then_farbe(&self, veccard: &mut [SCard]) {
        veccard.sort_unstable_by(|&card_lhs, &card_rhs| {
            match(self.trumpforfarbe(card_lhs), self.trumpforfarbe(card_rhs)) {
                (VTrumpfOrFarbe::Farbe(efarbe_lhs), VTrumpfOrFarbe::Farbe(efarbe_rhs)) => {
                    if efarbe_lhs==efarbe_rhs {
                        self.compare_in_stich_same_farbe(card_rhs, card_lhs)
                    } else {
                        efarbe_lhs.cmp(&efarbe_rhs)
                    }
                }
                (_, _) => { // at least one of them is trumpf
                    self.compare_in_stich(card_rhs, card_lhs)
                }
            }
        });
    }

    fn rulespecific_ai<'rules>(&'rules self) -> Option<Box<TRuleSpecificAI + 'rules>> {
        None
    }
}
box_clone_impl_box!(TRules);

make_upcastable!(TAsRules, TRules);

#[derive(PartialEq, Eq, Clone, PartialOrd, Ord, Debug)]
pub enum VGameAnnouncementPrioritySoloLike {
    // state priorities in ascending order
    SoloSimple(isize),
    SoloSteigern{n_points_to_win: isize, n_step: isize},
}

#[derive(PartialEq, Eq, Clone, PartialOrd, Ord, Debug)]
pub enum VGameAnnouncementPriority {
    // state priorities in ascending order
    RufspielLike,
    SoloLike(VGameAnnouncementPrioritySoloLike),
    SoloTout(isize),
    SoloSie,
}

#[test]
fn test_gameannouncementprio() {
    use self::VGameAnnouncementPriority::*;
    use self::VGameAnnouncementPrioritySoloLike::*;
    assert!(RufspielLike==RufspielLike);
    assert!(RufspielLike<SoloLike(SoloSimple(0)));
    assert!(RufspielLike<SoloTout(0));
    assert!(RufspielLike<SoloSie);
    assert!(SoloLike(SoloSimple(0))>RufspielLike);
    assert!(SoloLike(SoloSimple(0))==SoloLike(SoloSimple(0)));
    assert!(SoloLike(SoloSimple(0))<SoloTout(0));
    assert!(SoloLike(SoloSimple(0))<SoloSie);
    assert!(SoloTout(0)>RufspielLike);
    assert!(SoloTout(0)>SoloLike(SoloSimple(0)));
    assert!(SoloTout(0)==SoloTout(0));
    assert!(SoloTout(0)<SoloSie);
    assert!(SoloSie>RufspielLike);
    assert!(SoloSie>SoloLike(SoloSimple(0)));
    assert!(SoloSie>SoloTout(0));
    assert!(SoloSie==SoloSie);
    assert!(SoloLike(SoloSimple(0))<SoloLike(SoloSimple(1)));
    assert!(SoloTout(0)<SoloTout(1));
}

plain_enum_mod!(modebid, EBid {
    AtLeast,
    Higher,
});

pub trait TActivelyPlayableRules : TRules {
    box_clone_require!(TActivelyPlayableRules);
    fn priority(&self) -> VGameAnnouncementPriority;
    fn with_higher_prio_than(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Box<TActivelyPlayableRules>> {
        if match ebid {
            EBid::AtLeast => {*prio<=self.priority()},
            EBid::Higher => {*prio<self.priority()},
        } {
            Some(TActivelyPlayableRules::box_clone(self))
        } else {
            self.with_increased_prio(prio, ebid)
        }
    }
    fn with_increased_prio(&self, _prio: &VGameAnnouncementPriority, _ebid: EBid) -> Option<Box<TActivelyPlayableRules>> {
        None
    }
    fn active_playerindex(&self) -> EPlayerIndex {
        verify!(self.playerindex()).unwrap()
    }
}
box_clone_impl_box!(TActivelyPlayableRules);

pub fn compare_farbcards_same_color(card_fst: SCard, card_snd: SCard) -> Ordering {
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
