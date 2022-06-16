use crate::{
    game::SStichSequence,
    primitives::{
        eplayerindex::EPlayerIndex,
        hand::SHand,
        stich::SStich,
    },
    rules::TRules,
    util::*,
};

struct SStichOracle {
    vecstich: Vec<SStich>,
}

impl SStichOracle {
    pub fn new(
        ahand: &EnumMap<EPlayerIndex, SHand>,
        stichseq: &mut SStichSequence,
        rules: &dyn TRules,
    ) -> Self {
        fn for_each_allowed_card(
            n_depth: usize, // TODO? static enum type, possibly difference of EPlayerIndex
            ahand: &EnumMap<EPlayerIndex, SHand>,
            stichseq: &mut SStichSequence,
            rules: &dyn TRules,
            vecstich: &mut Vec<SStich>,
            stich_current_check: &SStich,
        ) {
            if n_depth==0 {
                assert!(stichseq.current_stich().is_empty());
                let stich = unwrap!(stichseq.completed_stichs().last()).clone();
                assert!(stich.is_full());
                assert!(stich.equal_up_to_size(&stich_current_check, stich_current_check.size()));
                vecstich.push(stich);
            } else {
                for card in rules.all_allowed_cards(
                    stichseq,
                    &ahand[unwrap!(stichseq.current_stich().current_playerindex())],
                ) {
                    stichseq.zugeben_and_restore(card, rules, |stichseq|
                        for_each_allowed_card(
                            n_depth-1,
                            ahand,
                            stichseq,
                            rules,
                            vecstich,
                            stich_current_check,
                        )
                    );
                }
            }
        }
        let n_stich_size = stichseq.current_stich().size();
        //assert!(0<=n_stich_size); // trivially true
        assert!(n_stich_size<=3);
        let mut vecstich = Vec::new();
        let stich_current_check = stichseq.current_stich().clone(); // TODO? debug-only
        for_each_allowed_card(
            4-n_stich_size,
            ahand,
            stichseq,
            rules,
            &mut vecstich,
            &stich_current_check,
        );
        SStichOracle{
            vecstich,
        }
    }
}

