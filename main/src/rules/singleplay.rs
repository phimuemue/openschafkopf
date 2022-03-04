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
}}
