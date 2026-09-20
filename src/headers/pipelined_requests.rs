// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::parser_helpers::parse_digits;
use super::*;

/// `Pipelined-Requests` header ([RFC 7826 section 18.33](https://tools.ietf.org/html/rfc7826#section-18.33)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PipelinedRequests(u32);

impl std::ops::Deref for PipelinedRequests {
    type Target = u32;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PipelinedRequests {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<u32> for PipelinedRequests {
    fn as_ref(&self) -> &u32 {
        &self.0
    }
}

impl AsMut<u32> for PipelinedRequests {
    fn as_mut(&mut self) -> &mut u32 {
        &mut self.0
    }
}

impl From<u32> for PipelinedRequests {
    fn from(v: u32) -> PipelinedRequests {
        PipelinedRequests(v)
    }
}

impl From<PipelinedRequests> for u32 {
    fn from(v: PipelinedRequests) -> u32 {
        v.0
    }
}

impl super::TypedHeader for PipelinedRequests {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&PIPELINED_REQUESTS) {
            None => return Ok(None),
            Some(header) => header,
        };

        let cseq = parse_digits::<u32>(header.as_str())
            .map(PipelinedRequests)
            .map_err(|_| HeaderParseError)?;

        Ok(Some(cseq))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        headers.insert(PIPELINED_REQUESTS, self.0.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_headers() {
        let valid = ["0", "1", "99999999", "4294967295"];
        for s in valid {
            let mut test_headers = Headers::new();
            test_headers.insert(PIPELINED_REQUESTS, s);
            let result = PipelinedRequests::from_headers(&test_headers)
                .expect("should not error")
                .expect("should be Some");
            assert_eq!(*result, s.parse::<u32>().unwrap(), "{}", s);
        }

        let invalid = ["+1", "-1", "1e2", "", " 1", "1 ", "4294967296", "0x1"];
        for s in invalid {
            let mut test_headers = Headers::new();
            test_headers.insert(PIPELINED_REQUESTS, s);
            assert!(
                PipelinedRequests::from_headers(&test_headers).is_err(),
                "{}",
                s
            );
        }
    }
}
