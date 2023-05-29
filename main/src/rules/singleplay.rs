macro_rules! impl_single_play {() => {
    fn playerindex(&self) -> Option<EPlayerIndex> {
        Some(self.epi)
    }

    fn stoss_allowed(&self, stichseq: &SStichSequence, hand: &SHand, epi: EPlayerIndex, vecstoss: &[SStoss]) -> bool {
        assert!(
            vecstoss.iter()
                .enumerate()
                .all(|(i_stoss, stoss)| (i_stoss%2==0) == (stoss.epi!=self.epi))
        );
        assert_eq!(stichseq.remaining_cards_per_hand()[epi], hand.cards().len());
        (epi==self.epi)==(vecstoss.len()%2==1)
    }
}}
