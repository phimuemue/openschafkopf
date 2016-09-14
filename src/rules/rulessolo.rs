use primitives::*;
use rules::*;
use rules::trumpfdecider::*;
use std::fmt;
use std::cmp::Ordering;
use std::marker::PhantomData;

pub struct SRulesActiveSinglePlay<ActiveSinglePlayCore> 
    where ActiveSinglePlayCore: TTrumpfDecider,
{
    pub m_str_name: String,
    pub m_eplayerindex : EPlayerIndex, // TODO should be static
    pub m_core : PhantomData<ActiveSinglePlayCore>,
}

impl<ActiveSinglePlayCore> fmt::Display for SRulesActiveSinglePlay<ActiveSinglePlayCore> 
    where ActiveSinglePlayCore: TTrumpfDecider,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.m_str_name)
    }
}

impl<ActiveSinglePlayCore> TRules for SRulesActiveSinglePlay<ActiveSinglePlayCore> 
    where ActiveSinglePlayCore: TTrumpfDecider,
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

    fn trumpf_or_farbe(&self, card: SCard) -> VTrumpfOrFarbe {
        ActiveSinglePlayCore::trumpf_or_farbe(card)
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_eplayerindex)
    }

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche) -> [isize; 4] {
        let (eschneiderschwarz, ab_winner) = points_to_schneiderschwarz_and_winners(
            gamefinishedstiche.get(),
            self,
            /*fn_is_player_party*/ |eplayerindex| {
                eplayerindex==self.m_eplayerindex
            },
        );
        let n_laufende = ActiveSinglePlayCore::count_laufende(gamefinishedstiche.get(), &ab_winner);
        create_playerindexmap(|eplayerindex| {
            (/*n_payout_solo*/ 50
             + { match eschneiderschwarz {
                 ESchneiderSchwarz::Nothing => 0,
                 ESchneiderSchwarz::Schneider => 10,
                 ESchneiderSchwarz::Schwarz => 20,
             }}
             + {if n_laufende<3 {0} else {n_laufende}} * 10
            ) * {
                if ab_winner[eplayerindex] {
                    1
                } else {
                    -1
                }
            } * {
                if self.m_eplayerindex==eplayerindex {
                    3
                } else {
                    1
                }
            }
        } )
    }

    fn all_allowed_cards_first_in_stich(&self, _vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        let card_first = vecstich.last().unwrap().first_card();
        let veccard_allowed : SHandVector = hand.cards().iter()
            .filter(|&&card| self.trumpf_or_farbe(card)==self.trumpf_or_farbe(card_first))
            .cloned()
            .collect();
        if veccard_allowed.is_empty() {
            hand.cards().clone()
        } else {
            veccard_allowed
        }
    }

    fn compare_in_stich_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        ActiveSinglePlayCore::compare_trumpfcards_solo(card_fst, card_snd)
    }
}

pub fn sololike<CoreType>(eplayerindex: EPlayerIndex, str_rulename: &str) -> Box<TRules> 
    where CoreType: TTrumpfDecider,
          CoreType: 'static,
{
    Box::new(SRulesActiveSinglePlay::<CoreType> {
        m_eplayerindex: eplayerindex,
        m_core: PhantomData::<CoreType>,
        m_str_name: str_rulename.to_string(),
    }) as Box<TRules>
}

macro_rules! generate_sololike_farbe {
    ($eplayerindex: ident, $coretype: ident, $rulename: expr) => {
        vec! [
            sololike::<$coretype<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>> ($eplayerindex, &format!("Eichel-{}", $rulename)),
            sololike::<$coretype<STrumpfDeciderFarbe<SFarbeDesignatorGras>>>   ($eplayerindex, &format!("Gras-{}", $rulename)),
            sololike::<$coretype<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>>   ($eplayerindex, &format!("Herz-{}", $rulename)),
            sololike::<$coretype<STrumpfDeciderFarbe<SFarbeDesignatorSchelln>>>($eplayerindex, &format!("Schelln-{}", $rulename)),
        ]
    }
}

pub type SCoreSolo<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>>;

pub fn all_rulessolo(eplayerindex: EPlayerIndex) -> Vec<Box<TRules>> {
    generate_sololike_farbe!(eplayerindex, SCoreSolo, "Solo")
}

macro_rules! generate_sololike_farbe_and_farblos {
    ($coretype: ident, $rulename: expr, $fn_all_farbe: ident, $fn_all_farblos: ident) => {
        pub fn $fn_all_farbe(eplayerindex: EPlayerIndex) -> Vec<Box<TRules>> { 
            generate_sololike_farbe!(eplayerindex, $coretype, $rulename)
        }
        pub fn $fn_all_farblos(eplayerindex: EPlayerIndex) -> Vec<Box<TRules>> {
            vec![sololike::<$coretype<STrumpfDeciderNoTrumpf>>(eplayerindex, $rulename)]
        }
    }
}

pub type SCoreGenericWenz<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>;

generate_sololike_farbe_and_farblos!(SCoreGenericWenz, &"Wenz", all_rulesfarbwenz, all_ruleswenz);

pub type SCoreGenericGeier<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, TrumpfFarbDecider>;

generate_sololike_farbe_and_farblos!(SCoreGenericGeier, &"Geier", all_rulesfarbgeier, all_rulesgeier);

