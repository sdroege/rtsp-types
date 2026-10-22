// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::parser_helpers::parse_float;
use super::*;

/// `Speed` header ([RFC 7826 section 18.50](https://tools.ietf.org/html/rfc7826#section-18.50)).
///
/// The header describes a range of speeds, all speeds between
/// `lower_bound` and `upper_bound` are acceptable.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Speed {
    /// The lowest acceptable speed, positive.
    pub lower_bound: f64,
    /// The highest acceptable speed, positive.
    pub upper_bound: f64,
}

impl super::TypedHeader for Speed {
    fn header_name() -> Option<HeaderName> {
        Some(SPEED.clone())
    }
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&SPEED) {
            None => return Ok(None),
            Some(header) => header,
        };

        let (lower, upper) = header
            .as_str()
            .split_once('-')
            .ok_or_else(HeaderParseError::new)?;

        let lower_bound = parse_float(lower.trim(), false).map_err(|_| HeaderParseError::new())?;
        let upper_bound = parse_float(upper.trim(), false).map_err(|_| HeaderParseError::new())?;

        // A speed value of zero is invalid.
        if lower_bound == 0.0 || upper_bound == 0.0 {
            return Err(HeaderParseError::new());
        }

        Ok(Some(Speed {
            lower_bound,
            upper_bound,
        }))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        headers.insert(SPEED, format!("{}-{}", self.lower_bound, self.upper_bound));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speed() {
        let headers = [
            ("1-2", 1.0, 2.0, "1-2"),
            ("1.5-2.5", 1.5, 2.5, "1.5-2.5"),
            ("0.5 - 1.5", 0.5, 1.5, "0.5-1.5"),
            ("2-2", 2.0, 2.0, "2-2"),
        ];

        for (header, lower_bound, upper_bound, serialized) in &headers {
            let request = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .header(crate::headers::SPEED, *header)
                .empty();

            let speed = request
                .typed_header::<Speed>()
                .unwrap_or_else(|_| panic!("couldn't parse {}", header))
                .unwrap();

            assert_eq!(speed.lower_bound, *lower_bound, "{}", header);
            assert_eq!(speed.upper_bound, *upper_bound, "{}", header);

            let request2 = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .typed_header(&speed)
                .empty();

            let speed = request2.header(&crate::headers::SPEED).unwrap();

            assert_eq!(speed, *serialized, "{}", header);
        }
    }

    #[test]
    fn test_speed_invalid() {
        let headers = [
            "", "-", "1", "1-", "-2", "1-2-3", "1.--2", "inf-NaN", "1e3-2", "-1-2", "0-1", "1-0",
            "0.0-0.0",
        ];

        for header in &headers {
            let request = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .header(crate::headers::SPEED, *header)
                .empty();

            assert!(request.typed_header::<Speed>().is_err(), "{}", header);
        }
    }
}
