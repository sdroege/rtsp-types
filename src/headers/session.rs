// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

/// `Session` header ([RFC 7826 section 18.49](https://tools.ietf.org/html/rfc7826#section-18.49)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Session(
    /// Session identifier.
    pub String,
    /// Optional session timeout in seconds.
    pub Option<u64>,
);

impl Session {
    pub fn with_timeout(id: String, timeout: u64) -> Self {
        Self(id, Some(timeout))
    }
}

impl std::ops::Deref for Session {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<str> for Session {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'a> From<&'a str> for Session {
    fn from(v: &'a str) -> Session {
        Session(v.into(), None)
    }
}

impl From<String> for Session {
    fn from(v: String) -> Session {
        Session(v, None)
    }
}

impl super::TypedHeader for Session {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&SESSION) {
            None => return Ok(None),
            Some(header) => header,
        };

        let mut iter = header.as_str().split(';');

        let session_id = iter.next().ok_or(HeaderParseError)?;
        let timeout = iter
            .next()
            .map(|s| s.parse::<u64>())
            .transpose()
            .map_err(|_| HeaderParseError)?;

        Ok(Some(Session(session_id.into(), timeout)))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        if let Some(timeout) = self.1 {
            headers.insert(SESSION, format!("{};timeout={}", self.0, timeout));
        } else {
            headers.insert(SESSION, self.0.to_string());
        }
    }
}
