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

pub trait TPayoutDecider {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
        n_payout_base: isize,
        n_payout_lauf: isize,
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
        n_payout_lauf: isize,
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
                        20 // schwarz
                    } else if (b_player_party_wins && n_points_player_party>90) || (!b_player_party_wins && n_points_player_party<=30) {
                        10 // schneider
                    } else {
                        0 // "nothing", i.e. neither schneider nor schwarz
                    }
                }
                + payout_laufende(rules, gamefinishedstiche, &ab_winner, n_payout_lauf),
            fn_player_multiplier,
            &ab_winner,
        )
    }
}

fn payout_laufende<Rules>(rules: &Rules, gamefinishedstiche: &SGameFinishedStiche, ab_winner: &SPlayerIndexMap<bool>, n_payout_lauf: isize) -> isize 
    where Rules: TRules,
{
    let n_laufende = rules.count_laufende(gamefinishedstiche, ab_winner);
    (if n_laufende<3 {0} else {n_laufende}) * n_payout_lauf
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
        n_payout_lauf: isize,
    ) -> SPlayerIndexMap<isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| fn_is_player_party(rules.winner_index(stich)));
        let ab_winner = create_playerindexmap(|eplayerindex| {
            fn_is_player_party(eplayerindex)==b_player_party_wins
        });
        internal_payout(
            /*n_payout_single_player*/ (n_payout_base + payout_laufende(rules, gamefinishedstiche, &ab_winner, n_payout_lauf)) * 2,
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
        n_payout_lauf: isize,
    ) -> SPlayerIndexMap<isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
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
            } * n_payout_lauf) * 4,
            fn_player_multiplier,
            /*ab_winner*/ &create_playerindexmap(|eplayerindex| {
                fn_is_player_party(eplayerindex)==b_player_party_wins
            })
        )
    }
}
