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

    fn payoutinfos2(&self, gamefinishedstiche: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutInfo> {
        self.payoutdecider.payout(
            self,
            rulestatecache,
            gamefinishedstiche,
            &SPlayerParties13::new(self.internal_playerindex()),
        ).map(|n_payout| SPayoutInfo::new(*n_payout, EStockAction::Ignore))
    }

    fn payouthints(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutHint> {
        self.payoutdecider.payouthints(
            self,
            stichseq,
            ahand,
            rulestatecache,
            &SPlayerParties13::new(self.internal_playerindex()),
        ).map(|tplon_payout| SPayoutHint::new((
             tplon_payout.0.map(|n_payout| SPayoutInfo::new(n_payout, EStockAction::Ignore)),
             tplon_payout.1.map(|n_payout| SPayoutInfo::new(n_payout, EStockAction::Ignore)),
        )))
    }

}}
