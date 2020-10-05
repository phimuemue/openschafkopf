use crate::game_analysis::*;
use crate::game::*;
use crate::sauspiel::*;
use crate::primitives::*;
use crate::util::*;
use std::io::Read;
use itertools::Itertools;

fn analyze_plain(str_plain: &str) -> Result<SAnalyzeParams, failure::Error> {
    let (str_rules, str_cards) = str_plain
        .split(":")
        .collect_tuple()
        .ok_or_else(|| format_err!("':' does not separate rules from stichs."))?;
    let str_cards = str_cards.trim();
    let rules = crate::rules::parser::parse_rule_description(
        str_rules,
        (/*n_tarif_extra*/10, /*n_tarif_ruf*/10, /*n_tarif_solo*/50), // TODO? make customizable
        /*fn_player_to_epi*/|str_epi| EPlayerIndex::checked_from_usize(str_epi.parse()?)
            .ok_or_else(|| format_err!("Cannot convert {} to EPlayerIndex.", str_epi)),
    )?;
    let veccard = crate::primitives::cardvector::parse_cards::<Vec<_>>(str_cards)
        .ok_or_else(|| format_err!("Could not parse cards: {}", str_cards))?;
    let stichseq = veccard.iter().fold(
        SStichSequence::new(
            /*ekurzlang*/EKurzLang::values()
                .find(|ekurzlang| ekurzlang.cards_per_player()*EPlayerIndex::SIZE==veccard.len())
                .ok_or_else(|| format_err!("Incorrect number of cards: {}", veccard.len()))?
        ),
        mutate_return!(|stichseq, &card| {
            stichseq.zugeben(card, rules.as_ref());
        })
    );
    Ok(SAnalyzeParams {
        rules,
        ahand: EPlayerIndex::map_from_fn(|epi|
            SHand::new_from_vec(
                stichseq
                    .completed_stichs()
                    .iter()
                    .map(|stich| stich[epi])
                    .collect()
            )
        ),
        vecn_doubling: vec![],
        vecn_stoss: vec![],
        n_stock: 0,
        vecstich: stichseq.completed_stichs().to_vec(),
    })
}

pub fn analyze<
    'str_sauspiel_html_file,
>(path_analysis: &std::path::Path, itstr_sauspiel_html_file: impl Iterator<Item=&'str_sauspiel_html_file str>) -> Result<(), Error> {
    let mut vecanalyzeparams = Vec::new();
    for str_file_sauspiel_html in itstr_sauspiel_html_file {
        for globresult in glob::glob(str_file_sauspiel_html)? {
            match globresult {
                Ok(path) => {
                    println!("Opening {:?}", path);
                    vecanalyzeparams.push(SAnalyzeParamsWithDesc{
                        str_description: path.to_string_lossy().into_owned(),
                        str_link: format!("file://{}", path.to_string_lossy()),
                        resanalyzeparams: {
                            let str_input = &via_out_param_result(|str_html|
                                std::fs::File::open(&path)?.read_to_string(str_html)
                            )?.0;
                            analyze_html(&str_input)
                                .or_else(|_e| analyze_plain(&str_input))
                        },
                    });
                },
                Err(e) => {
                    println!("Error: {:?}. Trying to continue.", e);
                },
            }
        }
    }
    analyze_games(
        path_analysis,
        /*fn_link*/|str_description: &str| str_description.to_string(),
        vecanalyzeparams.into_iter(),
    )
}
