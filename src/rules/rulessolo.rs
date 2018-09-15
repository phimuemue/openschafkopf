use primitives::*;
use rules::{
    *,
    trumpfdecider::*,
    payoutdecider::*,
};
use std::{
    fmt::self,
    cmp::Ordering,
    marker::PhantomData,
};
use util::*;

pub trait TPayoutDeciderSoloLike : TPayoutDecider {
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
            VGameAnnouncementPrioritySoloLike::SoloSimple(_) => 61,
            VGameAnnouncementPrioritySoloLike::SoloSteigern{n_points_to_win, n_step: _n_step} => *n_points_to_win,
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
    fn payout<Rules, PlayerParties>(
        &self,
        rules: &Rules,
        gamefinishedstiche: SGameFinishedStiche,
        playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, isize>
        where PlayerParties: TPlayerParties,
              Rules: TRulesNoObj,
    {
        // TODORULES optionally count schneider/schwarz
        internal_payout(
            /*n_payout_single_player*/ (self.payoutparams.n_payout_base + self.payoutparams.laufendeparams.payout_laufende::<Rules, _>(gamefinishedstiche, playerparties)) * 2,
            playerparties,
            /*b_primary_party_wins*/ gamefinishedstiche.get().iter()
                .all(|stich| playerparties.is_primary_party(rules.winner_index(stich))),
        )
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

impl TPayoutDecider for SPayoutDeciderSie {
    fn payout<Rules, PlayerParties>(
        &self,
        rules: &Rules,
        gamefinishedstiche: SGameFinishedStiche,
        playerparties: &PlayerParties,
    ) -> EnumMap<EPlayerIndex, isize>
        where PlayerParties: TPlayerParties,
              Rules: TRulesNoObj,
    {
        // TODORULES optionally count schneider/schwarz
        internal_payout(
            /*n_payout_single_player*/ (self.payoutparams.n_payout_base
            + {
                gamefinishedstiche.get().len().as_num::<isize>()
            } * self.payoutparams.laufendeparams.n_payout_per_lauf) * 4,
            playerparties,
            /*b_primary_party_wins*/ gamefinishedstiche.get().iter()
                .all(|stich| {
                    let epi_stich_winner = rules.winner_index(stich);
                    rules.trumpforfarbe(stich[epi_stich_winner]).is_trumpf() && playerparties.is_primary_party(epi_stich_winner)
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
        )
    }
}

impl TPayoutDeciderSoloLike for SPayoutDeciderSie {
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
        write!(f, "{}{}", self.str_name, self.payoutdecider.priorityinfo())
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

impl<TrumpfDecider, PayoutDecider> TRulesNoObj for SRulesSoloLike<TrumpfDecider, PayoutDecider> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDeciderSoloLike,
{
    impl_rules_trumpf_noobj!(TrumpfDecider);
}

impl<TrumpfDecider, PayoutDecider> TRules for SRulesSoloLike<TrumpfDecider, PayoutDecider> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDeciderSoloLike,
{
    box_clone_impl_by_clone!(TRules);
    impl_rules_trumpf!();
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
    pub fn new(epi: EPlayerIndex, payoutdecider: PayoutDecider, str_rulename: &str) -> SRulesSoloLike<TrumpfDecider, PayoutDecider> {
        Self::internal_new(epi, str_rulename, payoutdecider)
    }
}

pub fn sololike<TrumpfDecider, PayoutDecider>(epi: EPlayerIndex, payoutdecider: PayoutDecider, str_rulename: &str) -> Box<TActivelyPlayableRules> 
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDeciderSoloLike,
{
    Box::new(SRulesSoloLike::<TrumpfDecider, PayoutDecider>::new(epi, payoutdecider, str_rulename)) as Box<TActivelyPlayableRules>
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
