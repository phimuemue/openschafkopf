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

    fn payoutinfos(&self, gamefinishedstiche: SGameFinishedStiche) -> EnumMap<EPlayerIndex, SPayoutInfo> {
        self.payoutdecider.payout(
            self,
            gamefinishedstiche,
            &SPlayerParties13::new(self.epi),
        )
            .map(|n_payout| SPayoutInfo::new(*n_payout, EStockAction::Ignore))
    }

    fn payouthints(&self, _slcstich: &[SStich], _ahand: &EnumMap<EPlayerIndex, SHand>) -> EnumMap<EPlayerIndex, SPayoutHint> {
        // TODO sensible payouthints
        EPlayerIndex::map_from_fn(|_epi| SPayoutHint::new((None, None)))
    }

}}
