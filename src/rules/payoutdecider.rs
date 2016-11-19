use primitives::*;
use rules::*;

pub trait TPayoutDecider {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        rules: &Rules,
        gamefinishedstiche: &SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
        n_payout_base: isize,
    ) -> [isize; 4]
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
    ) -> [isize; 4]
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        let n_points_player_party : isize = gamefinishedstiche.get().iter()
            .filter(|stich| fn_is_player_party(rules.winner_index(stich)))
            .map(|stich| rules.points_stich(stich))
            .sum();
        let b_player_party_wins = n_points_player_party>=61;
        enum ESchneiderSchwarz {
            Nothing,
            Schneider,
            Schwarz,
        }
        let eschneiderschwarz = 
            if b_player_party_wins {
                if gamefinishedstiche.get().iter().all(|stich| fn_is_player_party(rules.winner_index(stich))) {
                    ESchneiderSchwarz::Schwarz
                } else if n_points_player_party>90 {
                    ESchneiderSchwarz::Schneider
                } else {
                    ESchneiderSchwarz::Nothing
                }
            } else {
                if gamefinishedstiche.get().iter().all(|stich| !fn_is_player_party(rules.winner_index(stich))) {
                    ESchneiderSchwarz::Schwarz
                } else if n_points_player_party<=30 {
                    ESchneiderSchwarz::Schneider
                } else {
                    ESchneiderSchwarz::Nothing
                }
            };
        let ab_winner = create_playerindexmap(|eplayerindex| {
            fn_is_player_party(eplayerindex)==b_player_party_wins
        });
        internal_payout(
            /*n_payout_single_player*/ n_payout_base
                + { match eschneiderschwarz {
                    ESchneiderSchwarz::Nothing => 0,
                    ESchneiderSchwarz::Schneider => 10,
                    ESchneiderSchwarz::Schwarz => 20,
                }}
                + payout_laufende(rules, gamefinishedstiche, &ab_winner),
            fn_player_multiplier,
            &ab_winner,
        )
    }
}

const N_PAYOUT_PER_LAUFENDE : isize = 10;

fn payout_laufende<Rules>(rules: &Rules, gamefinishedstiche: &SGameFinishedStiche, ab_winner: &[bool; 4]) -> isize 
    where Rules: TRules,
{
    let n_laufende = rules.count_laufende(gamefinishedstiche, ab_winner);
    (if n_laufende<3 {0} else {n_laufende}) * N_PAYOUT_PER_LAUFENDE
}

fn internal_payout<FnPlayerMultiplier>(n_payout_single_player: isize, fn_player_multiplier: FnPlayerMultiplier, ab_winner: &[bool; 4]) -> [isize; 4] 
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
    ) -> [isize; 4]
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
            /*n_payout_single_player*/ (n_payout_base + payout_laufende(rules, gamefinishedstiche, &ab_winner)) * 2,
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
    ) -> [isize; 4]
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
            } * N_PAYOUT_PER_LAUFENDE) * 4,
            fn_player_multiplier,
            /*ab_winner*/ &create_playerindexmap(|eplayerindex| {
                fn_is_player_party(eplayerindex)==b_player_party_wins
            })
        )
    }
}
