use crate::ai::rulespecific::airufspiel::*;
use crate::primitives::*;
use crate::rules::{payoutdecider::*, trumpfdecider::*, *};
use crate::util::*;
use std::{cmp::Ordering, fmt};

#[derive(Clone, Debug)]
pub struct SRulesRufspiel {
    epi : EPlayerIndex,
    efarbe : EFarbe,
    payoutdecider: SPayoutDeciderPointBased<SPointsToWin61>,
}

impl fmt::Display for SRulesRufspiel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rufspiel mit der {}-Sau", self.efarbe)
    }
}

pub type STrumpfDeciderRufspiel = STrumpfDeciderSchlag<
    SStaticSchlagOber, STrumpfDeciderSchlag<
    SStaticSchlagUnter, 
    SStaticFarbeHerz>>;

impl SRulesRufspiel {
    pub fn new(epi: EPlayerIndex, efarbe: EFarbe, payoutparams: SPayoutDeciderParams) -> SRulesRufspiel {
        assert_ne!(efarbe, EFarbe::Herz);
        SRulesRufspiel {
            epi,
            efarbe,
            payoutdecider: SPayoutDeciderPointBased::new(payoutparams, SPointsToWin61{}),
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
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::RufspielLike
    }
}

impl TRulesNoObj for SRulesRufspiel {
    impl_rules_trumpf_noobj!(STrumpfDeciderRufspiel);
}

struct SPlayerParties22 {
    aepi_pri: [EPlayerIndex; 2],
}

impl TPlayerParties for SPlayerParties22 {
    fn is_primary_party(&self, epi: EPlayerIndex) -> bool {
        self.aepi_pri[0]==epi || self.aepi_pri[1]==epi
    }
    fn multiplier(&self, _epi: EPlayerIndex) -> isize {
        1
    }
}

impl TRules for SRulesRufspiel {
    impl_rules_trumpf!();

    fn can_be_played(&self, hand: SFullHand) -> bool {
        let it = || {hand.get().iter().filter(|&card| self.is_ruffarbe(*card))};
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

    fn payoutinfos2(&self, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize> {
        let epi_coplayer = debug_verify_eq!(
            rulestatecache.fixed.who_has_card(self.rufsau()),
            unwrap!(gamefinishedstiche.get().completed_stichs().iter()
                .flat_map(|stich| stich.iter())
                .find(|&(_, card)| *card==self.rufsau())
                .map(|(epi, _)| epi))
        );
        assert_ne!(self.epi, epi_coplayer);
        let playerparties = SPlayerParties22{aepi_pri: [self.epi, epi_coplayer]};
        let an_payout_no_stock = &self.payoutdecider.payout(
            self,
            rulestatecache,
            gamefinishedstiche,
            &playerparties,
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
        EPlayerIndex::map_from_fn(|epi|
            payout_including_stoss_doubling(an_payout_no_stock[epi], tpln_stoss_doubling)
                + if playerparties.is_primary_party(epi) {
                    if 0<verify_eq!(an_payout_no_stock[epi], an_payout_no_stock[self.epi]) {
                        n_stock/2
                    } else {
                        -n_stock/2
                    }
                } else {
                    0
                },
        )
    }

    fn payouthints2(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), _n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutInterval> {
        let epi_coplayer = debug_verify_eq!(
            rulestatecache.fixed.who_has_card(self.rufsau()),
            stichseq.visible_cards()
                .find(|&(_, card)| *card==self.rufsau())
                .map(|(epi, _)| epi)
                .unwrap_or_else(|| {
                    unwrap!(EPlayerIndex::values().find(|epi|
                        ahand[*epi].cards().iter().any(|card| *card==self.rufsau())
                    ))
                })
        );
        assert_ne!(self.epi, epi_coplayer);
        self.payoutdecider.payouthints(self, stichseq, ahand, rulestatecache, &SPlayerParties22{aepi_pri: [self.epi, epi_coplayer]})
            .map(|tplon_payout| SPayoutInterval::from_raw([
                // TODO Stock
                tplon_payout.0.map(|n_payout| payout_including_stoss_doubling(n_payout, tpln_stoss_doubling)),
                tplon_payout.1.map(|n_payout| payout_including_stoss_doubling(n_payout, tpln_stoss_doubling)),
            ]))
    }

    fn all_allowed_cards_first_in_stich(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        if // do we already know who had the rufsau?
            stichseq.completed_stichs().iter()
                .any(|stich| {
                    assert!(stich.is_full()); // completed_stichs should only process full stichs
                    self.is_ruffarbe(*stich.first()) // gesucht or weggelaufen
                    || stich.iter().any(|(_, card)| *card==self.rufsau()) // We explicitly traverse all cards because it may be allowed (by exotic rules) to schmier rufsau even if not gesucht.
                } )
            // Remark: Player must have 4 cards of ruffarbe on his hand *at this point of time* (i.e. not only at the beginning!)
            || !hand.contains(self.rufsau())
            || 4 <= hand.cards().iter()
                .filter(|&card| self.is_ruffarbe(*card))
                .count()
        {
            hand.cards().clone()
        } else {
            hand.cards().iter()
                .copied()
                .filter(|&card| !self.is_ruffarbe(card) || self.rufsau()==card)
                .collect()
        }
    }

    fn all_allowed_cards_within_stich(&self, stichseq: &SStichSequence, hand: &SHand) -> SHandVector {
        if hand.cards().len()<=1 {
            hand.cards().clone()
        } else {
            assert!(!stichseq.current_stich().is_empty());
            let epi = unwrap!(stichseq.current_stich().current_playerindex());
            let card_first = *stichseq.current_stich().first();
            if /*either weggelaufen or epi is not partner, and accordingly does not hold rufsau*/stichseq.completed_stichs().iter()
                .any(|stich| epi==stich.first_playerindex() && self.is_ruffarbe(*stich.first()))
            {
                all_allowed_cards_within_stich_distinguish_farbe_frei(
                    self,
                    card_first,
                    hand,
                    /*fn_farbe_not_frei*/|veccard_same_farbe_nonempty| veccard_same_farbe_nonempty,
                )
            } else if self.is_ruffarbe(card_first) && hand.contains(self.rufsau()) {
                std::iter::once(self.rufsau()).collect()
            } else {
                let veccard_allowed : SHandVector = hand.cards().iter().copied()
                    .filter(|&card| 
                        self.rufsau()!=card 
                        && self.trumpforfarbe(card)==self.trumpforfarbe(card_first)
                    )
                    .collect();
                if veccard_allowed.is_empty() {
                    hand.cards().iter().copied().filter(|&card| self.rufsau()!=card).collect()
                } else {
                    veccard_allowed
                }
            }
        }
    }

    fn rulespecific_ai<'rules>(&'rules self) -> Option<Box<dyn TRuleSpecificAI + 'rules>> {
        Some(Box::new(SAIRufspiel::new(self)))
    }
}
