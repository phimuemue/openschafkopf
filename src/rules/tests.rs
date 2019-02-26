use crate::primitives::{
    *,
    card::card_values::*
};
use crate::rules::{
    *,
    rulesrufspiel::*,
    rulessolo::*,
    rulesramsch::*,
    rulesbettel::*,
    payoutdecider::*,
};
use crate::util::*;
use crate::game::*;
use crate::game_analysis::*;

fn internal_test_rules(
    str_info: &str,
    rules: &dyn TRules,
    ahand: EnumMap<EPlayerIndex, SHand>,
    vecn_doubling: Vec<usize>,
    vecn_stoss: Vec<usize>,
    n_stock: isize,
    slcstich_test: &[SStich],
    (an_payout, n_stock_payout): ([isize; 4], isize),
) {
    println!("Testing rules: {}", str_info);
    let game = analyze_game_internal(
        /*epi_first*/EPlayerIndex::EPI0, // TODO parametrize w.r.t. epi_first
        rules,
        ahand,
        vecn_doubling,
        vecn_stoss,
        n_stock,
        slcstich_test,
        /*fn_before_zugeben*/|_game, _i_stich, _epi, _card| {},
    );
    let an_payout_check = debug_verify!(game.finish()).unwrap().an_payout;
    assert_eq!(EPlayerIndex::map_from_fn(|epi| an_payout_check[epi]), EPlayerIndex::map_from_raw(an_payout));
    assert_eq!(-an_payout.iter().sum::<isize>(), n_stock_payout);
}

pub trait TCardArrayKurzLang {
    fn to_hand(&self) -> SHand; // TODO take self instead of &self
}
impl TCardArrayKurzLang for [SCard; 6] {
    fn to_hand(&self) -> SHand {
        SHand::new_from_vec(self.iter().cloned().collect())
    }
}
impl TCardArrayKurzLang for [SCard; 8] {
    fn to_hand(&self) -> SHand {
        SHand::new_from_vec(self.iter().cloned().collect())
    }
}

pub fn test_rules<CardArrayKurzLang: TCardArrayKurzLang>(
    str_info: &str,
    rules: &dyn TRules,
    aacard_hand: [CardArrayKurzLang; 4],
    vecn_doubling: Vec<usize>,
    vecn_stoss: Vec<usize>,
    vecpairnacard_stich: &[(usize, [SCard; 4])],
    an_payout: [isize; 4],
) {
    internal_test_rules(
        str_info,
        rules,
        EPlayerIndex::map_from_raw(aacard_hand)
            .map(TCardArrayKurzLang::to_hand),
        vecn_doubling,
        vecn_stoss,
        /*n_stock*/0,
        &make_stich_vector(vecpairnacard_stich),
        (an_payout, 0),
    );
}

pub fn test_rules_manual(
    str_info: &str,
    rules: &dyn TRules,
    vecn_doubling: Vec<usize>,
    vecn_stoss: Vec<usize>,
    n_stock: isize,
    vecpairnacard_stich: &[(usize, [SCard; 4])],
    (an_payout, n_stock_payout): ([isize; 4], isize),
) {
    let vecstich = make_stich_vector(vecpairnacard_stich);
    internal_test_rules(
        str_info,
        rules,
        EPlayerIndex::map_from_fn(|epi|
            SHand::new_from_vec(vecstich.iter().map(|stich| stich[epi]).collect())
        ),
        vecn_doubling,
        vecn_stoss,
        n_stock,
        &vecstich,
        (an_payout, n_stock_payout),
    );
}

