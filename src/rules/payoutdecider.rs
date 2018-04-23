use primitives::*;
use rules::{
    *,
    card_points::*,
};
use util::*;
use std::fmt::Display;

#[derive(Clone)]
pub struct SStossDoublingPayoutDecider {}
impl SStossDoublingPayoutDecider {
    pub fn payout(an_payout_raw: &EnumMap<EPlayerIndex, isize>, (n_stoss, n_doubling): (usize, usize)) -> EnumMap<EPlayerIndex, isize> {
        EPlayerIndex::map_from_fn(|epi| {
            an_payout_raw[epi] * 2isize.pow((n_stoss + n_doubling).as_num())
        })
    }
}

#[derive(Clone, new, Debug)]
pub struct SLaufendeParams {
    n_payout_per_lauf : isize,
    n_lauf_lbound : usize,
}

#[derive(Clone, new, Debug)]
pub struct SPayoutDeciderParams {
    pub n_payout_base : isize,
    pub n_payout_schneider_schwarz : isize,
    pub laufendeparams : SLaufendeParams,
}

pub trait TPayoutDecider : Sync + 'static + Clone + Display + fmt::Debug {
    type PrioParams;
    type PayoutParams;
    fn new(payoutparams: Self::PayoutParams, prioparams: Self::PrioParams) -> Self;
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        &self,
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules;
    fn priority(&self) -> VGameAnnouncementPriority;
    fn with_increased_prio(&self, _prio: &VGameAnnouncementPriority, _ebid: EBid) -> Option<Self> {
        None
    }
}

#[derive(Clone, Debug)]
pub struct SPayoutDeciderPointBased {
    payoutparams : SPayoutDeciderParams,
    priopointbased: VGameAnnouncementPriorityPointBased,
}

impl TPayoutDecider for SPayoutDeciderPointBased {
    type PrioParams = VGameAnnouncementPriorityPointBased;
    type PayoutParams = SPayoutDeciderParams;

    fn new(payoutparams: Self::PayoutParams, priopointbased: Self::PrioParams) -> Self {
        SPayoutDeciderPointBased {
            payoutparams,
            priopointbased,
        }
    }

    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::PointBased(self.priopointbased.clone())
    }

    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        &self,
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        let n_points_player_party : isize = gamefinishedstiche.get().iter()
            .filter(|stich| fn_is_player_party(rules.winner_index(stich)))
            .map(|stich| points_stich(stich))
            .sum();
        use self::VGameAnnouncementPriorityPointBased::*;
        let b_player_party_wins = n_points_player_party >= match self.priopointbased {
            RufspielLike | SoloSimple(_) => 61,
            SoloSteigern{n_points_to_win, ..} => n_points_to_win,
        };
        let ab_winner = EPlayerIndex::map_from_fn(|epi| {
            fn_is_player_party(epi)==b_player_party_wins
        });
        internal_payout(
            /*n_payout_single_player*/ self.payoutparams.n_payout_base
                + { 
                    if gamefinishedstiche.get().iter().all(|stich| b_player_party_wins==fn_is_player_party(rules.winner_index(stich))) {
                        2*self.payoutparams.n_payout_schneider_schwarz // schwarz
                    } else if (b_player_party_wins && n_points_player_party>90) || (!b_player_party_wins && n_points_player_party<=30) {
                        self.payoutparams.n_payout_schneider_schwarz // schneider
                    } else {
                        0 // "nothing", i.e. neither schneider nor schwarz
                    }
                }
                + self.payoutparams.laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner),
            fn_player_multiplier,
            &ab_winner,
        )
    }

    fn with_increased_prio(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Self> {
        use self::VGameAnnouncementPriority::*;
        use self::VGameAnnouncementPriorityPointBased::*;
        if let (PointBased(SoloSteigern{..}), &PointBased(SoloSteigern{n_points_to_win, n_step})) = (self.priority(), prio) {
            let n_points_to_win_steigered = n_points_to_win + match ebid {
                EBid::AtLeast => 0,
                EBid::Higher => n_step,
            };
            if n_points_to_win_steigered<=120 {
                let mut payoutdecider = self.clone();
                payoutdecider.priopointbased = SoloSteigern{n_points_to_win: n_points_to_win_steigered, n_step};
                return Some(payoutdecider)
            }
        }
        None
    }
}

