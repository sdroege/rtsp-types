// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

/// `Speed` header ([RFC 7826 section 18.50](https://tools.ietf.org/html/rfc7826#section-18.50)).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
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

        let speed = header
            .as_str()
            .parse::<f64>()
            .map(Speed)
            .map_err(|_| HeaderParseError)?;

        Ok(Some(speed))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        headers.insert(SPEED, self.0.to_string());
    }
}