fn rulesrufspiel_new_test(epi: EPlayerIndex, efarbe: EFarbe, n_payout_base: isize, n_payout_schneider_schwarz: isize, laufendeparams: SLaufendeParams) -> SRulesRufspiel {
    // Do not inline this function into SRulesRufspiel. It serves as a bridge between actual implementation and the data we extract for the test suite.
    SRulesRufspiel::new(
        epi,
        efarbe,
        SPayoutDeciderParams::new(
            n_payout_base,
            n_payout_schneider_schwarz,
            laufendeparams,
        ),
    )
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

#[test]
fn test_rulesrufspiel() {
    test_rules(
        "../../testdata/games/10.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        [[HO,SO,GU,SU,EK,GA,S9,S7],[GO,HK,H8,H7,EA,SA,SK,S8],[EU,HU,HA,EZ,E7,GZ,G9,G8],[EO,HZ,H9,E9,E8,GK,G7,SZ],],
        vec![],
        vec![],
        &[(0, [SU,GO,HU,H9]),(1, [H8,HA,EO,GU]),(3, [E8,EK,EA,E7]),(1, [SA,EU,SZ,S7]),(2, [G9,G7,GA,SK]),(0, [HO,H7,G8,HZ]),(0, [SO,HK,EZ,E9]),(0, [S9,S8,GZ,GK]),],
        [20, 20, -20, -20],
    );
    test_rules(
        "../../testdata/games/14.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        [[HO,HU,HA,H8,EZ,E9,SA,S9],[EU,H7,E8,GK,G9,G7,SK,S8],[GO,SO,GU,HZ,H9,E7,GA,G8],[EO,SU,HK,EA,EK,GZ,SZ,S7],],
        vec![],
        vec![],
        &[(0, [H8,H7,GU,EO]),(3, [HK,HU,EU,HZ]),(1, [E8,E7,EA,EZ]),(3, [SU,HO,SK,GO]),(2, [SO,S7,HA,GK]),(2, [GA,GZ,E9,G7]),(2, [G8,SZ,S9,G9]),(1, [S8,H9,EK,SA]),],
        [-30, 30, 30, -30],
    );
    test_rules(
        "../../testdata/games/16.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        [[GU,SU,HK,E9,E8,E7,GA,SZ],[SO,HU,HZ,H8,EZ,G9,G8,S7],[EO,GO,HA,H9,H7,SK,S9,S8],[HO,EU,EA,EK,GZ,GK,G7,SA],],
        vec![],
        vec![],
        &[(0, [SZ,S7,S8,SA]),(3, [HO,HK,H8,HA]),(3, [EU,SU,SO,GO]),(2, [EO,GZ,GU,HZ]),(2, [H7,EK,E7,HU]),(1, [G8,H9,GK,GA]),(2, [SK,G7,E8,G9]),(2, [S9,EA,E9,EZ]),],
        [-60, -60, 60, 60],
    );
    test_rules(
        "../../testdata/games/19.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI1, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[GO,GU,GA,GZ,G9,SZ,S9,S8],[HO,HU,HK,H8,H7,GK,SA,S7],[EO,SO,EU,HA,EA,E9,E8,SK],[SU,HZ,H9,EZ,EK,E7,G8,G7],],
        vec![],
        vec![],
        &[(0, [GO,H7,EO,HZ]),(2, [SK,SU,S8,S7]),(3, [G7,GA,GK,HA]),(2, [EA,E7,GU,SA]),(0, [G9,HU,E9,G8]),(1, [HO,EU,H9,GZ]),(1, [H8,SO,EZ,S9]),(2, [E8,EK,SZ,HK]),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/2.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[GU,SU,HZ,H7,EK,GA,GK,S7],[HK,EZ,E9,E8,E7,G7,S9,S8],[GO,SO,HU,HA,EA,GZ,SA,SK],[EO,HO,EU,H9,H8,G9,G8,SZ],],
        vec![],
        vec![3,0,],
        &[(0, [GU,HK,HU,EU]),(3, [G9,GA,G7,GZ]),(0, [H7,S8,SO,H8]),(2, [EA,H9,EK,EZ]),(3, [G8,GK,S9,HA]),(2, [SA,SZ,S7,E7]),(2, [SK,HO,SU,E8]),(3, [EO,HZ,E9,GO]),],
        [-80, 80, -80, 80],
    );
    test_rules(
        "../../testdata/games/21.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        [[HK,H7,EZ,E9,E7,GK,G8,SK],[GO,GU,EA,EK,E8,GZ,SA,S7],[HO,SO,EU,HZ,H8,G7,SZ,S8],[EO,HU,SU,HA,H9,GA,G9,S9],],
        vec![],
        vec![],
        &[(0, [SK,SA,S8,S9]),(1, [GO,H8,HA,HK]),(1, [GU,EU,EO,H7]),(3, [SU,EZ,E8,SO]),(2, [G7,GA,G8,GZ]),(3, [G9,GK,S7,SZ]),(0, [E7,EA,HO,H9]),(2, [HZ,HU,E9,EK]),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/22.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[EO,EU,HU,HA,HK,G7,SZ,S8],[HO,EZ,E9,E7,GZ,GK,G9,SA],[GO,SO,HZ,H9,EK,E8,SK,S9],[GU,SU,H8,H7,EA,GA,G8,S7],],
        vec![],
        vec![],
        &[(0, [HK,HO,H9,H7]),(1, [GZ,HZ,GA,G7]),(2, [E8,EA,SZ,E7]),(3, [GU,HU,EZ,SO]),(2, [S9,S7,S8,SA]),(1, [GK,GO,G8,EO]),(0, [EU,G9,EK,H8]),(0, [HA,E9,SK,SU]),],
        [-20, 20, 20, -20],
    );
    test_rules(
        "../../testdata/games/26.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        [[EO,HO,HZ,H9,H8,GA,GK,S8],[HU,SU,HK,EA,E7,GZ,SA,SZ],[GO,SO,EU,GU,E8,G9,S9,S7],[HA,H7,EZ,EK,E9,G8,G7,SK],],
        vec![],
        vec![],
        &[(0, [H8,SU,EU,HA]),(2, [S7,SK,S8,SA]),(1, [EA,E8,E9,GA]),(1, [E7,G9,EK,HZ]),(0, [GK,GZ,GU,G7]),(2, [S9,H7,H9,SZ]),(0, [EO,HK,SO,G8]),(0, [HO,HU,GO,EZ]),],
        [20, 20, -20, -20],
    );
    test_rules(
        "../../testdata/games/29.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        [[SO,SU,HA,HZ,H7,EK,E9,SZ],[GO,HK,H8,E7,GZ,G9,SA,S7],[EO,EU,H9,EZ,GA,GK,SK,S8],[HO,GU,HU,EA,E8,G8,G7,S9],],
        vec![],
        vec![],
        &[(0, [H7,GO,H9,HU]),(1, [E7,EZ,EA,EK]),(3, [HO,SU,H8,EO]),(2, [EU,GU,SO,HK]),(0, [E9,GZ,S8,E8]),(0, [SZ,SA,SK,S9]),(1, [G9,GA,G7,HA]),(0, [HZ,S7,GK,G8]),],
        [20, -20, -20, 20],
    );
    test_rules(
        "../../testdata/games/30.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI1, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[HA,EA,EZ,E8,E7,GA,GK,SK],[EU,HU,SU,HZ,H8,H7,G9,G7],[EO,HO,GU,HK,E9,GZ,SA,S9],[GO,SO,H9,EK,G8,SZ,S8,S7],],
        vec![],
        vec![],
        &[(0, [HA,H7,HO,H9]),(2, [SA,S8,SK,HZ]),(1, [H8,HK,SO,E7]),(3, [G8,GA,G7,GZ]),(0, [GK,G9,S9,EK]),(0, [E8,HU,E9,GO]),(3, [SZ,EA,SU,GU]),(2, [EO,S7,EZ,EU]),],
        [-60, -60, 60, 60],
    );
    test_rules(
        "../../testdata/games/31.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        [[EO,HZ,H9,E8,GA,GK,SK,S9],[GU,HU,EZ,EK,E7,GZ,G9,S7],[HO,SO,SU,HA,E9,G8,G7,SA],[GO,EU,HK,H8,H7,EA,SZ,S8],],
        vec![],
        vec![],
        &[(0, [SK,S7,SA,SZ]),(2, [HO,H7,EO,HU]),(0, [GA,GZ,G7,HK]),(3, [GO,H9,GU,HA]),(3, [EA,E8,E7,E9]),(3, [S8,S9,G9,G8]),(0, [GK,EK,SU,H8]),(2, [SO,EU,HZ,EZ]),],
        [-30, -30, 30, 30],
    );
    test_rules(
        "../../testdata/games/32.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[EO,GO,HO,HZ,H7,E9,GZ,SA],[SO,GU,HU,HK,H8,EA,EZ,G7],[SU,E8,GK,G9,G8,SK,S9,S7],[EU,HA,H9,EK,E7,GA,SZ,S8],],
        vec![],
        vec![],
        &[(0, [EO,H8,SU,HA]),(0, [GO,HK,E8,H9]),(0, [H7,SO,S9,EU]),(1, [G7,G8,GA,GZ]),(3, [EK,E9,EA,GK]),(1, [GU,G9,SZ,HO]),(0, [SA,HU,SK,S8]),(1, [EZ,S7,E7,HZ]),],
        [50, -50, -50, 50],
    );
    test_rules(
        "../../testdata/games/33.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[SO,EU,SU,H7,EK,G7,SK,S8],[EO,H9,EA,EZ,E9,E8,G8,S7],[GO,HO,GU,HA,HZ,G9,SA,SZ],[HU,HK,H8,E7,GA,GZ,GK,S9],],
        vec![],
        vec![],
        &[(0, [G7,G8,G9,GA]),(3, [HU,H7,EO,GU]),(1, [EA,HA,E7,EK]),(2, [GO,HK,SU,H9]),(2, [HO,H8,EU,S7]),(2, [SA,S9,S8,E8]),(2, [SZ,GK,SK,E9]),(2, [HZ,GZ,SO,EZ]),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/35.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[EO,SO,EU,H7,E7,GK,G9,G8],[GU,HU,EK,E8,GA,SZ,S9,S7],[GO,HO,SU,HZ,HK,H8,GZ,S8],[HA,H9,EA,EZ,E9,G7,SA,SK],],
        vec![],
        vec![],
        &[(0, [GK,GA,GZ,G7]),(1, [GU,H8,HA,EU]),(0, [E7,EK,HZ,E9]),(2, [HO,H9,H7,HU]),(2, [SU,EA,SO,E8]),(0, [G9,SZ,HK,SK]),(2, [S8,SA,G8,S7]),(3, [EZ,EO,S9,GO]),],
        [-20, 20, 20, -20],
    );
    test_rules(
        "../../testdata/games/36.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI1, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        [[SO,E9,GA,G9,G8,SA,SZ,S9],[EO,GU,HZ,H9,H8,H7,EZ,S7],[GO,EU,HU,SU,HK,EK,GK,SK],[HO,HA,EA,E8,E7,GZ,G7,S8],],
        vec![],
        vec![],
        &[(0, [E9,EZ,EK,EA]),(3, [HO,SO,H7,GO]),(2, [SK,S8,SA,S7]),(0, [GA,HZ,GK,GZ]),(1, [EO,SU,HA,G9]),(1, [H8,HU,G7,SZ]),(2, [HK,E7,S9,H9]),(2, [EU,E8,G8,GU]),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/38.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[SU,HA,EZ,E9,E7,GK,G9,S8],[GO,GU,HU,H9,G7,SZ,S9,S7],[EO,SO,EU,EK,GA,GZ,SA,SK],[HO,HZ,HK,H8,H7,EA,E8,G8],],
        vec![],
        vec![],
        &[(0, [GK,G7,GA,G8]),(2, [EO,HZ,SU,H9]),(2, [SO,H7,HA,GO]),(1, [S9,SA,E8,S8]),(2, [EU,HK,G9,HU]),(2, [EK,EA,E7,GU]),(1, [SZ,SK,H8,E9]),(3, [HO,EZ,S7,GZ]),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/40.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        [[GO,SO,EU,HA,H9,E9,SA,S9],[EO,HO,HZ,EK,E7,G9,G8,G7],[GU,HK,H8,EZ,GZ,GK,SZ,SK],[HU,SU,H7,EA,E8,GA,S8,S7],],
        vec![],
        vec![],
        &[(0, [H9,HO,HK,H7]),(1, [EK,EZ,EA,E9]),(3, [HU,EU,EO,H8]),(1, [G7,GK,GA,S9]),(3, [SU,SO,HZ,GU]),(0, [GO,E7,GZ,E8]),(0, [HA,G8,SK,S7]),(0, [SA,G9,SZ,S8]),],
        [30, -30, -30, 30],
    );
    test_rules(
        "../../testdata/games/41.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        [[EO,GU,HU,EZ,GZ,G8,G7,SZ],[SU,HK,H9,EA,E7,G9,S9,S8],[SO,EU,HA,H8,EK,E9,SK,S7],[GO,HO,HZ,H7,E8,GA,GK,SA],],
        vec![],
        vec![],
        &[(0, [EZ,EA,E9,E8]),(1, [H9,EU,H7,HU]),(2, [SK,SA,SZ,S8]),(3, [HO,EO,HK,H8]),(0, [GZ,G9,SO,GA]),(2, [EK,HZ,GU,E7]),(0, [G7,SU,S7,GK]),(1, [S9,HA,GO,G8]),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/43.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[HZ,H9,EA,EK,G9,SZ,SK,S9],[EO,SU,HA,H8,GA,GZ,SA,S7],[SO,GU,H7,E9,E7,GK,G8,S8],[GO,HO,EU,HU,HK,EZ,E8,G7],],
        vec![],
        vec![],
        &[(0, [G9,GA,G8,G7]),(1, [EO,H7,HK,H9]),(1, [SA,S8,EZ,S9]),(1, [H8,GU,HO,HZ]),(3, [E8,EA,HA,E7]),(1, [SU,SO,GO,EK]),(3, [EU,SK,GZ,E9]),(3, [HU,SZ,S7,GK]),],
        [-70, 70, -70, 70],
    );
    test_rules(
        "../../testdata/games/45.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        [[HO,HU,SU,H9,E9,E8,GK,SK],[HA,HZ,H7,EA,EZ,GA,SA,S9],[SO,EU,GU,EK,E7,GZ,G9,S7],[EO,GO,HK,H8,G8,G7,SZ,S8],],
        vec![],
        vec![],
        &[(0, [SK,SA,S7,SZ]),(1, [HA,GU,GO,H9]),(3, [G8,GK,GA,G9]),(1, [EA,E7,G7,E8]),(1, [EZ,EK,EO,E9]),(3, [S8,HU,S9,GZ]),(0, [SU,HZ,SO,H8]),(2, [EU,HK,HO,H7]),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/46.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        [[HO,SO,EU,HU,H8,GA,G8,S8],[GO,HA,HZ,H7,EA,EK,S9,S7],[GU,SU,H9,E9,GZ,G9,G7,SZ],[EO,HK,EZ,E8,E7,GK,SA,SK],],
        vec![],
        vec![],
        &[(0, [H8,HA,SU,EO]),(3, [HK,EU,HZ,H9]),(0, [SO,GO,GU,E7]),(1, [EA,E9,EZ,HU]),(0, [HO,H7,G7,GK]),(0, [GA,EK,G9,SK]),(0, [S8,S7,SZ,SA]),(3, [E8,G8,S9,GZ]),],
        [30, -30, -30, 30],
    );
    test_rules(
        "../../testdata/games/47.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[HO,H9,E7,GA,GZ,G9,SZ,S9],[GO,SO,EU,EA,E9,G8,SK,S8],[GU,SU,HK,H8,H7,EZ,EK,E8],[EO,HU,HA,HZ,GK,G7,SA,S7],],
        vec![],
        vec![],
        &[(0, [HO,GO,H7,HU]),(1, [G8,HK,G7,GA]),(2, [EK,HA,E7,E9]),(3, [EO,H9,EU,H8]),(3, [SA,S9,S8,SU]),(2, [EZ,HZ,SZ,EA]),(3, [S7,G9,SK,E8]),(1, [SO,GU,GK,GZ]),],
        [20, -20, -20, 20],
    );
    test_rules(
        "../../testdata/games/48.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        [[EU,HA,H8,EA,E8,E7,GA,G7],[EO,HK,EZ,EK,SZ,S9,S8,S7],[GO,SO,GU,HU,H9,H7,E9,SA],[HO,SU,HZ,GZ,GK,G9,G8,SK],],
        vec![],
        vec![],
        &[(0, [HA,EO,H7,HZ]),(1, [EK,E9,SU,EA]),(3, [SK,H8,S7,SA]),(0, [EU,HK,H9,HO]),(3, [G8,GA,S8,HU]),(2, [GO,G9,G7,S9]),(2, [SO,GK,E7,SZ]),(2, [GU,GZ,E8,EZ]),],
        [20, -20, 20, -20],
    );
    test_rules(
        "../../testdata/games/49.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI1, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[H9,H8,EZ,E8,G8,SK,S9,S8],[EO,SO,GU,HA,HK,E9,GK,S7],[EU,HU,SU,HZ,H7,GZ,G9,SZ],[GO,HO,EA,EK,E7,GA,G7,SA],],
        vec![],
        vec![],
        &[(0, [G8,GK,G9,GA]),(3, [GO,H8,HA,H7]),(3, [HO,H9,HK,SU]),(3, [SA,S9,S7,SZ]),(3, [EA,EZ,E9,HZ]),(2, [EU,EK,SK,SO]),(1, [GU,HU,E7,S8]),(1, [EO,GZ,G7,E8]),],
        [-60, 60, -60, 60],
    );
    test_rules(
        "../../testdata/games/5.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        [[GO,SO,EU,H9,EZ,GZ,SA,S9],[HU,SU,HA,HZ,E9,E7,G9,S8],[HK,H8,EK,E8,G7,SZ,SK,S7],[EO,HO,GU,H7,EA,GA,GK,G8],],
        vec![],
        vec![],
        &[(0, [H9,SU,HK,GU]),(3, [EO,SO,HU,H8]),(3, [HO,EU,HZ,G7]),(3, [H7,GO,HA,S7]),(0, [SA,S8,SK,GK]),(0, [EZ,E7,E8,EA]),(3, [GA,GZ,G9,EK]),(3, [G8,S9,E9,SZ]),],
        [100, -100, -100, 100],
    );
    test_rules(
        "../../testdata/games/50.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        [[HO,SO,GU,GK,G9,SA,S9,S7],[HA,EA,EZ,EK,E9,GZ,G8,G7],[HU,HK,H9,H7,E8,E7,GA,SZ],[EO,GO,EU,SU,HZ,H8,SK,S8],],
        vec![],
        vec![],
        &[(0, [HO,HA,H7,HZ]),(0, [SO,E9,H9,H8]),(0, [GU,G7,HK,SU]),(0, [SA,EK,SZ,S8]),(0, [S9,EZ,HU,SK]),(2, [GA,EU,GK,G8]),(3, [GO,G9,GZ,E7]),(3, [EO,S7,EA,E8]),],
        [90, -90, -90, 90],
    );
    test_rules(
        "../../testdata/games/51.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[EU,SU,H9,EA,GZ,G9,SZ,SK],[EO,HO,GU,H7,EZ,G8,G7,S8],[GO,EK,E9,E7,GA,SA,S9,S7],[SO,HU,HA,HZ,HK,H8,E8,GK],],
        vec![],
        vec![],
        &[(0, [G9,G7,GA,GK]),(2, [GO,H8,H9,EO]),(1, [G8,E7,HK,GZ]),(3, [E8,EA,EZ,E9]),(0, [SK,S8,SA,HA]),(3, [HU,EU,H7,S7]),(0, [SZ,HO,S9,SO]),(1, [GU,EK,HZ,SU]),],
        [20, 20, -20, -20],
    );
    test_rules(
        "../../testdata/games/53.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI1, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        [[SO,EZ,EK,E7,GA,SZ,SK,S8],[GO,HU,HA,HZ,HK,H8,E9,GK],[EO,GU,H9,H7,G8,SA,S9,S7],[HO,EU,SU,EA,E8,GZ,G9,G7],],
        vec![],
        vec![],
        &[(0, [SO,H8,H7,HO]),(3, [GZ,GA,GK,G8]),(0, [SK,HA,S7,SU]),(3, [G7,S8,HK,S9]),(1, [HU,GU,EU,E7]),(3, [EA,EZ,E9,SA]),(3, [E8,EK,HZ,EO]),(2, [H9,G9,SZ,GO]),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/55.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        [[SU,HK,H7,EK,GA,GZ,G9,G7],[GO,SO,EU,EZ,E8,GK,G8,SZ],[HO,GU,HU,HA,HZ,H8,E7,S7],[EO,H9,EA,E9,SA,SK,S9,S8],],
        vec![],
        vec![],
        &[(0, [EK,E8,E7,EA]),(3, [EO,H7,EU,HA]),(3, [H9,SU,GO,H8]),(1, [EZ,HU,E9,G7]),(2, [HO,SA,HK,SO]),(2, [GU,SK,GZ,G8]),(2, [S7,S8,G9,SZ]),(1, [GK,HZ,S9,GA]),],
        [-30, -30, 30, 30],
    );
    test_rules(
        "../../testdata/games/6.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        [[EO,GO,SO,HA,HK,EK,GZ,G9],[SU,H9,E9,E8,GK,G7,S9,S8],[HO,EU,HU,H8,EZ,E7,GA,SZ],[GU,HZ,H7,EA,G8,SA,SK,S7],],
        vec![],
        vec![],
        &[(0, [EO,H9,H8,HZ]),(0, [HK,SU,EU,H7]),(2, [E7,EA,EK,E9]),(3, [GU,GO,GK,HU]),(0, [G9,G7,GA,G8]),(2, [HO,S7,HA,E8]),(2, [EZ,SK,SO,S9]),(0, [GZ,S8,SZ,SA]),],
        [20, -20, -20, 20],
    );
}

#[test]
fn test_rulesfarbwenz() {
    test_rules(
        "../../testdata/games/11.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Gras, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GA,G9,EZ,E8,HZ,H9,H8,SK],[EK,EO,E9,E7,HA,HK,SZ,S8],[SU,GK,GO,G7,EA,H7,SO,S9],[EU,GU,HU,GZ,G8,HO,SA,S7],],
        vec![],
        vec![],
        &[(0, [SK,S8,S9,SA]),(3, [GU,GA,EO,G7]),(3, [HU,G9,E7,GO]),(3, [EU,H8,HK,GK]),(3, [G8,HZ,SZ,SU]),(2, [EA,GZ,E8,E9]),(3, [S7,EZ,HA,SO]),(2, [H7,HO,H9,EK]),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/12.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Herz, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,HA,HZ,H9,H7,GA,GZ,G8],[HU,EK,EO,E9,GO,G9,G7,SK],[SU,HO,EA,EZ,E8,E7,S9,S7],[GU,HK,H8,GK,SA,SZ,SO,S8],],
        vec![],
        vec![],
        &[(0, [H7,HU,HO,HK]),(1, [SK,S7,SZ,HA]),(0, [EU,G7,SU,H8]),(0, [H9,GO,EA,GU]),(3, [GK,GA,G9,S9]),(0, [HZ,E9,E7,S8]),(0, [GZ,EO,E8,SO]),(0, [G8,EK,EZ,SA]),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/15.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Herz, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HK,EZ,E8,GK,G7,SK,SO,S9],[GU,HA,HZ,HO,H7,EK,SA,S8],[EU,H8,EO,E9,E7,GA,G9,S7],[HU,SU,H9,EA,GZ,GO,G8,SZ],],
        vec![],
        vec![],
        &[(0, [SK,SA,S7,SZ]),(1, [H7,H8,SU,HK]),(3, [EA,EZ,EK,EO]),(3, [GO,G7,HA,G9]),(1, [HO,EU,H9,GK]),(2, [E9,HU,E8,S8]),(3, [G8,S9,HZ,GA]),(1, [GU,E7,GZ,SO]),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/17.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SU,GA,GZ,GO,G8,EA,EK,HA],[EU,GU,GK,G7,E8,HK,SO,S7],[HU,G9,HO,H8,H7,SA,S9,S8],[EZ,EO,E9,E7,HZ,H9,SZ,SK],],
        vec![],
        vec![],
        &[(0, [G8,GK,G9,EZ]),(1, [HK,H7,H9,HA]),(0, [SU,GU,HU,HZ]),(1, [E8,HO,E9,EA]),(0, [GO,G7,H8,E7]),(0, [EK,S7,S8,EO]),(0, [GZ,EU,SA,SZ]),(1, [SO,S9,SK,GA]),],
        [-240, 80, 80, 80],
    );
    test_rules(
        "../../testdata/games/23.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Gras, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,HU,G9,EK,E9,E8,E7,HK],[GO,EZ,HA,HZ,H9,H7,SZ,S8],[EU,GA,GZ,GK,G7,EA,SA,S9],[SU,G8,EO,HO,H8,SK,SO,S7],],
        vec![],
        vec![],
        &[(0, [E9,EZ,EA,EO]),(2, [G7,G8,G9,GO]),(1, [HZ,GA,H8,HK]),(2, [EU,SU,HU,H7]),(2, [GK,SK,GU,HA]),(0, [EK,SZ,GZ,S7]),(2, [SA,SO,E7,S8]),(2, [S9,HO,E8,H9]),],
        [-60, -60, 180, -60],
    );
    test_rules(
        "../../testdata/games/25.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,HU,GA,GK,G7,HA,H7,SA],[EU,EA,EZ,E7,HO,H8,SZ,S8],[GZ,GO,G9,G8,EO,H9,S9,S7],[SU,EK,E9,E8,HZ,HK,SK,SO],],
        vec![],
        vec![],
        &[(0, [HU,EU,GZ,SU]),(1, [EA,EO,EK,G7]),(0, [GU,E7,G8,E8]),(0, [GK,EZ,G9,E9]),(0, [GA,H8,GO,SO]),(0, [HA,HO,H9,HK]),(0, [SA,S8,S7,SK]),(0, [H7,SZ,S9,HZ]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/37.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,HU,GA,G9,HZ,HK,SO,S9],[EA,GZ,G7,HO,H9,SK,S8,S7],[SU,EO,E9,GK,GO,G8,HA,H8],[EU,EZ,EK,E8,E7,H7,SA,SZ],],
        vec![],
        vec![],
        &[(0, [SO,SK,EO,SA]),(2, [HA,H7,HZ,HO]),(2, [H8,EK,HK,H9]),(3, [EU,HU,EA,E9]),(3, [E7,GU,GZ,SU]),(0, [GA,G7,G8,EZ]),(3, [SZ,S9,S7,GO]),(3, [E8,G9,S8,GK]),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/4.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Herz, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,HA,HZ,HK,H9,H7,EK,GA],[EU,HO,H8,GZ,G8,SA,S8,S7],[HU,EA,E9,GO,G9,SZ,SK,SO],[SU,EZ,EO,E8,E7,GK,G7,S9],],
        vec![],
        vec![],
        &[(0, [H7,HO,HU,SU]),(2, [GO,GK,GA,G8]),(0, [H9,H8,SZ,G7]),(0, [GU,EU,EA,EZ]),(1, [SA,SK,S9,HA]),(0, [HK,S7,G9,E7]),(0, [HZ,S8,E9,E8]),(0, [EK,GZ,SO,EO]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/54.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Herz, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,HA,HK,HO,H9,G9,SA,S9],[HU,H7,E7,GO,G8,G7,SO,S8],[GU,H8,EZ,E9,E8,GZ,SK,S7],[SU,HZ,EA,EK,EO,GA,GK,SZ],],
        vec![],
        vec![],
        &[(0, [EU,H7,H8,SU]),(0, [H9,HU,GU,HZ]),(2, [GZ,GA,G9,G7]),(3, [EA,HA,E7,E8]),(0, [S9,SO,SK,SZ]),(3, [GK,HK,G8,S7]),(0, [HO,S8,E9,EO]),(0, [SA,GO,EZ,EK]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/9.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Herz, ESoloLike::Wenz, SPayoutDeciderTout::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[H8,EZ,E9,E8,GA,GZ,GK,G7],[EU,GU,HU,SU,HA,HK,H9,SA],[HO,EA,EK,EO,SZ,SK,S8,S7],[HZ,H7,E7,GO,G9,G8,SO,S9],],
        vec![],
        vec![],
        &[(0, [GA,HA,HO,G8]),(1, [GU,S7,H7,H8]),(1, [EU,S8,HZ,E8]),(1, [HU,EO,E7,G7]),(1, [SU,EK,G9,E9]),(1, [HK,SK,GO,EZ]),(1, [H9,SZ,S9,GK]),(1, [SA,EA,SO,GZ]),],
        [-200, 600, -200, -200],
    );
    test_rules(
        "../../testdata/games/farbwenz/1.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Wenz, SPayoutDeciderTout::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,GU,HU,SU,GK,G9],[GZ,EK,E9,HZ,SO,S9],[GO,EA,EO,HK,H9,SZ],[GA,EZ,HA,HO,SA,SK],],
        vec![0,],
        vec![],
        &[(0, [EU,GZ,GO,GA]),(0, [GU,S9,H9,HO]),(0, [HU,SO,HK,SK]),(0, [SU,HZ,EO,EZ]),(0, [GK,E9,SZ,SA]),(0, [G9,EK,EA,HA]),],
        [1080, -360, -360, -360],
    );
    test_rules(
        "../../testdata/games/farbwenz/10.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,EA,EO,GO,G9,H9],[HU,GZ,HA,HZ,HO,S9],[EU,GA,GK,HK,SZ,SO],[SU,EZ,EK,E9,SA,SK],],
        vec![],
        vec![],
        &[(0, [H9,HA,HK,EZ]),(3, [E9,EA,HU,EU]),(2, [SZ,SA,GU,S9]),(0, [G9,GZ,GK,EK]),(3, [SK,GO,HO,SO]),(3, [SU,EO,HZ,GA]),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/farbwenz/2.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,EA,EO,GO,G9,H9],[HU,GZ,HA,HZ,HO,S9],[EU,GA,GK,HK,SZ,SO],[SU,EZ,EK,E9,SA,SK],],
        vec![],
        vec![],
        &[(0, [H9,HA,HK,EZ]),(3, [E9,EA,HU,EU]),(2, [SZ,SA,GU,S9]),(0, [G9,GZ,GK,EK]),(3, [SK,GO,HO,SO]),(3, [SU,EO,HZ,GA]),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/farbwenz/5.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Herz, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,HU,HA,EK,GZ,S9],[HZ,HK,HO,GA,GK,SA],[EA,EO,E9,G9,SK,SO],[GU,SU,H9,EZ,GO,SZ],],
        vec![1,],
        vec![],
        &[(0, [EK,HZ,E9,EZ]),(1, [HO,EA,SU,HA]),(3, [GO,GZ,GA,G9]),(1, [HK,SK,H9,HU]),(0, [S9,SA,SO,SZ]),(1, [GK,EO,GU,EU]),],
        [-200, 600, -200, -200],
    );
    test_rules(
        "../../testdata/games/farbwenz/7.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Gras, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SU,GZ,EA,HA,HZ,SZ],[HU,G9,EK,EO,E9,SO],[GA,EZ,HK,HO,SK,S9],[EU,GU,GK,GO,H9,SA],],
        vec![3,],
        vec![],
        &[(0, [SZ,SO,S9,SA]),(3, [GU,GZ,G9,GA]),(3, [EU,SU,HU,HO]),(3, [GO,HZ,EK,SK]),(3, [GK,HA,E9,HK]),(3, [H9,EA,EO,EZ]),],
        [-140, -140, -140, 420],
    );
    test_rules(
        "../../testdata/games/farbwenz/8.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Gras, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SU,GK,GO,EO,E8,H7,SZ,S8],[EU,GU,HU,E7,HZ,H8,S9,S7],[GZ,G9,G8,G7,EA,EK,HA,SA],[GA,EZ,E9,HK,HO,H9,SK,SO],],
        vec![3,1,],
        vec![1,],
        &[(0, [H7,H8,HA,H9]),(2, [G9,GA,GK,HU]),(1, [E7,EA,E9,E8]),(2, [G8,EZ,GO,GU]),(1, [EU,G7,HK,SU]),(1, [S7,SA,SO,S8]),(2, [GZ,SK,EO,S9]),(2, [EK,HO,SZ,HZ]),],
        [-800, -800, 2400, -800],
    );
    test_rules(
        "../../testdata/games/farbwenz/9.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,SU,EA,EK,E9,E8,E7,S7],[EU,HU,GK,G9,G8,SA,SO,S8],[GA,GO,HZ,HK,HO,H8,SZ,SK],[EZ,EO,GZ,G7,HA,H9,H7,S9],],
        vec![],
        vec![],
        &[(0, [SU,HU,HZ,EZ]),(1, [SA,SK,S9,S7]),(1, [G8,GA,G7,EA]),(0, [GU,EU,SZ,EO]),(1, [S8,HO,H7,EK]),(0, [E9,GK,HK,H9]),(0, [E8,SO,H8,GZ]),(0, [E7,G9,GO,HA]),],
        [150, -50, -50, -50],
    );
}