mod tests {
    #[test]
    fn test_stichoracle() {
        use crate::{
            game::SStichSequence,
            primitives::{
                card::{
                    card_values::*,
                    EFarbe,
                    EKurzLang,
                    SCard,
                },
                eplayerindex::EPlayerIndex,
                hand::SHand,
                stich::SStich,
            },
            rules::{
                payoutdecider::{SPayoutDeciderParams, SLaufendeParams},
                rulesrufspiel::SRulesRufspiel,
            },
            util::*,
        };
        use super::SStichOracle;
        let rules = SRulesRufspiel::new(
            EPlayerIndex::EPI0,
            EFarbe::Eichel,
            SPayoutDeciderParams::new(
                /*n_payout_base*/10,
                /*n_payout_schneider_schwarz*/10,
                SLaufendeParams::new(
                    /*n_payout_per_lauf*/10,
                    /*n_lauf_lbound*/3,
                ),
            ),
        );
        let assert_stichoracle = |
            aslccard_hand: [&[SCard]; EPlayerIndex::SIZE],
            slccard_stichseq: &[SCard],
            slcacard_stich: &[[SCard; EPlayerIndex::SIZE]],
        | {
            let stichoracle = SStichOracle::new(
                &EPlayerIndex::map_from_raw(aslccard_hand)
                    .map_into(|acard| SHand::new_from_iter(acard.iter().copied())),
                &mut SStichSequence::new_from_cards(
                    EKurzLang::Lang,
                    slccard_stichseq.iter().copied(),
                    &rules,
                ),
                &rules,
            );
            let setstich_oracle = stichoracle.vecstich.iter().cloned().collect::<std::collections::HashSet<_>>();
            let setstich_check = slcacard_stich
                .iter()
                .map(|acard| SStich::new_full(
                    EPlayerIndex::EPI0,
                    acard.clone(),
                ))
                .collect::<std::collections::HashSet<_>>();
            assert_eq!(setstich_oracle, setstich_check);
        };
        assert_stichoracle(
            [
                &[HO,SO,GU,SU,EK,GA,S9,S7],
                &[GO,HK,H8,H7,EA,SA,SK,S8],
                &[EU,HU,HA,EZ,E7,GZ,G9,G8],
                &[EO,HZ,H9,E9,E8,GK,G7,SZ],
            ],
            &[],
            &[
                // Opening with Trumpf
                [HO, GO, EU, EO], [SO, GO, EU, EO], [GU, GO, EU, EO], [SU, GO, EU, EO],
                [HO, HK, EU, EO], [SO, HK, EU, EO], [GU, HK, EU, EO], [SU, HK, EU, EO],
                [HO, H8, EU, EO], [SO, H8, EU, EO], [GU, H8, EU, EO], [SU, H8, EU, EO],
                [HO, H7, EU, EO], [SO, H7, EU, EO], [GU, H7, EU, EO], [SU, H7, EU, EO],
                [HO, GO, HU, EO], [SO, GO, HU, EO], [GU, GO, HU, EO], [SU, GO, HU, EO],
                [HO, HK, HU, EO], [SO, HK, HU, EO], [GU, HK, HU, EO], [SU, HK, HU, EO],
                [HO, H8, HU, EO], [SO, H8, HU, EO], [GU, H8, HU, EO], [SU, H8, HU, EO],
                [HO, H7, HU, EO], [SO, H7, HU, EO], [GU, H7, HU, EO], [SU, H7, HU, EO],
                [HO, GO, HA, EO], [SO, GO, HA, EO], [GU, GO, HA, EO], [SU, GO, HA, EO],
                [HO, HK, HA, EO], [SO, HK, HA, EO], [GU, HK, HA, EO], [SU, HK, HA, EO],
                [HO, H8, HA, EO], [SO, H8, HA, EO], [GU, H8, HA, EO], [SU, H8, HA, EO],
                [HO, H7, HA, EO], [SO, H7, HA, EO], [GU, H7, HA, EO], [SU, H7, HA, EO],
                [HO, GO, EU, HZ], [SO, GO, EU, HZ], [GU, GO, EU, HZ], [SU, GO, EU, HZ],
                [HO, HK, EU, HZ], [SO, HK, EU, HZ], [GU, HK, EU, HZ], [SU, HK, EU, HZ],
                [HO, H8, EU, HZ], [SO, H8, EU, HZ], [GU, H8, EU, HZ], [SU, H8, EU, HZ],
                [HO, H7, EU, HZ], [SO, H7, EU, HZ], [GU, H7, EU, HZ], [SU, H7, EU, HZ],
                [HO, GO, HU, HZ], [SO, GO, HU, HZ], [GU, GO, HU, HZ], [SU, GO, HU, HZ],
                [HO, HK, HU, HZ], [SO, HK, HU, HZ], [GU, HK, HU, HZ], [SU, HK, HU, HZ],
                [HO, H8, HU, HZ], [SO, H8, HU, HZ], [GU, H8, HU, HZ], [SU, H8, HU, HZ],
                [HO, H7, HU, HZ], [SO, H7, HU, HZ], [GU, H7, HU, HZ], [SU, H7, HU, HZ],
                [HO, GO, HA, HZ], [SO, GO, HA, HZ], [GU, GO, HA, HZ], [SU, GO, HA, HZ],
                [HO, HK, HA, HZ], [SO, HK, HA, HZ], [GU, HK, HA, HZ], [SU, HK, HA, HZ],
                [HO, H8, HA, HZ], [SO, H8, HA, HZ], [GU, H8, HA, HZ], [SU, H8, HA, HZ],
                [HO, H7, HA, HZ], [SO, H7, HA, HZ], [GU, H7, HA, HZ], [SU, H7, HA, HZ],
                [HO, GO, EU, H9], [SO, GO, EU, H9], [GU, GO, EU, H9], [SU, GO, EU, H9],
                [HO, HK, EU, H9], [SO, HK, EU, H9], [GU, HK, EU, H9], [SU, HK, EU, H9],
                [HO, H8, EU, H9], [SO, H8, EU, H9], [GU, H8, EU, H9], [SU, H8, EU, H9],
                [HO, H7, EU, H9], [SO, H7, EU, H9], [GU, H7, EU, H9], [SU, H7, EU, H9],
                [HO, GO, HU, H9], [SO, GO, HU, H9], [GU, GO, HU, H9], [SU, GO, HU, H9],
                [HO, HK, HU, H9], [SO, HK, HU, H9], [GU, HK, HU, H9], [SU, HK, HU, H9],
                [HO, H8, HU, H9], [SO, H8, HU, H9], [GU, H8, HU, H9], [SU, H8, HU, H9],
                [HO, H7, HU, H9], [SO, H7, HU, H9], [GU, H7, HU, H9], [SU, H7, HU, H9],
                [HO, GO, HA, H9], [SO, GO, HA, H9], [GU, GO, HA, H9], [SU, GO, HA, H9],
                [HO, HK, HA, H9], [SO, HK, HA, H9], [GU, HK, HA, H9], [SU, HK, HA, H9],
                [HO, H8, HA, H9], [SO, H8, HA, H9], [GU, H8, HA, H9], [SU, H8, HA, H9],
                [HO, H7, HA, H9], [SO, H7, HA, H9], [GU, H7, HA, H9], [SU, H7, HA, H9],
                // Opening with Eichel
                [EK, EA, EZ, E9], [EK, EA, E7, E9], [EK, EA, EZ, E8], [EK, EA, E7, E8],
                // Opening with Gras
                [GA, GO, GZ, GK], [GA, HK, GZ, GK], [GA, H8, GZ, GK], [GA, H7, GZ, GK], [GA, SA, GZ, GK], [GA, SK, GZ, GK], [GA, S8, GZ, GK],
                [GA, GO, G9, GK], [GA, HK, G9, GK], [GA, H8, G9, GK], [GA, H7, G9, GK], [GA, SA, G9, GK], [GA, SK, G9, GK], [GA, S8, G9, GK],
                [GA, GO, G8, GK], [GA, HK, G8, GK], [GA, H8, G8, GK], [GA, H7, G8, GK], [GA, SA, G8, GK], [GA, SK, G8, GK], [GA, S8, G8, GK],
                [GA, GO, GZ, G7], [GA, HK, GZ, G7], [GA, H8, GZ, G7], [GA, H7, GZ, G7], [GA, SA, GZ, G7], [GA, SK, GZ, G7], [GA, S8, GZ, G7],
                [GA, GO, G9, G7], [GA, HK, G9, G7], [GA, H8, G9, G7], [GA, H7, G9, G7], [GA, SA, G9, G7], [GA, SK, G9, G7], [GA, S8, G9, G7],
                [GA, GO, G8, G7], [GA, HK, G8, G7], [GA, H8, G8, G7], [GA, H7, G8, G7], [GA, SA, G8, G7], [GA, SK, G8, G7], [GA, S8, G8, G7],
                // Opening with Schelln
                [S9, SA, EU, SZ], [S7, SA, EU, SZ], [S9, SK, EU, SZ],
                [S7, SK, EU, SZ], [S9, S8, EU, SZ], [S7, S8, EU, SZ],
                [S9, SA, HU, SZ], [S7, SA, HU, SZ], [S9, SK, HU, SZ],
                [S7, SK, HU, SZ], [S9, S8, HU, SZ], [S7, S8, HU, SZ],
                [S9, SA, HA, SZ], [S7, SA, HA, SZ], [S9, SK, HA, SZ],
                [S7, SK, HA, SZ], [S9, S8, HA, SZ], [S7, S8, HA, SZ],
                [S9, SA, EZ, SZ], [S7, SA, EZ, SZ], [S9, SK, EZ, SZ],
                [S7, SK, EZ, SZ], [S9, S8, EZ, SZ], [S7, S8, EZ, SZ],
                [S9, SA, E7, SZ], [S7, SA, E7, SZ], [S9, SK, E7, SZ],
                [S7, SK, E7, SZ], [S9, S8, E7, SZ], [S7, S8, E7, SZ],
                [S9, SA, GZ, SZ], [S7, SA, GZ, SZ], [S9, SK, GZ, SZ],
                [S7, SK, GZ, SZ], [S9, S8, GZ, SZ], [S7, S8, GZ, SZ],
                [S9, SA, G9, SZ], [S7, SA, G9, SZ], [S9, SK, G9, SZ],
                [S7, SK, G9, SZ], [S9, S8, G9, SZ], [S7, S8, G9, SZ],
                [S9, SA, G8, SZ], [S7, SA, G8, SZ], [S9, SK, G8, SZ],
                [S7, SK, G8, SZ], [S9, S8, G8, SZ], [S7, S8, G8, SZ],
            ]
        );
        assert_stichoracle(
            [
                &[SO,GU,SU,EK,GA,S9,S7],
                &[GO,HK,H8,H7,EA,SA,SK,S8],
                &[EU,HU,HA,EZ,E7,GZ,G9,G8],
                &[EO,HZ,H9,E9,E8,GK,G7,SZ],
            ],
            &[HO],
            &[
                [HO, GO, EU, EO], [HO, HK, EU, EO], [HO, H8, EU, EO], [HO, H7, EU, EO],
                [HO, GO, HU, EO], [HO, HK, HU, EO], [HO, H8, HU, EO], [HO, H7, HU, EO],
                [HO, GO, HA, EO], [HO, HK, HA, EO], [HO, H8, HA, EO], [HO, H7, HA, EO],
                [HO, GO, EU, HZ], [HO, HK, EU, HZ], [HO, H8, EU, HZ], [HO, H7, EU, HZ],
                [HO, GO, HU, HZ], [HO, HK, HU, HZ], [HO, H8, HU, HZ], [HO, H7, HU, HZ],
                [HO, GO, HA, HZ], [HO, HK, HA, HZ], [HO, H8, HA, HZ], [HO, H7, HA, HZ],
                [HO, GO, EU, H9], [HO, HK, EU, H9], [HO, H8, EU, H9], [HO, H7, EU, H9],
                [HO, GO, HU, H9], [HO, HK, HU, H9], [HO, H8, HU, H9], [HO, H7, HU, H9],
                [HO, GO, HA, H9], [HO, HK, HA, H9], [HO, H8, HA, H9], [HO, H7, HA, H9],
            ]
        );
        assert_stichoracle(
            [
                &[SO,GU,SU,EK,GA,S9,S7],
                &[HK,H8,H7,EA,SA,SK,S8],
                &[EU,HU,HA,EZ,E7,GZ,G9,G8],
                &[EO,HZ,H9,E9,E8,GK,G7,SZ],
            ],
            &[HO, GO],
            &[
                [HO, GO, EU, EO], [HO, GO, HU, EO], [HO, GO, HA, EO],
                [HO, GO, EU, HZ], [HO, GO, HU, HZ], [HO, GO, HA, HZ],
                [HO, GO, EU, H9], [HO, GO, HU, H9], [HO, GO, HA, H9],
            ],
        );
        assert_stichoracle(
            [
                &[SO,GU,SU,EK,GA,S9,S7],
                &[HK,H8,H7,EA,SA,SK,S8],
                &[HU,HA,EZ,E7,GZ,G9,G8],
                &[EO,HZ,H9,E9,E8,GK,G7,SZ],
            ],
            &[HO, GO, EU],
            &[[HO, GO, EU, EO], [HO, GO, EU, HZ], [HO, GO, EU, H9]],
        );
    }
}
