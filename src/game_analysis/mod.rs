use crate::primitives::*;
use crate::util::*;
use crate::game::*;
use crate::rules::{*, rulessolo::*, ruleset::*, payoutdecider::*};

pub trait TPayoutDeciderSoloLikeDefault : TPayoutDeciderSoloLike {
    fn default_payoutdecider(n_payout_base: isize, n_payout_schneider_schwarz: isize, laufendeparams: SLaufendeParams) -> Self;
}
impl TPayoutDeciderSoloLikeDefault for SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike> {
    fn default_payoutdecider(n_payout_base: isize, n_payout_schneider_schwarz: isize, laufendeparams: SLaufendeParams) -> Self {
        Self::new(
            SPayoutDeciderParams::new(n_payout_base, n_payout_schneider_schwarz, laufendeparams),
            VGameAnnouncementPrioritySoloLike::SoloSimple(0),
        )
    }
}
impl TPayoutDeciderSoloLikeDefault for SPayoutDeciderTout {
    fn default_payoutdecider(n_payout_base: isize, n_payout_schneider_schwarz: isize, laufendeparams: SLaufendeParams) -> Self {
        Self::new(
            SPayoutDeciderParams::new(n_payout_base, n_payout_schneider_schwarz, laufendeparams),
            0,
        )
    }
}

#[cfg(test)]
pub fn make_stich_vector(vecpairnacard_stich: &[(usize, [SCard; 4])]) -> Vec<SStich> {
    vecpairnacard_stich.iter()
        .map(|&(n_epi, acard)| {
            SStich::new_full(EPlayerIndex::from_usize(n_epi), acard)
        })
        .collect()
}

pub fn analyze_game_internal(
    epi_first: EPlayerIndex,
    rules: &dyn TRules,
    ahand: EnumMap<EPlayerIndex, SHand>,
    vecn_doubling: Vec<usize>,
    vecn_stoss: Vec<usize>,
    n_stock: isize,
    slcstich_test: &[SStich],
    mut fn_before_zugeben: impl FnMut(&SGame, /*i_stich*/usize, EPlayerIndex, SCard),
) -> SGame { // TODO return SGameResult
    let mut game = SGame::new(
        ahand,
        SDoublings::new_full(
            epi_first,
            EPlayerIndex::map_from_fn(|epi| 
                vecn_doubling.contains(&epi.wrapping_add(epi_first.to_usize()).to_usize())
            ).into_raw()
        ),
        Some(SStossParams::new(
            /*n_stoss_max*/4,
        )),
        rules.box_clone(),
        n_stock,
    );
    for n_epi_stoss in vecn_stoss {
        debug_verify!(game.stoss(EPlayerIndex::from_usize(n_epi_stoss))).unwrap();
    }
    for (i_stich, stich) in slcstich_test.iter().enumerate() {
        println!("Stich {}: {}", i_stich, stich);
        assert_eq!(Some(stich.first_playerindex()), game.which_player_can_do_something().map(|gameaction| gameaction.0));
        for (epi, card) in stich.iter() {
            assert_eq!(Some(epi), game.which_player_can_do_something().map(|gameaction| gameaction.0));
            println!("{}, {}", card, epi);
            fn_before_zugeben(&game, i_stich, epi, *card);
            debug_verify!(game.zugeben(*card, epi)).unwrap();
        }
    }
    for (i_stich, stich) in game.stichseq.visible_stichs().enumerate() {
        assert_eq!(stich, &slcstich_test[i_stich]);
        println!("Stich {}: {}", i_stich, stich);
    }
    game
}

