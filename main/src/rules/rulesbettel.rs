use crate::primitives::*;
use crate::rules::{
    payoutdecider::{internal_payout, TPayoutDecider}, trumpfdecider::*, *,
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
    payoutdecider : SPayoutDeciderBettel,
    trumpfdecider: STrumpfDeciderBettel,
    phantom : PhantomData<BettelAllAllowedCardsWithinStich>,
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> SRulesBettel<BettelAllAllowedCardsWithinStich> {
    pub fn new(epi: EPlayerIndex, i_prio: isize, n_payout_base: isize) -> SRulesBettel<BettelAllAllowedCardsWithinStich> {
        SRulesBettel{
            epi,
            i_prio,
            payoutdecider: SPayoutDeciderBettel{n_payout_base},
            trumpfdecider: STrumpfDeciderBettel::default(),
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
}

#[derive(Clone, Debug)]
struct SPayoutDeciderBettel {
    n_payout_base : isize,
}

impl TPayoutDecider<SPlayerParties13> for SPayoutDeciderBettel {
    fn payout<Rules>(
        &self,
        if_dbg_else!({rules}{_rules}): &Rules,
        rulestatecache: &SRuleStateCache,
        if_dbg_else!({gamefinishedstiche}{_gamefinishedstiche}): SStichSequenceGameFinished,
        playerparties13: &SPlayerParties13,
    ) -> EnumMap<EPlayerIndex, isize>
        where Rules: TRules
    {
        internal_payout(
            /*n_payout_single_player*/ self.n_payout_base,
            playerparties13,
            /*b_primary_party_wins*/debug_verify_eq!(
                rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich==0,
                gamefinishedstiche.get().completed_stichs_winner_index(rules)
                    .all(|(_stich, epi_winner)| !playerparties13.is_primary_party(epi_winner))
            )
        )
    }

    fn payouthints<Rules>(
        &self,
        if_dbg_else!({rules}{_rules}): &Rules,
        rulestatecache: &SRuleStateCache,
        if_dbg_else!({stichseq}{_stichseq}): &SStichSequence,
        _ahand: &EnumMap<EPlayerIndex, SHand>,
        playerparties13: &SPlayerParties13,
    ) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>
        where Rules: TRulesNoObj
    {
        if debug_verify_eq!(
            0 < rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich,
            !stichseq.completed_stichs_winner_index(rules)
                .all(|(_stich, epi_winner)| !playerparties13.is_primary_party(epi_winner))
        ) {
            internal_payout(
                /*n_payout_single_player*/ self.n_payout_base,
                playerparties13,
                /*b_primary_party_wins*/ false,
            )
                .map(|n_payout| SInterval::from_raw([Some(*n_payout), Some(*n_payout)]))
        } else {
            EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
        }
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
        let card_highest = stich[rulesbettel.preliminary_winner_index(stich)];
        assert!(!stichseq.current_stich().is_empty());
        all_allowed_cards_within_stich_distinguish_farbe_frei(
            rulesbettel,
            /*card_first_in_stich*/ *stichseq.current_stich().first(),
            hand,
            /*fn_farbe_not_frei*/|veccard_same_farbe| {
                let veccard_allowed_higher_than_current_best = veccard_same_farbe.iter().copied()
                    .filter(|card| 
                        match SCompareFarbcardsBettel::compare_farbcards(card_highest, *card) {
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

type STrumpfDeciderBettel = STrumpfDeciderNoTrumpf<SCompareFarbcardsBettel>;

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> TRulesNoObj for SRulesBettel<BettelAllAllowedCardsWithinStich> {
    impl_rules_trumpf_noobj!(STrumpfDeciderBettel);
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> TRules for SRulesBettel<BettelAllAllowedCardsWithinStich> {
    impl_rules_trumpf!();
    impl_single_play!();

    fn payout_no_invariant(&self, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), _n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize> {
        self.payoutdecider.payout(
            self,
            rulestatecache,
            gamefinishedstiche,
            &SPlayerParties13::new(self.epi),
        ).map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
    }

    fn payouthints(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), _n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        self.payoutdecider.payouthints(
            self,
            rulestatecache,
            stichseq,
            ahand,
            &SPlayerParties13::new(self.epi),
        ).map(|intvlon_payout| intvlon_payout.map(|on_payout|
             on_payout.map(|n_payout| payout_including_stoss_doubling(n_payout, tpln_stoss_doubling)),
        ))
    }

    fn equivalent_when_on_same_hand(&self) -> SCardsPartition {
        use crate::primitives::card_values::*;
        debug_verify_eq!(
            SCardsPartition::new_from_slices(&[
                &[EA, EK, EO, EU, EZ, E9, E8, E7] as &[SCard],
                &[GA, GK, GO, GU, GZ, G9, G8, G7],
                &[HA, HK, HO, HU, HZ, H9, H8, H7],
                &[SA, SK, SO, SU, SZ, S9, S8, S7],
            ]),
            {
                let (mapefarbeveccard, veccard_trumpf) = self.trumpfdecider.equivalent_when_on_same_hand();
                assert!(veccard_trumpf.is_empty());
                SCardsPartition::new_from_slices(
                    &mapefarbeveccard.iter()
                        .map(|veccard| veccard as &[SCard]).collect::<Vec<_>>(),
                )
            }
        )
    }

    fn all_allowed_cards_within_stich(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        BettelAllAllowedCardsWithinStich::all_allowed_cards_within_stich(self, stichseq, hand)
    }
}

#[derive(Clone, Debug, Default)]
pub struct SCompareFarbcardsBettel;
impl TCompareFarbcards for SCompareFarbcardsBettel {
    fn compare_farbcards(card_fst: SCard, card_snd: SCard) -> Ordering {
        assert_eq!(card_fst.farbe(), card_snd.farbe());
        let get_schlag_value = |card: SCard| { match card.schlag() {
            ESchlag::S7 => 0,
            ESchlag::S8 => 1,
            ESchlag::S9 => 2,
            ESchlag::Zehn => 3,
            ESchlag::Unter => 4,
            ESchlag::Ober => 5,
            ESchlag::Koenig => 6,
            ESchlag::Ass => 7,
        } };
        get_schlag_value(card_fst).cmp(&get_schlag_value(card_snd))
    }
}

#[test]
fn test_equivalent_when_on_same_hand_rulesbettel() {
    SRulesBettel::<SBettelAllAllowedCardsWithinStichNormal>{
        epi: EPlayerIndex::EPI0,
        i_prio: 0,
        payoutdecider: SPayoutDeciderBettel{
            n_payout_base: 10,
        },
        phantom: PhantomData,
        trumpfdecider: STrumpfDeciderBettel::default(),
    }.equivalent_when_on_same_hand(); // does test internally
    SRulesBettel::<SBettelAllAllowedCardsWithinStichStichzwang>{
        epi: EPlayerIndex::EPI0,
        i_prio: 0,
        payoutdecider: SPayoutDeciderBettel{
            n_payout_base: 10,
        },
        phantom: PhantomData,
        trumpfdecider: STrumpfDeciderBettel::default(),
    }.equivalent_when_on_same_hand(); // does test internally
}
