use primitives::*;
use rules::*;
use rules::trumpfdecider::*;
use rules::payoutdecider::{SStossDoublingPayoutDecider, internal_payout};
use util::*;

#[derive(Clone)]
pub struct SRulesBettel {
    epi : EPlayerIndex,
    i_prio : isize,
    payoutdecider : SPayoutDeciderBettel,
}

impl SRulesBettel {
    pub fn new(epi: EPlayerIndex, i_prio: isize, n_payout_base: isize) -> SRulesBettel {
        SRulesBettel{
            epi,
            i_prio,
            payoutdecider: SPayoutDeciderBettel{n_payout_base},
        }
    }
}

impl fmt::Display for SRulesBettel {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bettel von {}", self.epi)
    }
}

impl TActivelyPlayableRules for SRulesBettel {
    box_clone_impl_by_clone!(TActivelyPlayableRules);
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloLikeSimple(self.i_prio)
    }
}

#[derive(Clone)]
struct SPayoutDeciderBettel { // TODO clean up and use TPayoutDecider
    n_payout_base : isize,
}

impl SPayoutDeciderBettel {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        &self,
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules
    {
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| !fn_is_player_party(rules.winner_index(stich)));
        internal_payout(
            /*n_payout_single_player*/ self.n_payout_base,
            fn_player_multiplier,
            /*ab_winner*/ &EPlayerIndex::map_from_fn(|epi| {
                fn_is_player_party(epi)==b_player_party_wins
            })
        )
    }
}

impl TRules for SRulesBettel {
    box_clone_impl_by_clone!(TRules);
    impl_rules_trumpf!(STrumpfDeciderNoTrumpf);
    impl_single_play!();

    // TODORULES Grasober like bettel, i.e. Stichzwang
    // fn all_allowed_cards_within_stich(&self, vecstich: &[SStich], hand: &SHand) -> SHandVector;

    fn compare_in_stich_same_farbe(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        assert_eq!(self.trumpforfarbe(card_fst), self.trumpforfarbe(card_snd));
        assert_eq!(card_fst.farbe(), card_snd.farbe());
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
