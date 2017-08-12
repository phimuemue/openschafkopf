use primitives::*;
use rules::*;
use rules::trumpfdecider::*;
use rules::payoutdecider::{SStossDoublingPayoutDecider, internal_payout};
use util::*;

#[derive(Clone, new)]
pub struct SRulesBettel {
    epi : EPlayerIndex,
    m_i_prio : isize,
    n_payout_base : isize,
}

impl fmt::Display for SRulesBettel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bettel von {}", self.epi)
    }
}

impl TActivelyPlayableRules for SRulesBettel {
    box_clone_impl_by_clone!(TActivelyPlayableRules);
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloLikeSimple(self.m_i_prio)
    }
}

impl TRules for SRulesBettel {
    box_clone_impl_by_clone!(TRules);
    impl_rules_trumpf!(STrumpfDeciderNoTrumpf);

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

    fn payout(&self, gamefinishedstiche: &SGameFinishedStiche, n_stoss: usize, n_doubling: usize, _n_stock: isize) -> SAccountBalance {
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| self.epi!=self.winner_index(stich));
        SAccountBalance::new(
            SStossDoublingPayoutDecider::payout(
                internal_payout(
                    /*n_payout_single_player*/ self.n_payout_base,
                    /*fn_player_multiplier*/ |epi| {
                        if self.epi==epi {
                            3
                        } else {
                            1
                        }
                    },
                    /*ab_winner*/ &EPlayerIndex::map_from_fn(|epi| {
                        (self.epi==epi)==b_player_party_wins
                    })
                ),
                n_stoss,
                n_doubling,
            ),
            0
        )
    }

    // TODORULES Grasober like bettel, i.e. Stichzwang
    // fn all_allowed_cards_within_stich(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector;

    fn compare_in_stich_farbe(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        if card_fst.farbe() != card_snd.farbe() {
            Ordering::Greater
        } else {
            let get_schlag_value = |card: SCard| { match card.schlag() {
                ESchlag::S7 => 0,
                ESchlag::S8 => 1,
                ESchlag::S9 => 2,
                ESchlag::Zehn => 3,
                ESchlag::Unter => 4,
                ESchlag::Ober => 5,
                ESchlag::Koenig => 6,
                ESchlag::Ass => 7,
            } };
            get_schlag_value(card_fst).cmp(&get_schlag_value(card_snd))
        }
    }
}
