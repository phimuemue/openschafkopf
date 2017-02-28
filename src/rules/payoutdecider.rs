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
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
        payoutdeciderparams: &SPayoutDeciderParams,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules;
}

#[derive(Clone)]
pub struct SPayoutDeciderPointBased {}

impl TPayoutDecider for SPayoutDeciderPointBased {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
        payoutdeciderparams: &SPayoutDeciderParams,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        let n_points_player_party : isize = gamefinishedstiche.get().iter()
            .filter(|stich| fn_is_player_party(rules.winner_index(stich)))
            .map(|stich| points_stich(stich))
            .sum();
        let b_player_party_wins = n_points_player_party>=61;
        let ab_winner = EPlayerIndex::map_from_fn(|epi| {
            fn_is_player_party(epi)==b_player_party_wins
        });
        internal_payout(
            /*n_payout_single_player*/ payoutdeciderparams.m_n_payout_base
                + { 
                    if gamefinishedstiche.get().iter().all(|stich| b_player_party_wins==fn_is_player_party(rules.winner_index(stich))) {
                        2*payoutdeciderparams.m_n_payout_schneider_schwarz // schwarz
                    } else if (b_player_party_wins && n_points_player_party>90) || (!b_player_party_wins && n_points_player_party<=30) {
                        payoutdeciderparams.m_n_payout_schneider_schwarz // schneider
                    } else {
                        0 // "nothing", i.e. neither schneider nor schwarz
                    }
                }
                + payoutdeciderparams.m_laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner),
            fn_player_multiplier,
            &ab_winner,
        )
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
pub struct SPayoutDeciderTout {}

impl TPayoutDecider for SPayoutDeciderTout {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
        payoutdeciderparams: &SPayoutDeciderParams,
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
            /*n_payout_single_player*/ (payoutdeciderparams.m_n_payout_base + payoutdeciderparams.m_laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner)) * 2,
            fn_player_multiplier,
            &ab_winner,
        )
    }
}

#[derive(Clone)]
pub struct SPayoutDeciderSie {}

impl TPayoutDecider for SPayoutDeciderSie {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
        payoutdeciderparams: &SPayoutDeciderParams,
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
            /*n_payout_single_player*/ (payoutdeciderparams.m_n_payout_base
            + {
                assert_eq!(8, gamefinishedstiche.get().len()); // TODO Kurze Karte supports Sie?
                gamefinishedstiche.get().len().as_num::<isize>()
            } * payoutdeciderparams.m_laufendeparams.m_n_payout_per_lauf) * 4,
            fn_player_multiplier,
            /*ab_winner*/ &EPlayerIndex::map_from_fn(|epi| {
                fn_is_player_party(epi)==b_player_party_wins
            })
        )
    }
}
