use crate::{
    game::SStichSequence,
    primitives::{
        card::SCard,
        eplayerindex::EPlayerIndex,
        hand::{SHand, SHandVector},
        stich::SStich,
    },
    rules::{
        SPlayerPartiesTable,
        card_points::points_card,
        TRules,
    },
    util::*,
    ai::{
        SRuleStateCacheFixed,
        TFilterAllowedCards,
        cardspartition::*,
    },
};
use arrayvec::ArrayVec;
use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct SStichTrie {
    vectplcardtrie: Box<ArrayVec<(SCard, SStichTrie), 8>>, // TODO? improve
}

impl SStichTrie {
    fn new() -> Self {
        Self {
            vectplcardtrie: Box::new(ArrayVec::new()),
        }
    }

    fn traverse_trie(&self, epi_first: EPlayerIndex) -> Vec<SStich> {
        fn internal_traverse_trie(stichtrie: &SStichTrie, stich: &mut SStich) -> Vec<SStich> {
            if verify_eq!(stich.is_full(), stichtrie.vectplcardtrie.is_empty()) {
                vec![stich.clone()]
            } else {
                let mut vecstich = Vec::new();
                for (card, stichtrie_child) in stichtrie.vectplcardtrie.iter() {
                    stich.push(*card);
                    vecstich.extend(internal_traverse_trie(stichtrie_child, stich));
                    stich.undo_most_recent();
                }
                debug_assert!(vecstich.iter().all_unique());
                vecstich
            }
        }
        internal_traverse_trie(self, &mut SStich::new(epi_first))
    }

    pub fn new_with(
        ahand: &mut EnumMap<EPlayerIndex, SHand>,
        stichseq: &mut SStichSequence,
        rules: &dyn TRules,
        cardspartition_completed_cards: &SCardsPartition,
        playerparties: &SPlayerPartiesTable,
    ) -> Self {
        fn for_each_allowed_card(
            n_depth: usize, // TODO? static enum type, possibly difference of EPlayerIndex
            ahand: &mut EnumMap<EPlayerIndex, SHand>,
            stichseq: &mut SStichSequence,
            rules: &dyn TRules,
            cardspartition_completed_cards: &SCardsPartition,
            playerparties: &SPlayerPartiesTable,
        ) -> (SStichTrie, Option<bool/*b_stich_winner_primary_party*/>) {
            if n_depth==0 {
                assert!(stichseq.current_stich().is_empty());
                let stich = unwrap!(stichseq.completed_stichs().last());
                assert!(stich.is_full());
                (
                    SStichTrie::new(),
                    Some(playerparties.is_primary_party(rules.winner_index(stich))),
                )
            } else {
                let epi_card = unwrap!(stichseq.current_stich().current_playerindex());
                let mut veccard_allowed = rules.all_allowed_cards(
                    stichseq,
                    &ahand[epi_card],
                );
                assert!(!veccard_allowed.is_empty());
                enum VStichWinnerPrimaryParty {
                    NotYetAssigned,
                    Same(bool),
                    Different,
                }
                let mut stichwinnerprimaryparty = VStichWinnerPrimaryParty::NotYetAssigned;
                let mut stichtrie = SStichTrie::new();
                while !veccard_allowed.is_empty() {
                    let card_representative = veccard_allowed[0];
                    let (stichtrie_representative, ob_stich_winner_primary_party_representative) = stichseq.zugeben_and_restore(card_representative, rules, |stichseq| {
                        ahand[epi_card].play_card(card_representative);
                        let tplstichtrieob_stich_winner_primary_party = for_each_allowed_card(
                            n_depth-1,
                            ahand,
                            stichseq,
                            rules,
                            cardspartition_completed_cards,
                            playerparties,
                        );
                        ahand[epi_card].add_card(card_representative);
                        tplstichtrieob_stich_winner_primary_party
                    });
                    let mut cardspartition_actual = cardspartition_completed_cards.clone(); // TODO avoid cloning.
                    let epi_preliminary_winner = rules.preliminary_winner_index(stichseq.current_stich());
                    for (epi, card) in stichseq.current_stich().iter() {
                        if epi!=epi_preliminary_winner {
                            cardspartition_actual.remove_from_chain(*card);
                        }
                    }
                    fn iterate_chain(
                        cardspartition: &SCardsPartition,
                        veccard: &mut SHandVector,
                        card_representative: SCard,
                        mut func: impl FnMut(SCard),
                    ) {
                        // TODO avoid backward-forward iteration
                        let mut card_chain = cardspartition.prev_while_contained(card_representative, veccard);
                        veccard.must_find_swap_remove(&card_chain);
                        func(card_chain);
                        while let Some(card_chain_next) = cardspartition.next(card_chain)
                            .filter(|card| veccard.contains(card))
                        {
                            card_chain = card_chain_next;
                            veccard.must_find_swap_remove(&card_chain);
                            func(card_chain);
                        }
                    }
                    match ob_stich_winner_primary_party_representative {
                        None => {
                            let mut ab_points = [false; 12]; // TODO? couple with points_card
                            iterate_chain(&cardspartition_actual, &mut veccard_allowed, card_representative, |card_chain| {
                                if !ab_points[points_card(card_chain).as_num::<usize>()] {
                                    ab_points[points_card(card_chain).as_num::<usize>()]=true;
                                    stichtrie.vectplcardtrie.push((
                                        card_chain,
                                        stichtrie_representative.clone(),
                                    ));
                                }
                            });
                            stichwinnerprimaryparty = VStichWinnerPrimaryParty::Different;
                        },
                        Some(b_stich_winner_primary_party) => {
                            macro_rules! card_min_or_max(($fn_assign_by:expr) => {{
                                let card_chain = cardspartition_actual
                                    .prev_while_contained(card_representative, &veccard_allowed);
                                let mut card_min_or_max = card_chain;
                                iterate_chain(&cardspartition_actual, &mut veccard_allowed, card_representative, |card_chain| {
                                    $fn_assign_by(
                                        &mut card_min_or_max,
                                        card_chain,
                                        |card| points_card(*card),
                                    );
                                });
                                card_min_or_max
                            }});
                            stichtrie.vectplcardtrie.push((
                                if b_stich_winner_primary_party==playerparties.is_primary_party(epi_card) {
                                    card_min_or_max!(assign_max_by_key)
                                } else {
                                    card_min_or_max!(assign_min_by_key)
                                },
                                stichtrie_representative,
                            ));
                            use VStichWinnerPrimaryParty::*;
                            match &stichwinnerprimaryparty {
                                NotYetAssigned => {
                                    stichwinnerprimaryparty = Same(b_stich_winner_primary_party);
                                },
                                Same(b_stich_winner_primary_party_prev) => {
                                    if b_stich_winner_primary_party!=*b_stich_winner_primary_party_prev {
                                        stichwinnerprimaryparty = Different;
                                    }
                                },
                                Different => {/*stay different*/}
                            }
                        },
                    }
                }
                (
                    stichtrie,
                    match stichwinnerprimaryparty {
                        VStichWinnerPrimaryParty::NotYetAssigned => panic!(),
                        VStichWinnerPrimaryParty::Same(b_stich_winner_primary_party) => Some(b_stich_winner_primary_party),
                        VStichWinnerPrimaryParty::Different => None,
                    },
                )
            }
        }
        let stich_current = stichseq.current_stich().clone();
        let n_stich_size = stich_current.size();
        debug_assert_eq!(
            cardspartition_completed_cards,
            &{
                let mut cardspartition_check = unwrap!(rules.only_minmax_points_when_on_same_hand(
                    &SRuleStateCacheFixed::new(stichseq, ahand),
                )).0;
                for (_epi, &card) in stichseq.completed_cards() {
                    cardspartition_check.remove_from_chain(card);
                }
                cardspartition_check
            }
        );
        let mut make_stichtrie = || for_each_allowed_card(
            4-n_stich_size,
            ahand,
            stichseq,
            rules,
            cardspartition_completed_cards,
            playerparties,
        ).0;
        let make_singleton_stichtrie = |i_epi_offset, stichtrie| {
            let mut vectplcardtrie = Box::new(ArrayVec::new());
            vectplcardtrie.push((
                stich_current[stich_current.first_playerindex().wrapping_add(i_epi_offset)],
                stichtrie
            ));
            SStichTrie {
                vectplcardtrie
            }
        };
        let stichtrie = match n_stich_size {
            0 => make_stichtrie(),
            1 => make_singleton_stichtrie(0, make_stichtrie()),
            2 => make_singleton_stichtrie(0, make_singleton_stichtrie(1, make_stichtrie())),
            n_stich_size => {
                assert_eq!(n_stich_size, 3);
                make_singleton_stichtrie(0, make_singleton_stichtrie(1, make_singleton_stichtrie(2, make_stichtrie())))
            },
        };
        debug_assert!(stichtrie.traverse_trie(stichseq.current_stich().first_playerindex()).iter().all(|stich|
            stich.equal_up_to_size(&stich_current, stich_current.size())
        ));
        stichtrie
    }
}

