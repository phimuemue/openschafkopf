use combine::{char::*, *};
use failure::{format_err, Error};

pub fn parse_trimmed<'str_in, P: combine::Parser<Input=&'str_in str>>(
    str_in: &'str_in str,
    str_semantics: &str,
    parser: P,
) -> Result<P::Output, Error> {
    spaces().with(parser).skip((spaces(), eof()))
        .parse(str_in)
        .map(|pairoutconsumed| pairoutconsumed.0)
        .map_err(|err|
            format_err!("Error in parsing {}: {:?}", str_semantics, err)
        )
}
