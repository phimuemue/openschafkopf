use crate::{
    primitives::*,
    rules::{
        SPlayerPartiesTable,
        card_points::points_card,
        SRules,
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
    vectplcardtrie: Box<ArrayVec<(ECard, SStichTrie), {EKurzLang::max_cards_per_player()}>>, // TODO? improve
}

impl SStichTrie {
    fn new() -> Self {
        let slf = Self {
            vectplcardtrie: Box::new(ArrayVec::new()),
        };
        #[cfg(debug_assertions)] slf.assert_invariant();
        slf
    }

    fn push(&mut self, card: ECard, trie_child: SStichTrie) {
        debug_assert!(self.depth_in_edges()==0 || self.depth_in_edges()==trie_child.depth_in_edges()+1);
        self.vectplcardtrie.push((card, trie_child));
        #[cfg(debug_assertions)] self.assert_invariant();
    }

    fn depth_in_edges(&self) -> usize {
        #[cfg(debug_assertions)] self.assert_invariant(); // checks that trie holds stichs of equal length
        if self.vectplcardtrie.is_empty() {
            0
        } else {
            1 + self
                .vectplcardtrie[/*use first as representative; TODO? IterExt::first_as_representative*/0]
                .1
                .depth_in_edges()
        }
    }

    #[cfg(debug_assertions)]
    fn assert_invariant(&self) {
        assert!(
            self.vectplcardtrie.iter()
                .map(|tplcardtrie| tplcardtrie.1.depth_in_edges())
                .all_equal()
        );
        assert!(
            self.vectplcardtrie.iter()
                .map(|tplcardtrie| tplcardtrie.0)
                .all_unique()
        );
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
        (ahand, stichseq): (&mut EnumMap<EPlayerIndex, SHand>, &mut SStichSequence),
        rules: &SRules,
        cardspartition_completed_cards: &SCardsPartition,
        playerparties: &SPlayerPartiesTable,
    ) -> Self {
        fn for_each_allowed_card(
            n_depth: usize, // TODO? static enum type, possibly difference of EPlayerIndex
            (ahand, stichseq): (&mut EnumMap<EPlayerIndex, SHand>, &mut SStichSequence),
            rules: &SRules,
            cardspartition_completed_cards: &SCardsPartition,
            playerparties: &SPlayerPartiesTable,
        ) -> (SStichTrie, Option<bool/*b_stich_winner_is_primary*/>) {
            if n_depth==0 {
                assert!(stichseq.current_stich().is_empty());
                (
                    SStichTrie::new(),
                    Some(playerparties.is_primary_party(rules.winner_index(unwrap!(stichseq.last_completed_stich())))),
                )
            } else {
                let epi_card = unwrap!(stichseq.current_stich().current_playerindex());
                // TODO: If all remaining players (starting with epi_card) belong to the same playerparty, then, if this playerparty...
                // * ... wins the trick, only the highest-points card from each partition's chain must be examined.
                // * ... loses the trick, only the lowest-points card from each partition's chain must be examined.
                let mut veccard_allowed = rules.all_allowed_cards(
                    stichseq,
                    &ahand[epi_card],
                );
                assert!(!veccard_allowed.is_empty());
                let mut oresb_stich_winner_is_primary = None;
                let mut stichtrie = SStichTrie::new();
                while !veccard_allowed.is_empty() {
                    let card_representative = veccard_allowed[0];
                    let (stichtrie_representative, ob_stich_winner_primary_party_representative) = stichseq.zugeben_and_restore_with_hands(ahand, epi_card, card_representative, rules, |ahand, stichseq| {
                        for_each_allowed_card(
                            n_depth-1,
                            (ahand, stichseq),
                            rules,
                            cardspartition_completed_cards,
                            playerparties,
                        )
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
                        card_representative: ECard,
                        mut func: impl FnMut(ECard),
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
                                if assign_neq(&mut ab_points[points_card(card_chain).as_num::<usize>()], true) {
                                    stichtrie.push(card_chain, stichtrie_representative.clone());
                                }
                            });
                            oresb_stich_winner_is_primary = Some(Err(()));
                        },
                        Some(b_stich_winner_is_primary) => {
                            macro_rules! card_min_or_max(($fn_assign_by:expr) => {{
                                let mut card_min_or_max = cardspartition_actual
                                    .prev_while_contained(card_representative, &veccard_allowed);
                                iterate_chain(&cardspartition_actual, &mut veccard_allowed, card_representative, |card_chain| {
                                    $fn_assign_by(
                                        &mut card_min_or_max,
                                        card_chain,
                                        |card| points_card(*card),
                                    );
                                });
                                card_min_or_max
                            }});
                            stichtrie.push(
                                if b_stich_winner_is_primary==playerparties.is_primary_party(epi_card) {
                                    card_min_or_max!(assign_max_by_key)
                                } else {
                                    card_min_or_max!(assign_min_by_key)
                                },
                                stichtrie_representative,
                            );
                            match &oresb_stich_winner_is_primary {
                                None => {
                                    oresb_stich_winner_is_primary = Some(Ok(b_stich_winner_is_primary));
                                },
                                Some(Ok(b_stich_winner_is_primary_prev)) => {
                                    if b_stich_winner_is_primary!=*b_stich_winner_is_primary_prev {
                                        oresb_stich_winner_is_primary = Some(Err(()));
                                    }
                                },
                                Some(Err(())) => {/*stay different*/}
                            }
                        },
                    }
                }
                (
                    stichtrie,
                    match unwrap!(oresb_stich_winner_is_primary) {
                        Ok(b_stich_winner_is_primary) => Some(b_stich_winner_is_primary),
                        Err(()) => None,
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
                    &SRuleStateCacheFixed::new(ahand, stichseq),
                )).0;
                for (_epi, &card) in stichseq.completed_cards() {
                    cardspartition_check.remove_from_chain(card);
                }
                cardspartition_check
            }
        );
        let mut make_stichtrie = || for_each_allowed_card(
            EPlayerIndex::SIZE-n_stich_size,
            (ahand, stichseq),
            rules,
            cardspartition_completed_cards,
            playerparties,
        ).0;
        let make_singleton_stichtrie = |i_epi_offset, stichtrie| {
            let mut stichtrie_singleton = SStichTrie::new();
            stichtrie_singleton.push(
                *unwrap!(stich_current.get(stich_current.first_playerindex().wrapping_add(i_epi_offset))),
                stichtrie
            );
            stichtrie_singleton
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
        #[cfg(debug_assertions)] stichtrie.assert_invariant();
        debug_assert!(stichtrie.traverse_trie(stichseq.current_stich().first_playerindex()).iter().all(|stich|
            stich.equal_up_to_size(&stich_current, stich_current.size())
        ));
        stichtrie
    }
}

// TODO: Another oracle could be useful: Explore all possible stichs, and - for each winner index -
// keep the stich with minimum resp. maximum points (and possibly one randomly chosen one). Is this
// maybe a sensible proxy for exploring a gametree completely?

#[derive(Debug)]
pub struct SFilterByOracle<'rules> {
    rules: &'rules SRules,
    stichtrie: SStichTrie,
    cardspartition_completed_cards: SCardsPartition,
    playerparties: SPlayerPartiesTable,
}

impl<'rules> SFilterByOracle<'rules> {
    pub fn new(
        rules: &'rules SRules,
        ahand_in_game: &EnumMap<EPlayerIndex, SHand>,
        stichseq_in_game: &SStichSequence,
    ) -> Option<Self> {
        let mut ahand = EPlayerIndex::map_from_fn(|epi| SHand::new_from_iter(
            stichseq_in_game.cards_from_player(&ahand_in_game[epi], epi)
        ));
        assert!(crate::ai::ahand_vecstich_card_count_is_compatible(ahand_in_game, stichseq_in_game));
        let mut stichseq = SStichSequence::new(stichseq_in_game.kurzlang());
        assert!(crate::ai::ahand_vecstich_card_count_is_compatible(&ahand, &stichseq));
        rules.only_minmax_points_when_on_same_hand(
            &verify_eq!(
                SRuleStateCacheFixed::new(ahand_in_game, stichseq_in_game),
                SRuleStateCacheFixed::new(&ahand, &stichseq)
            )
        ).map(|(cardspartition, playerparties)| {
            let mut slf = Self {
                rules,
                stichtrie: SStichTrie::new(), // TODO this is a dummy value that should not be needed. Eliminate it.
                cardspartition_completed_cards: cardspartition,
                playerparties,
            };
            for stich in stichseq_in_game.completed_stichs() {
                for (epi, card) in stich.iter() {
                    stichseq.zugeben(*card, rules);
                    ahand[epi].play_card(*card);
                }
                slf.register_stich(&mut ahand, &mut stichseq);
            }
            let stichtrie = SStichTrie::new_with(
                (&mut ahand_in_game.clone(), &mut stichseq_in_game.clone()),
                rules,
                &slf.cardspartition_completed_cards,
                &slf.playerparties,
            );
            slf.stichtrie = stichtrie;
            slf
        })
    }
}

impl<'rules> TFilterAllowedCards for SFilterByOracle<'rules> {
    type UnregisterStich = (SStichTrie, EnumMap<EPlayerIndex, SRemoved>);
    fn register_stich(&mut self, ahand: &mut EnumMap<EPlayerIndex, SHand>, stichseq: &mut SStichSequence) -> Self::UnregisterStich {
        assert!(stichseq.current_stich().is_empty());
        let aremovedcard = EPlayerIndex::map_from_fn(|epi|
            self.cardspartition_completed_cards.remove_from_chain(unwrap!(stichseq.last_completed_stich())[epi])
        );
        let stichtrie = SStichTrie::new_with(
            (ahand, stichseq),
            self.rules,
            &self.cardspartition_completed_cards,
            &self.playerparties,
        );
        (std::mem::replace(&mut self.stichtrie, stichtrie), aremovedcard)
    }
    fn unregister_stich(&mut self, (stichtrie, aremovedcard): Self::UnregisterStich) {
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
        game::SGameGeneric,
        player::{
            TPlayer,
            playerrandom::SPlayerRandom,
        },
        primitives::{*, card::ECard::*, hand::display_card_slices},
        rules::{
            payoutdecider::{SPayoutDeciderParams, SPayoutDeciderPointBased, SLaufendeParams},
            rulesrufspiel::SRulesRufspiel,
            rulessolo::{sololike, ESoloLike, TPayoutDeciderSoloLikeDefault},
            ruleset::{
                SRuleSet,
                allowed_rules,
                VStockOrT,
            },
            SStossParams,
            SRules,
            SActivelyPlayableRules,
            TRules,
        },
        util::*,
        ai::{
            determine_best_card,
            SMinReachablePayout,
            SNoFilter,
            SNoVisualization,
            SRuleStateCacheFixed,
            SSnapshotCacheNone,
        },
    };
    use super::{SStichTrie, SFilterByOracle};
    use itertools::Itertools;

