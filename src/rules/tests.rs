use primitives::*;
use primitives::cardvector::parse_cards;
use rules::*;
use rules::ruleset::*;
use rules::rulesrufspiel::*;
use rules::rulessolo::*;
use rules::rulesramsch::*;
use rules::rulesbettel::*;
use rules::trumpfdecider::*;
use rules::payoutdecider::*;
use util::*;

fn internal_test_rules(
    str_info: &str,
    rules: &TRules,
    ahand: EnumMap<EPlayerIndex, SHand>,
    vecn_doubling: Vec<usize>,
    vecn_stoss: Vec<usize>,
    vecstich_test: &[SStich],
    an_payout: [isize; 4],
) {
    use game::*;
    println!("Testing rules: {}", str_info);
    let epi_first = EPlayerIndex::EPI0; // TODO parametrize w.r.t. epi_first
    let mut game = SGame::new(
        ahand,
        {
            let mut doublings = SDoublings::new(epi_first);
            for epi_doubling in EPlayerIndex::values().map(|epi| epi.wrapping_add(epi_first.to_usize())) {
                doublings.push(/*b_doubling*/vecn_doubling.contains(&epi_doubling.to_usize()));
            }
            doublings
        },
        Some(SStossParams::new(
            /*n_stoss_max*/4,
        )),
        rules.box_clone(),
        /*n_stock*/ 0, // TODO test stock
    );
    for n_epi_stoss in vecn_stoss {
        game.stoss(EPlayerIndex::from_usize(n_epi_stoss)).unwrap();
    }
    for (i_stich, stich) in vecstich_test.iter().enumerate() {
        println!("Stich {}: {}", i_stich, stich);
        assert_eq!(Some(stich.first_playerindex()), game.which_player_can_do_something().map(|gameaction| gameaction.0));
        for (epi, card) in stich.iter() {
            assert_eq!(Some(epi), game.which_player_can_do_something().map(|gameaction| gameaction.0));
            println!("{}, {}", card, epi);
            game.zugeben(*card, epi).unwrap();
        }
    }
    for (i_stich, stich) in game.vecstich.iter().enumerate() {
        assert_eq!(stich, &vecstich_test[i_stich]);
        println!("Stich {}: {}", i_stich, stich);
    }
    let accountbalance_payout = game.finish().unwrap();
    assert_eq!(EPlayerIndex::map_from_fn(|epi| accountbalance_payout.get_player(epi)), EPlayerIndex::map_from_raw(an_payout));
}

fn make_stich_vector(vecpairnstr_stich: &[(usize, &str)]) -> Vec<SStich> {
    vecpairnstr_stich.iter()
        .map(|&(n_epi, str_stich)| {
            let mut stich = SStich::new(EPlayerIndex::from_usize(n_epi));
            let veccard = parse_cards::<Vec<_>>(str_stich).unwrap();
            assert_eq!(4, veccard.len());
            for card in veccard {
                stich.push(card);
            }
            stich
        })
        .collect()
}

pub fn test_rules(
    str_info: &str,
    rules: &TRules,
    astr_hand: [&str; 4],
    vecn_doubling: Vec<usize>,
    vecn_stoss: Vec<usize>,
    vecpairnstr_stich: &[(usize, &str)],
    an_payout: [isize; 4],
) {
    internal_test_rules(
        str_info,
        rules,
        EPlayerIndex::map_from_fn(|epi| {
            SHand::new_from_vec(parse_cards(astr_hand[epi.to_usize()]).unwrap())
        }),
        vecn_doubling,
        vecn_stoss,
        &make_stich_vector(vecpairnstr_stich),
        an_payout,
    );
}

