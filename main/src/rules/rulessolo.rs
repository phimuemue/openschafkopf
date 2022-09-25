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
    fn payout<TrumpfDecider: TTrumpfDecider>(&self, rules: &SRulesSoloLike<TrumpfDecider, Self>, rulestatecache: &SRuleStateCache, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> EnumMap<EPlayerIndex, isize>;
    fn payouthints<TrumpfDecider: TTrumpfDecider>(&self, rules: &SRulesSoloLike<TrumpfDecider, Self>, rulestatecache: &SRuleStateCache, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), n_stock: isize) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>;
    fn equivalent_when_on_same_hand(slccard_ordered: &[SCard]) -> Vec<Vec<SCard>>;

    fn points_as_payout(&self, _rules: &SRulesSoloLike<impl TTrumpfDecider, Self>) -> Option<(
        Box<dyn TRules>,
        Box<dyn Fn(&SStichSequence, &SHand, f32)->f32>,
    )> {
        None
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

    fn payout<TrumpfDecider: TTrumpfDecider>(&self, rules: &SRulesSoloLike<TrumpfDecider, Self>, rulestatecache: &SRuleStateCache, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, isize> {
        TPayoutDecider::payout(self,
            rules,
            rulestatecache,
            gamefinishedstiche,
            &SPlayerParties13::new(rules.epi),
        ).map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
    }

    fn payouthints<TrumpfDecider: TTrumpfDecider>(&self, rules: &SRulesSoloLike<TrumpfDecider, Self>, rulestatecache: &SRuleStateCache, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        TPayoutDecider::payouthints(self,
            rules,
            rulestatecache,
            stichseq,
            ahand,
            &SPlayerParties13::new(rules.epi),
        ).map(|intvlon_payout| intvlon_payout.map(|on_payout|
             on_payout.map(|n_payout| payout_including_stoss_doubling(n_payout, tpln_stoss_doubling)),
        ))
    }

    fn equivalent_when_on_same_hand(slccard_ordered: &[SCard]) -> Vec<Vec<SCard>> {
        equivalent_when_on_same_hand_point_based(slccard_ordered)
    }

    fn points_as_payout(&self, rules: &SRulesSoloLike<impl TTrumpfDecider, Self>) -> Option<(
        Box<dyn TRules>,
        Box<dyn Fn(&SStichSequence, &SHand, f32)->f32>,
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
            Box::new(move |stichseq: &SStichSequence, _hand: &SHand, f_payout: f32| {
                SPayoutDeciderPointsAsPayout::payout_to_points(
                    epi_active,
                    stichseq,
                    &pointstowin,
                    f_payout,
                )
            }) as Box<dyn Fn(&SStichSequence, &SHand, f32)->f32>,
        )
    )}
}

