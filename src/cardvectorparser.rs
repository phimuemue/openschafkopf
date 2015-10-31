extern crate combine;

use card::*;
use self::combine::*;
use self::combine::combinator::FnParser;
use self::combine::primitives::Stream;

// For now, parsing is only used to simplify input in programming.
// But it is clear that these methods are far from perfect.
// TODO: case-insensitive parsing
// TODO: error handling
// TODO farbe_parse and schlag_parse should be simplified (eliminate "duplicate enumeration")
// TODO: enable parsing stuff like egho gazk9 s7

fn farbe_parse<I>(input: State<I>) -> ParseResult<EFarbe, I>
where I: Stream<Item=char> {
    combine::choice([
        combine::char('e'),
        combine::char('g'),
        combine::char('h'),
        combine::char('s'),
    ])
    .map(|chrFarbe| {
        match chrFarbe {
            'e' => efarbeEICHEL,
            'g' => efarbeGRAS,
            'h' => efarbeHERZ,
            's' => efarbeSCHELLN,
            _ => unreachable!(),
        }
    } )
    .parse_state(input)
}

fn schlag_parse<I>(input: State<I>) -> ParseResult<ESchlag, I>
where I: Stream<Item=char> {
    combine::choice([
        combine::char('7'),
        combine::char('8'),
        combine::char('9'),
        combine::char('z'),
        combine::char('u'),
        combine::char('o'),
        combine::char('k'),
        combine::char('a'),
    ])
    .map(|chrFarbe| {
        match chrFarbe {
            '7' => eschlag7,
            '8' => eschlag8,
            '9' => eschlag9,
            'z' => eschlagZ,
            'u' => eschlagU,
            'o' => eschlagO,
            'k' => eschlagK,
            'a' => eschlagA,
            _ => unreachable!(),
        }
    } )
    .parse_state(input)
}

fn card_parse<I>(input: State<I>) -> ParseResult<CCard, I>
where I: Stream<Item=char> {
    (parser(farbe_parse), parser(schlag_parse))
        .map(|(efarbe, eschlag)| CCard::new(efarbe, eschlag))
        .parse_state(input)
}

pub fn ParseCards(strCards: &str) -> Vec<CCard> {
    sep_by::<Vec<_>,_,_>(parser(card_parse), spaces())
        .parse(strCards)
        .unwrap()
        .0
}
