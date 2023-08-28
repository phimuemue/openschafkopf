use combine::{char::*, *};

pub fn parse_trimmed<'str_in, P: combine::Parser<Input = &'str_in str>>(
    str_in: &'str_in str,
    parser: P,
) -> Result<P::Output, combine::error::StringStreamError> {
    (spaces(), parser, spaces(), eof())
        .parse(str_in)
        .map(|tploutconsumed| tploutconsumed.0.1)
}
