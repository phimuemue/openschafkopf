use crate::primitives::*;
use crate::rules::{
    *,
    trumpfdecider::*,
    payoutdecider::*,
};
use std::{
    fmt::self,
    cmp::Ordering,
    marker::PhantomData,
};
use crate::util::*;

pub trait TPayoutDecider : Sync + 'static + Clone + fmt::Debug {
    fn payout<Rules>(
        &self,
        rules: &Rules,
        rulestatecache: &SRuleStateCache,
        gamefinishedstiche: SStichSequenceGameFinished,
        playerparties13: &SPlayerParties13,
        perepi: impl TPerEPI,
    ) -> isize
        where Rules: TRulesNoObj;

    fn payouthints<Rules>(
        &self,
        rules: &Rules,
        stichseq: &SStichSequence,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        rulestatecache: &SRuleStateCache,
        playerparties13: &SPlayerParties13,
        perepi: impl TPerEPI,
    ) -> (Option<isize>, Option<isize>)
        where Rules: TRulesNoObj;
}

pub trait TPayoutDeciderSoloLike : Sync + 'static + Clone + fmt::Debug + TPayoutDecider {
    fn priority(&self) -> VGameAnnouncementPriority;
    fn with_increased_prio(&self, _prio: &VGameAnnouncementPriority, _ebid: EBid) -> Option<Self> {
        None
    }
    fn priorityinfo(&self) -> String {
        "".to_string()
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

impl TPayoutDecider for SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike> {
    fn payout<Rules>(
        &self,
        rules: &Rules,
        rulestatecache: &SRuleStateCache,
        gamefinishedstiche: SStichSequenceGameFinished,
        playerparties13: &SPlayerParties13,
        perepi: impl TPerEPI,
    ) -> isize
        where Rules: TRulesNoObj
    {
        self.payout(rules, rulestatecache, gamefinishedstiche, playerparties13, perepi)
    }

    fn payouthints<Rules>(
        &self,
        rules: &Rules,
        stichseq: &SStichSequence,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        rulestatecache: &SRuleStateCache,
        playerparties13: &SPlayerParties13,
        perepi: impl TPerEPI,
    ) -> (Option<isize>, Option<isize>)
        where Rules: TRulesNoObj
    {
        self.payouthints(rules, stichseq, ahand, rulestatecache, playerparties13, perepi)
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
            n_points_to_win = n_points_to_win + match ebid {
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
                    format!("for {}", n_points_to_win).to_string()
                } else {
                    "".to_string()
                }
            },
        }
    }
}

#[derive(Clone, Debug, new)]
pub struct SPayoutDeciderTout {
    payoutparams : SPayoutDeciderParams,
    i_prio: isize,
}

impl TPayoutDecider for SPayoutDeciderTout {
    fn payout<Rules>(
        &self,
        if_dbg_else!({rules}{_rules}): &Rules,
        rulestatecache: &SRuleStateCache,
        gamefinishedstiche: SStichSequenceGameFinished,
        playerparties13: &SPlayerParties13,
        perepi: impl TPerEPI,
    ) -> isize
        where Rules: TRulesNoObj,
    {
        // TODORULES optionally count schneider/schwarz
        internal_payout(
            /*n_payout_single_player*/ (self.payoutparams.n_payout_base + self.payoutparams.laufendeparams.payout_laufende::<Rules, _>(rulestatecache, gamefinishedstiche, playerparties13)) * 2,
            playerparties13,
            /*b_primary_party_wins*/ debug_verify_eq!(
                rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich==gamefinishedstiche.get().kurzlang().cards_per_player(),
                gamefinishedstiche.get().completed_stichs_winner_index(rules)
                    .all(|(_stich, epi_winner)| playerparties13.is_primary_party(epi_winner))
            ),
            perepi,
        )
    }

