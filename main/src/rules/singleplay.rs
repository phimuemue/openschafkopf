macro_rules! impl_single_play {() => {
    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.internal_playerindex())
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool {
        assert!(
            vecstoss.iter()
                .enumerate()
                .all(|(i_stoss, stoss)| (i_stoss%2==0) == (stoss.epi!=self.internal_playerindex()))
        );
        EKurzLang::from_cards_per_player(hand.cards().len());
        (epi==self.internal_playerindex())==(vecstoss.len()%2==1)
    }

    fn payout_no_invariant(&self, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), _n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize> {
        self.payoutdecider.payout(
            self,
            rulestatecache,
            gamefinishedstiche,
            &SPlayerParties13::new(self.internal_playerindex()),
        ).map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
    }

    fn payouthints(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), _n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        self.payoutdecider.payouthints(
            self,
            stichseq,
            ahand,
            rulestatecache,
            &SPlayerParties13::new(self.internal_playerindex()),
        ).map(|intvlon_payout| intvlon_payout.map(|on_payout|
             on_payout.map(|n_payout| payout_including_stoss_doubling(n_payout, tpln_stoss_doubling)),
        ))
    }

}}
