#[macro_use]
pub mod trumpfdecider;
#[macro_use]
pub mod singleplay;
pub mod rulesrufspiel;
// TODORULES implement Hochzeit
pub mod card_points;
pub mod parser;
pub mod payoutdecider;
pub mod rulesbettel;
pub mod ruleset;
pub mod rulesramsch;
pub mod rulessolo;

#[cfg(test)]
pub mod tests;

use crate::ai::ahand_vecstich_card_count_is_compatible;
use crate::ai::rulespecific::*;
use crate::ai::cardspartition::*;
use crate::ai::gametree::{TSnapshotCache, SMinMax};
use crate::game::SStichSequence;
use crate::primitives::*;
use crate::rules::card_points::points_stich;
use crate::util::*;
use std::{
    ops::Add,
    cmp::Ordering, fmt,
    collections::HashMap,
};
use itertools::Itertools;

#[derive(Clone, PartialEq, Eq, Debug)]
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

pub fn payout_including_stoss_doubling(n_payout: isize, tpln_stoss_doubling: (usize, usize)) -> isize {
    n_payout * 2isize.pow((tpln_stoss_doubling.0 + tpln_stoss_doubling.1).as_num::<u32>())
}

#[cfg(debug_assertions)]
fn payouthint_contains(intvlon_payout_lhs: &SInterval<Option<isize>>, intvlon_payout_rhs: &SInterval<Option<isize>>) -> bool {
    (match (&intvlon_payout_lhs[ELoHi::Lo], &intvlon_payout_rhs[ELoHi::Lo]) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(n_payout_self), Some(n_payout_other)) => n_payout_self<=n_payout_other,
    })
    && match (&intvlon_payout_lhs[ELoHi::Hi], &intvlon_payout_rhs[ELoHi::Hi]) {
        (None, _) => true,
        (Some(_), None) => false,
        (Some(n_payout_self), Some(n_payout_other)) => n_payout_self>=n_payout_other,
    }
}

pub trait TPlayerParties {
    fn is_primary_party(&self, epi: EPlayerIndex) -> bool;
    fn multiplier(&self, epi: EPlayerIndex) -> isize;
}

#[derive(Debug)]
pub struct SPlayerPartiesTable { // TODO? use this as canonical representation, and get rid of TPlayerParties and its implementors?
    mapepib_primary: EnumMap<EPlayerIndex, bool>, // TODO? use enumset
}

impl SPlayerPartiesTable {
    pub fn is_primary_party(&self, epi: EPlayerIndex) -> bool {
        self.mapepib_primary[epi]
    }
}

