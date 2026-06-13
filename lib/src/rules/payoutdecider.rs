use crate::primitives::*;
use crate::rules::{trumpfdecider::STrumpfDecider, *};
use crate::util::*;
use crate::ai::gametree::{TTplStrategies, SPerMinMaxStrategyGeneric};

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
pub fn equivalent_when_on_same_hand_point_based(slccard_ordered: &[ECard]) -> Vec<Vec<ECard>> {
    slccard_ordered.iter()
        .chunk_by(|card| card_points::points_card(**card)).into_iter()
        .map(|(_n_points, grpcard)| grpcard.copied().collect())
        .collect()
}

#[derive(Clone, Debug, new)]
pub struct SPayoutDeciderPointsAsPayout<PointsToWin> {
    pub pointstowin: PointsToWin,
}

pub fn pointstichcount_for_party(
    b_primary: bool,
    rulestatecache: &SRuleStateCache,
    playerparties: &impl TPlayerParties,
    if_dbg_else!({(rules, stichseq)}{_}): dbg_parameter!((&impl TRules, &SStichSequence)),
) -> SPointStichCount {
    let pointstichcount_primary = EPlayerIndex::values()
        .filter(|&epi| b_primary==playerparties.is_primary_party(epi))
        .map(|epi| &rulestatecache.changing.mapepipointstichcount[epi])
        .fold(
            SPointStichCount{n_stich: 0, n_point: 0},
            SPointStichCount::add,
        );
    #[cfg(debug_assertions)] {
        let itstich_primary = stichseq.completed_stichs_winner_index(rules)
            .filter(|&(_stich, epi_winner)| b_primary==playerparties.is_primary_party(epi_winner));
        assert_eq!(
            pointstichcount_primary.n_point,
            itstich_primary.clone()
                .map(|(stich, _epi_winner)| card_points::points_stich(stich))
                .sum::<isize>(),
        );
        assert_eq!(
            pointstichcount_primary.n_stich,
            itstich_primary.count(),
        );
    }
    pointstichcount_primary
}

fn payouthints_point_based(
    pointstowin: &impl TPointsToWin,
    if_dbg_else!({rules}{_}): dbg_parameter!(&impl TRules),
    rulestatecache: &SRuleStateCache,
    stichseq: &SStichSequence,
    playerparties: &impl TPlayerParties,
    fn_payout_one_player_if_premature_winner: impl FnOnce(&SPointStichCount)->isize,
) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
    let mapbpointstichcount = bool::map_from_fn(|b_primary|
        pointstichcount_for_party(
            b_primary,
            rulestatecache,
            playerparties,
            dbg_argument!((rules, stichseq)),
        )
    );
    let internal_payouthints = |pointstichcount_primary, b_premature_winner_is_primary_party: bool| {
        internal_payout(
            fn_payout_one_player_if_premature_winner(pointstichcount_primary).neg_if(!b_premature_winner_is_primary_party),
            playerparties,
        )
            .map(|n_payout| {
                 SInterval::from_tuple(tpl_flip_if(0<verify_ne!(*n_payout, 0), (None, Some(*n_payout))))
            })
    };
    if /*b_premature_winner_is_primary_party*/ mapbpointstichcount[/*b_primary*/true].n_point >= pointstowin.points_to_win() {
        internal_payouthints(
            /*minimum number of points that primary party can reach*/&mapbpointstichcount[/*b_primary*/true],
            /*b_premature_winner_is_primary_party*/true,
        )
    } else if mapbpointstichcount[/*b_primary*/false].n_point > 120-pointstowin.points_to_win() {
        // mapbpointstichcount[/*b_primary*/false].n_point > 120-pointstowin.points_to_win()
        // pointstowin.points_to_win() > 120-mapbpointstichcount[/*b_primary*/false].n_point
        // 120-mapbpointstichcount[/*b_primary*/false].n_point < pointstowin.points_to_win()
        internal_payouthints(
            /*maximum number of points that primary party can reach*/&(SPointStichCount{n_point: 120, n_stich: stichseq.kurzlang().cards_per_player()}-&mapbpointstichcount[/*b_primary*/false]),
            /*b_premature_winner_is_primary_party*/false,
        )
    } else {
        EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
    }
}

