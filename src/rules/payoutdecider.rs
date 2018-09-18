use primitives::*;
use rules::{
    *,
    card_points::*,
    trumpfdecider::TTrumpfDecider,
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
    fn payout<Rules, PlayerParties>(
        &self,
        rules: &Rules,
        gamefinishedstiche: SGameFinishedStiche,
        playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, isize>
        where PlayerParties: TPlayerParties,
              Rules: TRulesNoObj;

    fn payouthints<Rules, PlayerParties>(
        &self,
        _rules: &Rules,
        _slcstich: &[SStich],
        _ahand: &EnumMap<EPlayerIndex, SHand>,
        _playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, (Option<isize>, Option<isize>)>
        where PlayerParties: TPlayerParties,
              Rules: TRulesNoObj
    {
        // TODO remove default implementation and customize all instances
        EPlayerIndex::map_from_fn(|_epi| (None, None))
    }
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
    fn payout<Rules, PlayerParties>(
        &self,
        rules: &Rules,
        gamefinishedstiche: SGameFinishedStiche,
        playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, isize>
        where PlayerParties: TPlayerParties,
              Rules: TRulesNoObj,
    {
        let n_points_primary_party : isize = gamefinishedstiche.get().iter()
            .filter(|stich| playerparties.is_primary_party(rules.winner_index(stich)))
            .map(|stich| points_stich(stich))
            .sum();
        let b_primary_party_wins = n_points_primary_party >= self.pointstowin.points_to_win();
        internal_payout(
            /*n_payout_single_player*/ self.payoutparams.n_payout_base
                + { 
                    if gamefinishedstiche.get().iter().all(|stich| b_primary_party_wins==playerparties.is_primary_party(rules.winner_index(stich))) {
                        2*self.payoutparams.n_payout_schneider_schwarz // schwarz
                    } else if (b_primary_party_wins && n_points_primary_party>90) || (!b_primary_party_wins && n_points_primary_party<=30) {
                        self.payoutparams.n_payout_schneider_schwarz // schneider
                    } else {
                        0 // "nothing", i.e. neither schneider nor schwarz
                    }
                }
                + self.payoutparams.laufendeparams.payout_laufende::<Rules, _>(gamefinishedstiche, playerparties),
            playerparties,
            b_primary_party_wins,
        )
    }
}

impl SLaufendeParams {
    pub fn payout_laufende<Rules: TRulesNoObj, PlayerParties: TPlayerParties>(&self, gamefinishedstiche: SGameFinishedStiche, playerparties: &PlayerParties) -> isize {
        #[cfg(debug_assertions)]
        let mut mapcardb_used = SCard::map_from_fn(|_card| false);
        let mapcardepi = {
            let mut mapcardepi = SCard::map_from_fn(|_card| EPlayerIndex::EPI0);
            for (epi, card) in gamefinishedstiche.get().iter().flat_map(|stich| stich.iter()) {
                #[cfg(debug_assertions)] {
                    mapcardb_used[*card] = true;
                }
                mapcardepi[*card] = epi;
            }
            mapcardepi
        };
        let ekurzlang = EKurzLang::from_cards_per_player(gamefinishedstiche.get().len());
        #[cfg(debug_assertions)]
        assert!(SCard::values(ekurzlang).all(|card| mapcardb_used[card]));
        let laufende_relevant = |card: SCard| {
            playerparties.is_primary_party(mapcardepi[card])
        };
        let mut itcard_trumpf_descending = Rules::TrumpfDecider::trumpfs_in_descending_order();
        let b_might_have_lauf = laufende_relevant(verify!(itcard_trumpf_descending.nth(0)).unwrap());
        let n_laufende = itcard_trumpf_descending
            .filter(|card| ekurzlang.supports_card(*card))
            .take_while(|card| b_might_have_lauf==laufende_relevant(*card))
            .count()
            + 1 // consumed by nth(0)
        ;
        (if n_laufende<self.n_lauf_lbound {0} else {n_laufende}).as_num::<isize>() * self.n_payout_per_lauf
    }
}

pub fn internal_payout<PlayerParties>(n_payout_single_player: isize, playerparties: &PlayerParties, b_primary_party_wins: bool) -> EnumMap<EPlayerIndex, isize> 
    where PlayerParties: TPlayerParties,
{
    EPlayerIndex::map_from_fn(|epi| {
        n_payout_single_player 
        * {
            if playerparties.is_primary_party(epi)==b_primary_party_wins {
                1
            } else {
                -1
            }
        }
        * playerparties.multiplier(epi)
    })
}

