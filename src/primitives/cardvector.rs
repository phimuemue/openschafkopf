extern crate combine;

use primitives::card::*;
use self::combine::*;
use std::iter::FromIterator;

pub fn parse_cards<C>(str_cards: &str) -> Option<C>
    where C: FromIterator<SCard>
{
    use std::fmt;
    use std::error::Error as StdError;
    #[derive(Debug)]
    struct ParseEnumError;
    impl fmt::Display for ParseEnumError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            write!(f, "error")
        }
    }
    impl StdError for ParseEnumError {
        fn description(&self) -> &str {
            "error"
        }
    }
    spaces()
        .with(sep_by::<C,_,_>(
            (
                letter().and_then(|chr_farbe| { match chr_farbe {
                    'e'|'E'=> Ok(EFarbe::Eichel),
                    'g'|'G'=> Ok(EFarbe::Gras),
                    'h'|'H'=> Ok(EFarbe::Herz),
                    's'|'S'=> Ok(EFarbe::Schelln),
                    _ => Err(ParseEnumError),
                } } ),
                alpha_num().and_then(|chr_schlag| { match chr_schlag {
                    '7'    => Ok(ESchlag::S7),
                    '8'    => Ok(ESchlag::S8),
                    '9'    => Ok(ESchlag::S9),
                    'z'|'Z'=> Ok(ESchlag::Zehn),
                    'u'|'U'=> Ok(ESchlag::Unter),
                    'o'|'O'=> Ok(ESchlag::Ober),
                    'k'|'K'=> Ok(ESchlag::Koenig),
                    'a'|'A'=> Ok(ESchlag::Ass),
                    _ => Err(ParseEnumError),
                } } )
            ).map(|(efarbe, eschlag)| SCard::new(efarbe, eschlag)),
            spaces()
        ))
        .skip(spaces())
        .skip(eof())
        // end of parser
        .parse(str_cards)
        .ok()
        .map(|pairoutconsumed| pairoutconsumed.0)
}

#[test]
fn test_cardvectorparser() {
    let veccard = parse_cards::<Vec<_>>("ek gk hz hu s7").unwrap();
    assert_eq!(veccard.len(), 5);
    assert!(veccard[1] == SCard::new(EFarbe::Gras, ESchlag::Koenig));
}
