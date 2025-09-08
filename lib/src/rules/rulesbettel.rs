use crate::primitives::*;
use crate::rules::{
    payoutdecider::*, trumpfdecider::*, *,
};
use crate::util::*;
use std::marker::PhantomData;

pub trait TBettelAllAllowedCardsWithinStich : Sync + 'static + Clone + fmt::Debug + Send {
    fn all_allowed_cards_within_stich(rulesbettel: &SRulesBettel<Self>, stichseq: &SStichSequence, hand: &SHand) -> SHandVector;
}

#[derive(Clone, Debug)]
pub struct SRulesBettel<BettelAllAllowedCardsWithinStich> {
    epi : EPlayerIndex,
    i_prio : isize,
    n_payout_base: isize,
    trumpfdecider: STrumpfDecider,
    stossparams: SStossParams,
    phantom : PhantomData<BettelAllAllowedCardsWithinStich>,
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> SRulesBettel<BettelAllAllowedCardsWithinStich> {
    pub fn new(epi: EPlayerIndex, i_prio: isize, n_payout_base: isize, stossparams: SStossParams) -> SRulesBettel<BettelAllAllowedCardsWithinStich> {
        SRulesBettel{
            epi,
            i_prio,
            n_payout_base,
            trumpfdecider: STrumpfDecider::new_with_custom_ace_to_7_ordering(
                /*slcschlag_trumpf*/&[],
                /*oefarbe*/None,
                /*itschlag_no_trumpf*/[
                    ESchlag::Ass,
                    ESchlag::Koenig,
                    ESchlag::Ober,
                    ESchlag::Unter,
                    ESchlag::Zehn,
                    ESchlag::S9,
                    ESchlag::S8,
                    ESchlag::S7,
                ],
            ),
            stossparams,
            phantom: PhantomData,
        }
    }
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> fmt::Display for SRulesBettel<BettelAllAllowedCardsWithinStich> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bettel von {}", self.epi)
    }
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> TActivelyPlayableRules for SRulesBettel<BettelAllAllowedCardsWithinStich> {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloLike(VGameAnnouncementPrioritySoloLike::SoloSimple(self.i_prio))
    }
    fn playerindex(&self) -> EPlayerIndex {
        self.epi
    }
}

#[derive(Clone, Debug)]
pub struct SBettelAllAllowedCardsWithinStichNormal {}
#[derive(Clone, Debug)]
pub struct SBettelAllAllowedCardsWithinStichStichzwang {}

