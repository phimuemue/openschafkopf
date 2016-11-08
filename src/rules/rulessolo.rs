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
    fn stoss_allowed(&self, eplayerindex: EPlayerIndex, vecstoss: &Vec<SStoss>, hand: &SHand) -> bool {
        assert!(
            vecstoss.iter()
                .enumerate()
                .all(|(i_stoss, stoss)| (i_stoss%2==0) == (stoss.m_eplayerindex!=self.m_eplayerindex))
        );
        assert_eq!(hand.cards().len(), 8);
        (eplayerindex==self.m_eplayerindex)==(vecstoss.len()%2==1)
    }

    fn trumpforfarbe(&self, card: SCard) -> VTrumpfOrFarbe {
        TrumpfDecider::trumpforfarbe(card)
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_eplayerindex)
    }

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche) -> [isize; 4] {
        SPayoutDeciderPointBased::<TrumpfDecider>::payout(
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

    fn compare_in_stich_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        TrumpfDecider::compare_trumpfcards_solo(card_fst, card_snd)
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

pub fn sololike<CoreType>(eplayerindex: EPlayerIndex, i_prioindex: isize, str_rulename: &str) -> Box<TActivelyPlayableRules> 
    where CoreType: TTrumpfDecider,
          CoreType: 'static,
          CoreType: Sync,
{
    Box::new(SRulesSoloLike::<CoreType>::new(eplayerindex, i_prioindex, str_rulename)) as Box<TActivelyPlayableRules>
}

macro_rules! generate_sololike_farbe {
    ($eplayerindex: ident, $coretype: ident, $i_prioindex: expr, $rulename: expr) => {
        vec! [
            sololike::<$coretype<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>> ($eplayerindex, $i_prioindex, &format!("Eichel-{}", $rulename)),
            sololike::<$coretype<STrumpfDeciderFarbe<SFarbeDesignatorGras>>>   ($eplayerindex, $i_prioindex, &format!("Gras-{}", $rulename)),
            sololike::<$coretype<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>>   ($eplayerindex, $i_prioindex, &format!("Herz-{}", $rulename)),
            sololike::<$coretype<STrumpfDeciderFarbe<SFarbeDesignatorSchelln>>>($eplayerindex, $i_prioindex, &format!("Schelln-{}", $rulename)),
        ]
    }
}

pub type SCoreSolo<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>>;

pub fn all_rulessolo(eplayerindex: EPlayerIndex, i_prioindex: isize) -> Vec<Box<TActivelyPlayableRules>> {
    generate_sololike_farbe!(eplayerindex, SCoreSolo, i_prioindex, "Solo")
}

macro_rules! generate_sololike_farbe_and_farblos {
    ($coretype: ident, $rulename: expr, $fn_all_farbe: ident, $fn_all_farblos: ident) => {
        pub fn $fn_all_farbe(eplayerindex: EPlayerIndex, i_prioindex: isize) -> Vec<Box<TActivelyPlayableRules>> { 
            generate_sololike_farbe!(eplayerindex, $coretype, i_prioindex, $rulename)
        }
        pub fn $fn_all_farblos(eplayerindex: EPlayerIndex, i_prioindex: isize) -> Vec<Box<TActivelyPlayableRules>> {
            vec![sololike::<$coretype<STrumpfDeciderNoTrumpf>>(eplayerindex, i_prioindex, $rulename)]
        }
    }
}

pub type SCoreGenericWenz<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>;

generate_sololike_farbe_and_farblos!(SCoreGenericWenz, &"Wenz", all_rulesfarbwenz, all_ruleswenz);

pub type SCoreGenericGeier<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, TrumpfFarbDecider>;

generate_sololike_farbe_and_farblos!(SCoreGenericGeier, &"Geier", all_rulesfarbgeier, all_rulesgeier);

