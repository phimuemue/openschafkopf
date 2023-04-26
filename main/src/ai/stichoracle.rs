use crate::{
    primitives::*,
    rules::{
        SPlayerPartiesTable,
        card_points::*,
        TRules,
        VTrumpfOrFarbe,
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
use std::collections::HashMap;
use std::borrow::Borrow;

#[derive(Debug, Clone)]
pub struct SStichTrie {
    vectplcardtrie: Box<ArrayVec<(ECard, SStichTrie), {EKurzLang::max_cards_per_player()}>>, // TODO? improve
}

#[cfg(test)]
macro_rules! test_dbg{($e:expr) => {
    //dbg!($e)
    $e
}}
#[cfg(not(test))]
macro_rules! test_dbg{($e:expr) => {$e}}

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

fn chains(cardspartition: &SCardsPartition, slccard: &[ECard]) -> Vec<Vec<ECard>> {
    let mut vecveccard = Vec::new();
    let mut veccard_src = slccard.iter().copied().collect::<SHandVector>();
    while !veccard_src.is_empty() {
        let n_veccard_src_len_before = veccard_src.len();
        let card_representative = veccard_src[0];
        let mut veccard_chain = Vec::new();
        iterate_chain(
            cardspartition,
            &mut veccard_src,
            card_representative,
            |card_chain| veccard_chain.push(card_chain),
        );
        assert!(!veccard_chain.is_empty());
        vecveccard.push(veccard_chain);
        assert!(veccard_src.len() < n_veccard_src_len_before);
    }
    assert!(!vecveccard.is_empty());
    assert!(vecveccard.iter().all(|veccard| !veccard.is_empty()));
    test_dbg!((slccard, &vecveccard));
    vecveccard
}

impl SStichTrie {
    fn new() -> Self {
        let slf = Self {
            vectplcardtrie: Box::new(ArrayVec::new()),
        };
        #[cfg(debug_assertions)] slf.assert_invariant();
        slf
    }

    fn new_from_full_stich(stich: SFullStich<&SStich>) -> Self {
        let mut stichtrie = SStichTrie::new();
        for (_epi, &card) in stich.iter()
            .collect::<Vec<_>>() // TODO avoid collect
            .into_iter() // TODO avoid into_iter
            .rev()
        {
            let stichtrie_child = std::mem::replace(&mut stichtrie, SStichTrie::new());
            stichtrie.push(card, stichtrie_child);
        }
        stichtrie
    }

    fn new_from_full_stichs<Stich: std::borrow::Borrow<SStich>>(itstich: impl IntoIterator<Item=Stich>) -> Self {
        unwrap!(
            itstich
                .into_iter()
                .map(|stich| SStichTrie::new_from_full_stich(SFullStich::new(stich.borrow())))
                .reduce(mutate_return!(SStichTrie::merge))
        )
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

    fn merge(&mut self, stichtrie_other: SStichTrie) {
        assert_eq!(self.depth_in_edges(), stichtrie_other.depth_in_edges());
        for (card, stichtrie_child_other) in stichtrie_other.vectplcardtrie.into_iter() {
            if let Some((_card, stichtrie)) = self.vectplcardtrie.iter_mut()
                .find(|tplcardtrie_child| tplcardtrie_child.0==card)
            {
                stichtrie.merge(stichtrie_child_other);
            } else {
                assert!(!self.vectplcardtrie.is_full());
                self.vectplcardtrie.push((card, stichtrie_child_other));
            }
        }
        #[cfg(debug_assertions)] self.assert_invariant();
    }

    fn internal_traverse_trie(&self, stich: &mut SStich) -> Vec<SStich> {
        debug_assert_eq!(EPlayerIndex::SIZE-stich.size(), self.depth_in_edges());
        if verify_eq!(stich.is_full(), self.vectplcardtrie.is_empty()) {
            vec![stich.clone()]
        } else {
            let mut vecstich = Vec::new();
            for (card, stichtrie_child) in self.vectplcardtrie.iter() {
                stich.push(*card);
                vecstich.extend(stichtrie_child.internal_traverse_trie(stich));
                stich.undo_most_recent();
            }
            debug_assert!(vecstich.iter().all_unique());
            vecstich
        }
    }

    fn traverse_trie(&self, epi_first: EPlayerIndex) -> Vec<SStich> {
        self.internal_traverse_trie(&mut SStich::new(epi_first))
    }

    fn make_simple(
        (ahand, stichseq): (&mut EnumMap<EPlayerIndex, SHand>, &mut SStichSequence),
        rules: &dyn TRules,
        fn_filter: &impl Fn((&EnumMap<EPlayerIndex, SHand>, &SStichSequence), SHandVector) -> SHandVector,
    ) -> Vec<SStich> {
        fn internal_make_simple(
            n_depth: usize,
            (ahand, stichseq): (&mut EnumMap<EPlayerIndex, SHand>, &mut SStichSequence),
            rules: &dyn TRules,
            fn_filter: &impl Fn((&EnumMap<EPlayerIndex, SHand>, &SStichSequence), SHandVector) -> SHandVector,
        ) -> Vec<SStich> {
            if n_depth==0 {
                assert!(stichseq.current_stich().is_empty());
                vec![unwrap!(stichseq.completed_stichs().last()).clone()]
            } else {
                let mut vecstich = Vec::new();
                let epi_card = unwrap!(stichseq.current_stich().current_playerindex());
                for card in fn_filter((ahand, stichseq), rules.all_allowed_cards(stichseq, &ahand[epi_card])) {
                    vecstich.extend(stichseq.zugeben_and_restore_with_hands(ahand, epi_card, card, rules, |ahand, stichseq| internal_make_simple(
                        n_depth - 1,
                        (ahand, stichseq),
                        rules,
                        fn_filter,
                    )));
                }
                vecstich
            }
        }
        internal_make_simple(
            /*n_depth*/EPlayerIndex::SIZE - stichseq.current_stich().size(),
            (ahand, stichseq),
            rules,
            fn_filter,
        )
    }

    fn outer_make(
        (ahand, stichseq): (&mut EnumMap<EPlayerIndex, SHand>, &mut SStichSequence),
        (rules, playerparties): (&dyn TRules, &SPlayerPartiesTable),
    ) -> Vec<SStich> {
        test_dbg!(("outer_make", display_card_slices(&ahand, &rules, ", "), &stichseq));
        let vecstich_all = Self::make_simple(
            (ahand, stichseq),
            rules,
            /*fn_filter*/&|_tplahandstichseq, veccard| veccard, // no filtering
        );
        assert!(!vecstich_all.is_empty());
        assert!(vecstich_all.iter().all(SStich::is_full));
        fn compute_cardspartition(
            (ahand, stichseq): (&EnumMap<EPlayerIndex, SHand>, &SStichSequence),
            rules: &dyn TRules,
        ) -> SCardsPartition { // TODO (a part of) cardspartition should be received as an input.
            let vecstich_all = SStichTrie::make_simple(
                (&mut ahand.clone(), &mut stichseq.clone()),
                rules,
                /*fn_filter*/&|_tplahandstichseq, veccard| veccard, // no filtering
            );
            let mut cardspartition = unwrap!(rules.only_minmax_points_when_on_same_hand(
                &SRuleStateCacheFixed::new(ahand, stichseq),
            )).0;
            for (_epi, &card) in stichseq.completed_cards() {
                cardspartition.remove_from_chain(card);
            }
            let mut mapepib_is_stich_winner = EPlayerIndex::map_from_fn(|_epi| false);
            for stich in vecstich_all.iter() {
                mapepib_is_stich_winner[rules.winner_index(SFullStich::new(stich))] = true;
            }
            for epi in EPlayerIndex::values()
                .map(|epi| stichseq.current_stich().first_playerindex().wrapping_add(epi.to_usize()))
            {
                let epi_card = unwrap!(stichseq.current_stich().current_playerindex());
                if epi!=epi_card { // do not remove own cards from chains
                    let ocard_surely_played = vecstich_all.iter()
                        .map(|stich| *unwrap!(stich.get(epi)))
                        .all_equal_item();
                    if let Some(&card_visible) = stichseq.current_stich().get(epi) {
                        assert_eq!(ocard_surely_played, Some(card_visible));
                    }
                    if let Some(card_surely_played) = ocard_surely_played {
                        if !mapepib_is_stich_winner[epi] {
                            // if epi never wins stich, there's *always* a higher card in the stich.
                            // => If the only person having higher card than epi was epi_card and
                            //    epi_card would hold the cards after epi's card again, then epi
                            //    could also win the stich => contradiction.
                            // => Thus, there is another person epi_other (different from both epi and epi_card)
                            //    who must winn the stich, meaning we can exclude epi's card from
                            //    the chain.
                            test_dbg!(stichseq.current_stich());
                            cardspartition.remove_from_chain(test_dbg!(card_surely_played));
                        }
                    }
                }
            }
            cardspartition
        }
        fn get_min_or_max_points(elohi: ELoHi, veccard: Vec<ECard>) -> ECard {
            // TODO assert that veccard is ordered from high to low cards.
            let mut itcard = veccard.into_iter();
            let mut card_min_or_max = unwrap!(itcard.next());
            // do not use Iterator::max_by_key/min_by_key: They yield the last/first max/min, respectively. We always traverse a chain from high to low cards, and choose the highest max/min.
            for card in itcard {
                match elohi {
                    ELoHi::Lo => assign_min_by_key(&mut card_min_or_max, card, |&card| points_card(card)),
                    ELoHi::Hi => assign_max_by_key(&mut card_min_or_max, card, |&card| points_card(card)),
                };
            }
            card_min_or_max
        }
        if let Some(b_remaining_players_primary) = EPlayerIndex::values()
            .map(|epi| stichseq.current_stich().first_playerindex().wrapping_add(epi.to_usize()))
            .skip(stichseq.current_stich().size())
            .map(|epi| playerparties.is_primary_party(epi))
            .all_equal_item()
        {
            type STrumpfOrFarbeProfile = (EnumMap<EPlayerIndex, EnumMap<VTrumpfOrFarbe, Vec<usize>>>, EPlayerIndex/*epi_winner*/);
            // In this case, players can cooperate.
            // * For each possible current stich:
            //   * for each trumpforfarbe:
            //     * Enumerate the remaining trumpforfarbe from high to low, and store which player has
            //       which indices on their hand after this stich is completed.
            //   => This results in an STrumpfOrFarbeProfile for each possible current stich
            // * Map each STrumpfOrFarbeProfile to a list of current sichs.
            //   => This results in a Map<STrumpfOrFarbeProfile, Vec<SStich>>
            let mut maptrumpforfarbeprofilevecstich = HashMap::<STrumpfOrFarbeProfile, Vec<SFullStich<SStich>>>::new();
            for stich in vecstich_all {
                let mut stichseq = stichseq.clone();
                let mut ahand = ahand.clone();
                for (epi, card) in SFullStich::new(stich).iter().skip(stichseq.current_stich().size()) {
                    stichseq.zugeben(*card, rules);
                    ahand[epi].play_card(*card);
                }
                assert!(crate::ai::ahand_vecstich_card_count_is_compatible(&ahand, &stichseq));
                assert!(stichseq.current_stich().is_empty());
                // TODO assert that working via VTrumpfOrFarbe is compatible with SCardsPartition
                let mut maptrumpforfarbeveccard = VTrumpfOrFarbe::map_from_fn(|_| Vec::new());
                for card in ECard::values(stichseq.kurzlang()) {
                    maptrumpforfarbeveccard[rules.trumpforfarbe(card)].push(card);
                }
                for veccard in maptrumpforfarbeveccard.iter_mut() {
                    veccard.sort_unstable_by(|card_lhs, card_rhs|
                        unwrap!(rules.compare_cards(*card_rhs, *card_lhs)) // sort descending
                    );
                }
                let mut trumpforfarbeprofile = (
                    EPlayerIndex::map_from_fn(|_epi|
                        VTrumpfOrFarbe::map_from_fn(|_trumpforfarbe| Vec::new())
                    ),
                    rules.winner_index(unwrap!(stichseq.last_completed_stich())),
                );
                let rulestatecache = SRuleStateCacheFixed::new(&ahand, &stichseq); // TODO avoid
                for trumpforfarbe in VTrumpfOrFarbe::values() {
                    for (i_card, epi) in maptrumpforfarbeveccard[trumpforfarbe.clone()]
                        .iter()
                        .filter_map(|&card| {
                            let epi = rulestatecache.who_has_card(card);
                            if_then_some!(ahand[epi].contains(card), epi)
                        })
                        .enumerate()
                    {
                        trumpforfarbeprofile.0[epi][trumpforfarbe.clone()].push(i_card);
                    }
                }
                maptrumpforfarbeprofilevecstich
                    .entry(trumpforfarbeprofile)
                    .or_default()
                    .push(SFullStich::new(Borrow::<SStich>::borrow(&unwrap!(stichseq.last_completed_stich())).clone()));
            }
            // * Traverse the Map<STrumpfOrFarbeProfile, Vec<SStich>>, and only keep one element from the Vec<SStich>:
            //   * If b_remaining_players_primary, then only keep the point-richest stich
            //   * If !b_remaining_players_primary, then only keep the point-poorest stich
            //   => Each STrumpfOrFarbeProfile contributes one stich.
            let mut vecstich_out = Vec::new();
            for (_trumpforfarbeprofile, mut vecstich) in maptrumpforfarbeprofilevecstich {
                assert!(!vecstich.is_empty());
                vecstich.sort_by(|stich_lhs, stich_rhs|
                    itertools::zip_eq(stich_lhs.iter(), stich_rhs.iter())
                        .map(|((epi_lhs, card_lhs), (epi_rhs, card_rhs))| {
                            assert_eq!(epi_lhs, epi_rhs);
                            unwrap!(rules.compare_cards(*card_rhs, *card_lhs)) // sort descending
                        })
                        .find(|&ord| ord!=std::cmp::Ordering::Equal)
                        .unwrap_or(std::cmp::Ordering::Equal)
                );
                let vecepi_winner = vecstich.iter().map(|stich| rules.winner_index(stich.as_ref())).collect::<Vec<_>>();
                let epi_winner = unwrap!(vecepi_winner.into_iter().all_equal_item());
                let mut ostich_out : Option<SStich> = None;
                for stich in vecstich.into_iter().map(SFullStich::into_inner) {
                    if let Some(stich_out) = ostich_out.as_mut() {
                        if playerparties.is_primary_party(epi_winner)==b_remaining_players_primary {
                            assign_max_by_key(stich_out, stich, |stich| points_stich(stich));
                        } else {
                            assign_min_by_key(stich_out, stich, |stich| points_stich(stich));
                        };
                    } else {
                        ostich_out = Some(stich);
                    }
                }
                vecstich_out.push(unwrap!(ostich_out));
            }
            vecstich_out
        } else if let Some(b_stich_winner_is_primary) = vecstich_all.iter()
            .map(|stich| playerparties.is_primary_party(rules.winner_index(SFullStich::new(stich))))
            .all_equal_item()
        {
            // Stich will surely go to one team.
            // => Players belonging to that team must play point-richest cards from each chain.
            // => Players not belonging to that team must play point-poor cards from each chain.
            Self::make_simple(
                (ahand, stichseq),
                rules,
                /*fn_filter*/&|(ahand, stichseq), veccard| {
                    let cardspartition = compute_cardspartition(
                        (ahand, stichseq),
                        rules,
                    );
                    chains(&cardspartition, &veccard)
                        .into_iter()
                        .map(|veccard_chain|
                            get_min_or_max_points(
                                if playerparties.is_primary_party(unwrap!(stichseq.current_stich().current_playerindex()))==b_stich_winner_is_primary {
                                    ELoHi::Hi
                                } else {
                                    ELoHi::Lo
                                },
                                veccard_chain,
                            )
                        )
                        .collect()
                },
            )
        } else {
            // Stich may go to either team, remaining players may belong to either team.
            // => For each chain, we must consider all different-point cards.
            let mut vecstich = Vec::new();
            let epi_card = unwrap!(stichseq.current_stich().current_playerindex());
            let cardspartition = compute_cardspartition((ahand, stichseq), rules);
            for veccard_chain in chains(&cardspartition, &rules.all_allowed_cards(stichseq, &ahand[epi_card])) {
                let vecstich_for_card_in_chain = stichseq.zugeben_and_restore_with_hands(ahand, epi_card, veccard_chain[0], rules, |ahand, stichseq|
                    Self::make_simple(
                        (ahand, stichseq),
                        rules,
                        /*fn_filter*/&|_epi, veccard| veccard, // no filtering
                    )
                );
                if let Some(b_stich_winner_is_primary) = vecstich_for_card_in_chain.iter()
                    .map(|stich| playerparties.is_primary_party(rules.winner_index(SFullStich::new(stich))))
                    .all_equal_item()
                {
                    let veccard_chain_relevant = test_dbg!(vec![get_min_or_max_points(
                        if playerparties.is_primary_party(epi_card)==b_stich_winner_is_primary {
                            ELoHi::Hi
                        } else {
                            ELoHi::Lo
                        },
                        veccard_chain,
                    )]);
                    let mut ab_points = [false; 12]; // TODO? couple with points_card
                    for card in veccard_chain_relevant {
                        if assign_neq(&mut ab_points[points_card(card).as_num::<usize>()], true) {
                            vecstich.extend(stichseq.zugeben_and_restore_with_hands(ahand, epi_card, card, rules, |ahand, stichseq|
                                Self::outer_make(
                                    (ahand, stichseq),
                                    (rules, playerparties),
                                )
                            ));
                        }
                    }
                } else if let Some(veccard_best_own_party) = {
                    // If a partner has cards [card_surely_win] that could *always* win the stich,
                    // => exclude stichs where epi_self played a non-point-richest card and the partner played card_surely_win :
                    // * if for each opponent card combination, one of our partners can enforce to
                    // win the stich, then, for the cards enforcing the stich win, we only need to
                    // consider epi_self's point-richest card.
                    let b_epi_card_is_primary = playerparties.is_primary_party(epi_card);
                    let vecepi_opponent = EPlayerIndex::values()
                        .filter(|&epi| playerparties.is_primary_party(epi)!=b_epi_card_is_primary)
                        .collect::<Vec<_>>();
                    let cards_from_stich = |stich: &SStich, vecepi: &[EPlayerIndex]| {
                        vecepi.iter()
                            .map(|&epi| *unwrap!(stich.get(epi)))
                            .collect::<Vec<_>>() // TODO needed?
                    };
                    let vecveccard_opponent = vecstich_for_card_in_chain.iter()
                        .map(|stich| cards_from_stich(stich, &vecepi_opponent))
                        .collect::<Vec<_>>();
                    test_dbg!(vecveccard_opponent).iter()
                        .map(|veccard_opponent| {
                            let vecstich_matching_opponent_cards = vecstich_for_card_in_chain.iter()
                                .filter(|stich| &cards_from_stich(stich, &vecepi_opponent)==veccard_opponent)
                                .collect::<Vec<_>>();
                            assert!(!vecstich_matching_opponent_cards.is_empty());
                            let veccard_better = vecstich_matching_opponent_cards.iter()
                                .filter_map(|stich| {
                                    let epi_winner = rules.winner_index(SFullStich::new(stich));
                                    if_then_some!(
                                        playerparties.is_primary_party(epi_winner)==b_epi_card_is_primary,
                                        *unwrap!(stich.get(epi_winner))
                                    )
                                })
                                .collect::<std::collections::HashSet<_>>();
                            test_dbg!(if_then_some!(!veccard_better.is_empty(), veccard_better))
                        })
                        .collect::<Option<Vec<_>>>()
                        .map(|vecveccard_better_src| {
                            assert!(!vecveccard_better_src.is_empty());
                            let mut itveccard_better_src = vecveccard_better_src.into_iter();
                            let mut veccard_better = unwrap!(itveccard_better_src.next());
                            assert!(!veccard_better.is_empty());
                            for veccard_better_src in itveccard_better_src {
                                assert!(!veccard_better_src.is_empty());
                                veccard_better.retain(|card| veccard_better_src.contains(card));
                            }
                            assert!(!veccard_better.is_empty());
                            veccard_better
                        })
                    //
                    // TODO: If partners can always enforce to lose the stich:
                    // here we exclude stichs where epi_self played non-point-poorest card and the partners played the stich-losing cards)
                } {
                    let mut vecstich_2 = Vec::new();
                    let card_richest = get_min_or_max_points(ELoHi::Hi, veccard_chain.clone());
                    let mut veccard_non_richest = Vec::new();
                    let mut ab_points = [false; 12]; // TODO? couple with points_card
                    for card in veccard_chain {
                        if assign_neq(&mut ab_points[points_card(card).as_num::<usize>()], true) {
                            vecstich_2.extend(stichseq.zugeben_and_restore_with_hands(ahand, epi_card, card, rules, |ahand, stichseq|
                                Self::outer_make(
                                    (ahand, stichseq),
                                    (rules, playerparties),
                                )
                            ));
                            if card!=card_richest {
                                veccard_non_richest.push(card);
                            }
                        }
                    }
                    vecstich_2.retain(|stich| {
                        let epi_winner = rules.winner_index(SFullStich::new(stich));
                        !(playerparties.is_primary_party(epi_winner)==playerparties.is_primary_party(epi_card)
                            && veccard_non_richest.contains(&unwrap!(stich.get(epi_card)))
                            && veccard_best_own_party.contains(&unwrap!(stich.get(epi_winner))))
                    });
                    vecstich.extend(vecstich_2);
                } else {
                    let mut ab_points = [false; 12]; // TODO? couple with points_card
                    for card in veccard_chain {
                        if assign_neq(&mut ab_points[points_card(card).as_num::<usize>()], true) {
                            vecstich.extend(stichseq.zugeben_and_restore_with_hands(ahand, epi_card, card, rules, |ahand, stichseq|
                                Self::outer_make(
                                    (ahand, stichseq),
                                    (rules, playerparties),
                                )
                            ));
                        }
                    }
                };
            }
            vecstich
        }
    }

    pub fn new_with(
        (ahand, stichseq): (&mut EnumMap<EPlayerIndex, SHand>, &mut SStichSequence),
        rules: &dyn TRules,
        cardspartition_completed_cards: &SCardsPartition,
        playerparties: &SPlayerPartiesTable,
    ) -> Self {
        // return Self::new_from_full_stichs(
        //     Self::outer_make(
        //         (ahand, stichseq),
        //         (rules, playerparties),
        //     )
        // );
        fn for_each_allowed_card(
            n_depth: usize, // TODO? static enum type, possibly difference of EPlayerIndex
            (ahand, stichseq): (&mut EnumMap<EPlayerIndex, SHand>, &mut SStichSequence),
            rules: &dyn TRules,
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

#[test]
fn test_stichtrie_merge() {
    let make_stichtrie = |acard: [ECard; EPlayerIndex::SIZE]| {
        SStichTrie::new_from_full_stich(SFullStich::new(&SStich::new_full(EPlayerIndex::EPI0, acard)))
    };
    use crate::primitives::card::ECard::*;
    use std::collections::HashSet;
    let acard_0 = [EO, GO, HO, SO];
    let mut stichtrie = make_stichtrie(acard_0.explicit_clone());
    let mut vecacard = vec!(acard_0);
    for acard in [
        [EO, GO, HO, EU],
        [EO, GO, GU, SO],
        [EO, HU, HO, SO],
        [SU, GO, HO, SO],
        [SU, HU, GU, EU],
        [EO, HU, GU, EU],
    ] {
        stichtrie.merge(make_stichtrie(acard.explicit_clone()));
        vecacard.push(acard);
        assert_eq!(
            stichtrie.traverse_trie(EPlayerIndex::EPI0)
                .into_iter()
                .collect::<HashSet<_>>(),
            vecacard.iter().cloned()
                .map(|acard| SStich::new_full(EPlayerIndex::EPI0, acard))
                .collect::<HashSet<_>>(),
        );
    }
}

#[test]
fn test_stichtrie_make_simple() {
    #![allow(clippy::redundant_clone)]
    use crate::{
        primitives::card::ECard::*,
        rules::{
            payoutdecider::{SPayoutDeciderParams, SLaufendeParams},
            rulesrufspiel::SRulesRufspiel,
        },
        util::*,
    };
    use std::collections::HashSet;
    let rules_rufspiel_eichel_epi1 = SRulesRufspiel::new(
        EPlayerIndex::EPI1,
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
    let ahand = EPlayerIndex::map_from_raw([
        [EO, EU, HA],
        [GO, GU, HZ],
        [HO, HU, HK],
        [SO, SU, H9],
    ]).map_into(SHand::new_from_iter);
    let stichseq = SStichSequence::new_from_cards(
        EKurzLang::Lang,
        [
            EA, EZ, EK, E9,
            E8, E7, G8, G7,
            GA, GZ, GK, G9,
            SZ, SA, SK, S9,
            /*stich initiated by EPI1*/S7, S8, H7, H8,
        ].into_iter(),
        &rules_rufspiel_eichel_epi1,
    );
    assert_eq!(
        SStichTrie::make_simple(
            (&mut ahand.clone(), &mut stichseq.clone()),
            &rules_rufspiel_eichel_epi1,
            /*fn_filter*/&|_epi, veccard| veccard,
        ).into_iter().collect::<HashSet<_>>(),
        [
            [EO, GO, HO, SO], [EO, GO, HO, SU], [EO, GO, HO, H9],
            [EO, GO, HU, SO], [EO, GO, HU, SU], [EO, GO, HU, H9],
            [EO, GO, HK, SO], [EO, GO, HK, SU], [EO, GO, HK, H9],
            [EO, GU, HO, SO], [EO, GU, HO, SU], [EO, GU, HO, H9],
            [EO, GU, HU, SO], [EO, GU, HU, SU], [EO, GU, HU, H9],
            [EO, GU, HK, SO], [EO, GU, HK, SU], [EO, GU, HK, H9],
            [EO, HZ, HO, SO], [EO, HZ, HO, SU], [EO, HZ, HO, H9],
            [EO, HZ, HU, SO], [EO, HZ, HU, SU], [EO, HZ, HU, H9],
            [EO, HZ, HK, SO], [EO, HZ, HK, SU], [EO, HZ, HK, H9],
            [EU, GO, HO, SO], [EU, GO, HO, SU], [EU, GO, HO, H9],
            [EU, GO, HU, SO], [EU, GO, HU, SU], [EU, GO, HU, H9],
            [EU, GO, HK, SO], [EU, GO, HK, SU], [EU, GO, HK, H9],
            [EU, GU, HO, SO], [EU, GU, HO, SU], [EU, GU, HO, H9],
            [EU, GU, HU, SO], [EU, GU, HU, SU], [EU, GU, HU, H9],
            [EU, GU, HK, SO], [EU, GU, HK, SU], [EU, GU, HK, H9],
            [EU, HZ, HO, SO], [EU, HZ, HO, SU], [EU, HZ, HO, H9],
            [EU, HZ, HU, SO], [EU, HZ, HU, SU], [EU, HZ, HU, H9],
            [EU, HZ, HK, SO], [EU, HZ, HK, SU], [EU, HZ, HK, H9],
            [HA, GO, HO, SO], [HA, GO, HO, SU], [HA, GO, HO, H9],
            [HA, GO, HU, SO], [HA, GO, HU, SU], [HA, GO, HU, H9],
            [HA, GO, HK, SO], [HA, GO, HK, SU], [HA, GO, HK, H9],
            [HA, GU, HO, SO], [HA, GU, HO, SU], [HA, GU, HO, H9],
            [HA, GU, HU, SO], [HA, GU, HU, SU], [HA, GU, HU, H9],
            [HA, GU, HK, SO], [HA, GU, HK, SU], [HA, GU, HK, H9],
            [HA, HZ, HO, SO], [HA, HZ, HO, SU], [HA, HZ, HO, H9],
            [HA, HZ, HU, SO], [HA, HZ, HU, SU], [HA, HZ, HU, H9],
            [HA, HZ, HK, SO], [HA, HZ, HK, SU], [HA, HZ, HK, H9],
        ].into_iter()
            .map(|acard| SStich::new_full(EPlayerIndex::EPI0, acard))
            .collect::<HashSet<_>>(),
    );
    {
        let mut ahand = ahand.clone();
        let mut stichseq = stichseq.clone();
        ahand[EPlayerIndex::EPI0].play_card(EO);
        stichseq.zugeben(EO, &rules_rufspiel_eichel_epi1);
        assert_eq!(
            SStichTrie::make_simple(
                (&mut ahand, &mut stichseq),
                &rules_rufspiel_eichel_epi1,
                /*fn_filter*/&|_epi, veccard| veccard,
            ).into_iter().collect::<HashSet<_>>(),
            [
                [EO, GO, HO, SO], [EO, GO, HO, SU], [EO, GO, HO, H9],
                [EO, GO, HU, SO], [EO, GO, HU, SU], [EO, GO, HU, H9],
                [EO, GO, HK, SO], [EO, GO, HK, SU], [EO, GO, HK, H9],
                [EO, GU, HO, SO], [EO, GU, HO, SU], [EO, GU, HO, H9],
                [EO, GU, HU, SO], [EO, GU, HU, SU], [EO, GU, HU, H9],
                [EO, GU, HK, SO], [EO, GU, HK, SU], [EO, GU, HK, H9],
                [EO, HZ, HO, SO], [EO, HZ, HO, SU], [EO, HZ, HO, H9],
                [EO, HZ, HU, SO], [EO, HZ, HU, SU], [EO, HZ, HU, H9],
                [EO, HZ, HK, SO], [EO, HZ, HK, SU], [EO, HZ, HK, H9],
            ].into_iter()
                .map(|acard| SStich::new_full(EPlayerIndex::EPI0, acard))
                .collect::<HashSet<_>>(),
        );
        ahand[EPlayerIndex::EPI1].play_card(GO);
        stichseq.zugeben(GO, &rules_rufspiel_eichel_epi1);
        assert_eq!(
            SStichTrie::make_simple(
                (&mut ahand, &mut stichseq),
                &rules_rufspiel_eichel_epi1,
                /*fn_filter*/&|_epi, veccard| veccard,
            ).into_iter().collect::<HashSet<_>>(),
            [
                [EO, GO, HO, SO], [EO, GO, HO, SU], [EO, GO, HO, H9],
                [EO, GO, HU, SO], [EO, GO, HU, SU], [EO, GO, HU, H9],
                [EO, GO, HK, SO], [EO, GO, HK, SU], [EO, GO, HK, H9],
            ].into_iter()
                .map(|acard| SStich::new_full(EPlayerIndex::EPI0, acard))
                .collect::<HashSet<_>>(),
        );
        ahand[EPlayerIndex::EPI2].play_card(HO);
        stichseq.zugeben(HO, &rules_rufspiel_eichel_epi1);
        assert_eq!(
            SStichTrie::make_simple(
                (&mut ahand, &mut stichseq),
                &rules_rufspiel_eichel_epi1,
                /*fn_filter*/&|_epi, veccard| veccard,
            ).into_iter().collect::<HashSet<_>>(),
            [
                [EO, GO, HO, SO], [EO, GO, HO, SU], [EO, GO, HO, H9],
            ].into_iter()
                .map(|acard| SStich::new_full(EPlayerIndex::EPI0, acard))
                .collect::<HashSet<_>>(),
        );
    }
    assert_eq!(
        SStichTrie::make_simple(
            (&mut ahand.clone(), &mut stichseq.clone()),
            &rules_rufspiel_eichel_epi1,
            /*fn_filter*/&|(_ahand, stichseq), mut veccard| {
                veccard.retain(|card| {
                    let eschlag = card.schlag();
                    match unwrap!(stichseq.current_stich().current_playerindex()) {
                        EPlayerIndex::EPI0 => eschlag==ESchlag::Ober,
                        EPlayerIndex::EPI1 => eschlag==ESchlag::Unter,
                        EPlayerIndex::EPI2 => eschlag!=ESchlag::Unter && eschlag!=ESchlag::Ober,
                        EPlayerIndex::EPI3 => true,
                    }
                });
                veccard
            },
        ).into_iter().collect::<HashSet<_>>(),
        [
            [EO, GU, HK, SO], [EO, GU, HK, SU], [EO, GU, HK, H9],
        ].into_iter()
            .map(|acard| SStich::new_full(EPlayerIndex::EPI0, acard))
            .collect::<HashSet<_>>(),
    );
}

#[derive(Debug)]
pub struct SFilterByOracle<'rules> {
    rules: &'rules dyn TRules,
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
        game::SGame,
        player::{
            TPlayer,
            playerrandom::SPlayerRandom,
        },
        primitives::{*, card::ECard::*},
        rules::{
            payoutdecider::{SPayoutDeciderParams, SPayoutDeciderPointBased, SLaufendeParams},
            rulesrufspiel::SRulesRufspiel,
            rulessolo::{sololike, ESoloLike},
            ruleset::{
                SRuleSet,
                allowed_rules,
                VStockOrT,
            },
            SStossParams,
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
        let stossparams = SStossParams::new(
            /*n_stoss_max*/4,
        );
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
            stossparams.clone(),
        );
        fn assert_stichoracle(
            rules: &dyn TRules,
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
            let stichtrie = SStichTrie::new_from_full_stichs(
                SStichTrie::outer_make(
                    (&mut ahand.clone(), &mut stichseq.clone()),
                    (rules, &playerparties)
                ),
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
                    display_card_slices(ahand, &rules, "\n "),
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
                // [GU, H8, HU, EO], // covered by [GU, H8, EU, EO]
                [GU, H8, HU, H9],
                [GU, H8, HU, HZ],
                [GU, HK, EU, EO],
                [GU, HK, EU, HZ],
                [GU, HK, HA, EO],
                [GU, HK, HA, H9],
                // [GU, HK, HU, EO], // covered by [GU, HK, EU, EO]
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
                // [SU, GO, HA, H9], // covered by [SU, GO, HU, H9]
                // [SU, GO, HA, HZ], // covered by [SU, GO, HU, HZ]
                // [SU, GO, HU, EO], // covered by [SU, GO, HA, EO]
                [SU, GO, HU, H9],
                [SU, GO, HU, HZ],
                [SU, H8, EU, EO],
                [SU, H8, EU, H9],
                [SU, H8, EU, HZ],
                [SU, H8, HA, EO],
                [SU, H8, HA, H9],
                [SU, H8, HA, HZ],
                // [SU, H8, HU, EO], // covered by [SU, H8, HA, EO]
                [SU, H8, HU, H9],
                [SU, H8, HU, HZ],
                [SU, HK, EU, EO],
                [SU, HK, EU, HZ],
                [SU, HK, HA, EO],
                [SU, HK, HA, H9],
                // [SU, HK, HU, EO], // covered by [SU, HK, HA, EO]
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
                // [S9, SK, EU, SZ], // covered by [S9, S8, EU, SZ]
                // [S9, SK, HU, SZ], // covered by [S9, S8, HU, SZ]
                // [S9, SK, HA, SZ], // covered by [S9, S8, HA, SZ]
                // [S9, SK, EZ, SZ], // covered by [S9, S8, EZ, SZ]
                // [S9, SK, E7, SZ], // covered by [S9, S8, E7, SZ]
                // [S9, SK, GZ, SZ], // covered by [S9, S8, GZ, SZ]
                // [S9, SK, G9, SZ], // covered by [S9, S8, G9, SZ]
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
            stossparams.clone(),
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
                // [H8, SU, HU, EU], // covered by [H8, SU, HA, EU]
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
                // [EO, HU, GU, EK], // covered by [EO, EU, GU, ..]
                // [EO, HU, GU, E8], // covered by [EO, EU, GU, ..]
                // [EO, HU, GU, E7], // covered by [EO, HU, GU, E8]
                // [EO, HU, GU, HZ], // covered by [EO, EU, GU, ..]
                // [EO, HU, GU, H9], // covered by [EO, EU, GU, ..]
                // [EO, HU, GU, H7], // covered by [EO, EU, GU, ..]
                // [EO, HU, GU, SZ], // covered by [EO, EU, GU, ..]
                // [EO, HU, GU, S8], // covered by [EO, EU, GU, ..]
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
                // [EO, GA, SU, EK], // covered by [EO, HU, .., ..]
                // [EO, GA, SU, E8], // covered by [EO, HU, .., ..]
                // [EO, GA, SU, E7], // covered by [EO, GA, SU, E8],
                // [EO, GA, SU, HZ], // covered by [EO, HU, .., ..]
                // [EO, GA, SU, H9], // covered by [EO, HU, .., ..]
                // [EO, GA, SU, H7], // covered by [EO, HU, .., ..]
                // [EO, GA, SU, SZ], // covered by [EO, HU, .., ..]
                // [EO, GA, SU, S8], // covered by [EO, HU, .., ..]
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
                // [GZ, HU, GU, EK], // covered by [GZ, GA, SU, ..]
                // [GZ, HU, GU, E8], // covered by [GZ, GA, SU, ..]
                // [GZ, HU, GU, E7], // covered by [GZ, HU, GU, E8],
                // [GZ, HU, GU, HZ], // covered by [GZ, GA, SU, ..]
                // [GZ, HU, GU, H9], // covered by [GZ, GA, SU, ..]
                // [GZ, HU, GU, H7], // covered by [GZ, GA, SU, ..]
                // [GZ, HU, GU, SZ], // covered by [GZ, GA, SU, ..]
                // [GZ, HU, GU, S8], // covered by [GZ, GA, SU, ..]
                // [GZ, HU, SU, EK], // covered by [GZ, SO, GU, ..]
                // [GZ, HU, SU, E8], // covered by [GZ, SO, GU, ..]
                // [GZ, HU, SU, E7], // covered by [GZ, HU, SU, E8],
                // [GZ, HU, SU, HZ], // covered by [GZ, SO, GU, ..]
                // [GZ, HU, SU, H9], // covered by [GZ, SO, GU, ..]
                // [GZ, HU, SU, H7], // covered by [GZ, SO, GU, ..]
                // [GZ, HU, SU, SZ], // covered by [GZ, SO, GU, ..]
                // [GZ, HU, SU, S8], // covered by [GZ, SO, GU, ..]
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
                // [GZ, GK, GU, EK], // covered by [GZ, GA, .., ..]
                // [GZ, GK, GU, E8], // covered by [GZ, GA, .., ..]
                // [GZ, GK, GU, E7], // covered by [GZ, GK, GU, E8],
                // [GZ, GK, GU, HZ], // covered by [GZ, GA, .., ..]
                // [GZ, GK, GU, H9], // covered by [GZ, GA, .., ..]
                // [GZ, GK, GU, H7], // covered by [GZ, GA, .., ..]
                // [GZ, GK, GU, SZ], // covered by [GZ, GA, .., ..]
                // [GZ, GK, GU, S8], // covered by [GZ, GA, .., ..]
                // [GZ, GK, SU, EK], // covered by [GZ, GA, .., ..]
                // [GZ, GK, SU, E8], // covered by [GZ, GA, .., ..]
                // [GZ, GK, SU, E7], // covered by [GZ, GK, SU, E8],
                // [GZ, GK, SU, HZ], // covered by [GZ, GA, .., ..]
                // [GZ, GK, SU, H9], // covered by [GZ, GA, .., ..]
                // [GZ, GK, SU, H7], // covered by [GZ, GA, .., ..]
                // [GZ, GK, SU, SZ], // covered by [GZ, GA, .., ..]
                // [GZ, GK, SU, S8], // covered by [GZ, GA, .., ..]
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
                // [G7, HU, GU, EK], // covered by [G7, GA, SU, ..]
                // [G7, HU, GU, E8], // covered by [G7, GA, SU, ..]
                // [G7, HU, GU, E7], // covered by [G7, HU, GU, E8],
                // [G7, HU, GU, HZ], // covered by [G7, GA, SU, ..]
                // [G7, HU, GU, H9], // covered by [G7, GA, SU, ..]
                // [G7, HU, GU, H7], // covered by [G7, GA, SU, ..]
                // [G7, HU, GU, SZ], // covered by [G7, GA, SU, ..]
                // [G7, HU, GU, S8], // covered by [G7, GA, SU, ..]
                // [G7, HU, SU, EK], // covered by [G7, SO, GU, ..]
                // [G7, HU, SU, E8], // covered by [G7, SO, GU, ..]
                // [G7, HU, SU, E7], // covered by [G7, HU, SU, E8],
                // [G7, HU, SU, HZ], // covered by [G7, SO, GU, ..]
                // [G7, HU, SU, H9], // covered by [G7, SO, GU, ..]
                // [G7, HU, SU, H7], // covered by [G7, SO, GU, ..]
                // [G7, HU, SU, SZ], // covered by [G7, SO, GU, ..]
                // [G7, HU, SU, S8], // covered by [G7, SO, GU, ..]
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
        );
        assert_stichoracle(
            TRulesBoxClone::box_clone(rules_farbwenz_eichel_epi3.as_ref()).as_ref(),
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
                // [GA, GO, EU, SU], // covered by [GA, GK, .., ..]
                // [GA, GO, EU, EZ], // covered by [.., .., .., EO]
                // [GA, GO, EU, EK], // covered by [.., .., .., EO]
                // [GA, GO, EU, EO], // covered by [GA, GK, .., ..]
                // [GA, GO, EU, E8], // covered by [GA, GK, .., ..]
                // [GA, GO, EU, E7], // covered by [.., .., .., E8]
                // [GA, GO, EU, SA], // covered by [GA, GK, .., ..]
                // [GA, GO, EU, S7], // covered by [GA, GK, .., ..]
                // [GA, GO, HU, SU], // covered by [GA, GK, .., ..]
                // [GA, GO, HU, EZ], // covered by [.., .., .., EO]
                // [GA, GO, HU, EK], // covered by [.., .., .., EO]
                // [GA, GO, HU, EO], // covered by [GA, GK, .., ..]
                // [GA, GO, HU, E8], // covered by [GA, GK, .., ..]
                // [GA, GO, HU, E7], // covered by [.., .., .., E8]
                // [GA, GO, HU, SA], // covered by [GA, GK, .., ..]
                // [GA, GO, HU, S7], // covered by [GA, GK, .., ..]
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
                // [GA, G9, EU, SU], // covered by [GA, GK, EU, SU]
                // [GA, G9, EU, EZ], // covered by [.., .., .., EO]
                // [GA, G9, EU, EK], // covered by [.., .., .., EO]
                // [GA, G9, EU, EO], // covered by [GA, GK, EU, EO]
                // [GA, G9, EU, E8], // covered by [GA, GK, EU, E8]
                // [GA, G9, EU, E7], // covered by [.., .., .., E8]
                // [GA, G9, EU, SA], // covered by [GA, GK, EU, E7]
                // [GA, G9, EU, S7], // covered by [GA, GK, EU, S7]
                // [GA, G9, HU, SU], // covered by [GA, GK, HU, SU]
                // [GA, G9, HU, EZ], // covered by [.., .., .., EO]
                // [GA, G9, HU, EK], // covered by [.., .., .., EO]
                // [GA, G9, HU, EO], // covered by [GA, GK, HU, EO]
                // [GA, G9, HU, E8], // covered by [GA, GK, HU, E7]
                // [GA, G9, HU, E7], // covered by [.., .., .., E8]
                // [GA, G9, HU, SA], // covered by [GA, GK, HU, S7]
                // [GA, G9, HU, S7], // covered by [GA, GK, HU, S7]
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
                // [GZ, GK, EU, SU], // covered by [GA, .., .U, ..]
                // [GZ, GK, EU, EZ], // covered by [.., .., .., EO]
                // [GZ, GK, EU, EK], // covered by [.., .., .., EO]
                // [GZ, GK, EU, EO], // covered by [GA, .., .U, ..]
                // [GZ, GK, EU, E8], // covered by [GA, .., .U, ..]
                // [GZ, GK, EU, E7], // covered by [.., .., .., E8]
                // [GZ, GK, EU, SA], // covered by [GA, .., .U, ..]
                // [GZ, GK, EU, S7], // covered by [GA, .., .U, ..]
                // [GZ, GK, HU, SU], // covered by [GA, .., .U, ..]
                // [GZ, GK, HU, EZ], // covered by [.., .., .., EO]
                // [GZ, GK, HU, EK], // covered by [.., .., .., EO]
                // [GZ, GK, HU, EO], // covered by [GA, .., .U, ..]
                // [GZ, GK, HU, E8], // covered by [GA, .., .U, ..]
                // [GZ, GK, HU, E7], // covered by [.., .., .., E8]
                // [GZ, GK, HU, SA], // covered by [GA, .., .U, ..]
                // [GZ, GK, HU, S7], // covered by [GA, .., .U, ..]
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
                // [GZ, GO, EU, SU], // covered by [GA, .., .U, ..]
                // [GZ, GO, EU, EZ], // covered by [.., .., .., EO]
                // [GZ, GO, EU, EK], // covered by [.., .., .., EO]
                // [GZ, GO, EU, EO], // covered by [GA, .., .U, ..]
                // [GZ, GO, EU, E8], // covered by [GA, .., .U, ..]
                // [GZ, GO, EU, E7], // covered by [.., .., .., E8]
                // [GZ, GO, EU, SA], // covered by [GA, .., .U, ..]
                // [GZ, GO, EU, S7], // covered by [GA, .., .U, ..]
                // [GZ, GO, HU, SU], // covered by [GA, .., .U, ..]
                // [GZ, GO, HU, EZ], // covered by [.., .., .., EO]
                // [GZ, GO, HU, EK], // covered by [.., .., .., EO]
                // [GZ, GO, HU, EO], // covered by [GA, .., .U, ..]
                // [GZ, GO, HU, E8], // covered by [GA, .., .U, ..]
                // [GZ, GO, HU, E7], // covered by [.., .., .., E8]
                // [GZ, GO, HU, SA], // covered by [GA, .., .U, ..]
                // [GZ, GO, HU, S7], // covered by [GA, .., .U, ..]
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
                // [GZ, G9, EU, SU], // covered by [GA, .., .U, ..]
                // [GZ, G9, EU, EZ], // covered by [.., .., .., EO]
                // [GZ, G9, EU, EK], // covered by [.., .., .., EO]
                // [GZ, G9, EU, EO], // covered by [GA, .., .U, ..]
                // [GZ, G9, EU, E8], // covered by [GA, .., .U, ..]
                // [GZ, G9, EU, E7], // covered by [.., .., .., E8]
                // [GZ, G9, EU, SA], // covered by [GA, .., .U, ..]
                // [GZ, G9, EU, S7], // covered by [GA, .., .U, ..]
                // [GZ, G9, HU, SU], // covered by [GA, .., .U, ..]
                // [GZ, G9, HU, EZ], // covered by [.., .., .., EO]
                // [GZ, G9, HU, EK], // covered by [.., .., .., EO]
                // [GZ, G9, HU, EO], // covered by [GA, .., .U, ..]
                // [GZ, G9, HU, E8], // covered by [GA, .., .U, ..]
                // [GZ, G9, HU, E7], // covered by [.., .., .., E8]
                // [GZ, G9, HU, SA], // covered by [GA, .., .U, ..]
                // [GZ, G9, HU, S7], // covered by [GA, .., .U, ..]
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
                // [GZ, G7, EU, SU], // covered by [GA, .., .U, ..]
                // [GZ, G7, EU, EZ], // covered by [.., .., .., EO]
                // [GZ, G7, EU, EK], // covered by [.., .., .., EO]
                // [GZ, G7, EU, EO], // covered by [GA, .., .U, ..]
                // [GZ, G7, EU, E8], // covered by [GA, .., .U, ..]
                // [GZ, G7, EU, E7], // covered by [.., .., .., E8]
                // [GZ, G7, EU, SA], // covered by [GA, .., .U, ..]
                // [GZ, G7, EU, S7], // covered by [GA, .., .U, ..]
                // [GZ, G7, HU, SU], // covered by [GA, .., .U, ..]
                // [GZ, G7, HU, EZ], // covered by [.., .., .., EO]
                // [GZ, G7, HU, EK], // covered by [.., .., .., EO]
                // [GZ, G7, HU, EO], // covered by [GA, .., .U, ..]
                // [GZ, G7, HU, E8], // covered by [GA, .., .U, ..]
                // [GZ, G7, HU, E7], // covered by [.., .., .., E8]
                // [GZ, G7, HU, SA], // covered by [GA, .., .U, ..]
                // [GZ, G7, HU, S7], // covered by [GA, .., .U, ..]
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
                // [G8, GO, EU, SU], // covered by [G8, GK, .U, ..]
                // [G8, GO, EU, EZ], // covered by [.., .., .., EO]
                // [G8, GO, EU, EK], // covered by [.., .., .., EO]
                // [G8, GO, EU, EO], // covered by [G8, GK, .U, ..]
                // [G8, GO, EU, E8], // covered by [G8, GK, .U, ..]
                // [G8, GO, EU, E7], // covered by [.., .., .., E8]
                // [G8, GO, EU, SA], // covered by [G8, GK, .U, ..]
                // [G8, GO, EU, S7], // covered by [G8, GK, .U, ..]
                // [G8, GO, HU, SU], // covered by [G8, GK, .U, ..]
                // [G8, GO, HU, EZ], // covered by [.., .., .., EO]
                // [G8, GO, HU, EK], // covered by [.., .., .., EO]
                // [G8, GO, HU, EO], // covered by [G8, GK, .U, ..]
                // [G8, GO, HU, E8], // covered by [G8, GK, .U, ..]
                // [G8, GO, HU, E7], // covered by [.., .., .., E8]
                // [G8, GO, HU, SA], // covered by [G8, GK, .U, ..]
                // [G8, GO, HU, S7], // covered by [G8, GK, .U, ..]
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
                // [G8, G9, EU, SU], // covered by [G8, GK, .U, ..]
                // [G8, G9, EU, EZ], // covered by [.., .., .., EO]
                // [G8, G9, EU, EK], // covered by [.., .., .., EO]
                // [G8, G9, EU, EO], // covered by [G8, GK, .U, ..]
                // [G8, G9, EU, E8], // covered by [G8, GK, .U, ..]
                // [G8, G9, EU, E7], // covered by [.., .., .., E8]
                // [G8, G9, EU, SA], // covered by [G8, GK, .U, ..]
                // [G8, G9, EU, S7], // covered by [G8, GK, .U, ..]
                // [G8, G9, HU, SU], // covered by [G8, GK, .U, ..]
                // [G8, G9, HU, EZ], // covered by [.., .., .., EO]
                // [G8, G9, HU, EK], // covered by [.., .., .., EO]
                // [G8, G9, HU, EO], // covered by [G8, GK, .U, ..]
                // [G8, G9, HU, E8], // covered by [G8, GK, .U, ..]
                // [G8, G9, HU, E7], // covered by [.., .., .., E8]
                // [G8, G9, HU, SA], // covered by [G8, GK, .U, ..]
                // [G8, G9, HU, S7], // covered by [G8, GK, .U, ..]
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
            TRulesBoxClone::box_clone(rules_farbwenz_eichel_epi3.as_ref()).as_ref(),
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
                // [EA, EO, GA, GU], // covered by [EA, SU, GA, GU]
                // [EA, EO, GZ, GU], // covered by [.., .., GA, GU]
                // [EA, EO, G8, GU], // covered by [EA, SU, G8, GU]
                // [EA, EO, HA, GU], // covered by [EA, SU, HA, GU]
                // [EA, EO, HK, GU], // covered by [EA, SU, HK, GU]
                // [EA, EO, H8, GU], // covered by [.., .., HK, ..]
                // [EA, EO, SO, GU], // covered by [EA, SU, SO, GU]
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
                // [E9, EO, GA, GU], // covered by [E9, E8, GA, GU]
                // [E9, EO, GZ, GU], // covered by [.., .., GA, GU]
                // [E9, EO, G8, GU], // covered by [E9, E8, G8, GU]
                // [E9, EO, HA, GU], // covered by [E9, E8, HA, GU]
                // [E9, EO, HK, GU], // covered by [E9, E8, HK, GU]
                // [E9, EO, H8, GU], // covered by [E9, EO, HK, GU]
                // [E9, EO, SO, GU], // covered by [E9, E8, SO, GU]
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
                // [HZ, SU, HA, G9], // covered by [HZ, SU, H8, G9]
                // [HZ, SU, HA, G7], // covered by [HZ, SU, H8, G7]
                // [HZ, SU, HA, SK], // covered by [HZ, SU, H8, SK]
                // [HZ, SU, HA, S8], // covered by [HZ, SU, H8, S8]
                // [HZ, SU, HK, GU], // covered by [HZ, SU, HA, GU]
                // [HZ, SU, HK, GK], // covered by [HZ, SU, H., GO]
                // [HZ, SU, HK, GO], // covered by [.., .., .., G9]
                // [HZ, SU, HK, G9], // covered by [HZ, SU, H8, G9]
                // [HZ, SU, HK, G7], // covered by [HZ, SU, H8, G7]
                // [HZ, SU, HK, SK], // covered by [HZ, SU, H8, SK]
                // [HZ, SU, HK, S8], // covered by [HZ, SU, H8, S8]
                // [HZ, SU, H8, GU], // covered by [HZ, SU, HA, GU]
                // [HZ, SU, H8, GK], // covered by [HZ, SU, H., GO]
                // [HZ, SU, H8, GO], // covered by [.., .., .., G9]
                [HZ, SU, H8, G9],
                [HZ, SU, H8, G7],
                [HZ, SU, H8, SK],
                [HZ, SU, H8, S8],
                [HZ, EZ, HA, GU],
                // [HZ, EZ, HA, GK], // covered by [HZ, .., .., GO]
                // [HZ, EZ, HA, GO], // covered by [.., .., .., G9]
                // [HZ, EZ, HA, G9], // covered by [HZ, EZ, H8, G9]
                // [HZ, EZ, HA, G7], // covered by [HZ, EZ, H8, G7]
                // [HZ, EZ, HA, SK], // covered by [HZ, EZ, H8, SK]
                // [HZ, EZ, HA, S8], // covered by [HZ, EZ, G8, S8]
                // [HZ, EZ, HK, GU], // covered by [EZ, EZ, HA, GU] (HA better than HK)
                // [HZ, EZ, HK, GK], // covered by [HZ, ... .., GO]
                // [HZ, EZ, HK, GO], // covered by [.., .., .., G9]
                // [HZ, EZ, HK, G9], // covered by [HZ, EZ, H8, G9]
                // [HZ, EZ, HK, G7], // covered by [HZ, EZ, H8, G7]
                // [HZ, EZ, HK, SK], // covered by [HZ, EZ, H8, SK]
                // [HZ, EZ, HK, S8], // covered by [HZ, EZ, H8, S8]
                // [HZ, EZ, H8, GU], // covered by [HZ, EZ, HA, GU]
                // [HZ, EZ, H8, GK], // covered by [HZ, ... .., GO]
                // [HZ, EZ, H8, GO], // covered by [.., .., .., G9]
                [HZ, EZ, H8, G9],
                [HZ, EZ, H8, G7],
                [HZ, EZ, H8, SK],
                [HZ, EZ, H8, S8],
                [HZ, EK, HA, GU],
                // [HZ, EK, HA, GK], // covered by [.., .., .., GO]
                // [HZ, EK, HA, GO], // covered by [.., .., .., G9]
                // [HZ, EK, HA, G9], // covered by [HZ, EK, H8, G9]
                // [HZ, EK, HA, G7], // covered by [HZ, EK, H8, G7]
                // [HZ, EK, HA, SK], // covered by [HZ, EK, H8, SK]
                // [HZ, EK, HA, S8], // covered by [HZ, EK, H8, S8]
                // [HZ, EK, HK, GU], // covered by [HZ, EK, HA, GU]
                // [HZ, EK, HK, GK], // covered by [.., .., .., GO]
                // [HZ, EK, HK, GO], // covered by [.., .., .., G9]
                // [HZ, EK, HK, G9], // covered by [HZ, EK, H8, G9]
                // [HZ, EK, HK, G7], // covered by [HZ, EK, H8, G7]
                // [HZ, EK, HK, SK], // covered by [HZ, EK, H8, SK]
                // [HZ, EK, HK, S8], // covered by [HZ, EK, H8, S8]
                // [HZ, EK, H8, GU], // covered by [HZ, EK, HA, GU]
                // [HZ, EK, H8, GK], // covered by [.., .., .., GO]
                // [HZ, EK, H8, GO], // covered by [.., .., .., G9]
                [HZ, EK, H8, G9],
                [HZ, EK, H8, G7],
                [HZ, EK, H8, SK],
                [HZ, EK, H8, S8],
                [HZ, EO, HA, GU],
                // [HZ, EO, HA, GK], // covered by [.., .., .., GO]
                // [HZ, EO, HA, GO], // covered by [.., .., .., G9]
                // [HZ, EO, HA, G9], // covered by [HZ, EO, H8, G9]
                // [HZ, EO, HA, G7], // covered by [HZ, EO, H8, G7]
                // [HZ, EO, HA, SK], // covered by [HZ, EO, H8, SK]
                // [HZ, EO, HA, S8], // covered by [HZ, EO, H8, S8]
                // [HZ, EO, HK, GU], // covered by [HZ, EO, HA, GU]
                // [HZ, EO, HK, GK], // covered by [.., .., .., GO]
                // [HZ, EO, HK, GO], // covered by [.., .., .., G9]
                // [HZ, EO, HK, G9], // covered by [HZ, EO, H8, G9]
                // [HZ, EO, HK, G7], // covered by [HZ, EO, H8, G7]
                // [HZ, EO, HK, SK], // covered by [HZ, EO, H8, SK]
                // [HZ, EO, HK, S8], // covered by [HZ, EO, H8, S8]
                // [HZ, EO, H8, GU], // covered by [HZ, EO, HA, GU]
                // [HZ, EO, H8, GK], // covered by [.., .., .., GO]
                // [HZ, EO, H8, GO], // covered by [.., .., .., G9]
                [HZ, EO, H8, G9],
                [HZ, EO, H8, G7],
                [HZ, EO, H8, SK],
                [HZ, EO, H8, S8],
                [HZ, E8, HA, GU],
                // [HZ, E8, HA, GK], // covered by [.., .., .., GO]
                // [HZ, E8, HA, GO], // covered by [.., .., .., G9]
                // [HZ, E8, HA, G9], // covered by [HZ, E8, H8, G9]
                // [HZ, E8, HA, G7], // covered by [HZ, E8, H8, G7]
                // [HZ, E8, HA, SK], // covered by [HZ, E8, H8, SK]
                // [HZ, E8, HA, S8], // covered by [HZ, E8, H8, S8]
                // [HZ, E8, HK, GU], // covered by [HZ, E8, HA, GU]
                // [HZ, E8, HK, GK], // covered by [.., .., .., GO]
                // [HZ, E8, HK, GO], // covered by [.., .., .., G9]
                // [HZ, E8, HK, G9], // covered by [HZ, E8, H8, G9]
                // [HZ, E8, HK, G7], // covered by [HZ, E8, H8, G7]
                // [HZ, E8, HK, SK], // covered by [HZ, E8, H8, SK]
                // [HZ, E8, HK, S8], // covered by [HZ, E8, H8, S8]
                // [HZ, E8, H8, GU], // covered by [HZ, E8, HA, GU]
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
                // [HZ, SA, HK, GU], // covered by [HZ, SA, HA, GU]
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
            TRulesBoxClone::box_clone(rules_farbwenz_eichel_epi3.as_ref()).as_ref(),
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
                // [SA, SO, SK, S9], // covered by [SA, SO, S8, S9]
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
                /*fn_check_ask_for_card*/|game: &SGame| {
                    if game.kurzlang().cards_per_player() - if_dbg_else!({4}{5}) < game.completed_stichs().len() {
                        //let epi = unwrap!(game.current_playable_stich().current_playerindex());
                        macro_rules! fwd{($ty_fn_make_filter:tt, $fn_make_filter:expr,) => {
                            unwrap!(determine_best_card::<$ty_fn_make_filter,_,_,_,_,_,_>(
                                &game.stichseq,
                                game.rules.as_ref(),
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
