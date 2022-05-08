use crate::ai::{*, handiterators::*};
use crate::game::SStichSequence;
use crate::primitives::*;
use crate::util::*;
use crate::rules::*;
use itertools::Itertools;

pub use super::handconstraint::*;

enum VChooseItAhand {
    All,
    Sample(usize),
}

pub fn subcommand_given_game(str_subcommand: &'static str, str_about: &'static str) -> clap::Command<'static> {
    clap::Command::new(str_subcommand)
        .about(str_about)
        .arg(clap::Arg::new("rules").long("rules").takes_value(true).required(true))
        .arg(clap::Arg::new("hand").long("hand").takes_value(true).required(true))
        .arg(clap::Arg::new("cards_on_table").long("cards-on-table").takes_value(true))
        .arg(clap::Arg::new("simulate_hands").long("simulate-hands").takes_value(true))
        .arg(clap::Arg::new("verbose").long("verbose").short('v'))
        .arg(clap::Arg::new("constrain_hands").long("constrain-hands").takes_value(true))
}

pub trait TWithCommonArgs {
    fn call<'rules>(
        self,
        rules: &'rules dyn TRules,
        itahand: Box<dyn Iterator<Item=EnumMap<EPlayerIndex, SHand>>+Send+'rules>,
        eremainingcards: ERemainingCards,
        determinebestcard: SDetermineBestCard,
        b_verbose: bool,
    ) -> Result<(), Error>;
}

