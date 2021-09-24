// Copyright (C) 2021 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use super::parser_helpers::split_once;
use std::fmt;

/// `Range` header ([RFC 7826 section 18.40](https://tools.ietf.org/html/rfc7826#section-18.40)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Range {
    /// Normal Play Time Range ([RFC 7826 section 4.4.2](https://tools.ietf.org/html/rfc7826#section-4.4.2)).
    Npt(NptRange),
    /// SMPTE-Relative Timecode Range ([RFC 7826 section 4.4.1](https://tools.ietf.org/html/rfc7826#section-4.4.1)).
    Smpte(SmpteRange),
    /// Absolute Time (UTC) Time Range ([RFC 7826 section 4.4.3](https://tools.ietf.org/html/rfc7826#section-4.4.3)).
    Utc(UtcRange),
    /// Other time range.
    Other(String),
}

impl fmt::Display for Range {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Range::Npt(r) => <NptRange as fmt::Display>::fmt(r, f),
            Range::Smpte(r) => <SmpteRange as fmt::Display>::fmt(r, f),
            Range::Utc(r) => <UtcRange as fmt::Display>::fmt(r, f),
            Range::Other(r) => <String as fmt::Display>::fmt(r, f),
        }
    }
}

impl std::str::FromStr for Range {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        if s.starts_with("npt") {
            Ok(Range::Npt(s.parse()?))
        } else if s.starts_with("clock") {
            Ok(Range::Utc(s.parse()?))
        } else if s.starts_with("smpte") {
            Ok(Range::Smpte(s.parse()?))
        } else {
            Ok(Range::Other(s.into()))
        }
    }
}

/// Normal Play Time Range ([RFC 7826 section 4.4.2](https://tools.ietf.org/html/rfc7826#section-4.4.2)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NptRange {
    /// Empty range.
    Empty,
    /// Range starting at a specific time.
    From(NptTime),
    /// Range from a specific time to another.
    FromTo(NptTime, NptTime),
    /// Range ending at a specific time.
    To(NptTime),
}

impl fmt::Display for NptRange {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NptRange::Empty => fmt.write_str("npt"),
            NptRange::From(f) => write!(fmt, "npt={}-", f),
            NptRange::FromTo(f, t) => write!(fmt, "npt={}-{}", f, t),
            NptRange::To(t) => write!(fmt, "npt=-{}", t),
        }
    }
}

impl std::str::FromStr for NptRange {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        let s = s.strip_prefix("npt").ok_or(HeaderParseError)?;

        if s.is_empty() {
            return Ok(NptRange::Empty);
        }

        let s = s.strip_prefix('=').ok_or(HeaderParseError)?;

        let (from, to) = split_once(s, '-').ok_or(HeaderParseError)?;
        let from = if from.is_empty() { None } else { Some(from) };
        let to = if to.is_empty() { None } else { Some(to) };

        let from = from
            .map(|s| s.parse::<NptTime>().map_err(|_| HeaderParseError))
            .transpose()?;
        let to = to
            .map(|s| s.parse::<NptTime>().map_err(|_| HeaderParseError))
            .transpose()?;

        match (from, to) {
            (Some(from), Some(to)) => Ok(NptRange::FromTo(from, to)),
            (None, Some(to)) => Ok(NptRange::To(to)),
            (Some(from), None) => Ok(NptRange::From(from)),
            (None, None) => Err(HeaderParseError),
        }
    }
}

/// Normal Play Time ([RFC 7826 section 4.4.2](https://tools.ietf.org/html/rfc7826#section-4.4.2)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NptTime {
    /// Now.
    Now,
    /// Seconds and nanoseconds.
    Seconds(u64, Option<u32>),
    /// Hours, minutes, seconds and nanoseconds.
    Hms(u64, u8, u8, Option<u32>),
}

impl fmt::Display for NptTime {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NptTime::Now => fmt.write_str("now"),
            NptTime::Seconds(seconds, None) => write!(fmt, "{}", seconds),
            NptTime::Seconds(seconds, Some(nanoseconds)) => {
                write!(fmt, "{}.{:09}", seconds, nanoseconds)
            }
            NptTime::Hms(hours, minutes, seconds, None) => {
                write!(fmt, "{:02}:{:02}:{:02}", hours, minutes, seconds)
            }
            NptTime::Hms(hours, minutes, seconds, Some(nanoseconds)) => write!(
                fmt,
                "{:02}:{:02}:{:02}.{:09}",
                hours, minutes, seconds, nanoseconds
            ),
        }
    }
}

