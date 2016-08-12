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
        let (eschneiderschwarz, ab_winner) = points_to_schneiderschwarz_and_winners(
            vecstich,
            self,
            /*fn_is_player_party*/ |eplayerindex| {
                eplayerindex==self.m_eplayerindex
            },
        );
        let n_laufende = ActiveSinglePlayCore::count_laufende(vecstich, &ab_winner);
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

macro_rules! generate_sololike {
    ($eplayerindex: expr, $coretype: ty, $rulename: expr) => {
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
            generate_sololike!($eplayerindex, $coretype<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, format!("Eichel-{}", $rulename)),
            generate_sololike!($eplayerindex, $coretype<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, format!("Gras-{}", $rulename)),
            generate_sololike!($eplayerindex, $coretype<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, format!("Herz-{}", $rulename)),
            generate_sololike!($eplayerindex, $coretype<STrumpfDeciderFarbe<SFarbeDesignatorSchelln>>, format!("Schelln-{}", $rulename)),
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
            vec![generate_sololike!(eplayerindex, $coretype<STrumpfDeciderNoTrumpf>, $rulename)]
        }
    }
}

pub type SCoreFarbwenz<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, TrumpfFarbDecider>;

generate_sololike_farbe_and_farblos!(SCoreFarbwenz, "Wenz", all_rulesfarbwenz, all_ruleswenz);