#[derive(Debug)]
pub struct SFilterByOracle<'rules> {
    rules: &'rules dyn TRules,
    ahand: EnumMap<EPlayerIndex, SHand>,
    stichseq: SStichSequence,
    stichtrie: SStichTrie,
    cardspartition_completed_cards: SCardsPartition,
    playerparties: SPlayerPartiesTable,
}

impl<'rules> SFilterByOracle<'rules> {
    pub fn new(
        rules: &'rules dyn TRules,
        ahand_in_game: &EnumMap<EPlayerIndex, SHand>,
        stichseq_in_game: &SStichSequence,
    ) -> Option<Self> {
        let ahand = EPlayerIndex::map_from_fn(|epi| SHand::new_from_iter(
            stichseq_in_game.cards_from_player(&ahand_in_game[epi], epi)
        ));
        assert!(crate::ai::ahand_vecstich_card_count_is_compatible(stichseq_in_game, ahand_in_game));
        let stichseq = SStichSequence::new(stichseq_in_game.kurzlang());
        assert!(crate::ai::ahand_vecstich_card_count_is_compatible(&stichseq, &ahand));
        rules.only_minmax_points_when_on_same_hand(
            &verify_eq!(
                SRuleStateCacheFixed::new(stichseq_in_game, ahand_in_game),
                SRuleStateCacheFixed::new(&stichseq, &ahand)
            )
        ).map(|(cardspartition, playerparties)| {
            let mut slf = Self {
                rules,
                ahand,
                stichseq,
                stichtrie: SStichTrie::new(), // TODO this is a dummy value that should not be needed. Eliminate it.
                cardspartition_completed_cards: cardspartition,
                playerparties,
            };
            for stich in stichseq_in_game.completed_stichs() {
                slf.internal_register_stich(stich);
            }
            let stichtrie = SStichTrie::new_with(
                &mut ahand_in_game.clone(),
                &mut stichseq_in_game.clone(),
                rules,
                &slf.cardspartition_completed_cards,
                &slf.playerparties,
            );
            slf.stichtrie = stichtrie;
            slf
        })
    }

    fn internal_register_stich(&mut self, stich: &SStich) -> <Self as TFilterAllowedCards>::UnregisterStich {
        assert!(stich.is_full());
        for (epi, card) in stich.iter() {
            self.stichseq.zugeben(*card, self.rules);
            self.ahand[epi].play_card(*card);
        }
        let aremovedcard = EPlayerIndex::map_from_fn(|epi|
            self.cardspartition_completed_cards.remove_from_chain(stich[epi])
        );
        let stichtrie = SStichTrie::new_with(
            &mut self.ahand,
            &mut self.stichseq,
            self.rules,
            &self.cardspartition_completed_cards,
            &self.playerparties,
        );
        (std::mem::replace(&mut self.stichtrie, stichtrie), aremovedcard)
    }
}

impl<'rules> TFilterAllowedCards for SFilterByOracle<'rules> {
    type UnregisterStich = (SStichTrie, EnumMap<EPlayerIndex, SRemoved>);
    fn register_stich(&mut self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>) -> Self::UnregisterStich {
        debug_assert!(stichseq.current_stich().is_empty());
        let unregisterstich = self.internal_register_stich(unwrap!(stichseq.completed_stichs().last()));
        assert_eq!(&self.stichseq, stichseq);
        assert_eq!(&self.ahand, ahand);
        unregisterstich
    }
    fn unregister_stich(&mut self, (stichtrie, aremovedcard): Self::UnregisterStich) {
        let stich_last_completed = unwrap!(self.stichseq.completed_stichs().last());
        for epi in EPlayerIndex::values() {
            self.ahand[epi].add_card(stich_last_completed[epi]);
        }
        for _ in 0..EPlayerIndex::SIZE {
            self.stichseq.undo_most_recent();
        }
        for removedcard in aremovedcard.into_raw().into_iter().rev() {
            self.cardspartition_completed_cards.readd(removedcard);
        }
        self.stichtrie = stichtrie;
    }
    fn filter_allowed_cards(&self, stichseq: &SStichSequence, veccard: &mut SHandVector) {
        let mut stichtrie = &self.stichtrie;
        for (_epi, card) in stichseq./*TODO current_playable_stich*/current_stich().iter() {
            stichtrie = &unwrap!(stichtrie.vectplcardtrie.iter().find(|(card_stichtrie, _stichtrie)| card_stichtrie==card)).1;
        }
        *veccard = stichtrie.vectplcardtrie.iter().map(|(card, _stichtrie)| *card).collect();
    }
    fn continue_with_filter(&self, stichseq: &SStichSequence) -> bool {
        stichseq.completed_stichs().len()<=5 // seems to be the best choice when starting after stich 1
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        game::{SGame, SStichSequence},
        player::{
            TPlayer,
            playerrandom::SPlayerRandom,
        },
        primitives::{
            card::{
                card_values::*,
                EFarbe,
                EKurzLang,
                SCard,
            },
            eplayerindex::EPlayerIndex,
            hand::SHand,
            stich::SStich,
        },
        rules::{
            payoutdecider::{SPayoutDeciderParams, SPayoutDeciderPointBased, SLaufendeParams},
            rulesrufspiel::SRulesRufspiel,
            rulessolo::{sololike, ESoloLike},
            ruleset::{
                SRuleSet,
                allowed_rules,
                VStockOrT,
            },
            tests::TPayoutDeciderSoloLikeDefault,
            TRules,
            TRulesBoxClone,
        },
        util::*,
        ai::{
            gametree::EMinMaxStrategy,
            determine_best_card,
            SMinReachablePayout,
            SNoFilter,
            SNoVisualization,
            SRuleStateCacheFixed,
            SSnapshotCacheNone,
        },
        display_card_slices,
    };
    use super::{SStichTrie, SFilterByOracle};
    use itertools::Itertools;