impl<PlayerParties: TPlayerParties> From<PlayerParties> for SPlayerPartiesTable {
    fn from(playerparties: PlayerParties) -> Self {
        Self {
            mapepib_primary: EPlayerIndex::map_from_fn(|epi|
                playerparties.is_primary_party(epi)
            ),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct SRuleStateCacheFixed {
    mapcardoepi: EnumMap<SCard, Option<EPlayerIndex>>, // TODO? Option<EPlayerIndex> is clean for EKurzLang. Does it incur runtime overhead?
}
impl SRuleStateCacheFixed {
    pub fn new(stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>) -> Self {
        debug_assert!(ahand_vecstich_card_count_is_compatible(stichseq, ahand));
        let mut mapcardoepi = SCard::map_from_fn(|_| None);
        let mut register_card = |card, epi| {
            assert!(mapcardoepi[card].is_none());
            mapcardoepi[card] = Some(epi);
        };
        for (epi, card) in stichseq.visible_cards() {
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
        unwrap!(self.mapcardoepi[card])
    }
}
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct SPointStichCount {
    pub n_stich: usize,
    pub n_point: isize,
}

impl Add for SPointStichCount {
    type Output = Self;
    fn add(mut self, rhs: SPointStichCount) -> Self::Output {
        self.n_stich += rhs.n_stich;
        self.n_point += rhs.n_point;
        self
    }
}


#[derive(Eq, PartialEq, Debug)]
pub struct SRuleStateCacheChanging {
    pub mapepipointstichcount: EnumMap<EPlayerIndex, SPointStichCount>,
}
#[derive(Eq, PartialEq, Debug)]
pub struct SRuleStateCache { // TODO should we have a cache type per rules? (Would possibly forbid having TRules trait objects.)
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

    pub fn new_from_gamefinishedstiche(gamefinishedstiche: SStichSequenceGameFinished, fn_winner_index: impl Fn(&SStich)->EPlayerIndex) -> SRuleStateCache {
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

#[derive(new, Debug)]
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
    fn trumpfdecider(&self) -> &Self::TrumpfDecider;
}

pub type SIfDebugBool = if_dbg_else!({bool}{()}); // TODO can we get rid of this?

pub trait TRules : fmt::Display + TAsRules + Sync + fmt::Debug + TRulesBoxClone + Send {
    // TTrumpfDecider
    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe;
    fn compare_cards(&self, card_fst: SCard, card_snd: SCard) -> Option<Ordering>;
    fn sort_cards_first_trumpf_then_farbe(&self, slccard: &mut [SCard]);

    fn playerindex(&self) -> Option<EPlayerIndex>;

    fn can_be_played(&self, _hand: SFullHand) -> bool {
        true // probably, only Rufspiel is prevented in some cases
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool;

    fn payout(&self, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), n_stock: isize, rulestatecache: &SRuleStateCache, if_dbg_else!({b_test_points_as_payout}{_}): SIfDebugBool) -> EnumMap<EPlayerIndex, isize> {
        let apayoutinfo = self.payout_no_invariant(
            gamefinishedstiche,
            tpln_stoss_doubling,
            n_stock,
            debug_verify_eq!(
                rulestatecache,
                &SRuleStateCache::new_from_gamefinishedstiche(gamefinishedstiche, |stich| self.winner_index(stich))
            ),
        );
        // TODO assert tpln_stoss_doubling consistent with stoss_allowed etc
        #[cfg(debug_assertions)] {
            let mut mapepiintvlon_payout = EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]));
            let mut stichseq_check = SStichSequence::new(gamefinishedstiche.get().kurzlang());
            let mut ahand_check = EPlayerIndex::map_from_fn(|epi|
                SHand::new_from_iter(gamefinishedstiche.get().completed_stichs().iter().map(|stich| stich[epi]))
            );
            for stich in gamefinishedstiche.get().completed_stichs().iter() {
                for (epi, card) in stich.iter() {
                    stichseq_check.zugeben_custom_winner_index(*card, |stich| self.winner_index(stich)); // TODO I could not simply pass rules. Why?
                    ahand_check[epi].play_card(*card);
                    let mapepiintvlon_payout_after = self.payouthints(
                        &stichseq_check,
                        &ahand_check,
                        tpln_stoss_doubling,
                        n_stock,
                        &SRuleStateCache::new(
                            &stichseq_check,
                            &ahand_check,
                            |stich| self.winner_index(stich),
                        ),
                    );
                    assert!(
                        mapepiintvlon_payout.iter().zip_eq(mapepiintvlon_payout_after.iter())
                            .all(|(intvlon_payout, intvlon_payout_other)| payouthint_contains(intvlon_payout, intvlon_payout_other)),
                        "{}\n{:?}\n{:?}\n{:?}", stichseq_check, ahand_check, mapepiintvlon_payout, mapepiintvlon_payout_after,
                    );
                    mapepiintvlon_payout = mapepiintvlon_payout_after;
                }
                assert!(
                    mapepiintvlon_payout.iter().zip_eq(apayoutinfo.iter().cloned())
                        .all(|(intvlon_payout, payoutinfo)|
                            payouthint_contains(intvlon_payout, &ELoHi::map_from_fn(|_lohi| {
                                Some(payoutinfo)
                            }))
                        ),
                    "{}\n{:?}\n{:?}\n{:?}", stichseq_check, ahand_check, mapepiintvlon_payout, apayoutinfo,
                );
            }
            if b_test_points_as_payout {
                if let Some((rules, _fn_payout_to_points)) = self.points_as_payout() {
                    rules.payout(
                        gamefinishedstiche,
                        tpln_stoss_doubling,
                        n_stock,
                        rulestatecache,
                        /*b_test_points_as_payout*/false,
                    );
                }
            }
        }
        apayoutinfo
    }

    fn payout_no_invariant(&self, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize>;

    fn payouthints(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>;

    fn equivalent_when_on_same_hand(&self) -> SCardsPartition;
    fn only_minmax_points_when_on_same_hand(&self, _rulestatecache: &SRuleStateCacheFixed) -> Option<(SCardsPartition, SPlayerPartiesTable)> {
        None
    }

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

    fn rulespecific_ai<'rules>(&'rules self) -> Option<Box<dyn TRuleSpecificAI + 'rules>> {
        None
    }

    fn points_as_payout(&self) -> Option<(
        Box<dyn TRules>,
        Box<dyn Fn(&SStichSequence, &SHand, f32)->f32>,
    )> {
        None
    }

    fn snapshot_cache(&self, _rulestatecachefixed: &SRuleStateCacheFixed) -> Option<Box<dyn TSnapshotCache<SMinMax>>> {
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
#[allow(clippy::eq_op)] // this method tests equality operators
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
        unwrap!(self.playerindex())
    }
}
make_box_clone!(TActivelyPlayableRulesBoxClone, TActivelyPlayableRules);

impl TCardSorter for &dyn TRules {
    fn sort_cards(&self, slccard: &mut [SCard]) {
        self.sort_cards_first_trumpf_then_farbe(slccard);
    }
}
impl TCardSorter for &Box<dyn TRules> {
    fn sort_cards(&self, slccard: &mut [SCard]) {
        self.as_ref().sort_cards(slccard)
    }
}
impl TCardSorter for &dyn TActivelyPlayableRules {
    fn sort_cards(&self, slccard: &mut [SCard]) {
        self.sort_cards_first_trumpf_then_farbe(slccard);
    }
}
impl TCardSorter for Box<dyn TActivelyPlayableRules> {
    fn sort_cards(&self, slccard: &mut [SCard]) {
        self.as_ref().sort_cards(slccard)
    }
}

fn snapshot_cache_point_based<PlayerParties: TPlayerParties+'static>(playerparties: PlayerParties) -> Option<Box<dyn TSnapshotCache<SMinMax>>> {
    type SSnapshotEquivalenceClass = u64; // space-saving variant of this:
    // struct SSnapshotEquivalenceClass { // packed into SSnapshotEquivalenceClass TODO? use bitfield crate
    //     pointstichcount_primary: SPointStichCount,
    //     pointstichcount_secondary: SPointStichCount,
    //     epi_next_stich: EPlayerIndex,
    //     setcard_played: EnumMap<SCard, bool>, // TODO enumset
    // }
    #[derive(Debug)]
    struct SSnapshotCachePointBased<PlayerParties: TPlayerParties> {
        playerparties: PlayerParties,
        mapsnapequivpayoutstats: HashMap<SSnapshotEquivalenceClass, SMinMax>,
    }
    impl<PlayerParties: TPlayerParties> SSnapshotCachePointBased<PlayerParties> {
        fn snap_equiv(&self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache) -> SSnapshotEquivalenceClass {
            debug_assert_eq!(stichseq.current_stich().size(), 0);
            let mut snapequiv = 0;
            macro_rules! set_bits(($bits:expr, $i_shift:expr) => {
                let bits = $bits.as_num::<u64>() << $i_shift;
                debug_assert_eq!(snapequiv & bits, 0); // none of the touched bits are set so far
                snapequiv |= bits;
            });
            let point_stich_count = |b_primary| {
                EPlayerIndex::values()
                    .filter(|epi| b_primary==self.playerparties.is_primary_party(*epi))
                    .map(|epi| rulestatecache.changing.mapepipointstichcount[epi].clone()) // TODO clone needed?
                    .fold(
                        SPointStichCount{n_stich: 0, n_point: 0},
                        SPointStichCount::add,
                    )
            };
            let pointstichcount_primary = point_stich_count(true);
            set_bits!(pointstichcount_primary.n_point, 0);
            set_bits!(pointstichcount_primary.n_stich, 7);
            // let pointstichcount_secondary = point_stich_count(false); // implicitly clear
            // set_bits!(pointstichcount_secondary.n_point, 11); // implicitly clear
            // set_bits!(pointstichcount_secondary.n_stich, 18); // implicitly clear
            set_bits!(/*epi_next_stich*/stichseq.current_stich().first_playerindex().to_usize(), 22);
            let setcard_played = {
                let mut setcard_played = 0;
                for (_, &card) in stichseq.visible_stichs().iter().flat_map(SStich::iter) {
                    let mask = 1 << card.to_usize();
                    debug_assert_eq!((setcard_played & mask), 0);
                    setcard_played |= mask;
                }
                setcard_played
            };
            set_bits!(setcard_played, 24);
            snapequiv
        }
    }
    impl<PlayerParties: TPlayerParties> TSnapshotCache<SMinMax> for SSnapshotCachePointBased<PlayerParties> {
        fn get(&self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache) -> Option<SMinMax> {
            debug_assert_eq!(stichseq.current_stich().size(), 0);
            self.mapsnapequivpayoutstats
                .get(&self.snap_equiv(stichseq, rulestatecache))
                .cloned()
        }
        fn put(&mut self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache, payoutstats: &SMinMax) {
            debug_assert_eq!(stichseq.current_stich().size(), 0);
            self.mapsnapequivpayoutstats
                .insert(
                    self.snap_equiv(stichseq, rulestatecache),
                    payoutstats.clone()
                );
            debug_assert!(self.get(stichseq, rulestatecache).is_some());
        }
        fn continue_with_cache(&self, stichseq: &SStichSequence) -> bool {
            stichseq.completed_stichs().len()<=5
        }
    }
    Some(Box::new(
        SSnapshotCachePointBased{
            playerparties,
            mapsnapequivpayoutstats: Default::default(),
        }
    ))
}