impl std::str::FromStr for NptTime {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        if s == "now" {
            return Ok(NptTime::Now);
        }

        match split_once(s, ':') {
            None => match split_once(s, '.') {
                None => {
                    let seconds = s.parse::<u64>().map_err(|_| HeaderParseError)?;
                    Ok(NptTime::Seconds(seconds, None))
                }
                Some((seconds, subseconds)) => {
                    let seconds = seconds.parse::<u64>().map_err(|_| HeaderParseError)?;
                    let digits = subseconds.len();
                    if digits > 9 || digits == 0 {
                        return Err(HeaderParseError);
                    }
                    let subseconds = subseconds.parse::<u32>().map_err(|_| HeaderParseError)?;

                    let nanoseconds = subseconds * u32::pow(10, 9 - digits as u32);

                    Ok(NptTime::Seconds(seconds, Some(nanoseconds)))
                }
            },
            Some((hours, s)) => {
                let hours = hours.parse::<u64>().map_err(|_| HeaderParseError)?;
                let mut it = s.split(':');
                let minutes = it
                    .next()
                    .map(|s| s.parse::<u8>().ok())
                    .flatten()
                    .ok_or(HeaderParseError)?;
                let seconds = it.next().ok_or(HeaderParseError)?;

                if let Some((seconds, subseconds)) = split_once(seconds, '.') {
                    let seconds = seconds.parse::<u8>().map_err(|_| HeaderParseError)?;
                    let digits = subseconds.len();
                    if digits > 9 || digits == 0 {
                        return Err(HeaderParseError);
                    }
                    let subseconds = subseconds.parse::<u32>().map_err(|_| HeaderParseError)?;

                    let nanoseconds = subseconds * u32::pow(10, 9 - digits as u32);

                    Ok(NptTime::Hms(hours, minutes, seconds, Some(nanoseconds)))
                } else {
                    let seconds = seconds.parse::<u8>().map_err(|_| HeaderParseError)?;

                    Ok(NptTime::Hms(hours, minutes, seconds, None))
                }
            }
        }
    }
}

/// SMPTE-Relative Timecode Range ([RFC 7826 section 4.4.1](https://tools.ietf.org/html/rfc7826#section-4.4.1)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmpteRange {
    /// Empty range.
    Empty(SmpteType),
    /// Range starting at a specific time.
    From(SmpteType, SmpteTime),
    /// Range from a specific time to another.
    FromTo(SmpteType, SmpteTime, SmpteTime),
    /// Range ending at a specific time.
    To(SmpteType, SmpteTime),
}

impl fmt::Display for SmpteRange {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SmpteRange::Empty(ty) => write!(fmt, "{}", ty),
            SmpteRange::From(ty, f) => write!(fmt, "{}={}-", ty, f),
            SmpteRange::FromTo(ty, f, t) => write!(fmt, "{}={}-{}", ty, f, t),
            SmpteRange::To(ty, t) => write!(fmt, "{}=-{}", ty, t),
        }
    }
}

impl std::str::FromStr for SmpteRange {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        if let Some((ty, range)) = split_once(s, '=') {
            let ty = ty.parse()?;

            if range.is_empty() {
                return Ok(SmpteRange::Empty(ty));
            }

            let range = range.strip_prefix('=').ok_or(HeaderParseError)?;

            let (from, to) = split_once(range, '-').ok_or(HeaderParseError)?;
            let from = if from.is_empty() { None } else { Some(from) };
            let to = if to.is_empty() { None } else { Some(to) };

            let from = from
                .map(|s| s.parse::<SmpteTime>().map_err(|_| HeaderParseError))
                .transpose()?;
            let to = to
                .map(|s| s.parse::<SmpteTime>().map_err(|_| HeaderParseError))
                .transpose()?;

            match (from, to) {
                (Some(from), Some(to)) => Ok(SmpteRange::FromTo(ty, from, to)),
                (None, Some(to)) => Ok(SmpteRange::To(ty, to)),
                (Some(from), None) => Ok(SmpteRange::From(ty, from)),
                (None, None) => Err(HeaderParseError),
            }
        } else {
            Ok(SmpteRange::Empty(s.parse()?))
        }
    }
}

/// SMPTE-Relative Timecode Type ([RFC 7826 section 4.4.1](https://tools.ietf.org/html/rfc7826#section-4.4.1)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SmpteType {
    /// SMPTE 30 frames per second timecodes.
    Smpte,
    /// SMPTE 30 frames per second timecodes with drop frames (29.97 fps).
    Smpte30Drop,
    /// SMPTE 25 frames per second timecodes.
    Smpte25,
    /// Other SMPTE timecode type.
    Other(String),
}

