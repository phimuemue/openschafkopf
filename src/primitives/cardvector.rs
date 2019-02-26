extern crate combine;

use crate::primitives::card::*;
use self::combine::{*, char::{spaces, letter, alpha_num,}, error::{StringStreamError},};

pub fn parse_cards<C: std::iter::Extend<SCard>+Default>(str_cards: &str) -> Option<C> {
    spaces()
        .with(sep_by::<C,_,_>(
            (
                letter().and_then(|chr_farbe| { match chr_farbe {
                    'e'|'E'=> Ok(EFarbe::Eichel),
                    'g'|'G'=> Ok(EFarbe::Gras),
                    'h'|'H'=> Ok(EFarbe::Herz),
                    's'|'S'=> Ok(EFarbe::Schelln),
                    _ => Err(StringStreamError::UnexpectedParse),
                } } ),
                alpha_num().and_then(|chr_schlag| { match chr_schlag {
                    '7'    => Ok(ESchlag::S7),
                    '8'    => Ok(ESchlag::S8),
                    '9'    => Ok(ESchlag::S9),
                    'z'|'Z'|'x'|'X'=> Ok(ESchlag::Zehn), // support both our own and sauspiel notation
                    'u'|'U'=> Ok(ESchlag::Unter),
                    'o'|'O'=> Ok(ESchlag::Ober),
                    'k'|'K'=> Ok(ESchlag::Koenig),
                    'a'|'A'=> Ok(ESchlag::Ass),
                    _ => Err(StringStreamError::UnexpectedParse),
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
    use crate::util::*;
    assert_eq!(
        verify!(parse_cards::<Vec<_>>("ek gk hz hu s7")).unwrap(),
        vec![
            SCard::new(EFarbe::Eichel, ESchlag::Koenig),
            SCard::new(EFarbe::Gras, ESchlag::Koenig),
            SCard::new(EFarbe::Herz, ESchlag::Zehn),
            SCard::new(EFarbe::Herz, ESchlag::Unter),
            SCard::new(EFarbe::Schelln, ESchlag::S7),
        ]
    );
}