#[test]
fn test_ruleswenz() {
    test_rules(
        "../../testdata/games/13.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EU,HU,GA,GZ,HA,HZ,SA,S9],[GU,GO,G7,HK,H7,SK,SO,S8],[SU,EK,EO,GK,G9,G8,HO,SZ],[EA,EZ,E9,E8,E7,H9,H8,S7],],
        vec![],
        vec![],
        &[(0, [EU,GU,SU,S7]),(0, [HU,G7,G8,H8]),(0, [GA,GO,G9,H9]),(0, [GZ,H7,GK,E7]),(0, [HA,HK,HO,E8]),(0, [HZ,S8,EO,E9]),(0, [SA,SO,SZ,EZ]),(0, [S9,SK,EK,EA]),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/52.html",
        sololike(EPlayerIndex::EPI3, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EK,E9,E7,GK,GO,G8,HA,HK],[HU,EZ,EO,E8,HZ,H9,SZ,SO],[GZ,G9,G7,HO,H7,SK,S9,S8],[EU,GU,SU,EA,GA,H8,SA,S7],],
        vec![],
        vec![],
        &[(0, [HA,HZ,HO,H8]),(0, [GK,HU,GZ,GA]),(1, [H9,H7,S7,HK]),(0, [GO,E8,G7,SU]),(3, [GU,G8,EO,G9]),(3, [EU,E7,SO,S8]),(3, [EA,E9,EZ,S9]),(3, [SA,EK,SZ,SK]),],
        [-70, -70, -70, 210],
    );
    test_rules(
        "../../testdata/games/8.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[HU,SU,EA,EZ,E9,HZ,SA,S8],[GU,EK,G9,G8,SK,SO,S9,S7],[EU,GA,GK,G7,HO,H9,H7,SZ],[EO,E8,E7,GZ,GO,HA,HK,H8],],
        vec![],
        vec![],
        &[(0, [HU,GU,EU,GZ]),(2, [GA,GO,S8,G8]),(2, [GK,EO,HZ,G9]),(2, [G7,HA,SU,EK]),(0, [SA,S7,SZ,E7]),(0, [EA,S9,H7,E8]),(0, [EZ,SO,H9,H8]),(0, [E9,SK,HO,HK]),],
        [210, -70, -70, -70],
    );
    test_rules(
        "../../testdata/games/wenz/1.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[GU,SU,EA,HZ,SA,SZ],[EU,HU,GA,GZ,GK,S9],[HA,HK,HO,H9,SK,SO],[EZ,EK,EO,E9,GO,G9],],
        vec![1,3,],
        vec![1,],
        &[(0, [GU,EU,HA,EZ]),(1, [HU,HK,EK,SU]),(1, [GA,SK,GO,SZ]),(1, [GZ,SO,G9,SA]),(1, [GK,HO,EO,HZ]),(1, [S9,H9,E9,EA]),],
        [-1680, 560, 560, 560],
    );
    test_rules(
        "../../testdata/games/wenz/10.html",
        sololike(EPlayerIndex::EPI2, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EZ,EK,GK,GO,HO,H9,SZ,S7],[GU,EO,G8,G7,HZ,H8,SO,S8],[EA,E9,E8,GA,G9,HA,H7,SA],[EU,HU,SU,E7,GZ,HK,SK,S9],],
        vec![],
        vec![],
        &[(0, [EK,EO,EA,E7]),(2, [SA,S9,S7,S8]),(2, [HA,HK,H9,H8]),(2, [GA,GZ,GO,G7]),(2, [H7,SU,HO,HZ]),(3, [EU,SZ,GU,G9]),(3, [HU,EZ,SO,E8]),(3, [SK,GK,G8,E9]),],
        [-90, -90, 270, -90],
    );
    test_rules(
        "../../testdata/games/wenz/11.html",
        sololike(EPlayerIndex::EPI1, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EU,SU,EO,E9,HZ,S9],[GU,HU,EA,GA,SA,SZ],[EK,GZ,G9,H9,SK,SO],[EZ,GK,GO,HA,HK,HO],],
        vec![0,],
        vec![0,1,],
        &[(0, [EU,GU,GZ,EZ]),(0, [EO,EA,EK,GO]),(1, [HU,G9,GK,SU]),(1, [GA,H9,HO,S9]),(1, [SA,SO,HK,E9]),(1, [SZ,SK,HA,HZ]),],
        [-480, 1440, -480, -480],
    );
    test_rules(
        "../../testdata/games/wenz/12.html",
        sololike(EPlayerIndex::EPI3, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EA,EO,HZ,HO,H9,S9],[EK,E9,GA,GK,G9,HK],[GZ,GO,HA,SA,SK,SO],[EU,GU,HU,SU,EZ,SZ],],
        vec![3,],
        vec![],
        &[(0, [HZ,HK,HA,SU]),(3, [HU,EO,EK,SO]),(3, [GU,S9,E9,SK]),(3, [SZ,EA,GK,SA]),(2, [GO,EU,H9,G9]),(3, [EZ,HO,GA,GZ]),],
        [-180, -180, -180, 540],
    );
    test_rules(
        "../../testdata/games/wenz/13.html",
        sololike(EPlayerIndex::EPI2, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EO,GA,GZ,GK,HA,HO,H9,SZ],[SU,E8,G7,HZ,SK,SO,S9,S7],[GU,HU,EA,EZ,EK,E9,GO,G9],[EU,E7,G8,HK,H8,H7,SA,S8],],
        vec![1,],
        vec![],
        &[(0, [EO,E8,EK,E7]),(2, [HU,EU,SZ,SU]),(3, [G8,GK,G7,G9]),(0, [GZ,S7,GO,SA]),(0, [GA,SO,GU,S8]),(2, [EA,H7,H9,S9]),(2, [EZ,H8,HO,SK]),(2, [E9,HK,HA,HZ]),],
        [-100, -100, 300, -100],
    );
    test_rules(
        "../../testdata/games/wenz/14.html",
        sololike(EPlayerIndex::EPI2, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[GU,HU,SU,EK,GZ,SZ],[E9,HZ,HK,HO,SK,S9],[EA,GA,GO,HA,H9,SA],[EU,EZ,EO,GK,G9,SO],],
        vec![0,3,],
        vec![0,],
        &[(0, [EK,E9,EA,EO]),(2, [SA,SO,SZ,S9]),(2, [HA,EU,GZ,HZ]),(3, [EZ,SU,SK,H9]),(0, [GU,HK,GO,GK]),(0, [HU,HO,GA,G9]),],
        [720, 720, -2160, 720],
    );
    test_rules(
        "../../testdata/games/wenz/2.html",
        sololike(EPlayerIndex::EPI1, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EO,E7,GA,HO,H7,SK,S9,S8],[EU,SU,GK,G8,G7,HA,HZ,SA],[GU,HU,EZ,EK,E8,HK,H9,SO],[EA,E9,GZ,GO,G9,H8,SZ,S7],],
        vec![1,],
        vec![],
        &[(0, [GA,G7,EZ,GZ]),(0, [EO,G8,EK,EA]),(3, [S7,SK,SA,SO]),(1, [EU,HU,H8,E7]),(1, [HZ,H9,E9,H7]),(1, [SU,GU,G9,HO]),(2, [HK,GO,S8,HA]),(1, [GK,E8,SZ,S9]),],
        [-100, 300, -100, -100],
    );
    test_rules(
        "../../testdata/games/wenz/3.html",
        sololike(EPlayerIndex::EPI1, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EO,E9,GZ,G7,HO,H7,SZ,S9],[EU,GU,HU,EK,GA,GO,HK,SA],[SU,EZ,GK,HA,H9,H8,SK,S7],[EA,E8,E7,G9,G8,HZ,SO,S8],],
        vec![],
        vec![],
        &[(0, [EO,EK,EZ,EA]),(3, [G9,G7,GO,GK]),(2, [HA,HZ,HO,HK]),(2, [H9,SO,H7,HU]),(1, [GU,SU,S8,E9]),(1, [EU,H8,G8,GZ]),(1, [GA,S7,E7,S9]),(1, [SA,SK,E8,SZ]),],
        [80, -240, 80, 80],
    );
    test_rules(
        "../../testdata/games/wenz/4.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EU,SU,GZ,HZ,SA,SZ],[GU,EA,EK,E9,G9,HA],[HU,EZ,GA,GK,GO,S9],[EO,HK,HO,H9,SK,SO],],
        vec![3,],
        vec![],
        &[(0, [EU,GU,HU,H9]),(0, [GZ,G9,GA,HO]),(2, [GK,SK,HZ,HA]),(2, [GO,HK,SU,E9]),(0, [SA,EK,S9,SO]),(0, [SZ,EA,EZ,EO]),],
        [300, -100, -100, -100],
    );
    test_rules(
        "../../testdata/games/wenz/5.html",
        sololike(EPlayerIndex::EPI3, None, ESoloLike::Wenz, SPayoutDeciderTout::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EK,EO,GO,SA,SK,S9],[HU,E9,GA,HZ,H9,SO],[EA,EZ,GZ,GK,G9,SZ],[EU,GU,SU,HA,HK,HO],],
        vec![3,0,1,],
        vec![],
        &[(0, [EK,E9,EA,GU]),(3, [EU,SA,HU,EZ]),(3, [SU,GO,H9,SZ]),(3, [HA,EO,HZ,G9]),(3, [HK,SK,SO,GK]),(3, [HO,S9,GA,GZ]),],
        [-1120, -1120, -1120, 3360],
    );
    test_rules(
        "../../testdata/games/wenz/6.html",
        sololike(EPlayerIndex::EPI3, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EK,E8,E7,GZ,G9,HK,SZ,SO],[EU,HU,GK,GO,G8,HO,H7,S8],[GU,SU,EZ,EO,H9,H8,SK,S7],[EA,E9,GA,G7,HA,HZ,SA,S9],],
        vec![2,],
        vec![],
        &[(0, [HK,H7,H8,HA]),(3, [SA,SO,S8,S7]),(3, [GA,G9,G8,SU]),(2, [SK,S9,SZ,HO]),(0, [GZ,GO,H9,G7]),(0, [EK,GK,EO,EA]),(3, [HZ,E7,HU,EZ]),(1, [EU,GU,E9,E8]),],
        [180, 180, 180, -540],
    );
    test_rules(
        "../../testdata/games/wenz/7.html",
        sololike(EPlayerIndex::EPI2, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EZ,EK,GK,GO,HO,H9,SZ,S7],[GU,EO,G8,G7,HZ,H8,SO,S8],[EA,E9,E8,GA,G9,HA,H7,SA],[EU,HU,SU,E7,GZ,HK,SK,S9],],
        vec![],
        vec![],
        &[(0, [EK,EO,EA,E7]),(2, [SA,S9,S7,S8]),(2, [HA,HK,H9,H8]),(2, [GA,GZ,GO,G7]),(2, [H7,SU,HO,HZ]),(3, [EU,SZ,GU,G9]),(3, [HU,EZ,SO,E8]),(3, [SK,GK,G8,E9]),],
        [-90, -90, 270, -90],
    );
    test_rules(
        "../../testdata/games/wenz/8.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EU,SU,GZ,G9,HZ,HO,H7,SA],[HU,E7,GA,GK,HA,HK,SO,S7],[EA,EZ,EK,G8,H9,H8,S9,S8],[GU,EO,E9,E8,GO,G7,SZ,SK],],
        vec![],
        vec![],
        &[(0, [EU,HU,G8,GU]),(0, [H7,HK,H8,GO]),(1, [HA,H9,EO,HO]),(1, [GA,S9,G7,G9]),(1, [SO,S8,SK,SA]),(0, [HZ,E7,EK,E9]),(0, [GZ,GK,EZ,E8]),(0, [SU,S7,EA,SZ]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/wenz/9.html",
        sololike(EPlayerIndex::EPI2, None, ESoloLike::Wenz, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[SU,EK,GZ,HK,SO,S9],[EO,GK,GO,G9,HA,SA],[GU,HU,EA,EZ,E9,GA],[EU,HZ,HO,H9,SZ,SK],],
        vec![],
        vec![],
        &[(0, [SO,SA,HU,SK]),(2, [GU,EU,SU,HA]),(3, [SZ,S9,EO,E9]),(3, [H9,HK,G9,EZ]),(0, [EK,GK,EA,HO]),(2, [GA,HZ,GZ,GO]),],
        [-50, -50, 150, -50],
    );
}

#[test]
fn test_rulessolo() {
    test_rules(
        "../../testdata/games/28.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,HO,GZ,G7,EA,EZ,HK],[SO,EU,HU,GA,GK,HA,SA,S9],[GU,SU,G9,G8,E9,H8,SK,S7],[EK,E8,E7,HZ,H9,H7,SZ,S8],],
        vec![],
        vec![],
        &[(0, [EO,GK,G8,E7]),(0, [HO,HU,G9,E8]),(0, [GO,EU,SU,H7]),(0, [EA,GA,E9,EK]),(1, [HA,H8,H9,HK]),(1, [SA,S7,S8,GZ]),(0, [G7,SO,GU,SZ]),(1, [S9,SK,HZ,EZ]),],
        [-240, 80, 80, 80],
    );
    test_rules(
        "../../testdata/games/34.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,HO,GU,HU,EK,E9,HA,H7],[GO,SO,EU,SU,E7,G7,HK,SZ],[EZ,E8,GA,G9,G8,H9,SK,S8],[EA,GZ,GK,HZ,H8,SA,S9,S7],],
        vec![],
        vec![],
        &[(0, [EO,E7,E8,EA]),(0, [HU,EU,EZ,GZ]),(1, [HK,H9,H8,HA]),(0, [GU,SO,GA,HZ]),(1, [G7,G8,GK,EK]),(0, [H7,SU,SK,SA]),(1, [SZ,S8,S7,HO]),(0, [E9,GO,G9,S9]),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/7.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SU,GA,HZ,H8,H7,SA,S9,S7],[GO,HU,G8,EZ,EK,E8,E7,S8],[EO,SO,GU,GZ,GK,G9,HA,HK],[HO,EU,G7,EA,E9,H9,SZ,SK],],
        vec![],
        vec![],
        &[(0, [H8,HU,HK,H9]),(1, [EZ,GU,E9,S7]),(2, [EO,G7,SU,G8]),(2, [G9,EU,GA,GO]),(1, [S8,GZ,SK,S9]),(2, [SO,HO,H7,EK]),(3, [EA,HZ,E7,GK]),(2, [HA,SZ,SA,E8]),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/1-herz-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,SO,HU,SU,HK,H8,H7,G7],[HO,HZ,E7,GK,G8,SA,S9,S7],[GU,HA,EA,EK,GA,GZ,G9,SZ],[GO,EU,H9,EZ,E9,E8,SK,S8],],
        vec![],
        vec![],
        &[(0, [EO,HO,GU,H9]),(0, [HU,HZ,HA,EU]),(3, [S8,HK,S7,SZ]),(0, [SU,SA,GA,GO]),(3, [SK,G7,S9,GZ]),(3, [E8,H7,E7,EK]),(0, [SO,G8,G9,E9]),(0, [H8,GK,EA,EZ]),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/10-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/100-eichel-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,EU,EZ,GA,G7,HZ,HK,SA],[SO,GU,EA,GK,G9,H8,SZ,SK],[EK,GZ,HA,H9,H7,S9,S8,S7],[GO,HO,HU,SU,E9,E8,E7,G8],],
        vec![],
        vec![],
        &[(0, [SA,SK,S7,E8]),(3, [HO,EO,EA,EK]),(0, [GA,GK,GZ,G8]),(0, [HK,H8,H7,E7]),(3, [GO,EZ,GU,S8]),(3, [SU,EU,SO,HA]),(1, [SZ,S9,E9,G7]),(3, [HU,HZ,G9,H9]),],
        [50, 50, 50, -150],
    );
    test_rules(
        "../../testdata/games/solo/104-herz-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HU,EA,EK,E9,E8,E7,SA,S9],[SO,HA,EZ,GZ,GK,G8,G7,SZ],[GU,SU,HZ,H9,GA,SK,S8,S7],[EO,GO,HO,EU,HK,H8,H7,G9],],
        vec![],
        vec![],
        &[(0, [SA,SZ,S8,HK]),(3, [GO,HU,HA,H9]),(3, [HO,S9,SO,SU]),(3, [EU,E7,G7,HZ]),(3, [EO,E8,G8,GU]),(3, [G9,E9,GZ,GA]),(2, [SK,H8,EK,GK]),(3, [H7,EA,EZ,S7]),],
        [-90, -90, -90, 270],
    );
    test_rules(
        "../../testdata/games/solo/105-eichel-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SO,EK,G7,HZ,HK,H7,SK,S8],[EU,HU,E7,GA,G8,H8,SZ,S9],[HO,EA,EZ,GZ,G9,H9,SA,S7],[EO,GO,GU,SU,E9,E8,GK,HA],],
        vec![],
        vec![],
        &[(0, [HK,H8,H9,HA]),(3, [GO,EK,E7,EA]),(3, [EO,SO,HU,EZ]),(3, [E8,HZ,EU,HO]),(2, [SA,SU,S8,S9]),(3, [GK,G7,GA,GZ]),(1, [SZ,S7,E9,SK]),(3, [GU,H7,G8,G9]),],
        [-50, -50, -50, 150],
    );
    // ../../testdata/games/solo/106-gras-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/109-herz-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,HO,SO,EU,GU,HK,E7,G8],[GO,H8,E8,GZ,GK,G9,G7,SZ],[HU,HA,H9,EA,EK,GA,S9,S8],[SU,HZ,H7,EZ,E9,SA,SK,S7],],
        vec![],
        vec![],
        &[(0, [GU,GO,HA,HZ]),(1, [E8,EA,EZ,E7]),(2, [EK,E9,G8,SZ]),(2, [GA,SU,EU,G7]),(0, [SO,H8,H9,H7]),(0, [HO,G9,HU,S7]),(0, [EO,GK,S9,SK]),(0, [HK,GZ,S8,SA]),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/11-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/111-herz-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,HU,EZ,E9,E8,GA,GZ,SA],[HO,GU,SU,HK,H9,EA,SZ,S9],[GO,SO,EU,HA,HZ,H8,H7,GK],[EK,E7,G9,G8,G7,SK,S8,S7],],
        vec![],
        vec![],
        &[(0, [SA,SZ,SO,S7]),(2, [EU,SK,EO,HK]),(0, [GA,S9,GK,G7]),(0, [GZ,HO,GO,G8]),(2, [H8,EK,HU,H9]),(0, [E9,EA,HA,E7]),(2, [H7,G9,E8,SU]),(1, [GU,HZ,S8,EZ]),],
        [-50, -50, 150, -50],
    );
    // ../../testdata/games/solo/112-gras-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/113-herz-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HZ,HK,EK,E7,GA,G9,SA,SK],[GO,H8,EZ,E8,GK,G8,S9,S8],[EO,SO,EU,HU,HA,H9,H7,S7],[HO,GU,SU,EA,E9,GZ,G7,SZ],],
        vec![],
        vec![],
        &[(0, [GA,GK,HA,G7]),(2, [EO,SU,HK,H8]),(2, [EU,HO,HZ,GO]),(1, [G8,S7,GZ,G9]),(3, [SZ,SA,S9,H7]),(2, [SO,GU,E7,S8]),(2, [HU,E9,SK,E8]),(2, [H9,EA,EK,EZ]),],
        [-60, -60, 180, -60],
    );
    test_rules(
        "../../testdata/games/solo/114-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,EU,GU,EZ,E9,E8,S7],[HU,SU,E7,G8,HA,H8,H7,SZ],[HO,SO,GK,HZ,HK,H9,S9,S8],[EA,EK,GA,GZ,G9,G7,SA,SK],],
        vec![],
        vec![],
        &[(0, [GO,E7,SO,EK]),(0, [EO,SU,HO,EA]),(0, [GU,HU,GK,G9]),(0, [S7,SZ,S8,SA]),(3, [GA,E9,G8,S9]),(0, [EU,H7,H9,G7]),(0, [EZ,H8,HK,GZ]),(0, [E8,HA,HZ,SK]),],
        [180, -60, -60, -60],
    );
    // ../../testdata/games/solo/116-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/119-eichel-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SO,EU,EK,E8,E7,GZ,GK,SZ],[HO,GU,SU,HK,H9,H8,SA,S9],[EO,GO,HU,EA,EZ,E9,GA,H7],[G9,G8,G7,HA,HZ,SK,S8,S7],],
        vec![],
        vec![],
        &[(0, [GK,GU,GA,G9]),(1, [HK,H7,HA,SZ]),(3, [G8,GZ,HO,GO]),(2, [E9,HZ,EK,SU]),(1, [H9,HU,SK,EU]),(0, [E7,H8,EA,S7]),(2, [EO,S8,E8,S9]),(2, [EZ,G7,SO,SA]),],
        [60, 60, -180, 60],
    );
    // ../../testdata/games/solo/120-herz-solo.html has wrong format
    // ../../testdata/games/solo/122-gras-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/123-eichel-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[E9,GK,G8,HA,H7,SA,SZ,S8],[GO,HO,EU,EA,EK,E8,E7,HK],[EO,SO,HU,SU,GZ,G9,H9,H8],[GU,EZ,GA,G7,HZ,SK,S9,S7],],
        vec![],
        vec![2,],
        &[(0, [SA,EA,SU,SK]),(2, [H8,HZ,H7,HK]),(3, [S9,SZ,HO,EO]),(2, [H9,GU,HA,EU]),(1, [GO,HU,EZ,E9]),(1, [E8,SO,GA,GK]),(2, [GZ,G7,G8,EK]),(1, [E7,G9,S7,S8]),],
        [100, -300, 100, 100],
    );
    test_rules(
        "../../testdata/games/solo/124-herz-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,SO,EU,GU,HK,H7,SA],[H8,EA,EZ,E9,E8,GA,G9,S9],[HA,E7,GZ,G8,G7,SZ,S8,S7],[HO,HU,SU,HZ,H9,EK,GK,SK],],
        vec![],
        vec![],
        &[(0, [EO,H8,HA,H9]),(0, [GO,S9,E7,SU]),(0, [GU,EA,GZ,HO]),(3, [EK,HK,E8,G7]),(0, [SO,EZ,G8,HZ]),(0, [EU,E9,S7,HU]),(0, [SA,G9,S8,SK]),(0, [H7,GA,SZ,GK]),],
        [180, -60, -60, -60],
    );
    // ../../testdata/games/solo/126-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/127-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,G9,G7,EA,E9,HA,H9],[EU,GA,GZ,E8,E7,HZ,HK,S7],[HO,SO,GU,HU,SU,H8,SZ,SK],[GK,G8,EZ,EK,H7,SA,S9,S8],],
        vec![],
        vec![],
        &[(0, [GO,GA,SU,G8]),(0, [EO,EU,HU,GK]),(0, [G7,GZ,GU,EZ]),(2, [H8,H7,HA,HK]),(0, [EA,E7,SO,EK]),(2, [HO,SA,G9,HZ]),(2, [SZ,S9,H9,S7]),(2, [SK,S8,E9,E8]),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/128-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/129-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,SO,HU,GK,G8,E7,S9],[G7,EZ,E8,HA,HZ,H9,SK,S8],[EU,GU,SU,GA,GZ,EA,EK,H8],[HO,G9,E9,HK,H7,SA,SZ,S7],],
        vec![],
        vec![],
        &[(0, [GO,G7,SU,G9]),(0, [EO,S8,GU,HO]),(0, [SO,SK,GZ,E9]),(0, [G8,HA,GA,SA]),(2, [EA,HK,E7,EZ]),(2, [EU,SZ,HU,HZ]),(2, [EK,S7,S9,E8]),(2, [H8,H7,GK,H9]),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/13-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/130-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,HU,GK,G9,G7,H8,SA],[GU,GZ,G8,E8,E7,HK,SZ,S9],[EU,SU,GA,EZ,E9,H9,H7,S8],[HO,SO,EA,EK,HA,HZ,SK,S7],],
        vec![],
        vec![],
        &[(0, [EO,G8,SU,SO]),(0, [GO,GU,EU,HO]),(0, [HU,GZ,GA,S7]),(0, [H8,HK,H9,HA]),(3, [HZ,G9,E7,H7]),(0, [G7,E8,S8,SK]),(0, [GK,S9,E9,EK]),(0, [SA,SZ,EZ,EA]),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/solo/131-herz-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HZ,EK,E9,E8,GA,GZ,G8,S8],[EU,H7,EA,EZ,GK,G7,SK,S7],[EO,HO,SO,GU,HU,HA,HK,SZ],[GO,SU,H9,H8,E7,G9,SA,S9],],
        vec![],
        vec![],
        &[(0, [EK,EZ,GU,E7]),(2, [EO,H8,HZ,H7]),(2, [SO,H9,S8,EU]),(2, [HU,GO,GA,EA]),(3, [SA,E8,SK,SZ]),(3, [S9,E9,S7,HA]),(2, [HO,SU,G8,G7]),(2, [HK,G9,GZ,GK]),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/132-herz-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SU,EA,EZ,EK,E9,GZ,SZ,S8],[SO,EU,HK,G9,G7,SK,S9,S7],[GO,GU,H8,E7,GA,GK,G8,SA],[EO,HO,HU,HA,HZ,H9,H7,E8],],
        vec![],
        vec![],
        &[(0, [GZ,G7,GA,HA]),(3, [HU,SU,SO,H8]),(1, [SK,SA,HZ,S8]),(3, [EO,E9,HK,GU]),(3, [H7,SZ,EU,GO]),(2, [GK,E8,EA,G9]),(2, [G8,H9,EZ,S9]),(3, [HO,EK,S7,E7]),],
        [-50, -50, -50, 150],
    );
    // ../../testdata/games/solo/134-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/135-eichel-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SO,SU,EA,E7,HA,HZ,H9,SK],[HO,EU,GU,GZ,GK,H7,SA,S9],[EK,G9,G8,G7,HK,H8,S8,S7],[EO,GO,HU,EZ,E9,E8,GA,SZ],],
        vec![],
        vec![],
        &[(0, [SK,SA,S7,SZ]),(1, [S9,S8,E9,H9]),(3, [GO,E7,GU,EK]),(3, [EO,SU,EU,G9]),(3, [E8,EA,HO,HK]),(1, [GK,G7,GA,SO]),(0, [HA,H7,H8,EZ]),(3, [HU,HZ,GZ,G8]),],
        [50, 50, 50, -150],
    );
    test_rules(
        "../../testdata/games/solo/137-gras-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,GK,G9,EA,E8,E7,SZ,SK],[EU,HU,EK,E9,HA,H8,S9,S8],[EO,GO,SO,GZ,G8,G7,H9,H7],[HO,SU,GA,EZ,HZ,HK,SA,S7],],
        vec![],
        vec![],
        &[(0, [EA,E9,GZ,EZ]),(2, [GO,SU,GK,HU]),(2, [EO,GA,G9,EU]),(2, [G7,HO,GU,EK]),(3, [SA,SK,S8,G8]),(2, [SO,S7,E8,S9]),(2, [H7,HZ,SZ,HA]),(1, [H8,H9,HK,E7]),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/139-gras-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,GU,G7,EA,E7,HK,H9,S9],[EU,HU,GZ,E9,HA,H8,H7,S7],[EO,HO,SO,SU,GA,GK,G8,EK],[G9,EZ,E8,HZ,SA,SZ,SK,S8],],
        vec![],
        vec![],
        &[(0, [S9,S7,EK,SA]),(3, [SZ,HK,EU,SO]),(2, [EO,G9,G7,HU]),(2, [SU,HZ,GU,GZ]),(0, [EA,E9,GA,E8]),(2, [HO,EZ,GO,HA]),(0, [H9,H7,GK,S8]),(2, [G8,SK,E7,H8]),],
        [50, 50, -150, 50],
    );
    test_rules(
        "../../testdata/games/solo/142-gras-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,SU,G7,EA,E9,SA,SZ,S8],[EO,GO,HO,SO,HU,GA,G9,E8],[G8,EZ,E7,HA,HZ,HK,S9,S7],[EU,GZ,GK,EK,H9,H8,H7,SK],],
        vec![],
        vec![],
        &[(0, [EA,E8,EZ,EK]),(0, [E9,HU,E7,SK]),(1, [SO,G8,GZ,G7]),(1, [HO,HA,GK,SU]),(1, [GO,S7,EU,GU]),(1, [EO,S9,H9,S8]),(1, [GA,HK,H7,SZ]),(1, [G9,HZ,H8,SA]),],
        [-100, 300, -100, -100],
    );
    test_rules(
        "../../testdata/games/solo/143-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,HO,SO,GU,HU,GK,G7,H9],[EU,GZ,EA,HZ,HK,SA,SZ,SK],[GA,G8,E9,E7,HA,H8,H7,S9],[GO,SU,G9,EZ,EK,E8,S8,S7],],
        vec![],
        vec![],
        &[(0, [EO,EU,G8,G9]),(0, [HU,GZ,GA,GO]),(3, [EZ,GU,EA,E9]),(0, [SO,SA,S9,SU]),(0, [HO,SK,E7,S8]),(0, [H9,HZ,HA,EK]),(2, [H8,E8,GK,HK]),(0, [G7,SZ,H7,S7]),],
        [150, -50, -50, -50],
    );
    // ../../testdata/games/solo/144-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/146-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HO,SO,SU,EA,EK,E8,HA,HZ],[EO,GO,EU,E9,GK,G7,HK,SZ],[G9,G8,H9,H8,H7,SA,SK,S8],[GU,HU,EZ,E7,GA,GZ,S9,S7],],
        vec![],
        vec![],
        &[(0, [SO,GO,SA,EZ]),(1, [HK,H7,HU,HA]),(3, [GA,EA,G7,G8]),(0, [SU,EU,SK,E7]),(1, [GK,G9,GZ,EK]),(0, [E8,E9,S8,GU]),(3, [S9,HO,SZ,H8]),(0, [HZ,EO,H9,S7]),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/149-herz-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,HO,SO,EU,GU,HK,H8,G8],[SU,H9,EZ,E8,E7,GA,SZ,S8],[EO,HU,HA,H7,EK,GK,SK,S9],[HZ,EA,E9,GZ,G9,G7,SA,S7],],
        vec![],
        vec![],
        &[(0, [GU,H9,HA,HZ]),(0, [EU,SU,H7,G7]),(0, [SO,EZ,EO,GZ]),(2, [GK,G9,G8,GA]),(1, [E8,EK,EA,HK]),(0, [HO,E7,HU,E9]),(0, [GO,S8,S9,S7]),(0, [H8,SZ,SK,SA]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/15-herz-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HU,SU,H9,GA,GK,G9],[EO,GU,HK,EZ,EK,SK],[GO,HO,SO,EU,HZ,SA],[HA,EA,E9,GZ,SZ,S9],],
        vec![],
        vec![],
        &[(0, [GA,SK,HZ,GZ]),(2, [EU,HA,H9,EO]),(1, [EZ,SO,EA,G9]),(2, [HO,E9,SU,HK]),(2, [GO,S9,HU,GU]),(2, [SA,SZ,GK,EK]),],
        [-60, -60, 180, -60],
    );
    test_rules(
        "../../testdata/games/solo/150-gras-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SO,EU,G9,EZ,E7,HZ,HK,SA],[EO,GO,GU,HU,GZ,G7,EA,E8],[SU,GA,GK,EK,H9,H8,SK,S9],[HO,G8,E9,HA,H7,SZ,S8,S7],],
        vec![],
        vec![],
        &[(0, [SA,HU,S9,S7]),(1, [GO,GA,G8,G9]),(1, [EO,SU,HO,EU]),(1, [G7,GK,SZ,SO]),(0, [HZ,GZ,H8,H7]),(1, [GU,H9,S8,HK]),(1, [EA,EK,E9,E7]),(1, [E8,SK,HA,EZ]),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/151-gras-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HU,GK,G9,G8,E9,HA,H9,H8],[EO,SO,GU,GA,GZ,G7,SZ,S8],[GO,EU,SU,EZ,EK,H7,S9,S7],[HO,EA,E8,E7,HZ,HK,SA,SK],],
        vec![],
        vec![],
        &[(0, [HA,GA,H7,HK]),(1, [GU,EU,HO,GK]),(3, [SA,E9,S8,S7]),(3, [SK,G9,SZ,S9]),(0, [H9,SO,EK,HZ]),(1, [EO,SU,E7,G8]),(1, [G7,GO,EA,HU]),(2, [EZ,E8,H8,GZ]),],
        [-50, 150, -50, -50],
    );
    // ../../testdata/games/solo/153-eichel-solo.html has wrong format
    // ../../testdata/games/solo/154-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/155-herz-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,SO,EU,GU,H8,EA,S9],[HK,H9,EZ,E9,GK,G7,SA,S7],[SU,HA,H7,EK,G9,G8,SK,S8],[HO,HU,HZ,E8,E7,GA,GZ,SZ],],
        vec![],
        vec![],
        &[(0, [GO,HK,HA,HU]),(0, [EO,H9,H7,HZ]),(0, [GU,EZ,SU,HO]),(3, [GA,H8,G7,G9]),(0, [EU,E9,EK,E7]),(0, [S9,S7,SK,SZ]),(3, [E8,EA,GK,G8]),(0, [SO,SA,S8,GZ]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/156-eichel-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[E9,E8,GA,HZ,HK,H8,H7,S8],[HU,SU,GZ,GK,G9,SZ,SK,S7],[EO,HO,EU,EA,EZ,EK,E7,SA],[GO,SO,GU,G8,G7,HA,H9,S9],],
        vec![],
        vec![],
        &[(0, [HK,HU,EU,H9]),(2, [E7,GU,E9,SU]),(3, [HA,H7,S7,EA]),(2, [EO,SO,E8,G9]),(2, [HO,GO,GA,SZ]),(3, [G8,S8,GZ,EZ]),(2, [SA,S9,H8,SK]),(2, [EK,G7,HZ,GK]),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/157-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,SU,E9,E8,G7,HA,HZ],[EA,E7,GA,GK,G9,H8,SK,S8],[HO,SO,GU,EK,GZ,G8,HK,SZ],[EU,HU,EZ,H9,H7,SA,S9,S7],],
        vec![],
        vec![],
        &[(0, [GO,EA,EK,HU]),(0, [EO,E7,GU,EU]),(0, [E9,SK,SO,EZ]),(2, [HK,H9,HA,H8]),(0, [HZ,GK,HO,H7]),(2, [SZ,SA,E8,S8]),(0, [G7,GA,GZ,S9]),(1, [G9,G8,S7,SU]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/159-herz-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SO,EU,E7,GZ,GK,G9,G7,SA],[EO,GO,HO,HU,SU,HA,H7,EK],[HK,H9,EZ,E9,G8,SZ,S9,S7],[GU,HZ,H8,EA,E8,GA,SK,S8],],
        vec![],
        vec![],
        &[(0, [GZ,SU,G8,GA]),(1, [GO,HK,H8,EU]),(1, [HO,H9,GU,SO]),(1, [HU,SZ,HZ,E7]),(1, [EO,S9,S8,G7]),(1, [EK,EZ,EA,SA]),(3, [E8,GK,H7,E9]),(1, [HA,S7,SK,G9]),],
        [-80, 240, -80, -80],
    );
    test_rules(
        "../../testdata/games/solo/160-gras-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,GU,G8,EA,HK,H8,SA,S9],[GA,E9,E8,E7,H9,SZ,SK,S8],[EO,GO,HO,HU,SU,GK,G9,EZ],[SO,GZ,G7,EK,HA,HZ,H7,S7],],
        vec![],
        vec![],
        &[(0, [SA,SZ,SU,S7]),(2, [EO,G7,G8,GA]),(2, [HO,GZ,GU,H9]),(2, [GO,SO,EU,E7]),(2, [G9,EK,S9,E8]),(2, [EZ,HA,EA,E9]),(0, [HK,SK,GK,H7]),(2, [HU,HZ,H8,S8]),],
        [-80, -80, 240, -80],
    );
    test_rules(
        "../../testdata/games/solo/161-herz-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,EU,H9,EA,E8,E7,G9,S9],[SO,GU,EZ,E9,G8,SZ,SK,S7],[SU,HA,HZ,HK,H7,GA,GZ,G7],[EO,HO,HU,H8,EK,GK,SA,S8],],
        vec![],
        vec![],
        &[(0, [EA,EZ,HA,EK]),(2, [H7,H8,H9,GU]),(1, [E9,HK,GK,E8]),(2, [SU,HU,EU,SO]),(1, [G8,GZ,HO,G9]),(3, [S8,S9,S7,G7]),(0, [E7,SK,HZ,EO]),(3, [SA,GO,SZ,GA]),],
        [120, 120, -360, 120],
    );
    test_rules(
        "../../testdata/games/solo/162-eichel-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[E7,G9,G7,HZ,H8,SA,SZ,S7],[GO,HO,HU,SU,EA,EZ,E8,HA],[EK,GA,GZ,GK,H9,SK,S9,S8],[EO,SO,EU,GU,E9,G8,HK,H7],],
        vec![],
        vec![3,],
        &[(0, [SA,SU,S8,GU]),(3, [G8,G9,EA,GK]),(1, [HU,EK,EU,E7]),(3, [H7,H8,HA,H9]),(1, [HO,GA,EO,HZ]),(3, [HK,G7,EZ,S9]),(1, [GO,SK,E9,S7]),(1, [E8,GZ,SO,SZ]),],
        [100, -300, 100, 100],
    );
    // ../../testdata/games/solo/163-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/164-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,SO,HU,EK,E9,HZ,HK],[HO,EA,EZ,E7,H8,SZ,SK,S7],[GU,GA,GZ,G9,G7,HA,H9,SA],[EU,SU,E8,GK,G8,H7,S9,S8],],
        vec![0,],
        vec![],
        &[(0, [GO,EZ,GU,E8]),(0, [EO,E7,G7,SU]),(0, [HU,EA,GA,EU]),(3, [H7,HK,H8,HA]),(2, [GZ,G8,E9,S7]),(0, [SO,HO,SA,GK]),(1, [SZ,H9,S8,EK]),(0, [HZ,SK,G9,S9]),],
        [-300, 100, 100, 100],
    );
    // ../../testdata/games/solo/165-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/166-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HO,SO,HU,SU,GA,G9,EA,HA],[GO,GZ,GK,EZ,E7,SA,S9,S8],[EU,GU,G8,G7,E9,E8,HZ,H7],[EO,EK,HK,H9,H8,SZ,SK,S7],],
        vec![],
        vec![],
        &[(0, [SO,GZ,G7,EO]),(3, [HK,HA,GK,HZ]),(1, [SA,H7,SK,GA]),(0, [SU,GO,G8,SZ]),(1, [S9,E8,S7,G9]),(0, [HO,S8,GU,H8]),(0, [EA,E7,E9,EK]),(0, [HU,EZ,EU,H9]),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/168-gras-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SU,GZ,GK,EZ,E9,E7,HZ,SK],[SO,G8,EA,E8,HA,H7,SA,S7],[EO,HU,EK,H9,H8,SZ,S9,S8],[GO,HO,EU,GU,GA,G9,G7,HK],],
        vec![],
        vec![],
        &[(0, [SK,SA,S8,GA]),(3, [EU,GZ,SO,HU]),(1, [HA,H9,HK,HZ]),(1, [H7,H8,G9,GK]),(0, [E7,EA,EK,G7]),(3, [HO,SU,G8,EO]),(2, [SZ,GU,E9,S7]),(3, [GO,EZ,E8,S9]),],
        [-50, -50, -50, 150],
    );
    // ../../testdata/games/solo/169-gras-solo.html has wrong format
    // ../../testdata/games/solo/170-eichel-solo.html has wrong format
    // ../../testdata/games/solo/171-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/172-gras-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SO,HU,EA,EZ,E7,H7,SZ,S9],[EU,GU,GA,GZ,G8,HK,H8,S7],[G7,EK,E9,E8,H9,SA,SK,S8],[EO,GO,HO,SU,GK,G9,HA,HZ],],
        vec![],
        vec![],
        &[(0, [H7,H8,H9,HA]),(3, [EO,HU,G8,G7]),(3, [HO,SO,GZ,E8]),(3, [GO,E7,GU,E9]),(3, [SU,EA,EU,EK]),(1, [HK,SK,HZ,S9]),(3, [G9,SZ,GA,SA]),(1, [S7,S8,GK,EZ]),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/solo/173-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,HO,EU,EA,EZ,E9,G9,HA],[SO,HU,SU,E8,HZ,H8,S8,S7],[GA,GZ,GK,G8,G7,HK,SZ,S9],[EO,GU,EK,E7,H9,H7,SA,SK],],
        vec![],
        vec![],
        &[(0, [EU,SO,GA,EK]),(1, [S7,S9,SA,EA]),(0, [HO,E8,HK,E7]),(0, [GO,SU,G7,EO]),(3, [H9,HA,H8,G8]),(0, [E9,HU,SZ,GU]),(3, [SK,EZ,S8,GK]),(0, [G9,HZ,GZ,H7]),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/174-eichel-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HU,SU,EK,GA,H9,H8,H7,SZ],[EO,GO,GU,EA,E8,E7,G8,SA],[HO,EU,E9,GK,G7,HA,SK,S9],[SO,EZ,GZ,G9,HZ,HK,S8,S7],],
        vec![],
        vec![],
        &[(0, [GA,G8,GK,GZ]),(0, [H7,EA,HA,HK]),(1, [GO,E9,EZ,SU]),(1, [EO,EU,SO,HU]),(1, [E8,HO,HZ,EK]),(2, [S9,S8,SZ,SA]),(1, [GU,SK,G9,H8]),(1, [E7,G7,S7,H9]),],
        [-50, 150, -50, -50],
    );
    // ../../testdata/games/solo/175-herz-solo.html has wrong format
    // ../../testdata/games/solo/176-gras-solo.html has wrong format
    // ../../testdata/games/solo/178-gras-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/179-herz-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HU,SU,HZ,EA,E8,GA,GZ,S7],[GU,HA,H8,EZ,EK,G9,SZ,S8],[H9,H7,E9,E7,GK,G8,SK,S9],[EO,GO,HO,SO,EU,HK,G7,SA],],
        vec![],
        vec![],
        &[(0, [EA,EK,E7,HK]),(3, [EO,SU,H8,H7]),(3, [GO,HU,GU,H9]),(3, [HO,HZ,HA,E9]),(3, [SA,S7,S8,S9]),(3, [G7,GA,G9,GK]),(0, [GZ,SZ,G8,SO]),(3, [EU,E8,EZ,SK]),],
        [-110, -110, -110, 330],
    );
    test_rules(
        "../../testdata/games/solo/18-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,HO,SO,GU,EK,G7,H8],[HU,SU,EA,EZ,E9,G8,H7,SK],[EU,E7,GA,GK,HZ,HK,SA,S7],[E8,GZ,G9,HA,H9,SZ,S9,S8],],
        vec![],
        vec![],
        &[(0, [EO,HU,E7,E8]),(0, [GO,E9,EU,S8]),(0, [SO,EZ,SA,S9]),(0, [GU,SU,S7,H9]),(0, [HO,EA,HK,SZ]),(0, [G7,G8,GK,GZ]),(3, [HA,H8,H7,HZ]),(3, [G9,EK,SK,GA]),],
        [270, -90, -90, -90],
    );
    test_rules(
        "../../testdata/games/solo/180-gras-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,EA,E8,H8,SZ,S9,S8,S7],[HO,SO,GU,SU,GK,G9,G8,HA],[EO,HU,EZ,E9,E7,HK,SA,SK],[GO,GA,GZ,G7,EK,HZ,H9,H7],],
        vec![],
        vec![],
        &[(0, [SZ,GK,SK,GA]),(3, [H9,H8,HA,HK]),(1, [GU,HU,GZ,EU]),(0, [S7,SU,SA,GO]),(3, [EK,E8,SO,E7]),(1, [G9,EO,G7,EA]),(2, [E9,H7,S8,G8]),(1, [HO,EZ,HZ,S9]),],
        [50, -150, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/181-eichel-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,HU,E8,GZ,HZ,H9,SZ,S7],[HO,GK,G8,G7,HA,H8,SA,S9],[SO,EU,GU,EA,EZ,EK,GA,S8],[EO,SU,E9,E7,G9,HK,H7,SK],],
        vec![],
        vec![],
        &[(0, [GZ,GK,GA,G9]),(2, [GU,E7,E8,HO]),(1, [G8,S8,SK,SZ]),(1, [G7,EK,SU,HZ]),(3, [H7,H9,H8,EA]),(2, [EU,E9,HU,S9]),(2, [SO,EO,GO,SA]),(3, [HK,S7,HA,EZ]),],
        [-80, -80, 240, -80],
    );
    test_rules(
        "../../testdata/games/solo/182-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HO,SO,EU,HU,SU,E7,GA,HA],[GU,E9,G8,G7,HZ,SZ,SK,S8],[EZ,E8,GZ,GK,HK,H8,H7,S7],[EO,GO,EA,EK,G9,H9,SA,S9],],
        vec![],
        vec![],
        &[(0, [EU,E9,EZ,GO]),(3, [SA,SU,S8,S7]),(0, [SO,GU,E8,EK]),(0, [HU,HZ,GZ,EO]),(3, [G9,GA,G7,GK]),(0, [HO,G8,H7,EA]),(0, [HA,SK,H8,H9]),(0, [E7,SZ,HK,S9]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/183-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HO,SO,SU,EA,EZ,E8,E7,S8],[EK,G9,HA,H8,H7,SA,SZ,SK],[EO,GO,EU,GU,HU,G7,H9,S7],[E9,GA,GZ,GK,G8,HZ,HK,S9],],
        vec![],
        vec![2,],
        &[(0, [E7,EK,HU,E9]),(2, [G7,GA,EA,G9]),(0, [E8,SA,GU,GZ]),(2, [EO,HZ,SU,SZ]),(2, [GO,HK,SO,HA]),(2, [S7,S9,S8,SK]),(1, [H7,H9,G8,EZ]),(0, [HO,H8,EU,GK]),],
        [-300, 100, 100, 100],
    );
    test_rules(
        "../../testdata/games/solo/184-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,HO,SO,EU,GA,G8,G7,SK],[G9,HA,HZ,H9,SZ,S9,S8,S7],[EO,HU,SU,GZ,GK,H8,H7,SA],[GU,EA,EZ,EK,E9,E8,E7,HK],],
        vec![],
        vec![2,],
        &[(0, [EU,G9,GK,GU]),(0, [SO,S7,SU,E7]),(0, [HO,HA,EO,EA]),(2, [SA,HK,SK,SZ]),(2, [H7,EK,G8,H9]),(0, [GO,S8,GZ,E8]),(0, [G7,HZ,HU,EZ]),(2, [H8,E9,GA,S9]),],
        [-300, 100, 100, 100],
    );
    // ../../testdata/games/solo/185-gras-solo.html has wrong format
    // ../../testdata/games/solo/186-gras-solo.html has wrong format
    // ../../testdata/games/solo/187-gras-solo.html has wrong format
    // ../../testdata/games/solo/188-eichel-solo.html has wrong format
    // ../../testdata/games/solo/189-herz-solo.html has wrong format
    // ../../testdata/games/solo/19-eichel-solo.html has wrong format
    // ../../testdata/games/solo/191-eichel-solo.html has wrong format
    // ../../testdata/games/solo/192-gras-solo.html has wrong format
    // ../../testdata/games/solo/193-herz-solo.html has wrong format
    // ../../testdata/games/solo/194-eichel-solo.html has wrong format
    // ../../testdata/games/solo/195-herz-solo.html has wrong format
    // ../../testdata/games/solo/196-herz-solo.html has wrong format
    // ../../testdata/games/solo/197-gras-solo.html has wrong format
    // ../../testdata/games/solo/198-eichel-solo.html has wrong format
    // ../../testdata/games/solo/199-eichel-solo.html has wrong format
    // ../../testdata/games/solo/2-herz-solo.html has wrong format
    // ../../testdata/games/solo/20-gras-solo.html has wrong format
    // ../../testdata/games/solo/200-gras-solo.html has wrong format
    // ../../testdata/games/solo/201-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/202-gras-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HO,HU,GA,EZ,E7,HK,H9,S8],[EO,GO,SO,EU,GZ,GK,G8,H7],[GU,SU,EK,E8,HA,HZ,SA,SK],[G9,G7,EA,E9,H8,SZ,S9,S7],],
        vec![],
        vec![],
        &[(0, [HK,H7,HA,H8]),(2, [SA,S7,S8,GZ]),(1, [GO,SU,G7,HU]),(1, [EO,GU,G9,GA]),(1, [EU,HZ,SZ,HO]),(0, [H9,GK,E8,S9]),(1, [SO,SK,E9,E7]),(1, [G8,EK,EA,EZ]),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/203-eichel-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,SO,GU,EZ,E9,G7,HA,SZ],[EK,GA,G9,G8,H9,H8,SA,S8],[E7,GZ,GK,HZ,HK,H7,SK,S9],[EO,HO,EU,HU,SU,EA,E8,S7],],
        vec![],
        vec![],
        &[(0, [HA,H9,HK,S7]),(0, [G7,GA,GK,EA]),(3, [HO,GO,EK,E7]),(0, [SZ,SA,S9,E8]),(3, [EO,E9,S8,SK]),(3, [SU,GU,H8,GZ]),(0, [SO,G8,HZ,HU]),(0, [EZ,G9,H7,EU]),],
        [-50, -50, -50, 150],
    );
    // ../../testdata/games/solo/204-gras-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/205-eichel-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,EK,E8,GK,G8,G7,HZ,H8],[EU,HU,EA,G9,H9,H7,SA,S9],[SU,GZ,HA,HK,SZ,SK,S8,S7],[EO,HO,SO,GU,EZ,E9,E7,GA],],
        vec![],
        vec![],
        &[(0, [G7,G9,GZ,GA]),(3, [SO,EK,EA,SU]),(3, [EO,E8,HU,S7]),(3, [GU,GO,EU,SZ]),(0, [H8,H7,HA,EZ]),(3, [HO,G8,H9,S8]),(3, [E9,GK,S9,SK]),(3, [E7,HZ,SA,HK]),],
        [-60, -60, -60, 180],
    );
    // ../../testdata/games/solo/206-eichel-solo.html has wrong format
    // ../../testdata/games/solo/207-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/209-eichel-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EZ,G9,G7,HA,H9,H8,SA,S8],[SO,EU,GU,SU,EK,E9,E8,E7],[EO,GA,GZ,GK,G8,HK,SZ,S7],[GO,HO,HU,EA,HZ,H7,SK,S9],],
        vec![],
        vec![],
        &[(0, [HA,SU,HK,H7]),(1, [GU,EO,EA,EZ]),(2, [GA,HZ,G7,EK]),(1, [EU,GZ,HO,G9]),(3, [S9,SA,E9,S7]),(1, [E8,SZ,HU,S8]),(3, [GO,H8,E7,GK]),(3, [SK,H9,SO,G8]),],
        [80, -240, 80, 80],
    );
    // ../../testdata/games/solo/21-gras-solo.html has wrong format
    // ../../testdata/games/solo/210-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/211-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,SO,EU,HU,EK,E9,E7,SA],[GO,HO,SU,EZ,H9,H8,H7,SZ],[GU,EA,GZ,GK,G9,HK,S9,S8],[E8,GA,G8,G7,HA,HZ,SK,S7],],
        vec![],
        vec![],
        &[(0, [EO,SU,GU,E8]),(0, [HU,GO,EA,HA]),(1, [H9,HK,HZ,EK]),(0, [EU,HO,GZ,GA]),(1, [SZ,S8,S7,SA]),(0, [SO,EZ,S9,G7]),(0, [E9,H8,G9,G8]),(0, [E7,H7,GK,SK]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/213-gras-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,HO,SU,HA,HK,SZ,S9,S7],[SO,EU,G7,EZ,E7,SA,SK,S8],[EO,GU,GA,GZ,GK,G9,G8,H9],[HU,EA,EK,E9,E8,HZ,H8,H7],],
        vec![],
        vec![],
        &[(0, [HA,SO,H9,HZ]),(1, [SA,GK,HU,SZ]),(3, [H7,HK,EU,G8]),(1, [SK,GU,H8,S7]),(2, [EO,E8,SU,G7]),(2, [G9,EA,HO,EZ]),(0, [GO,E7,GZ,EK]),(0, [S9,S8,GA,E9]),],
        [60, 60, -180, 60],
    );
    test_rules(
        "../../testdata/games/solo/215-herz-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,HO,SO,EU,HK,H9,H7,GK],[EO,HU,SU,HZ,E7,G8,G7,S9],[H8,EK,E8,GA,GZ,SZ,S8,S7],[GU,HA,EA,EZ,E9,G9,SA,SK],],
        vec![],
        vec![],
        &[(0, [EU,HZ,H8,GU]),(0, [SO,EO,GA,HA]),(1, [E7,EK,EA,HK]),(0, [HO,SU,E8,G9]),(0, [GO,HU,S7,E9]),(0, [GK,G7,GZ,EZ]),(2, [S8,SA,H7,S9]),(0, [H9,G8,SZ,SK]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/216-herz-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,HU,HA,EZ,E8,E7,SZ,S8],[GO,EK,GZ,GK,SA,SK,S9,S7],[HO,HZ,H9,H7,GA,G9,G8,G7],[EO,SO,GU,SU,HK,H8,EA,E9],],
        vec![],
        vec![],
        &[(0, [E7,EK,HZ,E9]),(2, [GA,GU,EU,GZ]),(0, [EZ,GO,G7,EA]),(1, [GK,G8,SU,HU]),(0, [S8,S7,G9,HK]),(3, [EO,HA,S9,H7]),(3, [H8,SZ,SA,H9]),(2, [HO,SO,E8,SK]),],
        [60, 60, 60, -180],
    );
    test_rules(
        "../../testdata/games/solo/217-eichel-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SO,HU,E7,GA,G7,H9,SZ,S8],[E8,GK,G9,G8,HK,H8,H7,S7],[EO,GO,HO,EU,SU,EA,EZ,HA],[GU,EK,E9,GZ,HZ,SA,SK,S9],],
        vec![],
        vec![],
        &[(0, [H9,H8,HA,HZ]),(2, [EO,E9,E7,E8]),(2, [HO,EK,HU,H7]),(2, [GO,GU,SO,S7]),(2, [EU,S9,G7,G8]),(2, [SU,SK,S8,G9]),(2, [EA,GZ,SZ,HK]),(2, [EZ,SA,GA,GK]),],
        [-100, -100, 300, -100],
    );
    test_rules(
        "../../testdata/games/solo/219-gras-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SU,GA,HZ,H8,H7,SA,S9,S7],[GO,HU,G8,EZ,EK,E8,E7,S8],[EO,SO,GU,GZ,GK,G9,HA,HK],[HO,EU,G7,EA,E9,H9,SZ,SK],],
        vec![],
        vec![],
        &[(0, [H8,HU,HK,H9]),(1, [EZ,GU,E9,S7]),(2, [EO,G7,SU,G8]),(2, [G9,EU,GA,GO]),(1, [S8,GZ,SK,S9]),(2, [SO,HO,H7,EK]),(3, [EA,HZ,E7,GK]),(2, [HA,SZ,SA,E8]),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/22-eichel-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,E8,GZ,G8,G7,HA,SZ,S8],[HU,G9,HZ,H9,H8,SA,S9,S7],[EO,SU,EA,EK,E7,GK,HK,H7],[GO,HO,SO,EU,EZ,E9,GA,SK],],
        vec![],
        vec![2,],
        &[(0, [HA,H9,HK,EZ]),(3, [EU,E8,HU,E7]),(3, [SO,GU,HZ,EO]),(2, [GK,GA,G8,G9]),(3, [HO,G7,H8,EK]),(3, [GO,S8,S7,SU]),(3, [SK,SZ,SA,H7]),(1, [S9,EA,E9,GZ]),],
        [100, 100, 100, -300],
    );
    test_rules(
        "../../testdata/games/solo/220-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,HO,GZ,G7,EA,EZ,HK],[SO,EU,HU,GA,GK,HA,SA,S9],[GU,SU,G9,G8,E9,H8,SK,S7],[EK,E8,E7,HZ,H9,H7,SZ,S8],],
        vec![],
        vec![],
        &[(0, [EO,GK,G8,E7]),(0, [HO,HU,G9,E8]),(0, [GO,EU,SU,H7]),(0, [EA,GA,E9,EK]),(1, [HA,H8,H9,HK]),(1, [SA,S7,S8,GZ]),(0, [G7,SO,GU,SZ]),(1, [S9,SK,HZ,EZ]),],
        [-240, 80, 80, 80],
    );
    test_rules(
        "../../testdata/games/solo/221-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,HO,GU,HU,EK,E9,HA,H7],[GO,SO,EU,SU,E7,G7,HK,SZ],[EZ,E8,GA,G9,G8,H9,SK,S8],[EA,GZ,GK,HZ,H8,SA,S9,S7],],
        vec![],
        vec![],
        &[(0, [EO,E7,E8,EA]),(0, [HU,EU,EZ,GZ]),(1, [HK,H9,H8,HA]),(0, [GU,SO,GA,HZ]),(1, [G7,G8,GK,EK]),(0, [H7,SU,SK,SA]),(1, [SZ,S8,S7,HO]),(0, [E9,GO,G9,S9]),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/23-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/25-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,HO,SO,HU,GA,G8,HA,S9],[GZ,G9,G7,EA,EZ,H9,H8,S7],[EO,EU,GU,E8,E7,HZ,HK,S8],[SU,GK,EK,E9,H7,SA,SZ,SK],],
        vec![],
        vec![],
        &[(0, [SO,GZ,EO,GK]),(2, [S8,SZ,S9,S7]),(3, [SA,HU,G7,GU]),(2, [E7,E9,G8,EZ]),(0, [HO,G9,EU,SU]),(0, [GO,H8,E8,H7]),(0, [GA,EA,HK,SK]),(0, [HA,H9,HZ,EK]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/26-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,HO,GU,GA,G9,EA,E8,HA],[GO,SU,G8,EZ,E7,HK,SA,S9],[SO,HU,GZ,G7,E9,HZ,H7,SZ],[EU,GK,EK,H9,H8,SK,S8,S7],],
        vec![],
        vec![],
        &[(0, [GU,SU,GZ,EU]),(3, [S7,GA,S9,SZ]),(0, [EO,G8,G7,GK]),(0, [G9,GO,HU,EK]),(1, [HK,H7,H9,HA]),(0, [HO,E7,SO,H8]),(0, [EA,EZ,E9,S8]),(0, [E8,SA,HZ,SK]),],
        [180, -60, -60, -60],
    );
    // ../../testdata/games/solo/27-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/29-herz-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HU,HK,H9,E9,GK,G7,SK,S9],[EZ,EK,E7,GZ,G9,G8,SA,SZ],[HO,SU,HA,H7,EA,E8,S8,S7],[EO,GO,SO,EU,GU,HZ,H8,GA],],
        vec![],
        vec![],
        &[(0, [E9,EK,EA,HZ]),(3, [EO,H9,E7,H7]),(3, [GO,HK,G8,SU]),(3, [GU,HU,EZ,HO]),(2, [S7,H8,S9,SZ]),(3, [EU,SK,SA,HA]),(3, [SO,G7,G9,S8]),(3, [GA,GK,GZ,E8]),],
        [-60, -60, -60, 180],
    );
    test_rules(
        "../../testdata/games/solo/30-gras-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SO,GA,G9,E7,HK,H9,H8,S9],[EU,GU,EK,HZ,H7,SZ,S8,S7],[HO,SU,GK,G7,EZ,E9,E8,SK],[EO,GO,HU,GZ,G8,EA,HA,SA],],
        vec![],
        vec![],
        &[(0, [H9,H7,GK,HA]),(2, [EZ,EA,E7,EK]),(3, [GO,G9,GU,G7]),(3, [EO,SO,EU,SU]),(3, [G8,GA,HZ,HO]),(2, [E9,GZ,S9,S7]),(3, [HU,H8,S8,E8]),(3, [SA,HK,SZ,SK]),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/31-herz-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,GU,EK,E9,G7,SZ,SK,S9],[HO,EU,H7,EZ,GA,GK,G8,SA],[SU,HZ,H9,E8,GZ,G9,S8,S7],[EO,SO,HU,HA,HK,H8,EA,E7],],
        vec![],
        vec![],
        &[(0, [SK,SA,S7,HA]),(3, [EO,GU,H7,H9]),(3, [H8,GO,EU,HZ]),(0, [G7,GA,G9,HK]),(3, [HU,SZ,HO,SU]),(1, [EZ,E8,EA,E9]),(3, [SO,S9,G8,S8]),(3, [E7,EK,GK,GZ]),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/32-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,GU,EA,E9,E7,G9,SA],[SU,EZ,EK,E8,GA,G7,H9,S8],[HO,EU,HU,GK,G8,HA,H8,SZ],[SO,GZ,HZ,HK,H7,SK,S9,S7],],
        vec![],
        vec![],
        &[(0, [GO,E8,HU,SO]),(0, [EO,EK,EU,S7]),(0, [GU,EZ,HO,GZ]),(2, [HA,HK,E9,H9]),(0, [E7,SU,SZ,HZ]),(1, [GA,GK,SK,G9]),(1, [S8,G8,S9,SA]),(0, [EA,G7,H8,H7]),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/34-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/36-herz-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,H8,EA,EK,E8,GK,SK,S8],[EO,EU,SU,HA,HZ,HK,H9,SA],[SO,H7,EZ,E7,GA,GZ,G9,SZ],[HO,GU,HU,E9,G8,G7,S9,S7],],
        vec![],
        vec![],
        &[(0, [GK,HZ,G9,G8]),(1, [H9,H7,HU,H8]),(3, [G7,GO,EO,GZ]),(1, [SU,SO,HO,EA]),(3, [E9,EK,HK,E7]),(1, [EU,EZ,GU,E8]),(1, [HA,GA,S7,S8]),(1, [SA,SZ,S9,SK]),],
        [-60, 180, -60, -60],
    );
    test_rules(
        "../../testdata/games/solo/37-gras-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HU,SU,GA,E9,E7,H8,H7,S9],[HO,SO,G9,G7,EZ,HA,HK,S8],[GU,EK,E8,HZ,H9,SA,SZ,SK],[EO,GO,EU,GZ,GK,G8,EA,S7],],
        vec![],
        vec![],
        &[(0, [S9,S8,SA,S7]),(2, [SZ,EU,H7,SO]),(1, [HA,H9,GZ,H8]),(3, [GO,SU,G7,GU]),(3, [EO,HU,G9,E8]),(3, [G8,GA,HO,HZ]),(1, [HK,EK,GK,E7]),(3, [EA,E9,EZ,SK]),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/38-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,SO,GU,GA,GZ,G9,G8,HA],[HO,HU,EZ,E7,HK,SA,SK,S9],[EO,EU,EA,E9,E8,H9,H8,S7],[SU,GK,G7,EK,HZ,H7,SZ,S8],],
        vec![],
        vec![],
        &[(0, [GU,HO,EU,GK]),(1, [SA,S7,S8,GA]),(0, [G8,HU,EO,G7]),(2, [H9,H7,HA,HK]),(0, [GO,EZ,H8,SU]),(0, [SO,S9,E8,EK]),(0, [GZ,E7,E9,HZ]),(0, [G9,SK,EA,SZ]),],
        [180, -60, -60, -60],
    );
    // ../../testdata/games/solo/39-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/4-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,HO,SO,G9,G7,EA,S7],[HU,SU,GA,EZ,E9,E7,HZ,H7],[EU,GK,G8,EK,HA,H8,SA,S9],[GU,GZ,E8,HK,H9,SZ,SK,S8],],
        vec![],
        vec![],
        &[(0, [EO,SU,G8,GU]),(0, [GO,HU,GK,GZ]),(0, [SO,GA,EU,E8]),(0, [S7,EZ,SA,SZ]),(2, [EK,HK,EA,E7]),(0, [HO,E9,S9,H9]),(0, [G7,HZ,H8,S8]),(0, [G9,H7,HA,SK]),],
        [270, -90, -90, -90],
    );
    test_rules(
        "../../testdata/games/solo/40-herz-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,HU,SU,H8,EA,E7,GA,G8],[SO,H9,EZ,E8,G9,G7,SZ,S7],[HA,HZ,EK,GZ,GK,SA,SK,S9],[EO,GO,HO,EU,HK,H7,E9,S8],],
        vec![],
        vec![],
        &[(0, [EA,E8,EK,E9]),(0, [E7,EZ,SK,HK]),(3, [EO,H8,H9,HZ]),(3, [HO,SU,SO,HA]),(3, [EU,HU,G9,S9]),(3, [GO,GU,G7,SA]),(3, [S8,GA,SZ,GZ]),(1, [S7,GK,H7,G8]),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/solo/41-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SO,HU,SU,EZ,E8,GA,HA,SA],[GO,EK,G9,HK,H8,SZ,S9,S7],[EO,HO,E9,GK,HZ,H9,H7,SK],[EU,GU,EA,E7,GZ,G8,G7,S8],],
        vec![],
        vec![],
        &[(0, [SU,EK,E9,GU]),(3, [G8,GA,G9,GK]),(0, [HU,GO,HO,EA]),(1, [S7,SK,S8,SA]),(0, [E8,SZ,EO,E7]),(2, [H7,EU,HA,HK]),(3, [G7,EZ,S9,H9]),(0, [SO,H8,HZ,GZ]),],
        [240, -80, -80, -80],
    );
    test_rules(
        "../../testdata/games/solo/44-gras-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,GZ,EA,EK,E7,SA,S9,S8],[EO,GO,HO,SO,GU,GK,E8,HK],[HU,SU,G9,G7,HA,HZ,H9,SK],[GA,G8,EZ,E9,H8,H7,SZ,S7],],
        vec![],
        vec![],
        &[(0, [SA,GK,SK,S7]),(1, [GO,G7,GA,EU]),(1, [EO,G9,G8,GZ]),(1, [GU,SU,SZ,S8]),(1, [E8,HA,EZ,EA]),(0, [EK,HK,HZ,E9]),(0, [E7,HO,H9,H8]),(1, [SO,HU,H7,S9]),],
        [-90, 270, -90, -90],
    );
    // ../../testdata/games/solo/46-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/49-herz-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,HU,HA,H7,E8,E7,GA,G9],[EZ,GZ,G7,SA,SZ,SK,S9,S8],[SU,HZ,HK,H9,H8,EA,EK,E9],[EO,GO,HO,SO,GU,GK,G8,S7],],
        vec![],
        vec![3,],
        &[(0, [E7,EZ,EA,GU]),(3, [EO,HA,SA,H8]),(3, [GO,H7,SZ,H9]),(3, [HO,HU,GZ,SU]),(3, [SO,EU,SK,HK]),(3, [GK,GA,G7,HZ]),(2, [EK,S7,E8,S9]),(2, [E9,G8,G9,S8]),],
        [260, 260, -780, 260],
    );
    // ../../testdata/games/solo/5-gras-solo.html has wrong format
    // ../../testdata/games/solo/50-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/51-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,HO,SU,EZ,E9,GA,HA,HK],[SO,HU,EK,E8,G9,G8,HZ,S9],[GU,E7,H9,H7,SA,SK,S8,S7],[GO,EU,EA,GZ,GK,G7,H8,SZ],],
        vec![],
        vec![],
        &[(0, [HO,E8,E7,GO]),(3, [GK,GA,G8,GU]),(2, [SA,SZ,EZ,S9]),(0, [EO,HU,H7,EU]),(0, [E9,EK,SK,EA]),(3, [H8,HA,HZ,H9]),(0, [SU,SO,S8,GZ]),(1, [G9,S7,G7,HK]),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/52-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/53-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,HO,EU,EA,E9,E8,G9,SA],[GU,HU,SU,E7,GA,GZ,SK,S8],[EZ,EK,GK,HA,H8,H7,SZ,S9],[GO,SO,G8,G7,HZ,HK,H9,S7],],
        vec![],
        vec![],
        &[(0, [HO,E7,EZ,GO]),(3, [G8,G9,GA,GK]),(1, [S8,S9,S7,SA]),(0, [EO,SU,EK,SO]),(0, [EU,HU,H7,G7]),(0, [E9,GU,SZ,HZ]),(1, [SK,H8,H9,E8]),(0, [EA,GZ,HA,HK]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/54-gras-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,G9,EZ,HZ,H9,S9,S8,S7],[GO,SO,GU,SU,EA,E7,HK,H8],[HO,G7,EK,E8,H7,SA,SZ,SK],[EO,HU,GA,GZ,GK,G8,E9,HA],],
        vec![],
        vec![],
        &[(0, [EZ,E7,E8,E9]),(0, [S7,SU,SK,HU]),(3, [G8,G9,GU,G7]),(1, [EA,EK,GK,EU]),(0, [S8,H8,SZ,GA]),(3, [EO,S9,SO,HO]),(3, [HA,HZ,HK,H7]),(3, [GZ,H9,GO,SA]),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/55-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,SO,GU,GK,G9,G8,E9],[GA,G7,EZ,EK,E7,HA,H7,SK],[EU,HU,SU,GZ,H9,H8,SA,S7],[HO,EA,E8,HZ,HK,SZ,S9,S8],],
        vec![],
        vec![],
        &[(0, [GO,GA,SU,HO]),(0, [SO,G7,HU,S8]),(0, [EO,E7,GZ,E8]),(0, [GU,SK,EU,SZ]),(2, [SA,S9,GK,H7]),(0, [E9,EZ,S7,EA]),(3, [HK,G9,HA,H8]),(0, [G8,EK,H9,HZ]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/57-eichel-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,EZ,GZ,G9,G7,HZ,H9,H7],[EO,HO,HU,SU,EA,E9,E7,S7],[GO,EK,GA,H8,SZ,SK,S9,S8],[SO,GU,E8,GK,G8,HA,HK,SA],],
        vec![],
        vec![],
        &[(0, [G9,EA,GA,G8]),(1, [EO,EK,E8,EU]),(1, [SU,GO,GU,EZ]),(2, [SZ,SA,HZ,S7]),(3, [HA,H7,HU,H8]),(1, [HO,S8,SO,H9]),(1, [E7,SK,HK,GZ]),(1, [E9,S9,GK,G7]),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/58-eichel-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HO,EU,EZ,E7,GZ,G8,SA,SZ],[HU,E9,G9,HA,HK,H8,H7,S9],[SO,GU,GK,G7,HZ,H9,SK,S7],[EO,GO,SU,EA,EK,E8,GA,S8],],
        vec![],
        vec![],
        &[(0, [SA,S9,SK,S8]),(0, [SZ,HU,S7,GO]),(3, [EO,E7,E9,GU]),(3, [SU,EU,HA,SO]),(2, [H9,EA,G8,H7]),(3, [GA,GZ,G9,G7]),(3, [E8,EZ,HK,HZ]),(0, [HO,H8,GK,EK]),],
        [50, 50, 50, -150],
    );
    test_rules(
        "../../testdata/games/solo/59-herz-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,EU,H8,EK,E7,GK,S8],[HO,HU,H9,G9,G8,SZ,SK,S9],[SO,GU,SU,HA,HZ,H7,EA,GA],[HK,EZ,E9,E8,GZ,G7,SA,S7],],
        vec![],
        vec![],
        &[(0, [S8,SK,HA,S7]),(2, [GU,HK,EU,H9]),(0, [GK,G8,GA,G7]),(2, [SU,SA,H8,HU]),(1, [S9,H7,E8,E7]),(2, [EA,EZ,EK,HO]),(1, [G9,HZ,GZ,GO]),(0, [EO,SZ,SO,E9]),],
        [90, 90, -270, 90],
    );
    test_rules(
        "../../testdata/games/solo/6-herz-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,HK,EA,E7,G9,G8,G7,S9],[HA,HZ,H7,E8,GZ,SZ,SK,S8],[HO,EU,HU,EZ,EK,E9,SA,S7],[EO,SO,GU,SU,H9,H8,GA,GK],],
        vec![],
        vec![],
        &[(0, [G9,GZ,EU,GK]),(2, [EZ,GU,E7,E8]),(3, [EO,HK,H7,HU]),(3, [SU,GO,HA,HO]),(0, [G8,HZ,SA,GA]),(1, [SK,S7,H8,S9]),(3, [SO,G7,S8,E9]),(3, [H9,EA,SZ,EK]),],
        [50, 50, 50, -150],
    );
    // ../../testdata/games/solo/62-herz-solo.html has wrong format
    // ../../testdata/games/solo/63-herz-solo.html has wrong format
    // ../../testdata/games/solo/64-herz-solo.html has wrong format
    // ../../testdata/games/solo/66-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/67-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,HO,EU,GU,SU,G9,G7],[SO,G8,E9,E8,HA,H8,SK,S9],[HU,GZ,GK,EA,EZ,EK,HZ,HK],[GA,E7,H9,H7,SA,SZ,S8,S7],],
        vec![],
        vec![],
        &[(0, [EO,G8,GK,GA]),(0, [GO,SO,HU,E7]),(0, [HO,E9,GZ,H7]),(0, [EU,E8,EK,H9]),(0, [GU,S9,HK,S7]),(0, [SU,SK,EZ,S8]),(0, [G9,H8,HZ,SZ]),(0, [G7,HA,EA,SA]),],
        [300, -100, -100, -100],
    );
    test_rules(
        "../../testdata/games/solo/68-gras-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,G8,EA,E9,E8,E7,HA,S8],[SO,G9,HK,H9,H7,SK,S9,S7],[EO,HO,GU,SU,GZ,GK,G7,HZ],[EU,HU,GA,EZ,EK,H8,SA,SZ],],
        vec![],
        vec![],
        &[(0, [EA,SO,HO,EK]),(2, [EO,HU,G8,G9]),(2, [SU,GA,GO,HK]),(0, [E9,H7,HZ,EZ]),(3, [SA,S8,S7,GZ]),(2, [GU,EU,HA,SK]),(3, [H8,E7,H9,G7]),(2, [GK,SZ,E8,S9]),],
        [-50, -50, 150, -50],
    );
    // ../../testdata/games/solo/7-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/70-gras-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,HU,EZ,E7,H8,SZ,SK,S7],[EU,GZ,G7,E9,HA,H9,SA,S8],[EO,SO,GU,SU,GA,GK,G9,E8],[HO,G8,EA,EK,HZ,HK,H7,S9],],
        vec![],
        vec![],
        &[(0, [SK,SA,GA,S9]),(2, [SO,HO,GO,GZ]),(0, [SZ,S8,EO,G8]),(2, [G9,HZ,HU,G7]),(0, [S7,E9,E8,EA]),(0, [EZ,EU,SU,EK]),(1, [HA,GK,HK,H8]),(2, [GU,H7,E7,H9]),],
        [50, 50, -150, 50],
    );
    test_rules(
        "../../testdata/games/solo/72-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,EU,GU,G9,G8,EA,S7],[SO,GZ,EK,E7,HA,HK,H7,SA],[HO,GA,EZ,E9,HZ,H9,H8,S8],[HU,SU,GK,G7,E8,SZ,SK,S9],],
        vec![],
        vec![],
        &[(0, [GO,GZ,GA,G7]),(0, [EO,SO,HO,SU]),(0, [GU,EK,S8,GK]),(0, [EU,E7,H8,HU]),(0, [EA,H7,E9,E8]),(0, [S7,SA,HZ,SZ]),(1, [HA,H9,S9,G8]),(0, [G9,HK,EZ,SK]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/73-herz-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,H8,EZ,EK,E9,GA,GK,S9],[HU,HK,H7,GZ,G8,G7,SK,S8],[EO,GO,SU,HZ,EA,E7,SA,S7],[HO,SO,GU,HA,H9,E8,G9,SZ],],
        vec![],
        vec![],
        &[(0, [GA,G7,HZ,G9]),(2, [GO,H9,H8,H7]),(2, [EO,GU,EU,HK]),(2, [EA,E8,EK,HU]),(1, [SK,SA,SZ,S9]),(2, [E7,HA,EZ,GZ]),(3, [HO,GK,G8,SU]),(3, [SO,E9,S8,S7]),],
        [50, 50, -150, 50],
    );
    test_rules(
        "../../testdata/games/solo/74-eichel-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EU,EA,E8,GA,G7,SA,SZ,S9],[GO,GU,E7,GZ,G9,H7,SK,S7],[EK,E9,GK,G8,HZ,H9,H8,S8],[EO,HO,SO,HU,SU,EZ,HA,HK],],
        vec![],
        vec![],
        &[(0, [GA,GZ,G8,EZ]),(3, [SO,EA,GO,EK]),(1, [H7,H9,HA,EU]),(0, [SA,SK,S8,SU]),(3, [HO,E8,E7,E9]),(3, [EO,G7,GU,GK]),(3, [HK,SZ,S7,HZ]),(2, [H8,HU,S9,G9]),],
        [-50, -50, -50, 150],
    );
    // ../../testdata/games/solo/75-herz-solo.html has wrong format
    // ../../testdata/games/solo/76-gras-solo.html has wrong format
    // ../../testdata/games/solo/79-herz-solo.html has wrong format
    // ../../testdata/games/solo/81-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/82-gras-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GU,GA,GK,EA,EZ,E7,SK,S7],[SO,G9,G8,E9,HA,HZ,H8,S8],[HU,SU,E8,HK,H9,H7,SZ,S9],[EO,GO,HO,EU,GZ,G7,EK,SA],],
        vec![],
        vec![],
        &[(0, [SK,S8,S9,SA]),(3, [GO,GA,G8,SU]),(3, [HO,GU,G9,HU]),(3, [EO,GK,SO,E8]),(3, [G7,EZ,HA,H7]),(3, [EK,EA,E9,SZ]),(0, [S7,H8,H9,GZ]),(3, [EU,E7,HZ,HK]),],
        [-90, -90, -90, 270],
    );
    // ../../testdata/games/solo/83-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/84-eichel-solo.html",
        sololike(EPlayerIndex::EPI2, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[SO,EU,GU,SU,G9,G8,H9,SA],[HO,E9,HK,H8,SZ,SK,S8,S7],[EO,GO,HU,EA,EZ,EK,E7,GA],[E8,GZ,GK,G7,HA,HZ,H7,S9],],
        vec![],
        vec![],
        &[(0, [SA,SZ,HU,S9]),(2, [GO,E8,SU,E9]),(2, [EO,G7,GU,HO]),(2, [E7,HA,EU,HK]),(0, [H9,H8,EA,H7]),(2, [EK,HZ,SO,S7]),(0, [G9,S8,GA,GK]),(2, [EZ,GZ,G8,SK]),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/86-eichel-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,EZ,GA,HZ,HK,SA,S9,S8],[GO,EU,E8,E7,G7,HA,H9,SZ],[HO,HU,SU,GZ,GK,G9,H8,S7],[SO,GU,EA,EK,E9,G8,H7,SK],],
        vec![],
        vec![],
        &[(0, [GA,G7,G9,G8]),(0, [HK,HA,H8,H7]),(1, [H9,SU,SK,HZ]),(2, [GZ,GU,EO,SZ]),(0, [S8,EU,S7,SO]),(3, [E9,EZ,E7,HU]),(2, [GK,EA,SA,GO]),(1, [E8,HO,EK,S9]),],
        [90, 90, 90, -270],
    );
    test_rules(
        "../../testdata/games/solo/87-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[GO,SO,EU,HU,GZ,GK,EK,SA],[GU,G8,E9,E7,HA,HZ,H9,H7],[EO,HO,G9,G7,EZ,E8,H8,SZ],[SU,GA,EA,HK,SK,S9,S8,S7],],
        vec![],
        vec![],
        &[(0, [EU,G8,HO,GA]),(2, [SZ,SK,SA,GU]),(1, [E7,E8,EA,EK]),(3, [S9,HU,E9,H8]),(0, [SO,HZ,EO,SU]),(2, [EZ,HK,GK,H7]),(0, [GO,H9,G7,S7]),(0, [GZ,HA,G9,S8]),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/9-herz-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,EU,HZ,HK,H9,GA,SA,S7],[GU,E9,E8,E7,GK,G7,SZ,SK],[GO,HO,H8,EA,EZ,EK,GZ,S9],[SO,HU,SU,HA,H7,G9,G8,S8],],
        vec![],
        vec![],
        &[(0, [EO,GU,H8,H7]),(0, [H9,SZ,HO,HA]),(2, [EA,S8,HZ,E7]),(0, [GA,G7,GZ,G8]),(0, [SA,SK,S9,SU]),(3, [G9,HK,GK,GO]),(2, [EK,HU,EU,E8]),(0, [S7,E9,EZ,SO]),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/90-herz-solo.html has wrong format
    // ../../testdata/games/solo/91-gras-solo.html has wrong format
    // ../../testdata/games/solo/92-herz-solo.html has wrong format
    // ../../testdata/games/solo/93-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/94-eichel-solo.html",
        sololike(EPlayerIndex::EPI1, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HU,SU,E8,G9,SZ,SK,S9,S8],[EO,GO,HO,SO,EU,EK,E7,H7],[GU,EA,GA,GZ,HA,HZ,HK,H9],[EZ,E9,GK,G8,G7,H8,SA,S7],],
        vec![],
        vec![],
        &[(0, [SZ,EU,H9,S7]),(1, [GO,GU,EZ,E8]),(1, [EO,EA,E9,SU]),(1, [SO,HA,SA,HU]),(1, [HO,HK,H8,G9]),(1, [H7,HZ,GK,SK]),(2, [GA,G7,S8,E7]),(1, [EK,GZ,G8,S9]),],
        [-110, 330, -110, -110],
    );
    test_rules(
        "../../testdata/games/solo/96-herz-solo.html",
        sololike(EPlayerIndex::EPI3, EFarbe::Herz, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[H7,EA,EZ,EK,E9,E7,GK,SZ],[GU,HK,E8,GA,GZ,G8,S9,S8],[HO,HA,HZ,H8,G7,SA,SK,S7],[EO,GO,SO,EU,HU,SU,H9,G9],],
        vec![],
        vec![],
        &[(0, [GK,GA,G7,G9]),(1, [GZ,SK,EU,H7]),(3, [GO,EZ,HK,H8]),(3, [EO,E7,GU,HZ]),(3, [SU,SZ,E8,HO]),(2, [SA,H9,E9,S8]),(3, [SO,EK,S9,HA]),(3, [HU,EA,G8,S7]),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/97-gras-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Gras, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[EO,GO,HO,EU,GA,G9,E8,SA],[HU,GZ,GK,G7,EK,E9,S9,S8],[SO,GU,SU,E7,HZ,H9,H8,S7],[G8,EA,EZ,HA,HK,H7,SZ,SK],],
        vec![],
        vec![],
        &[(0, [GO,G7,SU,G8]),(0, [HO,GK,GU,H7]),(0, [EO,HU,SO,HK]),(0, [EU,GZ,E7,SK]),(0, [E8,EK,HZ,EA]),(3, [EZ,G9,E9,S7]),(0, [SA,S8,H8,SZ]),(0, [GA,S9,H9,HA]),],
        [270, -90, -90, -90],
    );
    test_rules(
        "../../testdata/games/solo/98-eichel-solo.html",
        sololike(EPlayerIndex::EPI0, EFarbe::Eichel, ESoloLike::Solo, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3))).upcast(),
        [[HO,EU,HU,EZ,E9,E7,SA,SK],[SO,GU,SU,E8,GZ,G9,H9,S7],[EO,EK,G7,HZ,HK,H8,H7,S9],[GO,EA,GA,GK,G8,HA,SZ,S8],],
        vec![],
        vec![],
        &[(0, [EU,SO,EK,EA]),(1, [S7,S9,S8,SA]),(0, [HU,E8,EO,GO]),(2, [HK,HA,E9,H9]),(0, [HO,SU,G7,G8]),(0, [E7,GU,HZ,GA]),(1, [GZ,H8,GK,EZ]),(0, [SK,G9,H7,SZ]),],
        [-150, 50, 50, 50],
    );
}

