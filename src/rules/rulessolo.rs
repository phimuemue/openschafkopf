use primitives::*;
use rules::*;
use rules::trumpfdecider::*;
use rules::payoutdecider::*;
use std::fmt;
use std::cmp::Ordering;
use std::marker::PhantomData;
use util::*;

#[derive(Clone)]
pub struct SRulesSoloLike<TrumpfDecider, PayoutDecider>
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDecider,
{
    pub m_str_name: String,
    pub m_epi : EPlayerIndex, // TODO should be static
    pub m_trumpfdecider : PhantomData<TrumpfDecider>,
    pub m_payoutdecider : PhantomData<PayoutDecider>,
    pub m_prio : VGameAnnouncementPriority,
    m_payoutdeciderparams : SPayoutDeciderParams,
}

impl<TrumpfDecider, PayoutDecider> fmt::Display for SRulesSoloLike<TrumpfDecider, PayoutDecider> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDecider,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.m_str_name)
    }
}

impl<TrumpfDecider, PayoutDecider> TActivelyPlayableRules for SRulesSoloLike<TrumpfDecider, PayoutDecider>
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDecider,
{
    box_clone_impl_by_clone!(TActivelyPlayableRules);
    fn priority(&self) -> VGameAnnouncementPriority {
        self.m_prio.clone()
    }
    fn with_higher_prio_than(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Box<TActivelyPlayableRules>> {
        // TODO? move m_prio into PayoutDecider
        if match ebid {
            EBid::AtLeast => {prio<=&self.m_prio},
            EBid::Higher => {prio<&self.m_prio},
        } {
            Some(TActivelyPlayableRules::box_clone(self))
        } else {
            None
        }
    }
}

impl<TrumpfDecider, PayoutDecider> TRules for SRulesSoloLike<TrumpfDecider, PayoutDecider> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDecider,
{
    box_clone_impl_by_clone!(TRules);
    impl_rules_trumpf!(TrumpfDecider);

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool {
        assert!(
            vecstoss.iter()
                .enumerate()
                .all(|(i_stoss, stoss)| (i_stoss%2==0) == (stoss.m_epi!=self.m_epi))
        );
        assert_eq!(hand.cards().len(), 8);
        (epi==self.m_epi)==(vecstoss.len()%2==1)
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_epi)
    }

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche, n_stoss: usize, n_doubling: usize, _n_stock: isize) -> SAccountBalance {
        SAccountBalance::new(
            SStossDoublingPayoutDecider::payout(
                PayoutDecider::payout(
                    self,
                    gamefinishedstiche,
                    /*fn_is_player_party*/ |epi| {
                        epi==self.m_epi
                    },
                    /*fn_player_multiplier*/ |epi| {
                        if self.m_epi==epi {
                            3
                        } else {
                            1
                        }
                    },
                    &self.m_payoutdeciderparams,
                ),
                n_stoss,
                n_doubling,
            ),
            0,
        )
    }

    fn all_allowed_cards_first_in_stich(&self, _vecstich: &[SStich], hand: &SHand) -> SHandVector {
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        let card_first = *vecstich.last().unwrap().first();
        let veccard_allowed : SHandVector = hand.cards().iter()
            .filter(|&&card| self.trumpforfarbe(card)==self.trumpforfarbe(card_first))
            .cloned()
            .collect();
        if veccard_allowed.is_empty() {
            hand.cards().clone()
        } else {
            veccard_allowed
        }
    }
}

impl<TrumpfDecider, PayoutDecider> SRulesSoloLike<TrumpfDecider, PayoutDecider>
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDecider,
{
    pub fn new(epi: EPlayerIndex, prio: VGameAnnouncementPriority, str_rulename: &str, payoutdeciderparams: SPayoutDeciderParams) -> SRulesSoloLike<TrumpfDecider, PayoutDecider> {
        SRulesSoloLike::<TrumpfDecider, PayoutDecider> {
            m_epi: epi,
            m_trumpfdecider: PhantomData::<TrumpfDecider>,
            m_payoutdecider: PhantomData::<PayoutDecider>,
            m_prio: prio,
            m_str_name: str_rulename.to_string(),
            m_payoutdeciderparams : payoutdeciderparams,
        }
    }
}

pub fn sololike<TrumpfDecider, PayoutDecider>(epi: EPlayerIndex, prio: VGameAnnouncementPriority, str_rulename: &str, payoutdeciderparams: SPayoutDeciderParams) -> Box<TActivelyPlayableRules> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDecider,
{
    Box::new(SRulesSoloLike::<TrumpfDecider, PayoutDecider>::new(epi, prio, str_rulename, payoutdeciderparams)) as Box<TActivelyPlayableRules>
}

pub type SCoreSolo<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>>;
pub type SCoreGenericWenz<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>;
pub type SCoreGenericGeier<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, TrumpfFarbDecider>;