impl TBettelAllAllowedCardsWithinStich for SBettelAllAllowedCardsWithinStichNormal {
    fn all_allowed_cards_within_stich(rulesbettel: &SRulesBettel<Self>, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        assert!(!stichseq.current_stich().is_empty());
        all_allowed_cards_within_stich_distinguish_farbe_frei(
            rulesbettel,
            /*card_first_in_stich*/ *stichseq.current_stich().first(),
            hand,
            /*fn_farbe_not_frei*/|veccard_same_farbe| veccard_same_farbe,
        )
    }
}
impl TBettelAllAllowedCardsWithinStich for SBettelAllAllowedCardsWithinStichStichzwang {
    fn all_allowed_cards_within_stich(rulesbettel: &SRulesBettel<Self>, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        let stich = stichseq.current_stich();
        let card_highest = *unwrap!(stich.get(rulesbettel.preliminary_winner_index(stich)));
        assert!(!stichseq.current_stich().is_empty());
        all_allowed_cards_within_stich_distinguish_farbe_frei(
            rulesbettel,
            /*card_first_in_stich*/ *stichseq.current_stich().first(),
            hand,
            /*fn_farbe_not_frei*/|veccard_same_farbe| {
                let veccard_allowed_higher_than_current_best = veccard_same_farbe.iter().copied()
                    .filter(|card| 
                        match unwrap!(rulesbettel.trumpfdecider.compare_cards(card_highest, *card)) {
                            Ordering::Less => true,
                            Ordering::Equal => panic!("Unexpected comparison result in Bettel"),
                            Ordering::Greater => false,
                        }
                    )
                    .collect::<SHandVector>();
                if veccard_allowed_higher_than_current_best.is_empty() {
                    veccard_same_farbe
                } else {
                    veccard_allowed_higher_than_current_best
                }
            }
        )
    }
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> TRules for SRulesBettel<BettelAllAllowedCardsWithinStich> {
    fn trumpfdecider(&self) -> &STrumpfDecider {
        &self.trumpfdecider
    }
    impl_single_play!();

    fn payout_no_invariant(&self, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize> {
        let playerparties13 = SPlayerParties13::new(self.epi);
        internal_payout(
            /*n_payout_primary_unmultiplied*/ self.n_payout_base.neg_if(!/*b_primary_party_wins*/debug_verify_eq!(
                rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich==0,
                stichseq.get().completed_stichs_winner_index(self)
                    .all(|(_stich, epi_winner)| !playerparties13.is_primary_party(epi_winner))
            )),
            &playerparties13,
        ).map(|n_payout| n_payout * expensifiers.stoss_doubling_factor())
    }

    fn payouthints(&self, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        let playerparties13 = SPlayerParties13::new(self.epi);
        if debug_verify_eq!(
            0 < rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich,
            !tplahandstichseq.1.completed_stichs_winner_index(self)
                .all(|(_stich, epi_winner)| !playerparties13.is_primary_party(epi_winner))
        ) {
            internal_payout(
                /*n_payout_primary_unmultiplied; loss is certain*/-self.n_payout_base * expensifiers.stoss_doubling_factor(),
                &playerparties13,
            )
                .map(|n_payout| SInterval::from_raw([Some(*n_payout), Some(*n_payout)]))
        } else {
            EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
        }
    }

    fn equivalent_when_on_same_hand(&self) -> SCardsPartition {
        use crate::primitives::ECard::*;
        debug_verify_eq!(
            SCardsPartition::new_from_slices(&[
                &[EA, EK, EO, EU, EZ, E9, E8, E7] as &[ECard],
                &[GA, GK, GO, GU, GZ, G9, G8, G7],
                &[HA, HK, HO, HU, HZ, H9, H8, H7],
                &[SA, SK, SO, SU, SZ, S9, S8, S7],
            ]),
            SCardsPartition::new_from_slices(
                &self.trumpfdecider.equivalent_when_on_same_hand().iter()
                    .map(|veccard| veccard as &[ECard]).collect::<Vec<_>>(),
            )
        )
    }

    fn all_allowed_cards_within_stich(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        BettelAllAllowedCardsWithinStich::all_allowed_cards_within_stich(self, stichseq, hand)
    }

    fn snapshot_cache<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded>(&self, _rulestatecachefixed: &SRuleStateCacheFixed) -> Box<dyn TSnapshotCache<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>>>
        where
            MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: PartialEq+fmt::Debug+Clone,
    {
        super::snapshot_cache::<MinMaxStrategiesHK>(|rulestatecache| {
            let mut payload_stich_count = 0;
            for (i_epi, epi) in EPlayerIndex::values()
                .skip(1) // first EPI implicitly clear
                .enumerate()
            {
                set_bits!(
                    payload_stich_count,
                    rulestatecache.changing.mapepipointstichcount[epi].n_stich,
                    i_epi*4
                );
            }
            payload_stich_count
        })
    }
}

#[test]
fn test_equivalent_when_on_same_hand_rulesbettel() {
    SRulesBettel::<SBettelAllAllowedCardsWithinStichNormal>::new(
        EPlayerIndex::EPI0,
        /*i_prio*/0,
        /*n_payout_base*/10,
        SStossParams::new(/*n_stoss_max*/4),
    ).equivalent_when_on_same_hand(); // does test internally
    SRulesBettel::<SBettelAllAllowedCardsWithinStichStichzwang>::new(
        EPlayerIndex::EPI0,
        /*i_prio*/0,
        /*n_payout_base*/10,
        SStossParams::new(/*n_stoss_max*/4),
    ).equivalent_when_on_same_hand(); // does test internally
}
