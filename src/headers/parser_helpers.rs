// Copyright (C) 2021 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use nom::bytes::complete::take_while;
use nom::character::complete::space0;
use nom::character::is_alphanumeric;
use nom::{Err, IResult, Needed};
use std::str;

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

pub(super) fn token(input: &[u8]) -> IResult<&[u8], &[u8]> {
    pub(super) fn is_token_char(i: u8) -> bool {
        is_alphanumeric(i) || b"!#$%&'*+-.^_`|~".contains(&i)
    }

    take_while(is_token_char)(input)
}

pub(super) fn rtsp_unreserved(input: &[u8]) -> IResult<&[u8], &[u8]> {
    pub(super) fn is_rtsp_unreserved_char(i: u8) -> bool {
        // rtsp_unreserved
        is_alphanumeric(i) || b"$-_.+!*'()".contains(&i)
    }

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
            break;
        }
    }

    let (fst, snd) = input.split_at(input.len() - o.len());
    // Did not end with a quote
    if !fst.ends_with(b"\"") {
        return Err(Err::Incomplete(Needed::Size(NonZeroUsize::new(1).unwrap())));
    }

    // Must have the starting quote
    assert!(fst.starts_with(b"\""));

    Ok((snd, fst))
}

// FIXME: Remove once str::split_once is stabilized
pub(super) fn split_once(s: &str, d: char) -> Option<(&str, &str)> {
    let idx = s.find(d)?;
    let (fst, snd) = s.split_at(idx);

    let (_, snd) = snd.split_at(snd.char_indices().nth(1).map(|(idx, _c)| idx).unwrap_or(1));

    Some((fst, snd))
}
