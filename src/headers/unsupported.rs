// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::features::*;
use super::*;

/// `Unsupported` header ([RFC 7826 section 18.55](https://tools.ietf.org/html/rfc7826#section-18.55)).
#[derive(Debug, Clone)]
pub struct Unsupported(Vec<String>);

impl std::ops::Deref for Unsupported {
    type Target = Vec<String>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Unsupported {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Vec<String>> for Unsupported {
    fn as_ref(&self) -> &Vec<String> {
        &self.0
    }
}

impl AsMut<Vec<String>> for Unsupported {
    fn as_mut(&mut self) -> &mut Vec<String> {
        &mut self.0
    }
}

impl From<Vec<String>> for Unsupported {
    fn from(v: Vec<String>) -> Self {
        Unsupported(v)
    }
}

impl<'a> From<&'a [String]> for Unsupported {
    fn from(v: &'a [String]) -> Self {
        Unsupported(v.to_vec())
    }
}

impl<'a> From<&'a [&'a &str]> for Unsupported {
    fn from(v: &'a [&'a &str]) -> Self {
        Unsupported(v.iter().map(|s| String::from(**s)).collect())
    }
}

impl Unsupported {
    /// Creates a new `Unsupported` header builder.
    pub fn builder() -> UnsupportedBuilder {
        UnsupportedBuilder(Vec::new())
    }

    /// Check if the "play.basic" feature is unsupported.
    ///
    /// See [RFC 7826 section 11.1](https://tools.ietf.org/html/rfc7826#section-11.1).
    pub fn contains_play_basic(&self) -> bool {
        self.0.iter().any(|f| f == PLAY_BASIC)
    }

    /// Check if the "play.scale" feature is unsupported.
    ///
    /// See [RFC 7826 section 18.46](https://tools.ietf.org/html/rfc7826#section-18.46).
    pub fn contains_play_scale(&self) -> bool {
        self.0.iter().any(|f| f == PLAY_SCALE)
    }

    /// Check if the "play.speed" feature is unsupported.
    ///
    /// See [RFC 7826 section 18.50](https://tools.ietf.org/html/rfc7826#section-18.50).
    pub fn contains_play_speed(&self) -> bool {
        self.0.iter().any(|f| f == PLAY_SPEED)
    }

    /// Check if the "setup.rtp.rtcp.mux" feature is unsupported.
    ///
    /// See [RFC 7826 Appendix C.1.6.4](https://tools.ietf.org/html/rfc7826#appendix-C.1.6.4).
    pub fn contains_setup_rtp_rtcp_mux(&self) -> bool {
        self.0.iter().any(|f| f == SETUP_RTP_RTCP_MUX)
    }
}

/// Builder for the 'Unsupported' header.
#[derive(Debug, Clone)]
pub struct UnsupportedBuilder(Vec<String>);

impl UnsupportedBuilder {
    /// Add the provided feature to the `Unsupported` header.
    pub fn feature<S: Into<String>>(mut self, feature: S) -> Self {
        self.0.push(feature.into());
        self
    }

    /// Add the "play.basic" feature to the `Unsupported` header.
    ///
    /// See [RFC 7826 section 11.1](https://tools.ietf.org/html/rfc7826#section-11.1).
    pub fn play_basic(self) -> Self {
        self.feature(PLAY_BASIC)
    }

    /// Add the "play.scale" feature to the `Unsupported` header.
    ///
    /// See [RFC 7826 section 18.46](https://tools.ietf.org/html/rfc7826#section-18.46).
    pub fn play_scale(self) -> Self {
        self.feature(PLAY_SCALE)
    }

    /// Add the "play.speed" feature to the `Unsupported` header.
    ///
    /// See [RFC 7826 section 18.50](https://tools.ietf.org/html/rfc7826#section-18.50).
    pub fn play_speed(self) -> Self {
        self.feature(PLAY_SPEED)
    }

    /// Add the "setup.rtp.rtcp.mux" feature to the `Unsupported` header.
    ///
    /// See [RFC 7826 Appendix C.1.6.4](https://tools.ietf.org/html/rfc7826#appendix-C.1.6.4).
    pub fn setup_rtp_rtcp_mux(self) -> Self {
        self.feature(SETUP_RTP_RTCP_MUX)
    }

    /// Build the `Unsupported` header.
    pub fn build(self) -> Unsupported {
        Unsupported(self.0)
    }
}

impl super::TypedHeader for Unsupported {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&UNSUPPORTED) {
            None => return Ok(None),
            Some(header) => header,
        };

        let mut unsupported = Vec::new();
        for feature in header.as_str().split(',') {
            let feature = feature.trim();

            unsupported.push(feature.into());
        }

        Ok(Some(Unsupported(unsupported)))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        let mut unsupported = String::new();
        for feature in &self.0 {
            if !unsupported.is_empty() {
                unsupported.push_str(", ");
            }

            unsupported.push_str(feature);
        }

        headers.insert(UNSUPPORTED, unsupported);
    }
}

impl super::TypedAppendableHeader for Unsupported {
    fn append_to(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        let mut unsupported = String::new();
        for feature in &self.0 {
            if !unsupported.is_empty() {
                unsupported.push_str(", ");
            }

            unsupported.push_str(feature);
        }

        headers.append(UNSUPPORTED, unsupported);
    }
}