pub fn test_rules_manual(
    str_info: &str,
    rules: &TRules,
    vecn_doubling: Vec<usize>,
    vecn_stoss: Vec<usize>,
    vecpairnstr_stich: &[(usize, &str)],
    an_payout: [isize; 4],
) {
    let vecstich = make_stich_vector(vecpairnstr_stich);
    internal_test_rules(
        str_info,
        rules,
        EPlayerIndex::map_from_fn(|epi|
            SHand::new_from_vec(vecstich.iter().map(|stich| stich[epi]).collect())
        ),
        vecn_doubling,
        vecn_stoss,
        &vecstich,
        an_payout,
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

trait TPayoutDeciderDefaultPrioParams : TPayoutDecider {
    fn default_prioparams() -> Self::PrioParams;
}
impl TPayoutDeciderDefaultPrioParams for SPayoutDeciderPointBased {
    fn default_prioparams() -> Self::PrioParams {
        VGameAnnouncementPriority::SoloLikeSimple(0)
    }
}
impl TPayoutDeciderDefaultPrioParams for SPayoutDeciderTout {
    fn default_prioparams() -> Self::PrioParams {
        0
    }
}

fn rulessololike_new_test<TrumpfDecider, PayoutDecider>(epi: EPlayerIndex, n_payout_base: isize, n_payout_schneider_schwarz: isize, laufendeparams: SLaufendeParams) -> SRulesSoloLike<TrumpfDecider, PayoutDecider>
    where TrumpfDecider: TTrumpfDecider,
          PayoutDecider: TPayoutDeciderDefaultPrioParams,
{
    // Do not inline this function. It serves as a bridge between actual implementation and the data we extract for the test suite.
    SRulesSoloLike::<TrumpfDecider, PayoutDecider>::new(
        epi,
        PayoutDecider::default_prioparams(),
        "-", // should not matter within those tests
        SPayoutDeciderParams::new(
            n_payout_base,
            n_payout_schneider_schwarz,
            laufendeparams,
        ),
    )
}

#[test]
fn test_rulesrufspiel() {
    test_rules(
        "../../testdata/games/10.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        ["ho so gu su ek ga s9 s7","go hk h8 h7 ea sa sk s8","eu hu ha ez e7 gz g9 g8","eo hz h9 e9 e8 gk g7 sz",],
        vec![],
        vec![],
        &[(0, "su go hu h9"),(1, "h8 ha eo gu"),(3, "e8 ek ea e7"),(1, "sa eu sz s7"),(2, "g9 g7 ga sk"),(0, "ho h7 g8 hz"),(0, "so hk ez e9"),(0, "s9 s8 gz gk"),],
        [20, 20, -20, -20],
    );
    test_rules(
        "../../testdata/games/14.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        ["ho hu ha h8 ez e9 sa s9","eu h7 e8 gk g9 g7 sk s8","go so gu hz h9 e7 ga g8","eo su hk ea ek gz sz s7",],
        vec![],
        vec![],
        &[(0, "h8 h7 gu eo"),(3, "hk hu eu hz"),(1, "e8 e7 ea ez"),(3, "su ho sk go"),(2, "so s7 ha gk"),(2, "ga gz e9 g7"),(2, "g8 sz s9 g9"),(1, "s8 h9 ek sa"),],
        [-30, 30, 30, -30],
    );
    test_rules(
        "../../testdata/games/16.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        ["gu su hk e9 e8 e7 ga sz","so hu hz h8 ez g9 g8 s7","eo go ha h9 h7 sk s9 s8","ho eu ea ek gz gk g7 sa",],
        vec![],
        vec![],
        &[(0, "sz s7 s8 sa"),(3, "ho hk h8 ha"),(3, "eu su so go"),(2, "eo gz gu hz"),(2, "h7 ek e7 hu"),(1, "g8 h9 gk ga"),(2, "sk g7 e8 g9"),(2, "s9 ea e9 ez"),],
        [-60, -60, 60, 60],
    );
    test_rules(
        "../../testdata/games/19.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI1, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["go gu ga gz g9 sz s9 s8","ho hu hk h8 h7 gk sa s7","eo so eu ha ea e9 e8 sk","su hz h9 ez ek e7 g8 g7",],
        vec![],
        vec![],
        &[(0, "go h7 eo hz"),(2, "sk su s8 s7"),(3, "g7 ga gk ha"),(2, "ea e7 gu sa"),(0, "g9 hu e9 g8"),(1, "ho eu h9 gz"),(1, "h8 so ez s9"),(2, "e8 ek sz hk"),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/2.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["gu su hz h7 ek ga gk s7","hk ez e9 e8 e7 g7 s9 s8","go so hu ha ea gz sa sk","eo ho eu h9 h8 g9 g8 sz",],
        vec![],
        vec![3,0,],
        &[(0, "gu hk hu eu"),(3, "g9 ga g7 gz"),(0, "h7 s8 so h8"),(2, "ea h9 ek ez"),(3, "g8 gk s9 ha"),(2, "sa sz s7 e7"),(2, "sk ho su e8"),(3, "eo hz e9 go"),],
        [-80, 80, -80, 80],
    );
    test_rules(
        "../../testdata/games/21.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        ["hk h7 ez e9 e7 gk g8 sk","go gu ea ek e8 gz sa s7","ho so eu hz h8 g7 sz s8","eo hu su ha h9 ga g9 s9",],
        vec![],
        vec![],
        &[(0, "sk sa s8 s9"),(1, "go h8 ha hk"),(1, "gu eu eo h7"),(3, "su ez e8 so"),(2, "g7 ga g8 gz"),(3, "g9 gk s7 sz"),(0, "e7 ea ho h9"),(2, "hz hu e9 ek"),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/22.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["eo eu hu ha hk g7 sz s8","ho ez e9 e7 gz gk g9 sa","go so hz h9 ek e8 sk s9","gu su h8 h7 ea ga g8 s7",],
        vec![],
        vec![],
        &[(0, "hk ho h9 h7"),(1, "gz hz ga g7"),(2, "e8 ea sz e7"),(3, "gu hu ez so"),(2, "s9 s7 s8 sa"),(1, "gk go g8 eo"),(0, "eu g9 ek h8"),(0, "ha e9 sk su"),],
        [-20, 20, 20, -20],
    );
    test_rules(
        "../../testdata/games/26.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        ["eo ho hz h9 h8 ga gk s8","hu su hk ea e7 gz sa sz","go so eu gu e8 g9 s9 s7","ha h7 ez ek e9 g8 g7 sk",],
        vec![],
        vec![],
        &[(0, "h8 su eu ha"),(2, "s7 sk s8 sa"),(1, "ea e8 e9 ga"),(1, "e7 g9 ek hz"),(0, "gk gz gu g7"),(2, "s9 h7 h9 sz"),(0, "eo hk so g8"),(0, "ho hu go ez"),],
        [20, 20, -20, -20],
    );
    test_rules(
        "../../testdata/games/29.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        ["so su ha hz h7 ek e9 sz","go hk h8 e7 gz g9 sa s7","eo eu h9 ez ga gk sk s8","ho gu hu ea e8 g8 g7 s9",],
        vec![],
        vec![],
        &[(0, "h7 go h9 hu"),(1, "e7 ez ea ek"),(3, "ho su h8 eo"),(2, "eu gu so hk"),(0, "e9 gz s8 e8"),(0, "sz sa sk s9"),(1, "g9 ga g7 ha"),(0, "hz s7 gk g8"),],
        [20, -20, -20, 20],
    );
    test_rules(
        "../../testdata/games/30.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI1, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["ha ea ez e8 e7 ga gk sk","eu hu su hz h8 h7 g9 g7","eo ho gu hk e9 gz sa s9","go so h9 ek g8 sz s8 s7",],
        vec![],
        vec![],
        &[(0, "ha h7 ho h9"),(2, "sa s8 sk hz"),(1, "h8 hk so e7"),(3, "g8 ga g7 gz"),(0, "gk g9 s9 ek"),(0, "e8 hu e9 go"),(3, "sz ea su gu"),(2, "eo s7 ez eu"),],
        [-60, -60, 60, 60],
    );
    test_rules(
        "../../testdata/games/31.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        ["eo hz h9 e8 ga gk sk s9","gu hu ez ek e7 gz g9 s7","ho so su ha e9 g8 g7 sa","go eu hk h8 h7 ea sz s8",],
        vec![],
        vec![],
        &[(0, "sk s7 sa sz"),(2, "ho h7 eo hu"),(0, "ga gz g7 hk"),(3, "go h9 gu ha"),(3, "ea e8 e7 e9"),(3, "s8 s9 g9 g8"),(0, "gk ek su h8"),(2, "so eu hz ez"),],
        [-30, -30, 30, 30],
    );
    test_rules(
        "../../testdata/games/32.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["eo go ho hz h7 e9 gz sa","so gu hu hk h8 ea ez g7","su e8 gk g9 g8 sk s9 s7","eu ha h9 ek e7 ga sz s8",],
        vec![],
        vec![],
        &[(0, "eo h8 su ha"),(0, "go hk e8 h9"),(0, "h7 so s9 eu"),(1, "g7 g8 ga gz"),(3, "ek e9 ea gk"),(1, "gu g9 sz ho"),(0, "sa hu sk s8"),(1, "ez s7 e7 hz"),],
        [50, -50, -50, 50],
    );
    test_rules(
        "../../testdata/games/33.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["so eu su h7 ek g7 sk s8","eo h9 ea ez e9 e8 g8 s7","go ho gu ha hz g9 sa sz","hu hk h8 e7 ga gz gk s9",],
        vec![],
        vec![],
        &[(0, "g7 g8 g9 ga"),(3, "hu h7 eo gu"),(1, "ea ha e7 ek"),(2, "go hk su h9"),(2, "ho h8 eu s7"),(2, "sa s9 s8 e8"),(2, "sz gk sk e9"),(2, "hz gz so ez"),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/35.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["eo so eu h7 e7 gk g9 g8","gu hu ek e8 ga sz s9 s7","go ho su hz hk h8 gz s8","ha h9 ea ez e9 g7 sa sk",],
        vec![],
        vec![],
        &[(0, "gk ga gz g7"),(1, "gu h8 ha eu"),(0, "e7 ek hz e9"),(2, "ho h9 h7 hu"),(2, "su ea so e8"),(0, "g9 sz hk sk"),(2, "s8 sa g8 s7"),(3, "ez eo s9 go"),],
        [-20, 20, 20, -20],
    );
    test_rules(
        "../../testdata/games/36.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI1, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        ["so e9 ga g9 g8 sa sz s9","eo gu hz h9 h8 h7 ez s7","go eu hu su hk ek gk sk","ho ha ea e8 e7 gz g7 s8",],
        vec![],
        vec![],
        &[(0, "e9 ez ek ea"),(3, "ho so h7 go"),(2, "sk s8 sa s7"),(0, "ga hz gk gz"),(1, "eo su ha g9"),(1, "h8 hu g7 sz"),(2, "hk e7 s9 h9"),(2, "eu e8 g8 gu"),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/38.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["su ha ez e9 e7 gk g9 s8","go gu hu h9 g7 sz s9 s7","eo so eu ek ga gz sa sk","ho hz hk h8 h7 ea e8 g8",],
        vec![],
        vec![],
        &[(0, "gk g7 ga g8"),(2, "eo hz su h9"),(2, "so h7 ha go"),(1, "s9 sa e8 s8"),(2, "eu hk g9 hu"),(2, "ek ea e7 gu"),(1, "sz sk h8 e9"),(3, "ho ez s7 gz"),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/40.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        ["go so eu ha h9 e9 sa s9","eo ho hz ek e7 g9 g8 g7","gu hk h8 ez gz gk sz sk","hu su h7 ea e8 ga s8 s7",],
        vec![],
        vec![],
        &[(0, "h9 ho hk h7"),(1, "ek ez ea e9"),(3, "hu eu eo h8"),(1, "g7 gk ga s9"),(3, "su so hz gu"),(0, "go e7 gz e8"),(0, "ha g8 sk s7"),(0, "sa g9 sz s8"),],
        [30, -30, -30, 30],
    );
    test_rules(
        "../../testdata/games/41.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        ["eo gu hu ez gz g8 g7 sz","su hk h9 ea e7 g9 s9 s8","so eu ha h8 ek e9 sk s7","go ho hz h7 e8 ga gk sa",],
        vec![],
        vec![],
        &[(0, "ez ea e9 e8"),(1, "h9 eu h7 hu"),(2, "sk sa sz s8"),(3, "ho eo hk h8"),(0, "gz g9 so ga"),(2, "ek hz gu e7"),(0, "g7 su s7 gk"),(1, "s9 ha go g8"),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/43.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["hz h9 ea ek g9 sz sk s9","eo su ha h8 ga gz sa s7","so gu h7 e9 e7 gk g8 s8","go ho eu hu hk ez e8 g7",],
        vec![],
        vec![],
        &[(0, "g9 ga g8 g7"),(1, "eo h7 hk h9"),(1, "sa s8 ez s9"),(1, "h8 gu ho hz"),(3, "e8 ea ha e7"),(1, "su so go ek"),(3, "eu sk gz e9"),(3, "hu sz s7 gk"),],
        [-70, 70, -70, 70],
    );
    test_rules(
        "../../testdata/games/45.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        ["ho hu su h9 e9 e8 gk sk","ha hz h7 ea ez ga sa s9","so eu gu ek e7 gz g9 s7","eo go hk h8 g8 g7 sz s8",],
        vec![],
        vec![],
        &[(0, "sk sa s7 sz"),(1, "ha gu go h9"),(3, "g8 gk ga g9"),(1, "ea e7 g7 e8"),(1, "ez ek eo e9"),(3, "s8 hu s9 gz"),(0, "su hz so h8"),(2, "eu hk ho h7"),],
        [-20, 20, -20, 20],
    );
    test_rules(
        "../../testdata/games/46.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        ["ho so eu hu h8 ga g8 s8","go ha hz h7 ea ek s9 s7","gu su h9 e9 gz g9 g7 sz","eo hk ez e8 e7 gk sa sk",],
        vec![],
        vec![],
        &[(0, "h8 ha su eo"),(3, "hk eu hz h9"),(0, "so go gu e7"),(1, "ea e9 ez hu"),(0, "ho h7 g7 gk"),(0, "ga ek g9 sk"),(0, "s8 s7 sz sa"),(3, "e8 g8 s9 gz"),],
        [30, -30, -30, 30],
    );
    test_rules(
        "../../testdata/games/47.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["ho h9 e7 ga gz g9 sz s9","go so eu ea e9 g8 sk s8","gu su hk h8 h7 ez ek e8","eo hu ha hz gk g7 sa s7",],
        vec![],
        vec![],
        &[(0, "ho go h7 hu"),(1, "g8 hk g7 ga"),(2, "ek ha e7 e9"),(3, "eo h9 eu h8"),(3, "sa s9 s8 su"),(2, "ez hz sz ea"),(3, "s7 g9 sk e8"),(1, "so gu gk gz"),],
        [20, -20, -20, 20],
    );
    test_rules(
        "../../testdata/games/48.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        ["eu ha h8 ea e8 e7 ga g7","eo hk ez ek sz s9 s8 s7","go so gu hu h9 h7 e9 sa","ho su hz gz gk g9 g8 sk",],
        vec![],
        vec![],
        &[(0, "ha eo h7 hz"),(1, "ek e9 su ea"),(3, "sk h8 s7 sa"),(0, "eu hk h9 ho"),(3, "g8 ga s8 hu"),(2, "go g9 g7 s9"),(2, "so gk e7 sz"),(2, "gu gz e8 ez"),],
        [20, -20, 20, -20],
    );
    test_rules(
        "../../testdata/games/49.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI1, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["h9 h8 ez e8 g8 sk s9 s8","eo so gu ha hk e9 gk s7","eu hu su hz h7 gz g9 sz","go ho ea ek e7 ga g7 sa",],
        vec![],
        vec![],
        &[(0, "g8 gk g9 ga"),(3, "go h8 ha h7"),(3, "ho h9 hk su"),(3, "sa s9 s7 sz"),(3, "ea ez e9 hz"),(2, "eu ek sk so"),(1, "gu hu e7 s8"),(1, "eo gz g7 e8"),],
        [-60, 60, -60, 60],
    );
    test_rules(
        "../../testdata/games/5.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        ["go so eu h9 ez gz sa s9","hu su ha hz e9 e7 g9 s8","hk h8 ek e8 g7 sz sk s7","eo ho gu h7 ea ga gk g8",],
        vec![],
        vec![],
        &[(0, "h9 su hk gu"),(3, "eo so hu h8"),(3, "ho eu hz g7"),(3, "h7 go ha s7"),(0, "sa s8 sk gk"),(0, "ez e7 e8 ea"),(3, "ga gz g9 ek"),(3, "g8 s9 e9 sz"),],
        [100, -100, -100, 100],
    );
    test_rules(
        "../../testdata/games/50.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Schelln, 20, 10, SLaufendeParams::new(10, 3)),
        ["ho so gu gk g9 sa s9 s7","ha ea ez ek e9 gz g8 g7","hu hk h9 h7 e8 e7 ga sz","eo go eu su hz h8 sk s8",],
        vec![],
        vec![],
        &[(0, "ho ha h7 hz"),(0, "so e9 h9 h8"),(0, "gu g7 hk su"),(0, "sa ek sz s8"),(0, "s9 ez hu sk"),(2, "ga eu gk g8"),(3, "go g9 gz e7"),(3, "eo s7 ea e8"),],
        [90, -90, -90, 90],
    );
    test_rules(
        "../../testdata/games/51.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI3, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["eu su h9 ea gz g9 sz sk","eo ho gu h7 ez g8 g7 s8","go ek e9 e7 ga sa s9 s7","so hu ha hz hk h8 e8 gk",],
        vec![],
        vec![],
        &[(0, "g9 g7 ga gk"),(2, "go h8 h9 eo"),(1, "g8 e7 hk gz"),(3, "e8 ea ez e9"),(0, "sk s8 sa ha"),(3, "hu eu h7 s7"),(0, "sz ho s9 so"),(1, "gu ek hz su"),],
        [20, 20, -20, -20],
    );
    test_rules(
        "../../testdata/games/53.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI1, EFarbe::Gras, 20, 10, SLaufendeParams::new(10, 3)),
        ["so ez ek e7 ga sz sk s8","go hu ha hz hk h8 e9 gk","eo gu h9 h7 g8 sa s9 s7","ho eu su ea e8 gz g9 g7",],
        vec![],
        vec![],
        &[(0, "so h8 h7 ho"),(3, "gz ga gk g8"),(0, "sk ha s7 su"),(3, "g7 s8 hk s9"),(1, "hu gu eu e7"),(3, "ea ez e9 sa"),(3, "e8 ek hz eo"),(2, "h9 g9 sz go"),],
        [-20, -20, 20, 20],
    );
    test_rules(
        "../../testdata/games/55.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI2, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        ["su hk h7 ek ga gz g9 g7","go so eu ez e8 gk g8 sz","ho gu hu ha hz h8 e7 s7","eo h9 ea e9 sa sk s9 s8",],
        vec![],
        vec![],
        &[(0, "ek e8 e7 ea"),(3, "eo h7 eu ha"),(3, "h9 su go h8"),(1, "ez hu e9 g7"),(2, "ho sa hk so"),(2, "gu sk gz g8"),(2, "s7 s8 g9 sz"),(1, "gk hz s9 ga"),],
        [-30, -30, 30, 30],
    );
    test_rules(
        "../../testdata/games/6.html",
        &rulesrufspiel_new_test(EPlayerIndex::EPI0, EFarbe::Eichel, 20, 10, SLaufendeParams::new(10, 3)),
        ["eo go so ha hk ek gz g9","su h9 e9 e8 gk g7 s9 s8","ho eu hu h8 ez e7 ga sz","gu hz h7 ea g8 sa sk s7",],
        vec![],
        vec![],
        &[(0, "eo h9 h8 hz"),(0, "hk su eu h7"),(2, "e7 ea ek e9"),(3, "gu go gk hu"),(0, "g9 g7 ga g8"),(2, "ho s7 ha e8"),(2, "ez sk so s9"),(0, "gz s8 sz sa"),],
        [20, -20, -20, 20],
    );
}

#[test]
fn test_rulesfarbwenz() {
    test_rules(
        "../../testdata/games/11.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["ga g9 ez e8 hz h9 h8 sk","ek eo e9 e7 ha hk sz s8","su gk go g7 ea h7 so s9","eu gu hu gz g8 ho sa s7",],
        vec![],
        vec![],
        &[(0, "sk s8 s9 sa"),(3, "gu ga eo g7"),(3, "hu g9 e7 go"),(3, "eu h8 hk gk"),(3, "g8 hz sz su"),(2, "ea gz e8 e9"),(3, "s7 ez ha so"),(2, "h7 ho h9 ek"),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/12.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu ha hz h9 h7 ga gz g8","hu ek eo e9 go g9 g7 sk","su ho ea ez e8 e7 s9 s7","gu hk h8 gk sa sz so s8",],
        vec![],
        vec![],
        &[(0, "h7 hu ho hk"),(1, "sk s7 sz ha"),(0, "eu g7 su h8"),(0, "h9 go ea gu"),(3, "gk ga g9 s9"),(0, "hz e9 e7 s8"),(0, "gz eo e8 so"),(0, "g8 ek ez sa"),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/15.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hk ez e8 gk g7 sk so s9","gu ha hz ho h7 ek sa s8","eu h8 eo e9 e7 ga g9 s7","hu su h9 ea gz go g8 sz",],
        vec![],
        vec![],
        &[(0, "sk sa s7 sz"),(1, "h7 h8 su hk"),(3, "ea ez ek eo"),(3, "go g7 ha g9"),(1, "ho eu h9 gk"),(2, "e9 hu e8 s8"),(3, "g8 s9 hz ga"),(1, "gu e7 gz so"),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/17.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["su ga gz go g8 ea ek ha","eu gu gk g7 e8 hk so s7","hu g9 ho h8 h7 sa s9 s8","ez eo e9 e7 hz h9 sz sk",],
        vec![],
        vec![],
        &[(0, "g8 gk g9 ez"),(1, "hk h7 h9 ha"),(0, "su gu hu hz"),(1, "e8 ho e9 ea"),(0, "go g7 h8 e7"),(0, "ek s7 s8 eo"),(0, "gz eu sa sz"),(1, "so s9 sk ga"),],
        [-240, 80, 80, 80],
    );
    test_rules(
        "../../testdata/games/23.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu hu g9 ek e9 e8 e7 hk","go ez ha hz h9 h7 sz s8","eu ga gz gk g7 ea sa s9","su g8 eo ho h8 sk so s7",],
        vec![],
        vec![],
        &[(0, "e9 ez ea eo"),(2, "g7 g8 g9 go"),(1, "hz ga h8 hk"),(2, "eu su hu h7"),(2, "gk sk gu ha"),(0, "ek sz gz s7"),(2, "sa so e7 s8"),(2, "s9 ho e8 h9"),],
        [-60, -60, 180, -60],
    );
    test_rules(
        "../../testdata/games/25.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu hu ga gk g7 ha h7 sa","eu ea ez e7 ho h8 sz s8","gz go g9 g8 eo h9 s9 s7","su ek e9 e8 hz hk sk so",],
        vec![],
        vec![],
        &[(0, "hu eu gz su"),(1, "ea eo ek g7"),(0, "gu e7 g8 e8"),(0, "gk ez g9 e9"),(0, "ga h8 go so"),(0, "ha ho h9 hk"),(0, "sa s8 s7 sk"),(0, "h7 sz s9 hz"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/37.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu hu ga g9 hz hk so s9","ea gz g7 ho h9 sk s8 s7","su eo e9 gk go g8 ha h8","eu ez ek e8 e7 h7 sa sz",],
        vec![],
        vec![],
        &[(0, "so sk eo sa"),(2, "ha h7 hz ho"),(2, "h8 ek hk h9"),(3, "eu hu ea e9"),(3, "e7 gu gz su"),(0, "ga g7 g8 ez"),(3, "sz s9 s7 go"),(3, "e8 g9 s8 gk"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/4.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu ha hz hk h9 h7 ek ga","eu ho h8 gz g8 sa s8 s7","hu ea e9 go g9 sz sk so","su ez eo e8 e7 gk g7 s9",],
        vec![],
        vec![],
        &[(0, "h7 ho hu su"),(2, "go gk ga g8"),(0, "h9 h8 sz g7"),(0, "gu eu ea ez"),(1, "sa sk s9 ha"),(0, "hk s7 g9 e7"),(0, "hz s8 e9 e8"),(0, "ek gz so eo"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/54.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu ha hk ho h9 g9 sa s9","hu h7 e7 go g8 g7 so s8","gu h8 ez e9 e8 gz sk s7","su hz ea ek eo ga gk sz",],
        vec![],
        vec![],
        &[(0, "eu h7 h8 su"),(0, "h9 hu gu hz"),(2, "gz ga g9 g7"),(3, "ea ha e7 e8"),(0, "s9 so sk sz"),(3, "gk hk g8 s7"),(0, "ho s8 e9 eo"),(0, "sa go ez ek"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/9.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderTout>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["h8 ez e9 e8 ga gz gk g7","eu gu hu su ha hk h9 sa","ho ea ek eo sz sk s8 s7","hz h7 e7 go g9 g8 so s9",],
        vec![],
        vec![],
        &[(0, "ga ha ho g8"),(1, "gu s7 h7 h8"),(1, "eu s8 hz e8"),(1, "hu eo e7 g7"),(1, "su ek g9 e9"),(1, "hk sk go ez"),(1, "h9 sz s9 gk"),(1, "sa ea so gz"),],
        [-200, 600, -200, -200],
    );
    test_rules(
        "../../testdata/games/farbwenz/1.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderTout>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu gu hu su gk g9","gz ek e9 hz so s9","go ea eo hk h9 sz","ga ez ha ho sa sk",],
        vec![0,],
        vec![],
        &[(0, "eu gz go ga"),(0, "gu s9 h9 ho"),(0, "hu so hk sk"),(0, "su hz eo ez"),(0, "gk e9 sz sa"),(0, "g9 ek ea ha"),],
        [1080, -360, -360, -360],
    );
    test_rules(
        "../../testdata/games/farbwenz/10.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu ea eo go g9 h9","hu gz ha hz ho s9","eu ga gk hk sz so","su ez ek e9 sa sk",],
        vec![],
        vec![],
        &[(0, "h9 ha hk ez"),(3, "e9 ea hu eu"),(2, "sz sa gu s9"),(0, "g9 gz gk ek"),(3, "sk go ho so"),(3, "su eo hz ga"),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/farbwenz/2.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu ea eo go g9 h9","hu gz ha hz ho s9","eu ga gk hk sz so","su ez ek e9 sa sk",],
        vec![],
        vec![],
        &[(0, "h9 ha hk ez"),(3, "e9 ea hu eu"),(2, "sz sa gu s9"),(0, "g9 gz gk ek"),(3, "sk go ho so"),(3, "su eo hz ga"),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/farbwenz/5.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu hu ha ek gz s9","hz hk ho ga gk sa","ea eo e9 g9 sk so","gu su h9 ez go sz",],
        vec![1,],
        vec![],
        &[(0, "ek hz e9 ez"),(1, "ho ea su ha"),(3, "go gz ga g9"),(1, "hk sk h9 hu"),(0, "s9 sa so sz"),(1, "gk eo gu eu"),],
        [-200, 600, -200, -200],
    );
    test_rules(
        "../../testdata/games/farbwenz/7.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["su gz ea ha hz sz","hu g9 ek eo e9 so","ga ez hk ho sk s9","eu gu gk go h9 sa",],
        vec![3,],
        vec![],
        &[(0, "sz so s9 sa"),(3, "gu gz g9 ga"),(3, "eu su hu ho"),(3, "go hz ek sk"),(3, "gk ha e9 hk"),(3, "h9 ea eo ez"),],
        [-140, -140, -140, 420],
    );
    test_rules(
        "../../testdata/games/farbwenz/8.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["su gk go eo e8 h7 sz s8","eu gu hu e7 hz h8 s9 s7","gz g9 g8 g7 ea ek ha sa","ga ez e9 hk ho h9 sk so",],
        vec![3,1,],
        vec![1,],
        &[(0, "h7 h8 ha h9"),(2, "g9 ga gk hu"),(1, "e7 ea e9 e8"),(2, "g8 ez go gu"),(1, "eu g7 hk su"),(1, "s7 sa so s8"),(2, "gz sk eo s9"),(2, "ek ho sz hz"),],
        [-800, -800, 2400, -800],
    );
    test_rules(
        "../../testdata/games/farbwenz/9.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu su ea ek e9 e8 e7 s7","eu hu gk g9 g8 sa so s8","ga go hz hk ho h8 sz sk","ez eo gz g7 ha h9 h7 s9",],
        vec![],
        vec![],
        &[(0, "su hu hz ez"),(1, "sa sk s9 s7"),(1, "g8 ga g7 ea"),(0, "gu eu sz eo"),(1, "s8 ho h7 ek"),(0, "e9 gk hk h9"),(0, "e8 so h8 gz"),(0, "e7 g9 go ha"),],
        [150, -50, -50, -50],
    );
}

#[test]
fn test_ruleswenz() {
    test_rules(
        "../../testdata/games/13.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eu hu ga gz ha hz sa s9","gu go g7 hk h7 sk so s8","su ek eo gk g9 g8 ho sz","ea ez e9 e8 e7 h9 h8 s7",],
        vec![],
        vec![],
        &[(0, "eu gu su s7"),(0, "hu g7 g8 h8"),(0, "ga go g9 h9"),(0, "gz h7 gk e7"),(0, "ha hk ho e8"),(0, "hz s8 eo e9"),(0, "sa so sz ez"),(0, "s9 sk ek ea"),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/52.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["ek e9 e7 gk go g8 ha hk","hu ez eo e8 hz h9 sz so","gz g9 g7 ho h7 sk s9 s8","eu gu su ea ga h8 sa s7",],
        vec![],
        vec![],
        &[(0, "ha hz ho h8"),(0, "gk hu gz ga"),(1, "h9 h7 s7 hk"),(0, "go e8 g7 su"),(3, "gu g8 eo g9"),(3, "eu e7 so s8"),(3, "ea e9 ez s9"),(3, "sa ek sz sk"),],
        [-70, -70, -70, 210],
    );
    test_rules(
        "../../testdata/games/8.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["hu su ea ez e9 hz sa s8","gu ek g9 g8 sk so s9 s7","eu ga gk g7 ho h9 h7 sz","eo e8 e7 gz go ha hk h8",],
        vec![],
        vec![],
        &[(0, "hu gu eu gz"),(2, "ga go s8 g8"),(2, "gk eo hz g9"),(2, "g7 ha su ek"),(0, "sa s7 sz e7"),(0, "ea s9 h7 e8"),(0, "ez so h9 h8"),(0, "e9 sk ho hk"),],
        [210, -70, -70, -70],
    );
    test_rules(
        "../../testdata/games/wenz/1.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["gu su ea hz sa sz","eu hu ga gz gk s9","ha hk ho h9 sk so","ez ek eo e9 go g9",],
        vec![1,3,],
        vec![1,],
        &[(0, "gu eu ha ez"),(1, "hu hk ek su"),(1, "ga sk go sz"),(1, "gz so g9 sa"),(1, "gk ho eo hz"),(1, "s9 h9 e9 ea"),],
        [-1680, 560, 560, 560],
    );
    test_rules(
        "../../testdata/games/wenz/10.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["ez ek gk go ho h9 sz s7","gu eo g8 g7 hz h8 so s8","ea e9 e8 ga g9 ha h7 sa","eu hu su e7 gz hk sk s9",],
        vec![],
        vec![],
        &[(0, "ek eo ea e7"),(2, "sa s9 s7 s8"),(2, "ha hk h9 h8"),(2, "ga gz go g7"),(2, "h7 su ho hz"),(3, "eu sz gu g9"),(3, "hu ez so e8"),(3, "sk gk g8 e9"),],
        [-90, -90, 270, -90],
    );
    test_rules(
        "../../testdata/games/wenz/11.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eu su eo e9 hz s9","gu hu ea ga sa sz","ek gz g9 h9 sk so","ez gk go ha hk ho",],
        vec![0,],
        vec![0,1,],
        &[(0, "eu gu gz ez"),(0, "eo ea ek go"),(1, "hu g9 gk su"),(1, "ga h9 ho s9"),(1, "sa so hk e9"),(1, "sz sk ha hz"),],
        [-480, 1440, -480, -480],
    );
    test_rules(
        "../../testdata/games/wenz/12.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["ea eo hz ho h9 s9","ek e9 ga gk g9 hk","gz go ha sa sk so","eu gu hu su ez sz",],
        vec![3,],
        vec![],
        &[(0, "hz hk ha su"),(3, "hu eo ek so"),(3, "gu s9 e9 sk"),(3, "sz ea gk sa"),(2, "go eu h9 g9"),(3, "ez ho ga gz"),],
        [-180, -180, -180, 540],
    );
    test_rules(
        "../../testdata/games/wenz/13.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eo ga gz gk ha ho h9 sz","su e8 g7 hz sk so s9 s7","gu hu ea ez ek e9 go g9","eu e7 g8 hk h8 h7 sa s8",],
        vec![1,],
        vec![],
        &[(0, "eo e8 ek e7"),(2, "hu eu sz su"),(3, "g8 gk g7 g9"),(0, "gz s7 go sa"),(0, "ga so gu s8"),(2, "ea h7 h9 s9"),(2, "ez h8 ho sk"),(2, "e9 hk ha hz"),],
        [-100, -100, 300, -100],
    );
    test_rules(
        "../../testdata/games/wenz/14.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["gu hu su ek gz sz","e9 hz hk ho sk s9","ea ga go ha h9 sa","eu ez eo gk g9 so",],
        vec![0,3,],
        vec![0,],
        &[(0, "ek e9 ea eo"),(2, "sa so sz s9"),(2, "ha eu gz hz"),(3, "ez su sk h9"),(0, "gu hk go gk"),(0, "hu ho ga g9"),],
        [720, 720, -2160, 720],
    );
    test_rules(
        "../../testdata/games/wenz/2.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eo e7 ga ho h7 sk s9 s8","eu su gk g8 g7 ha hz sa","gu hu ez ek e8 hk h9 so","ea e9 gz go g9 h8 sz s7",],
        vec![1,],
        vec![],
        &[(0, "ga g7 ez gz"),(0, "eo g8 ek ea"),(3, "s7 sk sa so"),(1, "eu hu h8 e7"),(1, "hz h9 e9 h7"),(1, "su gu g9 ho"),(2, "hk go s8 ha"),(1, "gk e8 sz s9"),],
        [-100, 300, -100, -100],
    );
    test_rules(
        "../../testdata/games/wenz/3.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eo e9 gz g7 ho h7 sz s9","eu gu hu ek ga go hk sa","su ez gk ha h9 h8 sk s7","ea e8 e7 g9 g8 hz so s8",],
        vec![],
        vec![],
        &[(0, "eo ek ez ea"),(3, "g9 g7 go gk"),(2, "ha hz ho hk"),(2, "h9 so h7 hu"),(1, "gu su s8 e9"),(1, "eu h8 g8 gz"),(1, "ga s7 e7 s9"),(1, "sa sk e8 sz"),],
        [80, -240, 80, 80],
    );
    test_rules(
        "../../testdata/games/wenz/4.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eu su gz hz sa sz","gu ea ek e9 g9 ha","hu ez ga gk go s9","eo hk ho h9 sk so",],
        vec![3,],
        vec![],
        &[(0, "eu gu hu h9"),(0, "gz g9 ga ho"),(2, "gk sk hz ha"),(2, "go hk su e9"),(0, "sa ek s9 so"),(0, "sz ea ez eo"),],
        [300, -100, -100, -100],
    );
    test_rules(
        "../../testdata/games/wenz/5.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderTout>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["ek eo go sa sk s9","hu e9 ga hz h9 so","ea ez gz gk g9 sz","eu gu su ha hk ho",],
        vec![3,0,1,],
        vec![],
        &[(0, "ek e9 ea gu"),(3, "eu sa hu ez"),(3, "su go h9 sz"),(3, "ha eo hz g9"),(3, "hk sk so gk"),(3, "ho s9 ga gz"),],
        [-1120, -1120, -1120, 3360],
    );
    test_rules(
        "../../testdata/games/wenz/6.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["ek e8 e7 gz g9 hk sz so","eu hu gk go g8 ho h7 s8","gu su ez eo h9 h8 sk s7","ea e9 ga g7 ha hz sa s9",],
        vec![2,],
        vec![],
        &[(0, "hk h7 h8 ha"),(3, "sa so s8 s7"),(3, "ga g9 g8 su"),(2, "sk s9 sz ho"),(0, "gz go h9 g7"),(0, "ek gk eo ea"),(3, "hz e7 hu ez"),(1, "eu gu e9 e8"),],
        [180, 180, 180, -540],
    );
    test_rules(
        "../../testdata/games/wenz/7.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["ez ek gk go ho h9 sz s7","gu eo g8 g7 hz h8 so s8","ea e9 e8 ga g9 ha h7 sa","eu hu su e7 gz hk sk s9",],
        vec![],
        vec![],
        &[(0, "ek eo ea e7"),(2, "sa s9 s7 s8"),(2, "ha hk h9 h8"),(2, "ga gz go g7"),(2, "h7 su ho hz"),(3, "eu sz gu g9"),(3, "hu ez so e8"),(3, "sk gk g8 e9"),],
        [-90, -90, 270, -90],
    );
    test_rules(
        "../../testdata/games/wenz/8.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eu su gz g9 hz ho h7 sa","hu e7 ga gk ha hk so s7","ea ez ek g8 h9 h8 s9 s8","gu eo e9 e8 go g7 sz sk",],
        vec![],
        vec![],
        &[(0, "eu hu g8 gu"),(0, "h7 hk h8 go"),(1, "ha h9 eo ho"),(1, "ga s9 g7 g9"),(1, "so s8 sk sa"),(0, "hz e7 ek e9"),(0, "gz gk ez e8"),(0, "su s7 ea sz"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/wenz/9.html",
        &rulessololike_new_test::<SCoreGenericWenz<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["su ek gz hk so s9","eo gk go g9 ha sa","gu hu ea ez e9 ga","eu hz ho h9 sz sk",],
        vec![],
        vec![],
        &[(0, "so sa hu sk"),(2, "gu eu su ha"),(3, "sz s9 eo e9"),(3, "h9 hk g9 ez"),(0, "ek gk ea ho"),(2, "ga hz gz go"),],
        [-50, -50, 150, -50],
    );
}

#[test]
fn test_rulessolo() {
    test_rules(
        "../../testdata/games/28.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go ho gz g7 ea ez hk","so eu hu ga gk ha sa s9","gu su g9 g8 e9 h8 sk s7","ek e8 e7 hz h9 h7 sz s8",],
        vec![],
        vec![],
        &[(0, "eo gk g8 e7"),(0, "ho hu g9 e8"),(0, "go eu su h7"),(0, "ea ga e9 ek"),(1, "ha h8 h9 hk"),(1, "sa s7 s8 gz"),(0, "g7 so gu sz"),(1, "s9 sk hz ez"),],
        [-240, 80, 80, 80],
    );
    test_rules(
        "../../testdata/games/34.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo ho gu hu ek e9 ha h7","go so eu su e7 g7 hk sz","ez e8 ga g9 g8 h9 sk s8","ea gz gk hz h8 sa s9 s7",],
        vec![],
        vec![],
        &[(0, "eo e7 e8 ea"),(0, "hu eu ez gz"),(1, "hk h9 h8 ha"),(0, "gu so ga hz"),(1, "g7 g8 gk ek"),(0, "h7 su sk sa"),(1, "sz s8 s7 ho"),(0, "e9 go g9 s9"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/7.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["su ga hz h8 h7 sa s9 s7","go hu g8 ez ek e8 e7 s8","eo so gu gz gk g9 ha hk","ho eu g7 ea e9 h9 sz sk",],
        vec![],
        vec![],
        &[(0, "h8 hu hk h9"),(1, "ez gu e9 s7"),(2, "eo g7 su g8"),(2, "g9 eu ga go"),(1, "s8 gz sk s9"),(2, "so ho h7 ek"),(3, "ea hz e7 gk"),(2, "ha sz sa e8"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/1-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo so hu su hk h8 h7 g7","ho hz e7 gk g8 sa s9 s7","gu ha ea ek ga gz g9 sz","go eu h9 ez e9 e8 sk s8",],
        vec![],
        vec![],
        &[(0, "eo ho gu h9"),(0, "hu hz ha eu"),(3, "s8 hk s7 sz"),(0, "su sa ga go"),(3, "sk g7 s9 gz"),(3, "e8 h7 e7 ek"),(0, "so g8 g9 e9"),(0, "h8 gk ea ez"),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/10-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/100-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo eu ez ga g7 hz hk sa","so gu ea gk g9 h8 sz sk","ek gz ha h9 h7 s9 s8 s7","go ho hu su e9 e8 e7 g8",],
        vec![],
        vec![],
        &[(0, "sa sk s7 e8"),(3, "ho eo ea ek"),(0, "ga gk gz g8"),(0, "hk h8 h7 e7"),(3, "go ez gu s8"),(3, "su eu so ha"),(1, "sz s9 e9 g7"),(3, "hu hz g9 h9"),],
        [50, 50, 50, -150],
    );
    test_rules(
        "../../testdata/games/solo/104-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hu ea ek e9 e8 e7 sa s9","so ha ez gz gk g8 g7 sz","gu su hz h9 ga sk s8 s7","eo go ho eu hk h8 h7 g9",],
        vec![],
        vec![],
        &[(0, "sa sz s8 hk"),(3, "go hu ha h9"),(3, "ho s9 so su"),(3, "eu e7 g7 hz"),(3, "eo e8 g8 gu"),(3, "g9 e9 gz ga"),(2, "sk h8 ek gk"),(3, "h7 ea ez s7"),],
        [-90, -90, -90, 270],
    );
    test_rules(
        "../../testdata/games/solo/105-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["so ek g7 hz hk h7 sk s8","eu hu e7 ga g8 h8 sz s9","ho ea ez gz g9 h9 sa s7","eo go gu su e9 e8 gk ha",],
        vec![],
        vec![],
        &[(0, "hk h8 h9 ha"),(3, "go ek e7 ea"),(3, "eo so hu ez"),(3, "e8 hz eu ho"),(2, "sa su s8 s9"),(3, "gk g7 ga gz"),(1, "sz s7 e9 sk"),(3, "gu h7 g8 g9"),],
        [-50, -50, -50, 150],
    );
    // ../../testdata/games/solo/106-gras-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/109-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo ho so eu gu hk e7 g8","go h8 e8 gz gk g9 g7 sz","hu ha h9 ea ek ga s9 s8","su hz h7 ez e9 sa sk s7",],
        vec![],
        vec![],
        &[(0, "gu go ha hz"),(1, "e8 ea ez e7"),(2, "ek e9 g8 sz"),(2, "ga su eu g7"),(0, "so h8 h9 h7"),(0, "ho g9 hu s7"),(0, "eo gk s9 sk"),(0, "hk gz s8 sa"),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/11-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/111-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo hu ez e9 e8 ga gz sa","ho gu su hk h9 ea sz s9","go so eu ha hz h8 h7 gk","ek e7 g9 g8 g7 sk s8 s7",],
        vec![],
        vec![],
        &[(0, "sa sz so s7"),(2, "eu sk eo hk"),(0, "ga s9 gk g7"),(0, "gz ho go g8"),(2, "h8 ek hu h9"),(0, "e9 ea ha e7"),(2, "h7 g9 e8 su"),(1, "gu hz s8 ez"),],
        [-50, -50, 150, -50],
    );
    // ../../testdata/games/solo/112-gras-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/113-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hz hk ek e7 ga g9 sa sk","go h8 ez e8 gk g8 s9 s8","eo so eu hu ha h9 h7 s7","ho gu su ea e9 gz g7 sz",],
        vec![],
        vec![],
        &[(0, "ga gk ha g7"),(2, "eo su hk h8"),(2, "eu ho hz go"),(1, "g8 s7 gz g9"),(3, "sz sa s9 h7"),(2, "so gu e7 s8"),(2, "hu e9 sk e8"),(2, "h9 ea ek ez"),],
        [-60, -60, 180, -60],
    );
    test_rules(
        "../../testdata/games/solo/114-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go eu gu ez e9 e8 s7","hu su e7 g8 ha h8 h7 sz","ho so gk hz hk h9 s9 s8","ea ek ga gz g9 g7 sa sk",],
        vec![],
        vec![],
        &[(0, "go e7 so ek"),(0, "eo su ho ea"),(0, "gu hu gk g9"),(0, "s7 sz s8 sa"),(3, "ga e9 g8 s9"),(0, "eu h7 h9 g7"),(0, "ez h8 hk gz"),(0, "e8 ha hz sk"),],
        [180, -60, -60, -60],
    );
    // ../../testdata/games/solo/116-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/119-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["so eu ek e8 e7 gz gk sz","ho gu su hk h9 h8 sa s9","eo go hu ea ez e9 ga h7","g9 g8 g7 ha hz sk s8 s7",],
        vec![],
        vec![],
        &[(0, "gk gu ga g9"),(1, "hk h7 ha sz"),(3, "g8 gz ho go"),(2, "e9 hz ek su"),(1, "h9 hu sk eu"),(0, "e7 h8 ea s7"),(2, "eo s8 e8 s9"),(2, "ez g7 so sa"),],
        [60, 60, -180, 60],
    );
    // ../../testdata/games/solo/120-herz-solo.html has wrong format
    // ../../testdata/games/solo/122-gras-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/123-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["e9 gk g8 ha h7 sa sz s8","go ho eu ea ek e8 e7 hk","eo so hu su gz g9 h9 h8","gu ez ga g7 hz sk s9 s7",],
        vec![],
        vec![2,],
        &[(0, "sa ea su sk"),(2, "h8 hz h7 hk"),(3, "s9 sz ho eo"),(2, "h9 gu ha eu"),(1, "go hu ez e9"),(1, "e8 so ga gk"),(2, "gz g7 g8 ek"),(1, "e7 g9 s7 s8"),],
        [100, -300, 100, 100],
    );
    test_rules(
        "../../testdata/games/solo/124-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go so eu gu hk h7 sa","h8 ea ez e9 e8 ga g9 s9","ha e7 gz g8 g7 sz s8 s7","ho hu su hz h9 ek gk sk",],
        vec![],
        vec![],
        &[(0, "eo h8 ha h9"),(0, "go s9 e7 su"),(0, "gu ea gz ho"),(3, "ek hk e8 g7"),(0, "so ez g8 hz"),(0, "eu e9 s7 hu"),(0, "sa g9 s8 sk"),(0, "h7 ga sz gk"),],
        [180, -60, -60, -60],
    );
    // ../../testdata/games/solo/126-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/127-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go g9 g7 ea e9 ha h9","eu ga gz e8 e7 hz hk s7","ho so gu hu su h8 sz sk","gk g8 ez ek h7 sa s9 s8",],
        vec![],
        vec![],
        &[(0, "go ga su g8"),(0, "eo eu hu gk"),(0, "g7 gz gu ez"),(2, "h8 h7 ha hk"),(0, "ea e7 so ek"),(2, "ho sa g9 hz"),(2, "sz s9 h9 s7"),(2, "sk s8 e9 e8"),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/128-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/129-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go so hu gk g8 e7 s9","g7 ez e8 ha hz h9 sk s8","eu gu su ga gz ea ek h8","ho g9 e9 hk h7 sa sz s7",],
        vec![],
        vec![],
        &[(0, "go g7 su g9"),(0, "eo s8 gu ho"),(0, "so sk gz e9"),(0, "g8 ha ga sa"),(2, "ea hk e7 ez"),(2, "eu sz hu hz"),(2, "ek s7 s9 e8"),(2, "h8 h7 gk h9"),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/13-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/130-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go hu gk g9 g7 h8 sa","gu gz g8 e8 e7 hk sz s9","eu su ga ez e9 h9 h7 s8","ho so ea ek ha hz sk s7",],
        vec![],
        vec![],
        &[(0, "eo g8 su so"),(0, "go gu eu ho"),(0, "hu gz ga s7"),(0, "h8 hk h9 ha"),(3, "hz g9 e7 h7"),(0, "g7 e8 s8 sk"),(0, "gk s9 e9 ek"),(0, "sa sz ez ea"),],
        [180, -60, -60, -60],
    );
    test_rules(
        "../../testdata/games/solo/131-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hz ek e9 e8 ga gz g8 s8","eu h7 ea ez gk g7 sk s7","eo ho so gu hu ha hk sz","go su h9 h8 e7 g9 sa s9",],
        vec![],
        vec![],
        &[(0, "ek ez gu e7"),(2, "eo h8 hz h7"),(2, "so h9 s8 eu"),(2, "hu go ga ea"),(3, "sa e8 sk sz"),(3, "s9 e9 s7 ha"),(2, "ho su g8 g7"),(2, "hk g9 gz gk"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/132-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["su ea ez ek e9 gz sz s8","so eu hk g9 g7 sk s9 s7","go gu h8 e7 ga gk g8 sa","eo ho hu ha hz h9 h7 e8",],
        vec![],
        vec![],
        &[(0, "gz g7 ga ha"),(3, "hu su so h8"),(1, "sk sa hz s8"),(3, "eo e9 hk gu"),(3, "h7 sz eu go"),(2, "gk e8 ea g9"),(2, "g8 h9 ez s9"),(3, "ho ek s7 e7"),],
        [-50, -50, -50, 150],
    );
    // ../../testdata/games/solo/134-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/135-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["so su ea e7 ha hz h9 sk","ho eu gu gz gk h7 sa s9","ek g9 g8 g7 hk h8 s8 s7","eo go hu ez e9 e8 ga sz",],
        vec![],
        vec![],
        &[(0, "sk sa s7 sz"),(1, "s9 s8 e9 h9"),(3, "go e7 gu ek"),(3, "eo su eu g9"),(3, "e8 ea ho hk"),(1, "gk g7 ga so"),(0, "ha h7 h8 ez"),(3, "hu hz gz g8"),],
        [50, 50, 50, -150],
    );
    test_rules(
        "../../testdata/games/solo/137-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu gk g9 ea e8 e7 sz sk","eu hu ek e9 ha h8 s9 s8","eo go so gz g8 g7 h9 h7","ho su ga ez hz hk sa s7",],
        vec![],
        vec![],
        &[(0, "ea e9 gz ez"),(2, "go su gk hu"),(2, "eo ga g9 eu"),(2, "g7 ho gu ek"),(3, "sa sk s8 g8"),(2, "so s7 e8 s9"),(2, "h7 hz sz ha"),(1, "h8 h9 hk e7"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/139-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go gu g7 ea e7 hk h9 s9","eu hu gz e9 ha h8 h7 s7","eo ho so su ga gk g8 ek","g9 ez e8 hz sa sz sk s8",],
        vec![],
        vec![],
        &[(0, "s9 s7 ek sa"),(3, "sz hk eu so"),(2, "eo g9 g7 hu"),(2, "su hz gu gz"),(0, "ea e9 ga e8"),(2, "ho ez go ha"),(0, "h9 h7 gk s8"),(2, "g8 sk e7 h8"),],
        [50, 50, -150, 50],
    );
    test_rules(
        "../../testdata/games/solo/142-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu su g7 ea e9 sa sz s8","eo go ho so hu ga g9 e8","g8 ez e7 ha hz hk s9 s7","eu gz gk ek h9 h8 h7 sk",],
        vec![],
        vec![],
        &[(0, "ea e8 ez ek"),(0, "e9 hu e7 sk"),(1, "so g8 gz g7"),(1, "ho ha gk su"),(1, "go s7 eu gu"),(1, "eo s9 h9 s8"),(1, "ga hk h7 sz"),(1, "g9 hz h8 sa"),],
        [-100, 300, -100, -100],
    );
    test_rules(
        "../../testdata/games/solo/143-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo ho so gu hu gk g7 h9","eu gz ea hz hk sa sz sk","ga g8 e9 e7 ha h8 h7 s9","go su g9 ez ek e8 s8 s7",],
        vec![],
        vec![],
        &[(0, "eo eu g8 g9"),(0, "hu gz ga go"),(3, "ez gu ea e9"),(0, "so sa s9 su"),(0, "ho sk e7 s8"),(0, "h9 hz ha ek"),(2, "h8 e8 gk hk"),(0, "g7 sz h7 s7"),],
        [150, -50, -50, -50],
    );
    // ../../testdata/games/solo/144-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/146-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["ho so su ea ek e8 ha hz","eo go eu e9 gk g7 hk sz","g9 g8 h9 h8 h7 sa sk s8","gu hu ez e7 ga gz s9 s7",],
        vec![],
        vec![],
        &[(0, "so go sa ez"),(1, "hk h7 hu ha"),(3, "ga ea g7 g8"),(0, "su eu sk e7"),(1, "gk g9 gz ek"),(0, "e8 e9 s8 gu"),(3, "s9 ho sz h8"),(0, "hz eo h9 s7"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/149-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go ho so eu gu hk h8 g8","su h9 ez e8 e7 ga sz s8","eo hu ha h7 ek gk sk s9","hz ea e9 gz g9 g7 sa s7",],
        vec![],
        vec![],
        &[(0, "gu h9 ha hz"),(0, "eu su h7 g7"),(0, "so ez eo gz"),(2, "gk g9 g8 ga"),(1, "e8 ek ea hk"),(0, "ho e7 hu e9"),(0, "go s8 s9 s7"),(0, "h8 sz sk sa"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/15-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hu su h9 ga gk g9","eo gu hk ez ek sk","go ho so eu hz sa","ha ea e9 gz sz s9",],
        vec![],
        vec![],
        &[(0, "ga sk hz gz"),(2, "eu ha h9 eo"),(1, "ez so ea g9"),(2, "ho e9 su hk"),(2, "go s9 hu gu"),(2, "sa sz gk ek"),],
        [-60, -60, 180, -60],
    );
    test_rules(
        "../../testdata/games/solo/150-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["so eu g9 ez e7 hz hk sa","eo go gu hu gz g7 ea e8","su ga gk ek h9 h8 sk s9","ho g8 e9 ha h7 sz s8 s7",],
        vec![],
        vec![],
        &[(0, "sa hu s9 s7"),(1, "go ga g8 g9"),(1, "eo su ho eu"),(1, "g7 gk sz so"),(0, "hz gz h8 h7"),(1, "gu h9 s8 hk"),(1, "ea ek e9 e7"),(1, "e8 sk ha ez"),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/151-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hu gk g9 g8 e9 ha h9 h8","eo so gu ga gz g7 sz s8","go eu su ez ek h7 s9 s7","ho ea e8 e7 hz hk sa sk",],
        vec![],
        vec![],
        &[(0, "ha ga h7 hk"),(1, "gu eu ho gk"),(3, "sa e9 s8 s7"),(3, "sk g9 sz s9"),(0, "h9 so ek hz"),(1, "eo su e7 g8"),(1, "g7 go ea hu"),(2, "ez e8 h8 gz"),],
        [-50, 150, -50, -50],
    );
    // ../../testdata/games/solo/153-eichel-solo.html has wrong format
    // ../../testdata/games/solo/154-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/155-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go so eu gu h8 ea s9","hk h9 ez e9 gk g7 sa s7","su ha h7 ek g9 g8 sk s8","ho hu hz e8 e7 ga gz sz",],
        vec![],
        vec![],
        &[(0, "go hk ha hu"),(0, "eo h9 h7 hz"),(0, "gu ez su ho"),(3, "ga h8 g7 g9"),(0, "eu e9 ek e7"),(0, "s9 s7 sk sz"),(3, "e8 ea gk g8"),(0, "so sa s8 gz"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/156-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["e9 e8 ga hz hk h8 h7 s8","hu su gz gk g9 sz sk s7","eo ho eu ea ez ek e7 sa","go so gu g8 g7 ha h9 s9",],
        vec![],
        vec![],
        &[(0, "hk hu eu h9"),(2, "e7 gu e9 su"),(3, "ha h7 s7 ea"),(2, "eo so e8 g9"),(2, "ho go ga sz"),(3, "g8 s8 gz ez"),(2, "sa s9 h8 sk"),(2, "ek g7 hz gk"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/157-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go su e9 e8 g7 ha hz","ea e7 ga gk g9 h8 sk s8","ho so gu ek gz g8 hk sz","eu hu ez h9 h7 sa s9 s7",],
        vec![],
        vec![],
        &[(0, "go ea ek hu"),(0, "eo e7 gu eu"),(0, "e9 sk so ez"),(2, "hk h9 ha h8"),(0, "hz gk ho h7"),(2, "sz sa e8 s8"),(0, "g7 ga gz s9"),(1, "g9 g8 s7 su"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/159-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["so eu e7 gz gk g9 g7 sa","eo go ho hu su ha h7 ek","hk h9 ez e9 g8 sz s9 s7","gu hz h8 ea e8 ga sk s8",],
        vec![],
        vec![],
        &[(0, "gz su g8 ga"),(1, "go hk h8 eu"),(1, "ho h9 gu so"),(1, "hu sz hz e7"),(1, "eo s9 s8 g7"),(1, "ek ez ea sa"),(3, "e8 gk h7 e9"),(1, "ha s7 sk g9"),],
        [-80, 240, -80, -80],
    );
    test_rules(
        "../../testdata/games/solo/160-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu gu g8 ea hk h8 sa s9","ga e9 e8 e7 h9 sz sk s8","eo go ho hu su gk g9 ez","so gz g7 ek ha hz h7 s7",],
        vec![],
        vec![],
        &[(0, "sa sz su s7"),(2, "eo g7 g8 ga"),(2, "ho gz gu h9"),(2, "go so eu e7"),(2, "g9 ek s9 e8"),(2, "ez ha ea e9"),(0, "hk sk gk h7"),(2, "hu hz h8 s8"),],
        [-80, -80, 240, -80],
    );
    test_rules(
        "../../testdata/games/solo/161-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go eu h9 ea e8 e7 g9 s9","so gu ez e9 g8 sz sk s7","su ha hz hk h7 ga gz g7","eo ho hu h8 ek gk sa s8",],
        vec![],
        vec![],
        &[(0, "ea ez ha ek"),(2, "h7 h8 h9 gu"),(1, "e9 hk gk e8"),(2, "su hu eu so"),(1, "g8 gz ho g9"),(3, "s8 s9 s7 g7"),(0, "e7 sk hz eo"),(3, "sa go sz ga"),],
        [120, 120, -360, 120],
    );
    test_rules(
        "../../testdata/games/solo/162-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["e7 g9 g7 hz h8 sa sz s7","go ho hu su ea ez e8 ha","ek ga gz gk h9 sk s9 s8","eo so eu gu e9 g8 hk h7",],
        vec![],
        vec![3,],
        &[(0, "sa su s8 gu"),(3, "g8 g9 ea gk"),(1, "hu ek eu e7"),(3, "h7 h8 ha h9"),(1, "ho ga eo hz"),(3, "hk g7 ez s9"),(1, "go sk e9 s7"),(1, "e8 gz so sz"),],
        [100, -300, 100, 100],
    );
    // ../../testdata/games/solo/163-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/164-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go so hu ek e9 hz hk","ho ea ez e7 h8 sz sk s7","gu ga gz g9 g7 ha h9 sa","eu su e8 gk g8 h7 s9 s8",],
        vec![0,],
        vec![],
        &[(0, "go ez gu e8"),(0, "eo e7 g7 su"),(0, "hu ea ga eu"),(3, "h7 hk h8 ha"),(2, "gz g8 e9 s7"),(0, "so ho sa gk"),(1, "sz h9 s8 ek"),(0, "hz sk g9 s9"),],
        [-300, 100, 100, 100],
    );
    // ../../testdata/games/solo/165-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/166-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["ho so hu su ga g9 ea ha","go gz gk ez e7 sa s9 s8","eu gu g8 g7 e9 e8 hz h7","eo ek hk h9 h8 sz sk s7",],
        vec![],
        vec![],
        &[(0, "so gz g7 eo"),(3, "hk ha gk hz"),(1, "sa h7 sk ga"),(0, "su go g8 sz"),(1, "s9 e8 s7 g9"),(0, "ho s8 gu h8"),(0, "ea e7 e9 ek"),(0, "hu ez eu h9"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/168-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["su gz gk ez e9 e7 hz sk","so g8 ea e8 ha h7 sa s7","eo hu ek h9 h8 sz s9 s8","go ho eu gu ga g9 g7 hk",],
        vec![],
        vec![],
        &[(0, "sk sa s8 ga"),(3, "eu gz so hu"),(1, "ha h9 hk hz"),(1, "h7 h8 g9 gk"),(0, "e7 ea ek g7"),(3, "ho su g8 eo"),(2, "sz gu e9 s7"),(3, "go ez e8 s9"),],
        [-50, -50, -50, 150],
    );
    // ../../testdata/games/solo/169-gras-solo.html has wrong format
    // ../../testdata/games/solo/170-eichel-solo.html has wrong format
    // ../../testdata/games/solo/171-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/172-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["so hu ea ez e7 h7 sz s9","eu gu ga gz g8 hk h8 s7","g7 ek e9 e8 h9 sa sk s8","eo go ho su gk g9 ha hz",],
        vec![],
        vec![],
        &[(0, "h7 h8 h9 ha"),(3, "eo hu g8 g7"),(3, "ho so gz e8"),(3, "go e7 gu e9"),(3, "su ea eu ek"),(1, "hk sk hz s9"),(3, "g9 sz ga sa"),(1, "s7 s8 gk ez"),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/solo/173-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go ho eu ea ez e9 g9 ha","so hu su e8 hz h8 s8 s7","ga gz gk g8 g7 hk sz s9","eo gu ek e7 h9 h7 sa sk",],
        vec![],
        vec![],
        &[(0, "eu so ga ek"),(1, "s7 s9 sa ea"),(0, "ho e8 hk e7"),(0, "go su g7 eo"),(3, "h9 ha h8 g8"),(0, "e9 hu sz gu"),(3, "sk ez s8 gk"),(0, "g9 hz gz h7"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/174-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hu su ek ga h9 h8 h7 sz","eo go gu ea e8 e7 g8 sa","ho eu e9 gk g7 ha sk s9","so ez gz g9 hz hk s8 s7",],
        vec![],
        vec![],
        &[(0, "ga g8 gk gz"),(0, "h7 ea ha hk"),(1, "go e9 ez su"),(1, "eo eu so hu"),(1, "e8 ho hz ek"),(2, "s9 s8 sz sa"),(1, "gu sk g9 h8"),(1, "e7 g7 s7 h9"),],
        [-50, 150, -50, -50],
    );
    // ../../testdata/games/solo/175-herz-solo.html has wrong format
    // ../../testdata/games/solo/176-gras-solo.html has wrong format
    // ../../testdata/games/solo/178-gras-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/179-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hu su hz ea e8 ga gz s7","gu ha h8 ez ek g9 sz s8","h9 h7 e9 e7 gk g8 sk s9","eo go ho so eu hk g7 sa",],
        vec![],
        vec![],
        &[(0, "ea ek e7 hk"),(3, "eo su h8 h7"),(3, "go hu gu h9"),(3, "ho hz ha e9"),(3, "sa s7 s8 s9"),(3, "g7 ga g9 gk"),(0, "gz sz g8 so"),(3, "eu e8 ez sk"),],
        [-110, -110, -110, 330],
    );
    test_rules(
        "../../testdata/games/solo/18-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go ho so gu ek g7 h8","hu su ea ez e9 g8 h7 sk","eu e7 ga gk hz hk sa s7","e8 gz g9 ha h9 sz s9 s8",],
        vec![],
        vec![],
        &[(0, "eo hu e7 e8"),(0, "go e9 eu s8"),(0, "so ez sa s9"),(0, "gu su s7 h9"),(0, "ho ea hk sz"),(0, "g7 g8 gk gz"),(3, "ha h8 h7 hz"),(3, "g9 ek sk ga"),],
        [270, -90, -90, -90],
    );
    test_rules(
        "../../testdata/games/solo/180-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu ea e8 h8 sz s9 s8 s7","ho so gu su gk g9 g8 ha","eo hu ez e9 e7 hk sa sk","go ga gz g7 ek hz h9 h7",],
        vec![],
        vec![],
        &[(0, "sz gk sk ga"),(3, "h9 h8 ha hk"),(1, "gu hu gz eu"),(0, "s7 su sa go"),(3, "ek e8 so e7"),(1, "g9 eo g7 ea"),(2, "e9 h7 s8 g8"),(1, "ho ez hz s9"),],
        [50, -150, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/181-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go hu e8 gz hz h9 sz s7","ho gk g8 g7 ha h8 sa s9","so eu gu ea ez ek ga s8","eo su e9 e7 g9 hk h7 sk",],
        vec![],
        vec![],
        &[(0, "gz gk ga g9"),(2, "gu e7 e8 ho"),(1, "g8 s8 sk sz"),(1, "g7 ek su hz"),(3, "h7 h9 h8 ea"),(2, "eu e9 hu s9"),(2, "so eo go sa"),(3, "hk s7 ha ez"),],
        [-80, -80, 240, -80],
    );
    test_rules(
        "../../testdata/games/solo/182-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["ho so eu hu su e7 ga ha","gu e9 g8 g7 hz sz sk s8","ez e8 gz gk hk h8 h7 s7","eo go ea ek g9 h9 sa s9",],
        vec![],
        vec![],
        &[(0, "eu e9 ez go"),(3, "sa su s8 s7"),(0, "so gu e8 ek"),(0, "hu hz gz eo"),(3, "g9 ga g7 gk"),(0, "ho g8 h7 ea"),(0, "ha sk h8 h9"),(0, "e7 sz hk s9"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/183-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["ho so su ea ez e8 e7 s8","ek g9 ha h8 h7 sa sz sk","eo go eu gu hu g7 h9 s7","e9 ga gz gk g8 hz hk s9",],
        vec![],
        vec![2,],
        &[(0, "e7 ek hu e9"),(2, "g7 ga ea g9"),(0, "e8 sa gu gz"),(2, "eo hz su sz"),(2, "go hk so ha"),(2, "s7 s9 s8 sk"),(1, "h7 h9 g8 ez"),(0, "ho h8 eu gk"),],
        [-300, 100, 100, 100],
    );
    test_rules(
        "../../testdata/games/solo/184-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go ho so eu ga g8 g7 sk","g9 ha hz h9 sz s9 s8 s7","eo hu su gz gk h8 h7 sa","gu ea ez ek e9 e8 e7 hk",],
        vec![],
        vec![2,],
        &[(0, "eu g9 gk gu"),(0, "so s7 su e7"),(0, "ho ha eo ea"),(2, "sa hk sk sz"),(2, "h7 ek g8 h9"),(0, "go s8 gz e8"),(0, "g7 hz hu ez"),(2, "h8 e9 ga s9"),],
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
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["ho hu ga ez e7 hk h9 s8","eo go so eu gz gk g8 h7","gu su ek e8 ha hz sa sk","g9 g7 ea e9 h8 sz s9 s7",],
        vec![],
        vec![],
        &[(0, "hk h7 ha h8"),(2, "sa s7 s8 gz"),(1, "go su g7 hu"),(1, "eo gu g9 ga"),(1, "eu hz sz ho"),(0, "h9 gk e8 s9"),(1, "so sk e9 e7"),(1, "g8 ek ea ez"),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/203-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go so gu ez e9 g7 ha sz","ek ga g9 g8 h9 h8 sa s8","e7 gz gk hz hk h7 sk s9","eo ho eu hu su ea e8 s7",],
        vec![],
        vec![],
        &[(0, "ha h9 hk s7"),(0, "g7 ga gk ea"),(3, "ho go ek e7"),(0, "sz sa s9 e8"),(3, "eo e9 s8 sk"),(3, "su gu h8 gz"),(0, "so g8 hz hu"),(0, "ez g9 h7 eu"),],
        [-50, -50, -50, 150],
    );
    // ../../testdata/games/solo/204-gras-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/205-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go ek e8 gk g8 g7 hz h8","eu hu ea g9 h9 h7 sa s9","su gz ha hk sz sk s8 s7","eo ho so gu ez e9 e7 ga",],
        vec![],
        vec![],
        &[(0, "g7 g9 gz ga"),(3, "so ek ea su"),(3, "eo e8 hu s7"),(3, "gu go eu sz"),(0, "h8 h7 ha ez"),(3, "ho g8 h9 s8"),(3, "e9 gk s9 sk"),(3, "e7 hz sa hk"),],
        [-60, -60, -60, 180],
    );
    // ../../testdata/games/solo/206-eichel-solo.html has wrong format
    // ../../testdata/games/solo/207-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/209-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["ez g9 g7 ha h9 h8 sa s8","so eu gu su ek e9 e8 e7","eo ga gz gk g8 hk sz s7","go ho hu ea hz h7 sk s9",],
        vec![],
        vec![],
        &[(0, "ha su hk h7"),(1, "gu eo ea ez"),(2, "ga hz g7 ek"),(1, "eu gz ho g9"),(3, "s9 sa e9 s7"),(1, "e8 sz hu s8"),(3, "go h8 e7 gk"),(3, "sk h9 so g8"),],
        [80, -240, 80, 80],
    );
    // ../../testdata/games/solo/21-gras-solo.html has wrong format
    // ../../testdata/games/solo/210-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/211-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo so eu hu ek e9 e7 sa","go ho su ez h9 h8 h7 sz","gu ea gz gk g9 hk s9 s8","e8 ga g8 g7 ha hz sk s7",],
        vec![],
        vec![],
        &[(0, "eo su gu e8"),(0, "hu go ea ha"),(1, "h9 hk hz ek"),(0, "eu ho gz ga"),(1, "sz s8 s7 sa"),(0, "so ez s9 g7"),(0, "e9 h8 g9 g8"),(0, "e7 h7 gk sk"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/213-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go ho su ha hk sz s9 s7","so eu g7 ez e7 sa sk s8","eo gu ga gz gk g9 g8 h9","hu ea ek e9 e8 hz h8 h7",],
        vec![],
        vec![],
        &[(0, "ha so h9 hz"),(1, "sa gk hu sz"),(3, "h7 hk eu g8"),(1, "sk gu h8 s7"),(2, "eo e8 su g7"),(2, "g9 ea ho ez"),(0, "go e7 gz ek"),(0, "s9 s8 ga e9"),],
        [60, 60, -180, 60],
    );
    test_rules(
        "../../testdata/games/solo/215-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go ho so eu hk h9 h7 gk","eo hu su hz e7 g8 g7 s9","h8 ek e8 ga gz sz s8 s7","gu ha ea ez e9 g9 sa sk",],
        vec![],
        vec![],
        &[(0, "eu hz h8 gu"),(0, "so eo ga ha"),(1, "e7 ek ea hk"),(0, "ho su e8 g9"),(0, "go hu s7 e9"),(0, "gk g7 gz ez"),(2, "s8 sa h7 s9"),(0, "h9 g8 sz sk"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/216-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu hu ha ez e8 e7 sz s8","go ek gz gk sa sk s9 s7","ho hz h9 h7 ga g9 g8 g7","eo so gu su hk h8 ea e9",],
        vec![],
        vec![],
        &[(0, "e7 ek hz e9"),(2, "ga gu eu gz"),(0, "ez go g7 ea"),(1, "gk g8 su hu"),(0, "s8 s7 g9 hk"),(3, "eo ha s9 h7"),(3, "h8 sz sa h9"),(2, "ho so e8 sk"),],
        [60, 60, 60, -180],
    );
    test_rules(
        "../../testdata/games/solo/217-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["so hu e7 ga g7 h9 sz s8","e8 gk g9 g8 hk h8 h7 s7","eo go ho eu su ea ez ha","gu ek e9 gz hz sa sk s9",],
        vec![],
        vec![],
        &[(0, "h9 h8 ha hz"),(2, "eo e9 e7 e8"),(2, "ho ek hu h7"),(2, "go gu so s7"),(2, "eu s9 g7 g8"),(2, "su sk s8 g9"),(2, "ea gz sz hk"),(2, "ez sa ga gk"),],
        [-100, -100, 300, -100],
    );
    test_rules(
        "../../testdata/games/solo/219-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["su ga hz h8 h7 sa s9 s7","go hu g8 ez ek e8 e7 s8","eo so gu gz gk g9 ha hk","ho eu g7 ea e9 h9 sz sk",],
        vec![],
        vec![],
        &[(0, "h8 hu hk h9"),(1, "ez gu e9 s7"),(2, "eo g7 su g8"),(2, "g9 eu ga go"),(1, "s8 gz sk s9"),(2, "so ho h7 ek"),(3, "ea hz e7 gk"),(2, "ha sz sa e8"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/22-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu e8 gz g8 g7 ha sz s8","hu g9 hz h9 h8 sa s9 s7","eo su ea ek e7 gk hk h7","go ho so eu ez e9 ga sk",],
        vec![],
        vec![2,],
        &[(0, "ha h9 hk ez"),(3, "eu e8 hu e7"),(3, "so gu hz eo"),(2, "gk ga g8 g9"),(3, "ho g7 h8 ek"),(3, "go s8 s7 su"),(3, "sk sz sa h7"),(1, "s9 ea e9 gz"),],
        [100, 100, 100, -300],
    );
    test_rules(
        "../../testdata/games/solo/220-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go ho gz g7 ea ez hk","so eu hu ga gk ha sa s9","gu su g9 g8 e9 h8 sk s7","ek e8 e7 hz h9 h7 sz s8",],
        vec![],
        vec![],
        &[(0, "eo gk g8 e7"),(0, "ho hu g9 e8"),(0, "go eu su h7"),(0, "ea ga e9 ek"),(1, "ha h8 h9 hk"),(1, "sa s7 s8 gz"),(0, "g7 so gu sz"),(1, "s9 sk hz ez"),],
        [-240, 80, 80, 80],
    );
    test_rules(
        "../../testdata/games/solo/221-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo ho gu hu ek e9 ha h7","go so eu su e7 g7 hk sz","ez e8 ga g9 g8 h9 sk s8","ea gz gk hz h8 sa s9 s7",],
        vec![],
        vec![],
        &[(0, "eo e7 e8 ea"),(0, "hu eu ez gz"),(1, "hk h9 h8 ha"),(0, "gu so ga hz"),(1, "g7 g8 gk ek"),(0, "h7 su sk sa"),(1, "sz s8 s7 ho"),(0, "e9 go g9 s9"),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/23-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/25-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go ho so hu ga g8 ha s9","gz g9 g7 ea ez h9 h8 s7","eo eu gu e8 e7 hz hk s8","su gk ek e9 h7 sa sz sk",],
        vec![],
        vec![],
        &[(0, "so gz eo gk"),(2, "s8 sz s9 s7"),(3, "sa hu g7 gu"),(2, "e7 e9 g8 ez"),(0, "ho g9 eu su"),(0, "go h8 e8 h7"),(0, "ga ea hk sk"),(0, "ha h9 hz ek"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/26-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo ho gu ga g9 ea e8 ha","go su g8 ez e7 hk sa s9","so hu gz g7 e9 hz h7 sz","eu gk ek h9 h8 sk s8 s7",],
        vec![],
        vec![],
        &[(0, "gu su gz eu"),(3, "s7 ga s9 sz"),(0, "eo g8 g7 gk"),(0, "g9 go hu ek"),(1, "hk h7 h9 ha"),(0, "ho e7 so h8"),(0, "ea ez e9 s8"),(0, "e8 sa hz sk"),],
        [180, -60, -60, -60],
    );
    // ../../testdata/games/solo/27-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/29-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hu hk h9 e9 gk g7 sk s9","ez ek e7 gz g9 g8 sa sz","ho su ha h7 ea e8 s8 s7","eo go so eu gu hz h8 ga",],
        vec![],
        vec![],
        &[(0, "e9 ek ea hz"),(3, "eo h9 e7 h7"),(3, "go hk g8 su"),(3, "gu hu ez ho"),(2, "s7 h8 s9 sz"),(3, "eu sk sa ha"),(3, "so g7 g9 s8"),(3, "ga gk gz e8"),],
        [-60, -60, -60, 180],
    );
    test_rules(
        "../../testdata/games/solo/30-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["so ga g9 e7 hk h9 h8 s9","eu gu ek hz h7 sz s8 s7","ho su gk g7 ez e9 e8 sk","eo go hu gz g8 ea ha sa",],
        vec![],
        vec![],
        &[(0, "h9 h7 gk ha"),(2, "ez ea e7 ek"),(3, "go g9 gu g7"),(3, "eo so eu su"),(3, "g8 ga hz ho"),(2, "e9 gz s9 s7"),(3, "hu h8 s8 e8"),(3, "sa hk sz sk"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/31-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go gu ek e9 g7 sz sk s9","ho eu h7 ez ga gk g8 sa","su hz h9 e8 gz g9 s8 s7","eo so hu ha hk h8 ea e7",],
        vec![],
        vec![],
        &[(0, "sk sa s7 ha"),(3, "eo gu h7 h9"),(3, "h8 go eu hz"),(0, "g7 ga g9 hk"),(3, "hu sz ho su"),(1, "ez e8 ea e9"),(3, "so s9 g8 s8"),(3, "e7 ek gk gz"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/32-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go gu ea e9 e7 g9 sa","su ez ek e8 ga g7 h9 s8","ho eu hu gk g8 ha h8 sz","so gz hz hk h7 sk s9 s7",],
        vec![],
        vec![],
        &[(0, "go e8 hu so"),(0, "eo ek eu s7"),(0, "gu ez ho gz"),(2, "ha hk e9 h9"),(0, "e7 su sz hz"),(1, "ga gk sk g9"),(1, "s8 g8 s9 sa"),(0, "ea g7 h8 h7"),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/34-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/36-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go h8 ea ek e8 gk sk s8","eo eu su ha hz hk h9 sa","so h7 ez e7 ga gz g9 sz","ho gu hu e9 g8 g7 s9 s7",],
        vec![],
        vec![],
        &[(0, "gk hz g9 g8"),(1, "h9 h7 hu h8"),(3, "g7 go eo gz"),(1, "su so ho ea"),(3, "e9 ek hk e7"),(1, "eu ez gu e8"),(1, "ha ga s7 s8"),(1, "sa sz s9 sk"),],
        [-60, 180, -60, -60],
    );
    test_rules(
        "../../testdata/games/solo/37-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hu su ga e9 e7 h8 h7 s9","ho so g9 g7 ez ha hk s8","gu ek e8 hz h9 sa sz sk","eo go eu gz gk g8 ea s7",],
        vec![],
        vec![],
        &[(0, "s9 s8 sa s7"),(2, "sz eu h7 so"),(1, "ha h9 gz h8"),(3, "go su g7 gu"),(3, "eo hu g9 e8"),(3, "g8 ga ho hz"),(1, "hk ek gk e7"),(3, "ea e9 ez sk"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/38-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go so gu ga gz g9 g8 ha","ho hu ez e7 hk sa sk s9","eo eu ea e9 e8 h9 h8 s7","su gk g7 ek hz h7 sz s8",],
        vec![],
        vec![],
        &[(0, "gu ho eu gk"),(1, "sa s7 s8 ga"),(0, "g8 hu eo g7"),(2, "h9 h7 ha hk"),(0, "go ez h8 su"),(0, "so s9 e8 ek"),(0, "gz e7 e9 hz"),(0, "g9 sk ea sz"),],
        [180, -60, -60, -60],
    );
    // ../../testdata/games/solo/39-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/4-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go ho so g9 g7 ea s7","hu su ga ez e9 e7 hz h7","eu gk g8 ek ha h8 sa s9","gu gz e8 hk h9 sz sk s8",],
        vec![],
        vec![],
        &[(0, "eo su g8 gu"),(0, "go hu gk gz"),(0, "so ga eu e8"),(0, "s7 ez sa sz"),(2, "ek hk ea e7"),(0, "ho e9 s9 h9"),(0, "g7 hz h8 s8"),(0, "g9 h7 ha sk"),],
        [270, -90, -90, -90],
    );
    test_rules(
        "../../testdata/games/solo/40-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu hu su h8 ea e7 ga g8","so h9 ez e8 g9 g7 sz s7","ha hz ek gz gk sa sk s9","eo go ho eu hk h7 e9 s8",],
        vec![],
        vec![],
        &[(0, "ea e8 ek e9"),(0, "e7 ez sk hk"),(3, "eo h8 h9 hz"),(3, "ho su so ha"),(3, "eu hu g9 s9"),(3, "go gu g7 sa"),(3, "s8 ga sz gz"),(1, "s7 gk h7 g8"),],
        [-80, -80, -80, 240],
    );
    test_rules(
        "../../testdata/games/solo/41-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["so hu su ez e8 ga ha sa","go ek g9 hk h8 sz s9 s7","eo ho e9 gk hz h9 h7 sk","eu gu ea e7 gz g8 g7 s8",],
        vec![],
        vec![],
        &[(0, "su ek e9 gu"),(3, "g8 ga g9 gk"),(0, "hu go ho ea"),(1, "s7 sk s8 sa"),(0, "e8 sz eo e7"),(2, "h7 eu ha hk"),(3, "g7 ez s9 h9"),(0, "so h8 hz gz"),],
        [240, -80, -80, -80],
    );
    test_rules(
        "../../testdata/games/solo/44-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu gz ea ek e7 sa s9 s8","eo go ho so gu gk e8 hk","hu su g9 g7 ha hz h9 sk","ga g8 ez e9 h8 h7 sz s7",],
        vec![],
        vec![],
        &[(0, "sa gk sk s7"),(1, "go g7 ga eu"),(1, "eo g9 g8 gz"),(1, "gu su sz s8"),(1, "e8 ha ez ea"),(0, "ek hk hz e9"),(0, "e7 ho h9 h8"),(1, "so hu h7 s9"),],
        [-90, 270, -90, -90],
    );
    // ../../testdata/games/solo/46-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/49-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu hu ha h7 e8 e7 ga g9","ez gz g7 sa sz sk s9 s8","su hz hk h9 h8 ea ek e9","eo go ho so gu gk g8 s7",],
        vec![],
        vec![3,],
        &[(0, "e7 ez ea gu"),(3, "eo ha sa h8"),(3, "go h7 sz h9"),(3, "ho hu gz su"),(3, "so eu sk hk"),(3, "gk ga g7 hz"),(2, "ek s7 e8 s9"),(2, "e9 g8 g9 s8"),],
        [260, 260, -780, 260],
    );
    // ../../testdata/games/solo/5-gras-solo.html has wrong format
    // ../../testdata/games/solo/50-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/51-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo ho su ez e9 ga ha hk","so hu ek e8 g9 g8 hz s9","gu e7 h9 h7 sa sk s8 s7","go eu ea gz gk g7 h8 sz",],
        vec![],
        vec![],
        &[(0, "ho e8 e7 go"),(3, "gk ga g8 gu"),(2, "sa sz ez s9"),(0, "eo hu h7 eu"),(0, "e9 ek sk ea"),(3, "h8 ha hz h9"),(0, "su so s8 gz"),(1, "g9 s7 g7 hk"),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/52-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/53-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo ho eu ea e9 e8 g9 sa","gu hu su e7 ga gz sk s8","ez ek gk ha h8 h7 sz s9","go so g8 g7 hz hk h9 s7",],
        vec![],
        vec![],
        &[(0, "ho e7 ez go"),(3, "g8 g9 ga gk"),(1, "s8 s9 s7 sa"),(0, "eo su ek so"),(0, "eu hu h7 g7"),(0, "e9 gu sz hz"),(1, "sk h8 h9 e8"),(0, "ea gz ha hk"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/54-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu g9 ez hz h9 s9 s8 s7","go so gu su ea e7 hk h8","ho g7 ek e8 h7 sa sz sk","eo hu ga gz gk g8 e9 ha",],
        vec![],
        vec![],
        &[(0, "ez e7 e8 e9"),(0, "s7 su sk hu"),(3, "g8 g9 gu g7"),(1, "ea ek gk eu"),(0, "s8 h8 sz ga"),(3, "eo s9 so ho"),(3, "ha hz hk h7"),(3, "gz h9 go sa"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/55-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go so gu gk g9 g8 e9","ga g7 ez ek e7 ha h7 sk","eu hu su gz h9 h8 sa s7","ho ea e8 hz hk sz s9 s8",],
        vec![],
        vec![],
        &[(0, "go ga su ho"),(0, "so g7 hu s8"),(0, "eo e7 gz e8"),(0, "gu sk eu sz"),(2, "sa s9 gk h7"),(0, "e9 ez s7 ea"),(3, "hk g9 ha h8"),(0, "g8 ek h9 hz"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/57-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu ez gz g9 g7 hz h9 h7","eo ho hu su ea e9 e7 s7","go ek ga h8 sz sk s9 s8","so gu e8 gk g8 ha hk sa",],
        vec![],
        vec![],
        &[(0, "g9 ea ga g8"),(1, "eo ek e8 eu"),(1, "su go gu ez"),(2, "sz sa hz s7"),(3, "ha h7 hu h8"),(1, "ho s8 so h9"),(1, "e7 sk hk gz"),(1, "e9 s9 gk g7"),],
        [-50, 150, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/58-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["ho eu ez e7 gz g8 sa sz","hu e9 g9 ha hk h8 h7 s9","so gu gk g7 hz h9 sk s7","eo go su ea ek e8 ga s8",],
        vec![],
        vec![],
        &[(0, "sa s9 sk s8"),(0, "sz hu s7 go"),(3, "eo e7 e9 gu"),(3, "su eu ha so"),(2, "h9 ea g8 h7"),(3, "ga gz g9 g7"),(3, "e8 ez hk hz"),(0, "ho h8 gk ek"),],
        [50, 50, 50, -150],
    );
    test_rules(
        "../../testdata/games/solo/59-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go eu h8 ek e7 gk s8","ho hu h9 g9 g8 sz sk s9","so gu su ha hz h7 ea ga","hk ez e9 e8 gz g7 sa s7",],
        vec![],
        vec![],
        &[(0, "s8 sk ha s7"),(2, "gu hk eu h9"),(0, "gk g8 ga g7"),(2, "su sa h8 hu"),(1, "s9 h7 e8 e7"),(2, "ea ez ek ho"),(1, "g9 hz gz go"),(0, "eo sz so e9"),],
        [90, 90, -270, 90],
    );
    test_rules(
        "../../testdata/games/solo/6-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go hk ea e7 g9 g8 g7 s9","ha hz h7 e8 gz sz sk s8","ho eu hu ez ek e9 sa s7","eo so gu su h9 h8 ga gk",],
        vec![],
        vec![],
        &[(0, "g9 gz eu gk"),(2, "ez gu e7 e8"),(3, "eo hk h7 hu"),(3, "su go ha ho"),(0, "g8 hz sa ga"),(1, "sk s7 h8 s9"),(3, "so g7 s8 e9"),(3, "h9 ea sz ek"),],
        [50, 50, 50, -150],
    );
    // ../../testdata/games/solo/62-herz-solo.html has wrong format
    // ../../testdata/games/solo/63-herz-solo.html has wrong format
    // ../../testdata/games/solo/64-herz-solo.html has wrong format
    // ../../testdata/games/solo/66-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/67-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go ho eu gu su g9 g7","so g8 e9 e8 ha h8 sk s9","hu gz gk ea ez ek hz hk","ga e7 h9 h7 sa sz s8 s7",],
        vec![],
        vec![],
        &[(0, "eo g8 gk ga"),(0, "go so hu e7"),(0, "ho e9 gz h7"),(0, "eu e8 ek h9"),(0, "gu s9 hk s7"),(0, "su sk ez s8"),(0, "g9 h8 hz sz"),(0, "g7 ha ea sa"),],
        [300, -100, -100, -100],
    );
    test_rules(
        "../../testdata/games/solo/68-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go g8 ea e9 e8 e7 ha s8","so g9 hk h9 h7 sk s9 s7","eo ho gu su gz gk g7 hz","eu hu ga ez ek h8 sa sz",],
        vec![],
        vec![],
        &[(0, "ea so ho ek"),(2, "eo hu g8 g9"),(2, "su ga go hk"),(0, "e9 h7 hz ez"),(3, "sa s8 s7 gz"),(2, "gu eu ha sk"),(3, "h8 e7 h9 g7"),(2, "gk sz e8 s9"),],
        [-50, -50, 150, -50],
    );
    // ../../testdata/games/solo/7-eichel-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/70-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go hu ez e7 h8 sz sk s7","eu gz g7 e9 ha h9 sa s8","eo so gu su ga gk g9 e8","ho g8 ea ek hz hk h7 s9",],
        vec![],
        vec![],
        &[(0, "sk sa ga s9"),(2, "so ho go gz"),(0, "sz s8 eo g8"),(2, "g9 hz hu g7"),(0, "s7 e9 e8 ea"),(0, "ez eu su ek"),(1, "ha gk hk h8"),(2, "gu h7 e7 h9"),],
        [50, 50, -150, 50],
    );
    test_rules(
        "../../testdata/games/solo/72-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go eu gu g9 g8 ea s7","so gz ek e7 ha hk h7 sa","ho ga ez e9 hz h9 h8 s8","hu su gk g7 e8 sz sk s9",],
        vec![],
        vec![],
        &[(0, "go gz ga g7"),(0, "eo so ho su"),(0, "gu ek s8 gk"),(0, "eu e7 h8 hu"),(0, "ea h7 e9 e8"),(0, "s7 sa hz sz"),(1, "ha h9 s9 g8"),(0, "g9 hk ez sk"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/solo/73-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu h8 ez ek e9 ga gk s9","hu hk h7 gz g8 g7 sk s8","eo go su hz ea e7 sa s7","ho so gu ha h9 e8 g9 sz",],
        vec![],
        vec![],
        &[(0, "ga g7 hz g9"),(2, "go h9 h8 h7"),(2, "eo gu eu hk"),(2, "ea e8 ek hu"),(1, "sk sa sz s9"),(2, "e7 ha ez gz"),(3, "ho gk g8 su"),(3, "so e9 s8 s7"),],
        [50, 50, -150, 50],
    );
    test_rules(
        "../../testdata/games/solo/74-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eu ea e8 ga g7 sa sz s9","go gu e7 gz g9 h7 sk s7","ek e9 gk g8 hz h9 h8 s8","eo ho so hu su ez ha hk",],
        vec![],
        vec![],
        &[(0, "ga gz g8 ez"),(3, "so ea go ek"),(1, "h7 h9 ha eu"),(0, "sa sk s8 su"),(3, "ho e8 e7 e9"),(3, "eo g7 gu gk"),(3, "hk sz s7 hz"),(2, "h8 hu s9 g9"),],
        [-50, -50, -50, 150],
    );
    // ../../testdata/games/solo/75-herz-solo.html has wrong format
    // ../../testdata/games/solo/76-gras-solo.html has wrong format
    // ../../testdata/games/solo/79-herz-solo.html has wrong format
    // ../../testdata/games/solo/81-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/82-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["gu ga gk ea ez e7 sk s7","so g9 g8 e9 ha hz h8 s8","hu su e8 hk h9 h7 sz s9","eo go ho eu gz g7 ek sa",],
        vec![],
        vec![],
        &[(0, "sk s8 s9 sa"),(3, "go ga g8 su"),(3, "ho gu g9 hu"),(3, "eo gk so e8"),(3, "g7 ez ha h7"),(3, "ek ea e9 sz"),(0, "s7 h8 h9 gz"),(3, "eu e7 hz hk"),],
        [-90, -90, -90, 270],
    );
    // ../../testdata/games/solo/83-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/84-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["so eu gu su g9 g8 h9 sa","ho e9 hk h8 sz sk s8 s7","eo go hu ea ez ek e7 ga","e8 gz gk g7 ha hz h7 s9",],
        vec![],
        vec![],
        &[(0, "sa sz hu s9"),(2, "go e8 su e9"),(2, "eo g7 gu ho"),(2, "e7 ha eu hk"),(0, "h9 h8 ea h7"),(2, "ek hz so s7"),(0, "g9 s8 ga gk"),(2, "ez gz g8 sk"),],
        [-50, -50, 150, -50],
    );
    test_rules(
        "../../testdata/games/solo/86-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo ez ga hz hk sa s9 s8","go eu e8 e7 g7 ha h9 sz","ho hu su gz gk g9 h8 s7","so gu ea ek e9 g8 h7 sk",],
        vec![],
        vec![],
        &[(0, "ga g7 g9 g8"),(0, "hk ha h8 h7"),(1, "h9 su sk hz"),(2, "gz gu eo sz"),(0, "s8 eu s7 so"),(3, "e9 ez e7 hu"),(2, "gk ea sa go"),(1, "e8 ho ek s9"),],
        [90, 90, 90, -270],
    );
    test_rules(
        "../../testdata/games/solo/87-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["go so eu hu gz gk ek sa","gu g8 e9 e7 ha hz h9 h7","eo ho g9 g7 ez e8 h8 sz","su ga ea hk sk s9 s8 s7",],
        vec![],
        vec![],
        &[(0, "eu g8 ho ga"),(2, "sz sk sa gu"),(1, "e7 e8 ea ek"),(3, "s9 hu e9 h8"),(0, "so hz eo su"),(2, "ez hk gk h7"),(0, "go h9 g7 s7"),(0, "gz ha g9 s8"),],
        [-150, 50, 50, 50],
    );
    test_rules(
        "../../testdata/games/solo/9-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo eu hz hk h9 ga sa s7","gu e9 e8 e7 gk g7 sz sk","go ho h8 ea ez ek gz s9","so hu su ha h7 g9 g8 s8",],
        vec![],
        vec![],
        &[(0, "eo gu h8 h7"),(0, "h9 sz ho ha"),(2, "ea s8 hz e7"),(0, "ga g7 gz g8"),(0, "sa sk s9 su"),(3, "g9 hk gk go"),(2, "ek hu eu e8"),(0, "s7 e9 ez so"),],
        [-150, 50, 50, 50],
    );
    // ../../testdata/games/solo/90-herz-solo.html has wrong format
    // ../../testdata/games/solo/91-gras-solo.html has wrong format
    // ../../testdata/games/solo/92-herz-solo.html has wrong format
    // ../../testdata/games/solo/93-herz-solo.html has wrong format
    test_rules(
        "../../testdata/games/solo/94-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["hu su e8 g9 sz sk s9 s8","eo go ho so eu ek e7 h7","gu ea ga gz ha hz hk h9","ez e9 gk g8 g7 h8 sa s7",],
        vec![],
        vec![],
        &[(0, "sz eu h9 s7"),(1, "go gu ez e8"),(1, "eo ea e9 su"),(1, "so ha sa hu"),(1, "ho hk h8 g9"),(1, "h7 hz gk sk"),(2, "ga g7 s8 e7"),(1, "ek gz g8 s9"),],
        [-110, 330, -110, -110],
    );
    test_rules(
        "../../testdata/games/solo/96-herz-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorHerz>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["h7 ea ez ek e9 e7 gk sz","gu hk e8 ga gz g8 s9 s8","ho ha hz h8 g7 sa sk s7","eo go so eu hu su h9 g9",],
        vec![],
        vec![],
        &[(0, "gk ga g7 g9"),(1, "gz sk eu h7"),(3, "go ez hk h8"),(3, "eo e7 gu hz"),(3, "su sz e8 ho"),(2, "sa h9 e9 s8"),(3, "so ek s9 ha"),(3, "hu ea g8 s7"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/solo/97-gras-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorGras>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["eo go ho eu ga g9 e8 sa","hu gz gk g7 ek e9 s9 s8","so gu su e7 hz h9 h8 s7","g8 ea ez ha hk h7 sz sk",],
        vec![],
        vec![],
        &[(0, "go g7 su g8"),(0, "ho gk gu h7"),(0, "eo hu so hk"),(0, "eu gz e7 sk"),(0, "e8 ek hz ea"),(3, "ez g9 e9 s7"),(0, "sa s8 h8 sz"),(0, "ga s9 h9 ha"),],
        [270, -90, -90, -90],
    );
    test_rules(
        "../../testdata/games/solo/98-eichel-solo.html",
        &rulessololike_new_test::<SCoreSolo<STrumpfDeciderFarbe<SFarbeDesignatorEichel>>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 3)),
        ["ho eu hu ez e9 e7 sa sk","so gu su e8 gz g9 h9 s7","eo ek g7 hz hk h8 h7 s9","go ea ga gk g8 ha sz s8",],
        vec![],
        vec![],
        &[(0, "eu so ek ea"),(1, "s7 s9 s8 sa"),(0, "hu e8 eo go"),(2, "hk ha e9 h9"),(0, "ho su g7 g8"),(0, "e7 gu hz ga"),(1, "gz h8 gk ez"),(0, "sk g9 h7 sz"),],
        [-150, 50, 50, 50],
    );
}

#[test]
fn test_rulesgeier() {
    test_rules(
        "../../testdata/games/39.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eo ez eu gk g9 h8 su s9","go ho e7 gz g8 hk hu s7","so ea ek ga g7 ha sa sz","e9 e8 gu hz h9 h7 sk s8",],
        vec![],
        vec![],
        &[(0, "h8 hu ha h7"),(2, "so sk eo ho"),(0, "s9 s7 sa s8"),(2, "sz e8 su go"),(1, "gz ga gu g9"),(2, "ea e9 eu e7"),(2, "g7 h9 gk g8"),(0, "ez hk ek hz"),],
        [80, 80, -240, 80],
    );
    test_rules(
        "../../testdata/games/42.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI2,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["e9 e7 ga gz hu h8 s9 s8","go ek gu g9 h7 sa sz s7","eo ho ea ez g8 ha hz hk","so eu e8 gk g7 h9 sk su",],
        vec![],
        vec![],
        &[(0, "ga gu g8 gk"),(0, "gz g9 ho g7"),(2, "eo so e7 go"),(2, "ha h9 h8 h7"),(2, "ez e8 e9 ek"),(2, "hz eu hu s7"),(2, "ea su s8 sz"),(2, "hk sk s9 sa"),],
        [-60, -60, 180, -60],
    );
    test_rules(
        "../../testdata/games/geier/1.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/200, /*n_payout_schneider_schwarz*/50, SLaufendeParams::new(10, 2)),
        ["eo so ea eu e7 g8 sa s7","go ho ha hk h9 sz sk s9","ez e9 e8 gk gu g9 h7 su","ek ga gz g7 hz hu h8 s8",],
        vec![],
        vec![],
        &[(0, "eo ho h7 s8"),(0, "e7 sz ez ek"),(2, "su gz sa s9"),(0, "ea go e8 ga"),(1, "ha gk hz so"),(0, "eu sk e9 g7"),(0, "s7 hk g9 h8"),(0, "g8 h9 gu hu"),],
        [600, -200, -200, -200],
    );
    test_rules(
        "../../testdata/games/geier/10.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["go e9 e7 gz hk h7 su s7","so ek g9 g7 h9 sk s9 s8","ho e8 gk gu g8 hz hu sz","eo ea ez eu ga ha h8 sa",],
        vec![],
        vec![],
        &[(0, "e7 ek e8 ea"),(3, "eo go so ho"),(3, "ez e9 h9 sz"),(3, "sa s7 s8 g8"),(3, "ga gz g7 gu"),(3, "ha h7 g9 hu"),(3, "eu hk s9 hz"),(3, "h8 su sk gk"),],
        [-70, -70, -70, 210],
    );
    test_rules(
        "../../testdata/games/geier/2.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eo ho so ez e9 gz gk sa","go eu ga g8 hu h9 h8 s9","ea g9 g7 ha hz h7 sz sk","ek e8 e7 gu hk su s8 s7",],
        vec![],
        vec![],
        &[(0, "eo go g7 s7"),(0, "gz ga g9 gu"),(1, "s9 sk s8 sa"),(0, "ez eu ea ek"),(2, "ha hk ho h8"),(0, "so g8 h7 e7"),(0, "gk h9 hz e8"),(0, "e9 hu sz su"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/geier/3.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderTout>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eo so ea ez sa sz su s9","e8 e7 g7 ha hz h7 s8 s7","ho ga gz gk g9 g8 hu h9","go ek eu e9 gu hk h8 sk",],
        vec![],
        vec![],
        &[(0, "eo g7 ho go"),(0, "ea e8 h9 e9"),(0, "ez e7 hu eu"),(0, "sa s8 g8 sk"),(0, "sz s7 g9 h8"),(0, "su h7 gk hk"),(0, "s9 hz gz gu"),(0, "so ha ga ek"),],
        [300, -100, -100, -100],
    );
    test_rules(
        "../../testdata/games/geier/4.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eo ea gz gk g9 hu sa s7","ek eu ga g7 hz hk h8 s9","gu g8 ha h7 sz sk su s8","go ho so ez e9 e8 e7 h9",],
        vec![],
        vec![],
        &[(0, "ea ek sz e7"),(0, "sa s9 sk h9"),(0, "hu hz ha so"),(3, "go eo ga su"),(0, "gz g7 g8 ho"),(3, "ez s7 eu s8"),(3, "e9 g9 h8 h7"),(3, "e8 gk hk gu"),],
        [-50, -50, -50, 150],
    );
    test_rules(
        "../../testdata/games/geier/5.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["ho so ea ez ga h9 sa s7","eo gz g7 ha hz hu h8 su","ek e8 e7 g9 g8 hk h7 sk","go eu e9 gk gu sz s9 s8",],
        vec![3,0,1,],
        vec![],
        &[(0, "ho eo sk go"),(1, "ha hk sz h9"),(1, "hz h7 gk s7"),(1, "hu g9 gu so"),(0, "ea h8 e7 e9"),(0, "ez su e8 eu"),(0, "ga g7 g8 s8"),(0, "sa gz ek s9"),],
        [1680, -560, -560, -560],
    );
    test_rules(
        "../../testdata/games/geier/6.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["go ho so ez gz gk","e9 gu hz hk sa su","eo ea ek g9 ha h9","eu ga hu sz sk s9",],
        vec![2,],
        vec![],
        &[(0, "go e9 eo eu"),(2, "g9 ga gk gu"),(3, "sz so su h9"),(0, "gz sa ek s9"),(0, "ho hk ha sk"),(0, "ez hz ea hu"),],
        [300, -100, -100, -100],
    );
    test_rules(
        "../../testdata/games/geier/7.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI3,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["gz gk ha hk hu h8 h7 s8","eu e9 e7 gu g8 g7 sz su","go so ek ga g9 h9 sk s7","eo ho ea ez e8 hz sa s9",],
        vec![3,1,],
        vec![],
        &[(0, "s8 su s7 sa"),(3, "eo h7 g7 so"),(3, "ea h8 e7 ek"),(3, "ez hu e9 h9"),(3, "e8 hk eu sk"),(1, "g8 ga ho gk"),(3, "s9 gz sz g9"),(1, "gu go hz ha"),],
        [-200, -200, -200, 600],
    );
    test_rules(
        "../../testdata/games/geier/8.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI0,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["eo so ea eu e9 e8 e7 g9","go ho ek ga g7 ha hk h7","gk g8 hu h9 h8 sz s9 s7","ez gz gu hz sa sk su s8",],
        vec![],
        vec![],
        &[(0, "eo ho h8 s8"),(0, "ea ek h9 ez"),(0, "e9 g7 s7 su"),(0, "e8 h7 hu sk"),(0, "eu go sz hz"),(1, "ga gk gu g9"),(1, "ha s9 gz so"),(0, "e7 hk g8 sa"),],
        [150, -50, -50, -50],
    );
    test_rules(
        "../../testdata/games/geier/9.html",
        &rulessololike_new_test::<SCoreGenericGeier<STrumpfDeciderNoTrumpf>, SPayoutDeciderPointBased>(EPlayerIndex::EPI1,/*n_payout_base*/50, /*n_payout_schneider_schwarz*/10, SLaufendeParams::new(10, 2)),
        ["ho e8 gu g8 hz hu s9 s7","eo go so ez ek e9 hk h8","gz g9 g7 ha h9 sa sk su","ea eu e7 ga gk h7 sz s8",],
        vec![],
        vec![],
        &[(0, "e8 ek gz ea"),(3, "ga gu go g7"),(1, "eo g9 h7 ho"),(1, "h8 ha sz hz"),(2, "sk s8 s7 so"),(1, "ez h9 e7 g8"),(1, "hk su eu hu"),(1, "e9 sa gk s9"),],
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
        &[
            (0, "eo go ho so"),
            (0, "eu gu hu su"),
            (0, "ha hz hk h9"),
            (0, "ea ez ek e9"),
            (0, "ga gz gk g9"),
            (0, "sa sz sk s9"),
            (0, "e8 e7 g8 g7"),
            (0, "h8 h7 s8 s7"),
        ],
        [30, -10, -10, -10],
    );
    test_rules_manual(
        "0 has durchmarsch 120",
        &SRulesRamsch::new(10, VDurchmarsch::AtLeast(120)),
        vec![],
        vec![],
        &[
            (0, "eo go ho so"),
            (0, "eu gu hu su"),
            (0, "ha hz hk h9"),
            (0, "ea ez ek e9"),
            (0, "ga gz gk g9"),
            (0, "sa sz sk s9"),
            (0, "e8 e7 g8 g7"),
            (0, "h8 h7 s8 s7"),
        ],
        [30, -10, -10, -10],
    );
    test_rules_manual(
        "0 has 120, but no durchmarsch",
        &SRulesRamsch::new(10, VDurchmarsch::All),
        vec![],
        vec![],
        &[
            (0, "eo go ho so"),
            (0, "eu gu hu su"),
            (0, "ha hz hk h9"),
            (0, "ea ez ek e9"),
            (0, "ga gz gk g9"),
            (0, "sa sz sk s9"),
            (0, "e8 e7 g8 g7"),
            (0, "h7 h8 s8 s7"),
        ],
        [-30, 10, 10, 10],
    );
}

#[test]
fn test_rulesbettel() {
    test_rules_manual(
        "3 wins Bettel",
        &SRulesBettel::<SBettelAllAllowedCardsWithinStichNormal>::new(EPlayerIndex::EPI3, /*i_prio*/0, /*n_payout_base*/10),
        vec![],
        vec![],
        &[
            (0, "eo ez ek e9"),
            (2, "ho h9 ha hz"),
            (0, "h8 h7 hu so"),
            (2, "g8 g9 ga go"),
            (0, "e8 e7 gk su"),
            (0, "sa sz sk s9"),
            (0, "eu gz hk s7"),
            (0, "ea gu s8 g7"),
        ],
        [-10, -10, -10, 30],
    );
    test_rules_manual(
        "2 looses Bettel",
        &SRulesBettel::<SBettelAllAllowedCardsWithinStichNormal>::new(EPlayerIndex::EPI2, /*i_prio*/0, /*n_payout_base*/10),
        vec![],
        vec![],
        &[
            (0, "eo ez ek e9"),
            (2, "ho h9 ha hz"),
            (0, "h8 h7 hu so"),
            (2, "g8 g9 ga go"),
            (0, "e8 e7 gk su"),
            (0, "sa sz sk s9"),
            (0, "eu gz hk s7"),
            (0, "ea gu s8 g7"),
        ],
        [10, 10, -30, 10],
    );
}
