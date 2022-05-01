use crate::ai::{*, handiterators::*};
use crate::game::SStichSequence;
use crate::primitives::*;
use crate::util::*;
use crate::rules::*;

pub use super::handconstraint::*;

enum VChooseItAhand {
    All,
    Sample(usize),
    Concrete(Vec<SCard>),
}

pub fn subcommand_given_game(str_subcommand: &'static str, str_about: &'static str) -> clap::Command<'static> {
    let single_arg = |str_name, str_long| {
        clap::Arg::new(str_name)
            .long(str_long)
            .required(true)
            .takes_value(true)
    };
    clap::Command::new(str_subcommand)
        .about(str_about)
        .arg(single_arg("rules", "rules"))
        .arg(single_arg("hand", "hand"))
        .arg(single_arg("cards_on_table", "cards-on-table"))
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
    let hand_fixed = super::str_to_hand(unwrap!(clapmatches.value_of("hand")))?;
    let veccard_as_played = &cardvector::parse_cards::<Vec<_>>(
        unwrap!(clapmatches.value_of("cards_on_table")),
    ).ok_or_else(||format_err!("Could not parse played cards"))?;
    // TODO check that everything is ok (no duplicate cards, cards are allowed, current stich not full, etc.)
    let rules = crate::rules::parser::parse_rule_description_simple(unwrap!(clapmatches.value_of("rules")))?;
    let rules = rules.as_ref();
    let ekurzlang = EKurzLang::checked_from_cards_per_player(
        /*n_stichs_complete*/veccard_as_played.len() / EPlayerIndex::SIZE
            + hand_fixed.cards().len()
    ).ok_or_else(|| format_err!("Cannot determine ekurzlang from {} and {:?}.", hand_fixed, veccard_as_played))?;
    let stichseq = SStichSequence::new_from_cards(
        ekurzlang,
        veccard_as_played.iter().copied(),
        rules
    );
    let determinebestcard =  SDetermineBestCard::new(
        rules,
        &stichseq,
        &hand_fixed,
    );
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
        } else if let Some(veccard)=cardvector::parse_cards(str_itahand) {
            Concrete(veccard)
        } else {
            bail!("Failed to parse simulate_hands");
        }
    );
    let epi_fixed = determinebestcard.epi_fixed;
    let mapepin_cards_per_hand = remaining_cards_per_hand(&stichseq);
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
            (Some(Concrete(veccard)), _) => ({
                // TODO error handling
                // TODO should we offer an option such that hand_fixed can be specified in line with other hands?
                let mut ahand = EPlayerIndex::map_from_fn(|_epi| SHand::new_from_vec(SHandVector::new()));
                let mut i_card_lo = 0;
                for epi in EPlayerIndex::values() {
                    if epi==epi_fixed {
                        ahand[verify_eq!(epi, epi_fixed)] = hand_fixed.clone();
                    } else {
                        let i_card_hi = i_card_lo + mapepin_cards_per_hand[epi];
                        ahand[epi] = SHand::new_from_iter(veccard[i_card_lo..i_card_hi].iter().copied()); 
                        i_card_lo = i_card_hi;
                    }
                }
                std::iter::once(ahand)
            }),
            (Some(All), _)|(None, _1|_2|_3|_4) => (
                all_possible_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                    .filter(|ahand| oconstraint.as_ref().map_or(true, |relation|
                        relation.eval(ahand, rules)
                    ))
            ),
            (Some(Sample(n_samples)), _) => (
                forever_rand_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                    .filter(|ahand| oconstraint.as_ref().map_or(true, |relation|
                        relation.eval(ahand, rules)
                    ))
                    .take(n_samples)
            ),
            (None, _5|_6|_7|_8) => (
                forever_rand_hands(&stichseq, hand_fixed.clone(), epi_fixed, rules)
                    .filter(|ahand| oconstraint.as_ref().map_or(true, |relation|
                        relation.eval(ahand, rules)
                    ))
                    .take(/*n_suggest_card_samples*/50)
            ),
        },
    )
}

