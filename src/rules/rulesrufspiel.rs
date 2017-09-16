use primitives::*;
use rules::*;
use rules::trumpfdecider::*;
use rules::payoutdecider::*;
use std::fmt;
use std::cmp::Ordering;
use util::*;
use ai::rulespecific::airufspiel::*;

#[derive(Clone, Debug)]
pub struct SRulesRufspiel {
    epi : EPlayerIndex,
    efarbe : EFarbe,
    payoutdecider: SPayoutDeciderPointBased,
}

impl fmt::Display for SRulesRufspiel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rufspiel mit der {}-Sau von {}", self.efarbe, self.epi)
    }
}

pub type STrumpfDeciderRufspiel = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, STrumpfDeciderFarbe<
    SFarbeDesignatorHerz>>>;

impl SRulesRufspiel {
    pub fn new(epi: EPlayerIndex, efarbe: EFarbe, payoutdeciderparams: SPayoutDeciderParams) -> SRulesRufspiel {
        assert_ne!(efarbe, EFarbe::Herz);
        SRulesRufspiel {
            epi,
            efarbe,
            payoutdecider: SPayoutDeciderPointBased::new(payoutdeciderparams, VGameAnnouncementPriority::RufspielLike),
        }
    }

    pub fn rufsau(&self) -> SCard {
        SCard::new(self.efarbe, ESchlag::Ass)
    }

    fn is_ruffarbe(&self, card: SCard) -> bool {
        VTrumpfOrFarbe::Farbe(self.efarbe)==self.trumpforfarbe(card)
    }
}

impl TActivelyPlayableRules for SRulesRufspiel {
    box_clone_impl_by_clone!(TActivelyPlayableRules);
    fn priority(&self) -> VGameAnnouncementPriority {
        assert_eq!(VGameAnnouncementPriority::RufspielLike, self.payoutdecider.priority());
        self.payoutdecider.priority()
    }
}

impl TRules for SRulesRufspiel {
    box_clone_impl_by_clone!(TRules);
    impl_rules_trumpf!(STrumpfDeciderRufspiel);

    fn can_be_played(&self, hand: &SFullHand) -> bool {
        let it = || {hand.get().cards().iter().filter(|&card| self.is_ruffarbe(*card))};
        it().all(|card| card.schlag()!=ESchlag::Ass)
        && 0<it().count()
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.epi)
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool {
        EKurzLang::from_cards_per_player(hand.cards().len());
        assert!(epi!=self.epi || !hand.contains(self.rufsau()));
        (epi==self.epi || hand.contains(self.rufsau())) == (vecstoss.len()%2==1)
    }

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> SAccountBalance {
        let epi_coplayer = gamefinishedstiche.get().iter()
            .flat_map(|stich| stich.iter())
            .find(|&(_, card)| *card==self.rufsau())
            .map(|(epi, _)| epi)
            .unwrap();
        assert_ne!(self.epi, epi_coplayer);
        macro_rules! fn_is_player_party {
            () => {|epi| {
                epi==self.epi || epi==epi_coplayer
            }}
        }
        let an_payout_no_stock = SStossDoublingPayoutDecider::payout(
            self.payoutdecider.payout(
                self,
                gamefinishedstiche,
                fn_is_player_party!(),
                /*fn_player_multiplier*/ |_epi| 1, // everyone pays/gets the same
            ),
            tpln_stoss_doubling,
        );
        assert!(an_payout_no_stock.iter().all(|n_payout_no_stock| 0!=*n_payout_no_stock));
        assert_eq!(an_payout_no_stock[self.epi], an_payout_no_stock[epi_coplayer]);
        assert_eq!(
            an_payout_no_stock.iter()
                .filter(|&n_payout_no_stock| 0<*n_payout_no_stock)
                .count(),
            2
        );
        assert_eq!(n_stock%2, 0);
        let n_stock_per_player = n_stock/2;
        if /*b_player_party_wins*/ 0<an_payout_no_stock[self.epi] {
            SAccountBalance::new(
                EPlayerIndex::map_from_fn(|epi|
                    an_payout_no_stock[epi] + if fn_is_player_party!()(epi) { n_stock_per_player } else {0}
                ),
                -n_stock
            )
        } else {
            SAccountBalance::new(
                EPlayerIndex::map_from_fn(|epi|
                    an_payout_no_stock[epi] - if fn_is_player_party!()(epi) { n_stock_per_player } else {0}
                ),
                n_stock
            )
        }
    }

    fn all_allowed_cards_first_in_stich(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        if // do we already know who had the rufsau?
            !completed_stichs(vecstich).iter()
                .any(|stich| {
                    assert_eq!(stich.size(), 4); // completed_stichs should only process full stichs
                    self.is_ruffarbe(*stich.first()) // gesucht or weggelaufen
                    || stich.iter().any(|(_, card)| *card==self.rufsau()) // We explicitly traverse all cards because it may be allowed (by exotic rules) to schmier rufsau even if not gesucht.
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
        assert!(current_stich(vecstich).size()<4);
        assert!(current_stich(vecstich).current_playerindex().is_some());
        if hand.cards().len()<=1 {
            hand.cards().clone()
        } else {
            let epi = current_stich(vecstich).current_playerindex().unwrap();
            let b_weggelaufen = completed_stichs(vecstich).iter()
                .any(|stich| epi==stich.first_playerindex() && self.is_ruffarbe(*stich.first()));
            let card_first = *current_stich(vecstich).first();
            if self.is_ruffarbe(card_first) && hand.contains(self.rufsau()) && !b_weggelaufen {
                return Some(self.rufsau()).into_iter().collect()
            }
            let veccard_allowed : SHandVector = hand.cards().iter().cloned()
                .filter(|&card| 
                    self.rufsau()!=card 
                    && self.trumpforfarbe(card)==self.trumpforfarbe(card_first)
                )
                .collect();
            if veccard_allowed.is_empty() {
                if b_weggelaufen {
                    hand.cards().iter().cloned().collect()
                } else {
                    hand.cards().iter().cloned().filter(|&card| self.rufsau()!=card).collect()
                }
            } else {
                veccard_allowed
            }
        }
    }

    fn rulespecific_ai<'rules>(&'rules self) -> Option<Box<TRuleSpecificAI + 'rules>> {
        Some(Box::new(SAIRufspiel::new(self)))
    }
}
