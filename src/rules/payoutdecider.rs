use primitives::*;
use rules::*;

pub struct SPayoutDeciderPointBased {}

impl SPayoutDeciderPointBased {
    pub fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
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
        let n_laufende = rules.count_laufende(gamefinishedstiche, &ab_winner);
        let n_payout_single_player = 
            n_payout_base
            + { match eschneiderschwarz {
                ESchneiderSchwarz::Nothing => 0,
                ESchneiderSchwarz::Schneider => 10,
                ESchneiderSchwarz::Schwarz => 20,
            }}
            + {if n_laufende<3 {0} else {n_laufende}} * 10;
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
}
