use crate::ai::rulespecific::airufspiel::*;
use crate::primitives::*;
use crate::rules::{payoutdecider::*, trumpfdecider::*, *};
use crate::util::*;
use std::{cmp::Ordering, fmt};

pub trait TRufspielPayout : Clone + Sync + fmt::Debug + Send + 'static {
    fn payout(&self, rules: &SRulesRufspielGeneric<Self>, rulestatecache: &SRuleStateCache, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> EnumMap<EPlayerIndex, isize>;
    fn payouthints(&self, rules: &SRulesRufspielGeneric<Self>, rulestatecache: &SRuleStateCache, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>;
}

#[derive(Debug, Clone)]
pub struct SRufspielPayout {
    payoutdecider: SPayoutDeciderPointBased<SPointsToWin61>,
}

fn rufspiel_payout_no_stock_stoss_doubling<RufspielPayout: TRufspielPayout>(payoutdecider: &impl TPayoutDecider<SPlayerParties22>, rules: &SRulesRufspielGeneric<RufspielPayout>, rulestatecache: &SRuleStateCache, gamefinishedstiche: SStichSequenceGameFinished) -> (EnumMap<EPlayerIndex, isize>, SPlayerParties22) {
    let epi_coplayer = debug_verify_eq!(
        rulestatecache.fixed.who_has_card(rules.rufsau()),
        unwrap!(gamefinishedstiche.get().completed_stichs().iter()
            .flat_map(|stich| stich.iter())
            .find(|&(_, card)| *card==rules.rufsau())
            .map(|(epi, _)| epi))
    );
    assert_ne!(rules.epi, epi_coplayer);
    let playerparties = SPlayerParties22{aepi_pri: [rules.epi, epi_coplayer]};
    let an_payout_no_stock = payoutdecider.payout(
        rules,
        rulestatecache,
        gamefinishedstiche,
        &playerparties,
    );
    assert!(an_payout_no_stock.iter().all(|n_payout_no_stock| 0!=*n_payout_no_stock));
    assert_eq!(an_payout_no_stock[rules.epi], an_payout_no_stock[epi_coplayer]);
    assert_eq!(
        an_payout_no_stock.iter()
            .filter(|&n_payout_no_stock| 0<*n_payout_no_stock)
            .count(),
        2
    );
    (an_payout_no_stock, playerparties)
}

fn rufspiel_payouthints_no_stock_stoss_doubling<RufspielPayout: TRufspielPayout>(payoutdecider: &impl TPayoutDecider<SPlayerParties22>, rules: &SRulesRufspielGeneric<RufspielPayout>, rulestatecache: &SRuleStateCache, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
    let epi_coplayer = debug_verify_eq!(
        rulestatecache.fixed.who_has_card(rules.rufsau()),
        stichseq.visible_cards()
            .find(|&(_, card)| *card==rules.rufsau())
            .map(|(epi, _)| epi)
            .unwrap_or_else(|| {
                unwrap!(EPlayerIndex::values().find(|epi|
                    ahand[*epi].cards().iter().any(|card| *card==rules.rufsau())
                ))
            })
    );
    assert_ne!(rules.epi, epi_coplayer);
    payoutdecider.payouthints(rules, rulestatecache, stichseq, ahand, &SPlayerParties22{aepi_pri: [rules.epi, epi_coplayer]})
}

impl TRufspielPayout for SRufspielPayout {
    fn payout(&self, rules: &SRulesRufspielGeneric<Self>, rulestatecache: &SRuleStateCache, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> EnumMap<EPlayerIndex, isize> {
        let (an_payout_no_stock, playerparties) = rufspiel_payout_no_stock_stoss_doubling(
            &self.payoutdecider,
            rules,
            rulestatecache,
            gamefinishedstiche,
        );
        assert_eq!(n_stock%2, 0);
        EPlayerIndex::map_from_fn(|epi|
            payout_including_stoss_doubling(an_payout_no_stock[epi], tpln_stoss_doubling)
                + if playerparties.is_primary_party(epi) {
                    if 0<verify_eq!(an_payout_no_stock[epi], an_payout_no_stock[rules.epi]) {
                        n_stock/2
                    } else {
                        -n_stock/2
                    }
                } else {
                    0
                },
        )
    }
    fn payouthints(&self, rules: &SRulesRufspielGeneric<Self>, rulestatecache: &SRuleStateCache, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        rufspiel_payouthints_no_stock_stoss_doubling(
            &self.payoutdecider,
            rules,
            rulestatecache,
            stichseq,
            ahand,
        ).map(|intvlon_payout| intvlon_payout.map(|on_payout|
            // TODO Stock
            on_payout.map(|n_payout| payout_including_stoss_doubling(n_payout, tpln_stoss_doubling)),
        ))
    }
}

#[derive(Clone, Debug)]
pub struct SRulesRufspielGeneric<RufspielPayout: TRufspielPayout> {
    epi : EPlayerIndex,
    efarbe : EFarbe,
    rufspielpayout: RufspielPayout,
    trumpfdecider: STrumpfDeciderRufspiel,
}

pub type SRulesRufspiel = SRulesRufspielGeneric<SRufspielPayout>;

impl<RufspielPayout: TRufspielPayout> fmt::Display for SRulesRufspielGeneric<RufspielPayout> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Rufspiel mit der {}-Sau", self.efarbe)
    }
}

