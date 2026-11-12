// Copyright (C) 2021 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use std::str;
use winnow::ascii::space0;
use winnow::combinator::delimited;
use winnow::error::{ContextError, ErrMode};
use winnow::prelude::*;
use winnow::stream::{AsChar, Stream, StreamIsPartial};
use winnow::token::take_while;
use winnow::ModalResult;
use winnow::Parser;

use super::HeaderParseError;

pub(super) fn backtrack(input: &[u8]) -> ErrMode<ContextError> {
    ErrMode::Backtrack(ContextError::from_input(&input))
}

pub(super) fn cond_parser<I, O1, O2, F, G>(
    mut cond: F,
    mut parser: G,
) -> impl FnMut(&mut I) -> ModalResult<Option<O2>>
where
    F: Parser<I, O1, ErrMode<ContextError>>,
    G: Parser<I, O2, ErrMode<ContextError>>,
    I: Stream,
{
    move |input: &mut I| {
        let start = input.checkpoint();

        match cond.parse_next(input) {
            Ok(_) => parser.parse_next(input).map(Some),
            Err(ErrMode::Backtrack(_)) => {
                input.reset(&start);
                Ok(None)
            }
            Err(err) => Err(err),
        }
    }
}

pub(super) fn trim<I, O, F>(parser: F) -> impl Parser<I, O, ErrMode<ContextError>>
where
    F: Parser<I, O, ErrMode<ContextError>>,
    I: StreamIsPartial + Stream,
    <I as Stream>::Token: AsChar,
{
    delimited(space0, parser, space0)
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
    i.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&i)
}

pub(super) fn token<'i>(input: &mut &'i [u8]) -> ModalResult<&'i [u8]> {
    take_while(0.., is_token_char).parse_next(input)
}

pub(super) fn is_rtsp_unreserved_char(i: u8) -> bool {
    // rtsp_unreserved
    i.is_ascii_alphanumeric() || b"$-_.+!*'()".contains(&i)
}

pub(super) fn rtsp_unreserved<'i>(input: &mut &'i [u8]) -> ModalResult<&'i [u8]> {
    take_while(0.., is_rtsp_unreserved_char).parse_next(input)
}

pub(super) fn quoted_string<'i>(input: &mut &'i [u8]) -> ModalResult<&'i [u8]> {
    if !input.starts_with(b"\"") {
        return Err(backtrack(input));
    }

    let i = &input[1..];
    let mut o = i;
    let mut closed = false;

    while !o.is_empty() {
        if o.len() >= 2 && o.starts_with(b"\\") {
            o = &o[2..];
        } else if o.starts_with(b"\\") {
            // Trailing backslash, the string can never be closed here.
            return Err(backtrack(input));
        } else if !o.starts_with(b"\"") {
            o = &o[1..];
        } else {
            // Closing quote, also include it
            o = &o[1..];
            closed = true;
            break;
        }
    }

    // The input did not contain the closing quote, e.g. it ended in an escaped
    // quote. Header parsing operates on complete input, so this is just an
    // error and not a request for more data.
    if !closed {
        return Err(backtrack(input));
    }

    let (fst, snd) = input.split_at(input.len() - o.len());
    // Must have the starting quote
    assert!(fst.starts_with(b"\""));

    *input = snd;

    Ok(fst)
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
        return Err(HeaderParseError::new());
    }

    let mut value = Vec::new();
    let mut escaped = false;
    let mut closed = false;

    for (index, byte) in bytes[1..].iter().enumerate() {
        if escaped {
            if *byte != b'"' && *byte != b'\\' {
                return Err(HeaderParseError::new());
            }
            value.push(*byte);
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == b'"' {
            if index + 1 != bytes.len() - 1 {
                return Err(HeaderParseError::new());
            }
            closed = true;
            break;
        } else if !(byte == &b' '
            || byte == &b'!'
            || (b'#'..=b'[').contains(byte)
            || (b']'..=b'~').contains(byte))
        {
            return Err(HeaderParseError::new());
        } else {
            value.push(*byte);
        }
    }

    if !closed || escaped {
        return Err(HeaderParseError::new());
    }

    String::from_utf8(value).map_err(|_| HeaderParseError::new())
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

/// Splits a comma-separated list into trimmed items, skipping empty
/// elements.
pub(crate) fn split_list(value: &str) -> impl Iterator<Item = &str> {
    value
        .split(',')
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
}

/// Joins the given items into a comma-separated list.
pub(crate) fn join_list<'a>(items: impl IntoIterator<Item = &'a str>) -> String {
    items.into_iter().collect::<Vec<_>>().join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_trims_tabs() {
        fn value<'i>(input: &mut &'i [u8]) -> ModalResult<&'i [u8]> {
            take_while(1.., |c: u8| c.is_ascii_alphabetic()).parse_next(input)
        }

        let mut input: &[u8] = b"\t \tab \t";
        let out = trim(value).parse_next(&mut input).unwrap();
        assert_eq!(out, b"ab");
        assert!(input.is_empty());
    }

    #[test]
    fn list_helpers() {
        let items: Vec<&str> = split_list(" a ,,b ,\t").collect();
        assert_eq!(items, vec!["a", "b"]);
        assert_eq!(join_list(items), "a, b");
        assert_eq!(join_list(std::iter::empty::<&str>()), "");
    }
}
