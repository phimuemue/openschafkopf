use crate::primitives::*;
use crate::rules::{trumpfdecider::TTrumpfDecider, *};
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


pub trait TPointsToWin : Sync + Send + 'static + Clone + fmt::Debug {
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

// TODO this should probably be a method on the pointbased TPayoutDecider impementations
pub fn equivalent_when_on_same_hand_point_based(slccard_ordered: &[SCard]) -> Vec<Vec<SCard>> {
    slccard_ordered.iter()
        .group_by(|card| card_points::points_card(**card)).into_iter()
        .map(|(_n_points, grpcard)| grpcard.copied().collect())
        .collect()
}

#[derive(Clone, Debug, new)]
pub struct SPayoutDeciderPointsAsPayout<PointsToWin> {
    pub pointstowin: PointsToWin,
}

fn payout_point_based (
    pointstowin: &impl TPointsToWin,
    if_dbg_else!({rules}{_rules}): &impl TRulesNoObj,
    rulestatecache: &SRuleStateCache,
    if_dbg_else!({stichseq}{_stichseq}): SStichSequenceGameFinished,
    playerparties: &impl TPlayerParties,
    fn_payout_one_player: impl FnOnce(isize, bool)->isize,
) -> EnumMap<EPlayerIndex, isize> {
    let n_points_primary_party = debug_verify_eq!(
        EPlayerIndex::values()
            .filter(|epi| playerparties.is_primary_party(*epi))
            .map(|epi| rulestatecache.changing.mapepipointstichcount[epi].n_point)
            .sum::<isize>(),
        stichseq.get().completed_stichs_winner_index(rules)
            .filter(|&(_stich, epi_winner)| playerparties.is_primary_party(epi_winner))
            .map(|(stich, _epi_winner)| card_points::points_stich(stich))
            .sum::<isize>()
    );
    let b_primary_party_wins = n_points_primary_party >= pointstowin.points_to_win();
    internal_payout(
        fn_payout_one_player(n_points_primary_party, b_primary_party_wins),
        playerparties,
        b_primary_party_wins,
    )
}

fn payouthints_point_based(
    pointstowin: &impl TPointsToWin,
    if_dbg_else!({rules}{_rules}): &impl TRulesNoObj,
    rulestatecache: &SRuleStateCache,
    if_dbg_else!({stichseq}{_stichseq}): &SStichSequence,
    _ahand: &EnumMap<EPlayerIndex, SHand>,
    playerparties: &impl TPlayerParties,
    fn_payout_one_player_if_premature_winner: impl FnOnce(isize)->isize,
) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
    let mapbn_points = debug_verify_eq!(
        EPlayerIndex::values()
            .fold(bool::map_from_fn(|_b_primary| 0), mutate_return!(|mapbn_points, epi| {
                mapbn_points[/*b_primary*/playerparties.is_primary_party(epi)] += rulestatecache.changing.mapepipointstichcount[epi].n_point;
            })),
        stichseq.completed_stichs_winner_index(rules)
            .fold(bool::map_from_fn(|_b_primary| 0), mutate_return!(|mapbn_points, (stich, epi_winner)| {
                mapbn_points[/*b_primary*/playerparties.is_primary_party(epi_winner)] += card_points::points_stich(stich);
            }))
    );
    let internal_payouthints = |n_points_primary_party, b_premature_winner_is_primary_party| {
        internal_payout(
            fn_payout_one_player_if_premature_winner(n_points_primary_party),
            playerparties,
            b_premature_winner_is_primary_party,
        )
            .map(|n_payout| {
                 SInterval::from_tuple(tpl_flip_if(0<verify_ne!(*n_payout, 0), (None, Some(*n_payout))))
            })
    };
    if /*b_premature_winner_is_primary_party*/ mapbn_points[/*b_primary*/true] >= pointstowin.points_to_win() {
        internal_payouthints(
            /*minimum number of points that primary party can reach*/mapbn_points[/*b_primary*/true],
            /*b_premature_winner_is_primary_party*/true,
        )
    } else if mapbn_points[/*b_primary*/false] > 120-pointstowin.points_to_win() {
        // mapbn_points[/*b_primary*/false] > 120-pointstowin.points_to_win()
        // pointstowin.points_to_win() > 120-mapbn_points[/*b_primary*/false]
        // 120-mapbn_points[/*b_primary*/false] < pointstowin.points_to_win()
        internal_payouthints(
            /*maximum number of points that primary party can reach*/120-mapbn_points[/*b_primary*/false],
            /*b_premature_winner_is_primary_party*/false,
        )
    } else {
        EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
    }
}

