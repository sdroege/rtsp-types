// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use nom::IResult;
use smallvec::SmallVec;

// Copy of nom's many0! combinator specialized for SmallVec instead of Vec
pub fn many0_smallvec<I, O, E, F, OI>(f: F) -> impl FnMut(I) -> IResult<I, SmallVec<O>, E>
where
    I: Clone + PartialEq,
    F: Fn(I) -> IResult<I, OI, E>,
    E: nom::error::ParseError<I>,
    O: smallvec::Array<Item = OI>,
    OI: Clone,
{
    nom::multi::fold_many0(f, SmallVec::new(), |mut acc, item| {
        acc.push(item);
        acc
    })
}
