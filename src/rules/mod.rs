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

use crate::primitives::*;
use std::{
    cmp::Ordering,
    fmt,
};
use crate::util::*;
use crate::ai::rulespecific::*;
use crate::game::SStichSequence;

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

fn all_allowed_cards_within_stich_distinguish_farbe_frei<Result>(
    rules: &(impl TRules + ?Sized),
    stichseq: &SStichSequence,
    hand: &SHand,
    fn_farbe_frei: impl Fn()->Result,
    fn_farbe_not_frei: impl Fn(SHandVector)->Result,
) -> Result {
    assert!(!stichseq.current_stich().is_empty());
    let trumpforfarbe_first = rules.trumpforfarbe(*stichseq.current_stich().first());
    let veccard_same_farbe : SHandVector = hand.cards().iter().cloned()
        .filter(|&card| rules.trumpforfarbe(card)==trumpforfarbe_first)
        .collect();
    if veccard_same_farbe.is_empty() {
        fn_farbe_frei()
    } else {
        fn_farbe_not_frei(veccard_same_farbe)
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum EStockAction {
    Ignore,
    TakeHalf,
    GiveHalf,
}

#[derive(Eq, PartialEq, Clone, Debug, new)]
pub struct SPayoutInfo {
    n_payout: isize,
    estockaction: EStockAction,
}

impl SPayoutInfo {
    pub fn payout_including_stock(&self, n_stock: isize, tpln_stoss_doubling: (usize, usize)) -> isize {
        assert_eq!(n_stock%2, 0);
        assert!(self.estockaction!=EStockAction::TakeHalf || 0<self.n_payout);
        assert!(self.estockaction!=EStockAction::GiveHalf || self.n_payout<0);
        self.n_payout * 2isize.pow((tpln_stoss_doubling.0 + tpln_stoss_doubling.1).as_num::<u32>()) + match self.estockaction {
            EStockAction::Ignore => 0,
            EStockAction::TakeHalf => n_stock/2,
            EStockAction::GiveHalf => -n_stock/2,
        }
    }
}

#[derive(Debug, new, Clone)]
pub struct SPayoutHint {
    tpln_payout: (Option<SPayoutInfo>, Option<SPayoutInfo>),
}

impl SPayoutHint {
    fn contains_payouthint(&self, payouthint_other: &SPayoutHint) -> bool {
        (match (&self.tpln_payout.0, &payouthint_other.tpln_payout.0) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(payoutinfo_self), Some(payoutinfo_other)) => payoutinfo_self.n_payout<=payoutinfo_other.n_payout,
        })
        && match (&self.tpln_payout.1, &payouthint_other.tpln_payout.1) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(payoutinfo_self), Some(payoutinfo_other)) => payoutinfo_self.n_payout>=payoutinfo_other.n_payout,
        }
        // TODO check estockaction
    }

    pub fn lower_bound(&self) -> &Option<SPayoutInfo> {
        &self.tpln_payout.0
    }
}

pub trait TPlayerParties {
    fn is_primary_party(&self, epi: EPlayerIndex) -> bool;
    fn multiplier(&self, epi: EPlayerIndex) -> isize;
}

#[derive(new)]
pub struct SPlayerParties13 {
    epi: EPlayerIndex,
}

impl SPlayerParties13 {
    pub fn primary_player(&self) -> EPlayerIndex {
        self.epi
    }
}

impl TPlayerParties for SPlayerParties13 {
    fn is_primary_party(&self, epi: EPlayerIndex) -> bool {
        self.epi==epi
    }
    fn multiplier(&self, epi: EPlayerIndex) -> isize {
        if self.is_primary_party(epi) {3} else {1}
    }
}

pub trait TRulesNoObj : TRules {
    type TrumpfDecider: trumpfdecider::TTrumpfDecider;
}

pub trait TRules : fmt::Display + TAsRules + Sync + fmt::Debug {
    box_clone_require!(TRules);

    // TTrumpfDecider
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe;
    fn compare_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering;

    fn playerindex(&self) -> Option<EPlayerIndex>;

