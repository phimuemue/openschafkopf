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

    fn payoutinfos(&self, gamefinishedstiche: SGameFinishedStiche, tpln_stoss_doubling: (usize, usize)) -> EnumMap<EPlayerIndex, SPayoutInfo> {
        SStossDoublingPayoutDecider::payout(
            &self.payoutdecider.payout(
                self,
                gamefinishedstiche,
                /*fn_is_player_party*/ |epi| {
                    epi==self.epi
                },
                /*fn_player_multiplier*/ |epi| {
                    if self.epi==epi {
                        3
                    } else {
                        1
                    }
                },
            ),
            tpln_stoss_doubling,
        )
            .map(|n_payout| SPayoutInfo::new(*n_payout, EStockAction::Ignore))
    }
}}