#[test]
fn test_rulessolo() {
    use rules::test_rules::*;
    test_rules(
        "../../testdata/games/solo/1.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["gu e8 gz g8 g7 ha sz s8","hu g9 hz h9 h8 sa s9 s7","eo su ea ek e7 gk hk h7","go ho so eu ez e9 ga sk",],
        [(0, "ha h9 hk ez"),(3, "eu e8 hu e7"),(3, "so gu hz eo"),(2, "gk ga g8 g9"),(3, "ho g7 h8 ek"),(3, "go s8 s7 su"),(3, "sk sz sa h7"),(1, "s9 ea e9 gz"),],
        [50, 50, 50, -150], // TODO support Kontra
    );
    test_rules(
        "../../testdata/games/solo/2.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["ho so su ea ek e8 ha hz","eo go eu e9 gk g7 hk sz","g9 g8 h9 h8 h7 sa sk s8","gu hu ez e7 ga gz s9 s7",],
        [(0, "so go sa ez"),(1, "hk h7 hu ha"),(3, "ga ea g7 g8"),(0, "su eu sk e7"),(1, "gk g9 gz ek"),(0, "e8 e9 s8 gu"),(3, "s9 ho sz h8"),(0, "hz eo h9 s7"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/3.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["go ho so hu ga g8 ha s9","gz g9 g7 ea ez h9 h8 s7","eo eu gu e8 e7 hz hk s8","su gk ek e9 h7 sa sz sk",],
        [(0, "so gz eo gk"),(2, "s8 sz s9 s7"),(3, "sa hu g7 gu"),(2, "e7 e9 g8 ez"),(0, "ho g9 eu su"),(0, "go h8 e8 h7"),(0, "ga ea hk sk"),(0, "ha h9 hz ek"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/4.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["eo ho so eu gu hk e7 g8","go h8 e8 gz gk g9 g7 sz","hu ha h9 ea ek ga s9 s8","su hz h7 ez e9 sa sk s7",],
        [(0, "gu go ha hz"),(1, "e8 ea ez e7"),(2, "ek e9 g8 sz"),(2, "ga su eu g7"),(0, "so h8 h9 h7"),(0, "ho g9 hu s7"),(0, "eo gk s9 sk"),(0, "hk gz s8 sa"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/5.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo go ho so g9 g7 ea s7","hu su ga ez e9 e7 hz h7","eu gk g8 ek ha h8 sa s9","gu gz e8 hk h9 sz sk s8",],
        [(0, "eo su g8 gu"),(0, "go hu gk gz"),(0, "so ga eu e8"),(0, "s7 ez sa sz"),(2, "ek hk ea e7"),(0, "ho e9 s9 h9"),(0, "g7 hz h8 s8"),(0, "g9 h7 ha sk"),],
        [270, -90, -90, -90],
    );
    test_rules(
        "../../testdata/games/solo/6.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["go hu e8 gz hz h9 sz s7","ho gk g8 g7 ha h8 sa s9","so eu gu ea ez ek ga s8","eo su e9 e7 g9 hk h7 sk",],
        [(0, "gz gk ga g9"),(2, "gu e7 e8 ho"),(1, "g8 s8 sk sz"),(1, "g7 ek su hz"),(3, "h7 h9 h8 ea"),(2, "eu e9 hu s9"),(2, "so eo go sa"),(3, "hk s7 ha ez"),],
        [-80, -80, 240, -80],
    );
    test_rules(
        "../../testdata/games/solo/7.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["so eu g9 ez e7 hz hk sa","eo go gu hu gz g7 ea e8","su ga gk ek h9 h8 sk s9","ho g8 e9 ha h7 sz s8 s7",],
        [(0, "sa hu s9 s7"),(1, "go ga g8 g9"),(1, "eo su ho eu"),(1, "g7 gk sz so"),(0, "hz gz h8 h7"),(1, "gu h9 s8 hk"),(1, "ea ek e9 e7"),(1, "e8 sk ha ez"),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/8.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["go ho su ha hk sz s9 s7","so eu g7 ez e7 sa sk s8","eo gu ga gz gk g9 g8 h9","hu ea ek e9 e8 hz h8 h7",],
        [(0, "ha so h9 hz"),(1, "sa gk hu sz"),(3, "h7 hk eu g8"),(1, "sk gu h8 s7"),(2, "eo e8 su g7"),(2, "g9 ea ho ez"),(0, "go e7 gz ek"),(0, "s9 s8 ga e9"),],
        [60, 60, -180, 60],
    );
    test_rules(
        "../../testdata/games/solo/9.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo ho so gu hu gk g7 h9","eu gz ea hz hk sa sz sk","ga g8 e9 e7 ha h8 h7 s9","go su g9 ez ek e8 s8 s7",],
        [(0, "eo eu g8 g9"),(0, "hu gz ga go"),(3, "ez gu ea e9"),(0, "so sa s9 su"),(0, "ho sk e7 s8"),(0, "h9 hz ha ek"),(2, "h8 e8 gk hk"),(0, "g7 sz h7 s7"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/10.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["go hu ez e7 h8 sz sk s7","eu gz g7 e9 ha h9 sa s8","eo so gu su ga gk g9 e8","ho g8 ea ek hz hk h7 s9",],
        [(0, "sk sa ga s9"),(2, "so ho go gz"),(0, "sz s8 eo g8"),(2, "g9 hz hu g7"),(0, "s7 e9 e8 ea"),(0, "ez eu su ek"),(1, "ha gk hk h8"),(2, "gu h7 e7 h9"),],
        [50, 50, -150, 50],
    );
    test_rules(
        "../../testdata/games/solo/11.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eu gz ea ek e7 sa s9 s8","eo go ho so gu gk e8 hk","hu su g9 g7 ha hz h9 sk","ga g8 ez e9 h8 h7 sz s7",],
        [(0, "sa gk sk s7"),(1, "go g7 ga eu"),(1, "eo g9 g8 gz"),(1, "gu su sz s8"),(1, "e8 ha ez ea"),(0, "ek hk hz e9"),(0, "e7 ho h9 h8"),(1, "so hu h7 s9"),],
        [-90, 270, -90, -90],
    );
    test_rules(
        "../../testdata/games/solo/12.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["go h8 ea ek e8 gk sk s8","eo eu su ha hz hk h9 sa","so h7 ez e7 ga gz g9 sz","ho gu hu e9 g8 g7 s9 s7",],
        [(0, "gk hz g9 g8"),(1, "h9 h7 hu h8"),(3, "g7 go eo gz"),(1, "su so ho ea"),(3, "e9 ek hk e7"),(1, "eu ez gu e8"),(1, "ha ga s7 s8"),(1, "sa sz s9 sk"),],
        [-60, 180, -60, -60],
    );
    test_rules(
        "../../testdata/games/solo/13.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["ho so eu hu su e7 ga ha","gu e9 g8 g7 hz sz sk s8","ez e8 gz gk hk h8 h7 s7","eo go ea ek g9 h9 sa s9",],
        [(0, "eu e9 ez go"),(3, "sa su s8 s7"),(0, "so gu e8 ek"),(0, "hu hz gz eo"),(3, "g9 ga g7 gk"),(0, "ho g8 h7 ea"),(0, "ha sk h8 h9"),(0, "e7 sz hk s9"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/14.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["go g8 ea e9 e8 e7 ha s8","so g9 hk h9 h7 sk s9 s7","eo ho gu su gz gk g7 hz","eu hu ga ez ek h8 sa sz",],
        [(0, "ea so ho ek"),(2, "eo hu g8 g9"),(2, "su ga go hk"),(0, "e9 h7 hz ez"),(3, "sa s8 s7 gz"),(2, "gu eu ha sk"),(3, "h8 e7 h9 g7"),(2, "gk sz e8 s9"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/15.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["ho so su ea ez e8 e7 s8","ek g9 ha h8 h7 sa sz sk","eo go eu gu hu g7 h9 s7","e9 ga gz gk g8 hz hk s9",],
        [(0, "e7 ek hu e9"),(2, "g7 ga ea g9"),(0, "e8 sa gu gz"),(2, "eo hz su sz"),(2, "go hk so ha"),(2, "s7 s9 s8 sk"),(1, "h7 h9 g8 ez"),(0, "ho h8 eu gk"),],
        [-150, 50, 50, 50], // TODO support Kontra
    );
    test_rules(
        "../../testdata/games/solo/16.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["ho so hu su ga g9 ea ha","go gz gk ez e7 sa s9 s8","eu gu g8 g7 e9 e8 hz h7","eo ek hk h9 h8 sz sk s7",],
        [(0, "so gz g7 eo"),(3, "hk ha gk hz"),(1, "sa h7 sk ga"),(0, "su go g8 sz"),(1, "s9 e8 s7 g9"),(0, "ho s8 gu h8"),(0, "ea e7 e9 ek"),(0, "hu ez eu h9"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/17.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo ho eu ea e9 e8 g9 sa","gu hu su e7 ga gz sk s8","ez ek gk ha h8 h7 sz s9","go so g8 g7 hz hk h9 s7",],
        [(0, "ho e7 ez go"),(3, "g8 g9 ga gk"),(1, "s8 s9 s7 sa"),(0, "eo su ek so"),(0, "eu hu h7 g7"),(0, "e9 gu sz hz"),(1, "sk h8 h9 e8"),(0, "ea gz ha hk"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/18.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["eu hu ha ez e8 e7 sz s8","go ek gz gk sa sk s9 s7","ho hz h9 h7 ga g9 g8 g7","eo so gu su hk h8 ea e9",],
        [(0, "e7 ek hz e9"),(2, "ga gu eu gz"),(0, "ez go g7 ea"),(1, "gk g8 su hu"),(0, "s8 s7 g9 hk"),(3, "eo ha s9 h7"),(3, "h8 sz sa h9"),(2, "ho so e8 sk"),],
        [60, 60, 60, -180],
    );
    test_rules(
        "../../testdata/games/solo/19.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo go eu gu ez e9 e8 s7","hu su e7 g8 ha h8 h7 sz","ho so gk hz hk h9 s9 s8","ea ek ga gz g9 g7 sa sk",],
        [(0, "go e7 so ek"),(0, "eo su ho ea"),(0, "gu hu gk g9"),(0, "s7 sz s8 sa"),(3, "ga e9 g8 s9"),(0, "eu h7 h9 g7"),(0, "ez h8 hk gz"),(0, "e8 ha hz sk"),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/solo/20.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["eo go so eu gu h8 ea s9","hk h9 ez e9 gk g7 sa s7","su ha h7 ek g9 g8 sk s8","ho hu hz e8 e7 ga gz sz",],
        [(0, "go hk ha hu"),(0, "eo h9 h7 hz"),(0, "gu ez su ho"),(3, "ga h8 g7 g9"),(0, "eu e9 ek e7"),(0, "s9 s7 sk sz"),(3, "e8 ea gk g8"),(0, "so sa s8 gz"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/21.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["hu su e8 g9 sz sk s9 s8","eo go ho so eu ek e7 h7","gu ea ga gz ha hz hk h9","ez e9 gk g8 g7 h8 sa s7",],
        [(0, "sz eu h9 s7"),(1, "go gu ez e8"),(1, "eo ea e9 su"),(1, "so ha sa hu"),(1, "ho hk h8 g9"),(1, "h7 hz gk sk"),(2, "ga g7 s8 e7"),(1, "ek gz g8 s9"),],
        [-110, 330, -110, -110],
    );
    test_rules(
        "../../testdata/games/solo/22.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo eu ez ga g7 hz hk sa","so gu ea gk g9 h8 sz sk","ek gz ha h9 h7 s9 s8 s7","go ho hu su e9 e8 e7 g8",],
        [(0, "sa sk s7 e8"),(3, "ho eo ea ek"),(0, "ga gk gz g8"),(0, "hk h8 h7 e7"),(3, "go ez gu s8"),(3, "su eu so ha"),(1, "sz s9 e9 g7"),(3, "hu hz g9 h9"),],
        [50, 50, 50, -150],
    );
    test_rules(
        "../../testdata/games/solo/23.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eu gu g8 ea hk h8 sa s9","ga e9 e8 e7 h9 sz sk s8","eo go ho hu su gk g9 ez","so gz g7 ek ha hz h7 s7",],
        [(0, "sa sz su s7"),(2, "eo g7 g8 ga"),(2, "ho gz gu h9"),(2, "go so eu e7"),(2, "g9 ek s9 e8"),(2, "ez ha ea e9"),(0, "hk sk gk h7"),(2, "hu hz h8 s8"),],
        [-80, -80, 240, -80],
    );
    test_rules(
        "../../testdata/games/solo/24.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["eu h8 ez ek e9 ga gk s9","hu hk h7 gz g8 g7 sk s8","eo go su hz ea e7 sa s7","ho so gu ha h9 e8 g9 sz",],
        [(0, "ga g7 hz g9"),(2, "go h9 h8 h7"),(2, "eo gu eu hk"),(2, "ea e8 ek hu"),(1, "sk sa sz s9"),(2, "e7 ha ez gz"),(3, "ho gk g8 su"),(3, "so e9 s8 s7"),],
        [50, 50, -150, 50],
    );
    test_rules(
        "../../testdata/games/solo/25.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["go so gu ez e9 g7 ha sz","ek ga g9 g8 h9 h8 sa s8","e7 gz gk hz hk h7 sk s9","eo ho eu hu su ea e8 s7",],
        [(0, "ha h9 hk s7"),(0, "g7 ga gk ea"),(3, "ho go ek e7"),(0, "sz sa s9 e8"),(3, "eo e9 s8 sk"),(3, "su gu h8 gz"),(0, "so g8 hz hu"),(0, "ez g9 h7 eu"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/26.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["so eu gu su g9 g8 h9 sa","ho e9 hk h8 sz sk s8 s7","eo go hu ea ez ek e7 ga","e8 gz gk g7 ha hz h7 s9",],
        [(0, "sa sz hu s9"),(2, "go e8 su e9"),(2, "eo g7 gu ho"),(2, "e7 ha eu hk"),(0, "h9 h8 ea h7"),(2, "ek hz so s7"),(0, "g9 s8 ga gk"),(2, "ez gz g8 sk"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/27.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo go su e9 e8 g7 ha hz","ea e7 ga gk g9 h8 sk s8","ho so gu ek gz g8 hk sz","eu hu ez h9 h7 sa s9 s7",],
        [(0, "go ea ek hu"),(0, "eo e7 gu eu"),(0, "e9 sk so ez"),(2, "hk h9 ha h8"),(0, "hz gk ho h7"),(2, "sz sa e8 s8"),(0, "g7 ga gz s9"),(1, "g9 g8 s7 su"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/28.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["gu ga gk ea ez e7 sk s7","so g9 g8 e9 ha hz h8 s8","hu su e8 hk h9 h7 sz s9","eo go ho eu gz g7 ek sa",],
        [(0, "sk s8 s9 sa"),(3, "go ga g8 su"),(3, "ho gu g9 hu"),(3, "eo gk so e8"),(3, "g7 ez ha h7"),(3, "ek ea e9 sz"),(0, "s7 h8 h9 gz"),(3, "eu e7 hz hk"),],
        [-90, -90, -90, 270],
    );
    test_rules(
        "../../testdata/games/solo/29.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["su gz gk ez e9 e7 hz sk","so g8 ea e8 ha h7 sa s7","eo hu ek h9 h8 sz s9 s8","go ho eu gu ga g9 g7 hk",],
        [(0, "sk sa s8 ga"),(3, "eu gz so hu"),(1, "ha h9 hk hz"),(1, "h7 h8 g9 gk"),(0, "e7 ea ek g7"),(3, "ho su g8 eo"),(2, "sz gu e9 s7"),(3, "go ez e8 s9"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/30.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["so ek g7 hz hk h7 sk s8","eu hu e7 ga g8 h8 sz s9","ho ea ez gz g9 h9 sa s7","eo go gu su e9 e8 gk ha",],
        [(0, "hk h8 h9 ha"),(3, "go ek e7 ea"),(3, "eo so hu ez"),(3, "e8 hz eu ho"),(2, "sa su s8 s9"),(3, "gk g7 ga gz"),(1, "sz s7 e9 sk"),(3, "gu h7 g8 g9"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/31.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo ez ga hz hk sa s9 s8","go eu e8 e7 g7 ha h9 sz","ho hu su gz gk g9 h8 s7","so gu ea ek e9 g8 h7 sk",],
        [(0, "ga g7 g9 g8"),(0, "hk ha h8 h7"),(1, "h9 su sk hz"),(2, "gz gu eo sz"),(0, "s8 eu s7 so"),(3, "e9 ez e7 hu"),(2, "gk ea sa go"),(1, "e8 ho ek s9"),],
        [90, 90, 90, -270],
    );
    test_rules(
        "../../testdata/games/solo/32.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["e7 g9 g7 hz h8 sa sz s7","go ho hu su ea ez e8 ha","ek ga gz gk h9 sk s9 s8","eo so eu gu e9 g8 hk h7",],
        [(0, "sa su s8 gu"),(3, "g8 g9 ea gk"),(1, "hu ek eu e7"),(3, "h7 h8 ha h9"),(1, "ho ga eo hz"),(3, "hk g7 ez s9"),(1, "go sk e9 s7"),(1, "e8 gz so sz"),],
        [50, -150, 50, 50], // TODO support Kontra
    );
    test_rules(
        "../../testdata/games/solo/33.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo go g9 g7 ea e9 ha h9","eu ga gz e8 e7 hz hk s7","ho so gu hu su h8 sz sk","gk g8 ez ek h7 sa s9 s8",],
        [(0, "go ga su g8"),(0, "eo eu hu gk"),(0, "g7 gz gu ez"),(2, "h8 h7 ha hk"),(0, "ea e7 so ek"),(2, "ho sa g9 hz"),(2, "sz s9 h9 s7"),(2, "sk s8 e9 e8"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/34.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["hz ek e9 e8 ga gz g8 s8","eu h7 ea ez gk g7 sk s7","eo ho so gu hu ha hk sz","go su h9 h8 e7 g9 sa s9",],
        [(0, "ek ez gu e7"),(2, "eo h8 hz h7"),(2, "so h9 s8 eu"),(2, "hu go ga ea"),(3, "sa e8 sk sz"),(3, "s9 e9 s7 ha"),(2, "ho su g8 g7"),(2, "hk g9 gz gk"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/35.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["gu su g7 ea e9 sa sz s8","eo go ho so hu ga g9 e8","g8 ez e7 ha hz hk s9 s7","eu gz gk ek h9 h8 h7 sk",],
        [(0, "ea e8 ez ek"),(0, "e9 hu e7 sk"),(1, "so g8 gz g7"),(1, "ho ha gk su"),(1, "go s7 eu gu"),(1, "eo s9 h9 s8"),(1, "ga hk h7 sz"),(1, "g9 hz h8 sa"),],
        [-100, 300, -100, -100],
    );
    test_rules(
        "../../testdata/games/solo/36.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["go ek e8 gk g8 g7 hz h8","eu hu ea g9 h9 h7 sa s9","su gz ha hk sz sk s8 s7","eo ho so gu ez e9 e7 ga",],
        [(0, "g7 g9 gz ga"),(3, "so ek ea su"),(3, "eo e8 hu s7"),(3, "gu go eu sz"),(0, "h8 h7 ha ez"),(3, "ho g8 h9 s8"),(3, "e9 gk s9 sk"),(3, "e7 hz sa hk"),],
        [-60, -60, -60, 180],
    );
    test_rules(
        "../../testdata/games/solo/37.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["e9 e8 ga hz hk h8 h7 s8","hu su gz gk g9 sz sk s7","eo ho eu ea ez ek e7 sa","go so gu g8 g7 ha h9 s9",],
        [(0, "hk hu eu h9"),(2, "e7 gu e9 su"),(3, "ha h7 s7 ea"),(2, "eo so e8 g9"),(2, "ho go ga sz"),(3, "g8 s8 gz ez"),(2, "sa s9 h8 sk"),(2, "ek g7 hz gk"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/38.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo go ho gz g7 ea ez hk","so eu hu ga gk ha sa s9","gu su g9 g8 e9 h8 sk s7","ek e8 e7 hz h9 h7 sz s8",],
        [(0, "eo gk g8 e7"),(0, "ho hu g9 e8"),(0, "go eu su h7"),(0, "ea ga e9 ek"),(1, "ha h8 h9 hk"),(1, "sa s7 s8 gz"),(0, "g7 so gu sz"),(1, "s9 sk hz ez"),],
        [-240, 80, 80, 80],
    );
    test_rules(
        "../../testdata/games/solo/39.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["eu hu ha h7 e8 e7 ga g9","ez gz g7 sa sz sk s9 s8","su hz hk h9 h8 ea ek e9","eo go ho so gu gk g8 s7",],
        [(0, "e7 ez ea gu"),(3, "eo ha sa h8"),(3, "go h7 sz h9"),(3, "ho hu gz su"),(3, "so eu sk hk"),(3, "gk ga g7 hz"),(2, "ek s7 e8 s9"),(2, "e9 g8 g9 s8"),],
        [130, 130, -390, 130], // TODO support Kontra
    );
    test_rules(
        "../../testdata/games/solo/40.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["gu gk g9 ea e8 e7 sz sk","eu hu ek e9 ha h8 s9 s8","eo go so gz g8 g7 h9 h7","ho su ga ez hz hk sa s7",],
        [(0, "ea e9 gz ez"),(2, "go su gk hu"),(2, "eo ga g9 eu"),(2, "g7 ho gu ek"),(3, "sa sk s8 g8"),(2, "so s7 e8 s9"),(2, "h7 hz sz ha"),(1, "h8 h9 hk e7"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/41.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo go hu gk g9 g7 h8 sa","gu gz g8 e8 e7 hk sz s9","eu su ga ez e9 h9 h7 s8","ho so ea ek ha hz sk s7",],
        [(0, "eo g8 su so"),(0, "go gu eu ho"),(0, "hu gz ga s7"),(0, "h8 hk h9 ha"),(3, "hz g9 e7 h7"),(0, "g7 e8 s8 sk"),(0, "gk s9 e9 ek"),(0, "sa sz ez ea"),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/solo/42.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["hu su hz ea e8 ga gz s7","gu ha h8 ez ek g9 sz s8","h9 h7 e9 e7 gk g8 sk s9","eo go ho so eu hk g7 sa",],
        [(0, "ea ek e7 hk"),(3, "eo su h8 h7"),(3, "go hu gu h9"),(3, "ho hz ha e9"),(3, "sa s7 s8 s9"),(3, "g7 ga g9 gk"),(0, "gz sz g8 so"),(3, "eu e8 ez sk"),],
        [-110, -110, -110, 330],
    );
    test_rules(
        "../../testdata/games/solo/43.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eu ea e8 ga g7 sa sz s9","go gu e7 gz g9 h7 sk s7","ek e9 gk g8 hz h9 h8 s8","eo ho so hu su ez ha hk",],
        [(0, "ga gz g8 ez"),(3, "so ea go ek"),(1, "h7 h9 ha eu"),(0, "sa sk s8 su"),(3, "ho e8 e7 e9"),(3, "eo g7 gu gk"),(3, "hk sz s7 hz"),(2, "h8 hu s9 g9"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/44.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["ho hu ga ez e7 hk h9 s8","eo go so eu gz gk g8 h7","gu su ek e8 ha hz sa sk","g9 g7 ea e9 h8 sz s9 s7",],
        [(0, "hk h7 ha h8"),(2, "sa s7 s8 gz"),(1, "go su g7 hu"),(1, "eo gu g9 ga"),(1, "eu hz sz ho"),(0, "h9 gk e8 s9"),(1, "so sk e9 e7"),(1, "g8 ek ea ez"),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/STrumpfDeciderFarbe<45>.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["ho eu ez e7 gz g8 sa sz","hu e9 g9 ha hk h8 h7 s9","so gu gk g7 hz h9 sk s7","eo go su ea ek e8 ga s8",],
        [(0, "sa s9 sk s8"),(0, "sz hu s7 go"),(3, "eo e7 e9 gu"),(3, "su eu ha so"),(2, "h9 ea g8 h7"),(3, "ga gz g9 g7"),(3, "e8 ez hk hz"),(0, "ho h8 gk ek"),],
        [50, 50, 50, -150],
    );
    test_rules(
        "../../testdata/games/solo/46.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["eo hu ez e9 e8 ga gz sa","ho gu su hk h9 ea sz s9","go so eu ha hz h8 h7 gk","ek e7 g9 g8 g7 sk s8 s7",],
        [(0, "sa sz so s7"),(2, "eu sk eo hk"),(0, "ga s9 gk g7"),(0, "gz ho go g8"),(2, "h8 ek hu h9"),(0, "e9 ea ha e7"),(2, "h7 g9 e8 su"),(1, "gu hz s8 ez"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/47.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["eo go so eu gu hk h7 sa","h8 ea ez e9 e8 ga g9 s9","ha e7 gz g8 g7 sz s8 s7","ho hu su hz h9 ek gk sk",],
        [(0, "eo h8 ha h9"),(0, "go s9 e7 su"),(0, "gu ea gz ho"),(3, "ek hk e8 g7"),(0, "so ez g8 hz"),(0, "eu e9 s7 hu"),(0, "sa g9 s8 sk"),(0, "h7 ga sz gk"),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/solo/48.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["go ho eu ea ez e9 g9 ha","so hu su e8 hz h8 s8 s7","ga gz gk g8 g7 hk sz s9","eo gu ek e7 h9 h7 sa sk",],
        [(0, "eu so ga ek"),(1, "s7 s9 sa ea"),(0, "ho e8 hk e7"),(0, "go su g7 eo"),(3, "h9 ha h8 g8"),(0, "e9 hu sz gu"),(3, "sk ez s8 gk"),(0, "g9 hz gz h7"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/49.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["hu su ga e9 e7 h8 h7 s9","ho so g9 g7 ez ha hk s8","gu ek e8 hz h9 sa sz sk","eo go eu gz gk g8 ea s7",],
        [(0, "s9 s8 sa s7"),(2, "sz eu h7 so"),(1, "ha h9 gz h8"),(3, "go su g7 gu"),(3, "eo hu g9 e8"),(3, "g8 ga ho hz"),(1, "hk ek gk e7"),(3, "ea e9 ez sk"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/50.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["go ho so eu gu hk h8 g8","su h9 ez e8 e7 ga sz s8","eo hu ha h7 ek gk sk s9","hz ea e9 gz g9 g7 sa s7",],
        [(0, "gu h9 ha hz"),(0, "eu su h7 g7"),(0, "so ez eo gz"),(2, "gk g9 g8 ga"),(1, "e8 ek ea hk"),(0, "ho e7 hu e9"),(0, "go s8 s9 s7"),(0, "h8 sz sk sa"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/51.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["eo so hu su hk h8 h7 g7","ho hz e7 gk g8 sa s9 s7","gu ha ea ek ga gz g9 sz","go eu h9 ez e9 e8 sk s8",],
        [(0, "eo ho gu h9"),(0, "hu hz ha eu"),(3, "s8 hk s7 sz"),(0, "su sa ga go"),(3, "sk g7 s9 gz"),(3, "e8 h7 e7 ek"),(0, "so g8 g9 e9"),(0, "h8 gk ea ez"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/52.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["go gu g7 ea e7 hk h9 s9","eu hu gz e9 ha h8 h7 s7","eo ho so su ga gk g8 ek","g9 ez e8 hz sa sz sk s8",],
        [(0, "s9 s7 ek sa"),(3, "sz hk eu so"),(2, "eo g9 g7 hu"),(2, "su hz gu gz"),(0, "ea e9 ga e8"),(2, "ho ez go ha"),(0, "h9 h7 gk s8"),(2, "g8 sk e7 h8"),],
        [50, 50, -150, 50],
    );
    test_rules(
        "../../testdata/games/solo/53.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["hu su ek ga h9 h8 h7 sz","eo go gu ea e8 e7 g8 sa","ho eu e9 gk g7 ha sk s9","so ez gz g9 hz hk s8 s7",],
        [(0, "ga g8 gk gz"),(0, "h7 ea ha hk"),(1, "go e9 ez su"),(1, "eo eu so hu"),(1, "e8 ho hz ek"),(2, "s9 s8 sz sa"),(1, "gu sk g9 h8"),(1, "e7 g7 s7 h9"),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/54.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["hu ea ek e9 e8 e7 sa s9","so ha ez gz gk g8 g7 sz","gu su hz h9 ga sk s8 s7","eo go ho eu hk h8 h7 g9",],
        [(0, "sa sz s8 hk"),(3, "go hu ha h9"),(3, "ho s9 so su"),(3, "eu e7 g7 hz"),(3, "eo e8 g8 gu"),(3, "g9 e9 gz ga"),(2, "sk h8 ek gk"),(3, "h7 ea ez s7"),],
        [-90, -90, -90, 270],
    );
    test_rules(
        "../../testdata/games/solo/55.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["su ea ez ek e9 gz sz s8","so eu hk g9 g7 sk s9 s7","go gu h8 e7 ga gk g8 sa","eo ho hu ha hz h9 h7 e8",],
        [(0, "gz g7 ga ha"),(3, "hu su so h8"),(1, "sk sa hz s8"),(3, "eo e9 hk gu"),(3, "h7 sz eu go"),(2, "gk e8 ea g9"),(2, "g8 h9 ez s9"),(3, "ho ek s7 e7"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/56.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo ho gu hu ek e9 ha h7","go so eu su e7 g7 hk sz","ez e8 ga g9 g8 h9 sk s8","ea gz gk hz h8 sa s9 s7",],
        [(0, "eo e7 e8 ea"),(0, "hu eu ez gz"),(1, "hk h9 h8 ha"),(0, "gu so ga hz"),(1, "g7 g8 gk ek"),(0, "h7 su sk sa"),(1, "sz s8 s7 ho"),(0, "e9 go g9 s9"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/57.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo so eu hu ek e9 e7 sa","go ho su ez h9 h8 h7 sz","gu ea gz gk g9 hk s9 s8","e8 ga g8 g7 ha hz sk s7",],
        [(0, "eo su gu e8"),(0, "hu go ea ha"),(1, "h9 hk hz ek"),(0, "eu ho gz ga"),(1, "sz s8 s7 sa"),(0, "so ez s9 g7"),(0, "e9 h8 g9 g8"),(0, "e7 h7 gk sk"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/58.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["ez g9 g7 ha h9 h8 sa s8","so eu gu su ek e9 e8 e7","eo ga gz gk g8 hk sz s7","go ho hu ea hz h7 sk s9",],
        [(0, "ha su hk h7"),(1, "gu eo ea ez"),(2, "ga hz g7 ek"),(1, "eu gz ho g9"),(3, "s9 sa e9 s7"),(1, "e8 sz hu s8"),(3, "go h8 e7 gk"),(3, "sk h9 so g8"),],
        [80, -240, 80, 80],
    );
    test_rules(
        "../../testdata/games/solo/59.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["go so gu ga gz g9 g8 ha","ho hu ez e7 hk sa sk s9","eo eu ea e9 e8 h9 h8 s7","su gk g7 ek hz h7 sz s8",],
        [(0, "gu ho eu gk"),(1, "sa s7 s8 ga"),(0, "g8 hu eo g7"),(2, "h9 h7 ha hk"),(0, "go ez h8 su"),(0, "so s9 e8 ek"),(0, "gz e7 e9 hz"),(0, "g9 sk ea sz"),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/solo/60.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo go gu ea e9 e7 g9 sa","su ez ek e8 ga g7 h9 s8","ho eu hu gk g8 ha h8 sz","so gz hz hk h7 sk s9 s7",],
        [(0, "go e8 hu so"),(0, "eo ek eu s7"),(0, "gu ez ho gz"),(2, "ha hk e9 h9"),(0, "e7 su sz hz"),(1, "ga gk sk g9"),(1, "s8 g8 s9 sa"),(0, "ea g7 h8 h7"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/61.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["go eu h9 ea e8 e7 g9 s9","so gu ez e9 g8 sz sk s7","su ha hz hk h7 ga gz g7","eo ho hu h8 ek gk sa s8",],
        [(0, "ea ez ha ek"),(2, "h7 h8 h9 gu"),(1, "e9 hk gk e8"),(2, "su hu eu so"),(1, "g8 gz ho g9"),(3, "s8 s9 s7 g7"),(0, "e7 sk hz eo"),(3, "sa go sz ga"),],
        [120, 120, -360, 120],
    );
    test_rules(
        "../../testdata/games/solo/62.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["so hu e7 ga g7 h9 sz s8","e8 gk g9 g8 hk h8 h7 s7","eo go ho eu su ea ez ha","gu ek e9 gz hz sa sk s9",],
        [(0, "h9 h8 ha hz"),(2, "eo e9 e7 e8"),(2, "ho ek hu h7"),(2, "go gu so s7"),(2, "eu s9 g7 g8"),(2, "su sk s8 g9"),(2, "ea gz sz hk"),(2, "ez sa ga gk"),],
        [-100, -100, 300, -100],
    );
    test_rules(
        "../../testdata/games/solo/63.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["go so eu hu gz gk ek sa","gu g8 e9 e7 ha hz h9 h7","eo ho g9 g7 ez e8 h8 sz","su ga ea hk sk s9 s8 s7",],
        [(0, "eu g8 ho ga"),(2, "sz sk sa gu"),(1, "e7 e8 ea ek"),(3, "s9 hu e9 h8"),(0, "so hz eo su"),(2, "ez hk gk h7"),(0, "go h9 g7 s7"),(0, "gz ha g9 s8"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../STrumpfDeciderFarbe<testdata>/games/solo/64.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo go ho so gu ek g7 h8","hu su ea ez e9 g8 h7 sk","eu e7 ga gk hz hk sa s7","e8 gz g9 ha h9 sz s9 s8",],
        [(0, "eo hu e7 e8"),(0, "go e9 eu s8"),(0, "so ez sa s9"),(0, "gu su s7 h9"),(0, "ho ea hk sz"),(0, "g7 g8 gk gz"),(3, "ha h8 h7 hz"),(3, "g9 ek sk ga"),],
        [270, -90, -90, -90],
    );
    test_rules(
        "../../testdata/games/solo/65.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["h7 ea ez ek e9 e7 gk sz","gu hk e8 ga gz g8 s9 s8","ho ha hz h8 g7 sa sk s7","eo go so eu hu su h9 g9",],
        [(0, "gk ga g7 g9"),(1, "gz sk eu h7"),(3, "go ez hk h8"),(3, "eo e7 gu hz"),(3, "su sz e8 ho"),(2, "sa h9 e9 s8"),(3, "so ek s9 ha"),(3, "hu ea g8 s7"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/66.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo ho gu ga g9 ea e8 ha","go su g8 ez e7 hk sa s9","so hu gz g7 e9 hz h7 sz","eu gk ek h9 h8 sk s8 s7",],
        [(0, "gu su gz eu"),(3, "s7 ga s9 sz"),(0, "eo g8 g7 gk"),(0, "g9 go hu ek"),(1, "hk h7 h9 ha"),(0, "ho e7 so h8"),(0, "ea ez e9 s8"),(0, "e8 sa hz sk"),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/solo/67.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["hu gk g9 g8 e9 ha h9 h8","eo so gu ga gz g7 sz s8","go eu su ez ek h7 s9 s7","ho ea e8 e7 hz hk sa sk",],
        [(0, "ha ga h7 hk"),(1, "gu eu ho gk"),(3, "sa e9 s8 s7"),(3, "sk g9 sz s9"),(0, "h9 so ek hz"),(1, "eo su e7 g8"),(1, "g7 go ea hu"),(2, "ez e8 h8 gz"),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/68.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["so eu ek e8 e7 gz gk sz","ho gu su hk h9 h8 sa s9","eo go hu ea ez e9 ga h7","g9 g8 g7 ha hz sk s8 s7",],
        [(0, "gk gu ga g9"),(1, "hk h7 ha sz"),(3, "g8 gz ho go"),(2, "e9 hz ek su"),(1, "h9 hu sk eu"),(0, "e7 h8 ea s7"),(2, "eo s8 e8 s9"),(2, "ez g7 so sa"),],
        [60, 60, -180, 60],
    );
    test_rules(
        "../../testdata/games/solo/69.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo go eu gu g9 g8 ea s7","so gz ek e7 ha hk h7 sa","ho ga ez e9 hz h9 h8 s8","hu su gk g7 e8 sz sk s9",],
        [(0, "go gz ga g7"),(0, "eo so ho su"),(0, "gu ek s8 gk"),(0, "eu e7 h8 hu"),(0, "ea h7 e9 e8"),(0, "s7 sa hz sz"),(1, "ha h9 s9 g8"),(0, "g9 hk ez sk"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/70.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["e9 gk g8 ha h7 sa sz s8","go ho eu ea ek e8 e7 hk","eo so hu su gz g9 h9 h8","gu ez ga g7 hz sk s9 s7",],
        [(0, "sa ea su sk"),(2, "h8 hz h7 hk"),(3, "s9 sz ho eo"),(2, "h9 gu ha eu"),(1, "go hu ez e9"),(1, "e8 so ga gk"),(2, "gz g7 g8 ek"),(1, "e7 g9 s7 s8"),],
        [50, -150, 50, 50], // TODO support Kontra
    );
    test_rules(
        "../../testdata/games/solo/71.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eu ea e8 h8 sz s9 s8 s7","ho so gu su gk g9 g8 ha","eo hu ez e9 e7 hk sa sk","go ga gz g7 ek hz h9 h7",],
        [(0, "sz gk sk ga"),(3, "h9 h8 ha hk"),(1, "gu hu gz eu"),(0, "s7 su sa go"),(3, "ek e8 so e7"),(1, "g9 eo g7 ea"),(2, "e9 h7 s8 g8"),(1, "ho ez hz s9"),],
        [50, -150, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/72.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["go ho so eu ga g8 g7 sk","g9 ha hz h9 sz s9 s8 s7","eo hu su gz gk h8 h7 sa","gu ea ez ek e9 e8 e7 hk",],
        [(0, "eu g9 gk gu"),(0, "so s7 su e7"),(0, "ho ha eo ea"),(2, "sa hk sk sz"),(2, "h7 ek g8 h9"),(0, "go s8 gz e8"),(0, "g7 hz hu ez"),(2, "h8 e9 ga s9"),],
        [-150, 50, 50, 50], // TODO support Kontra
    );
    test_rules(
        "../../testdata/games/solo/73.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["go gu ek e9 g7 sz sk s9","ho eu h7 ez ga gk g8 sa","su hz h9 e8 gz g9 s8 s7","eo so hu ha hk h8 ea e7",],
        [(0, "sk sa s7 ha"),(3, "eo gu h7 h9"),(3, "h8 go eu hz"),(0, "g7 ga g9 hk"),(3, "hu sz ho su"),(1, "ez e8 ea e9"),(3, "so s9 g8 s8"),(3, "e7 ek gk gz"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/74.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["gu hu su h8 ea e7 ga g8","so h9 ez e8 g9 g7 sz s7","ha hz ek gz gk sa sk s9","eo go ho eu hk h7 e9 s8",],
        [(0, "ea e8 ek e9"),(0, "e7 ez sk hk"),(3, "eo h8 h9 hz"),(3, "ho su so ha"),(3, "eu hu g9 s9"),(3, "go gu g7 sa"),(3, "s8 ga sz gz"),(1, "s7 gk h7 g8"),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/solo/75.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo go so hu ek e9 hz hk","ho ea ez e7 h8 sz sk s7","gu ga gz g9 g7 ha h9 sa","eu su e8 gk g8 h7 s9 s8",],
        [(0, "go ez gu e8"),(0, "eo e7 g7 su"),(0, "hu ea ga eu"),(3, "h7 hk h8 ha"),(2, "gz g8 e9 s7"),(0, "so ho sa gk"),(1, "sz h9 s8 ek"),(0, "hz sk g9 s9"),],
        [-150, 50, 50, 50], // TODO support doppeln
    );
    test_rules(
        "../../testdata/games/solo/76.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["so su ea e7 ha hz h9 sk","ho eu gu gz gk h7 sa s9","ek g9 g8 g7 hk h8 s8 s7","eo go hu ez e9 e8 ga sz",],
        [(0, "sk sa s7 sz"),(1, "s9 s8 e9 h9"),(3, "go e7 gu ek"),(3, "eo su eu g9"),(3, "e8 ea ho hk"),(1, "gk g7 ga so"),(0, "ha h7 h8 ez"),(3, "hu hz gz g8"),],
        [50, 50, 50, -150],
    );
    test_rules(
        "../../testdata/games/solo/77.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["hu hk h9 e9 gk g7 sk s9","ez ek e7 gz g9 g8 sa sz","ho su ha h7 ea e8 s8 s7","eo go so eu gu hz h8 ga",],
        [(0, "e9 ek ea hz"),(3, "eo h9 e7 h7"),(3, "go hk g8 su"),(3, "gu hu ez ho"),(2, "s7 h8 s9 sz"),(3, "eu sk sa ha"),(3, "so g7 g9 s8"),(3, "ga gk gz e8"),],
        [-60, -60, -60, 180],
    );
    test_rules(
        "../../testdata/games/solo/78.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo go ho eu gu su g9 g7","so g8 e9 e8 ha h8 sk s9","hu gz gk ea ez ek hz hk","ga e7 h9 h7 sa sz s8 s7",],
        [(0, "eo g8 gk ga"),(0, "go so hu e7"),(0, "ho e9 gz h7"),(0, "eu e8 ek h9"),(0, "gu s9 hk s7"),(0, "su sk ez s8"),(0, "g9 h8 hz sz"),(0, "g7 ha ea sa"),],
        [300, -100, -100, -100],
    );
    test_rules(
        "../../testdata/games/solo/79.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["hz hk ek e7 ga g9 sa sk","go h8 ez e8 gk g8 s9 s8","eo so eu hu ha h9 h7 s7","ho gu su ea e9 gz g7 sz",],
        [(0, "ga gk ha g7"),(2, "eo su hk h8"),(2, "eu ho hz go"),(1, "g8 s7 gz g9"),(3, "sz sa s9 h7"),(2, "so gu e7 s8"),(2, "hu e9 sk e8"),(2, "h9 ea ek ez"),],
        [-60, -60, 180, -60],
    );
    test_rules(
        "../../testdata/games/solo/80.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["so ga g9 e7 hk h9 h8 s9","eu gu ek hz h7 sz s8 s7","ho su gk g7 ez e9 e8 sk","eo go hu gz g8 ea ha sa",],
        [(0, "h9 h7 gk ha"),(2, "ez ea e7 ek"),(3, "go g9 gu g7"),(3, "eo so eu su"),(3, "g8 ga hz ho"),(2, "e9 gz s9 s7"),(3, "hu h8 s8 e8"),(3, "sa hk sz sk"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/81.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["so hu ea ez e7 h7 sz s9","eu gu ga gz g8 hk h8 s7","g7 ek e9 e8 h9 sa sk s8","eo go ho su gk g9 ha hz",],
        [(0, "h7 h8 h9 ha"),(3, "eo hu g8 g7"),(3, "ho so gz e8"),(3, "go e7 gu e9"),(3, "su ea eu ek"),(1, "hk sk hz s9"),(3, "g9 sz ga sa"),(1, "s7 s8 gk ez"),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/solo/82.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo go so hu gk g8 e7 s9","g7 ez e8 ha hz h9 sk s8","eu gu su ga gz ea ek h8","ho g9 e9 hk h7 sa sz s7",],
        [(0, "go g7 su g9"),(0, "eo s8 gu ho"),(0, "so sk gz e9"),(0, "g8 ha ga sa"),(2, "ea hk e7 ez"),(2, "eu sz hu hz"),(2, "ek s7 s9 e8"),(2, "h8 h7 gk h9"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/83.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["eo go eu h8 ek e7 gk s8","ho hu h9 g9 g8 sz sk s9","so gu su ha hz h7 ea ga","hk ez e9 e8 gz g7 sa s7",],
        [(0, "s8 sk ha s7"),(2, "gu hk eu h9"),(0, "gk g8 ga g7"),(2, "su sa h8 hu"),(1, "s9 h7 e8 e7"),(2, "ea ez ek ho"),(1, "g9 hz gz go"),(0, "eo sz so e9"),],
        [90, 90, -270, 90],
    );
    test_rules(
        "../../testdata/games/solo/84.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["so eu e7 gz gk g9 g7 sa","eo go ho hu su ha h7 ek","hk h9 ez e9 g8 sz s9 s7","gu hz h8 ea e8 ga sk s8",],
        [(0, "gz su g8 ga"),(1, "go hk h8 eu"),(1, "ho h9 gu so"),(1, "hu sz hz e7"),(1, "eo s9 s8 g7"),(1, "ek ez ea sa"),(3, "e8 gk h7 e9"),(1, "ha s7 sk g9"),],
        [-80, 240, -80, -80],
    );
    test_rules(
        "../../testdata/games/solo/85.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["eo eu hz hk h9 ga sa s7","gu e9 e8 e7 gk g7 sz sk","go ho h8 ea ez ek gz s9","so hu su ha h7 g9 g8 s8",],
        [(0, "eo gu h8 h7"),(0, "h9 sz ho ha"),(2, "ea s8 hz e7"),(0, "ga g7 gz g8"),(0, "sa sk s9 su"),(3, "g9 hk gk go"),(2, "ek hu eu e8"),(0, "s7 e9 ez so"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/86.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["go ho so eu hk h9 h7 gk","eo hu su hz e7 g8 g7 s9","h8 ek e8 ga gz sz s8 s7","gu ha ea ez e9 g9 sa sk",],
        [(0, "eu hz h8 gu"),(0, "so eo ga ha"),(1, "e7 ek ea hk"),(0, "ho su e8 g9"),(0, "go hu s7 e9"),(0, "gk g7 gz ez"),(2, "s8 sa h7 s9"),(0, "h9 g8 sz sk"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/87.html",
        &*generate_sololike!(1, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eu ez gz g9 g7 hz h9 h7","eo ho hu su ea e9 e7 s7","go ek ga h8 sz sk s9 s8","so gu e8 gk g8 ha hk sa",],
        [(0, "g9 ea ga g8"),(1, "eo ek e8 eu"),(1, "su go gu ez"),(2, "sz sa hz s7"),(3, "ha h7 hu h8"),(1, "ho s8 so h9"),(1, "e7 sk hk gz"),(1, "e9 s9 gk g7"),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/88.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["ho eu hu ez e9 e7 sa sk","so gu su e8 gz g9 h9 s7","eo ek g7 hz hk h8 h7 s9","go ea ga gk g8 ha sz s8",],
        [(0, "eu so ek ea"),(1, "s7 s9 s8 sa"),(0, "hu e8 eo go"),(2, "hk ha e9 h9"),(0, "ho su g7 g8"),(0, "e7 gu hz ga"),(1, "gz h8 gk ez"),(0, "sk g9 h7 sz"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/89.html",
        &*generate_sololike!(2, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["su ga hz h8 h7 sa s9 s7","go hu g8 ez ek e8 e7 s8","eo so gu gz gk g9 ha hk","ho eu g7 ea e9 h9 sz sk",],
        [(0, "h8 hu hk h9"),(1, "ez gu e9 s7"),(2, "eo g7 su g8"),(2, "g9 eu ga go"),(1, "s8 gz sk s9"),(2, "so ho h7 ek"),(3, "ea hz e7 gk"),(2, "ha sz sa e8"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/90.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eu g9 ez hz h9 s9 s8 s7","go so gu su ea e7 hk h8","ho g7 ek e8 h7 sa sz sk","eo hu ga gz gk g8 e9 ha",],
        [(0, "ez e7 e8 e9"),(0, "s7 su sk hu"),(3, "g8 g9 gu g7"),(1, "ea ek gk eu"),(0, "s8 h8 sz ga"),(3, "eo s9 so ho"),(3, "ha hz hk h7"),(3, "gz h9 go sa"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/91.html",
        &*generate_sololike!(3, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, "Herz-Solo"),
        ["go hk ea e7 g9 g8 g7 s9","ha hz h7 e8 gz sz sk s8","ho eu hu ez ek e9 sa s7","eo so gu su h9 h8 ga gk",],
        [(0, "g9 gz eu gk"),(2, "ez gu e7 e8"),(3, "eo hk h7 hu"),(3, "su go ha ho"),(0, "g8 hz sa ga"),(1, "sk s7 h8 s9"),(3, "so g7 s8 e9"),(3, "h9 ea sz ek"),],
        [50, 50, 50, -150],
    );
    test_rules(
        "../../testdata/games/solo/92.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo go so gu gk g9 g8 e9","ga g7 ez ek e7 ha h7 sk","eu hu su gz h9 h8 sa s7","ho ea e8 hz hk sz s9 s8",],
        [(0, "go ga su ho"),(0, "so g7 hu s8"),(0, "eo e7 gz e8"),(0, "gu sk eu sz"),(2, "sa s9 gk h7"),(0, "e9 ez s7 ea"),(3, "hk g9 ha h8"),(0, "g8 ek h9 hz"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/93.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["so hu su ez e8 ga ha sa","go ek g9 hk h8 sz s9 s7","eo ho e9 gk hz h9 h7 sk","eu gu ea e7 gz g8 g7 s8",],
        [(0, "su ek e9 gu"),(3, "g8 ga g9 gk"),(0, "hu go ho ea"),(1, "s7 sk s8 sa"),(0, "e8 sz eo e7"),(2, "h7 eu ha hk"),(3, "g7 ez s9 h9"),(0, "so h8 hz gz"),],
        [240, -80, -80, -80],
    );
    test_rules(
        "../../testdata/games/solo/94.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, "Eichel-Solo"),
        ["eo ho su ez e9 ga ha hk","so hu ek e8 g9 g8 hz s9","gu e7 h9 h7 sa sk s8 s7","go eu ea gz gk g7 h8 sz",],
        [(0, "ho e8 e7 go"),(3, "gk ga g8 gu"),(2, "sa sz ez s9"),(0, "eo hu h7 eu"),(0, "e9 ek sk ea"),(3, "h8 ha hz h9"),(0, "su so s8 gz"),(1, "g9 s7 g7 hk"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/95.html",
        &*generate_sololike!(0, SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, "Gras-Solo"),
        ["eo go ho eu ga g9 e8 sa","hu gz gk g7 ek e9 s9 s8","so gu su e7 hz h9 h8 s7","g8 ea ez ha hk h7 sz sk",],
        [(0, "go g7 su g8"),(0, "ho gk gu h7"),(0, "eo hu so hk"),(0, "eu gz e7 sk"),(0, "e8 ek hz ea"),(3, "ez g9 e9 s7"),(0, "sa s8 h8 sz"),(0, "ga s9 h9 ha"),],
        [270, -90, -90, -90],
    );
}
