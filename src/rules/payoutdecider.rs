use primitives::*;
use rules::*;
use rules::card_points::*;
use util::*;
use std::fmt::Display;

#[derive(Clone)]
pub struct SStossDoublingPayoutDecider {}
impl SStossDoublingPayoutDecider {
    pub fn payout(an_payout_raw: EnumMap<EPlayerIndex, isize>, (n_stoss, n_doubling): (usize, usize)) -> EnumMap<EPlayerIndex, isize> {
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
    fn new(payoutdeciderparams: SPayoutDeciderParams, prioparams: Self::PrioParams) -> Self;
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
    payoutdeciderparams : SPayoutDeciderParams,
    prio: VGameAnnouncementPriority,
}

impl TPayoutDecider for SPayoutDeciderPointBased {
    type PrioParams = VGameAnnouncementPriority;
    fn new(payoutdeciderparams: SPayoutDeciderParams, prio: VGameAnnouncementPriority) -> Self {
        SPayoutDeciderPointBased {
            payoutdeciderparams,
            prio,
        }
    }

    fn priority(&self) -> VGameAnnouncementPriority {
        self.prio.clone()
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
        use self::VGameAnnouncementPriority::*;
        let b_player_party_wins = n_points_player_party >= match self.priority() {
            RufspielLike | SoloLikeSimple(_) => 61,
            SoloLikeSteigern(n_points) => n_points,
            SoloTout(_) | SoloSie => panic!("Unexpected priority in SPayoutDeciderPointBased"),
        };
        let ab_winner = EPlayerIndex::map_from_fn(|epi| {
            fn_is_player_party(epi)==b_player_party_wins
        });
        internal_payout(
            /*n_payout_single_player*/ self.payoutdeciderparams.n_payout_base
                + { 
                    if gamefinishedstiche.get().iter().all(|stich| b_player_party_wins==fn_is_player_party(rules.winner_index(stich))) {
                        2*self.payoutdeciderparams.n_payout_schneider_schwarz // schwarz
                    } else if (b_player_party_wins && n_points_player_party>90) || (!b_player_party_wins && n_points_player_party<=30) {
                        self.payoutdeciderparams.n_payout_schneider_schwarz // schneider
                    } else {
                        0 // "nothing", i.e. neither schneider nor schwarz
                    }
                }
                + self.payoutdeciderparams.laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner),
            fn_player_multiplier,
            &ab_winner,
        )
    }

    fn with_increased_prio(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Self> {
        use self::VGameAnnouncementPriority::*;
        if let (SoloLikeSteigern(_), &SoloLikeSteigern(n_points_player_to_win_steigered)) = (self.priority(), prio) {
            let n_points_to_win = n_points_player_to_win_steigered + match ebid {
                EBid::AtLeast => 0,
                EBid::Higher => 10, // TODORULES custom steps
            };
            if n_points_to_win<=120 {
                let mut payoutdecider = self.clone();
                payoutdecider.prio = SoloLikeSteigern(n_points_to_win);
                return Some(payoutdecider)
            }
        }
        None
    }
}

impl Display for SPayoutDeciderPointBased {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::VGameAnnouncementPriority::*;
        match self.priority() {
            RufspielLike | SoloLikeSimple(_) => Ok(()), // no special indication required
            SoloLikeSteigern(n_points) => {
                assert!(61<=n_points);
                if n_points<61 {
                    write!(f, "for {}", n_points)
                } else {
                    Ok(())
                }
            },
            SoloTout(_) | SoloSie => panic!("Unexpected priority in SPayoutDeciderPointBased"),
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
    payoutdeciderparams : SPayoutDeciderParams,
    i_prio: isize,
}

impl TPayoutDecider for SPayoutDeciderTout {
    type PrioParams = isize;
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloTout(self.i_prio)
    }

    fn new(payoutdeciderparams: SPayoutDeciderParams, i_prio: isize) -> Self {
        SPayoutDeciderTout {
            payoutdeciderparams,
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
            /*n_payout_single_player*/ (self.payoutdeciderparams.n_payout_base + self.payoutdeciderparams.laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner)) * 2,
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
    payoutdeciderparams : SPayoutDeciderParams,
}

impl TPayoutDecider for SPayoutDeciderSie {
    type PrioParams = ();

    fn new(payoutdeciderparams: SPayoutDeciderParams, _prioparams: ()) -> Self {
        SPayoutDeciderSie {
            payoutdeciderparams,
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
            /*n_payout_single_player*/ (self.payoutdeciderparams.n_payout_base
            + {
                gamefinishedstiche.get().len().as_num::<isize>()
            } * self.payoutdeciderparams.laufendeparams.n_payout_per_lauf) * 4,
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