#[test]
fn test_rulesgeier() {
    test_rules(
        "../../testdata/games/39.html",
        sololike(EPlayerIndex::EPI2, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EO,EZ,EU,GK,G9,H8,SU,S9],[GO,HO,E7,GZ,G8,HK,HU,S7],[SO,EA,EK,GA,G7,HA,SA,SZ],[E9,E8,GU,HZ,H9,H7,SK,S8],],
        vec![],
        vec![],
        &[(0, [H8,HU,HA,H7]),(2, [SO,SK,EO,HO]),(0, [S9,S7,SA,S8]),(2, [SZ,E8,SU,GO]),(1, [GZ,GA,GU,G9]),(2, [EA,E9,EU,E7]),(2, [G7,H9,GK,G8]),(0, [EZ,HK,EK,HZ]),],
        [80, 80, -240, 80],
    );
    test_rules(
        "../../testdata/games/42.html",
        sololike(EPlayerIndex::EPI2, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[E9,E7,GA,GZ,HU,H8,S9,S8],[GO,EK,GU,G9,H7,SA,SZ,S7],[EO,HO,EA,EZ,G8,HA,HZ,HK],[SO,EU,E8,GK,G7,H9,SK,SU],],
        vec![],
        vec![],
        &[(0, [GA,GU,G8,GK]),(0, [GZ,G9,HO,G7]),(2, [EO,SO,E7,GO]),(2, [HA,H9,H8,H7]),(2, [EZ,E8,E9,EK]),(2, [HZ,EU,HU,S7]),(2, [EA,SU,S8,SZ]),(2, [HK,SK,S9,SA]),],
        [-60, -60, 180, -60],
    );
    test_rules(
        "../../testdata/games/geier/1.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/200, /*n_payout_schneider_schwarz*/50, SLaufendeParams::new(10, 2))).upcast(),
        [[EO,SO,EA,EU,E7,G8,SA,S7],[GO,HO,HA,HK,H9,SZ,SK,S9],[EZ,E9,E8,GK,GU,G9,H7,SU],[EK,GA,GZ,G7,HZ,HU,H8,S8],],
        vec![],
        vec![],
        &[(0, [EO,HO,H7,S8]),(0, [E7,SZ,EZ,EK]),(2, [SU,GZ,SA,S9]),(0, [EA,GO,E8,GA]),(1, [HA,GK,HZ,SO]),(0, [EU,SK,E9,G7]),(0, [S7,HK,G9,H8]),(0, [G8,H9,GU,HU]),],
        [600, -200, -200, -200],
    );
    test_rules(
        "../../testdata/games/geier/10.html",
        sololike(EPlayerIndex::EPI3, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[GO,E9,E7,GZ,HK,H7,SU,S7],[SO,EK,G9,G7,H9,SK,S9,S8],[HO,E8,GK,GU,G8,HZ,HU,SZ],[EO,EA,EZ,EU,GA,HA,H8,SA],],
        vec![],
        vec![],
        &[(0, [E7,EK,E8,EA]),(3, [EO,GO,SO,HO]),(3, [EZ,E9,H9,SZ]),(3, [SA,S7,S8,G8]),(3, [GA,GZ,G7,GU]),(3, [HA,H7,G9,HU]),(3, [EU,HK,S9,HZ]),(3, [H8,SU,SK,GK]),],
        [-70, -70, -70, 210],
    );
    test_rules(
        "../../testdata/games/geier/2.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EO,HO,SO,EZ,E9,GZ,GK,SA],[GO,EU,GA,G8,HU,H9,H8,S9],[EA,G9,G7,HA,HZ,H7,SZ,SK],[EK,E8,E7,GU,HK,SU,S8,S7],],
        vec![],
        vec![],
        &[(0, [EO,GO,G7,S7]),(0, [GZ,GA,G9,GU]),(1, [S9,SK,S8,SA]),(0, [EZ,EU,EA,EK]),(2, [HA,HK,HO,H8]),(0, [SO,G8,H7,E7]),(0, [GK,H9,HZ,E8]),(0, [E9,HU,SZ,SU]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/geier/3.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Geier, SPayoutDeciderTout::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EO,SO,EA,EZ,SA,SZ,SU,S9],[E8,E7,G7,HA,HZ,H7,S8,S7],[HO,GA,GZ,GK,G9,G8,HU,H9],[GO,EK,EU,E9,GU,HK,H8,SK],],
        vec![],
        vec![],
        &[(0, [EO,G7,HO,GO]),(0, [EA,E8,H9,E9]),(0, [EZ,E7,HU,EU]),(0, [SA,S8,G8,SK]),(0, [SZ,S7,G9,H8]),(0, [SU,H7,GK,HK]),(0, [S9,HZ,GZ,GU]),(0, [SO,HA,GA,EK]),],
        [300, -100, -100, -100],
    );
    test_rules(
        "../../testdata/games/geier/4.html",
        sololike(EPlayerIndex::EPI3, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EO,EA,GZ,GK,G9,HU,SA,S7],[EK,EU,GA,G7,HZ,HK,H8,S9],[GU,G8,HA,H7,SZ,SK,SU,S8],[GO,HO,SO,EZ,E9,E8,E7,H9],],
        vec![],
        vec![],
        &[(0, [EA,EK,SZ,E7]),(0, [SA,S9,SK,H9]),(0, [HU,HZ,HA,SO]),(3, [GO,EO,GA,SU]),(0, [GZ,G7,G8,HO]),(3, [EZ,S7,EU,S8]),(3, [E9,G9,H8,H7]),(3, [E8,GK,HK,GU]),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/geier/5.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[HO,SO,EA,EZ,GA,H9,SA,S7],[EO,GZ,G7,HA,HZ,HU,H8,SU],[EK,E8,E7,G9,G8,HK,H7,SK],[GO,EU,E9,GK,GU,SZ,S9,S8],],
        vec![3,0,1,],
        vec![],
        &[(0, [HO,EO,SK,GO]),(1, [HA,HK,SZ,H9]),(1, [HZ,H7,GK,S7]),(1, [HU,G9,GU,SO]),(0, [EA,H8,E7,E9]),(0, [EZ,SU,E8,EU]),(0, [GA,G7,G8,S8]),(0, [SA,GZ,EK,S9]),],
        [1680, -560, -560, -560],
    );
    test_rules(
        "../../testdata/games/geier/6.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[GO,HO,SO,EZ,GZ,GK],[E9,GU,HZ,HK,SA,SU],[EO,EA,EK,G9,HA,H9],[EU,GA,HU,SZ,SK,S9],],
        vec![2,],
        vec![],
        &[(0, [GO,E9,EO,EU]),(2, [G9,GA,GK,GU]),(3, [SZ,SO,SU,H9]),(0, [GZ,SA,EK,S9]),(0, [HO,HK,HA,SK]),(0, [EZ,HZ,EA,HU]),],
        [300, -100, -100, -100],
    );
    test_rules(
        "../../testdata/games/geier/7.html",
        sololike(EPlayerIndex::EPI3, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[GZ,GK,HA,HK,HU,H8,H7,S8],[EU,E9,E7,GU,G8,G7,SZ,SU],[GO,SO,EK,GA,G9,H9,SK,S7],[EO,HO,EA,EZ,E8,HZ,SA,S9],],
        vec![3,1,],
        vec![],
        &[(0, [S8,SU,S7,SA]),(3, [EO,H7,G7,SO]),(3, [EA,H8,E7,EK]),(3, [EZ,HU,E9,H9]),(3, [E8,HK,EU,SK]),(1, [G8,GA,HO,GK]),(3, [S9,GZ,SZ,G9]),(1, [GU,GO,HZ,HA]),],
        [-200, -200, -200, 600],
    );
    test_rules(
        "../../testdata/games/geier/8.html",
        sololike(EPlayerIndex::EPI0, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[EO,SO,EA,EU,E9,E8,E7,G9],[GO,HO,EK,GA,G7,HA,HK,H7],[GK,G8,HU,H9,H8,SZ,S9,S7],[EZ,GZ,GU,HZ,SA,SK,SU,S8],],
        vec![],
        vec![],
        &[(0, [EO,HO,H8,S8]),(0, [EA,EK,H9,EZ]),(0, [E9,G7,S7,SU]),(0, [E8,H7,HU,SK]),(0, [EU,GO,SZ,HZ]),(1, [GA,GK,GU,G9]),(1, [HA,S9,GZ,SO]),(0, [E7,HK,G8,SA]),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/geier/9.html",
        sololike(EPlayerIndex::EPI1, None, ESoloLike::Geier, SPayoutDeciderPointBased::default_payoutdecider(/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2))).upcast(),
        [[HO,E8,GU,G8,HZ,HU,S9,S7],[EO,GO,SO,EZ,EK,E9,HK,H8],[GZ,G9,G7,HA,H9,SA,SK,SU],[EA,EU,E7,GA,GK,H7,SZ,S8],],
        vec![],
        vec![],
        &[(0, [E8,EK,GZ,EA]),(3, [GA,GU,GO,G7]),(1, [EO,G9,H7,HO]),(1, [H8,HA,SZ,HZ]),(2, [SK,S8,S7,SO]),(1, [EZ,H9,E7,G8]),(1, [HK,SU,EU,HU]),(1, [E9,SA,GK,S9]),],
        [-70, 210, -70, -70],
    );
}