    fn payouthints<Rules>(
        &self,
        if_dbg_else!({rules}{_rules}): &Rules,
        stichseq: &SStichSequence,
        _ahand: &EnumMap<EPlayerIndex, SHand>,
        rulestatecache: &SRuleStateCache,
        playerparties13: &SPlayerParties13,
        perepi: impl TPerEPI,
    ) -> (Option<isize>, Option<isize>)
        where Rules: TRulesNoObj
    {
        if debug_verify_eq!(
            rulestatecache.changing.mapepipointstichcount[playerparties13.primary_player()].n_stich < stichseq.completed_stichs().len(),
            !stichseq.completed_stichs_winner_index(rules)
                .all(|(_stich, epi_winner)| playerparties13.is_primary_party(epi_winner))
        ) {
            perepi.per_epi_map(
                internal_payout(
                    /*n_payout_single_player*/ (self.payoutparams.n_payout_base) * 2, // TODO laufende
                    playerparties13,
                    /*b_primary_party_wins*/ false,
                    perepi,
                ),
                |_epi, n_payout| {
                    assert_ne!(0, n_payout);
                    tpl_flip_if(0<n_payout, (None, Some(n_payout)))
                },
            )
        } else {
            perepi.per_epi(|_epi| (None, None))
        }
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderTout {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloTout(self.i_prio)
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

impl TPayoutDecider for SPayoutDeciderSie {
    fn payout<Rules>(
        &self,
        rules: &Rules,
        _rulestatecache: &SRuleStateCache,
        gamefinishedstiche: SStichSequenceGameFinished,
        playerparties13: &SPlayerParties13,
        perepi: impl TPerEPI,
    ) -> isize
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
            ),
            perepi,
        )
    }

    fn payouthints<Rules>(
        &self,
        rules: &Rules,
        stichseq: &SStichSequence,
        ahand: &EnumMap<EPlayerIndex, SHand>,
        _rulestatecache: &SRuleStateCache,
        playerparties13: &SPlayerParties13,
        perepi: impl TPerEPI,
    ) -> (Option<isize>, Option<isize>)
        where Rules: TRulesNoObj
    {
        let itcard = stichseq.visible_stichs().filter_map(|stich| stich.get(playerparties13.primary_player())).copied()
            .chain(ahand[playerparties13.primary_player()].cards().iter().copied());
        if
            !cards_valid_for_sie(
                rules,
                itcard.clone(),
                stichseq.kurzlang(),
            )
        {
            perepi.per_epi_map(
                internal_payout(
                    /*n_payout_single_player*/ self.payoutparams.n_payout_base * 4,
                    playerparties13,
                    /*b_primary_party_wins*/ false,
                    perepi,
                ),
                |_epi, n_payout| {
                    assert_ne!(0, n_payout);
                    tpl_flip_if(0<n_payout, (None, Some(n_payout)))
                },
            )
        } else {
            perepi.per_epi(|_epi| (None, None))
        }
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderSie {
    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloSie
    }
}

#[derive(Clone, Debug)]
pub struct SRulesSoloLike<StaticEPI, TrumpfDecider, PayoutDecider> {
    pub str_name: String,
    phantom : PhantomData<(StaticEPI, TrumpfDecider)>,
    payoutdecider: PayoutDecider,
}

impl<StaticEPI: TStaticValue<EPlayerIndex>, TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> fmt::Display for SRulesSoloLike<StaticEPI, TrumpfDecider, PayoutDecider> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.str_name, self.payoutdecider.priorityinfo())
    }
}