    fn can_be_played(&self, _hand: SFullHand) -> bool {
        true // probably, only Rufspiel is prevented in some cases
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool;

    fn payout(&self, gamefinishedstiche: SGameFinishedStiche, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> SAccountBalance {
        let apayoutinfo = self.payoutinfos(gamefinishedstiche);
        assert!({
            let count_stockaction = |estockaction| {
                apayoutinfo.iter().filter(|payoutinfo| estockaction==payoutinfo.estockaction).count()
            };
            count_stockaction(EStockAction::TakeHalf)==0 || count_stockaction(EStockAction::GiveHalf)==0
        });
        assert_eq!(n_stock%2, 0);
        // TODO assert tpln_stoss_doubling consistent with stoss_allowed etc
        #[cfg(debug_assertions)] {
            let mut mapepipayouthint = EPlayerIndex::map_from_fn(|_epi| SPayoutHint::new((None, None)));
            let mut stichseq_check = SStichSequence::new(
                gamefinishedstiche.get()[0].first_playerindex(),
                EKurzLang::from_cards_per_player(gamefinishedstiche.get().len()),
            );
            let mut ahand_check = EPlayerIndex::map_from_fn(|epi|
                SHand::new_from_vec(gamefinishedstiche.get().iter().map(|stich| stich[epi]).collect())
            );
            for stich in gamefinishedstiche.get().iter() {
                for (epi, card) in stich.iter() {
                    stichseq_check.zugeben_custom_winner_index(*card, |stich| self.winner_index(stich)); // TODO I could not simply pass rules. Why?
                    ahand_check[epi].play_card(*card);
                    let mapepipayouthint_after = self.payouthints(&stichseq_check, &ahand_check);
                    assert!(
                        mapepipayouthint.iter().zip(mapepipayouthint_after.iter())
                            .all(|(payouthint, payouthint_other)| payouthint.contains_payouthint(payouthint_other)),
                        "{:?}\n{:?}\n{:?}\n{:?}", stichseq_check, ahand_check, mapepipayouthint, mapepipayouthint_after,
                    );
                    mapepipayouthint = mapepipayouthint_after;
                }
                assert!(
                    mapepipayouthint.iter().zip(apayoutinfo.iter().cloned())
                        .all(|(payouthint, payoutinfo)|
                            payouthint.contains_payouthint(&SPayoutHint::new((Some(payoutinfo.clone()), Some(payoutinfo.clone()))))
                        )
                    "{:?}\n{:?}\n{:?}\n{:?}", stichseq_check, ahand_check, mapepipayouthint, apayoutinfo,
                );
            }
        }
        let an_payout = apayoutinfo.map(|payoutinfo| payoutinfo.payout_including_stock(n_stock, tpln_stoss_doubling));
        let n_stock = -an_payout.iter().sum::<isize>();
        SAccountBalance::new(an_payout, n_stock)
    }

    fn payoutinfos(&self, gamefinishedstiche: SGameFinishedStiche) -> EnumMap<EPlayerIndex, SPayoutInfo>;

    fn payouthints(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>) -> EnumMap<EPlayerIndex, SPayoutHint>;

    fn all_allowed_cards(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        assert!(!hand.cards().is_empty());
        assert!(!stichseq.game_finished());
        let veccard = if stichseq.current_stich().is_empty() {
            self.all_allowed_cards_first_in_stich(stichseq, hand)
        } else {
            self.all_allowed_cards_within_stich(stichseq, hand)
        };
        assert!(!veccard.is_empty());
        veccard
    }

    fn all_allowed_cards_first_in_stich(&self, _stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        // probably in most cases, every card can be played
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        // probably in most cases, only the first card of the current stich is decisive
        all_allowed_cards_within_stich_distinguish_farbe_frei(
            self,
            stichseq,
            hand,
            /*fn_farbe_frei*/|| hand.cards().clone(),
            /*fn_farbe_not_frei*/|veccard_same_farbe| veccard_same_farbe
        )
    }

    fn card_is_allowed(&self, stichseq: &SStichSequence, hand: &SHand, card: SCard) -> bool {
        self.all_allowed_cards(stichseq, hand).contains(&card)
    }

    fn winner_index(&self, stich: &SStich) -> EPlayerIndex {
        assert!(stich.is_full());
        self.preliminary_winner_index(stich)
    }

    fn preliminary_winner_index(&self, stich: &SStich) -> EPlayerIndex {
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
