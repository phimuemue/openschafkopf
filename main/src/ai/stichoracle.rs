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
    ai::SRuleStateCacheFixed,
};
use arrayvec::ArrayVec;

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

    fn traverse_trie(&self, stich: &mut SStich) -> Vec<SStich> {
        if verify_eq!(stich.is_full(), self.vectplcardtrie.is_empty()) {
            vec![stich.clone()]
        } else {
            let mut vecstich = Vec::new();
            for (card, stichtrie_child) in self.vectplcardtrie.iter() {
                stich.push(*card);
                vecstich.extend(stichtrie_child.traverse_trie(stich));
                stich.undo_most_recent();
            }
            use itertools::Itertools;
            debug_assert!(vecstich.iter().all_unique());
            vecstich
        }
    }

    pub fn new_with(
        ahand: &mut EnumMap<EPlayerIndex, SHand>,
        stichseq: &mut SStichSequence,
        rules: &dyn TRules,
        enumchainscard_completed_cards: &SEnumChains<SCard>,
        playerparties: &SPlayerPartiesTable,
    ) -> Self {
        fn for_each_allowed_card(
            n_depth: usize, // TODO? static enum type, possibly difference of EPlayerIndex
            ahand: &mut EnumMap<EPlayerIndex, SHand>,
            stichseq: &mut SStichSequence,
            rules: &dyn TRules,
            enumchainscard_completed_cards: &SEnumChains<SCard>,
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
                fn remove_from_allowed(veccard: &mut SHandVector, card_remove: SCard) {
                    let i_card = unwrap!(veccard.iter().position(|card| card==&card_remove));
                    veccard.swap_remove(i_card);
                }
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
                            enumchainscard_completed_cards,
                            playerparties,
                        );
                        ahand[epi_card].add_card(card_representative);
                        tplstichtrieob_stich_winner_primary_party
                    });
                    let mut enumchainscard_actual = enumchainscard_completed_cards.clone(); // TODO avoid cloning.
                    let epi_preliminary_winner = rules.preliminary_winner_index(stichseq.current_stich());
                    for (epi, card) in stichseq.current_stich().iter() {
                        if epi!=epi_preliminary_winner {
                            enumchainscard_actual.remove_from_chain(*card);
                        }
                    }
                    let next_in_chain = |veccard: &SHandVector, card_chain| {
                        enumchainscard_actual.next(card_chain)
                            .filter(|card| veccard.contains(card))
                    };
                    match ob_stich_winner_primary_party_representative {
                        None => {
                            let mut card_chain = enumchainscard_actual.prev_while(
                                card_representative,
                                |card| veccard_allowed.contains(&card),
                            );
                            let i_stichtrie_representative = stichtrie.vectplcardtrie.len();
                            stichtrie.vectplcardtrie.push((
                                card_chain,
                                stichtrie_representative,
                            ));
                            let mut ab_points = [false; 12]; // TODO? couple with points_card
                            ab_points[points_card(card_chain).as_num::<usize>()]=true;
                            remove_from_allowed(&mut veccard_allowed, card_chain);
                            while let Some(card_chain_next) = next_in_chain(&veccard_allowed, card_chain) {
                                if !ab_points[points_card(card_chain_next).as_num::<usize>()] {
                                    ab_points[points_card(card_chain_next).as_num::<usize>()]=true;
                                    stichtrie.vectplcardtrie.push((
                                        card_chain_next,
                                        stichtrie.vectplcardtrie[i_stichtrie_representative].1.clone(),
                                    ));
                                }
                                remove_from_allowed(&mut veccard_allowed, card_chain_next);
                                card_chain = card_chain_next;
                            }
                            stichwinnerprimaryparty = VStichWinnerPrimaryParty::Different;
                        },
                        Some(b_stich_winner_primary_party) => {
                            // TODO avoid backward-forward iteration
                            let mut card_chain = enumchainscard_actual.prev_while(
                                card_representative,
                                |card| veccard_allowed.contains(&card),
                            );
                            let is_primary_party = |epi| playerparties.is_primary_party(epi);
                            let card_min_or_max = if b_stich_winner_primary_party==is_primary_party(epi_card) {
                                // only play maximum points
                                let mut card_max_points = card_chain;
                                remove_from_allowed(&mut veccard_allowed, card_chain);
                                while let Some(card_chain_next) = next_in_chain(&veccard_allowed, card_chain) {
                                    card_chain = card_chain_next;
                                    remove_from_allowed(&mut veccard_allowed, card_chain);
                                    assign_max_by_key(
                                        &mut card_max_points,
                                        card_chain,
                                        |card| points_card(*card),
                                    );
                                }
                                card_max_points
                            } else {
                                // only play minimum points
                                let mut card_min_points = card_chain;
                                remove_from_allowed(&mut veccard_allowed, card_chain);
                                while let Some(card_chain_next) = next_in_chain(&veccard_allowed, card_chain) {
                                    card_chain = card_chain_next;
                                    remove_from_allowed(&mut veccard_allowed, card_chain);
                                    assign_min_by_key(
                                        &mut card_min_points,
                                        card_chain,
                                        |card| points_card(*card),
                                    );
                                }
                                card_min_points
                            };
                            stichtrie.vectplcardtrie.push((
                                card_min_or_max,
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
        let n_stich_size = stichseq.current_stich().size();
        //assert!(0<=n_stich_size); // trivially true
        assert!(n_stich_size<=3);
        let stich_current_check = stichseq.current_stich().clone(); // TODO? debug-only
        debug_assert_eq!(
            enumchainscard_completed_cards,
            &{
                let mut enumchainscard_check = unwrap!(rules.only_minmax_points_when_on_same_hand(
                    &SRuleStateCacheFixed::new(stichseq, ahand),
                )).0;
                for (_epi, &card) in stichseq.completed_cards() {
                    enumchainscard_check.remove_from_chain(card);
                }
                enumchainscard_check
            }
        );
        let stichtrie = for_each_allowed_card(
            4-n_stich_size,
            ahand,
            stichseq,
            rules,
            enumchainscard_completed_cards,
            playerparties,
        ).0;
        debug_assert!(stichtrie.traverse_trie(&mut stichseq.current_stich().clone()).iter().all(|stich|
            stich.equal_up_to_size(&stich_current_check, stich_current_check.size())
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
    enumchainscard_completed_cards: SEnumChains<SCard>,
    playerparties: SPlayerPartiesTable,
}

impl<'rules> SFilterByOracle<'rules> {
    pub fn new(
        rules: &'rules dyn TRules,
        ahand_in_game: &EnumMap<EPlayerIndex, SHand>,
        stichseq_in_game: &SStichSequence,
    ) -> Option<Self> {
        let mut ahand = EPlayerIndex::map_from_fn(|epi| SHand::new_from_iter(
            ahand_in_game[epi].cards().iter().copied()
                .chain(
                    stichseq_in_game.visible_cards()
                        .filter_map(|(epi_card, card)| if_then_some!(
                            epi_card==epi, *card
                        ))
                )
        ));
        let mut stichseq = SStichSequence::new(stichseq_in_game.kurzlang());
        assert!(crate::ai::ahand_vecstich_card_count_is_compatible(&stichseq, &ahand));
        rules.only_minmax_points_when_on_same_hand(
            &SRuleStateCacheFixed::new(&stichseq, &ahand),
        ).map(|(enumchainscard, playerparties)| {
            let stichtrie = SStichTrie::new_with(
                &mut ahand,
                &mut stichseq,
                rules,
                &enumchainscard,
                &playerparties,
            );
            Self {
                rules,
                ahand,
                stichseq,
                stichtrie,
                enumchainscard_completed_cards: enumchainscard,
                playerparties,
            }
        })
    }
}

impl<'rules> super::TFilterAllowedCards for SFilterByOracle<'rules> {
    type UnregisterStich = (SStichTrie, EnumMap<EPlayerIndex, SRemoved<SCard>>);
    fn register_stich(&mut self, stich: &SStich) -> Self::UnregisterStich {
        assert!(stich.is_full());
        for (epi, card) in stich.iter() {
            self.stichseq.zugeben(*card, self.rules);
            self.ahand[epi].play_card(*card);
        }
        let aremovedcard = EPlayerIndex::map_from_fn(|epi|
            self.enumchainscard_completed_cards.remove_from_chain(stich[epi])
        );
        let stichtrie = SStichTrie::new_with(
            &mut self.ahand,
            &mut self.stichseq,
            self.rules,
            &self.enumchainscard_completed_cards,
            &self.playerparties,
        );
        (std::mem::replace(&mut self.stichtrie, stichtrie), aremovedcard)
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
            self.enumchainscard_completed_cards.readd(removedcard);
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

mod tests {
    #[test]
    fn test_stichoracle() {
        use crate::{
            game::SStichSequence,
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
                payoutdecider::{SPayoutDeciderParams, SLaufendeParams},
                rulesrufspiel::SRulesRufspiel,
                TRules,
            },
            util::*,
            ai::SRuleStateCacheFixed,
        };
        use super::SStichTrie;
        use itertools::Itertools;
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
                .map_into(|acard| SHand::new_from_iter(acard.iter().copied()));
            let (mut enumchainscard, playerparties) = unwrap!(rules.only_minmax_points_when_on_same_hand(
                &SRuleStateCacheFixed::new(&stichseq, ahand),
            ));
            for (_epi, card) in stichseq.completed_cards() {
                enumchainscard.remove_from_chain(*card);
            }
            let stichtrie = SStichTrie::new_with(
                &mut ahand.clone(),
                &mut stichseq.clone(),
                rules,
                &enumchainscard,
                &playerparties,
            );
            let setstich_oracle = stichtrie.traverse_trie(&mut stichseq.current_stich().clone()).iter().cloned().collect::<std::collections::HashSet<_>>();
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
                    "\nHands:\n {}\n {}\n {}\n {}\nStichseq: {}\nStich{}\n{}\n",
                    &ahand[EPlayerIndex::EPI0],
                    &ahand[EPlayerIndex::EPI1],
                    &ahand[EPlayerIndex::EPI2],
                    &ahand[EPlayerIndex::EPI3],
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
                // Opening with Trumpf
                [HO, GO, EU, EO], /*[SO, GO, EU, EO],*/ [GU, GO, EU, EO], [SU, GO, EU, EO],
                [HO, HK, EU, EO], /*[SO, HK, EU, EO],*/ [GU, HK, EU, EO], [SU, HK, EU, EO],
                [HO, H8, EU, EO], /*[SO, H8, EU, EO],*/ [GU, H8, EU, EO], [SU, H8, EU, EO],
                /*[HO, H7, EU, EO],*/ /*[SO, H7, EU, EO],*/ /*[GU, H7, EU, EO],*/ /*[SU, H7, EU, EO],*/
                [HO, GO, HU, EO], /*[SO, GO, HU, EO],*/ /*[GU, GO, HU, EO],*/ [SU, GO, HU, EO],
                [HO, HK, HU, EO], /*[SO, HK, HU, EO],*/ [GU, HK, HU, EO], [SU, HK, HU, EO],
                [HO, H8, HU, EO], /*[SO, H8, HU, EO],*/ [GU, H8, HU, EO], [SU, H8, HU, EO],
                /*[HO, H7, HU, EO],*/ /*[SO, H7, HU, EO],*/ /*[GU, H7, HU, EO],*/ /*[SU, H7, HU, EO],*/
                [HO, GO, HA, EO], /*[SO, GO, HA, EO],*/ [GU, GO, HA, EO], [SU, GO, HA, EO],
                [HO, HK, HA, EO], /*[SO, HK, HA, EO],*/ [GU, HK, HA, EO], [SU, HK, HA, EO],
                [HO, H8, HA, EO], /*[SO, H8, HA, EO],*/ [GU, H8, HA, EO], [SU, H8, HA, EO],
                /*[HO, H7, HA, EO],*/ /*[SO, H7, HA, EO],*/ /*[GU, H7, HA, EO],*/ /*[SU, H7, HA, EO],*/
                [HO, GO, EU, HZ], /*[SO, GO, EU, HZ],*/ [GU, GO, EU, HZ], [SU, GO, EU, HZ],
                /*[HO, HK, EU, HZ],*/ /*[SO, HK, EU, HZ],*/ [GU, HK, EU, HZ], [SU, HK, EU, HZ],
                [HO, H8, EU, HZ], /*[SO, H8, EU, HZ],*/ [GU, H8, EU, HZ], [SU, H8, EU, HZ],
                /*[HO, H7, EU, HZ],*/ /*[SO, H7, EU, HZ],*/ /*[GU, H7, EU, HZ],*/ /*[SU, H7, EU, HZ],*/
                [HO, GO, HU, HZ], /*[SO, GO, HU, HZ],*/ /*[GU, GO, HU, HZ],*/ [SU, GO, HU, HZ],
                /*[HO, HK, HU, HZ],*/ /*[SO, HK, HU, HZ],*/ /*[GU, HK, HU, HZ],*/ [SU, HK, HU, HZ],
                [HO, H8, HU, HZ], /*[SO, H8, HU, HZ],*/ [GU, H8, HU, HZ], [SU, H8, HU, HZ],
                /*[HO, H7, HU, HZ],*/ /*[SO, H7, HU, HZ],*/ /*[GU, H7, HU, HZ],*/ /*[SU, H7, HU, HZ],*/
                [HO, GO, HA, HZ], /*[SO, GO, HA, HZ],*/ [GU, GO, HA, HZ], [SU, GO, HA, HZ],
                /*[HO, HK, HA, HZ],*/ /*[SO, HK, HA, HZ],*/ /*[GU, HK, HA, HZ],*/ /*[SU, HK, HA, HZ],*/
                [HO, H8, HA, HZ], /*[SO, H8, HA, HZ],*/ [GU, H8, HA, HZ], [SU, H8, HA, HZ],
                /*[HO, H7, HA, HZ],*/ /*[SO, H7, HA, HZ],*/ /*[GU, H7, HA, HZ],*/ /*[SU, H7, HA, HZ],*/
                [HO, GO, EU, H9], /*[SO, GO, EU, H9],*/ [GU, GO, EU, H9], [SU, GO, EU, H9],
                [HO, HK, EU, H9], /*[SO, HK, EU, H9],*/ /*[GU, HK, EU, H9],*/ /*[SU, HK, EU, H9],*/
                [HO, H8, EU, H9], /*[SO, H8, EU, H9],*/ [GU, H8, EU, H9], [SU, H8, EU, H9],
                /*[HO, H7, EU, H9],*/ /*[SO, H7, EU, H9],*/ /*[GU, H7, EU, H9],*/ /*[SU, H7, EU, H9],*/
                [HO, GO, HU, H9], /*[SO, GO, HU, H9],*/ /*[GU, GO, HU, H9],*/ [SU, GO, HU, H9],
                [HO, HK, HU, H9], /*[SO, HK, HU, H9],*/ [GU, HK, HU, H9], /*[SU, HK, HU, H9],*/
                [HO, H8, HU, H9], /*[SO, H8, HU, H9],*/ [GU, H8, HU, H9], [SU, H8, HU, H9],
                /*[HO, H7, HU, H9],*/ /*[SO, H7, HU, H9],*/ /*[GU, H7, HU, H9],*/ /*[SU, H7, HU, H9],*/
                [HO, GO, HA, H9], /*[SO, GO, HA, H9],*/ [GU, GO, HA, H9], [SU, GO, HA, H9],
                [HO, HK, HA, H9], /*[SO, HK, HA, H9],*/ [GU, HK, HA, H9], [SU, HK, HA, H9],
                [HO, H8, HA, H9], /*[SO, H8, HA, H9],*/ [GU, H8, HA, H9], [SU, H8, HA, H9],
                /*[HO, H7, HA, H9],*/ /*[SO, H7, HA, H9],*/ /*[GU, H7, HA, H9],*/ /*[SU, H7, HA, H9],*/
                // Opening with Eichel
                [EK, EA, EZ, E9], [EK, EA, E7, E9], /*[EK, EA, EZ, E8], [EK, EA, E7, E8],*/
                // Opening with Gras
                [GA, GO, GZ, GK], [GA, HK, GZ, GK], [GA, H8, GZ, GK], /*[GA, H7, GZ, GK],*/ [GA, SA, GZ, GK], [GA, SK, GZ, GK], [GA, S8, GZ, GK],
                [GA, GO, G9, GK], [GA, HK, G9, GK], [GA, H8, G9, GK], /*[GA, H7, G9, GK],*/ [GA, SA, G9, GK], [GA, SK, G9, GK], [GA, S8, G9, GK],
                /*[GA, GO, G8, GK],*/ /*[GA, HK, G8, GK],*/ /*[GA, H8, G8, GK],*/ /*[GA, H7, G8, GK],*/ /*[GA, SA, G8, GK],*/ /*[GA, SK, G8, GK],*/ /*[GA, S8, G8, GK],*/
                [GA, GO, GZ, G7], [GA, HK, GZ, G7], [GA, H8, GZ, G7], /*[GA, H7, GZ, G7],*/ [GA, SA, GZ, G7], [GA, SK, GZ, G7], [GA, S8, GZ, G7],
                [GA, GO, G9, G7], [GA, HK, G9, G7], [GA, H8, G9, G7], /*[GA, H7, G9, G7],*/ [GA, SA, G9, G7], [GA, SK, G9, G7], [GA, S8, G9, G7],
                /*[GA, GO, G8, G7],*/ /*[GA, HK, G8, G7],*/ /*[GA, H8, G8, G7],*/ /*[GA, H7, G8, G7],*/ /*[GA, SA, G8, G7],*/ /*[GA, SK, G8, G7],*/ /*[GA, S8, G8, G7],*/
                // Opening with Schelln
                [S9, SA, EU, SZ], [S7, SA, EU, SZ], [S9, SK, EU, SZ],
                [S7, SK, EU, SZ], [S9, S8, EU, SZ], [S7, S8, EU, SZ],
                [S9, SA, HU, SZ], [S7, SA, HU, SZ], [S9, SK, HU, SZ],
                [S7, SK, HU, SZ], [S9, S8, HU, SZ], [S7, S8, HU, SZ],
                [S9, SA, HA, SZ], [S7, SA, HA, SZ], [S9, SK, HA, SZ],
                [S7, SK, HA, SZ], [S9, S8, HA, SZ], [S7, S8, HA, SZ],
                [S9, SA, EZ, SZ], [S7, SA, EZ, SZ], [S9, SK, EZ, SZ],
                [S7, SK, EZ, SZ], [S9, S8, EZ, SZ], [S7, S8, EZ, SZ],
                [S9, SA, E7, SZ], [S7, SA, E7, SZ], [S9, SK, E7, SZ],
                [S7, SK, E7, SZ], [S9, S8, E7, SZ], [S7, S8, E7, SZ],
                [S9, SA, GZ, SZ], [S7, SA, GZ, SZ], [S9, SK, GZ, SZ],
                [S7, SK, GZ, SZ], [S9, S8, GZ, SZ], [S7, S8, GZ, SZ],
                [S9, SA, G9, SZ], [S7, SA, G9, SZ], [S9, SK, G9, SZ],
                [S7, SK, G9, SZ], [S9, S8, G9, SZ], [S7, S8, G9, SZ],
                /*[S9, SA, G8, SZ],*/ /*[S7, SA, G8, SZ],*/ /*[S9, SK, G8, SZ],*/
                /*[S7, SK, G8, SZ],*/ /*[S9, S8, G8, SZ],*/ /*[S7, S8, G8, SZ],*/
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
                [HO, GO, EU, EO], [HO, HK, EU, EO], [HO, H8, EU, EO], /*[HO, H7, EU, EO],*/
                [HO, GO, HU, EO], [HO, HK, HU, EO], [HO, H8, HU, EO], /*[HO, H7, HU, EO],*/
                [HO, GO, HA, EO], [HO, HK, HA, EO], [HO, H8, HA, EO], /*[HO, H7, HA, EO],*/
                [HO, GO, EU, HZ], /*[HO, HK, EU, HZ],*/ [HO, H8, EU, HZ], /*[HO, H7, EU, HZ],*/
                [HO, GO, HU, HZ], /*[HO, HK, HU, HZ],*/ [HO, H8, HU, HZ], /*[HO, H7, HU, HZ],*/
                [HO, GO, HA, HZ], /*[HO, HK, HA, HZ],*/ [HO, H8, HA, HZ], /*[HO, H7, HA, HZ],*/
                [HO, GO, EU, H9], [HO, HK, EU, H9], [HO, H8, EU, H9], /*[HO, H7, EU, H9],*/
                [HO, GO, HU, H9], [HO, HK, HU, H9], [HO, H8, HU, H9], /*[HO, H7, HU, H9],*/
                [HO, GO, HA, H9], [HO, HK, HA, H9], [HO, H8, HA, H9], /*[HO, H7, HA, H9],*/
            ]
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
                [HO, GO, EU, EO], [HO, GO, HU, EO], [HO, GO, HA, EO],
                [HO, GO, EU, HZ], [HO, GO, HU, HZ], [HO, GO, HA, HZ],
                [HO, GO, EU, H9], [HO, GO, HU, H9], [HO, GO, HA, H9],
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
            &[[HO, GO, EU, EO], [HO, GO, EU, HZ], [HO, GO, EU, H9]],
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
                // opening with Trumpf
                [HK, HU, E9, EO],
                // [HK, HU, E9, HO],
                // [HK, HU, E9, SO],
                // [HK, HU, E9, GU],
                // [HK, HU, E8, EO],
                // [HK, HU, E8, HO],
                // [HK, HU, E8, SO],
                // [HK, HU, E8, GU],
                // [HK, HU, GA, EO], [HK, HU, GA, HO], [HK, HU, GA, SO], [HK, HU, GA, GU], // GA worse than GK
                // [HK, HU, GZ, EO], [HK, HU, GZ, HO], [HK, HU, GZ, SO], [HK, HU, GZ, GU], // GZ worse than GK
                [HK, HU, GK, EO],
                //[HK, HU, GK, HO],
                //[HK, HU, GK, SO],
                //[HK, HU, GK, GU],
                [HK, HU, G7, EO],
                //[HK, HU, G7, HO],
                //[HK, HU, G7, SO],
                //[HK, HU, G7, GU],
                [HK, HU, SZ, EO],
                //[HK, HU, SZ, HO],
                //[HK, HU, SZ, SO],
                //[HK, HU, SZ, GU],
                // EPI2 playing HA or HZ surely suboptimal: opponent EPI0 will win stich
                // [HK, HA, E9, EO], [HK, HA, E9, HO], [HK, HA, E9, SO], [HK, HA, E9, GU],
                // [HK, HA, E8, EO], [HK, HA, E8, HO], [HK, HA, E8, SO], [HK, HA, E8, GU],
                // [HK, HA, GA, EO], [HK, HA, GA, HO], [HK, HA, GA, SO], [HK, HA, GA, GU],
                // [HK, HA, GZ, EO], [HK, HA, GZ, HO], [HK, HA, GZ, SO], [HK, HA, GZ, GU],
                // [HK, HA, GK, EO], [HK, HA, GK, HO], [HK, HA, GK, SO], [HK, HA, GK, GU],
                // [HK, HA, G7, EO], [HK, HA, G7, HO], [HK, HA, G7, SO], [HK, HA, G7, GU],
                // [HK, HA, SZ, EO], [HK, HA, SZ, HO], [HK, HA, SZ, SO], [HK, HA, SZ, GU],
                // [HK, HZ, E9, EO], [HK, HZ, E9, HO], [HK, HZ, E9, SO], [HK, HZ, E9, GU],
                // [HK, HZ, E8, EO], [HK, HZ, E8, HO], [HK, HZ, E8, SO], [HK, HZ, E8, GU],
                // [HK, HZ, GA, EO], [HK, HZ, GA, HO], [HK, HZ, GA, SO], [HK, HZ, GA, GU],
                // [HK, HZ, GZ, EO], [HK, HZ, GZ, HO], [HK, HZ, GZ, SO], [HK, HZ, GZ, GU],
                // [HK, HZ, GK, EO], [HK, HZ, GK, HO], [HK, HZ, GK, SO], [HK, HZ, GK, GU],
                // [HK, HZ, G7, EO], [HK, HZ, G7, HO], [HK, HZ, G7, SO], [HK, HZ, G7, GU],
                // [HK, HZ, SZ, EO], [HK, HZ, SZ, HO], [HK, HZ, SZ, SO], [HK, HZ, SZ, GU],
                // Opening with H8 and H7 surely suboptimal:
                // * H9 (between HK and H8/H7) already gone
                // * GO, EU already gone, and friend EPI0 will surely win stich
                // [H8, HU, E9, EO], [H8, HU, E9, HO], [H8, HU, E9, SO], [H8, HU, E9, GU],
                // [H8, HU, E8, EO], [H8, HU, E8, HO], [H8, HU, E8, SO], [H8, HU, E8, GU],
                // [H8, HU, GA, EO], [H8, HU, GA, HO], [H8, HU, GA, SO], [H8, HU, GA, GU],
                // [H8, HU, GZ, EO], [H8, HU, GZ, HO], [H8, HU, GZ, SO], [H8, HU, GZ, GU],
                // [H8, HU, GK, EO], [H8, HU, GK, HO], [H8, HU, GK, SO], [H8, HU, GK, GU],
                // [H8, HU, G7, EO], [H8, HU, G7, HO], [H8, HU, G7, SO], [H8, HU, G7, GU],
                // [H8, HU, SZ, EO], [H8, HU, SZ, HO], [H8, HU, SZ, SO], [H8, HU, SZ, GU],
                // [H8, HA, E9, EO], [H8, HA, E9, HO], [H8, HA, E9, SO], [H8, HA, E9, GU],
                // [H8, HA, E8, EO], [H8, HA, E8, HO], [H8, HA, E8, SO], [H8, HA, E8, GU],
                // [H8, HA, GA, EO], [H8, HA, GA, HO], [H8, HA, GA, SO], [H8, HA, GA, GU],
                // [H8, HA, GZ, EO], [H8, HA, GZ, HO], [H8, HA, GZ, SO], [H8, HA, GZ, GU],
                // [H8, HA, GK, EO], [H8, HA, GK, HO], [H8, HA, GK, SO], [H8, HA, GK, GU],
                // [H8, HA, G7, EO], [H8, HA, G7, HO], [H8, HA, G7, SO], [H8, HA, G7, GU],
                // [H8, HA, SZ, EO], [H8, HA, SZ, HO], [H8, HA, SZ, SO], [H8, HA, SZ, GU],
                // [H8, HZ, E9, EO], [H8, HZ, E9, HO], [H8, HZ, E9, SO], [H8, HZ, E9, GU],
                // [H8, HZ, E8, EO], [H8, HZ, E8, HO], [H8, HZ, E8, SO], [H8, HZ, E8, GU],
                // [H8, HZ, GA, EO], [H8, HZ, GA, HO], [H8, HZ, GA, SO], [H8, HZ, GA, GU],
                // [H8, HZ, GZ, EO], [H8, HZ, GZ, HO], [H8, HZ, GZ, SO], [H8, HZ, GZ, GU],
                // [H8, HZ, GK, EO], [H8, HZ, GK, HO], [H8, HZ, GK, SO], [H8, HZ, GK, GU],
                // [H8, HZ, G7, EO], [H8, HZ, G7, HO], [H8, HZ, G7, SO], [H8, HZ, G7, GU],
                // [H8, HZ, SZ, EO], [H8, HZ, SZ, HO], [H8, HZ, SZ, SO], [H8, HZ, SZ, GU],
                // [H7, HU, E9, EO], [H7, HU, E9, HO], [H7, HU, E9, SO], [H7, HU, E9, GU],
                // [H7, HU, E8, EO], [H7, HU, E8, HO], [H7, HU, E8, SO], [H7, HU, E8, GU],
                // [H7, HU, GA, EO], [H7, HU, GA, HO], [H7, HU, GA, SO], [H7, HU, GA, GU],
                // [H7, HU, GZ, EO], [H7, HU, GZ, HO], [H7, HU, GZ, SO], [H7, HU, GZ, GU],
                // [H7, HU, GK, EO], [H7, HU, GK, HO], [H7, HU, GK, SO], [H7, HU, GK, GU],
                // [H7, HU, G7, EO], [H7, HU, G7, HO], [H7, HU, G7, SO], [H7, HU, G7, GU],
                // [H7, HU, SZ, EO], [H7, HU, SZ, HO], [H7, HU, SZ, SO], [H7, HU, SZ, GU],
                // [H7, HA, E9, EO], [H7, HA, E9, HO], [H7, HA, E9, SO], [H7, HA, E9, GU],
                // [H7, HA, E8, EO], [H7, HA, E8, HO], [H7, HA, E8, SO], [H7, HA, E8, GU],
                // [H7, HA, GA, EO], [H7, HA, GA, HO], [H7, HA, GA, SO], [H7, HA, GA, GU],
                // [H7, HA, GZ, EO], [H7, HA, GZ, HO], [H7, HA, GZ, SO], [H7, HA, GZ, GU],
                // [H7, HA, GK, EO], [H7, HA, GK, HO], [H7, HA, GK, SO], [H7, HA, GK, GU],
                // [H7, HA, G7, EO], [H7, HA, G7, HO], [H7, HA, G7, SO], [H7, HA, G7, GU],
                // [H7, HA, SZ, EO], [H7, HA, SZ, HO], [H7, HA, SZ, SO], [H7, HA, SZ, GU],
                // [H7, HZ, E9, EO], [H7, HZ, E9, HO], [H7, HZ, E9, SO], [H7, HZ, E9, GU],
                // [H7, HZ, E8, EO], [H7, HZ, E8, HO], [H7, HZ, E8, SO], [H7, HZ, E8, GU],
                // [H7, HZ, GA, EO], [H7, HZ, GA, HO], [H7, HZ, GA, SO], [H7, HZ, GA, GU],
                // [H7, HZ, GZ, EO], [H7, HZ, GZ, HO], [H7, HZ, GZ, SO], [H7, HZ, GZ, GU],
                // [H7, HZ, GK, EO], [H7, HZ, GK, HO], [H7, HZ, GK, SO], [H7, HZ, GK, GU],
                // [H7, HZ, G7, EO], [H7, HZ, G7, HO], [H7, HZ, G7, SO], [H7, HZ, G7, GU],
                // [H7, HZ, SZ, EO], [H7, HZ, SZ, HO], [H7, HZ, SZ, SO], [H7, HZ, SZ, GU],
                // Opening with Eichel
                [EA, EZ, E9, EK], // [EA, EZ, E8, EK],
                [EA, E7, E9, EK], // [EA, E7, E8, EK],
                // Opening with Schelln
                // [SA, HU, SZ, S9],
                // [SA, HU, SZ, S7],
                [SA, HA, SZ, S9],
                [SA, HA, SZ, S7],
                // [SA, HZ, SZ, S9], // HZ worse than HA
                // [SA, HZ, SZ, S7], // HZ worse than HA
                [SA, EZ, SZ, S9],
                [SA, EZ, SZ, S7],
                [SA, E7, SZ, S9],
                [SA, E7, SZ, S7],
                [SA, G9, SZ, S9],
                [SA, G9, SZ, S7],
                // [SA, G8, SZ, S9],
                // [SA, G8, SZ, S7],
                // [SK, HU, SZ, S9],
                // [SK, HU, SZ, S7],
                [SK, HA, SZ, S9],
                [SK, HA, SZ, S7],
                // [SK, HZ, SZ, S9], // HZ worse than HA
                // [SK, HZ, SZ, S7], // HZ worse than HA
                [SK, EZ, SZ, S9],
                [SK, EZ, SZ, S7],
                [SK, E7, SZ, S9],
                [SK, E7, SZ, S7],
                [SK, G9, SZ, S9],
                [SK, G9, SZ, S7],
                // [SK, G8, SZ, S9],
                // [SK, G8, SZ, S7],
                // [S8, HU, SZ, S9],
                // [S8, HU, SZ, S7],
                [S8, HA, SZ, S9],
                /*[S8, HA, SZ, S7],*/
                // [S8, HZ, SZ, S9], // HZ worse than HA
                // [S8, HZ, SZ, S7], // HZ worse than HA
                [S8, EZ, SZ, S9],
                /*[S8, EZ, SZ, S7],*/
                [S8, E7, SZ, S9],
                /*[S8, E7, SZ, S7],*/
                [S8, G9, SZ, S9],
                /*[S8, G9, SZ, S7],*/
                // [S8, G8, SZ, S9],
                // [S8, G8, SZ, S7],
            ],
        );
    }
}