    #[test]
    fn test_stichoracle() {
        let rules_rufspiel_eichel_epi0 = SRulesRufspiel::new(
            EPlayerIndex::EPI0,
            EFarbe::Eichel,
            SPayoutDeciderParams::new(
                /*n_payout_base*/10,
                /*n_payout_schneider_schwarz*/10,
                SLaufendeParams::new(
                    /*n_payout_per_lauf*/10,
                    /*n_lauf_lbound*/3,
                ),
            ),
        );
        fn assert_stichoracle(
            rules: &dyn TRules,
            aslccard_hand: [&[SCard]; EPlayerIndex::SIZE],
            slccard_stichseq: &[SCard],
            slcacard_stich: &[[SCard; EPlayerIndex::SIZE]],
        ) {
            let stichseq = SStichSequence::new_from_cards(
                EKurzLang::Lang,
                slccard_stichseq.iter().copied(),
                rules,
            );
            let epi_first = stichseq.current_stich().first_playerindex();
            let ahand = &EPlayerIndex::map_from_raw(aslccard_hand)
                .map_into(SHand::new_from_iter);
            let (mut cardspartition, playerparties) = unwrap!(rules.only_minmax_points_when_on_same_hand(
                &SRuleStateCacheFixed::new(&stichseq, ahand),
            ));
            for (_epi, card) in stichseq.completed_cards() {
                cardspartition.remove_from_chain(*card);
            }
            let stichtrie = SStichTrie::new_with(
                &mut ahand.clone(),
                &mut stichseq.clone(),
                rules,
                &cardspartition,
                &playerparties,
            );
            let setstich_oracle = stichtrie.traverse_trie(stichseq.current_stich().first_playerindex()).iter().cloned().collect::<std::collections::HashSet<_>>();
            let setstich_check = slcacard_stich
                .iter()
                .map(|acard| SStich::new_full(
                    epi_first,
                    acard.explicit_clone(),
                ))
                .collect::<std::collections::HashSet<_>>();
            //assert_eq!(setstich_oracle.len(), setstich_check.len());
            let internal_assert = |setstich: &std::collections::HashSet<SStich>, stich, str_msg| {
                assert!(
                    setstich.contains(stich),
                    "\nRules:{} von {}\nHands:\n {}\nStichseq: {}\nStich{}\n{}\n",
                    rules,
                    unwrap!(rules.playerindex()),
                    display_card_slices(ahand, &rules, "\n "),
                    stichseq.visible_stichs().iter().join(", "),
                    stich,
                    str_msg,
                );
            };
            for stich_oracle in setstich_oracle.iter() {
                internal_assert(&setstich_check, stich_oracle, "setstich_check missing stich");
            }
            for stich_check in setstich_check.iter() {
                internal_assert(&setstich_oracle, stich_check, "setstich_oracle missing stich");
            }
            assert_eq!(setstich_oracle, setstich_check);
        }
        assert_stichoracle(
            &rules_rufspiel_eichel_epi0,
            [
                &[HO,SO,GU,SU,EK,GA,S9,S7],
                &[GO,HK,H8,H7,EA,SA,SK,S8],
                &[EU,HU,HA,EZ,E7,GZ,G9,G8],
                &[EO,HZ,H9,E9,E8,GK,G7,SZ],
            ],
            &[],
            &[
                [HO, GO, EU, EO],
                [HO, GO, EU, H9],
                [HO, GO, EU, HZ],
                [HO, GO, HA, EO],
                [HO, GO, HA, H9],
                [HO, GO, HA, HZ],
                [HO, GO, HU, EO],
                [HO, GO, HU, H9],
                [HO, GO, HU, HZ],
                [HO, H8, EU, EO],
                [HO, H8, EU, H9],
                [HO, H8, EU, HZ],
                [HO, H8, HA, EO],
                [HO, H8, HA, H9],
                [HO, H8, HA, HZ],
                [HO, H8, HU, EO],
                [HO, H8, HU, H9],
                [HO, H8, HU, HZ],
                [HO, HK, EU, EO],
                [HO, HK, EU, H9],
                [HO, HK, HA, EO],
                [HO, HK, HA, H9],
                [HO, HK, HU, EO],
                [HO, HK, HU, H9],
                // [HO, H7, __, __] // covered by [HO, H8, __, __]
                // [HO, HK, EU, HZ] // covered by [HO, HK, EU, H9]
                // [HO, HK, HA, HZ] // covered by [HO, HK, HA, H9]
                // [HO, HK, HU, HZ] // covered by [HO, HK, HU, H9]
                // [SO, __, __, __] // covered by [HO, __, __, __]
                [GU, GO, EU, EO],
                [GU, GO, EU, H9],
                [GU, GO, EU, HZ],
                [GU, GO, HA, EO],
                [GU, GO, HA, H9],
                [GU, GO, HA, HZ],
                [GU, H8, EU, EO],
                [GU, H8, EU, H9],
                [GU, H8, EU, HZ],
                [GU, H8, HA, EO],
                [GU, H8, HA, H9],
                [GU, H8, HA, HZ],
                [GU, H8, HU, EO],
                [GU, H8, HU, H9],
                [GU, H8, HU, HZ],
                [GU, HK, EU, EO],
                [GU, HK, EU, HZ],
                [GU, HK, HA, EO],
                [GU, HK, HA, H9],
                [GU, HK, HU, EO],
                [GU, HK, HU, H9],
                // [GU, H7, __, __] // covered by [GU, H8, __, __]
                // [SU, H7, __, __] // covered by [SU, H8, __, __]
                // [GU, GO, HU, __] // covered by [GU, GO, EU, __]
                // [GU, HK, EU, H9] // covered by [GU, HK, EU, HZ]
                // [GU, HK, HA, HZ] // covered by [GU, HK, HA, H9]
                // [GU, HK, HU, HZ] // covered by [GU, HK, HU, H9]
                [SU, GO, EU, EO],
                [SU, GO, EU, H9],
                [SU, GO, EU, HZ],
                [SU, GO, HA, EO],
                [SU, GO, HA, H9],
                [SU, GO, HA, HZ],
                [SU, GO, HU, EO], // TODO should not be needed (HA better than HU)
                [SU, GO, HU, H9],
                [SU, GO, HU, HZ],
                [SU, H8, EU, EO],
                [SU, H8, EU, H9],
                [SU, H8, EU, HZ],
                [SU, H8, HA, EO],
                [SU, H8, HA, H9],
                [SU, H8, HA, HZ],
                [SU, H8, HU, EO], // TODO should not be needed (HA better than HU)
                [SU, H8, HU, H9],
                [SU, H8, HU, HZ],
                [SU, HK, EU, EO],
                [SU, HK, EU, HZ],
                [SU, HK, HA, EO],
                [SU, HK, HA, H9],
                [SU, HK, HU, EO], // TODO should not be needed (HA better than HU)
                [SU, HK, HU, HZ],
                [EK, EA, EZ, E9],
                [EK, EA, E7, E9],
                // [EK, EA, EZ, E8] // covered by [EK, EA, EZ, E9]
                // [EK, EA, E7, E8] // covered by [EK, EA, E7, E9]
                [GA, GO, G9, G7], 
                [GA, GO, G9, GK], 
                [GA, GO, GZ, G7], 
                [GA, GO, GZ, GK], 
                [GA, HK, G9, G7], 
                [GA, HK, G9, GK], 
                [GA, HK, GZ, G7], 
                [GA, HK, GZ, GK], 
                [GA, H8, G9, G7], 
                [GA, H8, G9, GK], 
                [GA, H8, GZ, G7], 
                [GA, H8, GZ, GK], 
                [GA, SA, G9, G7], 
                [GA, SA, G9, GK], 
                [GA, SA, GZ, G7], 
                [GA, SA, GZ, GK], 
                [GA, SK, G9, G7], 
                [GA, SK, G9, GK], 
                [GA, SK, GZ, G7], 
                [GA, SK, GZ, GK], 
                [GA, S8, G9, G7],
                [GA, S8, G9, GK],
                [GA, S8, GZ, G7],
                [GA, S8, GZ, GK],
                // [GA, __, G8, __] // covered by [GA, __, G9, __]
                // [GA, H7, __, __] // covered by [GA, H8, __, __]
                [S9, S8, EU, SZ], 
                [S9, S8, HU, SZ], 
                [S9, S8, HA, SZ], 
                [S9, S8, EZ, SZ], 
                [S9, S8, E7, SZ], 
                [S9, S8, GZ, SZ], 
                [S9, S8, G9, SZ], 
                [S9, SA, EU, SZ], 
                [S9, SA, HU, SZ], 
                [S9, SA, HA, SZ], 
                [S9, SA, EZ, SZ], 
                [S9, SA, E7, SZ], 
                [S9, SA, GZ, SZ], 
                [S9, SA, G9, SZ], 
                [S9, SK, EU, SZ], // TODO should not be needed (S8 better than SK)
                [S9, SK, HU, SZ], // TODO should not be needed (S8 better than SK)
                [S9, SK, HA, SZ], // TODO should not be needed (S8 better than SK)
                [S9, SK, EZ, SZ], // TODO should not be needed (S8 better than SK)
                [S9, SK, E7, SZ], // TODO should not be needed (S8 better than SK)
                [S9, SK, GZ, SZ], // TODO should not be needed (S8 better than SK)
                [S9, SK, G9, SZ], // TODO should not be needed (S8 better than SK)
                [S7, S8, EU, SZ],
                [S7, S8, HU, SZ],
                [S7, S8, HA, SZ],
                [S7, S8, EZ, SZ],
                [S7, S8, E7, SZ],
                [S7, S8, GZ, SZ],
                [S7, S8, G9, SZ],
                [S7, SA, EU, SZ], 
                [S7, SA, HU, SZ], 
                [S7, SA, HA, SZ], 
                [S7, SA, EZ, SZ], 
                [S7, SA, E7, SZ], 
                [S7, SA, GZ, SZ], 
                [S7, SA, G9, SZ], 
                [S7, SK, EU, SZ], 
                [S7, SK, HU, SZ], 
                [S7, SK, HA, SZ], 
                [S7, SK, EZ, SZ], 
                [S7, SK, E7, SZ], 
                [S7, SK, GZ, SZ], 
                [S7, SK, G9, SZ], 
                // [S_, __, G8, SZ] // covered by [S_, __, G9, SZ]
            ]
        );
        assert_stichoracle(
            &rules_rufspiel_eichel_epi0,
            [
                &[SO,GU,SU,EK,GA,S9,S7],
                &[GO,HK,H8,H7,EA,SA,SK,S8],
                &[EU,HU,HA,EZ,E7,GZ,G9,G8],
                &[EO,HZ,H9,E9,E8,GK,G7,SZ],
            ],
            &[HO],
            &[
                [HO, GO, EU, EO],
                [HO, GO, EU, H9],
                [HO, GO, EU, HZ],
                [HO, GO, HA, EO],
                [HO, GO, HA, H9],
                [HO, GO, HA, HZ],
                [HO, GO, HU, EO],
                [HO, GO, HU, H9],
                [HO, GO, HU, HZ],
                [HO, H8, EU, EO],
                [HO, H8, EU, H9],
                [HO, H8, EU, HZ],
                [HO, H8, HA, EO],
                [HO, H8, HA, H9],
                [HO, H8, HA, HZ],
                [HO, H8, HU, EO],
                [HO, H8, HU, H9],
                [HO, H8, HU, HZ],
                [HO, HK, EU, EO],
                [HO, HK, EU, H9],
                [HO, HK, HA, EO],
                [HO, HK, HA, H9],
                [HO, HK, HU, EO],
                [HO, HK, HU, H9],
                // [HO, H7, __, __] // covered by [HO, H8, __, __]
                // [HO, HK, EU, HZ] // covered by [HO, HK, EU, H9]
                // [HO, HK, HA, HZ] // covered by [HO, HK, HA, H9]
                // [HO, HK, HU, HZ] // covered by [HO, HK, HU, H9]
            ],
        );
        assert_stichoracle(
            &rules_rufspiel_eichel_epi0,
            [
                &[SO,GU,SU,EK,GA,S9,S7],
                &[HK,H8,H7,EA,SA,SK,S8],
                &[EU,HU,HA,EZ,E7,GZ,G9,G8],
                &[EO,HZ,H9,E9,E8,GK,G7,SZ],
            ],
            &[HO, GO],
            &[
                [HO, GO, EU, EO], 
                [HO, GO, HU, EO], 
                [HO, GO, HA, EO],
                [HO, GO, EU, HZ], 
                [HO, GO, HU, HZ], 
                [HO, GO, HA, HZ],
                [HO, GO, EU, H9], 
                [HO, GO, HU, H9], 
                [HO, GO, HA, H9],
            ],
        );
        assert_stichoracle(
            &rules_rufspiel_eichel_epi0,
            [
                &[SO,GU,SU,EK,GA,S9,S7],
                &[HK,H8,H7,EA,SA,SK,S8],
                &[HU,HA,EZ,E7,GZ,G9,G8],
                &[EO,HZ,H9,E9,E8,GK,G7,SZ],
            ],
            &[HO, GO, EU],
            &[
                [HO, GO, EU, EO], 
                [HO, GO, EU, HZ],
                [HO, GO, EU, H9]
            ],
        );
        assert_stichoracle(
            &rules_rufspiel_eichel_epi0,
            [
                &[EO,HO,SO,GU,EK,S9,S7],
                &[HK,H8,H7,EA,SA,SK,S8],
                &[HU,HA,HZ,EZ,E7,G9,G8],
                &[E9,E8,GA,GZ,GK,G7,SZ],
            ],
            &[SU, GO, EU, H9],
            &[
                [HK, HU, E9, EO],
                [HK, HU, G7, EO],
                [HK, HU, GK, EO],
                [HK, HU, SZ, EO],
                // [H7, __, __, __] // covered by [HK, __, __, __]
                // [H8, __, __, __] // covered by [HK, __, __, __]
                // [__, __, E8, __] // covered by [__, __, E9, __]
                // [HK, HA, __, __] // covered by [HK, HU, __, __]
                // [HK, HZ, __, __] // covered by [HK, HU, __, __]
                // [HK, __, __, GU] // covered by [HK, __, __, EO]
                // [HK, __, __, HO] // covered by [HK, __, __, EO]
                // [HK, __, __, SO] // covered by [HK, __, __, EO]
                // [HK, HU, GA, EO] // covered by [HK, HU, GK, EO]
                // [HK, HU, GZ, EO] // covered by [HK, HU, GK, EO]
                [EA, EZ, E9, EK],
                [EA, E7, E9, EK],
                // [EA, __, E8, __] // covered by [EA, __, E9, __]
                [SA, HA, SZ, S9],
                [SA, HA, SZ, S7],
                [SA, EZ, SZ, S9],
                [SA, EZ, SZ, S7],
                [SA, E7, SZ, S9],
                [SA, E7, SZ, S7],
                [SA, G9, SZ, S9],
                [SA, G9, SZ, S7],
                [SK, HA, SZ, S9],
                [SK, HA, SZ, S7],
                [SK, EZ, SZ, S9],
                [SK, EZ, SZ, S7],
                [SK, E7, SZ, S9],
                [SK, E7, SZ, S7],
                [SK, G9, SZ, S9],
                [SK, G9, SZ, S7],
                [S8, HA, SZ, S9],
                [S8, EZ, SZ, S9],
                [S8, E7, SZ, S9],
                [S8, G9, SZ, S9],
                // [S_, HU, __, __] // covered by [S_, HA, __, __]
                // [S_, HZ, __, __] // covered by [S_, HA, __, __]
                // [S_, G8, __, __] // covered by [S_, G9, __, __]
                // [S8, __, SZ, S7] // covered by [S8, __, SZ, S9]
            ],
        );
        let rules_rufspiel_gras_epi3 = SRulesRufspiel::new(
            EPlayerIndex::EPI3,
            EFarbe::Gras,
            SPayoutDeciderParams::new(
                /*n_payout_base*/10,
                /*n_payout_schneider_schwarz*/10,
                SLaufendeParams::new(
                    /*n_payout_per_lauf*/10,
                    /*n_lauf_lbound*/3,
                ),
            ),
        );
        assert_stichoracle(
            &rules_rufspiel_gras_epi3,
            [
                &[SU, H9, EA, GA, G7, SK, SZ],
                &[HU, HA, HZ, HK, EK, GK, SA],
                &[EU, EZ, E9, G8, S8, S9, S7],
                &[HO, GU, H8, E8, E7, GZ, G9],
            ],
            &[SO, H7, GO, EO],
            &[
                [HO, SU, HU, EU],
                // [HO, SU, HA, EU], // covered by [HO, SU, HU, EU]
                // [HO, SU, HZ, EU], // covered by [HO, SU, HU, EU]
                // [HO, SU, HK, EU], // covered by [HO, SU, HU, EU]
                [HO, H9, HU, EU],
                // [HO, H9, HA, EU], // covered by [HO, H9, HZ, EU]
                // [HO, H9, HZ, EU], // covered by [HO, H9, EK, EU]
                [HO, H9, HK, EU],
                // [GU, SU, HU, EU], // covered by [GU, SU, HA, EU]
                [GU, SU, HA, EU],
                // [GU, SU, HZ, EU], // covered by [GU, SU, HA, EU]
                // [GU, SU, HK, EU], // covered by [GU, SU, HZ, EU]
                [GU, H9, HU, EU],
                [GU, H9, HA, EU],
                // [GU, H9, HZ, EU], // covered by [GU, H9, HA, EU]
                // [GU, H9, HK, EU], // covered by [GU, H9, HA, EU]
                [H8, SU, HU, EU], // TODO should not be needed (HA better than HU)
                [H8, SU, HA, EU],
                // [H8, SU, HZ, EU], // covered by [H8, SU, HA, EU]
                // [H8, SU, HK, EU], // covered by [H8, SU, HZ, EU]
                [H8, H9, HU, EU],
                [H8, H9, HA, EU],
                // [H8, H9, HZ, EU], // covered by [H8, H9, HA, EU]
                // [H8, H9, HK, EU], // covered by [H8, H9, HA, EU]
                // [E8, EA, EK, EZ], // covered by [E8, EA, EK, E9]
                [E8, EA, EK, E9],
                // [E7, EA, EK, EZ], // covered by [E7, EA, EK, E9]
                // [E7, EA, EK, E9], // covered by [E8, EA, EK, E9]
                [GZ, GA, GK, G8],
                [G9, GA, GK, G8], // TODO should not be needed (GZ better than G9)
            ],
        );
        let rules_solo_gras_epi0 = sololike(
            EPlayerIndex::EPI0,
            EFarbe::Gras,
            ESoloLike::Solo,
            SPayoutDeciderPointBased::default_payoutdecider(
                /*n_payout_base*/50,
                /*n_payout_schneider_schwarz*/10,
                SLaufendeParams::new(10, 3),
            ),
        );
        assert_stichoracle(
            TRulesBoxClone::box_clone(rules_solo_gras_epi0.as_ref()).as_ref(),
            [
                &[EO, GO, HO, GZ, G7, EA, EZ, HK],
                &[SO, EU, HU, GA, GK, HA, SA, S9],
                &[GU, SU, G9, G8, E9, H8, SK, S7],
                &[EK, E8, E7, HZ, H9, H7, SZ, S8],
            ],
            &[],
            &[
                // [EO, SO, __, __], // covered by [EO, EU, __, __]
                [EO, EU, GU, EK],
                [EO, EU, GU, E8],
                // [EO, EU, GU, E7], // covered by [EO, EU, GU, E8],
                [EO, EU, GU, HZ],
                [EO, EU, GU, H9],
                [EO, EU, GU, H7],
                [EO, EU, GU, SZ],
                [EO, EU, GU, S8],
                [EO, EU, SU, EK],
                [EO, EU, SU, E8],
                // [EO, EU, SU, E7], // covered by [EO, EU, SU, E8],
                [EO, EU, SU, HZ],
                [EO, EU, SU, H9],
                [EO, EU, SU, H7],
                [EO, EU, SU, SZ],
                [EO, EU, SU, S8],
                [EO, EU, G9, EK],
                [EO, EU, G9, E8],
                // [EO, EU, G9, E7], // covered by [EO, EU, G9, E8],
                [EO, EU, G9, HZ],
                [EO, EU, G9, H9],
                [EO, EU, G9, H7],
                [EO, EU, G9, SZ],
                [EO, EU, G9, S8],
                // [EO, EU, G8, __], // covered by [EO, EU, G9, __]
                [EO, HU, GU, EK],
                [EO, HU, GU, E8],
                // [EO, HU, GU, E7], // covered by [EO, HU, GU, E8]
                [EO, HU, GU, HZ],
                [EO, HU, GU, H9],
                [EO, HU, GU, H7],
                [EO, HU, GU, SZ],
                [EO, HU, GU, S8],
                // [EO, HU, SU, __], // covered by [EO, HU, GU, __]
                [EO, HU, G9, EK],
                [EO, HU, G9, E8],
                // [EO, HU, G9, E7], // covered by [EO, HU, G9, E8],
                [EO, HU, G9, HZ],
                [EO, HU, G9, H9],
                [EO, HU, G9, H7],
                [EO, HU, G9, SZ],
                [EO, HU, G9, S8],
                // [EO, HU, G8, __], // covered by [EO, HU, G9, __]
                [EO, GA, GU, EK],
                [EO, GA, GU, E8],
                // [EO, GA, GU, E7], // covered by [EO, GA, GU, E8],
                [EO, GA, GU, HZ],
                [EO, GA, GU, H9],
                [EO, GA, GU, H7],
                [EO, GA, GU, SZ],
                [EO, GA, GU, S8],
                [EO, GA, SU, EK], // TODO should not be needed (HU better than GA)
                [EO, GA, SU, E8], // TODO should not be needed (HU better than GA)
                // [EO, GA, SU, E7], // covered by [EO, GA, SU, E8],
                [EO, GA, SU, HZ], // TODO should not be needed (HU better than GA)
                [EO, GA, SU, H9], // TODO should not be needed (HU better than GA)
                [EO, GA, SU, H7], // TODO should not be needed (HU better than GA)
                [EO, GA, SU, SZ], // TODO should not be needed (HU better than GA)
                [EO, GA, SU, S8], // TODO should not be needed (HU better than GA)
                [EO, GA, G9, EK],
                [EO, GA, G9, E8],
                // [EO, GA, G9, E7], // covered by [EO, GA, G9, E8],
                [EO, GA, G9, HZ],
                [EO, GA, G9, H9],
                [EO, GA, G9, H7],
                [EO, GA, G9, SZ],
                [EO, GA, G9, S8],
                // [EO, GA, G8, __], // covered by [EO, GA, G8, __],
                [EO, GK, GU, EK],
                [EO, GK, GU, E8],
                // [EO, GK, GU, E7], // covered by [EO, GK, GU, E8],
                [EO, GK, GU, HZ],
                [EO, GK, GU, H9],
                [EO, GK, GU, H7],
                [EO, GK, GU, SZ],
                [EO, GK, GU, S8],
                [EO, GK, SU, EK],
                [EO, GK, SU, E8],
                // [EO, GK, SU, E7], // covered by [EO, GK, SU, E8],
                [EO, GK, SU, HZ],
                [EO, GK, SU, H9],
                [EO, GK, SU, H7],
                [EO, GK, SU, SZ],
                [EO, GK, SU, S8],
                [EO, GK, G9, EK],
                [EO, GK, G9, E8],
                // [EO, GK, G9, E7], // covered by [EO, GK, G9, E8]
                [EO, GK, G9, HZ],
                [EO, GK, G9, H9],
                [EO, GK, G9, H7],
                [EO, GK, G9, SZ],
                [EO, GK, G9, S8],
                // [EO, GK, G8, __], // covered by [EO, GK, G9, __]
                // [GO, __, __, __], // covered by [EO, __, __, __]
                // [HO, __, __, __], // covered by [EO, __, __, __]
                [GZ, SO, GU, EK],
                [GZ, SO, GU, E8],
                // [GZ, SO, GU, E7], // covered by [GZ, SO, GU, E8],
                [GZ, SO, GU, HZ],
                [GZ, SO, GU, H9],
                [GZ, SO, GU, H7],
                [GZ, SO, GU, SZ],
                [GZ, SO, GU, S8],
                [GZ, SO, SU, EK],
                [GZ, SO, SU, E8],
                // [GZ, SO, SU, E7], // covered by [GZ, SO, SU, E8],
                [GZ, SO, SU, HZ],
                [GZ, SO, SU, H9],
                [GZ, SO, SU, H7],
                [GZ, SO, SU, SZ],
                [GZ, SO, SU, S8],
                [GZ, SO, G9, EK],
                [GZ, SO, G9, E8],
                // [GZ, SO, G9, E7], // covered by [GZ, SO, G9, E8],
                [GZ, SO, G9, HZ],
                [GZ, SO, G9, H9],
                [GZ, SO, G9, H7],
                [GZ, SO, G9, SZ],
                [GZ, SO, G9, S8],
                // [GZ, SO, G8, __], // covered by [GZ, SO, G9, __],
                // [GZ, EU, __, __], // covered by [GZ, SO, __, __]
                [GZ, HU, GU, EK],
                [GZ, HU, GU, E8],
                // [GZ, HU, GU, E7], // covered by [GZ, HU, GU, E8],
                [GZ, HU, GU, HZ],
                [GZ, HU, GU, H9],
                [GZ, HU, GU, H7],
                [GZ, HU, GU, SZ],
                [GZ, HU, GU, S8],
                [GZ, HU, SU, EK],
                [GZ, HU, SU, E8],
                // [GZ, HU, SU, E7], // covered by [GZ, HU, SU, E8],
                [GZ, HU, SU, HZ],
                [GZ, HU, SU, H9],
                [GZ, HU, SU, H7],
                [GZ, HU, SU, SZ],
                [GZ, HU, SU, S8],
                [GZ, HU, G9, EK],
                [GZ, HU, G9, E8],
                // [GZ, HU, G9, E7], // covered by [GZ, HU, G9, E8],
                [GZ, HU, G9, HZ],
                [GZ, HU, G9, H9],
                [GZ, HU, G9, H7],
                [GZ, HU, G9, SZ],
                [GZ, HU, G9, S8],
                // [GZ, HU, G8, __], // covered by [GZ, HU, G9, __]
                [GZ, GA, GU, EK],
                [GZ, GA, GU, E8],
                // [GZ, GA, GU, E7], // covered by [GZ, GA, GU, E8],
                [GZ, GA, GU, HZ],
                [GZ, GA, GU, H9],
                [GZ, GA, GU, H7],
                [GZ, GA, GU, SZ],
                [GZ, GA, GU, S8],
                [GZ, GA, SU, EK],
                [GZ, GA, SU, E8],
                // [GZ, GA, SU, E7], // covered by [GZ, GA, SU, E8],
                [GZ, GA, SU, HZ],
                [GZ, GA, SU, H9],
                [GZ, GA, SU, H7],
                [GZ, GA, SU, SZ],
                [GZ, GA, SU, S8],
                [GZ, GA, G9, EK],
                [GZ, GA, G9, E8],
                // [GZ, GA, G9, E7], // covered by [GZ, GA, G9, E8],
                [GZ, GA, G9, HZ],
                [GZ, GA, G9, H9],
                [GZ, GA, G9, H7],
                [GZ, GA, G9, SZ],
                [GZ, GA, G9, S8],
                // [GZ, GA, G8, __], // covered by [GZ, GA, G9, __],
                [GZ, GK, GU, EK], // TODO should not be needed (GA is better than GK)
                [GZ, GK, GU, E8], // TODO should not be needed (GA is better than GK)
                // [GZ, GK, GU, E7], // covered by [GZ, GK, GU, E8],
                [GZ, GK, GU, HZ], // TODO should not be needed (GA is better than GK)
                [GZ, GK, GU, H9], // TODO should not be needed (GA is better than GK)
                [GZ, GK, GU, H7], // TODO should not be needed (GA is better than GK)
                [GZ, GK, GU, SZ], // TODO should not be needed (GA is better than GK)
                [GZ, GK, GU, S8], // TODO should not be needed (GA is better than GK)
                [GZ, GK, SU, EK], // TODO should not be needed (GA is better than GK)
                [GZ, GK, SU, E8], // TODO should not be needed (GA is better than GK)
                // [GZ, GK, SU, E7], // covered by [GZ, GK, SU, E8],
                [GZ, GK, SU, HZ], // TODO should not be needed (GA is better than GK)
                [GZ, GK, SU, H9], // TODO should not be needed (GA is better than GK)
                [GZ, GK, SU, H7], // TODO should not be needed (GA is better than GK)
                [GZ, GK, SU, SZ], // TODO should not be needed (GA is better than GK)
                [GZ, GK, SU, S8], // TODO should not be needed (GA is better than GK)
                [GZ, GK, G9, EK],
                [GZ, GK, G9, E8],
                // [GZ, GK, G9, E7], // covered by [GZ, GK, G9, E8],
                [GZ, GK, G9, HZ],
                [GZ, GK, G9, H9],
                [GZ, GK, G9, H7],
                [GZ, GK, G9, SZ],
                [GZ, GK, G9, S8],
                // [GZ, GK, G8, __], // covered by [GZ, GK, G9, __],
                [G7, SO, GU, EK],
                [G7, SO, GU, E8],
                // [G7, SO, GU, E7], // covered by [G7, SO, GU, E8],
                [G7, SO, GU, HZ],
                [G7, SO, GU, H9],
                [G7, SO, GU, H7],
                [G7, SO, GU, SZ],
                [G7, SO, GU, S8],
                [G7, SO, SU, EK],
                [G7, SO, SU, E8],
                // [G7, SO, SU, E7], // covered by [G7, SO, SU, E8],
                [G7, SO, SU, HZ],
                [G7, SO, SU, H9],
                [G7, SO, SU, H7],
                [G7, SO, SU, SZ],
                [G7, SO, SU, S8],
                [G7, SO, G9, EK],
                [G7, SO, G9, E8],
                // [G7, SO, G9, E7], // covered by [G7, SO, G9, E8],
                [G7, SO, G9, HZ],
                [G7, SO, G9, H9],
                [G7, SO, G9, H7],
                [G7, SO, G9, SZ],
                [G7, SO, G9, S8],
                // [G7, SO, G8, __], // covered by [G7, SO, G9, __],
                // [G7, EU, __, __], // covered by [G7, SO, __, __]
                [G7, HU, GU, EK],
                [G7, HU, GU, E8],
                // [G7, HU, GU, E7], // covered by [G7, HU, GU, E8],
                [G7, HU, GU, HZ],
                [G7, HU, GU, H9],
                [G7, HU, GU, H7],
                [G7, HU, GU, SZ],
                [G7, HU, GU, S8],
                [G7, HU, SU, EK],
                [G7, HU, SU, E8],
                // [G7, HU, SU, E7], // covered by [G7, HU, SU, E8],
                [G7, HU, SU, HZ],
                [G7, HU, SU, H9],
                [G7, HU, SU, H7],
                [G7, HU, SU, SZ],
                [G7, HU, SU, S8],
                [G7, HU, G9, EK],
                [G7, HU, G9, E8],
                // [G7, HU, G9, E7], // covered by [G7, HU, G9, E8],
                [G7, HU, G9, HZ],
                [G7, HU, G9, H9],
                [G7, HU, G9, H7],
                [G7, HU, G9, SZ],
                [G7, HU, G9, S8],
                // [G7, HU, G8, __], // covered by [G7, HU, G9, __],
                [G7, GA, GU, EK],
                [G7, GA, GU, E8],
                // [G7, GA, GU, E7], // covered by [G7, GA, GU, E8],
                [G7, GA, GU, HZ],
                [G7, GA, GU, H9],
                [G7, GA, GU, H7],
                [G7, GA, GU, SZ],
                [G7, GA, GU, S8],
                [G7, GA, SU, EK],
                [G7, GA, SU, E8],
                // [G7, GA, SU, E7], // covered by [G7, GA, SU, E8],
                [G7, GA, SU, HZ],
                [G7, GA, SU, H9],
                [G7, GA, SU, H7],
                [G7, GA, SU, SZ],
                [G7, GA, SU, S8],
                [G7, GA, G9, EK],
                [G7, GA, G9, E8],
                // [G7, GA, G9, E7], // covered by [G7, GA, G9, E8],
                [G7, GA, G9, HZ],
                [G7, GA, G9, H9],
                [G7, GA, G9, H7],
                [G7, GA, G9, SZ],
                [G7, GA, G9, S8],
                // [G7, GA, G8, __], // covered by [G7, GA, G8, __]
                [G7, GK, GU, EK],
                [G7, GK, GU, E8],
                // [G7, GK, GU, E7], // covered by [G7, GK, GU, E8],
                [G7, GK, GU, HZ],
                [G7, GK, GU, H9],
                [G7, GK, GU, H7],
                [G7, GK, GU, SZ],
                [G7, GK, GU, S8],
                [G7, GK, SU, EK],
                [G7, GK, SU, E8],
                // [G7, GK, SU, E7], // covered by [G7, GK, SU, E8],
                [G7, GK, SU, HZ],
                [G7, GK, SU, H9],
                [G7, GK, SU, H7],
                [G7, GK, SU, SZ],
                [G7, GK, SU, S8],
                [G7, GK, G9, EK],
                [G7, GK, G9, E8],
                // [G7, GK, G9, E7], // covered by [G7, GK, G9, E8],
                [G7, GK, G9, HZ],
                [G7, GK, G9, H9],
                [G7, GK, G9, H7],
                [G7, GK, G9, SZ],
                [G7, GK, G9, S8],
                // [G7, GK, G8, __], // covered by [G7, GK, G9, __],
                [EA, SO, E9, EK],
                // [EA, SO, E9, E8], // covered by [EA, SO, E9, EK],
                // [EA, SO, E9, E7], // covered by [EA, SO, E9, EK],
                // [EA, EU, __, __], // covered by [EA, SO, __, __]
                [EA, HU, E9, EK],
                // [EA, HU, E9, E8], // covered by [EA, HU, E9, EK],
                // [EA, HU, E9, E7], // covered by [EA, HU, E9, EK],
                [EA, GA, E9, EK],
                // [EA, GA, E9, E8], // covered by [EA, GA, E9, EK]
                // [EA, GA, E9, E7], // covered by [EA, GA, E9, E8],
                [EA, GK, E9, EK],
                // [EA, GK, E9, E8], // covered by [EA, GK, E9, EK]
                // [EA, GK, E9, E7], // covered by [EA, GK, E9, EK],
                // [EA, HA, E9, EK], // covered by [EA, HA, E9, E8],
                [EA, HA, E9, E8],
                // [EA, HA, E9, E7], // covered by [EA, HA, E9, E8],
                // [EA, SA, E9, EK], // covered by [EA, SA, E9, E8]
                [EA, SA, E9, E8],
                // [EA, SA, E9, E7], // covered by [EA, SA, E9, E8],
                // [EA, S9, E9, EK], // covered by [EA, S9, E9, E8]
                [EA, S9, E9, E8],
                // [EA, S9, E9, E7], // covered by [EA, S9, E9, E8],
                [EZ, SO, E9, EK],
                // [EZ, SO, E9, E8], // covered by [EZ, SO, E9, EK],
                // [EZ, SO, E9, E7], // covered by [EZ, SO, E9, EK],
                // [EZ, EU, __, __], // covered by [EZ, SO, __, __]
                [EZ, HU, E9, EK],
                // [EZ, HU, E9, E8], // covered by [EZ, HU, E9, EK]
                // [EZ, HU, E9, E7], // covered by [EZ, HU, E9, E8]
                [EZ, GA, E9, EK],
                // [EZ, GA, E9, E8], // covered by [EZ, GA, E9, EK],
                // [EZ, GA, E9, E7], // covered by [EZ, GA, E9, EK],
                [EZ, GK, E9, EK],
                // [EZ, GK, E9, E8], // covered by [EZ, GK, E9, EK]
                // [EZ, GK, E9, E7], // covered by [EZ, GK, E9, E8],
                // [EZ, HA, E9, EK], // covered by [EZ, HA, E9, E8]
                [EZ, HA, E9, E8],
                // [EZ, HA, E9, E7], // covered by [EZ, HA, E9, E8]
                // [EZ, SA, E9, EK], // covered by [EZ, SA, E9, E8]
                [EZ, SA, E9, E8],
                // [EZ, SA, E9, E7], // covered by [EZ, SA, E9, E8],
                // [EZ, S9, E9, EK], // covered by [EZ, S9, E9, E8]
                [EZ, S9, E9, E8],
                // [EZ, S9, E9, E7], // covered by [EZ, S9, E9, E8],
                [HK, HA, H8, HZ],
                // [HK, HA, H8, H9], // covered by [HK, HA, H8, HZ]
                // [HK, HA, H8, H7], // covered by [HK, HA, H8, HZ]
            ],
        );
    }