    #[test]
    fn test_stichoracle() {
        let stossparams = SStossParams::new(
            /*n_stoss_max*/4,
        );
        let rules_rufspiel_eichel_epi0 = SRules::from(SActivelyPlayableRules::from(SRulesRufspiel::new(
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
            stossparams.clone(),
        )));
        fn assert_stichoracle(
            rules: &SRules,
            aslccard_hand: [&[ECard]; EPlayerIndex::SIZE],
            slccard_stichseq: &[ECard],
            slcacard_stich: &[[ECard; EPlayerIndex::SIZE]],
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
                &SRuleStateCacheFixed::new(ahand, &stichseq),
            ));
            for (_epi, card) in stichseq.completed_cards() {
                cardspartition.remove_from_chain(*card);
            }
            let stichtrie = SStichTrie::new_with(
                (&mut ahand.clone(), &mut stichseq.clone()),
                rules,
                &cardspartition,
                &playerparties,
            );
            let setstich_oracle = stichtrie.traverse_trie(stichseq.current_stich().first_playerindex()).iter().cloned().collect::<std::collections::HashSet<_>>();
            let setstich_check = slcacard_stich
                .iter()
                .map(|acard| SStich::new_full(
                    epi_first,
                    acard.clone(),
                ))
                .collect::<std::collections::HashSet<_>>();
            //assert_eq!(setstich_oracle.len(), setstich_check.len());
            let assert_is_subset_of = |
                setstich_subset: &std::collections::HashSet<SStich>,
                setstich_superset: &std::collections::HashSet<SStich>,
                str_msg,
            | {
                let vecstich_not_in_superset = setstich_subset.iter()
                    .filter(|stich_subset| !setstich_superset.contains(stich_subset)) 
                    .collect::<Vec<_>>();
                assert!(
                    vecstich_not_in_superset.is_empty(),
                    "\nRules:{} von {}\nHands:\n {}\nStichseq: {}\nStichs:\n{:?}\n{}\n",
                    rules,
                    unwrap!(rules.playerindex()),
                    display_card_slices(ahand, rules, "\n "),
                    stichseq.visible_stichs().iter().join(", "),
                    vecstich_not_in_superset,
                    str_msg,
                );
            };
            assert_is_subset_of(&setstich_check, &setstich_oracle, "oracle missing stichs");
            assert_is_subset_of(&setstich_oracle, &setstich_check, "oracle contains unexpected stichs");
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
        let rules_rufspiel_gras_epi3 = SRules::from(SActivelyPlayableRules::from(SRulesRufspiel::new(
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
            stossparams.clone(),
        )));
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
        let sololike_internal = |epi, efarbe, esololike| {
            sololike(
                epi,
                efarbe,
                esololike,
                SPayoutDeciderPointBased::default_payoutdecider(
                    /*n_payout_base*/50,
                    /*n_payout_schneider_schwarz*/10,
                    SLaufendeParams::new(10, 3),
                ),
                stossparams.clone(),
            )
        };
        let rules_solo_gras_epi0 = sololike_internal(
            EPlayerIndex::EPI0,
            EFarbe::Gras,
            ESoloLike::Solo,
        );
        assert_stichoracle(
            &rules_solo_gras_epi0.into(),
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
        let rules_farbwenz_eichel_epi3 = sololike_internal(
            EPlayerIndex::EPI3,
            EFarbe::Eichel,
            ESoloLike::Wenz,
        ).into();
        assert_stichoracle(
            &rules_farbwenz_eichel_epi3,
            [ // This seems to be an expensive card distribution.
                &[GA, GZ, G8, HA, HK, H8, H7, SO],
                &[GU, GK, GO, G9, G7, H9, SK, S8],
                &[EU, HU, EA, E9, HZ, HO, SZ, S9],
                &[SU, EZ, EK, EO, E8, E7, SA, S7],
            ],
            &[],
            &[
                [GA, GK, EU, SU],
                // [GA, GK, EU, EZ], // covered by [.., .., .., EO]
                // [GA, GK, EU, EK], // covered by [.., .., .., EO]
                [GA, GK, EU, EO],
                [GA, GK, EU, E8],
                // [GA, GK, EU, E7], // covered by [.., .., .., E8]
                [GA, GK, EU, SA],
                [GA, GK, EU, S7],
                [GA, GK, HU, SU],
                // [GA, GK, HU, EZ], // covered by [.., .., .., EO]
                // [GA, GK, HU, EK], // covered by [.., .., .., EO]
                [GA, GK, HU, EO],
                [GA, GK, HU, E8],
                // [GA, GK, HU, E7], // covered by [.., .., .., E8]
                [GA, GK, HU, SA],
                [GA, GK, HU, S7],
                [GA, GK, EA, SU],
                // [GA, GK, EA, EZ], // covered by [.., .., .., EO]
                // [GA, GK, EA, EK], // covered by [.., .., .., EO]
                [GA, GK, EA, EO],
                [GA, GK, EA, E8],
                // [GA, GK, EA, E7], // covered by [.., .., .., E8]
                [GA, GK, EA, SA],
                [GA, GK, EA, S7],
                [GA, GK, E9, SU],
                [GA, GK, E9, EZ],
                // [GA, GK, E9, EK], // covered by [.., .., .., EZ]
                // [GA, GK, E9, EO], // covered by [.., .., .., EZ]
                [GA, GK, E9, E8],
                // [GA, GK, E9, E7], // covered by [.., .., .., E8]
                [GA, GK, E9, SA],
                [GA, GK, E9, S7],
                [GA, GK, HZ, SU],
                [GA, GK, HZ, EZ],
                // [GA, GK, HZ, EK], // covered by [.., .., .., EZ]
                // [GA, GK, HZ, EO], // covered by [.., .., .., EZ]
                [GA, GK, HZ, E8],
                // [GA, GK, HZ, E7], // covered by [.., .., .., E8]
                [GA, GK, HZ, SA],
                [GA, GK, HZ, S7],
                [GA, GK, HO, SU],
                [GA, GK, HO, EZ],
                // [GA, GK, HO, EK], // covered by [.., .., .., EZ]
                // [GA, GK, HO, EO], // covered by [.., .., .., EZ]
                [GA, GK, HO, E8],
                // [GA, GK, HO, E7], // covered by [.., .., .., E8]
                [GA, GK, HO, SA],
                [GA, GK, HO, S7],
                [GA, GK, SZ, SU],
                [GA, GK, SZ, EZ],
                // [GA, GK, SZ, EK], // covered by [.., .., .., EZ]
                // [GA, GK, SZ, EO], // covered by [.., .., .., EZ]
                [GA, GK, SZ, E8],
                // [GA, GK, SZ, E7], // covered by [.., .., .., E8]
                [GA, GK, SZ, SA],
                [GA, GK, SZ, S7],
                [GA, GK, S9, SU],
                [GA, GK, S9, EZ],
                // [GA, GK, S9, EK], // covered by [.., .., .., EZ]
                // [GA, GK, S9, EO], // covered by [.., .., .., EZ]
                [GA, GK, S9, E8],
                // [GA, GK, S9, E7], // covered by [.., .., .., E8]
                [GA, GK, S9, SA],
                [GA, GK, S9, S7],
                [GA, GO, EU, SU], // TODO should not be needed (GK better than GO)
                // [GA, GO, EU, EZ], // covered by [.., .., .., EO]
                // [GA, GO, EU, EK], // covered by [.., .., .., EO]
                [GA, GO, EU, EO], // TODO should not be needed (GK better than GO)
                [GA, GO, EU, E8], // TODO should not be needed (GK better than GO)
                // [GA, GO, EU, E7], // covered by [.., .., .., E8]
                [GA, GO, EU, SA], // TODO should not be needed (GK better than GO)
                [GA, GO, EU, S7], // TODO should not be needed (GK better than GO)
                [GA, GO, HU, SU],
                // [GA, GO, HU, EZ], // covered by [.., .., .., EO]
                // [GA, GO, HU, EK], // covered by [.., .., .., EO]
                [GA, GO, HU, EO],
                [GA, GO, HU, E8],
                // [GA, GO, HU, E7], // covered by [.., .., .., E8]
                [GA, GO, HU, SA],
                [GA, GO, HU, S7],
                [GA, GO, EA, SU],
                // [GA, GO, EA, EZ], // covered by [.., .., .., EO]
                // [GA, GO, EA, EK], // covered by [.., .., .., EO]
                [GA, GO, EA, EO],
                [GA, GO, EA, E8],
                // [GA, GO, EA, E7], // covered by [.., .., .., E8]
                [GA, GO, EA, SA],
                [GA, GO, EA, S7],
                [GA, GO, E9, SU],
                [GA, GO, E9, EZ],
                // [GA, GO, E9, EK], // covered by [.., .., .., EZ]
                // [GA, GO, E9, EO], // covered by [.., .., .., EZ]
                [GA, GO, E9, E8],
                // [GA, GO, E9, E7], // covered by [.., .., .., E8]
                [GA, GO, E9, SA],
                [GA, GO, E9, S7],
                [GA, GO, HZ, SU],
                [GA, GO, HZ, EZ],
                // [GA, GO, HZ, EK], // covered by [.., .., .., EZ]
                // [GA, GO, HZ, EO], // covered by [.., .., .., EZ]
                [GA, GO, HZ, E8],
                // [GA, GO, HZ, E7], // covered by [.., .., .., E8]
                [GA, GO, HZ, SA],
                [GA, GO, HZ, S7],
                [GA, GO, HO, SU],
                [GA, GO, HO, EZ],
                // [GA, GO, HO, EK], // covered by [.., .., .., EZ]
                // [GA, GO, HO, EO], // covered by [.., .., .., EZ]
                [GA, GO, HO, E8],
                // [GA, GO, HO, E7], // covered by [.., .., .., E8]
                [GA, GO, HO, SA],
                [GA, GO, HO, S7],
                [GA, GO, SZ, SU],
                [GA, GO, SZ, EZ],
                // [GA, GO, SZ, EK], // covered by [.., .., .., EZ]
                // [GA, GO, SZ, EO], // covered by [.., .., .., EZ]
                [GA, GO, SZ, E8],
                // [GA, GO, SZ, E7], // covered by [.., .., .., E8]
                [GA, GO, SZ, SA],
                [GA, GO, SZ, S7],
                [GA, GO, S9, SU],
                [GA, GO, S9, EZ],
                // [GA, GO, S9, EK], // covered by [.., .., .., EZ]
                // [GA, GO, S9, EO], // covered by [.., .., .., EZ]
                [GA, GO, S9, E8],
                // [GA, GO, S9, E7], // covered by [.., .., .., E8]
                [GA, GO, S9, SA],
                [GA, GO, S9, S7],
                [GA, G9, EU, SU], // TODO should not be needed (GK better than G9)
                // [GA, G9, EU, EZ], // covered by [.., .., .., EO]
                // [GA, G9, EU, EK], // covered by [.., .., .., EO]
                [GA, G9, EU, EO], // TODO should not be needed (GK better than G9)
                [GA, G9, EU, E8], // TODO should not be needed (GK better than G9)
                // [GA, G9, EU, E7], // covered by [.., .., .., E8]
                [GA, G9, EU, SA], // TODO should not be needed (GK better than G9)
                [GA, G9, EU, S7], // TODO should not be needed (GK better than G9)
                [GA, G9, HU, SU],
                // [GA, G9, HU, EZ], // covered by [.., .., .., EO]
                // [GA, G9, HU, EK], // covered by [.., .., .., EO]
                [GA, G9, HU, EO],
                [GA, G9, HU, E8],
                // [GA, G9, HU, E7], // covered by [.., .., .., E8]
                [GA, G9, HU, SA],
                [GA, G9, HU, S7],
                [GA, G9, EA, SU],
                // [GA, G9, EA, EZ], // covered by [.., .., .., EO]
                // [GA, G9, EA, EK], // covered by [.., .., .., EO]
                [GA, G9, EA, EO],
                [GA, G9, EA, E8],
                // [GA, G9, EA, E7], // covered by [.., .., .., E8]
                [GA, G9, EA, SA],
                [GA, G9, EA, S7],
                [GA, G9, E9, SU],
                [GA, G9, E9, EZ],
                // [GA, G9, E9, EK], // covered by [.., .., .., EZ]
                // [GA, G9, E9, EO], // covered by [.., .., .., EZ]
                [GA, G9, E9, E8],
                // [GA, G9, E9, E7], // covered by [.., .., .., E8]
                [GA, G9, E9, SA],
                [GA, G9, E9, S7],
                [GA, G9, HZ, SU],
                [GA, G9, HZ, EZ],
                // [GA, G9, HZ, EK], // covered by [.., .., .., EZ]
                // [GA, G9, HZ, EO], // covered by [.., .., .., EZ]
                [GA, G9, HZ, E8],
                // [GA, G9, HZ, E7], // covered by [.., .., .., E8]
                [GA, G9, HZ, SA],
                [GA, G9, HZ, S7],
                [GA, G9, HO, SU],
                [GA, G9, HO, EZ],
                // [GA, G9, HO, EK], // covered by [.., .., .., EZ]
                // [GA, G9, HO, EO], // covered by [.., .., .., EZ]
                [GA, G9, HO, E8],
                // [GA, G9, HO, E7], // covered by [.., .., .., E8]
                [GA, G9, HO, SA],
                [GA, G9, HO, S7],
                [GA, G9, SZ, SU],
                [GA, G9, SZ, EZ],
                // [GA, G9, SZ, EK], // covered by [.., .., .., EZ]
                // [GA, G9, SZ, EO], // covered by [.., .., .., EZ]
                [GA, G9, SZ, E8],
                // [GA, G9, SZ, E7], // covered by [.., .., .., E8]
                [GA, G9, SZ, SA],
                [GA, G9, SZ, S7],
                [GA, G9, S9, SU],
                [GA, G9, S9, EZ],
                // [GA, G9, S9, EK], // covered by [.., .., .., EZ]
                // [GA, G9, S9, EO], // covered by [.., .., .., EZ]
                [GA, G9, S9, E8],
                // [GA, G9, S9, E7], // covered by [.., .., .., E8]
                [GA, G9, S9, SA],
                [GA, G9, S9, S7],
                [GA, G7, EU, SU],
                // [GA, G7, EU, EZ], // covered by [.., .., .., EO]
                // [GA, G7, EU, EK], // covered by [.., .., .., EO]
                [GA, G7, EU, EO],
                [GA, G7, EU, E8],
                // [GA, G7, EU, E7], // covered by [.., .., .., E8]
                [GA, G7, EU, SA],
                [GA, G7, EU, S7],
                [GA, G7, HU, SU],
                // [GA, G7, HU, EZ], // covered by [.., .., .., EO]
                // [GA, G7, HU, EK], // covered by [.., .., .., EO]
                [GA, G7, HU, EO],
                [GA, G7, HU, E8],
                // [GA, G7, HU, E7], // covered by [.., .., .., E8]
                [GA, G7, HU, SA],
                [GA, G7, HU, S7],
                [GA, G7, EA, SU],
                // [GA, G7, EA, EZ], // covered by [.., .., .., EO]
                // [GA, G7, EA, EK], // covered by [.., .., .., EO]
                [GA, G7, EA, EO],
                [GA, G7, EA, E8],
                // [GA, G7, EA, E7], // covered by [.., .., .., E8]
                [GA, G7, EA, SA],
                [GA, G7, EA, S7],
                [GA, G7, E9, SU],
                [GA, G7, E9, EZ],
                // [GA, G7, E9, EK], // covered by [.., .., .., EZ]
                // [GA, G7, E9, EO], // covered by [.., .., .., EZ]
                [GA, G7, E9, E8],
                // [GA, G7, E9, E7], // covered by [.., .., .., E8]
                [GA, G7, E9, SA],
                [GA, G7, E9, S7],
                [GA, G7, HZ, SU],
                [GA, G7, HZ, EZ],
                // [GA, G7, HZ, EK], // covered by [.., .., .., EZ]
                // [GA, G7, HZ, EO], // covered by [.., .., .., EZ]
                [GA, G7, HZ, E8],
                // [GA, G7, HZ, E7], // covered by [.., .., .., E8]
                [GA, G7, HZ, SA],
                [GA, G7, HZ, S7],
                [GA, G7, HO, SU],
                [GA, G7, HO, EZ],
                // [GA, G7, HO, EK], // covered by [.., .., .., EZ]
                // [GA, G7, HO, EO], // covered by [.., .., .., EZ]
                [GA, G7, HO, E8],
                // [GA, G7, HO, E7], // covered by [.., .., .., E8]
                [GA, G7, HO, SA],
                [GA, G7, HO, S7],
                [GA, G7, SZ, SU],
                [GA, G7, SZ, EZ],
                // [GA, G7, SZ, EK], // covered by [.., .., .., EZ]
                // [GA, G7, SZ, EO], // covered by [.., .., .., EZ]
                [GA, G7, SZ, E8],
                // [GA, G7, SZ, E7], // covered by [.., .., .., E8]
                [GA, G7, SZ, SA],
                [GA, G7, SZ, S7],
                [GA, G7, S9, SU],
                [GA, G7, S9, EZ],
                // [GA, G7, S9, EK], // covered by [.., .., .., EZ]
                // [GA, G7, S9, EO], // covered by [.., .., .., EZ]
                [GA, G7, S9, E8],
                // [GA, G7, S9, E7], // covered by [.., .., .., E8]
                [GA, G7, S9, SA],
                [GA, G7, S9, S7],
                [GZ, GK, EU, SU], // TODO should not be needed (GA better than GZ)
                // [GZ, GK, EU, EZ], // covered by [.., .., .., EO]
                // [GZ, GK, EU, EK], // covered by [.., .., .., EO]
                [GZ, GK, EU, EO], // TODO should not be needed (GA better than GZ)
                [GZ, GK, EU, E8], // TODO should not be needed (GA better than GZ)
                // [GZ, GK, EU, E7], // covered by [.., .., .., E8]
                [GZ, GK, EU, SA], // TODO should not be needed (GA better than GZ)
                [GZ, GK, EU, S7], // TODO should not be needed (GA better than GZ)
                [GZ, GK, HU, SU], // TODO should not be needed (GA better than GZ)
                // [GZ, GK, HU, EZ], // covered by [.., .., .., EO]
                // [GZ, GK, HU, EK], // covered by [.., .., .., EO]
                [GZ, GK, HU, EO], // TODO should not be needed (GA better than GZ)
                [GZ, GK, HU, E8], // TODO should not be needed (GA better than GZ)
                // [GZ, GK, HU, E7], // covered by [.., .., .., E8]
                [GZ, GK, HU, SA], // TODO should not be needed (GA better than GZ)
                [GZ, GK, HU, S7], // TODO should not be needed (GA better than GZ)
                [GZ, GK, EA, SU],
                // [GZ, GK, EA, EZ], // covered by [.., .., .., EO]
                // [GZ, GK, EA, EK], // covered by [.., .., .., EO]
                [GZ, GK, EA, EO],
                [GZ, GK, EA, E8],
                // [GZ, GK, EA, E7], // covered by [.., .., .., E8]
                [GZ, GK, EA, SA],
                [GZ, GK, EA, S7],
                [GZ, GK, E9, SU],
                [GZ, GK, E9, EZ],
                // [GZ, GK, E9, EK], // covered by [.., .., .., EZ]
                // [GZ, GK, E9, EO], // covered by [.., .., .., EZ]
                [GZ, GK, E9, E8],
                // [GZ, GK, E9, E7], // covered by [.., .., .., E8]
                [GZ, GK, E9, SA],
                [GZ, GK, E9, S7],
                [GZ, GK, HZ, SU],
                [GZ, GK, HZ, EZ],
                // [GZ, GK, HZ, EK], // covered by [.., .., .., EZ]
                // [GZ, GK, HZ, EO], // covered by [.., .., .., EZ]
                [GZ, GK, HZ, E8],
                // [GZ, GK, HZ, E7], // covered by [.., .., .., E8]
                [GZ, GK, HZ, SA],
                [GZ, GK, HZ, S7],
                [GZ, GK, HO, SU],
                [GZ, GK, HO, EZ],
                // [GZ, GK, HO, EK], // covered by [.., .., .., EZ]
                // [GZ, GK, HO, EO], // covered by [.., .., .., EZ]
                [GZ, GK, HO, E8],
                // [GZ, GK, HO, E7], // covered by [.., .., .., E8]
                [GZ, GK, HO, SA],
                [GZ, GK, HO, S7],
                [GZ, GK, SZ, SU],
                [GZ, GK, SZ, EZ],
                // [GZ, GK, SZ, EK], // covered by [.., .., .., EZ]
                // [GZ, GK, SZ, EO], // covered by [.., .., .., EZ]
                [GZ, GK, SZ, E8],
                // [GZ, GK, SZ, E7], // covered by [.., .., .., E8]
                [GZ, GK, SZ, SA],
                [GZ, GK, SZ, S7],
                [GZ, GK, S9, SU],
                [GZ, GK, S9, EZ],
                // [GZ, GK, S9, EK], // covered by [.., .., .., EZ]
                // [GZ, GK, S9, EO], // covered by [.., .., .., EZ]
                [GZ, GK, S9, E8],
                // [GZ, GK, S9, E7], // covered by [.., .., .., E8]
                [GZ, GK, S9, SA],
                [GZ, GK, S9, S7],
                [GZ, GO, EU, SU], // TODO should not be needed (GK better than GO)
                // [GZ, GO, EU, EZ], // covered by [.., .., .., EO]
                // [GZ, GO, EU, EK], // covered by [.., .., .., EO]
                [GZ, GO, EU, EO], // TODO should not be needed (GK better than GO)
                [GZ, GO, EU, E8], // TODO should not be needed (GK better than GO)
                // [GZ, GO, EU, E7], // covered by [.., .., .., E8]
                [GZ, GO, EU, SA], // TODO should not be needed (GK better than GO)
                [GZ, GO, EU, S7], // TODO should not be needed (GK better than GO)
                [GZ, GO, HU, SU], // TODO should not be needed (GA better than GZ)
                // [GZ, GO, HU, EZ], // covered by [.., .., .., EO]
                // [GZ, GO, HU, EK], // covered by [.., .., .., EO]
                [GZ, GO, HU, EO], // TODO should not be needed (GA better than GZ)
                [GZ, GO, HU, E8], // TODO should not be needed (GA better than GZ)
                // [GZ, GO, HU, E7], // covered by [.., .., .., E8]
                [GZ, GO, HU, SA], // TODO should not be needed (GA better than GZ)
                [GZ, GO, HU, S7], // TODO should not be needed (GA better than GZ)
                [GZ, GO, EA, SU],
                // [GZ, GO, EA, EZ], // covered by [.., .., .., EO]
                // [GZ, GO, EA, EK], // covered by [.., .., .., EO]
                [GZ, GO, EA, EO],
                [GZ, GO, EA, E8],
                // [GZ, GO, EA, E7], // covered by [.., .., .., E8]
                [GZ, GO, EA, SA],
                [GZ, GO, EA, S7],
                [GZ, GO, E9, SU],
                [GZ, GO, E9, EZ],
                // [GZ, GO, E9, EK], // covered by [.., .., .., EZ]
                // [GZ, GO, E9, EO], // covered by [.., .., .., EZ]
                [GZ, GO, E9, E8],
                // [GZ, GO, E9, E7], // covered by [.., .., .., E8]
                [GZ, GO, E9, SA],
                [GZ, GO, E9, S7],
                [GZ, GO, HZ, SU],
                [GZ, GO, HZ, EZ],
                // [GZ, GO, HZ, EK], // covered by [.., .., .., EZ]
                // [GZ, GO, HZ, EO], // covered by [.., .., .., EZ]
                [GZ, GO, HZ, E8],
                // [GZ, GO, HZ, E7], // covered by [.., .., .., E8]
                [GZ, GO, HZ, SA],
                [GZ, GO, HZ, S7],
                [GZ, GO, HO, SU],
                [GZ, GO, HO, EZ],
                // [GZ, GO, HO, EK], // covered by [.., .., .., EZ]
                // [GZ, GO, HO, EO], // covered by [.., .., .., EZ]
                [GZ, GO, HO, E8],
                // [GZ, GO, HO, E7], // covered by [.., .., .., E8]
                [GZ, GO, HO, SA],
                [GZ, GO, HO, S7],
                [GZ, GO, SZ, SU],
                [GZ, GO, SZ, EZ],
                // [GZ, GO, SZ, EK], // covered by [.., .., .., EZ]
                // [GZ, GO, SZ, EO], // covered by [.., .., .., EZ]
                [GZ, GO, SZ, E8],
                // [GZ, GO, SZ, E7], // covered by [.., .., .., E8]
                [GZ, GO, SZ, SA],
                [GZ, GO, SZ, S7],
                [GZ, GO, S9, SU],
                [GZ, GO, S9, EZ],
                // [GZ, GO, S9, EK], // covered by [.., .., .., EZ]
                // [GZ, GO, S9, EO], // covered by [.., .., .., EZ]
                [GZ, GO, S9, E8],
                // [GZ, GO, S9, E7], // covered by [.., .., .., E8]
                [GZ, GO, S9, SA],
                [GZ, GO, S9, S7],
                [GZ, G9, EU, SU], // TODO should not be needed (GK better than G9)
                // [GZ, G9, EU, EZ], // covered by [.., .., .., EO]
                // [GZ, G9, EU, EK], // covered by [.., .., .., EO]
                [GZ, G9, EU, EO], // TODO should not be needed (GK better than G9)
                [GZ, G9, EU, E8], // TODO should not be needed (GK better than G9)
                // [GZ, G9, EU, E7], // covered by [.., .., .., E8]
                [GZ, G9, EU, SA], // TODO should not be needed (GK better than G9)
                [GZ, G9, EU, S7], // TODO should not be needed (GK better than G9)
                [GZ, G9, HU, SU], // TODO should not be needed (GA better than GZ)
                // [GZ, G9, HU, EZ], // covered by [.., .., .., EO]
                // [GZ, G9, HU, EK], // covered by [.., .., .., EO]
                [GZ, G9, HU, EO], // TODO should not be needed (GA better than GZ)
                [GZ, G9, HU, E8], // TODO should not be needed (GA better than GZ)
                // [GZ, G9, HU, E7], // covered by [.., .., .., E8]
                [GZ, G9, HU, SA], // TODO should not be needed (GA better than GZ)
                [GZ, G9, HU, S7], // TODO should not be needed (GA better than GZ)
                [GZ, G9, EA, SU],
                // [GZ, G9, EA, EZ], // covered by [.., .., .., EO]
                // [GZ, G9, EA, EK], // covered by [.., .., .., EO]
                [GZ, G9, EA, EO],
                [GZ, G9, EA, E8],
                // [GZ, G9, EA, E7], // covered by [.., .., .., E8]
                [GZ, G9, EA, SA],
                [GZ, G9, EA, S7],
                [GZ, G9, E9, SU],
                [GZ, G9, E9, EZ],
                // [GZ, G9, E9, EK], // covered by [.., .., .., EZ]
                // [GZ, G9, E9, EO], // covered by [.., .., .., EZ]
                [GZ, G9, E9, E8],
                // [GZ, G9, E9, E7], // covered by [.., .., .., E8]
                [GZ, G9, E9, SA],
                [GZ, G9, E9, S7],
                [GZ, G9, HZ, SU],
                [GZ, G9, HZ, EZ],
                // [GZ, G9, HZ, EK], // covered by [.., .., .., EZ]
                // [GZ, G9, HZ, EO], // covered by [.., .., .., EZ]
                [GZ, G9, HZ, E8],
                // [GZ, G9, HZ, E7], // covered by [.., .., .., E8]
                [GZ, G9, HZ, SA],
                [GZ, G9, HZ, S7],
                [GZ, G9, HO, SU],
                [GZ, G9, HO, EZ],
                // [GZ, G9, HO, EK], // covered by [.., .., .., EZ]
                // [GZ, G9, HO, EO], // covered by [.., .., .., EZ]
                [GZ, G9, HO, E8],
                // [GZ, G9, HO, E7], // covered by [.., .., .., E8]
                [GZ, G9, HO, SA],
                [GZ, G9, HO, S7],
                [GZ, G9, SZ, SU],
                [GZ, G9, SZ, EZ],
                // [GZ, G9, SZ, EK], // covered by [.., .., .., EZ]
                // [GZ, G9, SZ, EO], // covered by [.., .., .., EZ]
                [GZ, G9, SZ, E8],
                // [GZ, G9, SZ, E7], // covered by [.., .., .., E8]
                [GZ, G9, SZ, SA],
                [GZ, G9, SZ, S7],
                [GZ, G9, S9, SU],
                [GZ, G9, S9, EZ],
                // [GZ, G9, S9, EK], // covered by [.., .., .., EZ]
                // [GZ, G9, S9, EO], // covered by [.., .., .., EZ]
                [GZ, G9, S9, E8],
                // [GZ, G9, S9, E7], // covered by [.., .., .., E8]
                [GZ, G9, S9, SA],
                [GZ, G9, S9, S7],
                [GZ, G7, EU, SU], // TODO should not be needed (GA better than GZ)
                // [GZ, G7, EU, EZ], // covered by [.., .., .., EO]
                // [GZ, G7, EU, EK], // covered by [.., .., .., EO]
                [GZ, G7, EU, EO], // TODO should not be needed (GA better than GZ)
                [GZ, G7, EU, E8], // TODO should not be needed (GA better than GZ)
                // [GZ, G7, EU, E7], // covered by [.., .., .., E8]
                [GZ, G7, EU, SA], // TODO should not be needed (GA better than GZ)
                [GZ, G7, EU, S7], // TODO should not be needed (GA better than GZ)
                [GZ, G7, HU, SU], // TODO should not be needed (GA better than GZ)
                // [GZ, G7, HU, EZ], // covered by [.., .., .., EO]
                // [GZ, G7, HU, EK], // covered by [.., .., .., EO]
                [GZ, G7, HU, EO], // TODO should not be needed (GA better than GZ)
                [GZ, G7, HU, E8], // TODO should not be needed (GA better than GZ)
                // [GZ, G7, HU, E7], // covered by [.., .., .., E8]
                [GZ, G7, HU, SA], // TODO should not be needed (GA better than GZ)
                [GZ, G7, HU, S7], // TODO should not be needed (GA better than GZ)
                [GZ, G7, EA, SU],
                // [GZ, G7, EA, EZ], // covered by [.., .., .., EO]
                // [GZ, G7, EA, EK], // covered by [.., .., .., EO]
                [GZ, G7, EA, EO],
                [GZ, G7, EA, E8],
                // [GZ, G7, EA, E7], // covered by [.., .., .., E8]
                [GZ, G7, EA, SA],
                [GZ, G7, EA, S7],
                [GZ, G7, E9, SU],
                [GZ, G7, E9, EZ],
                // [GZ, G7, E9, EK], // covered by [.., .., .., EZ]
                // [GZ, G7, E9, EO], // covered by [.., .., .., EZ]
                [GZ, G7, E9, E8],
                // [GZ, G7, E9, E7], // covered by [.., .., .., E8]
                [GZ, G7, E9, SA],
                [GZ, G7, E9, S7],
                [GZ, G7, HZ, SU],
                [GZ, G7, HZ, EZ],
                // [GZ, G7, HZ, EK], // covered by [.., .., .., EZ]
                // [GZ, G7, HZ, EO], // covered by [.., .., .., EZ]
                [GZ, G7, HZ, E8],
                // [GZ, G7, HZ, E7], // covered by [.., .., .., E8]
                [GZ, G7, HZ, SA],
                [GZ, G7, HZ, S7],
                [GZ, G7, HO, SU],
                [GZ, G7, HO, EZ],
                // [GZ, G7, HO, EK], // covered by [.., .., .., EZ]
                // [GZ, G7, HO, EO], // covered by [.., .., .., EZ]
                [GZ, G7, HO, E8],
                // [GZ, G7, HO, E7], // covered by [.., .., .., E8]
                [GZ, G7, HO, SA],
                [GZ, G7, HO, S7],
                [GZ, G7, SZ, SU],
                [GZ, G7, SZ, EZ],
                // [GZ, G7, SZ, EK], // covered by [.., .., .., EZ]
                // [GZ, G7, SZ, EO], // covered by [.., .., .., EZ]
                [GZ, G7, SZ, E8],
                // [GZ, G7, SZ, E7], // covered by [.., .., .., E8]
                [GZ, G7, SZ, SA],
                [GZ, G7, SZ, S7],
                [GZ, G7, S9, SU],
                [GZ, G7, S9, EZ],
                // [GZ, G7, S9, EK], // covered by [.., .., .., EZ]
                // [GZ, G7, S9, EO], // covered by [.., .., .., EZ]
                [GZ, G7, S9, E8],
                // [GZ, G7, S9, E7], // covered by [.., .., .., E8]
                [GZ, G7, S9, SA],
                [GZ, G7, S9, S7],
                [G8, GK, EU, SU],
                // [G8, GK, EU, EZ], // covered by [.., .., .., EO]
                // [G8, GK, EU, EK], // covered by [.., .., .., EO]
                [G8, GK, EU, EO],
                [G8, GK, EU, E8],
                // [G8, GK, EU, E7], // covered by [.., .., .., E8]
                [G8, GK, EU, SA],
                [G8, GK, EU, S7],
                [G8, GK, HU, SU],
                // [G8, GK, HU, EZ], // covered by [.., .., .., EO]
                // [G8, GK, HU, EK], // covered by [.., .., .., EO]
                [G8, GK, HU, EO],
                [G8, GK, HU, E8],
                // [G8, GK, HU, E7], // covered by [.., .., .., E8]
                [G8, GK, HU, SA],
                [G8, GK, HU, S7],
                [G8, GK, EA, SU],
                // [G8, GK, EA, EZ], // covered by [.., .., .., EO]
                // [G8, GK, EA, EK], // covered by [.., .., .., EO]
                [G8, GK, EA, EO],
                [G8, GK, EA, E8],
                // [G8, GK, EA, E7], // covered by [.., .., .., E8]
                [G8, GK, EA, SA],
                [G8, GK, EA, S7],
                [G8, GK, E9, SU],
                [G8, GK, E9, EZ],
                // [G8, GK, E9, EK], // covered by [.., .., .., EZ]
                // [G8, GK, E9, EO], // covered by [.., .., .., EZ]
                [G8, GK, E9, E8],
                // [G8, GK, E9, E7], // covered by [.., .., .., E8]
                [G8, GK, E9, SA],
                [G8, GK, E9, S7],
                [G8, GK, HZ, SU],
                [G8, GK, HZ, EZ],
                // [G8, GK, HZ, EK], // covered by [.., .., .., EZ]
                // [G8, GK, HZ, EO], // covered by [.., .., .., EZ]
                [G8, GK, HZ, E8],
                // [G8, GK, HZ, E7], // covered by [.., .., .., E8]
                [G8, GK, HZ, SA],
                [G8, GK, HZ, S7],
                [G8, GK, HO, SU],
                [G8, GK, HO, EZ],
                // [G8, GK, HO, EK], // covered by [.., .., .., EZ]
                // [G8, GK, HO, EO], // covered by [.., .., .., EZ]
                [G8, GK, HO, E8],
                // [G8, GK, HO, E7], // covered by [.., .., .., E8]
                [G8, GK, HO, SA],
                [G8, GK, HO, S7],
                [G8, GK, SZ, SU],
                [G8, GK, SZ, EZ],
                // [G8, GK, SZ, EK], // covered by [.., .., .., EZ]
                // [G8, GK, SZ, EO], // covered by [.., .., .., EZ]
                [G8, GK, SZ, E8],
                // [G8, GK, SZ, E7], // covered by [.., .., .., E8]
                [G8, GK, SZ, SA],
                [G8, GK, SZ, S7],
                [G8, GK, S9, SU],
                [G8, GK, S9, EZ],
                // [G8, GK, S9, EK], // covered by [.., .., .., EZ]
                // [G8, GK, S9, EO], // covered by [.., .., .., EZ]
                [G8, GK, S9, E8],
                // [G8, GK, S9, E7], // covered by [.., .., .., E8]
                [G8, GK, S9, SA],
                [G8, GK, S9, S7],
                [G8, GO, EU, SU], // TODO should not be needed (GK better than GO)
                // [G8, GO, EU, EZ], // covered by [.., .., .., EO]
                // [G8, GO, EU, EK], // covered by [.., .., .., EO]
                [G8, GO, EU, EO], // TODO should not be needed (GK better than GO)
                [G8, GO, EU, E8], // TODO should not be needed (GK better than GO)
                // [G8, GO, EU, E7], // covered by [.., .., .., E8]
                [G8, GO, EU, SA], // TODO should not be needed (GK better than GO)
                [G8, GO, EU, S7], // TODO should not be needed (GK better than GO)
                [G8, GO, HU, SU],
                // [G8, GO, HU, EZ], // covered by [.., .., .., EO]
                // [G8, GO, HU, EK], // covered by [.., .., .., EO]
                [G8, GO, HU, EO],
                [G8, GO, HU, E8],
                // [G8, GO, HU, E7], // covered by [.., .., .., E8]
                [G8, GO, HU, SA],
                [G8, GO, HU, S7],
                [G8, GO, EA, SU],
                // [G8, GO, EA, EZ], // covered by [.., .., .., EO]
                // [G8, GO, EA, EK], // covered by [.., .., .., EO]
                [G8, GO, EA, EO],
                [G8, GO, EA, E8],
                // [G8, GO, EA, E7], // covered by [.., .., .., E8]
                [G8, GO, EA, SA],
                [G8, GO, EA, S7],
                [G8, GO, E9, SU],
                [G8, GO, E9, EZ],
                // [G8, GO, E9, EK], // covered by [.., .., .., EZ]
                // [G8, GO, E9, EO], // covered by [.., .., .., EZ]
                [G8, GO, E9, E8],
                // [G8, GO, E9, E7], // covered by [.., .., .., E8]
                [G8, GO, E9, SA],
                [G8, GO, E9, S7],
                [G8, GO, HZ, SU],
                [G8, GO, HZ, EZ],
                // [G8, GO, HZ, EK], // covered by [.., .., .., EZ]
                // [G8, GO, HZ, EO], // covered by [.., .., .., EZ]
                [G8, GO, HZ, E8],
                // [G8, GO, HZ, E7], // covered by [.., .., .., E8]
                [G8, GO, HZ, SA],
                [G8, GO, HZ, S7],
                [G8, GO, HO, SU],
                [G8, GO, HO, EZ],
                // [G8, GO, HO, EK], // covered by [.., .., .., EZ]
                // [G8, GO, HO, EO], // covered by [.., .., .., EZ]
                [G8, GO, HO, E8],
                // [G8, GO, HO, E7], // covered by [.., .., .., E8]
                [G8, GO, HO, SA],
                [G8, GO, HO, S7],
                [G8, GO, SZ, SU],
                [G8, GO, SZ, EZ],
                // [G8, GO, SZ, EK], // covered by [.., .., .., EZ]
                // [G8, GO, SZ, EO], // covered by [.., .., .., EZ]
                [G8, GO, SZ, E8],
                // [G8, GO, SZ, E7], // covered by [.., .., .., E8]
                [G8, GO, SZ, SA],
                [G8, GO, SZ, S7],
                [G8, GO, S9, SU],
                [G8, GO, S9, EZ],
                // [G8, GO, S9, EK], // covered by [.., .., .., EZ]
                // [G8, GO, S9, EO], // covered by [.., .., .., EZ]
                [G8, GO, S9, E8],
                // [G8, GO, S9, E7], // covered by [.., .., .., E8]
                [G8, GO, S9, SA],
                [G8, GO, S9, S7],
                [G8, G9, EU, SU], // TODO should not be needed (GK better than G9)
                // [G8, G9, EU, EZ], // covered by [.., .., .., EO]
                // [G8, G9, EU, EK], // covered by [.., .., .., EO]
                [G8, G9, EU, EO], // TODO should not be needed (GK better than G9)
                [G8, G9, EU, E8], // TODO should not be needed (GK better than G9)
                // [G8, G9, EU, E7], // covered by [.., .., .., E8]
                [G8, G9, EU, SA], // TODO should not be needed (GK better than G9)
                [G8, G9, EU, S7], // TODO should not be needed (GK better than G9)
                [G8, G9, HU, SU],
                // [G8, G9, HU, EZ], // covered by [.., .., .., EO]
                // [G8, G9, HU, EK], // covered by [.., .., .., EO]
                [G8, G9, HU, EO],
                [G8, G9, HU, E8],
                // [G8, G9, HU, E7], // covered by [.., .., .., E8]
                [G8, G9, HU, SA],
                [G8, G9, HU, S7],
                [G8, G9, EA, SU],
                // [G8, G9, EA, EZ], // covered by [.., .., .., EO]
                // [G8, G9, EA, EK], // covered by [.., .., .., EO]
                [G8, G9, EA, EO],
                [G8, G9, EA, E8],
                // [G8, G9, EA, E7], // covered by [.., .., .., E8]
                [G8, G9, EA, SA],
                [G8, G9, EA, S7],
                [G8, G9, E9, SU],
                [G8, G9, E9, EZ],
                // [G8, G9, E9, EK], // covered by [.., .., .., EZ]
                // [G8, G9, E9, EO], // covered by [.., .., .., EZ]
                [G8, G9, E9, E8],
                // [G8, G9, E9, E7], // covered by [.., .., .., E8]
                [G8, G9, E9, SA],
                [G8, G9, E9, S7],
                [G8, G9, HZ, SU],
                [G8, G9, HZ, EZ],
                // [G8, G9, HZ, EK], // covered by [.., .., .., EZ]
                // [G8, G9, HZ, EO], // covered by [.., .., .., EZ]
                [G8, G9, HZ, E8],
                // [G8, G9, HZ, E7], // covered by [.., .., .., E8]
                [G8, G9, HZ, SA],
                [G8, G9, HZ, S7],
                [G8, G9, HO, SU],
                [G8, G9, HO, EZ],
                // [G8, G9, HO, EK], // covered by [.., .., .., EZ]
                // [G8, G9, HO, EO], // covered by [.., .., .., EZ]
                [G8, G9, HO, E8],
                // [G8, G9, HO, E7], // covered by [.., .., .., E8]
                [G8, G9, HO, SA],
                [G8, G9, HO, S7],
                [G8, G9, SZ, SU],
                [G8, G9, SZ, EZ],
                // [G8, G9, SZ, EK], // covered by [.., .., .., EZ]
                // [G8, G9, SZ, EO], // covered by [.., .., .., EZ]
                [G8, G9, SZ, E8],
                // [G8, G9, SZ, E7], // covered by [.., .., .., E8]
                [G8, G9, SZ, SA],
                [G8, G9, SZ, S7],
                [G8, G9, S9, SU],
                [G8, G9, S9, EZ],
                // [G8, G9, S9, EK], // covered by [.., .., .., EZ]
                // [G8, G9, S9, EO], // covered by [.., .., .., EZ]
                [G8, G9, S9, E8],
                // [G8, G9, S9, E7], // covered by [.., .., .., E8]
                [G8, G9, S9, SA],
                [G8, G9, S9, S7],
                [G8, G7, EU, SU],
                // [G8, G7, EU, EZ], // covered by [.., .., .., EO]
                // [G8, G7, EU, EK], // covered by [.., .., .., EO]
                [G8, G7, EU, EO],
                [G8, G7, EU, E8],
                // [G8, G7, EU, E7], // covered by [.., .., .., E8]
                [G8, G7, EU, SA],
                [G8, G7, EU, S7],
                [G8, G7, HU, SU],
                // [G8, G7, HU, EZ], // covered by [.., .., .., EO]
                // [G8, G7, HU, EK], // covered by [.., .., .., EO]
                [G8, G7, HU, EO],
                [G8, G7, HU, E8],
                // [G8, G7, HU, E7], // covered by [.., .., .., E8]
                [G8, G7, HU, SA],
                [G8, G7, HU, S7],
                [G8, G7, EA, SU],
                // [G8, G7, EA, EZ], // covered by [.., .., .., EO]
                // [G8, G7, EA, EK], // covered by [.., .., .., EO]
                [G8, G7, EA, EO],
                [G8, G7, EA, E8],
                // [G8, G7, EA, E7], // covered by [.., .., .., E8]
                [G8, G7, EA, SA],
                [G8, G7, EA, S7],
                [G8, G7, E9, SU],
                [G8, G7, E9, EZ],
                // [G8, G7, E9, EK], // covered by [.., .., .., EZ]
                // [G8, G7, E9, EO], // covered by [.., .., .., EZ]
                [G8, G7, E9, E8],
                // [G8, G7, E9, E7], // covered by [.., .., .., E8]
                [G8, G7, E9, SA],
                [G8, G7, E9, S7],
                [G8, G7, HZ, SU],
                [G8, G7, HZ, EZ],
                // [G8, G7, HZ, EK], // covered by [.., .., .., EZ]
                // [G8, G7, HZ, EO], // covered by [.., .., .., EZ]
                [G8, G7, HZ, E8],
                // [G8, G7, HZ, E7], // covered by [.., .., .., E8]
                [G8, G7, HZ, SA],
                [G8, G7, HZ, S7],
                [G8, G7, HO, SU],
                [G8, G7, HO, EZ],
                // [G8, G7, HO, EK], // covered by [.., .., .., EZ]
                // [G8, G7, HO, EO], // covered by [.., .., .., EZ]
                [G8, G7, HO, E8],
                // [G8, G7, HO, E7], // covered by [.., .., .., E8]
                [G8, G7, HO, SA],
                [G8, G7, HO, S7],
                [G8, G7, SZ, SU],
                [G8, G7, SZ, EZ],
                // [G8, G7, SZ, EK], // covered by [.., .., .., EZ]
                // [G8, G7, SZ, EO], // covered by [.., .., .., EZ]
                [G8, G7, SZ, E8],
                // [G8, G7, SZ, E7], // covered by [.., .., .., E8]
                [G8, G7, SZ, SA],
                [G8, G7, SZ, S7],
                [G8, G7, S9, SU],
                [G8, G7, S9, EZ],
                // [G8, G7, S9, EK], // covered by [.., .., .., EZ]
                // [G8, G7, S9, EO], // covered by [.., .., .., EZ]
                [G8, G7, S9, E8],
                // [G8, G7, S9, E7], // covered by [.., .., .., E8]
                [G8, G7, S9, SA],
                [G8, G7, S9, S7],
                [HA, H9, HZ, SU],
                [HA, H9, HZ, EZ],
                // [HA, H9, HZ, EK], // covered by [H., H., H., EZ]
                // [HA, H9, HZ, EO], // covered by [H., H., H., EZ]
                [HA, H9, HZ, E8],
                // [HA, H9, HZ, E7], // covered by [.., .., .., E8]
                [HA, H9, HZ, SA],
                [HA, H9, HZ, S7],
                [HA, H9, HO, SU],
                [HA, H9, HO, EZ],
                // [HA, H9, HO, EK], // covered by [H., H., H., EZ]
                // [HA, H9, HO, EO], // covered by [H., H., H., EZ]
                [HA, H9, HO, E8],
                // [HA, H9, HO, E7], // covered by [.., .., .., E8]
                [HA, H9, HO, SA],
                [HA, H9, HO, S7],
                [HK, H9, HZ, SU],
                [HK, H9, HZ, EZ],
                // [HK, H9, HZ, EK], // covered by [H., H., H., EZ]
                // [HK, H9, HZ, EO], // covered by [H., H., H., EZ]
                [HK, H9, HZ, E8],
                // [HK, H9, HZ, E7], // covered by [.., .., .., E8]
                [HK, H9, HZ, SA],
                [HK, H9, HZ, S7],
                [HK, H9, HO, SU],
                [HK, H9, HO, EZ],
                // [HK, H9, HO, EK], // covered by [H., H., H., EZ]
                // [HK, H9, HO, EO], // covered by [H., H., H., EZ]
                [HK, H9, HO, E8],
                // [HK, H9, HO, E7], // covered by [.., .., .., E8]
                [HK, H9, HO, SA],
                [HK, H9, HO, S7],
                [H8, H9, HZ, SU],
                [H8, H9, HZ, EZ],
                // [H8, H9, HZ, EK], // covered by [H., H., H., EZ]
                // [H8, H9, HZ, EO], // covered by [H., H., H., EZ]
                [H8, H9, HZ, E8],
                // [H8, H9, HZ, E7], // covered by [.., .., .., E8]
                [H8, H9, HZ, SA],
                [H8, H9, HZ, S7],
                [H8, H9, HO, SU],
                [H8, H9, HO, EZ],
                // [H8, H9, HO, EK], // covered by [H., H., H., EZ]
                // [H8, H9, HO, EO], // covered by [H., H., H., EZ]
                [H8, H9, HO, E8],
                // [H8, H9, HO, E7], // covered by [.., .., .., E8]
                [H8, H9, HO, SA],
                [H8, H9, HO, S7],
                // [H7, .., .., ..] // covered by [H8, .., .., ..]
                [SO, SK, SZ, SA],
                [SO, SK, SZ, S7],
                [SO, SK, S9, SA],
                [SO, SK, S9, S7],
                [SO, S8, SZ, SA],
                [SO, S8, SZ, S7],
                [SO, S8, S9, SA],
                [SO, S8, S9, S7],
            ],
        );
        assert_stichoracle(
            &rules_farbwenz_eichel_epi3,
            [
                &[GA, GZ, G8, HA, HK, H8, SO],
                &[GU, GK, GO, G9, G7, SK, S8],
                &[EU, HU, EA, E9, HZ, SZ, S9],
                &[SU, EZ, EK, EO, E8, E7, SA],
            ],
            &[H7, H9, HO, S7],
            &[
                [EU, SU, GA, GU],
                // [EU, SU, GZ, GU], // covered by [.., .., GA, GU]
                [EU, SU, G8, GU],
                [EU, SU, HA, GU],
                [EU, SU, HK, GU],
                // [EU, SU, H8, GU], // covered by [.., .., HK, ..]
                [EU, SU, SO, GU],
                // [EU, EZ, GA, GU], // covered by [.., EO, .., GU]
                // [EU, EZ, GZ, GU], // covered by [.., .., GA, GU]
                // [EU, EZ, G8, GU], // covered by [.., EO, .., GU]
                // [EU, EZ, HA, GU], // covered by [.., EO, .., GU]
                // [EU, EZ, HK, GU], // covered by [.., EO, .., GU]
                // [EU, EZ, H8, GU], // covered by [.., EO, .., GU]
                // [EU, EZ, SO, GU], // covered by [.., EO, .., GU]
                // [EU, EK, GA, GU], // covered by [.., EO, .., GU]
                // [EU, EK, GZ, GU], // covered by [.., .., GA, GU]
                // [EU, EK, G8, GU], // covered by [.., EO, .., GU]
                // [EU, EK, HA, GU], // covered by [.., EO, .., GU]
                // [EU, EK, HK, GU], // covered by [.., EO, .., GU]
                // [EU, EK, H8, GU], // covered by [.., EO, .., GU]
                // [EU, EK, SO, GU], // covered by [.., EO, .., GU]
                [EU, EO, GA, GU],
                // [EU, EO, GZ, GU], // covered by [.., .., GA, GU]
                [EU, EO, G8, GU],
                [EU, EO, HA, GU],
                [EU, EO, HK, GU],
                // [EU, EO, H8, GU], // covered by [.., .., HK, ..]
                [EU, EO, SO, GU],
                [EU, E8, GA, GU],
                // [EU, E8, GZ, GU], // covered by [.., .., GA, GU]
                [EU, E8, G8, GU],
                [EU, E8, HA, GU],
                [EU, E8, HK, GU],
                // [EU, E8, H8, GU], // covered by [.., .., HK, ..]
                [EU, E8, SO, GU],
                // [EU, E7, GA, GU], // covered by [.., E8, .., ..]
                // [EU, E7, GZ, GU], // covered by [.., .., GA, GU]
                // [EU, E7, G8, GU], // covered by [.., E8, .., ..]
                // [EU, E7, HA, GU], // covered by [.., E8, .., ..]
                // [EU, E7, HK, GU], // covered by [.., E8, .., ..]
                // [EU, E7, H8, GU], // covered by [.., E8, .., ..]
                // [EU, E7, SO, GU], // covered by [.., E8, .., ..]
                [HU, SU, GA, GU],
                // [HU, SU, GZ, GU], // covered by [.., .., GA, GU]
                [HU, SU, G8, GU],
                [HU, SU, HA, GU],
                [HU, SU, HK, GU],
                // [HU, SU, H8, GU], // covered by [.., .., HK, ..]
                [HU, SU, SO, GU],
                // [HU, EZ, GA, GU], // covered by [.U, EO, .., GU]
                // [HU, EZ, GZ, GU], // covered by [.., .., GA, GU]
                // [HU, EZ, G8, GU], // covered by [.U, EO, .., GU]
                // [HU, EZ, HA, GU], // covered by [.U, EO, .., GU]
                // [HU, EZ, HK, GU], // covered by [.U, EO, .., GU]
                // [HU, EZ, H8, GU], // covered by [.., .., HK, ..]
                // [HU, EZ, SO, GU], // covered by [.U, EO, .., GU]
                // [HU, EK, GA, GU], // covered by [.U, EO, .., GU]
                // [HU, EK, GZ, GU], // covered by [.., .., GA, GU]
                // [HU, EK, G8, GU], // covered by [.U, EO, .., GU]
                // [HU, EK, HA, GU], // covered by [.U, EO, .., GU]
                // [HU, EK, HK, GU], // covered by [.U, EO, .., GU]
                // [HU, EK, H8, GU], // covered by [.., .., HK, ..]
                // [HU, EK, SO, GU], // covered by [.U, EO, .., GU]
                [HU, EO, GA, GU],
                // [HU, EO, GZ, GU], // covered by [.., .., GA, GU]
                [HU, EO, G8, GU],
                [HU, EO, HA, GU],
                [HU, EO, HK, GU],
                // [HU, EO, H8, GU], // covered by [.., .., HK, ..]
                [HU, EO, SO, GU],
                [HU, E8, GA, GU],
                // [HU, E8, GZ, GU], // covered by [.., .., GA, GU]
                [HU, E8, G8, GU],
                [HU, E8, HA, GU],
                [HU, E8, HK, GU],
                // [HU, E8, H8, GU], // covered by [.., .., HK, ..]
                [HU, E8, SO, GU],
                // [HU, E7, GA, GU], // covered by [.., E8, .., ..]
                // [HU, E7, GZ, GU], // covered by [.., .., GA, GU]
                // [HU, E7, G8, GU], // covered by [.., E8, .., ..]
                // [HU, E7, HA, GU], // covered by [.., E8, .., ..]
                // [HU, E7, HK, GU], // covered by [.., E8, .., ..]
                // [HU, E7, H8, GU], // covered by [.., E8, .., ..]
                // [HU, E7, SO, GU], // covered by [.., E8, .., ..]
                [EA, SU, GA, GU],
                // [EA, SU, GZ, GU], // covered by [.., .., GA, GU]
                [EA, SU, G8, GU],
                [EA, SU, HA, GU],
                [EA, SU, HK, GU],
                // [EA, SU, H8, GU], // covered by [.., .., HK, GU]
                [EA, SU, SO, GU],
                // [EA, EZ, GA, GU], // covered by [.., EO, .., GU]
                // [EA, EZ, GZ, GU], // covered by [.., .., GA, GU]
                // [EA, EZ, G8, GU], // covered by [.., EO, .., GU]
                // [EA, EZ, HA, GU], // covered by [.., EO, .., GU]
                // [EA, EZ, HK, GU], // covered by [.., EO, .., GU]
                // [EA, EZ, H8, GU], // covered by [.., EO, .., GU]
                // [EA, EZ, SO, GU], // covered by [.., EO, .., GU]
                // [EA, EK, GA, GU], // covered by [.., EO, .., GU]
                // [EA, EK, GZ, GU], // covered by [.., .., GA, GU]
                // [EA, EK, G8, GU], // covered by [.., EO, .., GU]
                // [EA, EK, HA, GU], // covered by [.., EO, .., GU]
                // [EA, EK, HK, GU], // covered by [.., EO, .., GU]
                // [EA, EK, H8, GU], // covered by [.., EO, .., GU]
                // [EA, EK, SO, GU], // covered by [.., EO, .., GU]
                [EA, EO, GA, GU],
                // [EA, EO, GZ, GU], // covered by [.., .., GA, GU]
                [EA, EO, G8, GU],
                [EA, EO, HA, GU],
                [EA, EO, HK, GU],
                // [EA, EO, H8, GU], // covered by [.., .., HK, ..]
                [EA, EO, SO, GU],
                [EA, E8, GA, GU],
                // [EA, E8, GZ, GU], // covered by [.., .., GA, GU]
                [EA, E8, G8, GU],
                [EA, E8, HA, GU],
                [EA, E8, HK, GU],
                // [EA, E8, H8, GU], // covered by [.., .., HK, ..]
                [EA, E8, SO, GU],
                // [EA, E7, GA, GU], // covered by [.., E8, .., ..]
                // [EA, E7, GZ, GU], // covered by [.., .., GA, GU]
                // [EA, E7, G8, GU], // covered by [.., E8, .., ..]
                // [EA, E7, HA, GU], // covered by [.., E8, .., ..]
                // [EA, E7, HK, GU], // covered by [.., E8, .., ..]
                // [EA, E7, H8, GU], // covered by [.., E8, .., ..]
                // [EA, E7, SO, GU], // covered by [.., E8, .., ..]
                [E9, SU, GA, GU],
                // [E9, SU, GZ, GU], // covered by [.., .., GA, GU]
                [E9, SU, G8, GU],
                [E9, SU, HA, GU],
                [E9, SU, HK, GU],
                // [E9, SU, H8, GU], // covered by [E9, SU, HK, GU]
                [E9, SU, SO, GU],
                // [E9, EZ, GA, GU], // covered by [.., EO, .., GU]
                // [E9, EZ, GZ, GU], // covered by [.., .., GA, GU]
                // [E9, EZ, G8, GU], // covered by [.., EO, .., GU]
                // [E9, EZ, HA, GU], // covered by [.., EO, .., GU]
                // [E9, EZ, HK, GU], // covered by [.., EO, .., GU]
                // [E9, EZ, H8, GU], // covered by [.., EO, .., GU]
                // [E9, EZ, SO, GU], // covered by [.., EO, .., GU]
                // [E9, EK, GA, GU], // covered by [.., EO, .., GU]
                // [E9, EK, GZ, GU], // covered by [.., .., GA, GU]
                // [E9, EK, G8, GU], // covered by [.., EO, .., GU]
                // [E9, EK, HA, GU], // covered by [.., EO, .., GU]
                // [E9, EK, HK, GU], // covered by [.., EO, .., GU]
                // [E9, EK, H8, GU], // covered by [.., EO, .., GU]
                // [E9, EK, SO, GU], // covered by [.., EO, .., GU]
                [E9, EO, GA, GU],
                // [E9, EO, GZ, GU], // covered by [.., .., GA, GU]
                [E9, EO, G8, GU],
                [E9, EO, HA, GU],
                [E9, EO, HK, GU],
                // [E9, EO, H8, GU], // covered by [E9, EO, HK, GU]
                [E9, EO, SO, GU],
                [E9, E8, GA, GU],
                // [E9, E8, GZ, GU], // covered by [.., .., GA, GU]
                [E9, E8, G8, GU],
                [E9, E8, HA, GU],
                [E9, E8, HK, GU],
                // [E9, E8, H8, GU], // covered by [E9, E8, HK, GU]
                [E9, E8, SO, GU],
                // [E9, E7, GA, GU], // covered by [.., E8, .., ..]
                // [E9, E7, GZ, GU], // covered by [.., .., GA, GU]
                // [E9, E7, G8, GU], // covered by [.., E8, .., ..]
                // [E9, E7, HA, GU], // covered by [.., E8, .., ..]
                // [E9, E7, HK, GU], // covered by [.., E8, .., ..]
                // [E9, E7, H8, GU], // covered by [.., E8, .., ..]
                // [E9, E7, SO, GU], // covered by [.., E8, .., ..]
                [HZ, SU, HA, GU],
                // [HZ, SU, HA, GK], // covered by [HZ, SU, HA, GO]
                // [HZ, SU, HA, GO], // covered by [.., .., .., G9]
                [HZ, SU, HA, G9],
                [HZ, SU, HA, G7],
                [HZ, SU, HA, SK],
                [HZ, SU, HA, S8],
                [HZ, SU, HK, GU], // TODO should not be needed (HA better than HK)
                // [HZ, SU, HK, GK], // covered by [HZ, SU, H., GO]
                // [HZ, SU, HK, GO], // covered by [.., .., .., G9]
                [HZ, SU, HK, G9],
                [HZ, SU, HK, G7],
                [HZ, SU, HK, SK],
                [HZ, SU, HK, S8],
                [HZ, SU, H8, GU],
                // [HZ, SU, H8, GK], // covered by [HZ, SU, H., GO]
                // [HZ, SU, H8, GO], // covered by [.., .., .., G9]
                [HZ, SU, H8, G9],
                [HZ, SU, H8, G7],
                [HZ, SU, H8, SK],
                [HZ, SU, H8, S8],
                [HZ, EZ, HA, GU],
                // [HZ, EZ, HA, GK], // covered by [HZ, .., .., GO]
                // [HZ, EZ, HA, GO], // covered by [.., .., .., G9]
                [HZ, EZ, HA, G9],
                [HZ, EZ, HA, G7],
                [HZ, EZ, HA, SK],
                [HZ, EZ, HA, S8],
                [HZ, EZ, HK, GU], // TODO should not be needed (HA better than HK)
                // [HZ, EZ, HK, GK], // covered by [HZ, ... .., GO]
                // [HZ, EZ, HK, GO], // covered by [.., .., .., G9]
                [HZ, EZ, HK, G9],
                [HZ, EZ, HK, G7],
                [HZ, EZ, HK, SK],
                [HZ, EZ, HK, S8],
                [HZ, EZ, H8, GU],
                // [HZ, EZ, H8, GK], // covered by [HZ, ... .., GO]
                // [HZ, EZ, H8, GO], // covered by [.., .., .., G9]
                [HZ, EZ, H8, G9],
                [HZ, EZ, H8, G7],
                [HZ, EZ, H8, SK],
                [HZ, EZ, H8, S8],
                [HZ, EK, HA, GU],
                // [HZ, EK, HA, GK], // covered by [.., .., .., GO]
                // [HZ, EK, HA, GO], // covered by [.., .., .., G9]
                [HZ, EK, HA, G9],
                [HZ, EK, HA, G7],
                [HZ, EK, HA, SK],
                [HZ, EK, HA, S8],
                [HZ, EK, HK, GU], // TODO should not be needed (HA better than HK)
                // [HZ, EK, HK, GK], // covered by [.., .., .., GO]
                // [HZ, EK, HK, GO], // covered by [.., .., .., G9]
                [HZ, EK, HK, G9],
                [HZ, EK, HK, G7],
                [HZ, EK, HK, SK],
                [HZ, EK, HK, S8],
                [HZ, EK, H8, GU],
                // [HZ, EK, H8, GK], // covered by [.., .., .., GO]
                // [HZ, EK, H8, GO], // covered by [.., .., .., G9]
                [HZ, EK, H8, G9],
                [HZ, EK, H8, G7],
                [HZ, EK, H8, SK],
                [HZ, EK, H8, S8],
                [HZ, EO, HA, GU],
                // [HZ, EO, HA, GK], // covered by [.., .., .., GO]
                // [HZ, EO, HA, GO], // covered by [.., .., .., G9]
                [HZ, EO, HA, G9],
                [HZ, EO, HA, G7],
                [HZ, EO, HA, SK],
                [HZ, EO, HA, S8],
                [HZ, EO, HK, GU], // TODO should not be needed (HA better than HK)
                // [HZ, EO, HK, GK], // covered by [.., .., .., GO]
                // [HZ, EO, HK, GO], // covered by [.., .., .., G9]
                [HZ, EO, HK, G9],
                [HZ, EO, HK, G7],
                [HZ, EO, HK, SK],
                [HZ, EO, HK, S8],
                [HZ, EO, H8, GU],
                // [HZ, EO, H8, GK], // covered by [.., .., .., GO]
                // [HZ, EO, H8, GO], // covered by [.., .., .., G9]
                [HZ, EO, H8, G9],
                [HZ, EO, H8, G7],
                [HZ, EO, H8, SK],
                [HZ, EO, H8, S8],
                [HZ, E8, HA, GU],
                // [HZ, E8, HA, GK], // covered by [.., .., .., GO]
                // [HZ, E8, HA, GO], // covered by [.., .., .., G9]
                [HZ, E8, HA, G9],
                [HZ, E8, HA, G7],
                [HZ, E8, HA, SK],
                [HZ, E8, HA, S8],
                [HZ, E8, HK, GU], // TODO should not be needed (HA better than HK)
                // [HZ, E8, HK, GK], // covered by [.., .., .., GO]
                // [HZ, E8, HK, GO], // covered by [.., .., .., G9]
                [HZ, E8, HK, G9],
                [HZ, E8, HK, G7],
                [HZ, E8, HK, SK],
                [HZ, E8, HK, S8],
                [HZ, E8, H8, GU],
                // [HZ, E8, H8, GK], // covered by [.., .., .., GO]
                // [HZ, E8, H8, GO], // covered by [.., .., .., G9]
                [HZ, E8, H8, G9],
                [HZ, E8, H8, G7],
                [HZ, E8, H8, SK],
                [HZ, E8, H8, S8],
                // [HZ, E7, HA, GU], // covered by [.., E8, .., ..]
                // [HZ, E7, HA, GK], // covered by [.., E8, .., ..]
                // [HZ, E7, HA, GO], // covered by [.., E8, .., ..]
                // [HZ, E7, HA, G9], // covered by [.., E8, .., ..]
                // [HZ, E7, HA, G7], // covered by [.., E8, .., ..]
                // [HZ, E7, HA, SK], // covered by [.., E8, .., ..]
                // [HZ, E7, HA, S8], // covered by [.., E8, .., ..]
                // [HZ, E7, HK, GU], // covered by [.., E8, .., ..]
                // [HZ, E7, HK, GK], // covered by [.., E8, .., ..]
                // [HZ, E7, HK, GO], // covered by [.., E8, .., ..]
                // [HZ, E7, HK, G9], // covered by [.., E8, .., ..]
                // [HZ, E7, HK, G7], // covered by [.., E8, .., ..]
                // [HZ, E7, HK, SK], // covered by [.., E8, .., ..]
                // [HZ, E7, HK, S8], // covered by [.., E8, .., ..]
                // [HZ, E7, H8, GU], // covered by [.., E8, .., ..]
                // [HZ, E7, H8, GK], // covered by [.., E8, .., ..]
                // [HZ, E7, H8, GO], // covered by [.., E8, .., ..]
                // [HZ, E7, H8, G9], // covered by [.., E8, .., ..]
                // [HZ, E7, H8, G7], // covered by [.., E8, .., ..]
                // [HZ, E7, H8, SK], // covered by [.., E8, .., ..]
                // [HZ, E7, H8, S8], // covered by [.., E8, .., ..]
                [HZ, SA, HA, GU],
                [HZ, SA, HA, GK],
                // [HZ, SA, HA, GO], // covered by [HZ, SA, HA, GK]
                // [HZ, SA, HA, G9], // covered by [HZ, SA, .., GK]
                [HZ, SA, HA, G7],
                [HZ, SA, HA, SK],
                [HZ, SA, HA, S8],
                [HZ, SA, HK, GU], // TODO should not be needed (HA better than HK)
                [HZ, SA, HK, GK],
                // [HZ, SA, HK, GO], // covered by [HZ, SA, HK, GK]
                // [HZ, SA, HK, G9], // covered by [HZ, SA, .., GK]
                [HZ, SA, HK, G7],
                [HZ, SA, HK, SK],
                [HZ, SA, HK, S8],
                // [HZ, SA, H8, GU], // covered by [.., .., HK, ..]
                // [HZ, SA, H8, GK], // covered by [.., .., HK, ..]
                // [HZ, SA, H8, GO], // covered by [.., .., HK, ..]
                // [HZ, SA, H8, G9], // covered by [.., .., HK, ..]
                // [HZ, SA, H8, G7], // covered by [.., .., HK, ..]
                // [HZ, SA, H8, SK], // covered by [.., .., HK, ..]
                // [HZ, SA, H8, S8], // covered by [.., .., HK, ..]
                [SZ, SA, SO, SK],
                [SZ, SA, SO, S8],
                // [S9, SA, SO, SK], // covered by [S9, SA, SO, S8]
                [S9, SA, SO, S8],
            ],
        );
        assert_stichoracle(
            &rules_farbwenz_eichel_epi3,
            [
                &[GA, GZ, G8, HA, HK, H8, SO],
                &[GU, GK, GO, G9, G7, SK, S8],
                &[EU, HU, EA, E9, HZ, SZ, S9],
                &[SU, EZ, EK, EO, E8, SA, S7],
            ],
            &[H7, H9, HO, E7],
            &[
                [SU, GA, GU, EU],
                // [SU, GA, GU, HU], // covered by [SU, .., GU, EA]
                [SU, GA, GU, EA],
                [SU, GA, GU, E9],
                // [SU, GZ, GU, ..], // covered by [SU, GA, GU, ..]
                [SU, G8, GU, EU],
                // [SU, G8, GU, HU], // covered by [SU, .., GU, EA]
                [SU, G8, GU, EA],
                [SU, G8, GU, E9],
                [SU, HA, GU, EU],
                // [SU, HA, GU, HU], // covered by [SU, .., GU, EA]
                [SU, HA, GU, EA],
                [SU, HA, GU, E9],
                [SU, HK, GU, EU],
                // [SU, HK, GU, HU], // covered by [SU, HK, GU, EA]
                [SU, HK, GU, EA],
                [SU, HK, GU, E9],
                // [SU, H8, GU, EU], // covered by [SU, HK, GU, ..]
                [SU, SO, GU, EU],
                // [SU, SO, GU, HU], // covered by [SU, .., GU, EA]
                [SU, SO, GU, EA],
                [SU, SO, GU, E9],
                // [EZ, .., .., ..], // covered by [EO, .., .., ..]
                // [EK, .., .., ..], // covered by [EO, .., .., ..]
                [EO, GA, GU, EU],
                [EO, GA, GU, HU],
                [EO, GA, GU, EA],
                [EO, GA, GU, E9],
                // [EO, GZ, GU, ..], // covered by [E., GA, GU, ..]
                [EO, G8, GU, EU],
                [EO, G8, GU, HU],
                [EO, G8, GU, EA],
                [EO, G8, GU, E9],
                [EO, HA, GU, EU],
                [EO, HA, GU, HU],
                [EO, HA, GU, EA],
                [EO, HA, GU, E9],
                [EO, HK, GU, EU],
                [EO, HK, GU, HU],
                [EO, HK, GU, EA],
                [EO, HK, GU, E9],
                // [EO, H8, GU, EU], // covered by [EO, HK, GU, ..]
                [EO, SO, GU, EU],
                [EO, SO, GU, HU],
                [EO, SO, GU, EA],
                [EO, SO, GU, E9],
                [E8, GA, GU, EU],
                [E8, GA, GU, HU],
                [E8, GA, GU, EA],
                [E8, GA, GU, E9],
                // [E8, GZ, GU, ..], // covered by [E., GA, GU, ..]
                [E8, G8, GU, EU],
                [E8, G8, GU, HU],
                [E8, G8, GU, EA],
                [E8, G8, GU, E9],
                [E8, HA, GU, EU],
                [E8, HA, GU, HU],
                [E8, HA, GU, EA],
                [E8, HA, GU, E9],
                [E8, HK, GU, EU],
                [E8, HK, GU, HU],
                [E8, HK, GU, EA],
                [E8, HK, GU, E9],
                // [E8, H8, GU, EU], // covered by [E8, HK, GU, ..]
                [E8, SO, GU, EU],
                [E8, SO, GU, HU],
                [E8, SO, GU, EA],
                [E8, SO, GU, E9],
                // [SA, SO, SK, SZ], // covered by [SA, SO, SK, S9]
                [SA, SO, SK, S9],
                [SA, SO, S8, SZ],
                [SA, SO, S8, S9],
                [S7, SO, SK, SZ],
                [S7, SO, SK, S9],
                [S7, SO, S8, SZ],
                [S7, SO, S8, S9],
            ],
        );
    }

