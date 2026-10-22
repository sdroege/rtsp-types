// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::parser_helpers::parse_digits;
use super::*;

/// `Content-Length` header ([RFC 7826 section 18.17](https://tools.ietf.org/html/rfc7826#section-18.17)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContentLength(u64);

impl std::ops::Deref for ContentLength {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for ContentLength {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<u64> for ContentLength {
    fn as_ref(&self) -> &u64 {
        &self.0
    }
}

impl AsMut<u64> for ContentLength {
    fn as_mut(&mut self) -> &mut u64 {
        &mut self.0
    }
}

impl From<u64> for ContentLength {
    fn from(v: u64) -> ContentLength {
        ContentLength(v)
    }
}

impl From<ContentLength> for u64 {
    fn from(v: ContentLength) -> u64 {
        v.0
    }
}

impl super::TypedHeader for ContentLength {
    fn header_name() -> Option<HeaderName> {
        Some(CONTENT_LENGTH.clone())
    }
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&CONTENT_LENGTH) {
            None => return Ok(None),
            Some(header) => header,
        };

        let length = parse_digits::<u64>(header.as_str())
            .map(ContentLength)
            .map_err(|_| HeaderParseError::new())?;

        Ok(Some(length))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        headers.insert(CONTENT_LENGTH, self.0.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_headers() {
        let valid = ["0", "1", "9999999999999999999", "18446744073709551615"];
        for s in valid {
            let mut test_headers = Headers::new();
            test_headers.insert(CONTENT_LENGTH, s);
            let result = ContentLength::from_headers(&test_headers)
                .expect("should not error")
                .expect("should be Some");
            assert_eq!(*result, s.parse::<u64>().unwrap(), "{}", s);
        }

        let invalid = [
            "+1",
            "-1",
            "1e2",
            "",
            " 1",
            "1 ",
            "18446744073709551616",
            "0x1",
        ];
        for s in invalid {
            let mut test_headers = Headers::new();
            test_headers.insert(CONTENT_LENGTH, s);
            assert!(ContentLength::from_headers(&test_headers).is_err(), "{}", s);
        }
    }
}