    #[test]
    fn test_filterbystichoracle() {
        crate::game::run::internal_run_simple_game_loop( // TODO simplify all this, and explicitly iterate over supported rules
            EPlayerIndex::map_from_fn(|_epi| Box::new(SPlayerRandom::new(
                /*fn_check_ask_for_card*/|game: &SGame| {
                    if game.kurzlang().cards_per_player() - if_dbg_else!({4}{5}) < game.completed_stichs().len() {
                        //let epi = unwrap!(game.current_playable_stich().current_playerindex());
                        macro_rules! fwd{($ty_fn_make_filter:tt, $fn_make_filter:expr,) => {
                            unwrap!(determine_best_card::<$ty_fn_make_filter,_,_,_,_,_>(
                                &game.stichseq,
                                game.rules.as_ref(),
                                Box::new(std::iter::once(game.ahand.clone())) as Box<_>,
                                $fn_make_filter,
                                &SMinReachablePayout::new_from_game(game),
                                /*fn_snapshotcache*/SSnapshotCacheNone::factory(), // TODO test cache
                                SNoVisualization::factory(),
                                /*fn_inspect*/&|_,_,_,_| {},
                            ))
                                .cards_and_ts()
                                .map(|(card, payoutstatsperstrategy)| (
                                    card,
                                    verify_eq!(
                                        &payoutstatsperstrategy.0[EMinMaxStrategy::SelfishMin],
                                        &payoutstatsperstrategy.0[EMinMaxStrategy::SelfishMax]
                                    ).clone()
                                ))
                                .collect::<Vec<_>>()
                        }}
                        assert_eq!(
                            fwd!(
                                SFilterByOracle,
                                /*fn_make_filter*/|stichseq, ahand| {
                                    SFilterByOracle::new(game.rules.as_ref(), ahand, stichseq)
                                },
                            ),
                            fwd!(
                                _,
                                /*fn_make_filter*/SNoFilter::factory(),
                            ),
                        );
                    }
                },
            )) as Box<dyn TPlayer>),
            /*n_games*/8,
            unwrap!(SRuleSet::from_string(
                r"
                base-price=10
                solo-price=50
                lauf-min=3
                [rufspiel]
                [solo]
                [wenz]
                [farbwenz]
                [geier]
                [farbgeier]
                [stoss]
                max=4
                ",
            )),
            /*fn_gamepreparations_to_stockorgame*/|gamepreparations, _aattable| {
                let itstockorgame = EPlayerIndex::values()
                    .flat_map(|epi| {
                        allowed_rules(
                            &gamepreparations.ruleset.avecrulegroup[epi],
                            gamepreparations.fullhand(epi),
                        )
                    })
                    .filter_map(|orules| {
                        orules.map(|rules| {
                            VStockOrT::OrT(
                                SGame::new(
                                    gamepreparations.aveccard.clone(),
                                    gamepreparations.expensifiers.clone(),
                                    gamepreparations.ruleset.ostossparams.clone(),
                                    rules.upcast().box_clone(),
                                )
                            )
                        })
                    })
                    .collect::<Vec<_>>().into_iter(); // TODO how can we avoid this?
                if_dbg_else!(
                    {{
                        use rand::seq::IteratorRandom;
                        itstockorgame.choose_multiple(&mut rand::thread_rng(), 1).into_iter()
                    }}
                    {itstockorgame}
                )
            },
        );
    }
}