impl fmt::Display for SmpteType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SmpteType::Smpte => f.write_str("smpte"),
            SmpteType::Smpte30Drop => f.write_str("smpte-30-drop"),
            SmpteType::Smpte25 => f.write_str("smpte-25"),
            SmpteType::Other(o) => f.write_str(o),
        }
    }
}

impl std::str::FromStr for SmpteType {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        let stripped = s.strip_prefix("smpte").ok_or(HeaderParseError)?;
        match stripped {
            "" => Ok(SmpteType::Smpte),
            "-30-drop" => Ok(SmpteType::Smpte30Drop),
            "-25" => Ok(SmpteType::Smpte25),
            _ => Ok(SmpteType::Other(s.into())),
        }
    }
}

/// SMPTE-Relative Timecode ([RFC 7826 section 4.4.1](https://tools.ietf.org/html/rfc7826#section-4.4.1)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmpteTime {
    /// Hours (0-23).
    pub hours: u8,
    /// Minutes (0-59).
    pub minutes: u8,
    /// Seconds (0-59).
    pub seconds: u8,
    /// Frames and subframes (0-framerate, 0-99).
    pub frames: Option<(u8, Option<u8>)>,
}

impl fmt::Display for SmpteTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.frames {
            None => write!(
                f,
                "{:02}:{:02}:{:02}",
                self.hours, self.minutes, self.seconds
            ),
            Some((frames, None)) => write!(
                f,
                "{:02}:{:02}:{:02}:{:02}",
                self.hours, self.minutes, self.seconds, frames
            ),
            Some((frames, Some(subframes))) => write!(
                f,
                "{:02}:{:02}:{:02}:{:02}.{:02}",
                self.hours, self.minutes, self.seconds, frames, subframes
            ),
        }
    }
}

impl std::str::FromStr for SmpteTime {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        let mut s = s.split(':');

        let hours = s
            .next()
            .map(|s| s.parse::<u8>().ok())
            .flatten()
            .ok_or(HeaderParseError)?;
        let minutes = s
            .next()
            .map(|s| s.parse::<u8>().ok())
            .flatten()
            .ok_or(HeaderParseError)?;
        let seconds = s
            .next()
            .map(|s| s.parse::<u8>().ok())
            .flatten()
            .ok_or(HeaderParseError)?;

        let frames = match s.next() {
            Some(frames) => frames,
            None => {
                return Ok(SmpteTime {
                    hours,
                    minutes,
                    seconds,
                    frames: None,
                })
            }
        };

        if s.next().is_some() {
            return Err(HeaderParseError);
        }

        if let Some((frames, subframes)) = split_once(frames, '.') {
            let frames = frames.parse::<u8>().map_err(|_| HeaderParseError)?;
            let digits = subframes.len();

            let factor = match digits {
                1 => 10,
                2 => 1,
                _ => return Err(HeaderParseError),
            };

            let subframes = subframes.parse::<u8>().map_err(|_| HeaderParseError)? * factor;

            Ok(SmpteTime {
                hours,
                minutes,
                seconds,
                frames: Some((frames, Some(subframes))),
            })
        } else {
            let frames = frames.parse::<u8>().map_err(|_| HeaderParseError)?;

            Ok(SmpteTime {
                hours,
                minutes,
                seconds,
                frames: Some((frames, None)),
            })
        }
    }
}

/// Absolute Time (UTC) Time Range ([RFC 7826 section 4.4.3](https://tools.ietf.org/html/rfc7826#section-4.4.3)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UtcRange {
    /// Empty range.
    Empty,
    /// Range starting at a specific time.
    From(UtcTime),
    /// Range from a specific time to another.
    FromTo(UtcTime, UtcTime),
    /// Range ending at a specific time.
    To(UtcTime),
}

impl fmt::Display for UtcRange {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UtcRange::Empty => fmt.write_str("clock"),
            UtcRange::From(f) => write!(fmt, "clock={}-", f),
            UtcRange::FromTo(f, t) => write!(fmt, "clock={}-{}", f, t),
            UtcRange::To(t) => write!(fmt, "clock=-{}", t),
        }
    }
}

impl std::str::FromStr for UtcRange {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        let s = s.strip_prefix("clock").ok_or(HeaderParseError)?;

        if s.is_empty() {
            return Ok(UtcRange::Empty);
        }

