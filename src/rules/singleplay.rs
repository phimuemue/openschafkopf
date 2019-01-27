macro_rules! impl_single_play {() => {
    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.epi)
    }

    fn stoss_allowed(&self, epi: EPlayerIndex, vecstoss: &[SStoss], hand: &SHand) -> bool {
        assert!(
            vecstoss.iter()
                .enumerate()
                .all(|(i_stoss, stoss)| (i_stoss%2==0) == (stoss.epi!=self.epi))
        );
        EKurzLang::from_cards_per_player(hand.cards().len());
        (epi==self.epi)==(vecstoss.len()%2==1)
    }

    fn payoutinfos(&self, gamefinishedstiche: SStichSequenceGameFinished, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutInfo> {
        self.payoutdecider.payout(
            self,
            rulestatecache,
            gamefinishedstiche,
            &SPlayerParties13::new(self.epi),
        )
            .map(|n_payout| SPayoutInfo::new(*n_payout, EStockAction::Ignore))
    }

    fn payouthints(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SPayoutHint> {
        self.payoutdecider.payouthints(
            self,
            stichseq,
            ahand,
            rulestatecache,
            &SPlayerParties13::new(self.epi),
        )
            .map(|pairon_payout| SPayoutHint::new((
                 pairon_payout.0.map(|n_payout| SPayoutInfo::new(n_payout, EStockAction::Ignore)),
                 pairon_payout.1.map(|n_payout| SPayoutInfo::new(n_payout, EStockAction::Ignore)),
            )))
    }

}}