#[test]
fn test_rulesramsch() {
    test_rules_manual(
        "0 has durchmarsch all",
        &SRulesRamsch::new(10, VDurchmarsch::All),
        vec![],
        vec![],
        /*n_stock*/20,
        &[
            (0, [EO,GO,HO,SO]),
            (0, [EU,GU,HU,SU]),
            (0, [HA,HZ,HK,H9]),
            (0, [EA,EZ,EK,E9]),
            (0, [GA,GZ,GK,G9]),
            (0, [SA,SZ,SK,S9]),
            (0, [E8,E7,G8,G7]),
            (0, [H8,H7,S8,S7]),
        ],
        ([30, -10, -10, -10], 0),
    );
    test_rules_manual(
        "0 has durchmarsch 120",
        &SRulesRamsch::new(10, VDurchmarsch::AtLeast(120)),
        vec![],
        vec![],
        /*n_stock*/160,
        &[
            (0, [EO,GO,HO,SO]),
            (0, [EU,GU,HU,SU]),
            (0, [HA,HZ,HK,H9]),
            (0, [EA,EZ,EK,E9]),
            (0, [GA,GZ,GK,G9]),
            (0, [SA,SZ,SK,S9]),
            (0, [E8,E7,G8,G7]),
            (0, [H8,H7,S8,S7]),
        ],
        ([30, -10, -10, -10], 0),
    );
    test_rules_manual(
        "0 has 120, but no durchmarsch",
        &SRulesRamsch::new(10, VDurchmarsch::All),
        vec![],
        vec![],
        /*n_stock*/40,
        &[
            (0, [EO,GO,HO,SO]),
            (0, [EU,GU,HU,SU]),
            (0, [HA,HZ,HK,H9]),
            (0, [EA,EZ,EK,E9]),
            (0, [GA,GZ,GK,G9]),
            (0, [SA,SZ,SK,S9]),
            (0, [E8,E7,G8,G7]),
            (0, [H7,H8,S8,S7]),
        ],
        ([-30, 10, 10, 10], 0),
    );
}

