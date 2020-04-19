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
pub mod parser;

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
use crate::ai::ahand_vecstich_card_count_is_compatible;
use crate::rules::card_points::points_stich;

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

#[derive(Debug, Clone)]
pub struct SStoss {
    pub epi : EPlayerIndex,
}

fn all_allowed_cards_within_stich_distinguish_farbe_frei (
    rules: &(impl TRules + ?Sized),
    card_first_in_stich: SCard,
    hand: &SHand,
    fn_farbe_not_frei: impl Fn(SHandVector)->SHandVector,
) -> SHandVector {
    let trumpforfarbe_first = rules.trumpforfarbe(card_first_in_stich);
    let veccard_same_farbe : SHandVector = hand.cards().iter().copied()
        .filter(|&card| rules.trumpforfarbe(card)==trumpforfarbe_first)
        .collect();
    if veccard_same_farbe.is_empty() {
        hand.cards().clone()
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
    #[cfg(debug_assertions)]
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

#[derive(Eq, PartialEq, Debug)]
pub struct SRuleStateCacheFixed {
    mapcardoepi: EnumMap<SCard, Option<EPlayerIndex>>, // TODO? Option<EPlayerIndex> is clean for EKurzLang. Does it incur runtime overhead?
}
impl SRuleStateCacheFixed {
    fn new(stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>) -> Self {
        debug_assert!(ahand_vecstich_card_count_is_compatible(stichseq, ahand));
        let mut mapcardoepi = SCard::map_from_fn(|_| None);
        let mut register_card = |card, epi| {
            assert!(mapcardoepi[card].is_none());
            mapcardoepi[card] = Some(epi);
        };
        for (epi, card) in stichseq.visible_stichs().flat_map(SStich::iter) {
            register_card(*card, epi);
        }
        for epi in EPlayerIndex::values() {
            for card in ahand[epi].cards().iter() {
                register_card(*card, epi);
            }
        }
        assert!(EPlayerIndex::values().all(|epi| {
            mapcardoepi.iter().filter_map(|&oepi_card| oepi_card).filter(|epi_card| *epi_card==epi).count()==stichseq.kurzlang().cards_per_player()
        }));
        assert!(SCard::values(stichseq.kurzlang()).all(|card| mapcardoepi[card].is_some()));
        Self {mapcardoepi}
    }
    fn who_has_card(&self, card: SCard) -> EPlayerIndex {
        debug_verify!(self.mapcardoepi[card]).unwrap()
    }
}
#[derive(Eq, PartialEq, Debug)]
pub struct SPointStichCount {
    pub n_stich: usize,
    pub n_point: isize,
}
#[derive(Eq, PartialEq, Debug)]
pub struct SRuleStateCacheChanging {
    pub mapepipointstichcount: EnumMap<EPlayerIndex, SPointStichCount>,
}
#[derive(Eq, PartialEq, Debug)]
pub struct SRuleStateCache { // TODO should we have a cache typer per rules? (Would possibly forbid having TRules trait objects.)
    pub fixed: SRuleStateCacheFixed,
    pub changing: SRuleStateCacheChanging,
}
pub struct SUnregisterStich {
    epi_winner: EPlayerIndex,
    n_points_epi_winner_before: isize,
}

impl SRuleStateCache {
    pub fn new(
        stichseq: &SStichSequence,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        fn_winner_index: impl Fn(&SStich)->EPlayerIndex,
    ) -> Self {
        assert!(ahand_vecstich_card_count_is_compatible(stichseq, ahand));
        stichseq.completed_stichs_custom_winner_index(fn_winner_index).fold(
            Self {
                changing: SRuleStateCacheChanging {
                    mapepipointstichcount: EPlayerIndex::map_from_fn(|_epi| SPointStichCount {
                        n_stich: 0,
                        n_point: 0,
                    }),
                },
                fixed: SRuleStateCacheFixed::new(stichseq, ahand),
            },
            mutate_return!(|rulestatecache, (stich, epi_winner)| {
                rulestatecache.register_stich(stich, epi_winner);
            }),
        )
    }

    fn new_from_gamefinishedstiche(gamefinishedstiche: SStichSequenceGameFinished, fn_winner_index: impl Fn(&SStich)->EPlayerIndex) -> SRuleStateCache {
        Self::new(
            gamefinishedstiche.get(),
            &EPlayerIndex::map_from_fn(|_epi|
                SHand::new_from_vec(SHandVector::new())
            ),
            fn_winner_index,
        )
    }

    pub fn register_stich(&mut self, stich: &SStich, epi_winner: EPlayerIndex) -> SUnregisterStich {
        let unregisterstich = SUnregisterStich {
            epi_winner,
            n_points_epi_winner_before: self.changing.mapepipointstichcount[epi_winner].n_point,
        };
        self.changing.mapepipointstichcount[epi_winner].n_stich += 1;
        self.changing.mapepipointstichcount[epi_winner].n_point += points_stich(stich);
        unregisterstich
    }

    pub fn unregister_stich(&mut self, unregisterstich: SUnregisterStich) {
        self.changing.mapepipointstichcount[unregisterstich.epi_winner].n_point = unregisterstich.n_points_epi_winner_before;
        self.changing.mapepipointstichcount[unregisterstich.epi_winner].n_stich -= 1;
    }
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

pub trait TRules : fmt::Display + TAsRules + Sync + fmt::Debug + TRulesBoxClone {
    // TTrumpfDecider
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe;
    fn compare_cards(&self, card_fst: SCard, card_snd: SCard) -> Option<Ordering>;

    fn playerindex(&self) -> Option<EPlayerIndex>;

    fn can_be_played(&self, _hand: SFullHand) -> bool {
        true // probably, only Rufspiel is prevented in some cases
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool;

    fn payout(&self, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> EnumMap<EPlayerIndex, isize> {
        self.payout_with_cache(
            gamefinishedstiche,
            tpln_stoss_doubling,
            n_stock,
            &SRuleStateCache::new_from_gamefinishedstiche(gamefinishedstiche, |stich| self.winner_index(stich)),
        )
    }

    fn payout_with_cache(&self, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize> {
        let apayoutinfo = self.payoutinfos(
            gamefinishedstiche,
            debug_verify_eq!(
                rulestatecache,
                &SRuleStateCache::new_from_gamefinishedstiche(gamefinishedstiche, |stich| self.winner_index(stich))
            ),
        );
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
                gamefinishedstiche.get().first_playerindex(),
                gamefinishedstiche.get().kurzlang(),
            );
            let mut ahand_check = EPlayerIndex::map_from_fn(|epi|
                SHand::new_from_vec(gamefinishedstiche.get().completed_stichs().iter().map(|stich| stich[epi]).collect())
            );
            for stich in gamefinishedstiche.get().completed_stichs().iter() {
                for (epi, card) in stich.iter() {
                    stichseq_check.zugeben_custom_winner_index(*card, |stich| self.winner_index(stich)); // TODO I could not simply pass rules. Why?
                    ahand_check[epi].play_card(*card);
                    let mapepipayouthint_after = self.payouthints(
                        &stichseq_check,
                        &ahand_check,
                        &SRuleStateCache::new(
                            &stichseq_check,
                            &ahand_check,
                            |stich| self.winner_index(stich),
                        ),
                    );
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
                            payouthint.contains_payouthint(&SPayoutHint::new((Some(payoutinfo.clone()), Some(payoutinfo))))
                        ),
                    "{:?}\n{:?}\n{:?}\n{:?}", stichseq_check, ahand_check, mapepipayouthint, apayoutinfo,
                );
            }
        }
        apayoutinfo.map(|payoutinfo| payoutinfo.payout_including_stock(n_stock, tpln_stoss_doubling))
    }

    fn payoutinfos(&self, gamefinishedstiche: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutInfo>;

    fn payouthints(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutHint>;

    fn all_allowed_cards(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        assert!(!hand.cards().is_empty());
        #[cfg(debug_assertions)]assert!(!stichseq.game_finished());
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
        assert!(!stichseq.current_stich().is_empty());
        all_allowed_cards_within_stich_distinguish_farbe_frei(
            self,
            /*card_first_in_stich*/ *stichseq.current_stich().first(),
            hand,
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
            if let Some(Ordering::Less) = self.compare_cards(stich[epi_best], *card) {
                epi_best = epi;
            }
        }
        epi_best
    }

    fn sort_cards_first_trumpf_then_farbe(&self, veccard: &mut [SCard]) {
        veccard.sort_unstable_by(|&card_lhs, &card_rhs| {
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

    fn rulespecific_ai<'rules>(&'rules self) -> Option<Box<dyn TRuleSpecificAI + 'rules>> {
        None
    }
}

make_upcastable!(TAsRules, TRules);
make_box_clone!(TRulesBoxClone, TRules);

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
    assert_eq!(RufspielLike, RufspielLike);
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
    assert_eq!(SoloSie, SoloSie);
    assert!(SoloLike(SoloSimple(0))<SoloLike(SoloSimple(1)));
    assert!(SoloTout(0)<SoloTout(1));
}

plain_enum_mod!(modebid, EBid {
    AtLeast,
    Higher,
});

pub trait TActivelyPlayableRules : TRules + TActivelyPlayableRulesBoxClone {
    fn priority(&self) -> VGameAnnouncementPriority;
    fn with_higher_prio_than(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Box<dyn TActivelyPlayableRules>> {
        if match ebid {
            EBid::AtLeast => {*prio<=self.priority()},
            EBid::Higher => {*prio<self.priority()},
        } {
            Some(TActivelyPlayableRulesBoxClone::box_clone(self))
        } else {
            self.with_increased_prio(prio, ebid)
        }
    }
    fn with_increased_prio(&self, _prio: &VGameAnnouncementPriority, _ebid: EBid) -> Option<Box<dyn TActivelyPlayableRules>> {
        None
    }
    fn active_playerindex(&self) -> EPlayerIndex {
        debug_verify!(self.playerindex()).unwrap()
    }
}
make_box_clone!(TActivelyPlayableRulesBoxClone, TActivelyPlayableRules);
