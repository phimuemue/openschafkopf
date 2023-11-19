use crate::primitives::*;
use crate::rules::{payoutdecider::*, trumpfdecider::*, *};
use crate::util::*;
use std::{cmp::Ordering, fmt};

pub trait TPayoutDeciderSoloLike : Sync + 'static + Clone + fmt::Debug + Send {
    fn priority(&self) -> VGameAnnouncementPriority;
    fn with_increased_prio(&self, _prio: &VGameAnnouncementPriority, _ebid: EBid) -> Option<Self> {
        None
    }
    fn priorityinfo(&self) -> String {
        "".to_string()
    }
    fn payout(&self, rules: &SRulesSoloLike<Self>, rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, isize>;
    fn payouthints(&self, rules: &SRulesSoloLike<Self>, rulestatecache: &SRuleStateCache, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>;
    fn equivalent_when_on_same_hand(slccard_ordered: &[ECard]) -> Vec<Vec<ECard>>;

    fn points_as_payout(&self, _rules: &SRulesSoloLike<Self>) -> Option<(
        SRules,
        Box<dyn Fn(&SStichSequence, (EPlayerIndex, &SHand), f32)->f32 + Sync>,
    )> {
        None
    }

    fn snapshot_cache<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded+Clone>(&self, rules: &SRulesSoloLike<Self>) -> Box<dyn TSnapshotCache<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>>>
        where
            MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: PartialEq+fmt::Debug+Clone,
    ;
}

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


impl TPointsToWin for VGameAnnouncementPrioritySoloLike {
    fn points_to_win(&self) -> isize {
        match self {
            Self::SoloSimple(_) => 61,
            Self::SoloSteigern{n_points_to_win, n_step: _n_step} => *n_points_to_win,
        }
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike> {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloLike(self.pointstowin.clone())
    }

    fn with_increased_prio(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Self> {
        use self::VGameAnnouncementPriority::*;
        use self::VGameAnnouncementPrioritySoloLike::*;
        if let (SoloLike(SoloSteigern{..}), &SoloLike(SoloSteigern{mut n_points_to_win, n_step})) = (self.priority(), prio) {
            n_points_to_win += match ebid {
                EBid::AtLeast => 0,
                EBid::Higher => n_step,
            };
            if n_points_to_win<=120 {
                return Some(Self{pointstowin:SoloSteigern{n_points_to_win, n_step}, ..self.clone()})
            }
        }
        None
    }

    fn priorityinfo(&self) -> String {
        use self::VGameAnnouncementPrioritySoloLike::*;
        match self.pointstowin {
            SoloSimple(_) => "".to_string(), // no special indication required
            SoloSteigern{n_points_to_win, ..} => {
                assert!(61<=n_points_to_win);
                if n_points_to_win<61 {
                    format!("for {}", n_points_to_win)
                } else {
                    "".to_string()
                }
            },
        }
    }

    fn payout(&self, rules: &SRulesSoloLike<Self>, rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, isize> {
        TPayoutDecider::payout(self,
            dbg_argument!(rules),
            &rules.trumpfdecider,
            rulestatecache,
            stichseq,
            &SPlayerParties13::new(rules.epi),
        ).map(|n_payout| n_payout * expensifiers.stoss_doubling_factor())
    }

    fn payouthints(&self, rules: &SRulesSoloLike<Self>, rulestatecache: &SRuleStateCache, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        TPayoutDecider::payouthints(self,
            dbg_argument!(rules),
            rulestatecache,
            tplahandstichseq,
            &SPlayerParties13::new(rules.epi),
        ).map(|intvlon_payout| intvlon_payout.map(|on_payout|
             on_payout.map(|n_payout| n_payout * expensifiers.stoss_doubling_factor()),
        ))
    }

    fn equivalent_when_on_same_hand(slccard_ordered: &[ECard]) -> Vec<Vec<ECard>> {
        equivalent_when_on_same_hand_point_based(slccard_ordered)
    }

    fn points_as_payout(&self, rules: &SRulesSoloLike<Self>) -> Option<(
        SRules,
        Box<dyn Fn(&SStichSequence, (EPlayerIndex, &SHand), f32)->f32 + Sync>,
    )> {
        //assert_eq!(self, rules.payoutdecider); // TODO
        let pointstowin = self.pointstowin.clone();
        let epi_active = rules.epi;
        Some((
            SActivelyPlayableRules::from(SRulesSoloLike{
                str_name: rules.str_name.clone(),
                epi: rules.epi,
                payoutdecider: SPayoutDeciderPointsAsPayout{
                    pointstowin: pointstowin.clone(),
                },
                trumpfdecider: rules.trumpfdecider.clone(),
                of_heuristic_active_occurence_probability: rules.of_heuristic_active_occurence_probability,
                stossparams: rules.stossparams.clone(),
            }).into(),
            Box::new(move |stichseq: &SStichSequence, (epi_hand, hand): (EPlayerIndex, &SHand), f_payout: f32| {
                assert!(stichseq.remaining_cards_per_hand()[epi_hand]==hand.cards().len());
                SPayoutDeciderPointsAsPayout::payout_to_points(
                    epi_active,
                    epi_hand,
                    &pointstowin,
                    f_payout,
                )
            }) as Box<dyn Fn(&SStichSequence, (EPlayerIndex, &SHand), f32)->f32 + Sync>,
        )
    )}

    fn snapshot_cache<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded+Clone>(&self, rules: &SRulesSoloLike<Self>) -> Box<dyn TSnapshotCache<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>>>
        where
            MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: PartialEq+fmt::Debug+Clone,
    {
        super::snapshot_cache_point_based::<MinMaxStrategiesHK, _>(SPlayerParties13::new(rules.epi))
    }
}

impl SPayoutDeciderPointsAsPayout<VGameAnnouncementPrioritySoloLike> {
    fn payout_to_points(epi_active: EPlayerIndex, epi_hand: EPlayerIndex, pointstowin: &impl TPointsToWin, f_payout: f32) -> f32 {
        normalized_points_to_points(
            f_payout / SPlayerParties13::new(epi_active).multiplier(epi_hand).as_num::<f32>(),
            pointstowin,
            /*b_primary*/ epi_hand==epi_active,
        )
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderPointsAsPayout<VGameAnnouncementPrioritySoloLike> {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloLike(self.pointstowin.clone())
    }

    fn payout(&self, rules: &SRulesSoloLike<Self>, rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, _expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, isize> {
        let an_payout = TPayoutDecider::payout(self,
            dbg_argument!(rules),
            &rules.trumpfdecider,
            rulestatecache,
            stichseq,
            &SPlayerParties13::new(rules.epi),
        );
        #[cfg(debug_assertions)] {
            let mut stichseq_check = SStichSequence::new(stichseq.get().kurzlang());
            let mut ahand_check = EPlayerIndex::map_from_fn(|epi|
                SHand::new_from_iter(stichseq.get().completed_cards_by(epi))
            );
            let playerparties = SPlayerParties13::new(rules.epi);
            for (epi_card, card) in stichseq.get().completed_cards() {
                let b_primary = playerparties.is_primary_party(epi_card);
                assert_eq!(
                    Self::payout_to_points(
                        /*epi_active*/rules.epi,
                        /*epi_hand*/epi_card,
                        &self.pointstowin,
                        an_payout[epi_card].as_num::<f32>(),
                    ).as_num::<isize>(),
                    EPlayerIndex::values()
                        .filter(|epi| playerparties.is_primary_party(*epi)==b_primary)
                        .map(|epi|
                            rulestatecache.changing.mapepipointstichcount[epi].n_point
                        )
                        .sum::<isize>(),
                );
                stichseq_check.zugeben(*card, rules);
                ahand_check[epi_card].play_card(*card);
            }

        }
        an_payout
    }

    fn payouthints(&self, rules: &SRulesSoloLike<Self>, rulestatecache: &SRuleStateCache, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), _expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        TPayoutDecider::payouthints(self,
            dbg_argument!(rules),
            rulestatecache,
            tplahandstichseq,
            &SPlayerParties13::new(rules.epi),
        )
    }

    fn equivalent_when_on_same_hand(slccard_ordered: &[ECard]) -> Vec<Vec<ECard>> {
        equivalent_when_on_same_hand_point_based(slccard_ordered)
    }

    fn snapshot_cache<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded+Clone>(&self, rules: &SRulesSoloLike<Self>) -> Box<dyn TSnapshotCache<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>>>
        where
            MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: PartialEq+fmt::Debug+Clone,
    {
        payoutdecider::snapshot_cache_points_monotonic::<MinMaxStrategiesHK>(
            SPlayerParties13::new(rules.epi),
            self.pointstowin.clone(),
        )
    }
}

#[derive(Clone, Debug, new)]
pub struct SPayoutDeciderTout {
    payoutparams : SPayoutDeciderParams,
    i_prio: isize,
}

impl TPayoutDeciderSoloLike for SPayoutDeciderTout {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloTout(self.i_prio)
    }

    fn payout(&self, rules: &SRulesSoloLike<Self>, rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, isize> {
        let playerparties13 = &SPlayerParties13::new(rules.epi);
        // TODORULES optionally count schneider/schwarz
        internal_payout(
            /*n_payout_primary_unmultiplied*/((self.payoutparams.n_payout_base + self.payoutparams.laufendeparams.payout_laufende(&rules.trumpfdecider, rulestatecache, stichseq, playerparties13)) * 2)
                .neg_if(!/*b_primary_party_wins*/debug_verify_eq!(
                    rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich==stichseq.get().kurzlang().cards_per_player(),
                    stichseq.get().completed_stichs_winner_index(rules)
                        .all(|(_stich, epi_winner)| playerparties13.is_primary_party(epi_winner))
                )),
            playerparties13,
        ).map(|n_payout| n_payout * expensifiers.stoss_doubling_factor())
    }

    fn payouthints(&self, rules: &SRulesSoloLike<Self>, rulestatecache: &SRuleStateCache, (_ahand, stichseq): (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        let playerparties13 = &SPlayerParties13::new(rules.epi);
        if debug_verify_eq!(
            rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich < stichseq.completed_stichs().len(),
            !stichseq.completed_stichs_winner_index(rules)
                .all(|(_stich, epi_winner)| playerparties13.is_primary_party(epi_winner))
        ) {
            internal_payout(
                /*n_payout_primary_unmultiplied*/ -(self.payoutparams.n_payout_base) * 2, // TODO laufende
                playerparties13,
            )
                .map(|n_payout| {
                     SInterval::from_tuple(tpl_flip_if(0<verify_ne!(*n_payout, 0), (None, Some(*n_payout * expensifiers.stoss_doubling_factor()))))
                })
        } else {
            EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
        }
    }

    fn equivalent_when_on_same_hand(slccard_ordered: &[ECard]) -> Vec<Vec<ECard>> {
        vec![slccard_ordered.to_vec()] // In Tout, neighboring cards are equivalent regardless of points_card.
    }

    fn snapshot_cache<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded+Clone>(&self, rules: &SRulesSoloLike<Self>) -> Box<dyn TSnapshotCache<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>>>
        where
            MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: PartialEq+fmt::Debug+Clone,
    {
        super::snapshot_cache_point_based::<MinMaxStrategiesHK, _>(SPlayerParties13::new(rules.epi))
    }
}

#[derive(Clone, Debug, new)]
pub struct SPayoutDeciderSie {
    payoutparams : SPayoutDeciderParams,
}

fn cards_valid_for_sie<ItCard: Clone+Iterator<Item=ECard>>(
    rules: &SRulesSoloLike<SPayoutDeciderSie>,
    mut itcard: ItCard,
    ekurzlang: EKurzLang,
) -> bool {
    let n_cards_per_player = ekurzlang.cards_per_player();
    assert_eq!(itcard.clone().count(), n_cards_per_player);
    let veccard_trumpf_relevant = rules.trumpfdecider
        .trumpfs_in_descending_order()
        .take(n_cards_per_player)
        .collect::<SHandVector>();
    itcard.all(|card| veccard_trumpf_relevant.contains(&card))
}

impl TPayoutDeciderSoloLike for SPayoutDeciderSie {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloSie
    }

    fn payout(&self, rules: &SRulesSoloLike<Self>, _rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, isize> {
        let playerparties13 = &SPlayerParties13::new(rules.epi);
        // TODORULES optionally count schneider/schwarz
        internal_payout(
            /*n_payout_primary_unmultiplied*/( (self.payoutparams.n_payout_base
            + {
                stichseq.get().kurzlang().cards_per_player().as_num::<isize>()
            } * self.payoutparams.laufendeparams.n_payout_per_lauf) * 4)
                .neg_if(!/*b_primary_party_wins*/cards_valid_for_sie(
                    rules,
                    stichseq.get().completed_cards_by(playerparties13.primary_player()),
                    stichseq.get().kurzlang(),
                )),
            playerparties13,
        ).map(|n_payout| n_payout * expensifiers.stoss_doubling_factor())
    }

    fn payouthints(&self, rules: &SRulesSoloLike<Self>, _rulestatecache: &SRuleStateCache, (ahand, stichseq): (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        if !cards_valid_for_sie(
            rules,
            stichseq.cards_from_player(&ahand[rules.epi], rules.epi),
            stichseq.kurzlang(),
        ) {
            internal_payout(
                /*n_payout_primary_unmultiplied*/-self.payoutparams.n_payout_base * 4,
                &SPlayerParties13::new(rules.epi),
            )
                .map(|n_payout| {
                     SInterval::from_tuple(tpl_flip_if(0<verify_ne!(*n_payout, 0), (None, Some(*n_payout * expensifiers.stoss_doubling_factor()))))
                })
        } else {
            EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
        }
    }

    fn equivalent_when_on_same_hand(slccard_ordered: &[ECard]) -> Vec<Vec<ECard>> {
        vec![slccard_ordered.to_vec()] // In Sie, neighboring cards are equivalent regardless of points_card.
    }

    fn snapshot_cache<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded+Clone>(&self, rules: &SRulesSoloLike<Self>) -> Box<dyn TSnapshotCache<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>>>
        where
            MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: PartialEq+fmt::Debug+Clone,
    {
        super::snapshot_cache_point_based::<MinMaxStrategiesHK, _>(SPlayerParties13::new(rules.epi))
    }
}

#[derive(Clone, Debug)]
pub struct SRulesSoloLike<PayoutDecider> {
    pub str_name: String,
    epi: EPlayerIndex,
    payoutdecider: PayoutDecider,
    trumpfdecider: STrumpfDecider,
    of_heuristic_active_occurence_probability: Option<f64>,
    stossparams: SStossParams,
}

impl<PayoutDecider: TPayoutDeciderSoloLike> fmt::Display for SRulesSoloLike<PayoutDecider> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.str_name, self.payoutdecider.priorityinfo())
    }
}

impl<PayoutDecider: TPayoutDeciderSoloLike> TActivelyPlayableRules for SRulesSoloLike<PayoutDecider> 
    where
        Self: Into<SActivelyPlayableRules>,
{
    fn priority(&self) -> VGameAnnouncementPriority {
        self.payoutdecider.priority()
    }
    fn with_increased_prio(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<SActivelyPlayableRules> {
        self.payoutdecider.with_increased_prio(prio, ebid)
            .map(|payoutdecider| Self{
                payoutdecider,
                trumpfdecider: self.trumpfdecider.clone(),
                epi: self.epi,
                str_name: self.str_name.clone(),
                of_heuristic_active_occurence_probability: None, // No data about occurence probabilities with increased priority.
                stossparams: self.stossparams.clone(),
            }.into())
    }
}

impl<PayoutDecider: TPayoutDeciderSoloLike> TRules for SRulesSoloLike<PayoutDecider> {
    impl_rules_trumpf!();
    impl_single_play!();

    fn payout_no_invariant(&self, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize> {
        self.payoutdecider.payout(
            self,
            rulestatecache,
            stichseq,
            expensifiers,
        )
    }

    fn payouthints(&self, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        self.payoutdecider.payouthints(
            self,
            rulestatecache,
            tplahandstichseq,
            expensifiers,
        )
    }

    fn equivalent_when_on_same_hand(&self) -> SCardsPartition {
        SCardsPartition::new_from_slices(
            &self.trumpfdecider.equivalent_when_on_same_hand().into_raw().into_iter()
                .flat_map(|veccard| PayoutDecider::equivalent_when_on_same_hand(&veccard))
                .collect::<Vec<_>>()
                .iter()
                .map(|veccard| veccard as &[ECard]).collect::<Vec<_>>(),
        )
    }

    fn only_minmax_points_when_on_same_hand(&self, _rulestatecache: &SRuleStateCacheFixed) -> Option<(SCardsPartition, SPlayerPartiesTable)> {
        // TODO this is ok for normal Solo, point based Solo, Tout, Sie. But we can possibly improve this for e.g. Tout/Sie.
        Some((
            SCardsPartition::new_from_slices(
                &self.trumpfdecider.equivalent_when_on_same_hand().into_raw().iter()
                    .map(|vec| -> &[_] { vec })
                    .collect::<Vec<_>>(),
            ),
            SPlayerParties13::new(self.epi).into(),
        ))
    }

    fn points_as_payout(&self) -> Option<(
        SRules,
        Box<dyn Fn(&SStichSequence, (EPlayerIndex, &SHand), f32)->f32 + Sync>,
    )> {
        self.payoutdecider.points_as_payout(self)
    }

    fn snapshot_cache<MinMaxStrategiesHK: TMinMaxStrategiesHigherKinded+Clone>(&self, _rulestatecachefixed: &SRuleStateCacheFixed) -> Box<dyn TSnapshotCache<MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>>>
        where
            MinMaxStrategiesHK::Type<EnumMap<EPlayerIndex, isize>>: PartialEq+fmt::Debug+Clone,
    {
        self.payoutdecider.snapshot_cache::<MinMaxStrategiesHK>(self)
    }

    fn heuristic_active_occurence_probability(&self) -> Option<f64> {
        self.of_heuristic_active_occurence_probability
    }
}

plain_enum_mod!(modesololike, ESoloLike {
    Solo,
    Wenz,
    Geier,
});

type_dispatch_enum!(pub enum VPayoutDeciderSoloLike {
    PointBased(SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike>),
    Tout(SPayoutDeciderTout),
    Sie(SPayoutDeciderSie),
});

pub fn sololike(
    epi: EPlayerIndex,
    oefarbe: impl Into<Option<EFarbe>>,
    esololike: ESoloLike,
    payoutdecider_in: impl Into<VPayoutDeciderSoloLike>,
    stossparams: SStossParams,
) -> SActivelyPlayableRules {
    let (oefarbe, payoutdecider_in) = (oefarbe.into(), payoutdecider_in.into());
    macro_rules! sololike_internal{(
        ($payoutdecider: expr, $str_payoutdecider: expr),
        $of_heuristic_active_occurence_probability: expr,
    ) => {
        SRulesSoloLike{
            payoutdecider: $payoutdecider,
            trumpfdecider: {
                STrumpfDecider::new(
                    match esololike {
                        ESoloLike::Solo => &[ESchlag::Ober, ESchlag::Unter],
                        ESoloLike::Wenz => &[ESchlag::Unter],
                        ESoloLike::Geier => &[ESchlag::Ober],
                    },
                    oefarbe,
                )
            },
            epi,
            str_name: format!("{}{}{}",
                match oefarbe {
                    None => "".to_string(),
                    Some(efarbe) => format!("{}-", efarbe),
                },
                match esololike {
                    ESoloLike::Solo => "Solo",
                    ESoloLike::Wenz => "Wenz",
                    ESoloLike::Geier => "Geier",
                },
                $str_payoutdecider
            ),
            of_heuristic_active_occurence_probability: $of_heuristic_active_occurence_probability,
            stossparams,
        }.into()
    }}
    cartesian_match!(
        sololike_internal,
        match (payoutdecider_in) {
            VPayoutDeciderSoloLike::PointBased(payoutdecider) => (payoutdecider, ""),
            VPayoutDeciderSoloLike::Tout(payoutdecider) => (payoutdecider, "-Tout"),
            VPayoutDeciderSoloLike::Sie(payoutdecider) => (payoutdecider, "-Sie"),
        },
        match ((oefarbe, esololike, &payoutdecider_in)) {
            (
                Some(_efarbe),
                ESoloLike::Solo,
                &VPayoutDeciderSoloLike::PointBased(SPayoutDeciderPointBased{
                    payoutparams: _,
                    pointstowin: VGameAnnouncementPrioritySoloLike::SoloSimple(_),
                }),
            ) => (Some(0.02)),
            (
                None,
                ESoloLike::Wenz|ESoloLike::Geier,
                &VPayoutDeciderSoloLike::PointBased(SPayoutDeciderPointBased{
                    payoutparams: _,
                    pointstowin: VGameAnnouncementPrioritySoloLike::SoloSimple(_),
                }),
            ) => (Some(0.04)),
            (
                Some(_efarbe),
                ESoloLike::Wenz|ESoloLike::Geier,
                &VPayoutDeciderSoloLike::PointBased(SPayoutDeciderPointBased{
                    payoutparams: _,
                    pointstowin: VGameAnnouncementPrioritySoloLike::SoloSimple(_),
                }),
            ) => (Some(0.06)),
            (
                Some(_efarbe),
                ESoloLike::Solo,
                &VPayoutDeciderSoloLike::Tout(_),
            ) => (Some(0.001)),
            (
                None,
                ESoloLike::Wenz|ESoloLike::Geier,
                &VPayoutDeciderSoloLike::Tout(_),
            ) => (Some(0.002)),
            (
                Some(_efarbe),
                ESoloLike::Wenz|ESoloLike::Geier,
                &VPayoutDeciderSoloLike::Tout(_),
            ) => (Some(0.003)),
            _ => None,
        },
    )
}

#[test]
fn test_trumpfdecider() {
    use crate::primitives::card::ECard::*;
    assert_eq!(
        STrumpfDecider::new(&[ESchlag::Ober, ESchlag::Unter], Some(EFarbe::Gras))
            .trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EO, GO, HO, SO, EU, GU, HU, SU, GA, GZ, GK, G9, G8, G7],
    );
    assert_eq!(
        STrumpfDecider::new(&[ESchlag::Unter], Some(EFarbe::Gras))
            .trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EU, GU, HU, SU, GA, GZ, GK, GO, G9, G8, G7],
    );
    assert_eq!(
        STrumpfDecider::new(&[ESchlag::Unter], None)
            .trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EU, GU, HU, SU],
    );
}

#[test]
fn test_equivalent_when_on_same_hand_rulessolo() {
    use crate::primitives::card::ECard::*;
    for epi in EPlayerIndex::values() {
        let sololike_internal = |
            oefarbe: Option<EFarbe>,
            payoutdecider: VPayoutDeciderSoloLike,
        | -> SActivelyPlayableRules {
            sololike(
                epi,
                oefarbe,
                ESoloLike::Solo,
                payoutdecider,
                SStossParams::new(/*n_stoss_max*/4),
            )
        };
        let payoutparams = SPayoutDeciderParams::new(
            /*n_payout_base*/50,
            /*n_payout_schneider_schwarz*/10,
            SLaufendeParams::new(
                /*n_payout_per_lauf*/10,
                /*n_lauf_lbound*/3,
            ),
        );
        assert_eq!(
            sololike_internal(
                Some(EFarbe::Herz),
                SPayoutDeciderPointBased::<VGameAnnouncementPrioritySoloLike>::new(
                    payoutparams.clone(),
                    /*i_prioindex*/VGameAnnouncementPrioritySoloLike::SoloSimple(0),
                ).into(),
            ).equivalent_when_on_same_hand(),
            SCardsPartition::new_from_slices(&[
                &[EO, GO, HO, SO] as &[ECard],
                &[EU, GU, HU, SU],
                &[H9, H8, H7],
                &[E9, E8, E7],
                &[G9, G8, G7],
                &[S9, S8, S7],
            ])
        );
        assert_eq!(
            sololike_internal(
                Some(EFarbe::Herz),
                SPayoutDeciderTout::new(
                    payoutparams.clone(),
                    /*i_prioindex*/0,
                ).into(),
            ).equivalent_when_on_same_hand(),
            SCardsPartition::new_from_slices(&[
                &[EO, GO, HO, SO, EU, GU, HU, SU, HA, HZ, HK, H9, H8, H7] as &[ECard],
                &[EA, EZ, EK, E9, E8, E7],
                &[GA, GZ, GK, G9, G8, G7],
                &[SA, SZ, SK, S9, S8, S7],
            ])
        );
        assert_eq!(
            sololike_internal(
                /*oefarbe*/None,
                SPayoutDeciderSie::new(
                    payoutparams.clone()
                ).into(),
            ).equivalent_when_on_same_hand(),
            SCardsPartition::new_from_slices(&[
                &[EO, GO, HO, SO, EU, GU, HU, SU,] as &[ECard],
                &[HA, HZ, HK, H9, H8, H7],
                &[EA, EZ, EK, E9, E8, E7],
                &[GA, GZ, GK, G9, G8, G7],
                &[SA, SZ, SK, S9, S8, S7],
            ])
        );
    }
}