#[test]
fn test_rulesbettel() {
    test_rules_manual(
        "3 wins Bettel",
        &SRulesBettel::<SBettelAllAllowedCardsWithinStichNormal>::new(EPlayerIndex::EPI3, /*i_prio*/0, /*n_payout_base*/10),
        vec![],
        vec![],
        /*n_stock*/20,
        &[
            (0, [EO,EZ,EK,E9]),
            (2, [HO,H9,HA,HZ]),
            (0, [H8,H7,HU,SO]),
            (2, [G8,G9,GA,GO]),
            (0, [E8,E7,GK,SU]),
            (0, [SA,SZ,SK,S9]),
            (0, [EU,GZ,HK,S7]),
            (0, [EA,GU,S8,G7]),
        ],
        ([-10, -10, -10, 30], 0),
    );
    test_rules_manual(
        "2 looses Bettel",
        &SRulesBettel::<SBettelAllAllowedCardsWithinStichNormal>::new(EPlayerIndex::EPI2, /*i_prio*/0, /*n_payout_base*/10),
        vec![],
        vec![],
        /*n_stock*/40,
        &[
            (0, [EO,EZ,EK,E9]),
            (2, [HO,H9,HA,HZ]),
            (0, [H8,H7,HU,SO]),
            (2, [G8,G9,GA,GO]),
            (0, [E8,E7,GK,SU]),
            (0, [SA,SZ,SK,S9]),
            (0, [EU,GZ,HK,S7]),
            (0, [EA,GU,S8,G7]),
        ],
        ([10, 10, -30, 10], 0),
    );
}

