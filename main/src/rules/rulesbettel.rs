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
    phantom : PhantomData<BettelAllAllowedCardsWithinStich>,
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> SRulesBettel<BettelAllAllowedCardsWithinStich> {
    pub fn new(epi: EPlayerIndex, i_prio: isize, n_payout_base: isize) -> SRulesBettel<BettelAllAllowedCardsWithinStich> {
        SRulesBettel{
            epi,
            i_prio,
            payoutdecider: SPayoutDeciderBettel{n_payout_base},
            phantom: PhantomData,
        }
    }
    fn internal_playerindex(&self) -> EPlayerIndex {
        self.epi
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

impl TPayoutDecider for SPayoutDeciderBettel {
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
        if_dbg_else!({stichseq}{_stichseq}): &SStichSequence,
        _ahand: &EnumMap<EPlayerIndex, SHand>,
        rulestatecache: &SRuleStateCache,
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

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> TRulesNoObj for SRulesBettel<BettelAllAllowedCardsWithinStich> {
    impl_rules_trumpf_noobj!(STrumpfDeciderNoTrumpf<SCompareFarbcardsBettel>);
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> TRules for SRulesBettel<BettelAllAllowedCardsWithinStich> {
    impl_rules_trumpf!();
    impl_single_play!();

    fn all_allowed_cards_within_stich(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        BettelAllAllowedCardsWithinStich::all_allowed_cards_within_stich(self, stichseq, hand)
    }
}

#[derive(Clone, Debug)]
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
