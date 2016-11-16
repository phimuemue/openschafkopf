use primitives::*;
use rules::*;
use rules::trumpfdecider::*;
use rules::payoutdecider::*;
use std::fmt;
use std::cmp::Ordering;
use std::marker::PhantomData;

pub struct SRulesSoloLike<TrumpfDecider> 
    where TrumpfDecider: TTrumpfDecider,
{
    pub m_str_name: String,
    pub m_eplayerindex : EPlayerIndex, // TODO should be static
    pub m_trumpfdecider : PhantomData<TrumpfDecider>,
    pub m_i_prioindex : isize,
}

impl<TrumpfDecider> fmt::Display for SRulesSoloLike<TrumpfDecider> 
    where TrumpfDecider: TTrumpfDecider,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.m_str_name)
    }
}

impl<TrumpfDecider> TActivelyPlayableRules for SRulesSoloLike<TrumpfDecider>
    where TrumpfDecider: TTrumpfDecider,
          TrumpfDecider: Sync,
{
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SinglePlayLike(self.m_i_prioindex)
    }
}

impl<TrumpfDecider> TRules for SRulesSoloLike<TrumpfDecider> 
    where TrumpfDecider: TTrumpfDecider,
          TrumpfDecider: Sync,
{
    impl_rules_trumpf!(TrumpfDecider);

    fn stoss_allowed(&self, eplayerindex: EPlayerIndex, vecstoss: &Vec<SStoss>, hand: &SHand) -> bool {
        assert!(
            vecstoss.iter()
                .enumerate()
                .all(|(i_stoss, stoss)| (i_stoss%2==0) == (stoss.m_eplayerindex!=self.m_eplayerindex))
        );
        assert_eq!(hand.cards().len(), 8);
        (eplayerindex==self.m_eplayerindex)==(vecstoss.len()%2==1)
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_eplayerindex)
    }

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche) -> [isize; 4] {
        SPayoutDeciderPointBased::payout(
            self,
            gamefinishedstiche,
            /*fn_is_player_party*/ |eplayerindex| {
                eplayerindex==self.m_eplayerindex
            },
            /*fn_player_multiplier*/ |eplayerindex| {
                if self.m_eplayerindex==eplayerindex {
                    3
                } else {
                    1
                }
            },
            /*n_payout_base*/50,
        )
    }

    fn all_allowed_cards_first_in_stich(&self, _vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
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

impl<TrumpfDecider> SRulesSoloLike<TrumpfDecider>
    where TrumpfDecider: TTrumpfDecider,
{
    pub fn new(eplayerindex: EPlayerIndex, i_prioindex: isize, str_rulename: &str) -> SRulesSoloLike<TrumpfDecider> {
        SRulesSoloLike::<TrumpfDecider> {
            m_eplayerindex: eplayerindex,
            m_trumpfdecider: PhantomData::<TrumpfDecider>,
            m_i_prioindex: i_prioindex,
            m_str_name: str_rulename.to_string(),
        }
    }
}

pub fn sololike<TrumpfDecider>(eplayerindex: EPlayerIndex, i_prioindex: isize, str_rulename: &str) -> Box<TActivelyPlayableRules> 
    where TrumpfDecider: TTrumpfDecider,
          TrumpfDecider: 'static,
          TrumpfDecider: Sync,
{
    Box::new(SRulesSoloLike::<TrumpfDecider>::new(eplayerindex, i_prioindex, str_rulename)) as Box<TActivelyPlayableRules>
}

pub type SCoreSolo<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>>;
pub type SCoreGenericWenz<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>;
pub type SCoreGenericGeier<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, TrumpfFarbDecider>;
