// Copyright (C) 2026 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

//! Error type of the message parser.
//!
//! `winnow` parser errors only carry the position of the failure, not what
//! was expected there. This error additionally remembers the innermost
//! [`context`](winnow::Parser::context) label, which is used to describe the
//! expected input in [`ParseSyntaxError`](crate::ParseSyntaxError).

use winnow::error::{AddContext, ErrMode, FromExternalError, ParserError};
use winnow::stream::{Partial, Stream};

/// Input of the message parser: a possibly incomplete slice of bytes.
pub(crate) type Input<'a> = Partial<&'a [u8]>;

/// Error of the message parser.
pub(crate) type Error<'a> = ErrMode<MsgError<'a>>;

/// Result of the message parser.
pub(crate) type Result<'a, O> = core::result::Result<O, Error<'a>>;

/// Input is a literal.
pub(crate) const EXPECTED_LITERAL: &str = "literal";
/// Input is a single character.
pub(crate) const EXPECTED_CHARACTER: &str = "character";
/// Input is one of a fixed set of characters.
pub(crate) const EXPECTED_ONE_OF: &str = "one of a set of characters";
/// Input is a non-empty run of characters.
pub(crate) const EXPECTED_NON_EMPTY: &str = "non-empty matching input";
/// Input parsed into a value, e.g. a number or a UTF-8 string.
pub(crate) const EXPECTED_VALUE: &str = "valid value";
/// Nothing more specific is known about the expected input.
pub(crate) const EXPECTED_INPUT: &str = "input";

/// Error of the message parser, pointing at the position where parsing
/// stopped and describing what was expected there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MsgError<'a> {
    input: &'a [u8],
    expected: Option<&'static str>,
}

impl<'a> MsgError<'a> {
    /// Error for a value that was parsed but turned out to be invalid.
    pub fn at(input: &'a [u8], expected: &'static str) -> Self {
        MsgError {
            input,
            expected: Some(expected),
        }
    }

    /// The input at the position where parsing stopped.
    pub fn input(&self) -> &'a [u8] {
        self.input
    }

    /// Description of the expected input.
    pub fn expected(&self) -> &'static str {
        self.expected.unwrap_or(EXPECTED_INPUT)
    }
}

impl<'a> ParserError<Input<'a>> for MsgError<'a> {
    type Inner = Self;

    fn from_input(input: &Input<'a>) -> Self {
        MsgError {
            input: input.into_inner(),
            expected: None,
        }
    }

    fn into_inner(self) -> core::result::Result<Self::Inner, Self> {
        Ok(self)
    }
}

impl<'a> AddContext<Input<'a>, &'static str> for MsgError<'a> {
    fn add_context(
        mut self,
        _input: &Input<'a>,
        _token_start: &<Input<'a> as Stream>::Checkpoint,
        expected: &'static str,
    ) -> Self {
        // The innermost label describes what the parser was looking for at the
        // position of the error, outer labels are only less specific fall-backs.
        if self.expected.is_none() {
            self.expected = Some(expected);
        }
        self
    }
}

impl<'a, E> FromExternalError<Input<'a>, E> for MsgError<'a> {
    fn from_external_error(input: &Input<'a>, _error: E) -> Self {
        MsgError {
            input: input.into_inner(),
            expected: None,
        }
    }
}