    #[test]
    fn test_filterbystichoracle() {
        crate::game::run::internal_run_simple_game_loop( // TODO simplify all this, and explicitly iterate over supported rules
            EPlayerIndex::map_from_fn(|_epi| Box::new(SPlayerRandom::new(
                /*fn_check_ask_for_card*/|game: &SGameGeneric<SRuleSet, (), ()>| {
                    if game.kurzlang().cards_per_player() - if_dbg_else!({4}{5}) < game.completed_stichs().len() {
                        //let epi = unwrap!(game.current_playable_stich().current_playerindex());
                        macro_rules! fwd{($ty_fn_make_filter:tt, $fn_make_filter:expr,) => {
                            unwrap!(determine_best_card::<$ty_fn_make_filter,_,_,_,_,_,_,_>(
                                &game.stichseq,
                                &game.rules,
                                Box::new(std::iter::once(game.ahand.clone())) as Box<_>,
                                $fn_make_filter,
                                &SMinReachablePayout::new_from_game(game),
                                /*fn_snapshotcache*/SSnapshotCacheNone::factory(),
                                SNoVisualization::factory(),
                                /*fn_inspect*/&|_,_,_,_| {},
                                unwrap!(game.stichseq.current_stich().current_playerindex()),
                                /*fn_payout*/&|_stichseq, _ahand, n_payout| (n_payout, ()),
                            ))
                                .cards_and_ts()
                                .map(|(card, payoutstatsperstrategy)| (
                                    card,
                                    verify_eq!(
                                        &payoutstatsperstrategy.maxselfishmin.0,
                                        &payoutstatsperstrategy.maxselfishmax.0
                                    ).clone()
                                ))
                                .collect::<Vec<_>>()
                        }}
                        assert_eq!(
                            fwd!(
                                SFilterByOracle,
                                /*fn_make_filter*/|stichseq, ahand| {
                                    SFilterByOracle::new(&game.rules, ahand, stichseq)
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
                            let gamepreparations = gamepreparations.clone();
                            VStockOrT::OrT(
                                SGameGeneric::new_with(
                                    gamepreparations.aveccard.clone(),
                                    gamepreparations.expensifiers.clone(),
                                    rules.clone().into(),
                                    gamepreparations.ruleset,
                                    /*gameannouncements*/(),
                                    /*determinerules*/(),
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
            /*fn_print_account_balance*/|_,_| {/* no output */},
        );
    }
}
