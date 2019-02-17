use crate::primitives::*;
use crate::rules::{
    *,
    trumpfdecider::*,
    rulessolo::TPayoutDecider,
    payoutdecider::internal_payout,
};
use std::marker::PhantomData;
use crate::util::*;

pub trait TBettelAllAllowedCardsWithinStich : Sync + 'static + Clone + fmt::Debug {
    fn all_allowed_cards_within_stich(rulesbettel: &SRulesBettel<Self>, stichseq: &SStichSequence, hand: &SHand) -> SHandVector;
}

#[derive(Clone, Debug)]
pub struct SRulesBettel<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> {
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
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> fmt::Display for SRulesBettel<BettelAllAllowedCardsWithinStich> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bettel von {}", self.epi)
    }
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> TActivelyPlayableRules for SRulesBettel<BettelAllAllowedCardsWithinStich> {
    box_clone_impl_by_clone!(TActivelyPlayableRules);
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
        rules: &Rules,
        rulestatecache: &SRuleStateCache,
        gamefinishedstiche: SStichSequenceGameFinished,
        playerparties13: &SPlayerParties13,
        perepi: impl TPerEPI,
    ) -> isize
        where Rules: TRules
    {
        internal_payout(
            /*n_payout_single_player*/ self.n_payout_base,
            playerparties13,
            /*b_primary_party_wins*/debug_verify_eq!(
                rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich==0,
                gamefinishedstiche.get().completed_stichs_winner_index(rules)
                    .all(|(_stich, epi_winner)| !playerparties13.is_primary_party(epi_winner))
            ),
            perepi,
        )
    }

    fn payouthints<Rules>(
        &self,
        rules: &Rules,
        stichseq: &SStichSequence,
        _ahand: &EnumMap<EPlayerIndex, SHand>,
        rulestatecache: &SRuleStateCache,
        playerparties13: &SPlayerParties13,
        perepi: impl TPerEPI,
    ) -> (Option<isize>, Option<isize>)
        where Rules: TRulesNoObj
    {
        if debug_verify_eq!(
            0 < rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich,
            !stichseq.completed_stichs_winner_index(rules)
                .all(|(_stich, epi_winner)| !playerparties13.is_primary_party(epi_winner))
        ) {
            perepi.per_epi_map(
                internal_payout(
                    /*n_payout_single_player*/ self.n_payout_base,
                    playerparties13,
                    /*b_primary_party_wins*/ false,
                    perepi,
                ),
                |_epi, n_payout| (Some(n_payout), Some(n_payout))
            )
        } else {
            perepi.per_epi(|_epi| (None, None))
        }
    }
}

#[derive(Clone, Debug)]
pub struct SBettelAllAllowedCardsWithinStichNormal {}
#[derive(Clone, Debug)]
pub struct SBettelAllAllowedCardsWithinStichStichzwang {}

impl TBettelAllAllowedCardsWithinStich for SBettelAllAllowedCardsWithinStichNormal {
    fn all_allowed_cards_within_stich(rulesbettel: &SRulesBettel<Self>, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        all_allowed_cards_within_stich_distinguish_farbe_frei(
            rulesbettel,
            stichseq,
            hand,
            /*fn_farbe_frei*/|| hand.cards().clone(),
            /*fn_farbe_not_frei*/|veccard_same_farbe| veccard_same_farbe,
        )
    }
}
impl TBettelAllAllowedCardsWithinStich for SBettelAllAllowedCardsWithinStichStichzwang {
    fn all_allowed_cards_within_stich(rulesbettel: &SRulesBettel<Self>, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        let stich = stichseq.current_stich();
        let card_highest = stich[rulesbettel.preliminary_winner_index(stich)];
        all_allowed_cards_within_stich_distinguish_farbe_frei(
            rulesbettel,
            stichseq,
            hand,
            /*fn_farbe_frei*/|| {
                debug_assert!(hand.cards().iter().cloned().all(|card|
                    Ordering::Greater==rulesbettel.compare_in_stich(card_highest, card)
                ));
                hand.cards().clone()
            },
            /*fn_farbe_not_frei*/|veccard_same_farbe| {
                let veccard_allowed_higher_than_current_best = veccard_same_farbe.iter().cloned()
                    .filter(|card| 
                        match rulesbettel.compare_in_stich_same_farbe(card_highest, *card) {
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
    impl_rules_trumpf_noobj!(STrumpfDeciderNoTrumpf);
}

impl<BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich> TRules for SRulesBettel<BettelAllAllowedCardsWithinStich> {
    box_clone_impl_by_clone!(TRules);
    impl_rules_trumpf!();
    impl_single_play!();

    fn all_allowed_cards_within_stich(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        BettelAllAllowedCardsWithinStich::all_allowed_cards_within_stich(self, stichseq, hand)
    }

    fn compare_in_stich_same_farbe(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        assert_eq!(self.trumpforfarbe(card_fst), self.trumpforfarbe(card_snd));
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
