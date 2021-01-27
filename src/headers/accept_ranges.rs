// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use std::fmt;

/// `Accept-Ranges` header ([RFC 7826 section 18.5](https://tools.ietf.org/html/rfc7826#section-18.5)).
#[derive(Debug, Clone)]
pub struct AcceptRanges(Vec<RangeUnit>);

/// Range units.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RangeUnit {
    /// Normal playback time.
    Npt,
    /// SMPTE 30 frames per second timecodes.
    Smpte,
    /// SMPTE 30 frames per second timecodes with drop frames (29.97 fps).
    Smpte30Drop,
    /// SMPTE 25 frames per second timecodes.
    Smpte25,
    /// Absolute time (UTC).
    Clock,
    /// Extension range unit.
    Extension(String),
}

impl RangeUnit {
    pub fn as_str(&self) -> &str {
        match self {
            RangeUnit::Npt => "npt",
            RangeUnit::Smpte => "smpte",
            RangeUnit::Smpte30Drop => "smpte-30-drop",
            RangeUnit::Smpte25 => "smpte-25",
            RangeUnit::Clock => "clock",
            RangeUnit::Extension(ref s) => s.as_str(),
        }
    }
}

impl fmt::Display for RangeUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for RangeUnit {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        match s {
            "npt" => Ok(RangeUnit::Npt),
            "smpte" => Ok(RangeUnit::Smpte),
            "smpte-30-drop" => Ok(RangeUnit::Smpte30Drop),
            "smpte-25" => Ok(RangeUnit::Smpte25),
            "clock" => Ok(RangeUnit::Clock),
            _ => Ok(RangeUnit::Extension(String::from(s))),
        }
    }
}

impl std::ops::Deref for AcceptRanges {
    type Target = Vec<RangeUnit>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for AcceptRanges {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Vec<RangeUnit>> for AcceptRanges {
    fn as_ref(&self) -> &Vec<RangeUnit> {
        &self.0
    }
}

impl AsMut<Vec<RangeUnit>> for AcceptRanges {
    fn as_mut(&mut self) -> &mut Vec<RangeUnit> {
        &mut self.0
    }
}

impl From<Vec<RangeUnit>> for AcceptRanges {
    fn from(v: Vec<RangeUnit>) -> Self {
        AcceptRanges(v)
    }
}

impl<'a> From<&'a [RangeUnit]> for AcceptRanges {
    fn from(v: &'a [RangeUnit]) -> Self {
        AcceptRanges(v.to_vec())
    }
}

impl AcceptRanges {
    /// Creates a new `Accept-Ranges` header builder.
    pub fn builder() -> AcceptRangesBuilder {
        AcceptRangesBuilder(Vec::new())
    }
}

/// Builder for the 'Accept-Ranges' header.
#[derive(Debug, Clone)]
pub struct AcceptRangesBuilder(Vec<RangeUnit>);

impl AcceptRangesBuilder {
    /// Add the provided range to the `Accept-Ranges` header.
    pub fn range(mut self, range: RangeUnit) -> Self {
        self.0.push(range);
        self
    }

    /// Build the `Accept-Ranges` header.
    pub fn build(self) -> AcceptRanges {
        AcceptRanges(self.0)
    }
}

impl super::TypedHeader for AcceptRanges {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&ACCEPT_RANGES) {
            None => return Ok(None),
            Some(header) => header,
        };

        let mut ranges = Vec::new();
        for range in header.as_str().split(',') {
            let range = range.trim();

            ranges.push(range.parse()?);
        }

        Ok(Some(AcceptRanges(ranges)))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        let mut ranges = String::new();
        for range in &self.0 {
            if !ranges.is_empty() {
                ranges.push_str(", ");
            }

            ranges.push_str(range.as_str());
        }

        headers.insert(ACCEPT_RANGES, ranges);
    }
}

impl super::TypedAppendableHeader for AcceptRanges {
    fn append_to(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        let mut ranges = String::new();
        for range in &self.0 {
            if !ranges.is_empty() {
                ranges.push_str(", ");
            }

            ranges.push_str(range.as_str());
        }

        headers.append(ACCEPT_RANGES, ranges);
    }
}
