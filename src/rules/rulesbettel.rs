use primitives::*;
use rules::{
    *,
    trumpfdecider::*,
    payoutdecider::{SStossDoublingPayoutDecider, internal_payout},
};
use std::marker::PhantomData;
use util::*;

pub trait TBettelAllAllowedCardsWithinStich : Sync + 'static + Clone + fmt::Debug {
    fn all_allowed_cards_within_stich(rulesbettel: &SRulesBettel<Self>, vecstich: &[SStich], hand: &SHand) -> SHandVector;
}

#[derive(Clone, Debug)]
pub struct SRulesBettel<BettelAllAllowedCardsWithinStich>
    where BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich,
{
    epi : EPlayerIndex,
    i_prio : isize,
    payoutdecider : SPayoutDeciderBettel,
    bettelallallowedcardswithinstich : PhantomData<BettelAllAllowedCardsWithinStich>,
}

impl<BettelAllAllowedCardsWithinStich> SRulesBettel<BettelAllAllowedCardsWithinStich>
    where BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich,
{
    pub fn new(epi: EPlayerIndex, i_prio: isize, n_payout_base: isize) -> SRulesBettel<BettelAllAllowedCardsWithinStich> {
        SRulesBettel{
            epi,
            i_prio,
            payoutdecider: SPayoutDeciderBettel{n_payout_base},
            bettelallallowedcardswithinstich: PhantomData::<BettelAllAllowedCardsWithinStich>,
        }
    }
}

impl<BettelAllAllowedCardsWithinStich> fmt::Display for SRulesBettel<BettelAllAllowedCardsWithinStich>
    where BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bettel von {}", self.epi)
    }
}

impl<BettelAllAllowedCardsWithinStich> TActivelyPlayableRules for SRulesBettel<BettelAllAllowedCardsWithinStich>
    where BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich,
{
    box_clone_impl_by_clone!(TActivelyPlayableRules);
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::PointBased(VGameAnnouncementPriorityPointBased::SoloSimple(self.i_prio))
    }
}

#[derive(Clone, Debug)]
struct SPayoutDeciderBettel { // TODO clean up and use TPayoutDecider
    n_payout_base : isize,
}

impl SPayoutDeciderBettel {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        &self,
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules
    {
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| !fn_is_player_party(rules.winner_index(stich)));
        internal_payout(
            /*n_payout_single_player*/ self.n_payout_base,
            fn_player_multiplier,
            /*ab_winner*/ &EPlayerIndex::map_from_fn(|epi| {
                fn_is_player_party(epi)==b_player_party_wins
            })
        )
    }
}

#[derive(Clone, Debug)]
pub struct SBettelAllAllowedCardsWithinStichNormal {}
#[derive(Clone, Debug)]
pub struct SBettelAllAllowedCardsWithinStichStichzwang {}

impl TBettelAllAllowedCardsWithinStich for SBettelAllAllowedCardsWithinStichNormal {
    fn all_allowed_cards_within_stich(rulesbettel: &SRulesBettel<Self>, vecstich: &[SStich], hand: &SHand) -> SHandVector {
        all_allowed_cards_within_stich_distinguish_farbe_frei(
            rulesbettel,
            vecstich,
            hand,
            /*fn_farbe_frei*/|| hand.cards().clone(),
            /*fn_farbe_not_frei*/|veccard_same_farbe| veccard_same_farbe,
        )
    }
}
impl TBettelAllAllowedCardsWithinStich for SBettelAllAllowedCardsWithinStichStichzwang {
    fn all_allowed_cards_within_stich(rulesbettel: &SRulesBettel<Self>, vecstich: &[SStich], hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        let stich = current_stich(vecstich);
        let card_highest = stich[rulesbettel.winner_index(stich)];
        all_allowed_cards_within_stich_distinguish_farbe_frei(
            rulesbettel,
            vecstich,
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

impl<BettelAllAllowedCardsWithinStich> TRules for SRulesBettel<BettelAllAllowedCardsWithinStich>
    where BettelAllAllowedCardsWithinStich: TBettelAllAllowedCardsWithinStich,
{
    box_clone_impl_by_clone!(TRules);
    impl_rules_trumpf!(STrumpfDeciderNoTrumpf);
    impl_single_play!();

    fn all_allowed_cards_within_stich(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector {
        BettelAllAllowedCardsWithinStich::all_allowed_cards_within_stich(self, vecstich, hand)
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
