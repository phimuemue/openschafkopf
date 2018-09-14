use primitives::*;
use rules::{
    *,
    trumpfdecider::*,
    payoutdecider::*,
};
use std::{
    fmt::{self, Display},
    cmp::Ordering,
    marker::PhantomData,
};
use util::*;

pub trait TPayoutDeciderSoloLike : TPayoutDecider + Display {
    type PrioParams;
    type PayoutParams;
    fn new(payoutparams: Self::PayoutParams, prioparams: Self::PrioParams) -> Self;
    fn priority(&self) -> VGameAnnouncementPriority;
    fn with_increased_prio(&self, _prio: &VGameAnnouncementPriority, _ebid: EBid) -> Option<Self> {
        None
    }
}

impl TPointsToWin for VGameAnnouncementPrioritySoloLike {
    fn points_to_win(&self) -> isize {
        match self {
            VGameAnnouncementPrioritySoloLike::SoloSimple(_) => 61,
            VGameAnnouncementPrioritySoloLike::SoloSteigern{n_points_to_win, n_step: _} => *n_points_to_win,
        }
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike> {
    type PrioParams = VGameAnnouncementPrioritySoloLike;
    type PayoutParams = SPayoutDeciderParams;

    fn new(payoutparams: Self::PayoutParams, pointstowin: Self::PrioParams) -> Self {
        SPayoutDeciderPointBased {
            payoutparams,
            pointstowin,
        }
    }

    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloLike(self.pointstowin.clone())
    }

    fn with_increased_prio(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Self> {
        use self::VGameAnnouncementPriority::*;
        use self::VGameAnnouncementPrioritySoloLike::*;
        if let (SoloLike(SoloSteigern{..}), &SoloLike(SoloSteigern{n_points_to_win, n_step})) = (self.priority(), prio) {
            let n_points_to_win_steigered = n_points_to_win + match ebid {
                EBid::AtLeast => 0,
                EBid::Higher => n_step,
            };
            if n_points_to_win_steigered<=120 {
                let mut payoutdecider = self.clone();
                payoutdecider.pointstowin = SoloSteigern{n_points_to_win: n_points_to_win_steigered, n_step};
                return Some(payoutdecider)
            }
        }
        None
    }
}

impl Display for SPayoutDeciderPointBased<VGameAnnouncementPrioritySoloLike> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use self::VGameAnnouncementPrioritySoloLike::*;
        match self.pointstowin {
            SoloSimple(_) => Ok(()), // no special indication required
            SoloSteigern{n_points_to_win, ..} => {
                assert!(61<=n_points_to_win);
                if n_points_to_win<61 {
                    write!(f, "for {}", n_points_to_win)
                } else {
                    Ok(())
                }
            },
        }
    }
}

#[derive(Clone, Debug)]
pub struct SPayoutDeciderTout {
    payoutparams : SPayoutDeciderParams,
    i_prio: isize,
}

impl TPayoutDecider for SPayoutDeciderTout {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        &self,
        rules: &Rules,
        gamefinishedstiche: SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        // TODORULES optionally count schneider/schwarz
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| fn_is_player_party(rules.winner_index(stich)));
        let ab_winner = EPlayerIndex::map_from_fn(|epi| {
            fn_is_player_party(epi)==b_player_party_wins
        });
        internal_payout(
            /*n_payout_single_player*/ (self.payoutparams.n_payout_base + self.payoutparams.laufendeparams.payout_laufende(rules, gamefinishedstiche, &ab_winner)) * 2,
            fn_player_multiplier,
            &ab_winner,
        )
    }
}

impl Display for SPayoutDeciderTout {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderTout {
    type PrioParams = isize;
    type PayoutParams = SPayoutDeciderParams;

    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloTout(self.i_prio)
    }

    fn new(payoutparams: Self::PayoutParams, i_prio: isize) -> Self {
        SPayoutDeciderTout {
            payoutparams,
            i_prio,
        }
    }
}

#[derive(Clone, Debug)]
pub struct SPayoutDeciderSie {
    payoutparams : SPayoutDeciderParams,
}

impl TPayoutDecider for SPayoutDeciderSie {
    fn payout<FnIsPlayerParty, FnPlayerMultiplier, Rules>(
        &self,
        rules: &Rules,
        gamefinishedstiche: SGameFinishedStiche,
        fn_is_player_party: FnIsPlayerParty,
        fn_player_multiplier: FnPlayerMultiplier,
    ) -> EnumMap<EPlayerIndex, isize>
        where FnIsPlayerParty: Fn(EPlayerIndex)->bool,
              FnPlayerMultiplier: Fn(EPlayerIndex)->isize,
              Rules: TRules,
    {
        // TODORULES optionally count schneider/schwarz
        let b_player_party_wins = gamefinishedstiche.get().iter()
            .all(|stich| {
                let epi_stich_winner = rules.winner_index(stich);
                rules.trumpforfarbe(stich[epi_stich_winner]).is_trumpf() && fn_is_player_party(epi_stich_winner)
            })
            && match EKurzLang::from_cards_per_player(gamefinishedstiche.get().len()) {
                EKurzLang::Lang => true,
                EKurzLang::Kurz => {
                    gamefinishedstiche.get().iter()
                        .all(|stich| {
                            let epi_stich_winner = rules.winner_index(stich);
                            let card = stich[epi_stich_winner];
                            assert!(rules.trumpforfarbe(card).is_trumpf());
                            card.schlag()==ESchlag::Ober || {
                                assert_eq!(card.schlag(), ESchlag::Unter);
                                card.farbe()==EFarbe::Eichel || card.farbe()==EFarbe::Gras
                            }
                        })
                },
            }
        ;
        internal_payout(
            /*n_payout_single_player*/ (self.payoutparams.n_payout_base
            + {
                gamefinishedstiche.get().len().as_num::<isize>()
            } * self.payoutparams.laufendeparams.n_payout_per_lauf) * 4,
            fn_player_multiplier,
            /*ab_winner*/ &EPlayerIndex::map_from_fn(|epi| {
                fn_is_player_party(epi)==b_player_party_wins
            })
        )
    }
}

