use primitives::*;
use rules::*;
use rules::trumpfdecider::*;
use rules::payoutdecider::*;
use std::fmt;
use std::cmp::Ordering;

pub struct SRulesRufspiel {
    pub m_eplayerindex : EPlayerIndex,
    pub m_efarbe : EFarbe, // TODO possibly wrap with ENonHerzFarbe or similar
    pub m_n_payout_base : isize,
    pub m_n_payout_schneider_schwarz : isize,
    pub m_laufendeparams : SLaufendeParams,
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
        VTrumpfOrFarbe::Farbe(self.m_efarbe)==self.trumpforfarbe(card)
    }
}

impl TActivelyPlayableRules for SRulesRufspiel {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::RufspielLike
    }
}

impl TRules for SRulesRufspiel {
    impl_rules_trumpf!(STrumpfDeciderRufspiel);

    fn can_be_played(&self, hand: &SFullHand) -> bool {
        let it = || {hand.get().cards().iter().filter(|&card| self.is_ruffarbe(*card))};
        it().all(|card| card.schlag()!=ESchlag::Ass)
        && 0<it().count()
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.m_eplayerindex)
    }

    fn stoss_allowed(&self, eplayerindex: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool {
        assert_eq!(hand.cards().len(), 8);
        assert!(eplayerindex!=self.m_eplayerindex || !hand.contains(self.rufsau()));
        (eplayerindex==self.m_eplayerindex || hand.contains(self.rufsau())) == (vecstoss.len()%2==1)
    }

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche, n_stoss: usize, n_doubling: usize, n_stock: isize) -> SAccountBalance {
        let eplayerindex_coplayer = gamefinishedstiche.get().iter()
            .flat_map(|stich| stich.iter())
            .find(|&(_, card)| *card==self.rufsau())
            .map(|(eplayerindex, _)| eplayerindex)
            .unwrap();
        assert!(self.m_eplayerindex!=eplayerindex_coplayer, "self.m_eplayerindex==eplayerindex_coplayer=={}", eplayerindex_coplayer);
        macro_rules! fn_is_player_party {
            () => {|eplayerindex| {
                eplayerindex==self.m_eplayerindex || eplayerindex==eplayerindex_coplayer
            }}
        }
        let an_payout_no_stock = SStossDoublingPayoutDecider::payout(
            SPayoutDeciderPointBased::payout(
                self,
                gamefinishedstiche,
                fn_is_player_party!(),
                /*fn_player_multiplier*/ |_eplayerindex| 1, // everyone pays/gets the same
                self.m_n_payout_base,
                self.m_n_payout_schneider_schwarz,
                &self.m_laufendeparams,
            ),
            n_stoss,
            n_doubling,
        );
        assert!(an_payout_no_stock.iter().all(|n_payout_no_stock| 0!=*n_payout_no_stock));
        assert_eq!(an_payout_no_stock[self.m_eplayerindex], an_payout_no_stock[eplayerindex_coplayer]);
        assert_eq!(
            an_payout_no_stock.iter()
                .filter(|&n_payout_no_stock| 0<*n_payout_no_stock)
                .count(),
            2
        );
        assert_eq!(n_stock%2, 0);
        let n_stock_per_player = n_stock/2;
        if /*b_player_party_wins*/ 0<an_payout_no_stock[self.m_eplayerindex] {
            SAccountBalance::new(
                create_playerindexmap(|eplayerindex|
                    an_payout_no_stock[eplayerindex] + if fn_is_player_party!()(eplayerindex) { n_stock_per_player } else {0}
                ),
                -n_stock
            )
        } else {
            SAccountBalance::new(
                create_playerindexmap(|eplayerindex|
                    an_payout_no_stock[eplayerindex] - if fn_is_player_party!()(eplayerindex) { n_stock_per_player } else {0}
                ),
                n_stock
            )
        }
    }

    fn all_allowed_cards_first_in_stich(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        if // do we already know who had the rufsau?
            !completed_stichs(vecstich).iter()
                .fold(/*b_rufsau_known_initial*/false, |b_rufsau_known_before_stich, stich| {
                    assert_eq!(stich.size(), 4); // completed_stichs should only process full stichs
                    if b_rufsau_known_before_stich {
                        // already known
                        true
                    } else if self.is_ruffarbe(*stich.first()) {
                        // gesucht or weggelaufen
                        true
                    } else {
                        // We explicitly traverse all cards because it may be allowed 
                        // (by exotic rules) to schmier rufsau even if not gesucht.
                        stich.iter().any(|(_, card)| *card==self.rufsau())
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

    fn all_allowed_cards_within_stich(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        if hand.cards().len()<=1 {
            hand.cards().clone()
        } else {
            let card_first = *vecstich.last().unwrap().first();
            if self.is_ruffarbe(card_first) && hand.contains(self.rufsau()) {
                // special case: gesucht
                // TODO Rules Consider the following distribution of cards:
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
                        && self.trumpforfarbe(card)==self.trumpforfarbe(card_first)
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

}
