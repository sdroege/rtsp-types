// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::features::*;
use super::parser_helpers::{join_list, split_list};
use super::*;

/// `Require` header ([RFC 7826 section 18.43](https://tools.ietf.org/html/rfc7826#section-18.43)).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Require(Vec<String>);

impl std::ops::Deref for Require {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Require {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Vec<String>> for Require {
    fn as_ref(&self) -> &Vec<String> {
        &self.0
    }
}

impl AsMut<Vec<String>> for Require {
    fn as_mut(&mut self) -> &mut Vec<String> {
        &mut self.0
    }
}

impl From<Vec<String>> for Require {
    fn from(v: Vec<String>) -> Self {
        Require(v)
    }
}

impl<'a> From<&'a [String]> for Require {
    fn from(v: &'a [String]) -> Self {
        Require(v.to_vec())
    }
}

impl<'a> From<&'a [&'a &str]> for Require {
    fn from(v: &'a [&'a &str]) -> Self {
        Require(v.iter().map(|s| String::from(**s)).collect())
    }
}

impl Require {
    /// Creates a new `Require` header builder.
    pub fn builder() -> RequireBuilder {
        RequireBuilder(Vec::new())
    }

    /// Check if the "play.basic" feature is required.
    ///
    /// See [RFC 7826 section 11.1](https://tools.ietf.org/html/rfc7826#section-11.1).
    pub fn contains_play_basic(&self) -> bool {
        self.0.iter().any(|f| f == PLAY_BASIC)
    }

    /// Check if the "play.scale" feature is required.
    ///
    /// See [RFC 7826 section 18.46](https://tools.ietf.org/html/rfc7826#section-18.46).
    pub fn contains_play_scale(&self) -> bool {
        self.0.iter().any(|f| f == PLAY_SCALE)
    }

    /// Check if the "play.speed" feature is required.
    ///
    /// See [RFC 7826 section 18.50](https://tools.ietf.org/html/rfc7826#section-18.50).
    pub fn contains_play_speed(&self) -> bool {
        self.0.iter().any(|f| f == PLAY_SPEED)
    }

    /// Check if the "setup.rtp.rtcp.mux" feature is required.
    ///
    /// See [RFC 7826 Appendix C.1.6.4](https://tools.ietf.org/html/rfc7826#appendix-C.1.6.4).
    pub fn contains_setup_rtp_rtcp_mux(&self) -> bool {
        self.0.iter().any(|f| f == SETUP_RTP_RTCP_MUX)
    }
}

/// Builder for the 'Require' header.
#[derive(Debug, Clone)]
pub struct RequireBuilder(Vec<String>);

impl RequireBuilder {
    /// Add the provided feature to the `Require` header.
    pub fn feature<S: Into<String>>(mut self, feature: S) -> Self {
        self.0.push(feature.into());
        self
    }

    /// Add the "play.basic" feature to the `Require` header.
    ///
    /// See [RFC 7826 section 11.1](https://tools.ietf.org/html/rfc7826#section-11.1).
    pub fn play_basic(self) -> Self {
        self.feature(PLAY_BASIC)
    }

    /// Add the "play.scale" feature to the `Require` header.
    ///
    /// See [RFC 7826 section 18.46](https://tools.ietf.org/html/rfc7826#section-18.46).
    pub fn play_scale(self) -> Self {
        self.feature(PLAY_SCALE)
    }

    /// Add the "play.speed" feature to the `Require` header.
    ///
    /// See [RFC 7826 section 18.50](https://tools.ietf.org/html/rfc7826#section-18.50).
    pub fn play_speed(self) -> Self {
        self.feature(PLAY_SPEED)
    }

    /// Add the "setup.rtp.rtcp.mux" feature to the `Require` header.
    ///
    /// See [RFC 7826 Appendix C.1.6.4](https://tools.ietf.org/html/rfc7826#appendix-C.1.6.4).
    pub fn setup_rtp_rtcp_mux(self) -> Self {
        self.feature(SETUP_RTP_RTCP_MUX)
    }

    /// Build the `Require` header.
    pub fn build(self) -> Require {
        Require(self.0)
    }
}

impl super::TypedHeader for Require {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&REQUIRE) {
            None => return Ok(None),
            Some(header) => header,
        };

        let require = split_list(header.as_str()).map(String::from).collect();

        Ok(Some(Require(require)))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        if self.0.is_empty() {
            // An empty list cannot be represented. Remove any existing
            // header instead of writing an empty header value.
            headers.remove(&REQUIRE);
            return;
        }

        headers.insert(
            REQUIRE,
            join_list(self.0.iter().map(|feature| feature.as_str())),
        );
    }
}

impl super::TypedAppendableHeader for Require {
    fn append_to(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        if self.0.is_empty() {
            return;
        }

        headers.append(
            REQUIRE,
            join_list(self.0.iter().map(|feature| feature.as_str())),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_headers_empty_elements() {
        let headers_strs = ["a,,b", ",a,b", "a,b,", " , a , b , "];

        let expected = vec!["a".to_string(), "b".to_string()];

        for s in headers_strs {
            let mut test_headers = Headers::new();
            test_headers.insert(REQUIRE, s);
            let result = Require::from_headers(&test_headers)
                .expect("should not error")
                .expect("should be Some");

            assert_eq!(&*result, &expected, "{s}");
        }
    }

    #[test]
    fn test_insert_into_empty_list() {
        let require = Require(Vec::new());

        let mut test_headers = Headers::new();
        test_headers.insert(REQUIRE, "implicit");
        require.insert_into(&mut test_headers);

        assert_eq!(test_headers.get(&REQUIRE), None);
    }

    #[test]
    fn test_partial_eq() {
        assert_eq!(
            Require::builder().play_basic().build(),
            Require::builder().play_basic().build()
        );
        assert_ne!(
            Require::builder().play_basic().build(),
            Require::builder().play_speed().build()
        );
    }

    #[test]
    fn test_append_to_empty_list() {
        let require = Require(Vec::new());

        let mut test_headers = Headers::new();
        test_headers.insert(REQUIRE, "implicit");
        require.append_to(&mut test_headers);

        assert_eq!(
            test_headers.get(&REQUIRE).expect("should be Some").as_str(),
            "implicit"
        );
    }
}
