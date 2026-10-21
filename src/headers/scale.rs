// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::parser_helpers::parse_float;
use super::*;

/// `Scale` header ([RFC 7826 section 18.46](https://tools.ietf.org/html/rfc7826#section-18.46)).
///
/// A scale of 0 is equivalent to pause and not allowed.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Scale(f64);

impl std::ops::Deref for Scale {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Scale {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<f64> for Scale {
    fn as_ref(&self) -> &f64 {
        &self.0
    }
}

impl AsMut<f64> for Scale {
    fn as_mut(&mut self) -> &mut f64 {
        &mut self.0
    }
}

impl From<f64> for Scale {
    fn from(v: f64) -> Scale {
        Scale(v)
    }
}

impl From<Scale> for f64 {
    fn from(v: Scale) -> f64 {
        v.0
    }
}

impl super::TypedHeader for Scale {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&SCALE) {
            None => return Ok(None),
            Some(header) => header,
        };

        let scale = parse_float(header.as_str(), true).map_err(|_| HeaderParseError)?;

        // A scale of 0 is equivalent to pause and not allowed.
        if scale == 0.0 {
            return Err(HeaderParseError);
        }

        Ok(Some(Scale(scale)))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        headers.insert(SCALE, self.0.to_string());
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_scale() {
        let headers = [("1", 1.0), ("0.5", 0.5), ("-2", -2.0), ("1.25", 1.25)];

        for (header, expected) in &headers {
            let request = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .header(crate::headers::SCALE, *header)
                .empty();

            let scale = request
                .typed_header::<super::Scale>()
                .unwrap_or_else(|_| panic!("couldn't parse {}", header))
                .unwrap();

            assert_eq!(*scale, *expected, "{}", header);

            let request2 = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .typed_header(&scale)
                .empty();

            let scale = request2.header(&crate::headers::SCALE).unwrap();

            assert_eq!(scale, *header, "{}", header);
        }
    }

    #[test]
    fn test_scale_invalid() {
        let headers = [
            "", "inf", "-inf", "NaN", "1e5", "1E-3", "+1", "1.", ".5", "1.2.3", "--1", "-", "0",
            "0.0", "-0.0", "0.000",
        ];

        for header in &headers {
            let request = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .header(crate::headers::SCALE, *header)
                .empty();

            assert!(
                request.typed_header::<super::Scale>().is_err(),
                "{}",
                header
            );
        }
    }
}
