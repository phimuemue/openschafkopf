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
    fn payout(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>, rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, isize>;
    fn payouthints(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>, rulestatecache: &SRuleStateCache, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>;
    fn equivalent_when_on_same_hand(slccard_ordered: &[ECard]) -> Vec<Vec<ECard>>;

    fn points_as_payout(&self, _rules: &SRulesSoloLike<impl TTrumpfDecider, Self>) -> Option<(
        Box<dyn TRules>,
        Box<dyn Fn(&SStichSequence, (EPlayerIndex, &SHand), f32)->f32>,
    )> {
        None
    }

    fn snapshot_cache(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>) -> Box<dyn TSnapshotCache<SMinMax>>;
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

    fn payout(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>, rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, isize> {
        TPayoutDecider::payout(self,
            rules,
            rulestatecache,
            stichseq,
            &SPlayerParties13::new(rules.epi),
        ).map(|n_payout| n_payout * expensifiers.stoss_doubling_factor())
    }

    fn payouthints(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>, rulestatecache: &SRuleStateCache, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        TPayoutDecider::payouthints(self,
            rules,
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

    fn points_as_payout(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>) -> Option<(
        Box<dyn TRules>,
        Box<dyn Fn(&SStichSequence, (EPlayerIndex, &SHand), f32)->f32>,
    )> {
        //assert_eq!(self, rules.payoutdecider); // TODO
        let pointstowin = self.pointstowin.clone();
        let epi_active = rules.epi;
        Some((
            Box::new(SRulesSoloLike{
                str_name: rules.str_name.clone(),
                epi: rules.epi,
                payoutdecider: SPayoutDeciderPointsAsPayout{
                    pointstowin: pointstowin.clone(),
                },
                trumpfdecider: rules.trumpfdecider.clone(),
            }) as Box<dyn TRules>,
            Box::new(move |_stichseq: &SStichSequence, (epi_hand, _hand): (EPlayerIndex, &SHand), f_payout: f32| {
                SPayoutDeciderPointsAsPayout::payout_to_points(
                    epi_active,
                    epi_hand,
                    &pointstowin,
                    f_payout,
                )
            }) as Box<dyn Fn(&SStichSequence, (EPlayerIndex, &SHand), f32)->f32>,
        )
    )}

    fn snapshot_cache(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>) -> Box<dyn TSnapshotCache<SMinMax>> {
        super::snapshot_cache_point_based(SPlayerParties13::new(rules.epi))
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

    fn payout(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>, rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, _expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, isize> {
        let an_payout = TPayoutDecider::payout(self,
            rules,
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

    fn payouthints(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>, rulestatecache: &SRuleStateCache, tplahandstichseq: (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), _expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        TPayoutDecider::payouthints(self,
            rules,
            rulestatecache,
            tplahandstichseq,
            &SPlayerParties13::new(rules.epi),
        )
    }

    fn equivalent_when_on_same_hand(slccard_ordered: &[ECard]) -> Vec<Vec<ECard>> {
        equivalent_when_on_same_hand_point_based(slccard_ordered)
    }

    fn snapshot_cache(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>) -> Box<dyn TSnapshotCache<SMinMax>> {
        payoutdecider::snapshot_cache_points_monotonic(
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

    fn payout(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>, rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, isize> {
        let playerparties13 = &SPlayerParties13::new(rules.epi);
        // TODORULES optionally count schneider/schwarz
        internal_payout(
            /*n_payout_primary_unmultiplied*/((self.payoutparams.n_payout_base + self.payoutparams.laufendeparams.payout_laufende(rules.trumpfdecider(), rulestatecache, stichseq, playerparties13)) * 2)
                .neg_if(!/*b_primary_party_wins*/debug_verify_eq!(
                    rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich==stichseq.get().kurzlang().cards_per_player(),
                    stichseq.get().completed_stichs_winner_index(rules)
                        .all(|(_stich, epi_winner)| playerparties13.is_primary_party(epi_winner))
                )),
            playerparties13,
        ).map(|n_payout| n_payout * expensifiers.stoss_doubling_factor())
    }

    fn payouthints(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>, rulestatecache: &SRuleStateCache, (_ahand, stichseq): (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
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

    fn snapshot_cache(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>) -> Box<dyn TSnapshotCache<SMinMax>> {
        super::snapshot_cache_point_based(SPlayerParties13::new(rules.epi))
    }
}

#[derive(Clone, Debug, new)]
pub struct SPayoutDeciderSie {
    payoutparams : SPayoutDeciderParams,
}

// TODO SPayoutDeciderSie should be able to work with any TTrumpfDecider
fn cards_valid_for_sie<Rules: TRules, ItCard: Iterator<Item=ECard>>(
    rules: &Rules,
    itcard: ItCard,
    ekurzlang: EKurzLang,
) -> bool {
    fn cards_valid_for_sie_internal<Rules: TRules, ItCard: Iterator<Item=ECard>, FnAllowUnter: Fn(EFarbe)->bool>(
        rules: &Rules,
        mut itcard: ItCard,
        fn_allow_unter: FnAllowUnter,
    ) -> bool {
        itcard.all(|card| {
            let b_card_valid = match card.schlag() {
                ESchlag::Ober => true,
                ESchlag::Unter => fn_allow_unter(card.farbe()),
                ESchlag::S7 | ESchlag::S8 | ESchlag::S9 | ESchlag::Zehn | ESchlag::Koenig | ESchlag::Ass => false,
            };
            assert!(!b_card_valid || rules.trumpforfarbe(card).is_trumpf());
            b_card_valid
        })
    }
    match ekurzlang {
        EKurzLang::Lang => cards_valid_for_sie_internal(rules, itcard, /*fn_allow_unter*/|_| true),
        EKurzLang::Kurz => cards_valid_for_sie_internal(rules, itcard, /*fn_allow_unter*/|efarbe|
            match efarbe {
                EFarbe::Eichel | EFarbe::Gras => true,
                EFarbe::Herz | EFarbe::Schelln => false,
            }
        ),
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderSie {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloSie
    }

    fn payout(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>, _rulestatecache: &SRuleStateCache, stichseq: SStichSequenceGameFinished, expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, isize> {
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

    fn payouthints(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>, _rulestatecache: &SRuleStateCache, (ahand, stichseq): (&EnumMap<EPlayerIndex, SHand>, &SStichSequence), expensifiers: &SExpensifiers) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        if !cards_valid_for_sie(
            rules,
            stichseq.cards_from_player(&ahand[rules.epi], rules.epi).copied(),
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
        use crate::card::ECard::*;
        assert!(matches!(slccard_ordered, // TODO SPayoutDeciderSie should be able to work with any TTrumpfDecider
            &[EO, GO, HO, SO, EU, GU, HU, SU]
            | &[HA, HZ, HK, H9, H8, H7]
            | &[EA, EZ, EK, E9, E8, E7]
            | &[GA, GZ, GK, G9, G8, G7]
            | &[SA, SZ, SK, S9, S8, S7]
        ));
        vec![slccard_ordered.to_vec()] // In Sie, neighboring cards are equivalent regardless of points_card.
    }

    fn snapshot_cache(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>) -> Box<dyn TSnapshotCache<SMinMax>> {
        super::snapshot_cache_point_based(SPlayerParties13::new(rules.epi))
    }
}

#[derive(Clone, Debug)]
pub struct SRulesSoloLike<TrumpfDecider, PayoutDecider> {
    pub str_name: String,
    epi: EPlayerIndex,
    payoutdecider: PayoutDecider,
    trumpfdecider: TrumpfDecider,
}

impl<TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> fmt::Display for SRulesSoloLike<TrumpfDecider, PayoutDecider> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.str_name, self.payoutdecider.priorityinfo())
    }
}

impl<TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> TActivelyPlayableRules for SRulesSoloLike<TrumpfDecider, PayoutDecider> {
    fn priority(&self) -> VGameAnnouncementPriority {
        self.payoutdecider.priority()
    }
    fn with_increased_prio(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Box<dyn TActivelyPlayableRules>> {
        self.payoutdecider.with_increased_prio(prio, ebid)
            .map(|payoutdecider| Box::new(Self{
                payoutdecider,
                trumpfdecider: self.trumpfdecider.clone(),
                epi: self.epi,
                str_name: self.str_name.clone(),
            }) as Box<dyn TActivelyPlayableRules>)
    }
}

impl<TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> TRulesWithTrumpfDecider for SRulesSoloLike<TrumpfDecider, PayoutDecider> {
    impl_rules_with_trumpfdecider!(TrumpfDecider);
}

impl<TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> TRules for SRulesSoloLike<TrumpfDecider, PayoutDecider> {
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
        let (mapefarbeveccard, veccard_trumpf) = self.trumpfdecider.equivalent_when_on_same_hand();
        let vecveccard = mapefarbeveccard.into_raw().into_iter().chain(Some(veccard_trumpf).into_iter())
            .flat_map(|veccard| PayoutDecider::equivalent_when_on_same_hand(&veccard))
            .collect::<Vec<_>>();
        SCardsPartition::new_from_slices(
            &vecveccard.iter()
                .map(|veccard| veccard as &[ECard]).collect::<Vec<_>>(),
        )
    }

    fn only_minmax_points_when_on_same_hand(&self, _rulestatecache: &SRuleStateCacheFixed) -> Option<(SCardsPartition, SPlayerPartiesTable)> {
        // TODO this is ok for normal Solo, point based Solo, Tout, Sie. But we can possibly improve this for e.g. Tout/Sie.
        let (mapefarbeveccard, veccard_trumpf) = self.trumpfdecider.equivalent_when_on_same_hand();
        Some((
            SCardsPartition::new_from_slices(
                &mapefarbeveccard.into_raw().iter().chain(Some(veccard_trumpf).iter())
                    .map(|vec| -> &[_] { vec })
                    .collect::<Vec<_>>(),
            ),
            SPlayerParties13::new(self.epi).into(),
        ))
    }

    fn points_as_payout(&self) -> Option<(
        Box<dyn TRules>,
        Box<dyn Fn(&SStichSequence, (EPlayerIndex, &SHand), f32)->f32>,
    )> {
        self.payoutdecider.points_as_payout(self)
    }

    fn snapshot_cache(&self, _rulestatecachefixed: &SRuleStateCacheFixed) -> Box<dyn TSnapshotCache<SMinMax>> {
        self.payoutdecider.snapshot_cache(self)
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

pub type STrumpfDeciderSolo<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SStaticSchlagOber,
    STrumpfDeciderSchlag<
        SStaticSchlagUnter,
        TrumpfFarbDecider
    >
>;
pub type STrumpfDecider1Primary<TrumpfFarbDecider> = STrumpfDeciderSchlag<ESchlag, TrumpfFarbDecider>;

pub fn sololike(
    epi: EPlayerIndex,
    oefarbe: impl Into<Option<EFarbe>>,
    esololike: ESoloLike,
    payoutdecider: impl Into<VPayoutDeciderSoloLike>,
) -> Box<dyn TActivelyPlayableRules> {
    let (oefarbe, payoutdecider) = (oefarbe.into(), payoutdecider.into());
    assert!(!matches!(payoutdecider, VPayoutDeciderSoloLike::Sie(_)) || oefarbe.is_none()); // TODO SPayoutDeciderSie should be able to work with any TTrumpfDecider
    macro_rules! sololike_internal{(
        ($trumpfdecider_farbe: expr, $str_oefarbe: expr),
        ($trumpfdecider_core: expr, $str_esololike: expr),
        ($payoutdecider: expr, $str_payoutdecider: expr),
    ) => {
        Box::new(SRulesSoloLike{
            payoutdecider: $payoutdecider,
            trumpfdecider: $trumpfdecider_core($trumpfdecider_farbe),
            epi,
            str_name: format!("{}{}{}", $str_oefarbe, $str_esololike, $str_payoutdecider),
        }) as Box<dyn TActivelyPlayableRules>
    }}
    cartesian_match!(
        sololike_internal,
        match (oefarbe) {
            None => (STrumpfDeciderNoTrumpf::<SCompareFarbcardsSimple>::default(), ""),
            Some(efarbe) => (efarbe, format!("{}-", efarbe)),
        },
        match (esololike) {
            ESoloLike::Solo => (|trumpfdecider_farbe| STrumpfDeciderSolo::new(SStaticSchlagOber{}, STrumpfDeciderSchlag::new(SStaticSchlagUnter{}, trumpfdecider_farbe)), "Solo"),
            ESoloLike::Wenz => (|trumpfdecider_farbe| STrumpfDecider1Primary::new(ESchlag::Unter, trumpfdecider_farbe), "Wenz"),
            ESoloLike::Geier => (|trumpfdecider_farbe| STrumpfDecider1Primary::new(ESchlag::Ober, trumpfdecider_farbe), "Geier"),
        },
        match (payoutdecider) {
            VPayoutDeciderSoloLike::PointBased(payoutdecider) => (payoutdecider, ""),
            VPayoutDeciderSoloLike::Tout(payoutdecider) => (payoutdecider, "-Tout"),
            VPayoutDeciderSoloLike::Sie(payoutdecider) => (payoutdecider, "-Sie"),
        },
    )
}

#[test]
fn test_trumpfdecider() {
    use crate::card::ECard::*;
    assert_eq!(
        STrumpfDeciderSolo::<SStaticFarbeGras>::default()
            .trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EO, GO, HO, SO, EU, GU, HU, SU, GA, GZ, GK, G9, G8, G7],
    );
    assert_eq!(
        STrumpfDecider1Primary::new(ESchlag::Unter, EFarbe::Gras)
            .trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EU, GU, HU, SU, GA, GZ, GK, GO, G9, G8, G7],
    );
    assert_eq!(
        STrumpfDecider1Primary::new(ESchlag::Unter, STrumpfDeciderNoTrumpf::<SCompareFarbcardsSimple>::default())
            .trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EU, GU, HU, SU],
    );
}

#[test]
fn test_equivalent_when_on_same_hand_rulessolo() {
    use crate::card::ECard::*;
    for epi in EPlayerIndex::values() {
        let sololike_internal = |
            oefarbe: Option<EFarbe>,
            payoutdecider: VPayoutDeciderSoloLike,
        | -> Box<dyn TActivelyPlayableRules> {
            sololike(
                epi,
                oefarbe,
                ESoloLike::Solo,
                payoutdecider,
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