impl<StaticEPI: TStaticValue<EPlayerIndex>, TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> TActivelyPlayableRules for SRulesSoloLike<StaticEPI, TrumpfDecider, PayoutDecider> {
    box_clone_impl_by_clone!(TActivelyPlayableRules);
    fn priority(&self) -> VGameAnnouncementPriority {
        self.payoutdecider.priority()
    }
    fn with_increased_prio(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Box<dyn TActivelyPlayableRules>> {
        self.payoutdecider.with_increased_prio(prio, ebid)
            .map(|payoutdecider| Box::new(Self::new(payoutdecider, self.str_name.clone())) as Box<dyn TActivelyPlayableRules>)
    }
}

impl<StaticEPI: TStaticValue<EPlayerIndex>, TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> TRulesNoObj for SRulesSoloLike<StaticEPI, TrumpfDecider, PayoutDecider> {
    impl_rules_trumpf_noobj!(TrumpfDecider);
}

impl<StaticEPI: TStaticValue<EPlayerIndex>, TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> TRules for SRulesSoloLike<StaticEPI, TrumpfDecider, PayoutDecider> {
    box_clone_impl_by_clone!(TRules);
    impl_rules_trumpf!();
    impl_single_play!();
}

impl<StaticEPI: TStaticValue<EPlayerIndex>, TrumpfDecider: TTrumpfDecider, PayoutDecider: TPayoutDeciderSoloLike> SRulesSoloLike<StaticEPI, TrumpfDecider, PayoutDecider> {
    pub fn new(payoutdecider: PayoutDecider, str_name: String) -> Self {
        Self {
            phantom: PhantomData,
            payoutdecider,
            str_name,
        }
    }
    fn internal_playerindex(&self) -> EPlayerIndex { // TODORUST const fn
        StaticEPI::VALUE
    }
}

pub enum ESoloLike {
    Solo,
    Wenz,
    Geier,
}

type_dispatch_enum!(pub enum VPayoutDeciderSoloLike {
    PointBased(SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike>),
    Tout(SPayoutDeciderTout),
    Sie(SPayoutDeciderSie),
});

pub type SCoreSolo<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SStaticSchlagOber, STrumpfDeciderSchlag<
    SStaticSchlagUnter, TrumpfFarbDecider>>;
pub type SCoreGenericWenz<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SStaticSchlagUnter, TrumpfFarbDecider>;
pub type SCoreGenericGeier<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SStaticSchlagOber, TrumpfFarbDecider>;

pub fn sololike(
    epi: EPlayerIndex,
    oefarbe: impl Into<Option<EFarbe>>,
    esololike: ESoloLike,
    payoutdecider: impl Into<VPayoutDeciderSoloLike>,
) -> Box<dyn TActivelyPlayableRules> {
    macro_rules! sololike_internal{(
        $staticepi: ident,
        ($trumpfdecider_farbe: ty, $str_oefarbe: expr),
        ($trumpfdecider_core: ident, $str_esololike: expr),
        ($payoutdecider: expr, $str_payoutdecider: expr),
    ) => {
        Box::new(SRulesSoloLike::<
            $staticepi,
            $trumpfdecider_core<$trumpfdecider_farbe>,
            _,
        >::new(
            $payoutdecider,
            format!("{}{}{}", $str_oefarbe, $str_esololike, $str_payoutdecider),
        )) as Box<dyn TActivelyPlayableRules>
    }};
    cartesian_match!(
        sololike_internal,
        match (epi) {
            EPlayerIndex::EPI0 => SStaticEPI0,
            EPlayerIndex::EPI1 => SStaticEPI1,
            EPlayerIndex::EPI2 => SStaticEPI2,
            EPlayerIndex::EPI3 => SStaticEPI3,
        },
        match (oefarbe.into()) {
            None => (STrumpfDeciderNoTrumpf<SCompareFarbcardsSimple>, ""),
            Some(EFarbe::Eichel) => (SStaticFarbeEichel, "Eichel"),
            Some(EFarbe::Gras) => (SStaticFarbeGras, "Gras"),
            Some(EFarbe::Herz) => (SStaticFarbeHerz, "Herz"),
            Some(EFarbe::Schelln) => (SStaticFarbeSchelln, "Schelln"),
        },
        match (esololike) {
            ESoloLike::Solo => (SCoreSolo, "Solo"),
            ESoloLike::Wenz => (SCoreGenericWenz, "Wenz"),
            ESoloLike::Geier => (SCoreGenericGeier, "Geier"),
        },
        match (payoutdecider.into()) {
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
        <SCoreSolo<SStaticFarbeGras> as TTrumpfDecider>
            ::trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EO, GO, HO, SO, EU, GU, HU, SU, GA, GZ, GK, G9, G8, G7],
    );
    assert_eq!(
        <SCoreGenericWenz<SStaticFarbeGras> as TTrumpfDecider>
            ::trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EU, GU, HU, SU, GA, GZ, GK, GO, G9, G8, G7],
    );
    assert_eq!(
        <SCoreGenericWenz<STrumpfDeciderNoTrumpf<SCompareFarbcardsSimple>> as TTrumpfDecider>
            ::trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EU, GU, HU, SU],
    );
}