impl Display for SPayoutDeciderSie {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "")
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderSie {
    type PrioParams = ();
    type PayoutParams = SPayoutDeciderParams;

    fn new(payoutparams: Self::PayoutParams, _prioparams: ()) -> Self {
        SPayoutDeciderSie {
            payoutparams,
        }
    }

    fn priority(&self) -> VGameAnnouncementPriority {
        VGameAnnouncementPriority::SoloSie
    }
}

#[derive(Clone, Debug)]
pub struct SRulesSoloLike<TrumpfDecider, PayoutDecider>
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDeciderSoloLike,
{
    pub str_name: String,
    pub epi : EPlayerIndex,
    pub trumpfdecider : PhantomData<TrumpfDecider>,
    payoutdecider: PayoutDecider,
}

impl<TrumpfDecider, PayoutDecider> fmt::Display for SRulesSoloLike<TrumpfDecider, PayoutDecider> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDeciderSoloLike,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.str_name, self.payoutdecider)
    }
}

impl<TrumpfDecider, PayoutDecider> TActivelyPlayableRules for SRulesSoloLike<TrumpfDecider, PayoutDecider>
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDeciderSoloLike,
{
    box_clone_impl_by_clone!(TActivelyPlayableRules);
    fn priority(&self) -> VGameAnnouncementPriority {
        self.payoutdecider.priority()
    }
    fn with_increased_prio(&self, prio: &VGameAnnouncementPriority, ebid: EBid) -> Option<Box<TActivelyPlayableRules>> {
        self.payoutdecider.with_increased_prio(prio, ebid)
            .map(|payoutdecider| Box::new(Self::internal_new(self.epi, &self.str_name, payoutdecider)) as Box<TActivelyPlayableRules>)
    }
}

impl<TrumpfDecider, PayoutDecider> TRules for SRulesSoloLike<TrumpfDecider, PayoutDecider> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDeciderSoloLike,
{
    box_clone_impl_by_clone!(TRules);
    impl_rules_trumpf!(TrumpfDecider);
    impl_single_play!();
}

impl<TrumpfDecider, PayoutDecider> SRulesSoloLike<TrumpfDecider, PayoutDecider>
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDeciderSoloLike,
{
    fn internal_new(epi: EPlayerIndex, str_rulename: &str, payoutdecider: PayoutDecider) -> SRulesSoloLike<TrumpfDecider, PayoutDecider> {
        SRulesSoloLike::<TrumpfDecider, PayoutDecider> {
            epi,
            trumpfdecider: PhantomData::<TrumpfDecider>,
            payoutdecider,
            str_name: str_rulename.to_string(),
        }
    }
    pub fn new(epi: EPlayerIndex, prioparams: PayoutDecider::PrioParams, str_rulename: &str, payoutparams: PayoutDecider::PayoutParams) -> SRulesSoloLike<TrumpfDecider, PayoutDecider> {
        Self::internal_new(epi, str_rulename, PayoutDecider::new(payoutparams, prioparams))
    }
}

pub fn sololike<TrumpfDecider, PayoutDecider>(epi: EPlayerIndex, prioparams: PayoutDecider::PrioParams, str_rulename: &str, payoutparams: PayoutDecider::PayoutParams) -> Box<TActivelyPlayableRules> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDeciderSoloLike,
{
    Box::new(SRulesSoloLike::<TrumpfDecider, PayoutDecider>::new(epi, prioparams, str_rulename, payoutparams)) as Box<TActivelyPlayableRules>
}

pub type SCoreSolo<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SStaticSchlagOber, STrumpfDeciderSchlag<
    SStaticSchlagUnter, TrumpfFarbDecider>>;
pub type SCoreGenericWenz<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SStaticSchlagUnter, TrumpfFarbDecider>;
pub type SCoreGenericGeier<TrumpfFarbDecider> = STrumpfDeciderSchlag<
    SStaticSchlagOber, TrumpfFarbDecider>;

#[test]
fn test_trumpfdecider() {
    use card::card_values::*;
    assert_eq!(
        <SCoreSolo<STrumpfDeciderFarbe<SStaticFarbeGras>> as TTrumpfDecider>
            ::trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EO, GO, HO, SO, EU, GU, HU, SU, GA, GZ, GK, G9, G8, G7],
    );
    assert_eq!(
        <SCoreGenericWenz<STrumpfDeciderFarbe<SStaticFarbeGras>> as TTrumpfDecider>
            ::trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EU, GU, HU, SU, GA, GZ, GK, GO, G9, G8, G7],
    );
    assert_eq!(
        <SCoreGenericWenz<STrumpfDeciderNoTrumpf> as TTrumpfDecider>
            ::trumpfs_in_descending_order().collect::<Vec<_>>(),
        vec![EU, GU, HU, SU],
    );
}
