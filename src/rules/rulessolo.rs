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
    fn trumpf_or_farbe(&self, card: SCard) -> VTrumpfOrFarbe {
        ActiveSinglePlayCore::trumpf_or_farbe(card)
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_eplayerindex)
    }

    fn payout(&self, vecstich: &Vec<SStich>) -> [isize; 4] {
        assert_eq!(vecstich.len(), 8);
        let b_active_player_wins = self.points_per_player(vecstich, self.m_eplayerindex)>=61;
        let ab_winner = create_playerindexmap(|eplayerindex| {
            (eplayerindex==self.m_eplayerindex) == b_active_player_wins
        });
        let n_laufende = ActiveSinglePlayCore::count_laufende(vecstich, &ab_winner);
        create_playerindexmap(|eplayerindex| {
            (/*n_payout_solo*/ 50
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

macro_rules! generate_sololike {
    ($eplayerindex: ident, $coretype: ty, $rulename: expr) => {
        Box::new(SRulesActiveSinglePlay::<$coretype> {
            m_eplayerindex: $eplayerindex,
            m_core: PhantomData::<$coretype>,
            m_str_name: $rulename.to_string(),
        }) as Box<TRules>
    }
}

macro_rules! generate_sololike_farbe {
    ($eplayerindex: ident, $coretype: ident, $rulename: expr) => {
        vec! [
            generate_sololike!($eplayerindex, $coretype<SFarbeDesignatorEichel>, format!("Eichel-{}", $rulename)),
            generate_sololike!($eplayerindex, $coretype<SFarbeDesignatorGras>, format!("Gras-{}", $rulename)),
            generate_sololike!($eplayerindex, $coretype<SFarbeDesignatorHerz>, format!("Herz-{}", $rulename)),
            generate_sololike!($eplayerindex, $coretype<SFarbeDesignatorSchelln>, format!("Schelln-{}", $rulename)),
        ]
    }
}

pub type SCoreSolo<FarbeDesignator> = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, STrumpfDeciderFarbe<
    FarbeDesignator>>>;

pub fn all_rulessolo(eplayerindex: EPlayerIndex) -> Vec<Box<TRules>> {
    generate_sololike_farbe!(eplayerindex, SCoreSolo, "Solo")
}

pub type SCoreFarbwenz<FarbeDesignator> = STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, STrumpfDeciderFarbe<
    FarbeDesignator>>;

pub fn all_rulesfarbwenz(eplayerindex: EPlayerIndex) -> Vec<Box<TRules>> {
    generate_sololike_farbe!(eplayerindex, SCoreFarbwenz, "Wenz")
}

pub fn all_ruleswenz(eplayerindex: EPlayerIndex) -> Vec<Box<TRules>> {
    vec![generate_sololike!(eplayerindex, STrumpfDeciderSchlag<SSchlagDesignatorUnter,STrumpfDeciderNoTrumpf>, "Wenz")]
}
