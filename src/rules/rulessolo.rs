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
    pub str_name: String,
    pub epi : EPlayerIndex,
    pub trumpfdecider : PhantomData<TrumpfDecider>,
    payoutdecider: PayoutDecider,
}

impl<TrumpfDecider, PayoutDecider> fmt::Display for SRulesSoloLike<TrumpfDecider, PayoutDecider> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDecider,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.str_name, self.payoutdecider)
    }
}

impl<TrumpfDecider, PayoutDecider> TActivelyPlayableRules for SRulesSoloLike<TrumpfDecider, PayoutDecider>
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDecider,
{
    box_clone_impl_by_clone!(TActivelyPlayableRules);
    fn priority(&self) -> VGameAnnouncementPriority {
        self.payoutdecider.priority()
    }
    fn with_increased_prio(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Box<TActivelyPlayableRules>> {
        self.payoutdecider.with_increased_prio(prio, ebid)
            .map(|payoutdecider| Box::new(Self::internal_new(self.epi, &self.str_name, payoutdecider)) as Box<TActivelyPlayableRules>)
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
                .all(|(i_stoss, stoss)| (i_stoss%2==0) == (stoss.epi!=self.epi))
        );
        EKurzLang::from_cards_per_player(hand.cards().len());
        (epi==self.epi)==(vecstoss.len()%2==1)
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.epi)
    }

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche, n_stoss: usize, n_doubling: usize, _n_stock: isize) -> SAccountBalance {
        SAccountBalance::new(
            SStossDoublingPayoutDecider::payout(
                self.payoutdecider.payout(
                    self,
                    gamefinishedstiche,
                    /*fn_is_player_party*/ |epi| {
                        epi==self.epi
                    },
                    /*fn_player_multiplier*/ |epi| {
                        if self.epi==epi {
                            3
                        } else {
                            1
                        }
                    },
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
}

impl<TrumpfDecider, PayoutDecider> SRulesSoloLike<TrumpfDecider, PayoutDecider>
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDecider,
{
    fn internal_new(epi: EPlayerIndex, str_rulename: &str, payoutdecider: PayoutDecider) -> SRulesSoloLike<TrumpfDecider, PayoutDecider> {
        SRulesSoloLike::<TrumpfDecider, PayoutDecider> {
            epi,
            trumpfdecider: PhantomData::<TrumpfDecider>,
            payoutdecider,
            str_name: str_rulename.to_string(),
        }
    }
    pub fn new(epi: EPlayerIndex, prioparams: PayoutDecider::PrioParams, str_rulename: &str, payoutdeciderparams: SPayoutDeciderParams) -> SRulesSoloLike<TrumpfDecider, PayoutDecider> {
        Self::internal_new(epi, str_rulename, PayoutDecider::new(payoutdeciderparams, prioparams))
    }
}

pub fn sololike<TrumpfDecider, PayoutDecider>(epi: EPlayerIndex, prioparams: PayoutDecider::PrioParams, str_rulename: &str, payoutdeciderparams: SPayoutDeciderParams) -> Box<TActivelyPlayableRules> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDecider,
{
    Box::new(SRulesSoloLike::<TrumpfDecider, PayoutDecider>::new(epi, prioparams, str_rulename, payoutdeciderparams)) as Box<TActivelyPlayableRules>
}

pub type SCoreSolo<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>>;
pub type SCoreGenericWenz<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>;
pub type SCoreGenericGeier<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, TrumpfFarbDecider>;