pub type STrumpfDeciderRufspiel = STrumpfDeciderSchlag<
    SStaticSchlagOber, STrumpfDeciderSchlag<
    SStaticSchlagUnter, 
    SStaticFarbeHerz>>;

impl<RufspielPayout: TRufspielPayout> SRulesRufspielGeneric<RufspielPayout> {
    pub fn new(epi: EPlayerIndex, efarbe: EFarbe, payoutparams: SPayoutDeciderParams) -> SRulesRufspiel {
        SRulesRufspiel {
            epi,
            efarbe: verify_ne!(efarbe, EFarbe::Herz),
            rufspielpayout: SRufspielPayout {
                payoutdecider: SPayoutDeciderPointBased::new(payoutparams, SPointsToWin61{}),
            },
            trumpfdecider: STrumpfDeciderRufspiel::default(),
        }
    }

    pub fn rufsau(&self) -> SCard {
        SCard::new(self.efarbe, ESchlag::Ass)
    }

    fn is_ruffarbe(&self, card: SCard) -> bool {
        VTrumpfOrFarbe::Farbe(self.efarbe)==self.trumpforfarbe(card)
    }
}

impl<RufspielPayout: TRufspielPayout> TActivelyPlayableRules for SRulesRufspielGeneric<RufspielPayout> {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::RufspielLike
    }
}

impl<RufspielPayout: TRufspielPayout> TRulesNoObj for SRulesRufspielGeneric<RufspielPayout> {
    impl_rules_trumpf_noobj!(STrumpfDeciderRufspiel);
}

pub struct SPlayerParties22 {
    aepi_pri: [EPlayerIndex; 2],
}

fn playerparties22_multiplier() -> isize {
    1
}

impl TPlayerParties for SPlayerParties22 {
    fn is_primary_party(&self, epi: EPlayerIndex) -> bool {
        self.aepi_pri[0]==epi || self.aepi_pri[1]==epi
    }
    fn multiplier(&self, _epi: EPlayerIndex) -> isize {
        playerparties22_multiplier()
    }
}

