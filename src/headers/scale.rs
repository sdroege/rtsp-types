// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

/// `Scale` header ([RFC 7826 section 18.46](https://tools.ietf.org/html/rfc7826#section-18.46)).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
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

        let scale = header
            .as_str()
            .parse::<f64>()
            .map(Scale)
            .map_err(|_| HeaderParseError)?;

        Ok(Some(scale))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        headers.insert(SCALE, self.0.to_string());
    }
}
