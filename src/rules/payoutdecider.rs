use primitives::*;
use rules::{
    *,
    card_points::*,
};
use util::*;

#[derive(Clone, new, Debug)]
pub struct SLaufendeParams {
    pub n_payout_per_lauf : isize,
    n_lauf_lbound : usize,
}

#[derive(Clone, new, Debug)]
pub struct SPayoutDeciderParams {
    pub n_payout_base : isize,
    pub n_payout_schneider_schwarz : isize,
    pub laufendeparams : SLaufendeParams,
}

pub trait TPayoutDecider : Sync + 'static + Clone + fmt::Debug {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        &self,
        rules: &Rules,
        gamefinishedstiche: SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules;
}

pub trait TPointsToWin : Sync + 'static + Clone + fmt::Debug {
    fn points_to_win(&self) -> isize;
}

#[derive(Clone, Debug)]
pub struct SPointsToWin61;

impl TPointsToWin for SPointsToWin61 {
    fn points_to_win(&self) -> isize {
        61
    }
}

#[derive(Clone, Debug, new)]
pub struct SPayoutDeciderPointBased<PointsToWin: TPointsToWin> {
    pub payoutparams : SPayoutDeciderParams,
    pub pointstowin: PointsToWin,
}

impl<PointsToWin: TPointsToWin> TPayoutDecider for SPayoutDeciderPointBased<PointsToWin> {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        &self,
        rules: &Rules,
        gamefinishedstiche: SGameFinishedStiche,
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
        let b_player_party_wins = n_points_player_party >= self.pointstowin.points_to_win();
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
}

impl SLaufendeParams {
    pub fn payout_laufende<Rules>(&self, rules: &Rules, gamefinishedstiche: SGameFinishedStiche, ab_winner: &EnumMap<EPlayerIndex, bool>) -> isize 
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

