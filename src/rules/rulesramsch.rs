use card::*;
use stich::*;
use hand::*;
use rules::*;
use rules::trumpfdecider::*;
use std::fmt;
use std::cmp::Ordering;

pub struct SRulesRamsch {}

impl fmt::Display for SRulesRamsch {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Ramsch")
    }
}

pub type STrumpfDeciderRamsch = STrumpfDeciderSchlag<
    SSchlagDesignatorOber, STrumpfDeciderSchlag<
    SSchlagDesignatorUnter, STrumpfDeciderFarbe<
    SFarbeDesignatorHerz, STrumpfDeciderNoTrumpf>>>;

impl TRules for SRulesRamsch {
    fn trumpf_or_farbe(&self, card: SCard) -> VTrumpfOrFarbe {
        STrumpfDeciderRamsch::trumpf_or_farbe(card)
    }

    fn playerindex(&self) -> Option<EPlayerIndex> {
        None
    }

    fn payout(&self, vecstich: &Vec<SStich>) -> [isize; 4] {
        let an_points = create_playerindexmap(|eplayerindex| {
            self.points_per_player(vecstich, eplayerindex)
        });
        let n_points_max = an_points.iter().max().unwrap().clone();
        let veceplayerindex_most_points : Vec<EPlayerIndex> = (0..4)
            .map(|eplayerindex| eplayerindex as usize)
            .filter(|eplayerindex| n_points_max==an_points[*eplayerindex])
            .collect();
        let n_price = 10;
        let eplayerindex_loser : EPlayerIndex = {
            if 1==veceplayerindex_most_points.len() {
                veceplayerindex_most_points[0]
            } else {
                let vecpaireplayerindexcard_highest_trumpf = veceplayerindex_most_points.into_iter()
                    .map(|eplayerindex| {
                        let hand = SHand::new_from_vec(
                            vecstich.iter()
                                .map(|stich| stich[eplayerindex])
                                .filter(|card| self.is_trumpf(*card))
                                .collect()
                        );
                        // If two ore more players have the maximum number of points,
                        // they both must have had at least one trumpf.
                        assert!(0<hand.cards().len());
                        // TODO: introduce max_by_cmp
                        let mut card_highest_trumpf = hand.cards()[0];
                        for card in hand.cards().iter().skip(1).cloned() {
                            if Ordering::Greater==STrumpfDeciderRamsch::compare_trumpfcards_solo(card, card_highest_trumpf) {
                                card_highest_trumpf = card;
                            }
                        }
                        (eplayerindex, card_highest_trumpf)
                    })
                    .collect::<Vec<_>>();
                // TODO: introduce max_by_cmp
                let (mut eplayerindex_highest_trumpf, mut card_highest_trumpf) = vecpaireplayerindexcard_highest_trumpf[0].clone();
                for (eplayerindex, card) in vecpaireplayerindexcard_highest_trumpf.into_iter().skip(1) {
                    if Ordering::Greater==STrumpfDeciderRamsch::compare_trumpfcards_solo(card, card_highest_trumpf) {
                        eplayerindex_highest_trumpf = eplayerindex;
                        card_highest_trumpf = card;
                    }
                }
                eplayerindex_highest_trumpf
            }
        };
        create_playerindexmap(|eplayerindex| {
            if eplayerindex_loser==eplayerindex {
                -3 * n_price
            } else {
                n_price
            }
        })
    }

    fn all_allowed_cards_first_in_stich(&self, _vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        hand.cards().clone()
    }

    fn all_allowed_cards_within_stich(&self, vecstich: &Vec<SStich>, hand: &SHand) -> SHandVector {
        assert!(!vecstich.is_empty());
        let card_first = vecstich.last().unwrap().first_card();
        let veccard_allowed : SHandVector = hand.cards().iter()
            .filter(|&&card| self.trumpf_or_farbe(card)==self.trumpf_or_farbe(card_first))
            .cloned()
            .collect();
        if veccard_allowed.is_empty() {
            hand.cards().clone()
        } else {
            veccard_allowed
        }
    }

    fn compare_in_stich_trumpf(&self, card_fst: SCard, card_snd: SCard) -> Ordering {
        STrumpfDeciderRamsch::compare_trumpfcards_solo(card_fst, card_snd)
    }
}