impl<
    PointsToWin: TPointsToWin,
> TPayoutDecider for SPayoutDeciderPointBased<PointsToWin> {
    fn payout(
        &self,
        if_dbg_else!({rules}{_}): dbg_parameter!(&impl TRules),
        trumpfdecider: &STrumpfDecider,
        rulestatecache: &SRuleStateCache,
        stichseq: SStichSequenceGameFinished,
        playerparties: &impl TPlayerParties,
    ) -> EnumMap<EPlayerIndex, isize> {
        let n_points_primary_party = pointstichcount_for_party(/*b_primary*/true, rulestatecache, playerparties, dbg_argument!((rules, stichseq.get()))).n_point;
        let b_primary_party_wins = n_points_primary_party >= self.pointstowin.points_to_win();
        internal_payout(
            (self.payoutparams.n_payout_base
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
            + self.payoutparams.laufendeparams.payout_laufende(trumpfdecider, rulestatecache, stichseq, playerparties)).neg_if(!b_primary_party_wins),
            playerparties,
        )
    }

    fn payouthints(
        &self,
        if_dbg_else!({rules}{_}): dbg_parameter!(&impl TRules),
        rulestatecache: &SRuleStateCache,
        (_ahand, stichseq): (&EnumMap<EPlayerIndex, SHand>, &SStichSequence),
        playerparties: &impl TPlayerParties,
    ) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        payouthints_point_based(
            &self.pointstowin,
            dbg_argument!(rules),
            rulestatecache,
            stichseq,
            playerparties,
            /*fn_payout_one_player_if_premature_winner*/|_pointstichcount_primary| {
                self.payoutparams.n_payout_base
            },
        )
    }
}

fn primary_pointstichcount_to_normalized(pointstichcount_primary: &SPointStichCount, pointstowin: &impl TPointsToWin) -> isize {
    let n_points_primary_party = pointstichcount_primary.n_point;
    let n_stichs_primary_party = pointstichcount_primary.n_stich;
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
    // 2*n_points_primary_party - 121
    // Then, to incorporate stichs, we multiply it by 9 (one more than stichs possible), and add the number of stichs:
    // (2*n_points_primary_party - 121) * 9 + n_stichs_primary_party
    assert!(n_stichs_primary_party < EKurzLang::max_cards_per_player() + 1);
    let n_normalized = (2*dbg!(n_points_primary_party) - 2*pointstowin.points_to_win() + 1) * (EKurzLang::max_cards_per_player().as_num::<isize>() + 1) + dbg!(n_stichs_primary_party).as_num::<isize>();
    debug_assert_eq!(
        &normalized_pointstichcount_to_primary(n_normalized, pointstowin),
        pointstichcount_primary
    );
    n_normalized
}

fn normalized_pointstichcount_to_primary(n_normalized: isize, pointstowin: &impl TPointsToWin) -> SPointStichCount {
    let n_max_cards_per_player_plus_1: isize = EKurzLang::max_cards_per_player().as_num::<isize>() + 1;
    let n_stich = n_normalized % n_max_cards_per_player_plus_1;
    let n_stich = if n_stich < 0 {
        (n_max_cards_per_player_plus_1 + n_stich).as_num::<usize>()
        //(-n_stich).as_num::<usize>()
    } else {
        n_stich.as_num::<usize>()
    };
    let n_point = unwrap!((((n_normalized - n_stich.as_num::<isize>()) / n_max_cards_per_player_plus_1) - 1 + 2*pointstowin.points_to_win()).div_exact_unstable_name_collision(2));
    SPointStichCount{
        n_point,
        n_stich,
    }
}