impl Display for SPayoutDeciderPointBased {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::VGameAnnouncementPriorityPointBased::*;
        match self.priopointbased {
            RufspielLike | SoloSimple(_) => Ok(()), // no special indication required
            SoloSteigern{n_points_to_win, ..} => {
                assert!(61<=n_points_to_win);
                if n_points_to_win<61 {
                    write!(f, "for {}", n_points_to_win)
                } else {
                    Ok(())
                }
            },
        }
    }
}

impl SLaufendeParams {
    pub fn payout_laufende<Rules>(&self, rules: &Rules, gamefinishedstiche: &SGameFinishedStiche, ab_winner: &EnumMap<EPlayerIndex, bool>) -> isize 
        where Rules: TRules,
    {
        let n_laufende = rules.count_laufende(gamefinishedstiche, ab_winner);
        (if n_laufende<self.n_lauf_lbound {0} else {n_laufende}).as_num::<isize>() * self.n_payout_per_lauf
    }
}

pub fn internal_payout<FnPlayerMultiplier>(n_payout_single_player: isize, fn_player_multiplier: FnPlayerMultiplier, ab_winner: &EnumMap<EPlayerIndex, bool>) -> EnumMap<EPlayerIndex, isize> 
    where FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
{
    EPlayerIndex::map_from_fn(|epi| {
        n_payout_single_player 
        * {
            if ab_winner[epi] {
                1
            } else {
                -1
            }
        }
        * fn_player_multiplier(epi)
    })
}

#[derive(Clone, Debug)]
pub struct SPayoutDeciderTout {
    payoutparams : SPayoutDeciderParams,
    i_prio: isize,
}

impl TPayoutDecider for SPayoutDeciderTout {
    type PrioParams = isize;
    type PayoutParams = SPayoutDeciderParams;

    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloTout(self.i_prio)
    }

    fn new(payoutparams: Self::PayoutParams, i_prio: isize) -> Self {
        SPayoutDeciderTout {
            payoutparams,
            i_prio,
        }
    }

    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        &self,
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        // TODORULES optionally count schneider/schwarz
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| fn_is_player_party(rules.winner_index(stich)));
        let ab_winner = EPlayerIndex::map_from_fn(|epi| {
            fn_is_player_party(epi)==b_player_party_wins
        });
        internal_payout(
            /*n_payout_single_player*/ (self.payoutparams.n_payout_base + self.payoutparams.laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner)) * 2,
            fn_player_multiplier,
            &ab_winner,
        )
    }
}
impl Display for SPayoutDeciderTout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

#[derive(Clone, Debug)]
pub struct SPayoutDeciderSie {
    payoutparams : SPayoutDeciderParams,
}

impl TPayoutDecider for SPayoutDeciderSie {
    type PrioParams = ();
    type PayoutParams = SPayoutDeciderParams;

    fn new(payoutparams: Self::PayoutParams, _prioparams: ()) -> Self {
        SPayoutDeciderSie {
            payoutparams,
        }
    }

    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloSie
    }

    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        &self,
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        // TODORULES optionally count schneider/schwarz
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| {
                let epi_stich_winner = rules.winner_index(stich);
                rules.trumpforfarbe(stich[epi_stich_winner]).is_trumpf() && fn_is_player_party(epi_stich_winner)
            })
            && match EKurzLang::from_cards_per_player(gamefinishedstiche.get().len()) {
                EKurzLang::Lang => true,
                EKurzLang::Kurz => {
                    gamefinishedstiche.get().iter()
                        .all(|stich| {
                            let epi_stich_winner = rules.winner_index(stich);
                            let card = stich[epi_stich_winner];
                            assert!(rules.trumpforfarbe(card).is_trumpf());
                            card.schlag()==ESchlag::Ober || {
                                assert_eq!(card.schlag(), ESchlag::Unter);
                                card.farbe()==EFarbe::Eichel || card.farbe()==EFarbe::Gras
                            }
                        })
                },
            }
        ;
        internal_payout(
            /*n_payout_single_player*/ (self.payoutparams.n_payout_base
            + {
                gamefinishedstiche.get().len().as_num::<isize>()
            } * self.payoutparams.laufendeparams.n_payout_per_lauf) * 4,
            fn_player_multiplier,
            /*ab_winner*/ &EPlayerIndex::map_from_fn(|epi| {
                fn_is_player_party(epi)==b_player_party_wins
            })
        )
    }
}

impl Display for SPayoutDeciderSie {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}
