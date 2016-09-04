use primitives::*;
use rules::*;
use rules::trumpfdecider::*;
use std::fmt;
use std::cmp::Ordering;

pub struct SRulesRufspiel {
    pub m_eplayerindex : EPlayerIndex,
    pub m_efarbe : EFarbe, // TODO possibly wrap with ENonHerzFarbe or similar
}

impl fmt::Display for SRulesRufspiel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rufspiel mit der {}-Sau von {}", self.m_efarbe, self.m_eplayerindex)
    }
}

pub type STrumpfDeciderRufspiel = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, STrumpfDeciderFarbe<
    SFarbeDesignatorHerz>>>;

impl SRulesRufspiel {
    fn rufsau(&self) -> SCard {
        SCard::new(self.m_efarbe, ESchlag::Ass)
    }

    fn is_ruffarbe(&self, card: SCard) -> bool {
        VTrumpfOrFarbe::Farbe(self.m_efarbe)==self.trumpf_or_farbe(card)
    }
}

impl TRules for SRulesRufspiel {
    fn can_be_played(&self, hand: &SHand) -> bool {
        let it = || {hand.cards().iter().filter(|&card| self.is_ruffarbe(*card))};
        it().all(|card| card.schlag()!=ESchlag::Ass)
        && 0<it().count()
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_eplayerindex)
    }

    fn trumpf_or_farbe(&self, card: SCard) -> VTrumpfOrFarbe {
        STrumpfDeciderRufspiel::trumpf_or_farbe(card)
    }

    fn stoss_allowed(&self, eplayerindex: EPlayerIndex, vecstoss: &Vec<SStoss>, hand: &SHand) -> bool {
        assert_eq!(hand.cards().len(), 8);
        assert!(eplayerindex!=self.m_eplayerindex || !hand.contains(self.rufsau()));
        (eplayerindex==self.m_eplayerindex || hand.contains(self.rufsau())) == (vecstoss.len()%2==1)
    }

    fn payout(&self, vecstich: &Vec<SStich>) -> [isize; 4] {
        assert_eq!(vecstich.len(), 8);
        let eplayerindex_coplayer = vecstich.iter()
            .flat_map(|stich| stich.indices_and_cards())
            .find(|&(_, card)| card==self.rufsau())
            .map(|(eplayerindex, _)| eplayerindex)
            .unwrap();
        let (eschneiderschwarz, ab_winner) = points_to_schneiderschwarz_and_winners(
            vecstich,
            self,
            /*fn_is_player_party*/|eplayerindex| {
                eplayerindex==self.m_eplayerindex || eplayerindex==eplayerindex_coplayer
            },
        );
        let n_laufende = STrumpfDeciderRufspiel::count_laufende(vecstich, &ab_winner);
        create_playerindexmap(|eplayerindex| {
            (/*n_payout_rufspiel_default*/ 20 
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
            }
        } )
    }

    fn all_allowed_cards_first_in_stich(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        if // do we already know who had the rufsau?
            !vecstich.iter()
                .take_while(|stich| 4==stich.size()) // process full stichs
                .fold(/*b_rufsau_known_initial*/false, |b_rufsau_known_before_stich, stich| {
                    if b_rufsau_known_before_stich {
                        // already known
                        true
                    } else if self.is_ruffarbe(stich.first_card()) {
                        // gesucht or weggelaufen
                        true
                    } else {
                        // We explicitly traverse all cards because it may be allowed 
                        // (by exotic rules) to schmier rufsau even if not gesucht.
                        stich.indices_and_cards().any(|(_, card)| card==self.rufsau())
                    }
                } )
        {
            // Remark: Player must have 4 cards of ruffarbe on his hand *at this point of time* (i.e. not only at the beginning!)
            if !hand.contains(self.rufsau()) 
                || 4 <= hand.cards().iter()
                    .filter(|&card| self.is_ruffarbe(*card))
                    .count()
            {
                hand.cards().clone()
            } else {
                hand.cards().iter()
                    .cloned()
                    .filter(|&card| !self.is_ruffarbe(card) || self.rufsau()==card)
                    .collect::<SHandVector>()
            }
        }
        else {
            hand.cards().clone()
        }
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        if hand.cards().len()<=1 {
            hand.cards().clone()
        } else {
            let card_first = vecstich.last().unwrap().first_card();
            if self.is_ruffarbe(card_first) && hand.contains(self.rufsau()) {
                // special case: gesucht
                // TODO Consider the following distribution of cards:
                // 0: GA GZ GK G8 ...   <- opens first stich
                // 1, 2: ..             <- mainly irrelevant
                // 3: G7 G9 ...         <- plays with GA
                // The first two stichs are as follows:
                //      e7        ..
                //   e9   g9    ..  >g7
                //     >g8        ..
                // Is player 0 obliged to play GA? We implement it this way for now.
                Some(self.rufsau()).into_iter().collect()
            } else {
                let veccard_allowed : SHandVector = hand.cards().iter()
                    .filter(|&&card| 
                        self.rufsau()!=card 
                        && self.trumpf_or_farbe(card)==self.trumpf_or_farbe(card_first)
                    )
                    .cloned()
                    .collect();
                if veccard_allowed.is_empty() {
                    hand.cards().iter().cloned().filter(|&card| self.rufsau()!=card).collect()
                } else {
                    veccard_allowed
                }
            }
        }
    }

    fn compare_in_stich_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        STrumpfDeciderRufspiel::compare_trumpfcards_solo(card_fst, card_snd)
    }

}