#[test]
fn test_stock() {
    let rulesrufspiel = rulesrufspiel_new_test(
        EPlayerIndex::EPI0,
        EFarbe::Eichel,
        /*n_payout_base*/10,
        /*n_payout_schneider_schwarz*/10,
        SLaufendeParams::new(10, 3),
    );
    for n_stock_initial in [0isize, 20, 40, 80, 160, 240, 320].into_iter() {
        let n_stock_initial = *n_stock_initial;
        assert_eq!(n_stock_initial%2, 0);
        test_rules_manual(
            "Rufspiel: Players win stock",
            &rulesrufspiel,
            vec![],
            vec![],
            n_stock_initial,
            &[
                (0, [EO, GO, HO, SO]),
                (0, [EU, GU, HU, SU]),
                (0, [HA, HZ, HK, H9]),
                (0, [EZ, EA, EK, E9]),
                (1, [E8, E7, S7, S8]),
                (1, [SA, SZ, SK, S9]),
                (1, [GA, GZ, GK, H8]),
                (0, [H7, G9, G8, G7]),
            ],
            ([30+n_stock_initial/2, 30+n_stock_initial/2, -30, -30], -n_stock_initial),
        );
        test_rules_manual(
            "Rufspiel: Players win stock",
            &rulesrufspiel,
            vec![],
            vec![],
            n_stock_initial,
            &[
                (0, [EZ, EA, EK, H7]),
                (3, [EO, GO, HO, SO]),
                (3, [EU, GU, HU, SU]),
                (3, [HA, HZ, HK, H9]),
                (3, [SA, SZ, SK, S9]),
                (3, [GA, GZ, GK, G9]),
                (3, [G8, G7, E9, E8]),
                (3, [H8, E7, S8, S7]),
            ],
            ([-30-n_stock_initial/2, -30-n_stock_initial/2, 30, 30], n_stock_initial),
        );
    }
}
