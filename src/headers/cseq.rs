// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

/// `CSeq` header ([RFC 7826 section 18.20](https://tools.ietf.org/html/rfc7826#section-18.20)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CSeq(u128);

impl std::ops::Deref for CSeq {
    type Target = u128;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for CSeq {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<u128> for CSeq {
    fn as_ref(&self) -> &u128 {
        &self.0
    }
}

impl AsMut<u128> for CSeq {
    fn as_mut(&mut self) -> &mut u128 {
        &mut self.0
    }
}

impl From<u128> for CSeq {
    fn from(v: u128) -> CSeq {
        CSeq(v)
    }
}

impl From<CSeq> for u128 {
    fn from(v: CSeq) -> u128 {
        v.0
    }
}

impl CSeq {
    /// Parses the `CSeq` header from headers.
    pub fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<CSeq>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&CSEQ) {
            None => return Ok(None),
            Some(header) => header,
        };

        let cseq = header
            .as_str()
            .parse::<u128>()
            .map(CSeq)
            .map_err(|_| HeaderParseError)?;

        Ok(Some(cseq))
    }

    /// Inserts the `CSeq` header into headers, possibly replacing an existing `CSeq` header.
    pub fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        headers.insert(CSEQ, self.0.to_string());
    }
}