#[test]
fn test_rulesrufspiel() {
    use rules::test_rules::*;
    test_rules(
        "../../testdata/games/1.html",
        &SRulesRufspiel{m_eplayerindex: 3, m_efarbe: EFarbe::Schelln},
        ["eo hz h9 e8 ga gk sk s9","gu hu ez ek e7 gz g9 s7","ho so su ha e9 g8 g7 sa","go eu hk h8 h7 ea sz s8",],
        [(0, "sk s7 sa sz"),(2, "ho h7 eo hu"),(0, "ga gz g7 hk"),(3, "go h9 gu ha"),(3, "ea e8 e7 e9"),(3, "s8 s9 g9 g8"),(0, "gk ek su h8"),(2, "so eu hz ez"),],
        [-30, -30, 30, 30],
    );
    test_rules(
        "../../testdata/games/2.html",
        &SRulesRufspiel{m_eplayerindex: 2, m_efarbe: EFarbe::Schelln},
        ["gu su hk e9 e8 e7 ga sz","so hu hz h8 ez g9 g8 s7","eo go ha h9 h7 sk s9 s8","ho eu ea ek gz gk g7 sa",],
        [(0, "sz s7 s8 sa"),(3, "ho hk h8 ha"),(3, "eu su so go"),(2, "eo gz gu hz"),(2, "h7 ek e7 hu"),(1, "g8 h9 gk ga"),(2, "sk g7 e8 g9"),(2, "s9 ea e9 ez"),],
        [-60, -60, 60, 60],
    );
    test_rules(
        "../../testdata/games/3.html",
        &SRulesRufspiel{m_eplayerindex: 3, m_efarbe: EFarbe::Gras},
        ["eu su h9 ea gz g9 sz sk","eo ho gu h7 ez g8 g7 s8","go ek e9 e7 ga sa s9 s7","so hu ha hz hk h8 e8 gk",],
        [(0, "g9 g7 ga gk"),(2, "go h8 h9 eo"),(1, "g8 e7 hk gz"),(3, "e8 ea ez e9"),(0, "sk s8 sa ha"),(3, "hu eu h7 s7"),(0, "sz ho s9 so"),(1, "gu ek hz su"),],
        [20, 20, -20, -20],
    );
    test_rules(
        "../../testdata/games/4.html",
        &SRulesRufspiel{m_eplayerindex: 1, m_efarbe: EFarbe::Gras},
        ["ha ea ez e8 e7 ga gk sk","eu hu su hz h8 h7 g9 g7","eo ho gu hk e9 gz sa s9","go so h9 ek g8 sz s8 s7",],
        [(0, "ha h7 ho h9"),(2, "sa s8 sk hz"),(1, "h8 hk so e7"),(3, "g8 ga g7 gz"),(0, "gk g9 s9 ek"),(0, "e8 hu e9 go"),(3, "sz ea su gu"),(2, "eo s7 ez eu"),],
        [-60, -60, 60, 60],
    );
    test_rules(
        "../../testdata/games/5.html",
        &SRulesRufspiel{m_eplayerindex: 2, m_efarbe: EFarbe::Gras},
        ["so eu su h7 ek g7 sk s8","eo h9 ea ez e9 e8 g8 s7","go ho gu ha hz g9 sa sz","hu hk h8 e7 ga gz gk s9",],
        [(0, "g7 g8 g9 ga"),(3, "hu h7 eo gu"),(1, "ea ha e7 ek"),(2, "go hk su h9"),(2, "ho h8 eu s7"),(2, "sa s9 s8 e8"),(2, "sz gk sk e9"),(2, "hz gz so ez"),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/6.html",
        &SRulesRufspiel{m_eplayerindex: 0, m_efarbe: EFarbe::Schelln},
        ["eo ho hz h9 h8 ga gk s8","hu su hk ea e7 gz sa sz","go so eu gu e8 g9 s9 s7","ha h7 ez ek e9 g8 g7 sk",],
        [(0, "h8 su eu ha"),(2, "s7 sk s8 sa"),(1, "ea e8 e9 ga"),(1, "e7 g9 ek hz"),(0, "gk gz gu g7"),(2, "s9 h7 h9 sz"),(0, "eo hk so g8"),(0, "ho hu go ez"),],
        [20, 20, -20, -20],
    );
    test_rules(
        "../../testdata/games/7.html",
        &SRulesRufspiel{m_eplayerindex: 0, m_efarbe: EFarbe::Gras},
        ["eo eu hu ha hk g7 sz s8","ho ez e9 e7 gz gk g9 sa","go so hz h9 ek e8 sk s9","gu su h8 h7 ea ga g8 s7",],
        [(0, "hk ho h9 h7"),(1, "gz hz ga g7"),(2, "e8 ea sz e7"),(3, "gu hu ez so"),(2, "s9 s7 s8 sa"),(1, "gk go g8 eo"),(0, "eu g9 ek h8"),(0, "ha e9 sk su"),],
        [-20, 20, 20, -20],
    );
    test_rules(
        "../../testdata/games/8.html",
        &SRulesRufspiel{m_eplayerindex: 0, m_efarbe: EFarbe::Gras},
        ["eo go ho hz h7 e9 gz sa","so gu hu hk h8 ea ez g7","su e8 gk g9 g8 sk s9 s7","eu ha h9 ek e7 ga sz s8",],
        [(0, "eo h8 su ha"),(0, "go hk e8 h9"),(0, "h7 so s9 eu"),(1, "g7 g8 ga gz"),(3, "ek e9 ea gk"),(1, "gu g9 sz ho"),(0, "sa hu sk s8"),(1, "ez s7 e7 hz"),],
        [50, -50, -50, 50],
    );
    test_rules(
        "../../testdata/games/9.html",
        &SRulesRufspiel{m_eplayerindex: 1, m_efarbe: EFarbe::Gras},
        ["so ez ek e7 ga sz sk s8","go hu ha hz hk h8 e9 gk","eo gu h9 h7 g8 sa s9 s7","ho eu su ea e8 gz g9 g7",],
        [(0, "so h8 h7 ho"),(3, "gz ga gk g8"),(0, "sk ha s7 su"),(3, "g7 s8 hk s9"),(1, "hu gu eu e7"),(3, "ea ez e9 sa"),(3, "e8 ek hz eo"),(2, "h9 g9 sz go"),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/10.html",
        &SRulesRufspiel{m_eplayerindex: 2, m_efarbe: EFarbe::Eichel},
        ["eu ha h8 ea e8 e7 ga g7","eo hk ez ek sz s9 s8 s7","go so gu hu h9 h7 e9 sa","ho su hz gz gk g9 g8 sk",],
        [(0, "ha eo h7 hz"),(1, "ek e9 su ea"),(3, "sk h8 s7 sa"),(0, "eu hk h9 ho"),(3, "g8 ga s8 hu"),(2, "go g9 g7 s9"),(2, "so gk e7 sz"),(2, "gu gz e8 ez"),],
        [20, -20, 20, -20],
    );
    test_rules(
        "../../testdata/games/11.html",
        &SRulesRufspiel{m_eplayerindex: 0, m_efarbe: EFarbe::Eichel},
        ["eo go so ha hk ek gz g9","su h9 e9 e8 gk g7 s9 s8","ho eu hu h8 ez e7 ga sz","gu hz h7 ea g8 sa sk s7",],
        [(0, "eo h9 h8 hz"),(0, "hk su eu h7"),(2, "e7 ea ek e9"),(3, "gu go gk hu"),(0, "g9 g7 ga g8"),(2, "ho s7 ha e8"),(2, "ez sk so s9"),(0, "gz s8 sz sa"),],
        [20, -20, -20, 20],
    );
    test_rules(
        "../../testdata/games/12.html",
        &SRulesRufspiel{m_eplayerindex: 2, m_efarbe: EFarbe::Gras},
        ["eo so eu h7 e7 gk g9 g8","gu hu ek e8 ga sz s9 s7","go ho su hz hk h8 gz s8","ha h9 ea ez e9 g7 sa sk",],
        [(0, "gk ga gz g7"),(1, "gu h8 ha eu"),(0, "e7 ek hz e9"),(2, "ho h9 h7 hu"),(2, "su ea so e8"),(0, "g9 sz hk sk"),(2, "s8 sa g8 s7"),(3, "ez eo s9 go"),],
        [-20, 20, 20, -20],
    );
    test_rules(
        "../../testdata/games/13.html",
        &SRulesRufspiel{m_eplayerindex: 2, m_efarbe: EFarbe::Eichel},
        ["su hk h7 ek ga gz g9 g7","go so eu ez e8 gk g8 sz","ho gu hu ha hz h8 e7 s7","eo h9 ea e9 sa sk s9 s8",],
        [(0, "ek e8 e7 ea"),(3, "eo h7 eu ha"),(3, "h9 su go h8"),(1, "ez hu e9 g7"),(2, "ho sa hk so"),(2, "gu sk gz g8"),(2, "s7 s8 g9 sz"),(1, "gk hz s9 ga"),],
        [-30, -30, 30, 30],
    );
    test_rules(
        "../../testdata/games/14.html",
        &SRulesRufspiel{m_eplayerindex: 3, m_efarbe: EFarbe::Gras},
        ["ho h9 e7 ga gz g9 sz s9","go so eu ea e9 g8 sk s8","gu su hk h8 h7 ez ek e8","eo hu ha hz gk g7 sa s7",],
        [(0, "ho go h7 hu"),(1, "g8 hk g7 ga"),(2, "ek ha e7 e9"),(3, "eo h9 eu h8"),(3, "sa s9 s8 su"),(2, "ez hz sz ea"),(3, "s7 g9 sk e8"),(1, "so gu gk gz"),],
        [20, -20, -20, 20],
    );
    test_rules(
        "../../testdata/games/15.html",
        &SRulesRufspiel{m_eplayerindex: 0, m_efarbe: EFarbe::Eichel},
        ["ho so gu su ek ga s9 s7","go hk h8 h7 ea sa sk s8","eu hu ha ez e7 gz g9 g8","eo hz h9 e9 e8 gk g7 sz",],
        [(0, "su go hu h9"),(1, "h8 ha eo gu"),(3, "e8 ek ea e7"),(1, "sa eu sz s7"),(2, "g9 g7 ga sk"),(0, "ho h7 g8 hz"),(0, "so hk ez e9"),(0, "s9 s8 gz gk"),],
        [20, 20, -20, -20],
    );
    test_rules(
        "../../testdata/games/16.html",
        &SRulesRufspiel{m_eplayerindex: 3, m_efarbe: EFarbe::Schelln},
        ["ho so gu gk g9 sa s9 s7","ha ea ez ek e9 gz g8 g7","hu hk h9 h7 e8 e7 ga sz","eo go eu su hz h8 sk s8",],
        [(0, "ho ha h7 hz"),(0, "so e9 h9 h8"),(0, "gu g7 hk su"),(0, "sa ek sz s8"),(0, "s9 ez hu sk"),(2, "ga eu gk g8"),(3, "go g9 gz e7"),(3, "eo s7 ea e8"),],
        [90, -90, -90, 90],
    );
    test_rules(
        "../../testdata/games/17.html",
        &SRulesRufspiel{m_eplayerindex: 0, m_efarbe: EFarbe::Schelln},
        ["ho so eu hu h8 ga g8 s8","go ha hz h7 ea ek s9 s7","gu su h9 e9 gz g9 g7 sz","eo hk ez e8 e7 gk sa sk",],
        [(0, "h8 ha su eo"),(3, "hk eu hz h9"),(0, "so go gu e7"),(1, "ea e9 ez hu"),(0, "ho h7 g7 gk"),(0, "ga ek g9 sk"),(0, "s8 s7 sz sa"),(3, "e8 g8 s9 gz"),],
        [30, -30, -30, 30],
    );
    test_rules(
        "../../testdata/games/18.html",
        &SRulesRufspiel{m_eplayerindex: 3, m_efarbe: EFarbe::Gras},
        ["su ha ez e9 e7 gk g9 s8","go gu hu h9 g7 sz s9 s7","eo so eu ek ga gz sa sk","ho hz hk h8 h7 ea e8 g8",],
        [(0, "gk g7 ga g8"),(2, "eo hz su h9"),(2, "so h7 ha go"),(1, "s9 sa e8 s8"),(2, "eu hk g9 hu"),(2, "ek ea e7 gu"),(1, "sz sk h8 e9"),(3, "ho ez s7 gz"),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/19.html",
        &SRulesRufspiel{m_eplayerindex: 2, m_efarbe: EFarbe::Gras},
        ["gu su hz h7 ek ga gk s7","hk ez e9 e8 e7 g7 s9 s8","go so hu ha ea gz sa sk","eo ho eu h9 h8 g9 g8 sz",],
        [(0, "gu hk hu eu"),(3, "g9 ga g7 gz"),(0, "h7 s8 so h8"),(2, "ea h9 ek ez"),(3, "g8 gk s9 ha"),(2, "sa sz s7 e7"),(2, "sk ho su e8"),(3, "eo hz e9 go"),],
        [-20, 20, -20, 20], // TODO: support Kontra
    );
    test_rules(
        "../../testdata/games/20.html",
        &SRulesRufspiel{m_eplayerindex: 0, m_efarbe: EFarbe::Eichel},
        ["ho hu ha h8 ez e9 sa s9","eu h7 e8 gk g9 g7 sk s8","go so gu hz h9 e7 ga g8","eo su hk ea ek gz sz s7",],
        [(0, "h8 h7 gu eo"),(3, "hk hu eu hz"),(1, "e8 e7 ea ez"),(3, "su ho sk go"),(2, "so s7 ha gk"),(2, "ga gz e9 g7"),(2, "g8 sz s9 g9"),(1, "s8 h9 ek sa"),],
        [-30, 30, 30, -30],
    );
    test_rules(
        "../../testdata/games/21.html",
        &SRulesRufspiel{m_eplayerindex: 3, m_efarbe: EFarbe::Schelln},
        ["ho hu su h9 e9 e8 gk sk","ha hz h7 ea ez ga sa s9","so eu gu ek e7 gz g9 s7","eo go hk h8 g8 g7 sz s8",],
        [(0, "sk sa s7 sz"),(1, "ha gu go h9"),(3, "g8 gk ga g9"),(1, "ea e7 g7 e8"),(1, "ez ek eo e9"),(3, "s8 hu s9 gz"),(0, "su hz so h8"),(2, "eu hk ho h7"),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/22.html",
        &SRulesRufspiel{m_eplayerindex: 3, m_efarbe: EFarbe::Gras},
        ["hz h9 ea ek g9 sz sk s9","eo su ha h8 ga gz sa s7","so gu h7 e9 e7 gk g8 s8","go ho eu hu hk ez e8 g7",],
        [(0, "g9 ga g8 g7"),(1, "eo h7 hk h9"),(1, "sa s8 ez s9"),(1, "h8 gu ho hz"),(3, "e8 ea ha e7"),(1, "su so go ek"),(3, "eu sk gz e9"),(3, "hu sz s7 gk"),],
        [-70, 70, -70, 70],
    );
    test_rules(
        "../../testdata/games/23.html",
        &SRulesRufspiel{m_eplayerindex: 1, m_efarbe: EFarbe::Gras},
        ["go gu ga gz g9 sz s9 s8","ho hu hk h8 h7 gk sa s7","eo so eu ha ea e9 e8 sk","su hz h9 ez ek e7 g8 g7",],
        [(0, "go h7 eo hz"),(2, "sk su s8 s7"),(3, "g7 ga gk ha"),(2, "ea e7 gu sa"),(0, "g9 hu e9 g8"),(1, "ho eu h9 gz"),(1, "h8 so ez s9"),(2, "e8 ek sz hk"),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/24.html",
        &SRulesRufspiel{m_eplayerindex: 0, m_efarbe: EFarbe::Eichel},
        ["go so eu ha h9 e9 sa s9","eo ho hz ek e7 g9 g8 g7","gu hk h8 ez gz gk sz sk","hu su h7 ea e8 ga s8 s7",],
        [(0, "h9 ho hk h7"),(1, "ek ez ea e9"),(3, "hu eu eo h8"),(1, "g7 gk ga s9"),(3, "su so hz gu"),(0, "go e7 gz e8"),(0, "ha g8 sk s7"),(0, "sa g9 sz s8"),],
        [30, -30, -30, 30],
    );
    test_rules(
        "../../testdata/games/25.html",
        &SRulesRufspiel{m_eplayerindex: 3, m_efarbe: EFarbe::Eichel},
        ["eo gu hu ez gz g8 g7 sz","su hk h9 ea e7 g9 s9 s8","so eu ha h8 ek e9 sk s7","go ho hz h7 e8 ga gk sa",],
        [(0, "ez ea e9 e8"),(1, "h9 eu h7 hu"),(2, "sk sa sz s8"),(3, "ho eo hk h8"),(0, "gz g9 so ga"),(2, "ek hz gu e7"),(0, "g7 su s7 gk"),(1, "s9 ha go g8"),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/26.html",
        &SRulesRufspiel{m_eplayerindex: 0, m_efarbe: EFarbe::Eichel},
        ["so su ha hz h7 ek e9 sz","go hk h8 e7 gz g9 sa s7","eo eu h9 ez ga gk sk s8","ho gu hu ea e8 g8 g7 s9",],
        [(0, "h7 go h9 hu"),(1, "e7 ez ea ek"),(3, "ho su h8 eo"),(2, "eu gu so hk"),(0, "e9 gz s8 e8"),(0, "sz sa sk s9"),(1, "g9 ga g7 ha"),(0, "hz s7 gk g8"),],
        [20, -20, -20, 20],
    );
    test_rules(
        "../../testdata/games/27.html",
        &SRulesRufspiel{m_eplayerindex: 1, m_efarbe: EFarbe::Gras},
        ["h9 h8 ez e8 g8 sk s9 s8","eo so gu ha hk e9 gk s7","eu hu su hz h7 gz g9 sz","go ho ea ek e7 ga g7 sa",],
        [(0, "g8 gk g9 ga"),(3, "go h8 ha h7"),(3, "ho h9 hk su"),(3, "sa s9 s7 sz"),(3, "ea ez e9 hz"),(2, "eu ek sk so"),(1, "gu hu e7 s8"),(1, "eo gz g7 e8"),],
        [-60, 60, -60, 60],
    );
    test_rules(
        "../../testdata/games/28.html",
        &SRulesRufspiel{m_eplayerindex: 1, m_efarbe: EFarbe::Eichel},
        ["so e9 ga g9 g8 sa sz s9","eo gu hz h9 h8 h7 ez s7","go eu hu su hk ek gk sk","ho ha ea e8 e7 gz g7 s8",],
        [(0, "e9 ez ek ea"),(3, "ho so h7 go"),(2, "sk s8 sa s7"),(0, "ga hz gk gz"),(1, "eo su ha g9"),(1, "h8 hu g7 sz"),(2, "hk e7 s9 h9"),(2, "eu e8 g8 gu"),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/29.html",
        &SRulesRufspiel{m_eplayerindex: 0, m_efarbe: EFarbe::Eichel},
        ["go so eu h9 ez gz sa s9","hu su ha hz e9 e7 g9 s8","hk h8 ek e8 g7 sz sk s7","eo ho gu h7 ea ga gk g8",],
        [(0, "h9 su hk gu"),(3, "eo so hu h8"),(3, "ho eu hz g7"),(3, "h7 go ha s7"),(0, "sa s8 sk gk"),(0, "ez e7 e8 ea"),(3, "ga gz g9 ek"),(3, "g8 s9 e9 sz"),],
        [100, -100, -100, 100],
    );
    test_rules(
        "../../testdata/games/30.html",
        &SRulesRufspiel{m_eplayerindex: 3, m_efarbe: EFarbe::Schelln},
        ["hk h7 ez e9 e7 gk g8 sk","go gu ea ek e8 gz sa s7","ho so eu hz h8 g7 sz s8","eo hu su ha h9 ga g9 s9",],
        [(0, "sk sa s8 s9"),(1, "go h8 ha hk"),(1, "gu eu eo h7"),(3, "su ez e8 so"),(2, "g7 ga g8 gz"),(3, "g9 gk s7 sz"),(0, "e7 ea ho h9"),(2, "hz hu e9 ek"),],
        [-20, 20, -20, 20],
    );
}
