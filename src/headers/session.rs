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
            .find_map(|s| s.strip_prefix("timeout="))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_headers() {
        let strict_headers = [
            ("12345678", Some(Session("12345678".to_string(), None))),
            (
                "12345678;timeout=60",
                Some(Session("12345678".to_string(), Some(60))),
            ),
            (
                "lskdjf238742dkjlskjd;timeout=60",
                Some(Session("lskdjf238742dkjlskjd".to_string(), Some(60))),
            ),
            (
                "alskdjalskjdalskjdalksjd;timeout=60",
                Some(Session("alskdjalskjdalskjdalksjd".to_string(), Some(60))),
            ),
        ];

        let loose_headers = [
            (
                "12345678;timeout=60;special",
                Some(Session("12345678".to_string(), Some(60))),
            ),
            (
                "12345678;timeout=60;393939393",
                Some(Session("12345678".to_string(), Some(60))),
            ),
            (
                "12345678;timeout=60;393;93;93;93",
                Some(Session("12345678".to_string(), Some(60))),
            ),
            (
                "12345678;special;timeout=600",
                Some(Session("12345678".to_string(), Some(600))),
            ),
            (
                "12345678;extra;extra;extra;timeout=600",
                Some(Session("12345678".to_string(), Some(600))),
            ),
            (
                "wjdl38ek98;timeout=60;special",
                Some(Session("wjdl38ek98".to_string(), Some(60))),
            ),
            (
                "wjdl38ek98;timeout=60;393939393",
                Some(Session("wjdl38ek98".to_string(), Some(60))),
            ),
            (
                "wjdl38ek98;timeout=60;393;93;93;93",
                Some(Session("wjdl38ek98".to_string(), Some(60))),
            ),
            (
                "wjdl38ek98;special;timeout=600",
                Some(Session("wjdl38ek98".to_string(), Some(600))),
            ),
            (
                "wjdl38ek98;extra;extra;extra;timeout=600",
                Some(Session("wjdl38ek98".to_string(), Some(600))),
            ),
        ];

        let bad_headers = [
            "12345678;timeout=aa",
            "12345678;timeout=a6a",
            "12345678;timeout=!!!",
            "12345678;timeout=18446744073709551616",
            "12345678;timeout=18446744073709551616;special",
            "12345678;special;timeout=18446744073709551616",
            "12345678;timeout=-1",
            "12345678;timeout=-2",
        ];

        let not_session_headers = [(AUTHORIZATION, "blah"), (ACCEPT, "application/sdp")];

        for (header, expected) in strict_headers {
            let mut test_headers = Headers::new();
            test_headers.insert(SESSION, header);
            let from_headers_result =
                Session::from_headers(test_headers).expect("strict_headers should not error");

            assert_eq!(from_headers_result, expected, "{}", header);
        }

        for (header, expected) in loose_headers {
            let mut test_headers = Headers::new();
            test_headers.insert(SESSION, header);
            let from_headers_result =
                Session::from_headers(test_headers).expect("loose_errors should not error");

            assert_eq!(from_headers_result, expected, "{}", header);
        }

        for header in bad_headers {
            let mut test_headers = Headers::new();
            test_headers.insert(SESSION, header);

            Session::from_headers(test_headers)
                .expect_err("bad_headers should all give HeaderParserErrors");
        }

        for (header, value) in not_session_headers {
            let mut test_headers = Headers::new();
            test_headers.insert(header.clone(), value);
            let from_headers_result =
                Session::from_headers(test_headers).expect("not_session_headers should not error");

            assert_eq!(from_headers_result, None, "{}:{}", header, value);
        }
    }
}
