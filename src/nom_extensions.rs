// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use nom::error::{ErrorKind, ParseError};
use nom::{Err, IResult, Parser};
use tinyvec::TinyVec;

// Copy of nom's many0 combinator specialized for TinyVec instead of Vec
pub fn many0_tinyvec<I, O, E, F, OI>(f: F) -> impl FnMut(I) -> IResult<I, TinyVec<O>, E>
where
    I: Clone + PartialEq + nom::InputLength,
    F: Fn(I) -> IResult<I, OI, E>,
    E: nom::error::ParseError<I>,
    O: tinyvec::Array<Item = OI> + Clone,
    OI: Clone + Default,
{
    nom::multi::fold_many0(f, TinyVec::new, |mut acc, item| {
        acc.push(item);
        acc
    })
}

// Copy of nom's separated_list0 converted into a fold function
#[allow(dead_code)]
pub fn separated_list0_fold<I, O, O2, E, F, G, H, R>(
    mut sep: G,
    mut f: F,
    init: R,
    mut h: H,
) -> impl FnMut(I) -> IResult<I, R, E>
where
    I: Clone + PartialEq,
    F: Parser<I, O, E>,
    G: Parser<I, O2, E>,
    E: ParseError<I>,
    R: Clone,
    H: FnMut(R, O) -> R,
{
    move |mut i: I| {
        let mut res = init.clone();

        match f.parse(i.clone()) {
            Err(Err::Error(_)) => return Ok((i, res)),
            Err(e) => return Err(e),
            Ok((i1, o)) => {
                res = h(res, o);
                i = i1;
            }
        }

        loop {
            match sep.parse(i.clone()) {
                Err(Err::Error(_)) => return Ok((i, res)),
                Err(e) => return Err(e),
                Ok((i1, _)) => {
                    if i1 == i {
                        return Err(Err::Error(E::from_error_kind(i1, ErrorKind::SeparatedList)));
                    }

                    match f.parse(i1.clone()) {
                        Err(Err::Error(_)) => return Ok((i, res)),
                        Err(e) => return Err(e),
                        Ok((i2, o)) => {
                            res = h(res, o);
                            i = i2;
                        }
                    }
                }
            }
        }
    }
}

// Copy of nom's separated_list1 converted into a fold function
pub fn separated_list1_fold<I, O, O2, E, F, G, H, R>(
    mut sep: G,
    mut f: F,
    init: R,
    mut h: H,
) -> impl FnMut(I) -> IResult<I, R, E>
where
    I: Clone + PartialEq,
    F: Parser<I, O, E>,
    G: Parser<I, O2, E>,
    E: ParseError<I>,
    R: Clone,
    H: FnMut(R, O) -> R,
{
    move |mut i: I| {
        let mut res = init.clone();

        // Parse the first element
        match f.parse(i.clone()) {
            Err(e) => return Err(e),
            Ok((i1, o)) => {
                res = h(res, o);
                i = i1;
            }
        }

        loop {
            match sep.parse(i.clone()) {
                Err(Err::Error(_)) => return Ok((i, res)),
                Err(e) => return Err(e),
                Ok((i1, _)) => {
                    if i1 == i {
                        return Err(Err::Error(E::from_error_kind(i1, ErrorKind::SeparatedList)));
                    }

                    match f.parse(i1.clone()) {
                        Err(Err::Error(_)) => return Ok((i, res)),
                        Err(e) => return Err(e),
                        Ok((i2, o)) => {
                            res = h(res, o);
                            i = i2;
                        }
                    }
                }
            }
        }
    }
}