pub fn normalized_pointstichcount_to_pointstichcount(n_normalized: isize, pointstowin: &impl TPointsToWin, b_primary: bool, ekurzlang: EKurzLang) -> SPointStichCount {
    let pointstichcount_primary = normalized_pointstichcount_to_primary(
        if b_primary { n_normalized } else { -n_normalized },
        pointstowin
    );
    if b_primary {
        pointstichcount_primary
    } else {
        SPointStichCount {
            n_point: 120,
            n_stich: ekurzlang.cards_per_player(),
        } - pointstichcount_primary
    }
}

impl<
    PointsToWin: TPointsToWin,
> TPayoutDecider for SPayoutDeciderPointsAsPayout<PointsToWin> {
    fn payout(
        &self,
        if_dbg_else!({rules}{_}): dbg_parameter!(&impl TRules),
        _trumpfdecider: &STrumpfDecider,
        rulestatecache: &SRuleStateCache,
        if_dbg_else!({stichseq}{_}): SStichSequenceGameFinished,
        playerparties: &impl TPlayerParties,
    ) -> EnumMap<EPlayerIndex, isize> {
        internal_payout(
            primary_pointstichcount_to_normalized(
                &pointstichcount_for_party(/*b_primary*/true, rulestatecache, playerparties, dbg_argument!((rules, stichseq.get()))),
                &self.pointstowin
            ),
            playerparties,
        )
    }

    fn payouthints(
        &self,
        if_dbg_else!({rules}{_}): dbg_parameter!(&impl TRules),
        rulestatecache: &SRuleStateCache,
        (_ahand, stichseq): (&EnumMap<EPlayerIndex, SHand>, &SStichSequence),
        playerparties: &impl TPlayerParties,
    ) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        payouthints_point_based(
            &self.pointstowin,
            dbg_argument!(rules),
            rulestatecache,
            stichseq,
            playerparties,
            /*fn_payout_one_player_if_premature_winner*/|pointstichcount_primary| {
                primary_pointstichcount_to_normalized(pointstichcount_primary, &self.pointstowin).abs()
            },
        )
    }
}

impl SLaufendeParams {
    pub fn payout_laufende<PlayerParties: TPlayerParties>(&self, trumpfdecider: &STrumpfDecider, rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, playerparties: &PlayerParties) -> isize {
        debug_assert_eq!(
            SRuleStateCacheFixed::new(
                /*ahand*/&EPlayerIndex::map_from_fn(|_epi| SHand::new_from_vec(SHandVector::new())),
                stichseq.get(),
            ),
            rulestatecache.fixed,
        );
        let n_laufende = trumpfdecider.count_laufende(
            stichseq.get().kurzlang(),
            playerparties,
            /*fn_who_has_card*/|card| rulestatecache.fixed.who_has_card(card),
        ).n_laufende;
        (if n_laufende<self.n_lauf_lbound {0} else {n_laufende}).as_num::<isize>() * self.n_payout_per_lauf
    }
}

pub fn internal_payout(n_payout_primary_unmultiplied: isize, playerparties: &impl TPlayerParties) -> EnumMap<EPlayerIndex, isize> {
    EPlayerIndex::map_from_fn(|epi| {
        n_payout_primary_unmultiplied.neg_if(!playerparties.is_primary_party(epi))
        * playerparties.multiplier(epi)
    })
}

pub trait TPayoutDecider : Sync + Send + 'static + Clone + fmt::Debug {
    fn payout(
        &self,
        if_dbg_else!({rules}{_}): dbg_parameter!(&impl TRules),
        trumpfdecider: &STrumpfDecider,
        rulestatecache: &SRuleStateCache,
        stichseq: SStichSequenceGameFinished,
        playerparties: &impl TPlayerParties,
    ) -> EnumMap<EPlayerIndex, isize>;

    fn payouthints(
        &self,
        if_dbg_else!({rules}{_}): dbg_parameter!(&impl TRules),
        rulestatecache: &SRuleStateCache,
        tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence),
        playerparties: &impl TPlayerParties,
    ) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>;
}

