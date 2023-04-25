#[macro_use]
pub mod trumpfdecider;
#[macro_use]
pub mod payoutdecider;
#[macro_use]
pub mod singleplay;
pub mod rulesrufspiel;
// TODORULES implement Hochzeit
pub mod card_points;
pub mod parser;
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
use crate::primitives::*;
use crate::rules::card_points::points_stich;
use crate::util::*;
use std::{
    borrow::Borrow,
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

impl PlainEnum for VTrumpfOrFarbe { // TODO(plain_enum) support enums with payload
    const SIZE : usize = EFarbe::SIZE + 1;
    type EnumMapArray<T> = [T; Self::SIZE];
    unsafe fn from_usize(n: usize) -> Self {
        debug_assert!(n < Self::SIZE);
        if n==0 {
            VTrumpfOrFarbe::Trumpf
        } else {
            VTrumpfOrFarbe::Farbe(EFarbe::from_usize(n-1))
        }
    }
    fn to_usize(self) -> usize {
        match self {
            VTrumpfOrFarbe::Trumpf => 0,
            VTrumpfOrFarbe::Farbe(efarbe) => 1 + efarbe.to_usize(),
        }
    }
}

impl VTrumpfOrFarbe {
    pub fn is_trumpf(&self) -> bool {
        match *self {
            VTrumpfOrFarbe::Trumpf => true,
            VTrumpfOrFarbe::Farbe(_efarbe) => false,
        }
    }
}

pub type SDoublings = SPlayersInRound<bool, SStaticEPI0>; // assume that doublings occur in order

#[derive(Debug, Clone)]
pub struct SStoss {
    pub epi : EPlayerIndex,
}

#[derive(Debug, Clone, new)]
pub struct SExpensifiers {
    pub n_stock: isize,
    pub doublings: SDoublings,
    pub vecstoss: Vec<SStoss>,
}

impl SExpensifiers {
    pub fn new_no_stock_doublings_stoss() -> Self {
        Self::new(
            /*n_stock*/0,
            SDoublings::new_full(
                SStaticEPI0{},
                [false; EPlayerIndex::SIZE],
            ),
            /*vecstoss*/vec![],
        )
    }

    pub fn stoss_doubling_factor(&self) -> isize {
        2isize.pow((
            self.vecstoss.len() +
            self.doublings.iter().filter(|&(_epi, &b_doubling)| b_doubling).count()
        ).as_num::<u32>())
    }
}

fn all_allowed_cards_within_stich_distinguish_farbe_frei (
    rules: &(impl TRules + ?Sized),
    card_first_in_stich: ECard,
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

pub trait TPlayerParties {
    fn is_primary_party(&self, epi: EPlayerIndex) -> bool;
    fn multiplier(&self, epi: EPlayerIndex) -> isize;
    type ItEpiPrimary: Iterator<Item=EPlayerIndex>;
    fn primary_players(&self) -> Self::ItEpiPrimary;
    fn primary_sum(&self, fn_val: impl Fn(EPlayerIndex)->isize) -> isize {
        self.primary_players().map(fn_val).sum()
    }
    fn primary_points_so_far(&self, rulestatecache: &SRuleStateCacheChanging) -> isize {
        self.primary_sum(|epi| rulestatecache.mapepipointstichcount[epi].n_point)
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
    type ItEpiPrimary = std::iter::Once<EPlayerIndex>;
    fn primary_players(&self) -> Self::ItEpiPrimary {
        std::iter::once(self.epi)
    }
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
    mapcardoepi: EnumMap<ECard, Option<EPlayerIndex>>, // TODO? Option<EPlayerIndex> is clean for EKurzLang. Does it incur runtime overhead?
}
impl SRuleStateCacheFixed {
    pub fn new(ahand: &EnumMap<EPlayerIndex, SHand>, stichseq: &SStichSequence) -> Self {
        debug_assert!(ahand_vecstich_card_count_is_compatible(ahand, stichseq));
        let mut mapcardoepi = ECard::map_from_fn(|_| None);
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
        assert!(ECard::values(stichseq.kurzlang()).all(|card| mapcardoepi[card].is_some()));
        Self {mapcardoepi}
    }
    fn who_has_card(&self, card: ECard) -> EPlayerIndex {
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
        (ahand, stichseq): (&EnumMap<EPlayerIndex, SHand>, &SStichSequence),
        winnerindex: &(impl TWinnerIndex + ?Sized),
    ) -> Self {
        assert!(ahand_vecstich_card_count_is_compatible(ahand, stichseq));
        stichseq.completed_stichs_winner_index(winnerindex).fold(
            Self {
                changing: SRuleStateCacheChanging {
                    mapepipointstichcount: EPlayerIndex::map_from_fn(|_epi| SPointStichCount {
                        n_stich: 0,
                        n_point: 0,
                    }),
                },
                fixed: SRuleStateCacheFixed::new(ahand, stichseq),
            },
            mutate_return!(|rulestatecache, (stich, epi_winner)| {
                rulestatecache.register_stich(stich, epi_winner);
            }),
        )
    }

    pub fn new_from_gamefinishedstiche(stichseq: SStichSequenceGameFinished, winnerindex: &(impl TWinnerIndex + ?Sized)) -> SRuleStateCache {
        Self::new(
            (
                &EPlayerIndex::map_from_fn(|_epi|
                    SHand::new_from_vec(SHandVector::new())
                ),
                stichseq.get(),
            ),
            winnerindex,
        )
    }

    pub fn register_stich(&mut self, stich: SFullStich<&SStich>, epi_winner: EPlayerIndex) -> SUnregisterStich {
        let unregisterstich = SUnregisterStich {
            epi_winner,
            n_points_epi_winner_before: self.changing.mapepipointstichcount[epi_winner].n_point,
        };
        self.changing.mapepipointstichcount[epi_winner].n_stich += 1;
        self.changing.mapepipointstichcount[epi_winner].n_point += points_stich(stich.borrow());
        unregisterstich
    }

    pub fn unregister_stich(&mut self, unregisterstich: SUnregisterStich) {
        self.changing.mapepipointstichcount[unregisterstich.epi_winner].n_point = unregisterstich.n_points_epi_winner_before;
        self.changing.mapepipointstichcount[unregisterstich.epi_winner].n_stich -= 1;
    }
}

pub trait TRules : fmt::Display + TAsRules + Sync + fmt::Debug + TRulesBoxClone + Send {
    // TTrumpfDecider
    fn trumpforfarbe(&self, card: ECard) -> VTrumpfOrFarbe;
    fn compare_cards(&self, card_fst: ECard, card_snd: ECard) -> Option<Ordering>;
    fn sort_cards_first_trumpf_then_farbe(&self, slccard: &mut [ECard]);

    fn playerindex(&self) -> Option<EPlayerIndex>;

    fn can_be_played(&self, _hand: SFullHand) -> bool {
        true // probably, only Rufspiel is prevented in some cases
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool;

    fn payout(&self, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers, rulestatecache: &SRuleStateCache, if_dbg_else!({b_test_points_as_payout}{_}): dbg_parameter!(bool)) -> EnumMap<EPlayerIndex, isize> {
        let apayoutinfo = self.payout_no_invariant(
            stichseq,
            expensifiers,
            debug_verify_eq!(
                rulestatecache,
                &SRuleStateCache::new_from_gamefinishedstiche(stichseq, /*winnerindex*/self)
            ),
        );
        // TODO assert expensifiers consistent with stoss_allowed etc
        #[cfg(debug_assertions)] {
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
            let mut mapepiintvlon_payout = EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]));
            let mut stichseq_check = SStichSequence::new(stichseq.get().kurzlang());
            let mut ahand_check = EPlayerIndex::map_from_fn(|epi|
                SHand::new_from_iter(stichseq.get().completed_cards_by(epi))
            );
            for stich in stichseq.get().completed_stichs().iter() {
                for (epi, card) in stich.iter() {
                    stichseq_check.zugeben(*card, self);
                    ahand_check[epi].play_card(*card);
                    let mapepiintvlon_payout_after = self.payouthints(
                        (&ahand_check, &stichseq_check),
                        expensifiers,
                        &SRuleStateCache::new(
                            (&ahand_check, &stichseq_check),
                            /*winnerindex*/self,
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
                        stichseq,
                        expensifiers,
                        rulestatecache,
                        /*b_test_points_as_payout*/false,
                    );
                }
            }
        }
        apayoutinfo
    }

    fn payout_no_invariant(&self, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize>;

    fn payouthints(&self, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>;

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

    fn card_is_allowed(&self, stichseq: &SStichSequence, hand: &SHand, card: ECard) -> bool {
        self.all_allowed_cards(stichseq, hand).contains(&card)
    }

    fn preliminary_winner_index(&self, stich: &SStich) -> EPlayerIndex {
        let mut epi_best = stich.epi_first;
        for (epi, card) in stich.iter().skip(1) {
            if let Some(Ordering::Less) = self.compare_cards(*unwrap!(stich.get(epi_best)), *card) {
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
        Box<dyn Fn(&SStichSequence, (EPlayerIndex, &SHand), f32)->f32 + Sync>,
    )> {
        None
    }

    fn snapshot_cache(&self, _rulestatecachefixed: &SRuleStateCacheFixed) -> Box<dyn TSnapshotCache<SMinMax>>;

    fn heuristic_active_occurence_probability(&self) -> Option<f64> {
        // This estimates the probability of the respective rules being played.
        // It is non-trivial to get reliable estimates on this, so I did the following:
        // Observe how often e.g. a Rufspiel has been played in 100000 games: ~36000.
        // As there are 4 players, we have ~9000 Rufspiel per player.
        // Thus, in 100000 games, ~9000 the cards were considered good enough for a rufspiel.
        // => Assume 9%. (All of this is hand-waving and oversimplifying, but as a heuristic it might still be better than nothing.)
        None
    }
}

impl<Rules: TRules + ?Sized> TWinnerIndex for Rules {
    fn winner_index(&self, stich: SFullStich<&SStich>) -> EPlayerIndex {
        self.preliminary_winner_index(stich.borrow())
    }
}
impl<'rules> TWinnerIndex for &'rules dyn TRules {
    fn winner_index(&self, stich: SFullStich<&SStich>) -> EPlayerIndex {
        self.preliminary_winner_index(stich.borrow())
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
    fn sort_cards(&self, slccard: &mut [ECard]) {
        self.sort_cards_first_trumpf_then_farbe(slccard);
    }
}
impl TCardSorter for Box<dyn TRules> {
    fn sort_cards(&self, slccard: &mut [ECard]) {
        self.as_ref().sort_cards(slccard)
    }
}
impl TCardSorter for &dyn TActivelyPlayableRules {
    fn sort_cards(&self, slccard: &mut [ECard]) {
        self.sort_cards_first_trumpf_then_farbe(slccard);
    }
}
impl TCardSorter for Box<dyn TActivelyPlayableRules> {
    fn sort_cards(&self, slccard: &mut [ECard]) {
        self.as_ref().sort_cards(slccard)
    }
}

fn snapshot_cache_point_based<PlayerParties: TPlayerParties+'static>(playerparties: PlayerParties) -> Box<dyn TSnapshotCache<SMinMax>> {
    snapshot_cache(move |rulestatecache| {
        let mut payload_point_stich_count = 0;
        let point_stich_count = |b_primary| {
            EPlayerIndex::values()
                .filter(|epi| b_primary==playerparties.is_primary_party(*epi))
                .map(|epi| rulestatecache.changing.mapepipointstichcount[epi].clone()) // TODO clone needed?
                .fold(
                    SPointStichCount{n_stich: 0, n_point: 0},
                    SPointStichCount::add,
                )
        };
        let pointstichcount_primary = point_stich_count(true);
        set_bits!(payload_point_stich_count, pointstichcount_primary.n_point, 0);
        set_bits!(payload_point_stich_count, pointstichcount_primary.n_stich, 7);
        // let pointstichcount_secondary = point_stich_count(false); // implicitly clear
        // set_bits!(payload_point_stich_count, pointstichcount_secondary.n_point, 11); // implicitly clear
        // set_bits!(payload_point_stich_count, pointstichcount_secondary.n_stich, 18); // implicitly clear
        payload_point_stich_count
    })
}


fn snap_equiv_base(stichseq: &SStichSequence) -> u64 {
    debug_assert_eq!(stichseq.current_stich().size(), 0);
    let mut snapequiv = 0;
    let setcard_played = {
        let mut setcard_played = 0u64;
        for (_, &card) in stichseq.visible_cards() {
            let mask = 1 << card.to_usize();
            debug_assert_eq!((setcard_played & mask), 0);
            setcard_played |= mask;
        }
        setcard_played
    };
    set_bits!(snapequiv, /*epi_next_stich*/stichseq.current_stich().first_playerindex().to_usize(), 0);
    set_bits!(snapequiv, setcard_played, 2);
    snapequiv
}

fn snapshot_cache(fn_payload: impl Fn(&SRuleStateCache)->u64 + 'static) -> Box<dyn TSnapshotCache<SMinMax>> {
    type SSnapshotEquivalenceClass = u64; // space-saving variant of this:
    // struct SSnapshotEquivalenceClass { // packed into SSnapshotEquivalenceClass TODO? use bitfield crate
    //     epi_next_stich: EPlayerIndex,
    //     setcard_played: EnumMap<ECard, bool>, // TODO enumset
    //     payload: <result of fn_payload>,
    // }
    #[derive(Debug)]
    struct SSnapshotCachePointBased<FnPayload> {
        fn_payload: FnPayload,
        mapsnapequivperminmaxmapepin_payout: HashMap<SSnapshotEquivalenceClass, SMinMax>,
    }
    impl<FnPayload: Fn(&SRuleStateCache)->u64> SSnapshotCachePointBased<FnPayload> {
        fn snap_equiv(&self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache) -> SSnapshotEquivalenceClass {
            let mut snapequiv = snap_equiv_base(stichseq);
            set_bits!(snapequiv, (self.fn_payload)(rulestatecache), 34);
            snapequiv
        }
    }
    impl<FnPayload: Fn(&SRuleStateCache)->u64> TSnapshotCache<SMinMax> for SSnapshotCachePointBased<FnPayload> {
        fn get(&self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache) -> Option<SMinMax> {
            debug_assert_eq!(stichseq.current_stich().size(), 0);
            self.mapsnapequivperminmaxmapepin_payout
                .get(&self.snap_equiv(stichseq, rulestatecache))
                .cloned()
        }
        fn put(&mut self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache, payoutstats: &SMinMax) {
            debug_assert_eq!(stichseq.current_stich().size(), 0);
            self.mapsnapequivperminmaxmapepin_payout
                .insert(
                    self.snap_equiv(stichseq, rulestatecache),
                    payoutstats.clone()
                );
            debug_assert_eq!(self.get(stichseq, rulestatecache).as_ref(), Some(payoutstats));
        }
        fn continue_with_cache(&self, stichseq: &SStichSequence) -> bool {
            stichseq.completed_stichs().len()<=5
        }
    }
    Box::new(
        SSnapshotCachePointBased{
            fn_payload,
            mapsnapequivperminmaxmapepin_payout: Default::default(),
        }
    )
}

#[test]
fn test_snapshotcache() {
    use crate::{
        game::SGame,
        player::{
            TPlayer,
            playerrandom::SPlayerRandom,
        },
        rules::ruleset::{
            SRuleSet,
            allowed_rules,
            VStockOrT,
        },
        util::*,
        ai::{
            gametree::{
                SMinReachablePayout,
                SNoFilter,
                SNoVisualization,
                SSnapshotCacheNone,
            },
            determine_best_card,
        },
    };
    crate::game::run::internal_run_simple_game_loop( // TODO simplify all this, and explicitly iterate over supported rules
        EPlayerIndex::map_from_fn(|_epi| Box::new(SPlayerRandom::new(
            /*fn_check_ask_for_card*/|game_in: &SGame| {
                let internal_test = |game: &SGame| {
                    if game.kurzlang().cards_per_player() - if_dbg_else!({4}{5}) < game.completed_stichs().len() {
                        //let epi = unwrap!(game.current_playable_stich().current_playerindex());
                        macro_rules! fwd{($fn_snapshotcache:expr) => {
                            unwrap!(determine_best_card(
                                &game.stichseq,
                                game.rules.as_ref(),
                                Box::new(std::iter::once(game.ahand.clone())) as Box<_>,
                                /*fn_make_filter*/SNoFilter::factory(),
                                &SMinReachablePayout::new_from_game(game),
                                $fn_snapshotcache,
                                SNoVisualization::factory(),
                                /*fn_inspect*/&|_,_,_,_| {},
                                unwrap!(game.stichseq.current_stich().current_playerindex()),
                                /*fn_payout*/&|_stichseq, _ahand, n_payout| (n_payout, ()),
                            ))
                                .cards_and_ts()
                                .map(|(card, payoutstatsperstrategy)| (
                                    card,
                                    verify_eq!(
                                        &payoutstatsperstrategy.0,
                                        &payoutstatsperstrategy.0
                                    ).clone()
                                ))
                                .collect::<Vec<_>>()
                        }}
                        assert_eq!(
                            fwd!(SSnapshotCacheNone::factory()),
                            fwd!(|rulestatecache| game.rules.snapshot_cache(rulestatecache)),
                        );
                    }
                };
                internal_test(game_in);
                if let Some((rules, _fn_payout_to_points))=game_in.rules.points_as_payout() {
                    internal_test(&game_in.clone().map(
                        /*fn_announcements*/|gameannouncements| gameannouncements,
                        /*fn_determinerules*/|determinerules| determinerules,
                        /*fn_ruleset*/|ruleset| ruleset,
                        /*fn_rules*/|_rules| rules,
                    ));
                }
            },
        )) as Box<dyn TPlayer>),
        /*n_games*/8,
        unwrap!(SRuleSet::from_string(
            r"
            base-price=10
            solo-price=50
            lauf-min=3
            [rufspiel]
            [solo]
            [wenz]
            [farbwenz]
            [geier]
            [farbgeier]
            [bettel]
            [ramsch]
            price=20
            [stoss]
            max=4
            ",
        )),
        /*fn_gamepreparations_to_stockorgame*/|gamepreparations, _aattable| {
            let itstockorgame = EPlayerIndex::values()
                .flat_map(|epi| {
                    allowed_rules(
                        &gamepreparations.ruleset.avecrulegroup[epi],
                        gamepreparations.fullhand(epi),
                    )
                })
                .filter_map(|orules| {
                    orules.map(|rules| {
                        VStockOrT::OrT(
                            SGame::new(
                                gamepreparations.aveccard.clone(),
                                gamepreparations.expensifiers.clone(),
                                gamepreparations.ruleset.ostossparams.clone(),
                                rules.upcast().box_clone(),
                            )
                        )
                    })
                })
                .collect::<Vec<_>>().into_iter(); // TODO how can we avoid this?
            if_dbg_else!(
                {{
                    use rand::seq::IteratorRandom;
                    itstockorgame.choose_multiple(&mut rand::thread_rng(), 1).into_iter()
                }}
                {itstockorgame}
            )
        },
    );
}
