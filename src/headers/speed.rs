// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::parser_helpers::parse_float;
use super::*;

/// `Speed` header ([RFC 7826 section 18.50](https://tools.ietf.org/html/rfc7826#section-18.50)).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Speed(f64);

impl std::ops::Deref for Speed {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Speed {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<f64> for Speed {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl AsMut<f64> for Speed {
    fn as_mut(&mut self) -> &mut f64 {
        &mut self.0
    }
}

impl From<f64> for Speed {
    fn from(v: f64) -> Speed {
        Speed(v)
    }
}

impl From<Speed> for f64 {
    fn from(v: Speed) -> f64 {
        v.0
    }
}

impl super::TypedHeader for Speed {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&SPEED) {
            None => return Ok(None),
            Some(header) => header,
        };

        let speed = parse_float(header.as_str(), false)
            .map(Speed)
            .map_err(|_| HeaderParseError)?;

        Ok(Some(speed))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        headers.insert(SPEED, self.0.to_string());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_speed() {
        let headers = [("1", 1.0), ("1.5", 1.5), ("0", 0.0)];

        for (header, expected) in &headers {
            let request = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .header(crate::headers::SPEED, *header)
                .empty();

            let speed = request
                .typed_header::<super::Speed>()
                .unwrap_or_else(|_| panic!("couldn't parse {}", header))
                .unwrap();

            assert_eq!(*speed, *expected, "{}", header);

            let request2 = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .typed_header(&speed)
                .empty();

            let speed = request2.header(&crate::headers::SPEED).unwrap();

            assert_eq!(speed, *header, "{}", header);
        }
    }

    #[test]
    fn test_speed_invalid() {
        let headers = ["", "inf", "NaN", "1e3", "+1", "-1", "-", "1."];

        for header in &headers {
            let request = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .header(crate::headers::SPEED, *header)
                .empty();

            assert!(
                request.typed_header::<super::Speed>().is_err(),
                "{}",
                header
            );
        }
    }
}