impl SPayoutDeciderPointsAsPayout<VGameAnnouncementPrioritySoloLike> {
    fn payout_to_points(epi_active: EPlayerIndex, stichseq: &SStichSequence, pointstowin: &impl TPointsToWin, f_payout: f32) -> f32 {
        let epi = unwrap!(stichseq.current_stich().current_playerindex());
        normalized_points_to_points(
            f_payout / SPlayerParties13::new(epi_active).multiplier(epi).as_num::<f32>(),
            pointstowin,
            /*b_primary*/ epi==epi_active,
        )
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderPointsAsPayout<VGameAnnouncementPrioritySoloLike> {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloLike(self.pointstowin.clone())
    }

    fn payout<TrumpfDecider: TTrumpfDecider>(&self, rules: &SRulesSoloLike<TrumpfDecider, Self>, rulestatecache: &SRuleStateCache, gamefinishedstiche: SStichSequenceGameFinished, _tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, isize> {
        let an_payout = TPayoutDecider::payout(self,
            rules,
            rulestatecache,
            gamefinishedstiche,
            &SPlayerParties13::new(rules.epi),
        );
        #[cfg(debug_assertions)] {
            let mut stichseq_check = SStichSequence::new(gamefinishedstiche.get().kurzlang());
            let mut ahand_check = EPlayerIndex::map_from_fn(|epi|
                SHand::new_from_iter(gamefinishedstiche.get().completed_stichs().iter().map(|stich| stich[epi]))
            );
            let playerparties = SPlayerParties13::new(rules.epi);
            for (epi_card, card) in gamefinishedstiche.get().completed_cards() {
                let b_primary = playerparties.is_primary_party(epi_card);
                assert_eq!(
                    Self::payout_to_points(
                        /*epi_active*/rules.epi,
                        &stichseq_check,
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
                stichseq_check.zugeben_custom_winner_index(*card, |stich| rules.winner_index(stich)); // TODO I could not simply pass rules. Why?
                ahand_check[epi_card].play_card(*card);
            }

        }
        an_payout
    }

    fn payouthints<TrumpfDecider: TTrumpfDecider>(&self, rules: &SRulesSoloLike<TrumpfDecider, Self>, rulestatecache: &SRuleStateCache, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, _tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        TPayoutDecider::payouthints(self,
            rules,
            rulestatecache,
            stichseq,
            ahand,
            &SPlayerParties13::new(rules.epi),
        )
    }

    fn equivalent_when_on_same_hand(slccard_ordered: &[SCard]) -> Vec<Vec<SCard>> {
        equivalent_when_on_same_hand_point_based(slccard_ordered)
    }
}

#[derive(Clone, Debug, new)]
pub struct SPayoutDeciderTout {
    payoutparams : SPayoutDeciderParams,
    i_prio: isize,
}

impl TPayoutDecider<SPlayerParties13> for SPayoutDeciderTout {
    fn payout<Rules>(
        &self,
        rules: &Rules,
        rulestatecache: &SRuleStateCache,
        gamefinishedstiche: SStichSequenceGameFinished,
        playerparties13: &SPlayerParties13,
    ) -> EnumMap<EPlayerIndex, isize>
        where Rules: TRulesNoObj,
    {
        // TODORULES optionally count schneider/schwarz
        internal_payout(
            /*n_payout_single_player*/ (self.payoutparams.n_payout_base + self.payoutparams.laufendeparams.payout_laufende(rules, rulestatecache, gamefinishedstiche, playerparties13)) * 2,
            playerparties13,
            /*b_primary_party_wins*/ debug_verify_eq!(
                rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich==gamefinishedstiche.get().kurzlang().cards_per_player(),
                gamefinishedstiche.get().completed_stichs_winner_index(rules)
                    .all(|(_stich, epi_winner)| playerparties13.is_primary_party(epi_winner))
            ),
        )
    }

    fn payouthints<Rules>(
        &self,
        if_dbg_else!({rules}{_rules}): &Rules,
        rulestatecache: &SRuleStateCache,
        stichseq: &SStichSequence,
        _ahand: &EnumMap<EPlayerIndex, SHand>,
        playerparties13: &SPlayerParties13,
    ) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>
        where Rules: TRulesNoObj
    {
        if debug_verify_eq!(
            rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich < stichseq.completed_stichs().len(),
            !stichseq.completed_stichs_winner_index(rules)
                .all(|(_stich, epi_winner)| playerparties13.is_primary_party(epi_winner))
        ) {
            internal_payout(
                /*n_payout_single_player*/ (self.payoutparams.n_payout_base) * 2, // TODO laufende
                playerparties13,
                /*b_primary_party_wins*/ false,
            )
                .map(|n_payout| {
                     SInterval::from_tuple(tpl_flip_if(0<verify_ne!(*n_payout, 0), (None, Some(*n_payout))))
                })
        } else {
            EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
        }
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderTout {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloTout(self.i_prio)
    }

    fn payout<TrumpfDecider: TTrumpfDecider>(&self, rules: &SRulesSoloLike<TrumpfDecider, Self>, rulestatecache: &SRuleStateCache, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, isize> {
        TPayoutDecider::payout(self,
            rules,
            rulestatecache,
            gamefinishedstiche,
            &SPlayerParties13::new(rules.epi),
        ).map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
    }

    fn payouthints<TrumpfDecider: TTrumpfDecider>(&self, rules: &SRulesSoloLike<TrumpfDecider, Self>, rulestatecache: &SRuleStateCache, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        TPayoutDecider::payouthints(self,
            rules,
            rulestatecache,
            stichseq,
            ahand,
            &SPlayerParties13::new(rules.epi),
        ).map(|intvlon_payout| intvlon_payout.map(|on_payout|
             on_payout.map(|n_payout| payout_including_stoss_doubling(n_payout, tpln_stoss_doubling)),
        ))
    }

    fn equivalent_when_on_same_hand(slccard_ordered: &[SCard]) -> Vec<Vec<SCard>> {
        vec![slccard_ordered.to_vec()] // In Tout, neighboring cards are equivalent regardless of points_card.
    }
}

#[derive(Clone, Debug, new)]
pub struct SPayoutDeciderSie {
    payoutparams : SPayoutDeciderParams,
}

fn cards_valid_for_sie_internal<Rules: TRulesNoObj, ItCard: Iterator<Item=SCard>, FnAllowUnter: Fn(EFarbe)->bool>(
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

// TODO SPayoutDeciderSie should be able to work with any TTrumpfDecider
fn cards_valid_for_sie<Rules: TRulesNoObj, ItCard: Iterator<Item=SCard>>(
    rules: &Rules,
    itcard: ItCard,
    ekurzlang: EKurzLang,
) -> bool {
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

impl TPayoutDecider<SPlayerParties13> for SPayoutDeciderSie {
    fn payout<Rules>(
        &self,
        rules: &Rules,
        _rulestatecache: &SRuleStateCache,
        gamefinishedstiche: SStichSequenceGameFinished,
        playerparties13: &SPlayerParties13,
    ) -> EnumMap<EPlayerIndex, isize>
        where Rules: TRulesNoObj,
    {
        // TODORULES optionally count schneider/schwarz
        internal_payout(
            /*n_payout_single_player*/ (self.payoutparams.n_payout_base
            + {
                gamefinishedstiche.get().completed_stichs().len().as_num::<isize>()
            } * self.payoutparams.laufendeparams.n_payout_per_lauf) * 4,
            playerparties13,
            /*b_primary_party_wins*/cards_valid_for_sie(
                rules,
                gamefinishedstiche.get().completed_stichs().iter().map(|stich| stich[playerparties13.primary_player()]),
                gamefinishedstiche.get().kurzlang(),
            )
        )
    }

    fn payouthints<Rules>(
        &self,
        rules: &Rules,
        _rulestatecache: &SRuleStateCache,
        stichseq: &SStichSequence,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        playerparties13: &SPlayerParties13,
    ) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>>
        where Rules: TRulesNoObj
    {
        let itcard = stichseq.visible_stichs().iter().filter_map(|stich| stich.get(playerparties13.primary_player())).copied()
            .chain(ahand[playerparties13.primary_player()].cards().iter().copied());
        if
            !cards_valid_for_sie(
                rules,
                itcard.clone(),
                stichseq.kurzlang(),
            )
        {
            internal_payout(
                /*n_payout_single_player*/ self.payoutparams.n_payout_base * 4,
                playerparties13,
                /*b_primary_party_wins*/ false,
            )
                .map(|n_payout| {
                     SInterval::from_tuple(tpl_flip_if(0<verify_ne!(*n_payout, 0), (None, Some(*n_payout))))
                })
        } else {
            EPlayerIndex::map_from_fn(|_epi| SInterval::from_raw([None, None]))
        }
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderSie {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloSie
    }

    fn payout<TrumpfDecider: TTrumpfDecider>(&self, rules: &SRulesSoloLike<TrumpfDecider, Self>, rulestatecache: &SRuleStateCache, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, isize> {
        TPayoutDecider::payout(self,
            rules,
            rulestatecache,
            gamefinishedstiche,
            &SPlayerParties13::new(rules.epi),
        ).map(|n_payout| payout_including_stoss_doubling(*n_payout, tpln_stoss_doubling))
    }

    fn payouthints<TrumpfDecider: TTrumpfDecider>(&self, rules: &SRulesSoloLike<TrumpfDecider, Self>, rulestatecache: &SRuleStateCache, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), _n_stock: isize) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        TPayoutDecider::payouthints(self,
            rules,
            rulestatecache,
            stichseq,
            ahand,
            &SPlayerParties13::new(rules.epi),
        ).map(|intvlon_payout| intvlon_payout.map(|on_payout|
             on_payout.map(|n_payout| payout_including_stoss_doubling(n_payout, tpln_stoss_doubling)),
        ))
    }

    fn equivalent_when_on_same_hand(slccard_ordered: &[SCard]) -> Vec<Vec<SCard>> {
        use crate::card::card_values::*;
        assert!(matches!(slccard_ordered, // TODO SPayoutDeciderSie should be able to work with any TTrumpfDecider
            &[EO, GO, HO, SO, EU, GU, HU, SU]
            | &[HA, HZ, HK, H9, H8, H7]
            | &[EA, EZ, EK, E9, E8, E7]
            | &[GA, GZ, GK, G9, G8, G7]
            | &[SA, SZ, SK, S9, S8, S7]
        ));
        vec![slccard_ordered.to_vec()] // In Sie, neighboring cards are equivalent regardless of points_card.
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

impl<TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> TRulesNoObj for SRulesSoloLike<TrumpfDecider, PayoutDecider> {
    impl_rules_trumpf_noobj!(TrumpfDecider);
}

impl<TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> TRules for SRulesSoloLike<TrumpfDecider, PayoutDecider> {
    impl_rules_trumpf!();
    impl_single_play!();

    fn payout_no_invariant(&self, gamefinishedstiche: SStichSequenceGameFinished, tpln_stoss_doubling: (usize, usize), n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, isize> {
        self.payoutdecider.payout(
            self,
            rulestatecache,
            gamefinishedstiche,
            tpln_stoss_doubling,
            n_stock,
        )
    }

    fn payouthints(&self, stichseq: &SStichSequence, ahand: &EnumMap<EPlayerIndex, SHand>, tpln_stoss_doubling: (usize, usize), n_stock: isize, rulestatecache: &SRuleStateCache) -> EnumMap<EPlayerIndex, SInterval<Option<isize>>> {
        self.payoutdecider.payouthints(
            self,
            rulestatecache,
            stichseq,
            ahand,
            tpln_stoss_doubling,
            n_stock,
        )
    }

    fn equivalent_when_on_same_hand(&self) -> SCardsPartition {
        let (mapefarbeveccard, veccard_trumpf) = self.trumpfdecider.equivalent_when_on_same_hand();
        let vecveccard = mapefarbeveccard.into_raw().into_iter().chain(Some(veccard_trumpf).into_iter())
            .flat_map(|veccard| PayoutDecider::equivalent_when_on_same_hand(&veccard))
            .collect::<Vec<_>>();
        SCardsPartition::new_from_slices(
            &vecveccard.iter()
                .map(|veccard| veccard as &[SCard]).collect::<Vec<_>>(),
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
        Box<dyn Fn(&SStichSequence, &SHand, f32)->f32>,
    )> {
        self.payoutdecider.points_as_payout(self)
    }

    fn snapshot_cache(&self, _rulestatecachefixed: &SRuleStateCacheFixed) -> Option<Box<dyn TSnapshotCache<SMinMax>>> {
        super::snapshot_cache_point_based(SPlayerParties13::new(self.epi))
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
            Some(efarbe) => (efarbe, efarbe.to_string()),
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
    use crate::card::card_values::*;
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
    use crate::card::card_values::*;
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
                &[EO, GO, HO, SO] as &[SCard],
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
                &[EO, GO, HO, SO, EU, GU, HU, SU, HA, HZ, HK, H9, H8, H7] as &[SCard],
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
                &[EO, GO, HO, SO, EU, GU, HU, SU,] as &[SCard],
                &[HA, HZ, HK, H9, H8, H7],
                &[EA, EZ, EK, E9, E8, E7],
                &[GA, GZ, GK, G9, G8, G7],
                &[SA, SZ, SK, S9, S8, S7],
            ])
        );
    }
}
