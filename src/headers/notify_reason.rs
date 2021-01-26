// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use std::fmt;

/// `Notify-Reason` header ([RFC 7826 section 18.32](https://tools.ietf.org/html/rfc7826#section-18.32)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum NotifyReason {
    EndOfStream,
    MediaPropertiesUpdate,
    ScaleChange,
    Extension(String),
}

impl NotifyReason {
    pub fn as_str(&self) -> &str {
        match self {
            NotifyReason::EndOfStream => "end-of-stream",
            NotifyReason::MediaPropertiesUpdate => "media-properties-update",
            NotifyReason::ScaleChange => "scale-change",
            NotifyReason::Extension(ref s) => s.as_str(),
        }
    }
}

impl fmt::Display for NotifyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for NotifyReason {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        match s {
            "end-of-stream" => Ok(NotifyReason::EndOfStream),
            "media-properties-update" => Ok(NotifyReason::MediaPropertiesUpdate),
            "scale-change" => Ok(NotifyReason::ScaleChange),
            _ => Ok(NotifyReason::Extension(String::from(s))),
        }
    }
}

impl super::TypedHeader for NotifyReason {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&NOTIFY_REASON) {
            None => return Ok(None),
            Some(header) => header,
        };

        let notify_reason = header.as_str().parse().map_err(|_| HeaderParseError)?;

        Ok(Some(notify_reason))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        headers.insert(NOTIFY_REASON, self.to_string());
    }
}