impl<
    PointsToWin: TPointsToWin,
    PlayerParties: TPlayerParties,
> TPayoutDecider<PlayerParties> for SPayoutDeciderPointBased<PointsToWin> {
    fn payout<Rules>(
        &self,
        rules: &Rules,
        rulestatecache: &SRuleStateCache,
        stichseq: SStichSequenceGameFinished,
        playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, isize>
        where 
            Rules: TRulesNoObj,
    {
        payout_point_based(
            &self.pointstowin,
            rules,
            rulestatecache,
            stichseq,
            playerparties,
            /*fn_payout_one_player*/|n_points_primary_party, b_primary_party_wins| {
                self.payoutparams.n_payout_base
                + { 
                    if debug_verify_eq!(
                        EPlayerIndex::values()
                            .filter(|epi| b_primary_party_wins==playerparties.is_primary_party(*epi))
                            .map(|epi|  rulestatecache.changing.mapepipointstichcount[epi].n_stich)
                            .sum::<usize>()==stichseq.get().kurzlang().cards_per_player(),
                        stichseq.get().completed_stichs_winner_index(rules)
                            .all(|(_stich, epi_winner)| b_primary_party_wins==playerparties.is_primary_party(epi_winner))
                    ) {
                        2*self.payoutparams.n_payout_schneider_schwarz // schwarz
                    } else if (b_primary_party_wins && n_points_primary_party>90) || (!b_primary_party_wins && n_points_primary_party<=30) {
                        self.payoutparams.n_payout_schneider_schwarz // schneider
                    } else {
                        0 // "nothing", i.e. neither schneider nor schwarz
                    }
                }
                + self.payoutparams.laufendeparams.payout_laufende(rules, rulestatecache, stichseq, playerparties)
            },
        )
    }

    fn payouthints<Rules: TRulesNoObj>(
        &self,
        rules: &Rules,
        rulestatecache: &SRuleStateCache,
        stichseq: &SStichSequence,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        payouthints_point_based(
            &self.pointstowin,
            rules,
            rulestatecache,
            stichseq,
            ahand,
            playerparties,
            /*fn_payout_one_player_if_premature_winner*/|_n_points_primary_party| {
                self.payoutparams.n_payout_base
            },
        )
    }
}

fn primary_points_to_normalized_points(n_points_primary_party: isize, pointstowin: &impl TPointsToWin) -> isize {
    // General idea: Just use the n_points_primary_party as payout.
    // => Problem: By convention a lost game is determined by a negative payout,
    //    i.e. we cannot simply use n_points_primary_party as payout 
    //    (with 0 <= n_points_primary_party <= 60 the game is actually lost).
    // To resolve this, we could subtract 60.5 from n_points_primary_party:
    // * n_points_primary_party - 60.5 > 0: game won
    // * n_points_primary_party - 60.5 < 0: game lost
    // * n_points_primary_party - 60.5 == 0: impossible as n_points_primary_party is integral
    // => Problem: would use f32/f64, but we generally use isize for points and payouts.
    // To resolve this, we can multiply the above equations with 2 to get "normalized points":
    // n_points_normalized = 2*n_points_primary_party - 121
    let n_points_normalized = 2*n_points_primary_party - 2*pointstowin.points_to_win() + 1;
    debug_assert_eq!(
        normalized_points_to_primary_points(n_points_normalized.as_num::<f32>(), pointstowin).as_num::<isize>(),
        n_points_primary_party
    );
    n_points_normalized
}

fn normalized_points_to_primary_points(f_points_normalized: f32, pointstowin: &impl TPointsToWin) -> f32 {
    (f_points_normalized - 1. + 2.*pointstowin.points_to_win().as_num::<f32>()) / 2.
}

pub fn normalized_points_to_points(f_points_normalized: f32, pointstowin: &impl TPointsToWin, b_primary: bool) -> f32 {
    let f_primary_points = normalized_points_to_primary_points(
        if b_primary { f_points_normalized } else { -f_points_normalized },
        pointstowin
    );
    if b_primary {
        f_primary_points
    } else {
        120. - f_primary_points
    }
}

impl<
    PointsToWin: TPointsToWin,
    PlayerParties: TPlayerParties,
> TPayoutDecider<PlayerParties> for SPayoutDeciderPointsAsPayout<PointsToWin> {
    fn payout<Rules>(
        &self,
        rules: &Rules,
        rulestatecache: &SRuleStateCache,
        stichseq: SStichSequenceGameFinished,
        playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, isize>
        where 
            Rules: TRulesNoObj,
    {
        payout_point_based(
            &self.pointstowin,
            rules,
            rulestatecache,
            stichseq,
            playerparties,
            /*fn_payout_one_player*/|n_points_primary_party, _b_primary_party_wins| {
                primary_points_to_normalized_points(n_points_primary_party, &self.pointstowin).abs()
            },
        )
    }

    fn payouthints<Rules: TRulesNoObj>(
        &self,
        rules: &Rules,
        rulestatecache: &SRuleStateCache,
        stichseq: &SStichSequence,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        payouthints_point_based(
            &self.pointstowin,
            rules,
            rulestatecache,
            stichseq,
            ahand,
            playerparties,
            /*fn_payout_one_player_if_premature_winner*/|n_points_primary_party| {
                primary_points_to_normalized_points(n_points_primary_party, &self.pointstowin).abs()
            },
        )
    }
}

impl SLaufendeParams {
    pub fn payout_laufende<PlayerParties: TPlayerParties>(&self, rules: &impl TRulesNoObj, rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, playerparties: &PlayerParties) -> isize {
        let ekurzlang = stichseq.get().kurzlang();
        debug_assert_eq!(
            SRuleStateCacheFixed::new(
                stichseq.get(),
                /*ahand*/&EPlayerIndex::map_from_fn(|_epi| SHand::new_from_vec(SHandVector::new())),
            ),
            rulestatecache.fixed,
        );
        let laufende_relevant = |card: SCard| { // TODO should we make this part of SRuleStateCacheFixed?
            playerparties.is_primary_party(rulestatecache.fixed.who_has_card(card))
        };
        let mut itcard_trumpf_descending = rules.trumpfdecider().trumpfs_in_descending_order();
        let b_might_have_lauf = laufende_relevant(unwrap!(itcard_trumpf_descending.next()));
        let n_laufende = itcard_trumpf_descending
            .filter(|card| ekurzlang.supports_card(*card))
            .take_while(|card| b_might_have_lauf==laufende_relevant(*card))
            .count()
            + 1 // consumed by next()
        ;
        (if n_laufende<self.n_lauf_lbound {0} else {n_laufende}).as_num::<isize>() * self.n_payout_per_lauf
    }
}

pub fn internal_payout(n_payout_single_player: isize, playerparties: &impl TPlayerParties, b_primary_party_wins: bool) -> EnumMap<EPlayerIndex, isize> {
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

pub trait TPayoutDecider<PlayerParties> : Sync + Send + 'static + Clone + fmt::Debug {
    fn payout<Rules>(
        &self,
        rules: &Rules,
        rulestatecache: &SRuleStateCache,
        stichseq: SStichSequenceGameFinished,
        playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, isize>
        where Rules: TRulesNoObj;

    fn payouthints<Rules>(
        &self,
        rules: &Rules,
        rulestatecache: &SRuleStateCache,
        stichseq: &SStichSequence,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>
        where Rules: TRulesNoObj;
}