pub fn snapshot_cache_points_monotonic<TplStrategies: TTplStrategies>(playerparties: impl TPlayerParties + 'static, pointstowin: impl TPointsToWin) -> Box<dyn TSnapshotCache<SPerMinMaxStrategyRawPayout<TplStrategies>>> {
    type SSnapshotEquivalenceClass = u64; // space-saving variant of this:
    // struct SSnapshotEquivalenceClass { // packed into SSnapshotEquivalenceClass TODO? use bitfield crate
    //     epi_next_stich: EPlayerIndex,
    //     setcard_played: EnumSet<ECard>,
    // }
    #[derive(Debug)]
    struct SSnapshotCachePointsMonotonic<TplStrategies: TTplStrategies, PlayerParties, PointsToWin> {
        mapsnapequivperminmaxn_payout: HashMap<SSnapshotEquivalenceClass, SPerMinMaxStrategyGeneric<SPointStichCount, TplStrategies>>,
        playerparties: PlayerParties,
        pointstowin: PointsToWin,
    }
    impl<TplStrategies: TTplStrategies, PlayerParties: TPlayerParties, PointsToWin: TPointsToWin> TSnapshotCache<SPerMinMaxStrategyRawPayout<TplStrategies>> for SSnapshotCachePointsMonotonic<TplStrategies, PlayerParties, PointsToWin> {
        fn get(&self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache, if_dbg_else!({rules}{_}): dbg_parameter!(&SRules)) -> Option<SPerMinMaxStrategyRawPayout<TplStrategies>> {
            debug_assert_eq!(stichseq.current_stich().size(), 0);
            let perminmaxn_payout = self.mapsnapequivperminmaxn_payout
                .get(&super::snap_equiv_base(stichseq))?;
            Some(perminmaxn_payout.map(|pointstichcount| {
                let pointstichcount_primary = pointstichcount
                    + pointstichcount_for_party(
                        /*b_primary*/true,
                        rulestatecache,
                        &self.playerparties,
                        dbg_argument!((rules, stichseq)),
                    );
                payoutdecider::internal_payout(
                    primary_pointstichcount_to_normalized(&pointstichcount_primary, &self.pointstowin),
                    &self.playerparties,
                )
            }))
        }
        fn put(&mut self, stichseq: &SStichSequence, rulestatecache: &SRuleStateCache, payoutstats: &SPerMinMaxStrategyRawPayout<TplStrategies>, if_dbg_else!({rules}{_}): dbg_parameter!(&SRules)) {
            debug_assert_eq!(stichseq.current_stich().size(), 0);
            let perminmaxn_payout = payoutstats.map(|mapepin_payout| {
                payoutdecider::normalized_pointstichcount_to_pointstichcount(
                    unwrap!(
                        unwrap!(self.playerparties.primary_players().map(|epi| mapepin_payout[epi]).all_equal_value())
                            .div_exact_unstable_name_collision(
                                /*n_multiplier_primary*/ unwrap!(self.playerparties.primary_players().map(|epi| self.playerparties.multiplier(epi)).all_equal_value())
                            )
                    ),
                    &self.pointstowin,
                    /*b_primary*/true,
                    stichseq.kurzlang(),
                ) - pointstichcount_for_party(
                    /*b_primary*/true,
                    rulestatecache,
                    &self.playerparties,
                    dbg_argument!((rules, stichseq))
                )
            });
            self.mapsnapequivperminmaxn_payout
                .insert(
                    super::snap_equiv_base(stichseq),
                    perminmaxn_payout,
                );
            debug_assert_eq!(self.get(stichseq, rulestatecache, dbg_argument!(rules)).as_ref(), Some(payoutstats));
        }
        fn continue_with_cache(&self, stichseq: &SStichSequence) -> bool {
            stichseq.completed_stichs().len()<=5
        }
    }
    Box::new(
        SSnapshotCachePointsMonotonic::<TplStrategies, _, _>{
            mapsnapequivperminmaxn_payout: Default::default(),
            playerparties,
            pointstowin,
        }
    )
}
