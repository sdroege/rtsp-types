// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use nom::IResult;
use tinyvec::TinyVec;

// Copy of nom's many0! combinator specialized for TinyVec instead of Vec
pub fn many0_tinyvec<I, O, E, F, OI>(f: F) -> impl FnMut(I) -> IResult<I, TinyVec<O>, E>
where
    I: Clone + PartialEq,
    F: Fn(I) -> IResult<I, OI, E>,
    E: nom::error::ParseError<I>,
    O: tinyvec::Array<Item = OI> + Clone,
    OI: Clone + Default,
{
    nom::multi::fold_many0(f, TinyVec::new(), |mut acc, item| {
        acc.push(item);
        acc
    })
}