impl<RufspielPayout: TRufspielPayout> TRules for SRulesRufspielGeneric<RufspielPayout> {
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
        assert!(EKurzLang::from_cards_per_player(hand.cards().len()).is_some());
        assert!(epi!=self.epi || !hand.contains(self.rufsau()));
        (epi==self.epi || hand.contains(self.rufsau())) == (vecstoss.len()%2==1)
    }

    fn payout_no_invariant(&self, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize> {
        self.rufspielpayout.payout(
            self,
            rulestatecache,
            gamefinishedstiche,
            tpln_stoss_doubling,
            n_stock,
        )
    }

    fn payouthints(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        self.rufspielpayout.payouthints(
            self,
            rulestatecache,
            stichseq,
            ahand,
            tpln_stoss_doubling,
            n_stock,
        )
    }

    fn equivalent_when_on_same_hand(&self) -> SEnumChains<SCard> {
        use crate::primitives::card_values::*;
        debug_verify_eq!(
            SEnumChains::new_from_slices(&[
                &[EO, GO, HO, SO] as &[SCard],
                &[EU, GU, HU, SU],
                &[H9, H8, H7],
                &[E9, E8, E7],
                &[G9, G8, G7],
                &[S9, S8, S7],
            ]),
            {
                let (mapefarbeveccard, veccard_trumpf) = self.trumpfdecider.equivalent_when_on_same_hand();
                let vecveccard = mapefarbeveccard.into_raw().into_iter().chain(Some(veccard_trumpf).into_iter())
                    .flat_map(|veccard| equivalent_when_on_same_hand_point_based(&veccard))
                    .collect::<Vec<_>>();
                SEnumChains::new_from_slices(
                    &vecveccard.iter()
                        .map(|veccard| veccard as &[SCard]).collect::<Vec<_>>(),
                )
            }
        )
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

    fn points_as_payout(&self) -> Option<(
        Box<dyn TRules>,
        Box<dyn Fn(&SStichSequence, &SHand, f32)->f32>,
    )> {
        #[derive(Debug, Clone)]
        struct SRufspielPayoutPointsAsPayout {
            payoutdecider: SPayoutDeciderPointsAsPayout<SPointsToWin61>,
        }
        impl SRufspielPayoutPointsAsPayout {
            fn payout_to_points(epi_active: EPlayerIndex, card_rufsau: SCard, stichseq: &SStichSequence, hand: &SHand, f_payout: f32) -> f32 {
                let epi = unwrap!(stichseq.current_stich().current_playerindex());
                normalized_points_to_points(
                    f_payout / playerparties22_multiplier().as_num::<f32>(),
                    &SPointsToWin61{},
                    /*b_primary*/ epi==epi_active
                        || stichseq.visible_stichs().iter().filter_map(|stich| stich.get(epi))
                            .chain(hand.cards().iter())
                            .any(|&card| card==card_rufsau),
                )
            }
        }
        impl TRufspielPayout for SRufspielPayoutPointsAsPayout {
            fn payout(&self, rules: &SRulesRufspielGeneric<Self>, rulestatecache: &SRuleStateCache, gamefinishedstiche: SStichSequenceGameFinished, _tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, isize> {
                let (an_payout, if_dbg_else!({playerparties}{_playerparties})) = rufspiel_payout_no_stock_stoss_doubling(
                    &self.payoutdecider,
                    rules,
                    rulestatecache,
                    gamefinishedstiche,
                );
                #[cfg(debug_assertions)] {
                    let mut stichseq_check = SStichSequence::new(gamefinishedstiche.get().kurzlang());
                    let mut ahand_check = EPlayerIndex::map_from_fn(|epi|
                        SHand::new_from_iter(gamefinishedstiche.get().completed_stichs().iter().map(|stich| stich[epi]))
                    );
                    for stich in gamefinishedstiche.get().completed_stichs().iter() {
                        for (epi_card, card) in stich.iter() {
                            let b_primary = playerparties.is_primary_party(epi_card);
                            assert_eq!(
                                Self::payout_to_points(
                                    /*epi_active*/rules.epi,
                                    rules.rufsau(),
                                    &stichseq_check,
                                    &ahand_check[epi_card],
                                    an_payout[epi_card].as_num::<f32>(),
                                ).as_num::<isize>(),
                                EPlayerIndex::values()
                                    .filter(|epi| playerparties.is_primary_party(*epi)==b_primary)
                                    .map(|epi|
                                        rulestatecache.changing.mapepipointstichcount[epi].n_point
                                    )
                                    .sum::<isize>(),
                            );
                            stichseq_check.zugeben_custom_winner_index(*card, |stich| rules.winner_index(stich)); // TODO I could not simply pass rules. Why?
                            ahand_check[epi_card].play_card(*card);
                        }
                    }

                }
                an_payout
            }
            fn payouthints(&self, rules: &SRulesRufspielGeneric<Self>, rulestatecache: &SRuleStateCache, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, _tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
                rufspiel_payouthints_no_stock_stoss_doubling(
                    &self.payoutdecider,
                    rules,
                    rulestatecache,
                    stichseq,
                    ahand,
                )
            }
        }
        let epi_active = self.epi;
        let card_rufsau = self.rufsau();
        Some((
            Box::new(SRulesRufspielGeneric{
                epi: self.epi,
                efarbe: self.efarbe,
                rufspielpayout: SRufspielPayoutPointsAsPayout {
                    payoutdecider: SPayoutDeciderPointsAsPayout::new(SPointsToWin61{}),
                },
                trumpfdecider: self.trumpfdecider.clone(),
            }) as Box<dyn TRules>,
            Box::new(move |stichseq: &SStichSequence, hand: &SHand, f_payout: f32| {
                SRufspielPayoutPointsAsPayout::payout_to_points(
                    epi_active,
                    card_rufsau,
                    stichseq,
                    hand,
                    f_payout,
                )
            }) as Box<dyn Fn(&SStichSequence, &SHand, f32)->f32>,
        ))
    }
}
