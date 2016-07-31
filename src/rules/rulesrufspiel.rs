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

    fn payout(&self, vecstich: &Vec<SStich>) -> [isize; 4] {
        assert_eq!(vecstich.len(), 8);
        let eplayerindex_coplayer = vecstich.iter()
            .flat_map(|stich| stich.indices_and_cards())
            .find(|&(_, card)| card==self.rufsau())
            .map(|(eplayerindex, _)| eplayerindex)
            .unwrap();
        let n_points_player_party = 
            self.points_per_player(vecstich, self.m_eplayerindex)
            + self.points_per_player(vecstich, eplayerindex_coplayer);
        let b_player_party_wins = n_points_player_party >= 61;
        let eschneiderschwarz = if b_player_party_wins {
            if vecstich.iter().all(|stich| eplayerindex_coplayer==self.winner_index(stich) || self.m_eplayerindex==self.winner_index(stich)) {
                ESchneiderSchwarz::Schwarz
            } else if n_points_player_party>90 {
                ESchneiderSchwarz::Schneider
            } else {
                ESchneiderSchwarz::Nothing
            }
        } else {
            if !vecstich.iter().any(|stich| eplayerindex_coplayer==self.winner_index(stich) || self.m_eplayerindex==self.winner_index(stich)) {
                ESchneiderSchwarz::Schwarz
            } else if n_points_player_party<=30 {
                ESchneiderSchwarz::Schneider
            } else {
                ESchneiderSchwarz::Nothing
            }
        };
        let ab_winner = create_playerindexmap(|eplayerindex| {
            (eplayerindex==self.m_eplayerindex || eplayerindex==eplayerindex_coplayer) == b_player_party_wins
        });
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
        &SRulesRufspiel{m_eplayerindex: 3, m_efarbe: EFarbe::Eichel},
        [
            "eo gu hu ez gz g8 g7 sz",
            "su hk h9 ea e7 g9 s9 s8",
            "so eu ha h8 ek e9 sk s7",
            "go ho hz h7 e8 ga gk sa",
        ],
        [
            (0, "ez ea e9 e8"), 
            (1, "h9 eu h7 hu"), 
            (2, "sk sa sz s8"), 
            (3, "ho eo hk h8"), 
            (0, "gz g9 so ga"), 
            (2, "ek hz gu e7"), 
            (0, "g7 su s7 gk"), 
            (1, "s9 ha go g8"), 
        ],
        [-20, 20, -20, 20],
    );
}