        let s = s.strip_prefix('=').ok_or(HeaderParseError)?;

        let (from, to) = split_once(s, '-').ok_or(HeaderParseError)?;
        let from = if from.is_empty() { None } else { Some(from) };
        let to = if to.is_empty() { None } else { Some(to) };

        let from = from
            .map(|s| s.parse::<UtcTime>().map_err(|_| HeaderParseError))
            .transpose()?;
        let to = to
            .map(|s| s.parse::<UtcTime>().map_err(|_| HeaderParseError))
            .transpose()?;

        match (from, to) {
            (Some(from), Some(to)) => Ok(UtcRange::FromTo(from, to)),
            (None, Some(to)) => Ok(UtcRange::To(to)),
            (Some(from), None) => Ok(UtcRange::From(from)),
            (None, None) => Err(HeaderParseError),
        }
    }
}

/// Absolute Time (UTC) Time ([RFC 7826 section 4.4.3](https://tools.ietf.org/html/rfc7826#section-4.4.3)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UtcTime {
    /// YYYYMMDD date.
    pub date: u32,
    /// HHMMSS time.
    pub time: u32,
    /// Nanoseconds.
    pub nanoseconds: Option<u32>,
}

impl fmt::Display for UtcTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ns) = self.nanoseconds {
            write!(f, "{:08}T{:06}.{:09}Z", self.date, self.time, ns)
        } else {
            write!(f, "{:08}T{:06}Z", self.date, self.time)
        }
    }
}

impl std::str::FromStr for UtcTime {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        let (date, time) = split_once(s, 'T').ok_or(HeaderParseError)?;
        let time = time.strip_suffix('Z').ok_or(HeaderParseError)?;

        let date = date.parse::<u32>().map_err(|_| HeaderParseError)?;
        let (time, nanoseconds) = if let Some((time, subseconds)) = split_once(time, '.') {
            let time = time.parse::<u32>().map_err(|_| HeaderParseError)?;
            let digits = subseconds.len();
            if digits > 9 || digits == 0 {
                return Err(HeaderParseError);
            }
            let subseconds = subseconds.parse::<u32>().map_err(|_| HeaderParseError)?;

            let nanoseconds = subseconds * u32::pow(10, 9 - digits as u32);

            (time, Some(nanoseconds))
        } else {
            let time = time.parse::<u32>().map_err(|_| HeaderParseError)?;

            (time, None)
        };

        Ok(UtcTime {
            date,
            time,
            nanoseconds,
        })
    }
}

impl super::TypedHeader for Range {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&RANGE) {
            None => return Ok(None),
            Some(header) => header,
        };

        Ok(Some(header.as_str().parse()?))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();
        headers.insert(RANGE, self.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range() {
        let headers = [
            ("npt", Range::Npt(NptRange::Empty), None),
            ("npt=now-", Range::Npt(NptRange::From(NptTime::Now)), None),
            (
                "npt=123-456",
                Range::Npt(NptRange::FromTo(
                    NptTime::Seconds(123, None),
                    NptTime::Seconds(456, None),
                )),
                None,
            ),
            (
                "npt=-456",
                Range::Npt(NptRange::To(NptTime::Seconds(456, None))),
                None,
            ),
            (
                "npt=123-",
                Range::Npt(NptRange::From(NptTime::Seconds(123, None))),
                None,
            ),
            (
                "npt=123.5-456.567",
                Range::Npt(NptRange::FromTo(
                    NptTime::Seconds(123, Some(500_000_000)),
                    NptTime::Seconds(456, Some(567_000_000)),
                )),
                Some("npt=123.500000000-456.567000000"),
            ),
            (
                "npt=43:53:10-1:17:59.123",
                Range::Npt(NptRange::FromTo(
                    NptTime::Hms(43, 53, 10, None),
                    NptTime::Hms(1, 17, 59, Some(123_000_000)),
                )),
                Some("npt=43:53:10-01:17:59.123000000"),
            ),
        ];

        for (header, expected, serialized) in &headers {
            let request = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .header(crate::headers::RANGE, *header)
                .empty();

            let range = request
                .typed_header::<super::Range>()
                .unwrap_or_else(|_| panic!("couldn't parse {}", header))
                .unwrap();

            assert_eq!(range, *expected, "{}", header);

            let request2 = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .typed_header(&range)
                .empty();

            let range = request2.header(&crate::headers::RANGE).unwrap();

            assert_eq!(range, serialized.unwrap_or(header), "{}", header);
        }
    }
}
