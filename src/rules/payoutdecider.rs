use primitives::*;
use rules::*;
use rules::card_points::*;

pub struct SStossDoublingPayoutDecider {}
impl SStossDoublingPayoutDecider {
    pub fn payout(an_payout_raw: SPlayerIndexMap<isize>, n_stoss: usize, n_doubling: usize) -> SPlayerIndexMap<isize> {
        create_playerindexmap(|eplayerindex| {
            an_payout_raw[eplayerindex] * 2isize.pow((n_stoss + n_doubling) as u32)
        })
    }
}

pub struct SLaufendeParams {
    m_n_payout_per_lauf : isize,
    m_n_lauf_lbound : usize,
}

pub trait TPayoutDecider {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
        n_payout_base: isize,
        n_payout_schneider_schwarz: isize,
        laufendeparams: &SLaufendeParams,
    ) -> SPlayerIndexMap<isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules;
}

pub struct SPayoutDeciderPointBased {}

impl TPayoutDecider for SPayoutDeciderPointBased {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
        n_payout_base: isize,
        n_payout_schneider_schwarz: isize,
        laufendeparams: &SLaufendeParams,
    ) -> SPlayerIndexMap<isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        let n_points_player_party : isize = gamefinishedstiche.get().iter()
            .filter(|stich| fn_is_player_party(rules.winner_index(stich)))
            .map(|stich| points_stich(stich))
            .sum();
        let b_player_party_wins = n_points_player_party>=61;
        let ab_winner = create_playerindexmap(|eplayerindex| {
            fn_is_player_party(eplayerindex)==b_player_party_wins
        });
        internal_payout(
            /*n_payout_single_player*/ n_payout_base
                + { 
                    if gamefinishedstiche.get().iter().all(|stich| b_player_party_wins==fn_is_player_party(rules.winner_index(stich))) {
                        2*n_payout_schneider_schwarz // schwarz
                    } else if (b_player_party_wins && n_points_player_party>90) || (!b_player_party_wins && n_points_player_party<=30) {
                        n_payout_schneider_schwarz // schneider
                    } else {
                        0 // "nothing", i.e. neither schneider nor schwarz
                    }
                }
                + laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner),
            fn_player_multiplier,
            &ab_winner,
        )
    }
}

impl SLaufendeParams {
    pub fn new(n_payout_per_lauf: isize, n_lauf_lbound: usize) -> SLaufendeParams {
        SLaufendeParams {
            m_n_payout_per_lauf : n_payout_per_lauf,
            m_n_lauf_lbound : n_lauf_lbound,
        }
    }
    pub fn payout_laufende<Rules>(&self, rules: &Rules, gamefinishedstiche: &SGameFinishedStiche, ab_winner: &SPlayerIndexMap<bool>) -> isize 
        where Rules: TRules,
    {
        let n_laufende = rules.count_laufende(gamefinishedstiche, ab_winner);
        (if n_laufende<self.m_n_lauf_lbound {0} else {n_laufende} as isize) * self.m_n_payout_per_lauf
    }
}

fn internal_payout<FnPlayerMultiplier>(n_payout_single_player: isize, fn_player_multiplier: FnPlayerMultiplier, ab_winner: &SPlayerIndexMap<bool>) -> SPlayerIndexMap<isize> 
    where FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
{
    create_playerindexmap(|eplayerindex| {
        n_payout_single_player 
        * {
            if ab_winner[eplayerindex] {
                1
            } else {
                -1
            }
        }
        * fn_player_multiplier(eplayerindex)
    })
}

pub struct SPayoutDeciderTout {}

impl TPayoutDecider for SPayoutDeciderTout {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
        n_payout_base: isize,
        _n_payout_schneider_schwarz: isize,
        laufendeparams: &SLaufendeParams,
    ) -> SPlayerIndexMap<isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        // TODO optionally count schneider/schwarz
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| fn_is_player_party(rules.winner_index(stich)));
        let ab_winner = create_playerindexmap(|eplayerindex| {
            fn_is_player_party(eplayerindex)==b_player_party_wins
        });
        internal_payout(
            /*n_payout_single_player*/ (n_payout_base + laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner)) * 2,
            fn_player_multiplier,
            &ab_winner,
        )
    }
}

pub struct SPayoutDeciderSie {}

impl TPayoutDecider for SPayoutDeciderSie {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
        n_payout_base: isize,
        _n_payout_schneider_schwarz: isize,
        laufendeparams: &SLaufendeParams,
    ) -> SPlayerIndexMap<isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        // TODO optionally count schneider/schwarz
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| {
                let eplayerindex_stich_winner = rules.winner_index(stich);
                rules.trumpforfarbe(stich[eplayerindex_stich_winner]).is_trumpf() && fn_is_player_party(eplayerindex_stich_winner)
            });
        internal_payout(
            /*n_payout_single_player*/ (n_payout_base
            + {
                assert_eq!(8, gamefinishedstiche.get().len()); // TODO Kurze Karte supports Sie?
                gamefinishedstiche.get().len() as isize
            } * laufendeparams.m_n_payout_per_lauf) * 4,
            fn_player_multiplier,
            /*ab_winner*/ &create_playerindexmap(|eplayerindex| {
                fn_is_player_party(eplayerindex)==b_player_party_wins
            })
        )
    }
}
