use primitives::*;
use rules::*;
use rules::card_points::*;
use util::*;

#[derive(Clone)]
pub struct SStossDoublingPayoutDecider {}
impl SStossDoublingPayoutDecider {
    pub fn payout(an_payout_raw: EnumMap<EPlayerIndex, isize>, n_stoss: usize, n_doubling: usize) -> EnumMap<EPlayerIndex, isize> {
        EPlayerIndex::map_from_fn(|epi| {
            an_payout_raw[epi] * 2isize.pow((n_stoss + n_doubling).as_num())
        })
    }
}

#[derive(Clone, new)]
pub struct SLaufendeParams {
    m_n_payout_per_lauf : isize,
    m_n_lauf_lbound : usize,
}

#[derive(Clone, new)]
pub struct SPayoutDeciderParams {
    pub m_n_payout_base : isize,
    pub m_n_payout_schneider_schwarz : isize,
    pub m_laufendeparams : SLaufendeParams,
}

pub trait TPayoutDecider : Sync + 'static + Clone {
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
    fn to_string(&self) -> String { // TODO? impl Display
        "".to_string()
    }
}

#[derive(Clone)]
pub struct SPayoutDeciderPointBased {
    m_n_payout_base : isize,
    m_n_payout_schneider_schwarz : isize,
    m_laufendeparams : SLaufendeParams,
    m_prio: VGameAnnouncementPriority,
}

impl SPayoutDeciderPointBased {
    fn internal_new(payoutdeciderparams: SPayoutDeciderParams, n_points_player_to_win: isize, prio: VGameAnnouncementPriority) -> SPayoutDeciderPointBased {
        assert!(61<=n_points_player_to_win);
        assert!(n_points_player_to_win<=120);
        SPayoutDeciderPointBased {
            m_n_payout_base: payoutdeciderparams.m_n_payout_base,
            m_n_payout_schneider_schwarz: payoutdeciderparams.m_n_payout_schneider_schwarz,
            m_laufendeparams: payoutdeciderparams.m_laufendeparams,
            m_prio: prio,
        }
    }
}

impl TPayoutDecider for SPayoutDeciderPointBased {
    type PrioParams = VGameAnnouncementPriority;
    fn new(payoutdeciderparams: SPayoutDeciderParams, prio: VGameAnnouncementPriority) -> Self {
        Self::internal_new(payoutdeciderparams, 61, prio)
    }

    fn priority(&self) -> VGameAnnouncementPriority {
        self.m_prio.clone()
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
            /*n_payout_single_player*/ self.m_n_payout_base
                + { 
                    if gamefinishedstiche.get().iter().all(|stich| b_player_party_wins==fn_is_player_party(rules.winner_index(stich))) {
                        2*self.m_n_payout_schneider_schwarz // schwarz
                    } else if (b_player_party_wins && n_points_player_party>90) || (!b_player_party_wins && n_points_player_party<=30) {
                        self.m_n_payout_schneider_schwarz // schneider
                    } else {
                        0 // "nothing", i.e. neither schneider nor schwarz
                    }
                }
                + self.m_laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner),
            fn_player_multiplier,
            &ab_winner,
        )
    }

    fn with_increased_prio(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Self> {
        use self::VGameAnnouncementPriority::*;
        if let (SoloLikeSteigern(_), &SoloLikeSteigern(n_points_player_to_win_steigered)) = (self.priority(), prio) {
            let n_points_to_win = n_points_player_to_win_steigered + match ebid {
                EBid::AtLeast => 0,
                EBid::Higher => 10, // TODO custom steps
            };
            if n_points_to_win<=120 {
                let mut payoutdecider = self.clone();
                payoutdecider.m_prio = SoloLikeSteigern(n_points_to_win);
                return Some(payoutdecider)
            }
        }
        None
    }
    fn to_string(&self) -> String {
        if let VGameAnnouncementPriority::SoloLikeSteigern(n_points_player_to_win) = self.m_prio {
            if 61<n_points_player_to_win {
                return format!(" for {}", n_points_player_to_win)
            }
        }
        "".to_string()
    }
}

impl SLaufendeParams {
    pub fn payout_laufende<Rules>(&self, rules: &Rules, gamefinishedstiche: &SGameFinishedStiche, ab_winner: &EnumMap<EPlayerIndex, bool>) -> isize 
        where Rules: TRules,
    {
        let n_laufende = rules.count_laufende(gamefinishedstiche, ab_winner);
        (if n_laufende<self.m_n_lauf_lbound {0} else {n_laufende}).as_num::<isize>() * self.m_n_payout_per_lauf
    }
}

fn internal_payout<FnPlayerMultiplier>(n_payout_single_player: isize, fn_player_multiplier: FnPlayerMultiplier, ab_winner: &EnumMap<EPlayerIndex, bool>) -> EnumMap<EPlayerIndex, isize> 
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

#[derive(Clone)]
pub struct SPayoutDeciderTout {
    m_n_payout_base : isize,
    m_laufendeparams : SLaufendeParams,
    m_i_prio: isize,
}

impl TPayoutDecider for SPayoutDeciderTout {
    type PrioParams = isize;
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloTout(self.m_i_prio)
    }

    fn new(payoutdeciderparams: SPayoutDeciderParams, i_prio: isize) -> Self {
        SPayoutDeciderTout {
            m_n_payout_base: payoutdeciderparams.m_n_payout_base,
            m_laufendeparams: payoutdeciderparams.m_laufendeparams,
            m_i_prio: i_prio,
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
        // TODO optionally count schneider/schwarz
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| fn_is_player_party(rules.winner_index(stich)));
        let ab_winner = EPlayerIndex::map_from_fn(|epi| {
            fn_is_player_party(epi)==b_player_party_wins
        });
        internal_payout(
            /*n_payout_single_player*/ (self.m_n_payout_base + self.m_laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner)) * 2,
            fn_player_multiplier,
            &ab_winner,
        )
    }
}

#[derive(Clone)]
pub struct SPayoutDeciderSie {
    m_n_payout_base : isize,
    m_laufendeparams : SLaufendeParams,
}

impl TPayoutDecider for SPayoutDeciderSie {
    type PrioParams = ();

    fn new(payoutdeciderparams: SPayoutDeciderParams, _prioparams: ()) -> Self {
        SPayoutDeciderSie {
            m_n_payout_base: payoutdeciderparams.m_n_payout_base,
            m_laufendeparams: payoutdeciderparams.m_laufendeparams,
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
        // TODO optionally count schneider/schwarz
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| {
                let epi_stich_winner = rules.winner_index(stich);
                rules.trumpforfarbe(stich[epi_stich_winner]).is_trumpf() && fn_is_player_party(epi_stich_winner)
            });
        internal_payout(
            /*n_payout_single_player*/ (self.m_n_payout_base
            + {
                assert_eq!(8, gamefinishedstiche.get().len()); // TODO Kurze Karte supports Sie?
                gamefinishedstiche.get().len().as_num::<isize>()
            } * self.m_laufendeparams.m_n_payout_per_lauf) * 4,
            fn_player_multiplier,
            /*ab_winner*/ &EPlayerIndex::map_from_fn(|epi| {
                fn_is_player_party(epi)==b_player_party_wins
            })
        )
    }
}
