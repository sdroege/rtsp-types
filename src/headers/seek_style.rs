// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use std::fmt;

/// `Seek-Style` header ([RFC 7826 section 18.47](https://tools.ietf.org/html/rfc7826#section-18.47)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SeekStyle {
    Rap,
    CoRap,
    FirstPrior,
    Next,
    Extension(String),
}

impl SeekStyle {
    pub fn as_str(&self) -> &str {
        match self {
            SeekStyle::Rap => "RAP",
            SeekStyle::CoRap => "CoRAP",
            SeekStyle::FirstPrior => "First-Prior",
            SeekStyle::Next => "Next",
            SeekStyle::Extension(ref s) => s.as_str(),
        }
    }
}

impl fmt::Display for SeekStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for SeekStyle {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        match s {
            "RAP" => Ok(SeekStyle::Rap),
            "CoRAP" => Ok(SeekStyle::CoRap),
            "First-Prior" => Ok(SeekStyle::FirstPrior),
            "Next" => Ok(SeekStyle::Next),
            _ => Ok(SeekStyle::Extension(String::from(s))),
        }
    }
}

impl super::TypedHeader for SeekStyle {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&SEEK_STYLE) {
            None => return Ok(None),
            Some(header) => header,
        };

        let seek_style = header.as_str().parse().map_err(|_| HeaderParseError)?;

        Ok(Some(seek_style))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        headers.insert(SEEK_STYLE, self.to_string());
    }
}
