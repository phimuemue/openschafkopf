macro_rules! impl_single_play {() => {
    fn stoss_allowed(&self, stichseq: &SStichSequence, hand: &SHand, epi: EPlayerIndex, vecstoss: &[SStoss]) -> bool {
        self.stossparams.stoss_allowed(stichseq, vecstoss) && {
            assert!(
                vecstoss.iter()
                    .enumerate()
                    .all(|(i_stoss, stoss)| (i_stoss%2==0) == (stoss.epi!=self.epi))
            );
            assert_eq!(stichseq.remaining_cards_per_hand()[epi], hand.cards().len());
            (epi==self.epi)==(vecstoss.len()%2==1)
        }
    }

    fn alpha_beta_pruner_lohi_values(&self) -> Option<Box<dyn Fn(&SRuleStateCacheFixed)->EnumMap<EPlayerIndex, ELoHi> + Sync>> {
        let epi_self = self.epi;
        Some(Box::new(move |_rulestatecache| {
            let mut mapepilohi = EPlayerIndex::map_from_fn(|_| ELoHi::Lo);
            mapepilohi[epi_self] = ELoHi::Hi;
            mapepilohi
        }))
    }
}}