pub fn with_common_args(
    clapmatches: &clap::ArgMatches,
    withcommanargs: impl TWithCommonArgs
) -> Result<(), Error> {
    let b_verbose = clapmatches.is_present("verbose");
    let rules = crate::rules::parser::parse_rule_description_simple(unwrap!(clapmatches.value_of("rules")))?;
    let rules = rules.as_ref();
    let vecocard_hand = cardvector::parse_optional_cards::<Vec<_>>(unwrap!(clapmatches.value_of("hand")))
        .ok_or_else(||format_err!("Could not parse hand."))?;
    let veccard_as_played = match clapmatches.value_of("cards_on_table") {
        None => Vec::new(),
        Some(str_cards_on_table) => cardvector::parse_cards(str_cards_on_table)
            .ok_or_else(||format_err!("Could not parse played cards"))?,
    };
    let veccard_duplicate = veccard_as_played.iter()
        .chain(vecocard_hand.iter().filter_map(|ocard| ocard.as_ref()))
        .duplicates()
        .collect::<Vec<_>>();
    if !veccard_duplicate.is_empty() {
        bail!("Cards are used more than once: {}", veccard_duplicate.iter().join(", "));
    }
    let mapekurzlangostichseq = EKurzLang::map_from_fn(|ekurzlang| {
        let mut stichseq = SStichSequence::new(ekurzlang);
        for &card in veccard_as_played.iter() {
            if !ekurzlang.supports_card(card) {
                return None; // TODO distinguish error
            }
            if stichseq.game_finished() {
                return None; // TODO distinguish error
            }
            stichseq.zugeben(card, rules);
        }
        Some(stichseq)
    });
    let (ekurzlang, ahand_fixed) = EKurzLang::values()
        .filter_map(|ekurzlang| {
            mapekurzlangostichseq[ekurzlang].as_ref().and_then(|stichseq| {
                let epi_fixed = unwrap!(stichseq.current_stich().current_playerindex());
                if_then_some!(
                    vecocard_hand.iter().all(Option::is_some)
                        && stichseq.remaining_cards_per_hand()[epi_fixed]==vecocard_hand.len(),
                    (
                        ekurzlang,
                        SHand::new_from_iter(
                            vecocard_hand.iter().map(|ocard| unwrap!(ocard))
                        ).to_ahand(epi_fixed),
                    )
                ).or_else(|| {
                    let n_cards_total = ekurzlang.cards_per_player()*EPlayerIndex::SIZE;
                    if stichseq.visible_cards().count()+vecocard_hand.len()==n_cards_total {
                        let an_remaining = stichseq.remaining_cards_per_hand();
                        let mut ai_card = an_remaining.explicit_clone();
                        let mut n_remaining = 0;
                        for epi in EPlayerIndex::values() {
                            ai_card[epi] += n_remaining;
                            n_remaining = ai_card[epi];
                        }
                        use EPlayerIndex::*;
                        let make_hand = |i_lo, i_hi| {
                            SHand::new_from_iter(
                                vecocard_hand[i_lo..i_hi].iter()
                                    .copied()
                                    .filter_map(|ocard| ocard)
                            )
                        };
                        let ahand = EPlayerIndex::map_from_raw([
                            make_hand(0, ai_card[EPI0]),
                            make_hand(ai_card[EPI0], ai_card[EPI1]),
                            make_hand(ai_card[EPI1], ai_card[EPI2]),
                            make_hand(ai_card[EPI2], ai_card[EPI3]),
                        ]);
                        for epi in EPlayerIndex::values() {
                            assert!(ahand[epi].cards().len() <= an_remaining[epi]);
                        }
                        if ahand[epi_fixed].cards().len()==an_remaining[epi_fixed] {
                            Some((
                                ekurzlang,
                                ahand,
                            ))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
            })
        })
        .exactly_one()
        .map_err(|err| format_err!("Could not determine ekurzlang: {}", err))?;
    // TODO check that everything is ok (no duplicate cards, cards are allowed, current stich not full, etc.)
    let stichseq = unwrap!(mapekurzlangostichseq[ekurzlang].as_ref());
    let oconstraint = if_then_some!(let Some(str_constrain_hands)=clapmatches.value_of("constrain_hands"), {
        let relation = str_constrain_hands.parse::<VConstraint>().map_err(|_|format_err!("Cannot parse hand constraints"))?;
        if b_verbose {
            println!("Constraint parsed as: {}", relation);
        }
        relation
    });
    use VChooseItAhand::*;
    use ERemainingCards::*;
    let oiteratehands = if_then_some!(let Some(str_itahand)=clapmatches.value_of("simulate_hands"),
        if "all"==str_itahand.to_lowercase() {
            All
        } else if let Ok(n_samples)=str_itahand.parse() {
            Sample(n_samples)
        } else {
            bail!("Failed to parse simulate_hands");
        }
    );
    let epi_fixed = unwrap!(stichseq.current_stich().current_playerindex());
    let hand_fixed = ahand_fixed[epi_fixed].clone(); // TODO can we get rid of this?
    let determinebestcard =  SDetermineBestCard::new(
        rules,
        stichseq,
        &hand_fixed,
    );
    assert_eq!(epi_fixed, determinebestcard.epi_fixed);
    let mapepin_cards_per_hand = stichseq.remaining_cards_per_hand();
    assert_eq!(mapepin_cards_per_hand[epi_fixed], hand_fixed.cards().len());
    let eremainingcards = unwrap!(ERemainingCards::checked_from_usize(mapepin_cards_per_hand[epi_fixed] - 1));
    macro_rules! forward{(($itahand: expr), ) => { // TODORUST generic closures
        withcommanargs.call(
            rules,
            Box::new($itahand),
            eremainingcards,
            determinebestcard,
            b_verbose,
        )
    }}
    cartesian_match!(forward,
        match ((oiteratehands, eremainingcards)) {
            (Some(All), _)|(None, _1|_2|_3|_4) => (
                all_possible_hands(stichseq, ahand_fixed, epi_fixed, rules)
                    .filter(|ahand| oconstraint.as_ref().map_or(true, |relation|
                        relation.eval(ahand, rules)
                    ))
            ),
            (Some(Sample(n_samples)), _) => (
                forever_rand_hands(stichseq, ahand_fixed, epi_fixed, rules)
                    .filter(|ahand| oconstraint.as_ref().map_or(true, |relation|
                        relation.eval(ahand, rules)
                    ))
                    .take(n_samples)
            ),
            (None, _5|_6|_7|_8) => (
                forever_rand_hands(stichseq, ahand_fixed, epi_fixed, rules)
                    .filter(|ahand| oconstraint.as_ref().map_or(true, |relation|
                        relation.eval(ahand, rules)
                    ))
                    .take(/*n_suggest_card_samples*/50)
            ),
        },
    )
}

