// Copyright (C) 2021 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use nom::bytes::complete::take_while;
use nom::character::complete::space0;
use nom::character::is_alphanumeric;
use nom::{Err, IResult, Needed};
use std::str;

use super::HeaderParseError;

pub(super) fn cond_parser<I, O1, O2, E: nom::error::ParseError<I>, F, G>(
    mut cond: F,
    mut parser: G,
) -> impl FnMut(I) -> IResult<I, Option<O2>, E>
where
    F: nom::Parser<I, O1, E>,
    G: nom::Parser<I, O2, E>,
    I: Clone,
{
    move |input: I| {
        let (input, res) = match cond.parse(input.clone()) {
            Ok((input, output)) => Ok((input, Some(output))),
            Err(Err::Error(_)) => Ok((input, None)),
            Err(err) => Err(err),
        }?;

        if res.is_some() {
            let (input, res) = parser.parse(input)?;
            Ok((input, Some(res)))
        } else {
            Ok((input, None))
        }
    }
}

pub(super) fn trim<I, O, E: nom::error::ParseError<I>, F>(
    mut parser: F,
) -> impl FnMut(I) -> IResult<I, O, E>
where
    F: nom::Parser<I, O, E>,
    I: nom::InputTakeAtPosition,
    <I as nom::InputTakeAtPosition>::Item: nom::AsChar + Clone,
{
    move |input: I| {
        let (input, _) = space0(input)?;
        let (input, val) = parser.parse(input)?;
        let (input, _) = space0(input)?;

        Ok((input, val))
    }
}

/// Parse an unsigned integer from a string of ASCII digits.
///
/// Unlike `str::parse()`, a leading `+` sign is rejected.
pub(crate) fn parse_digits<T>(s: &str) -> Result<T, ()>
where
    T: std::str::FromStr,
{
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(());
    }

    T::from_str(s).map_err(|_| ())
}

/// Parse a float in the `FLOAT`/`POS-FLOAT` form of RFC 7826 section 20.
///
/// Unlike `str::parse()`, scientific notation, `NaN`, `inf`, a leading
/// `+` sign and, if `allow_negative` is false, a leading `-` sign are
/// rejected.
pub(crate) fn parse_float(s: &str, allow_negative: bool) -> Result<f64, ()> {
    let bytes = s.as_bytes();
    let mut start = 0;

    if let Some(&first) = bytes.first() {
        if first == b'-' {
            if !allow_negative {
                return Err(());
            }
            start = 1;
        }
    }

    let mut digits_before_dot = 0;
    let mut digits_after_dot = 0;
    let mut seen_dot = false;

    for &byte in &bytes[start..] {
        match byte {
            b'0'..=b'9' => {
                if seen_dot {
                    digits_after_dot += 1;
                } else {
                    digits_before_dot += 1;
                }
            }
            b'.' if !seen_dot => seen_dot = true,
            _ => return Err(()),
        }
    }

    if digits_before_dot == 0 || (seen_dot && digits_after_dot == 0) {
        return Err(());
    }

    let value = s.parse::<f64>().map_err(|_| ())?;
    if !value.is_finite() {
        return Err(());
    }

    Ok(value)
}

pub(super) fn is_token_char(i: u8) -> bool {
    is_alphanumeric(i) || b"!#$%&'*+-.^_`|~".contains(&i)
}

pub(super) fn token(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while(is_token_char)(input)
}

pub(super) fn is_rtsp_unreserved_char(i: u8) -> bool {
    // rtsp_unreserved
    is_alphanumeric(i) || b"$-_.+!*'()".contains(&i)
}

pub(super) fn rtsp_unreserved(input: &[u8]) -> IResult<&[u8], &[u8]> {
    take_while(is_rtsp_unreserved_char)(input)
}

pub(super) fn quoted_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
    use std::num::NonZeroUsize;

    if !input.starts_with(b"\"") {
        return Err(Err::Error(nom::error::Error::new(
            input,
            nom::error::ErrorKind::Tag,
        )));
    }

    let i = &input[1..];
    let mut o = i;
    let mut closed = false;

    while !o.is_empty() {
        if o.len() >= 2 && o.starts_with(b"\\") {
            o = &o[2..];
        } else if o.starts_with(b"\\") {
            return Err(Err::Incomplete(Needed::Size(NonZeroUsize::new(1).unwrap())));
        } else if !o.starts_with(b"\"") {
            o = &o[1..];
        } else {
            // Closing quote, also include it
            o = &o[1..];
            closed = true;
            break;
        }
    }

    let (fst, snd) = input.split_at(input.len() - o.len());
    // The input did not contain the closing quote, e.g. it ended in an escaped
    // quote.
    if !closed {
        return Err(Err::Incomplete(Needed::Size(NonZeroUsize::new(1).unwrap())));
    }

    // Must have the starting quote
    assert!(fst.starts_with(b"\""));

    Ok((snd, fst))
}

/// Splits `s` on `separator`, ignoring separators inside quoted strings.
pub(crate) fn split_unquoted(s: &str, separator: u8) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    let mut escaped = false;

    for (index, byte) in s.bytes().enumerate() {
        if in_quotes {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_quotes = false;
            }
        } else if byte == b'"' {
            in_quotes = true;
        } else if byte == separator {
            result.push(&s[start..index]);
            start = index + 1;
        }
    }

    result.push(&s[start..]);

    result
}

/// Parses a `quoted-string` value, stripping the surrounding quotes and
/// unescaping quoted pairs.
pub(crate) fn parse_quoted_string(s: &str) -> Result<String, HeaderParseError> {
    let bytes = s.as_bytes();

    if bytes.first() != Some(&b'"') {
        return Err(HeaderParseError);
    }

    let mut value = Vec::new();
    let mut escaped = false;
    let mut closed = false;

    for (index, byte) in bytes[1..].iter().enumerate() {
        if escaped {
            if *byte != b'"' && *byte != b'\\' {
                return Err(HeaderParseError);
            }
            value.push(*byte);
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == b'"' {
            if index + 1 != bytes.len() - 1 {
                return Err(HeaderParseError);
            }
            closed = true;
            break;
        } else if !(byte == &b' '
            || byte == &b'!'
            || (b'#'..=b'[').contains(byte)
            || (b']'..=b'~').contains(byte))
        {
            return Err(HeaderParseError);
        } else {
            value.push(*byte);
        }
    }

    if !closed || escaped {
        return Err(HeaderParseError);
    }

    String::from_utf8(value).map_err(|_| HeaderParseError)
}

/// Writes `value` as a quoted string to `f`, escaping quotes and
/// backslashes.
pub(crate) fn write_quoted_string(f: &mut String, value: &str) {
    f.push('"');
    for char in value.chars() {
        if char == '"' || char == '\\' {
            f.push('\\');
        }
        f.push(char);
    }
    f.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_trims_tabs() {
        fn value(i: &[u8]) -> IResult<&[u8], &[u8]> {
            nom::bytes::complete::take_while(|c: u8| c.is_ascii_alphabetic())(i)
        }

        let (rest, out) = trim(value)(b"\t \tab \t").unwrap();
        assert_eq!(out, b"ab");
        assert!(rest.is_empty());
    }
}
