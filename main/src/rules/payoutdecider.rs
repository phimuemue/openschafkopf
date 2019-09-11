use crate::primitives::*;
use crate::rules::{
    *,
    trumpfdecider::TTrumpfDecider,
};
use crate::util::*;

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
pub struct SPayoutDeciderPointBased<PointsToWin> {
    pub payoutparams : SPayoutDeciderParams,
    pub pointstowin: PointsToWin,
}

impl<PointsToWin: TPointsToWin> SPayoutDeciderPointBased<PointsToWin> {
    pub fn payout<Rules>(
        &self,
        if_dbg_else!({rules}{_rules}): &Rules,
        rulestatecache: &SRuleStateCache,
        gamefinishedstiche: SStichSequenceGameFinished,
        playerparties: &impl TPlayerParties,
        perepi: impl TPerEPI,
    ) -> isize
        where 
            Rules: TRulesNoObj,
    {
        let n_points_primary_party = debug_verify_eq!(
            EPlayerIndex::values()
                .filter(|epi| playerparties.is_primary_party(*epi))
                .map(|epi| rulestatecache.changing.mapepipointstichcount[epi].n_point)
                .sum::<isize>(),
            gamefinishedstiche.get().completed_stichs_winner_index(rules)
                .filter(|&(_stich, epi_winner)| playerparties.is_primary_party(epi_winner))
                .map(|(stich, _epi_winner)| card_points::points_stich(stich))
                .sum::<isize>()
        );
        let b_primary_party_wins = n_points_primary_party >= self.pointstowin.points_to_win();
        internal_payout(
            /*n_payout_single_player*/ self.payoutparams.n_payout_base
                + { 
                    if debug_verify_eq!(
                        EPlayerIndex::values()
                            .filter(|epi| b_primary_party_wins==playerparties.is_primary_party(*epi))
                            .map(|epi|  rulestatecache.changing.mapepipointstichcount[epi].n_stich)
                            .sum::<usize>()==gamefinishedstiche.get().kurzlang().cards_per_player(),
                        gamefinishedstiche.get().completed_stichs_winner_index(rules)
                            .all(|(_stich, epi_winner)| b_primary_party_wins==playerparties.is_primary_party(epi_winner))
                    ) {
                        2*self.payoutparams.n_payout_schneider_schwarz // schwarz
                    } else if (b_primary_party_wins && n_points_primary_party>90) || (!b_primary_party_wins && n_points_primary_party<=30) {
                        self.payoutparams.n_payout_schneider_schwarz // schneider
                    } else {
                        0 // "nothing", i.e. neither schneider nor schwarz
                    }
                }
                + self.payoutparams.laufendeparams.payout_laufende::<Rules, _>(rulestatecache, gamefinishedstiche, playerparties),
            playerparties,
            b_primary_party_wins,
            perepi,
        )
    }

    pub fn payouthints<Rules: TRulesNoObj, PlayerParties: TPlayerParties>(
        &self,
        if_dbg_else!({rules}{_rules}): &Rules,
        if_dbg_else!({stichseq}{_stichseq}): &SStichSequence,
        _ahand: &EnumMap<EPlayerIndex, SHand>,
        rulestatecache: &SRuleStateCache,
        playerparties: &PlayerParties,
        perepi: impl TPerEPI,
    ) -> (Option<isize>, Option<isize>) {
        let mapbn_points = debug_verify_eq!(
            EPlayerIndex::values()
                .fold_mutating(bool::map_from_fn(|_b_primary| 0), |mapbn_points, epi| {
                    mapbn_points[/*b_primary*/playerparties.is_primary_party(epi)] += rulestatecache.changing.mapepipointstichcount[epi].n_point;
                }),
            stichseq.completed_stichs_winner_index(rules)
                .fold_mutating(bool::map_from_fn(|_b_primary| 0), |mapbn_points, (stich, epi_winner)| {
                    mapbn_points[/*b_primary*/playerparties.is_primary_party(epi_winner)] += card_points::points_stich(stich);
                })
        );
        let internal_payouthints = |b_primary_party_wins| {
            perepi.per_epi_map(
                internal_payout(
                    /*n_payout_single_player*/ self.payoutparams.n_payout_base,
                    playerparties,
                    b_primary_party_wins,
                    perepi,
                ),
                |_epi, n_payout| {
                    assert_ne!(0, n_payout);
                    tpl_flip_if(0<n_payout, (None, Some(n_payout)))
                }
            )
        };
        if /*b_primary_party_wins*/ mapbn_points[/*b_primary*/true] >= self.pointstowin.points_to_win() {
            internal_payouthints(/*b_primary_party_wins*/true)
        } else if mapbn_points[/*b_primary*/false] > 120-self.pointstowin.points_to_win() {
            internal_payouthints(/*b_primary_party_wins*/false)
        } else {
            perepi.per_epi(|_epi| (None, None))
        }
    }
}

impl SLaufendeParams {
    pub fn payout_laufende<Rules: TRulesNoObj, PlayerParties: TPlayerParties>(&self, rulestatecache: &SRuleStateCache, gamefinishedstiche: SStichSequenceGameFinished, playerparties: &PlayerParties) -> isize {
        let ekurzlang = gamefinishedstiche.get().kurzlang();
        #[cfg(debug_assertions)] {
            let mut mapcardoepi = SCard::map_from_fn(|_card| None);
            for (epi, card) in gamefinishedstiche.get().completed_stichs().iter().flat_map(|stich| stich.iter()) {
                assert!(mapcardoepi[*card].is_none());
                mapcardoepi[*card] = Some(epi);
            }
            assert_eq!(mapcardoepi, rulestatecache.fixed.mapcardoepi);
            #[cfg(debug_assertions)]
            assert!(SCard::values(ekurzlang).all(|card| mapcardoepi[card].is_some()));
        }
        let laufende_relevant = |card: SCard| { // TODO should we make this part of SRuleStateCacheFixed?
            playerparties.is_primary_party(debug_verify!(rulestatecache.fixed.mapcardoepi[card]).unwrap())
        };
        let mut itcard_trumpf_descending = Rules::TrumpfDecider::trumpfs_in_descending_order();
        let b_might_have_lauf = laufende_relevant(debug_verify!(itcard_trumpf_descending.nth(0)).unwrap());
        let n_laufende = itcard_trumpf_descending
            .filter(|card| ekurzlang.supports_card(*card))
            .take_while(|card| b_might_have_lauf==laufende_relevant(*card))
            .count()
            + 1 // consumed by nth(0)
        ;
        (if n_laufende<self.n_lauf_lbound {0} else {n_laufende}).as_num::<isize>() * self.n_payout_per_lauf
    }
}

pub fn internal_payout(n_payout_single_player: isize, playerparties: &impl TPlayerParties, b_primary_party_wins: bool, perepi: impl TPerEPI) -> isize {
    perepi.per_epi(|epi| {
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

