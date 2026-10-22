// Copyright (C) 2021 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use std::convert::TryFrom;
use std::fmt;

/// `Range` header ([RFC 7826 section 18.40](https://tools.ietf.org/html/rfc7826#section-18.40)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        // Route by the exact range type name, not just a prefix: per RFC 7826
        // section 20.39 any other token (even one starting with a known range
        // type, like "nptx") is a range extension.
        let name = match s.split_once('=') {
            Some((name, _)) => name,
            None => s,
        };

        if name == "npt" {
            Ok(Range::Npt(s.parse()?))
        } else if name == "clock" {
            Ok(Range::Utc(s.parse()?))
        } else if name.starts_with("smpte") {
            // "smpte" followed by any token is a valid SMPTE type extension.
            Ok(Range::Smpte(s.parse()?))
        } else {
            Ok(Range::Other(s.into()))
        }
    }
}

/// Normal Play Time Range ([RFC 7826 section 4.4.2](https://tools.ietf.org/html/rfc7826#section-4.4.2)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            NptRange::From(f) => write!(fmt, "npt={f}-"),
            NptRange::FromTo(f, t) => write!(fmt, "npt={f}-{t}"),
            NptRange::To(t) => write!(fmt, "npt=-{t}"),
        }
    }
}

impl std::str::FromStr for NptRange {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        let s = s.strip_prefix("npt").ok_or_else(HeaderParseError::new)?;

        if s.is_empty() {
            return Ok(NptRange::Empty);
        }

        let s = s.strip_prefix('=').ok_or_else(HeaderParseError::new)?;

        let (from, to) = s.split_once('-').ok_or_else(HeaderParseError::new)?;
        let from = if from.is_empty() { None } else { Some(from) };
        let to = if to.is_empty() { None } else { Some(to) };

        let from = from
            .map(|s| s.parse::<NptTime>().map_err(|_| HeaderParseError::new()))
            .transpose()?;
        let to = to
            .map(|s| s.parse::<NptTime>().map_err(|_| HeaderParseError::new()))
            .transpose()?;

        match (from, to) {
            (Some(from), Some(to)) => Ok(NptRange::FromTo(from, to)),
            (None, Some(to)) => Ok(NptRange::To(to)),
            (Some(from), None) => Ok(NptRange::From(from)),
            (None, None) => Err(HeaderParseError::new()),
        }
    }
}

/// Normal Play Time ([RFC 7826 section 4.4.2](https://tools.ietf.org/html/rfc7826#section-4.4.2)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum NptTime {
    /// Now.
    Now,
    /// Seconds and nanoseconds.
    Seconds(u64, Option<u32>),
    /// Hours, minutes, seconds and nanoseconds.
    Hms(u64, u8, u8, Option<u32>),
}

impl NptTime {
    fn write_fraction(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some(nanoseconds) = self.nanoseconds() else {
            return Ok(());
        };
        if nanoseconds == 0 {
            return Ok(());
        }
        let mut fraction = format!("{nanoseconds:09}");
        while fraction.ends_with('0') {
            fraction.pop();
        }
        write!(fmt, ".{fraction}")
    }

    fn nanoseconds(&self) -> Option<u32> {
        match self {
            NptTime::Now | NptTime::Seconds(_, None) | NptTime::Hms(_, _, _, None) => None,
            NptTime::Seconds(_, Some(nanoseconds)) | NptTime::Hms(_, _, _, Some(nanoseconds)) => {
                Some(*nanoseconds)
            }
        }
    }
}

impl fmt::Display for NptTime {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NptTime::Now => fmt.write_str("now"),
            NptTime::Seconds(seconds, _) => {
                write!(fmt, "{seconds}")?;
                self.write_fraction(fmt)
            }
            NptTime::Hms(hours, minutes, seconds, _) => {
                write!(fmt, "{hours:02}:{minutes:02}:{seconds:02}")?;
                self.write_fraction(fmt)
            }
        }
    }
}

impl std::str::FromStr for NptTime {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        if s == "now" {
            return Ok(NptTime::Now);
        }

        match s.split_once(':') {
            None => match s.split_once('.') {
                None => {
                    let seconds = parser_helpers::parse_digits::<u64>(s)
                        .map_err(|_| HeaderParseError::new())?;
                    Ok(NptTime::Seconds(seconds, None))
                }
                Some((seconds, subseconds)) => {
                    let seconds = parser_helpers::parse_digits::<u64>(seconds)
                        .map_err(|_| HeaderParseError::new())?;
                    let digits = subseconds.len();
                    if digits > 9 || digits == 0 {
                        return Err(HeaderParseError::new());
                    }
                    let subseconds = parser_helpers::parse_digits::<u32>(subseconds)
                        .map_err(|_| HeaderParseError::new())?;

                    let nanoseconds = subseconds * u32::pow(10, 9 - digits as u32);

                    Ok(NptTime::Seconds(seconds, Some(nanoseconds)))
                }
            },
            Some((hours, s)) => {
                let hours = parser_helpers::parse_digits::<u64>(hours)
                    .map_err(|_| HeaderParseError::new())?;
                let mut it = s.split(':');
                let minutes = it
                    .next()
                    .and_then(|s| parser_helpers::parse_digits::<u8>(s).ok())
                    .ok_or_else(HeaderParseError::new)?;
                // Minutes and seconds are 0-59 per RFC 7826 section 20.39.
                if minutes >= 60 {
                    return Err(HeaderParseError::new());
                }
                let seconds = it.next().ok_or_else(HeaderParseError::new)?;

                if let Some((seconds, subseconds)) = seconds.split_once('.') {
                    let seconds = parser_helpers::parse_digits::<u8>(seconds)
                        .map_err(|_| HeaderParseError::new())?;
                    if seconds >= 60 {
                        return Err(HeaderParseError::new());
                    }
                    let digits = subseconds.len();
                    if digits > 9 || digits == 0 {
                        return Err(HeaderParseError::new());
                    }
                    let subseconds = parser_helpers::parse_digits::<u32>(subseconds)
                        .map_err(|_| HeaderParseError::new())?;

                    let nanoseconds = subseconds * u32::pow(10, 9 - digits as u32);

                    Ok(NptTime::Hms(hours, minutes, seconds, Some(nanoseconds)))
                } else {
                    let seconds = parser_helpers::parse_digits::<u8>(seconds)
                        .map_err(|_| HeaderParseError::new())?;
                    if seconds >= 60 {
                        return Err(HeaderParseError::new());
                    }

                    Ok(NptTime::Hms(hours, minutes, seconds, None))
                }
            }
        }
    }
}

/// SMPTE-Relative Timecode Range ([RFC 7826 section 4.4.1](https://tools.ietf.org/html/rfc7826#section-4.4.1)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            SmpteRange::Empty(ty) => write!(fmt, "{ty}"),
            SmpteRange::From(ty, f) => write!(fmt, "{ty}={f}-"),
            SmpteRange::FromTo(ty, f, t) => write!(fmt, "{ty}={f}-{t}"),
            SmpteRange::To(ty, t) => write!(fmt, "{ty}=-{t}"),
        }
    }
}

impl std::str::FromStr for SmpteRange {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        if let Some((ty, range)) = s.split_once('=') {
            let ty = ty.parse()?;

            if range.is_empty() {
                return Ok(SmpteRange::Empty(ty));
            }

            let (from, to) = range.split_once('-').ok_or_else(HeaderParseError::new)?;
            let from = if from.is_empty() { None } else { Some(from) };
            let to = if to.is_empty() { None } else { Some(to) };

            let from = from
                .map(|s| s.parse::<SmpteTime>().map_err(|_| HeaderParseError::new()))
                .transpose()?;
            let to = to
                .map(|s| s.parse::<SmpteTime>().map_err(|_| HeaderParseError::new()))
                .transpose()?;

            if let Some(frames_per_second) = ty.frames_per_second() {
                for time in from.iter().chain(to.iter()) {
                    // The frames field is bounded by the frame rate of the
                    // timecode type, e.g. 0-29 for 30 frames/sec.
                    if let Some((frames, _)) = time.frames {
                        if frames >= frames_per_second {
                            return Err(HeaderParseError::new());
                        }
                    }
                }
            }

            match (from, to) {
                (Some(from), Some(to)) => Ok(SmpteRange::FromTo(ty, from, to)),
                (None, Some(to)) => Ok(SmpteRange::To(ty, to)),
                (Some(from), None) => Ok(SmpteRange::From(ty, from)),
                (None, None) => Err(HeaderParseError::new()),
            }
        } else {
            Ok(SmpteRange::Empty(s.parse()?))
        }
    }
}

/// SMPTE-Relative Timecode Type ([RFC 7826 section 4.4.1](https://tools.ietf.org/html/rfc7826#section-4.4.1)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        let stripped = s.strip_prefix("smpte").ok_or_else(HeaderParseError::new)?;
        match stripped {
            "" => Ok(SmpteType::Smpte),
            "-30-drop" => Ok(SmpteType::Smpte30Drop),
            "-25" => Ok(SmpteType::Smpte25),
            _ => Ok(SmpteType::Other(s.into())),
        }
    }
}

impl SmpteType {
    /// Frame rate in frames per second, for the timecode types with a known
    /// frame rate.
    fn frames_per_second(&self) -> Option<u8> {
        match self {
            SmpteType::Smpte | SmpteType::Smpte30Drop => Some(30),
            SmpteType::Smpte25 => Some(25),
            SmpteType::Other(_) => None,
        }
    }
}

/// SMPTE-Relative Timecode ([RFC 7826 section 4.4.1](https://tools.ietf.org/html/rfc7826#section-4.4.1)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        // All smpte-time components are `1*2DIGIT` per RFC 7826 section
        // 20.39, so at most two ASCII digits and in particular <= 99.
        let parse_component = |s: &str| -> Result<u8, HeaderParseError> {
            if s.len() > 2 {
                return Err(HeaderParseError::new());
            }
            parser_helpers::parse_digits::<u8>(s).map_err(|_| HeaderParseError::new())
        };

        let mut s = s.split(':');

        let hours = s
            .next()
            .map(parse_component)
            .ok_or_else(HeaderParseError::new)??;
        let minutes = s
            .next()
            .map(parse_component)
            .ok_or_else(HeaderParseError::new)??;
        let seconds = s
            .next()
            .map(parse_component)
            .ok_or_else(HeaderParseError::new)??;

        // Minutes and seconds are 0-59 in SMPTE timecodes.
        if minutes >= 60 || seconds >= 60 {
            return Err(HeaderParseError::new());
        }

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
            return Err(HeaderParseError::new());
        }

        if let Some((frames, subframes)) = frames.split_once('.') {
            let frames = parse_component(frames)?;
            let digits = subframes.len();

            let factor = match digits {
                1 => 10,
                2 => 1,
                _ => return Err(HeaderParseError::new()),
            };

            let subframes = parser_helpers::parse_digits::<u8>(subframes)
                .map_err(|_| HeaderParseError::new())?
                * factor;

            Ok(SmpteTime {
                hours,
                minutes,
                seconds,
                frames: Some((frames, Some(subframes))),
            })
        } else {
            let frames = parse_component(frames)?;

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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            UtcRange::From(f) => write!(fmt, "clock={f}-"),
            UtcRange::FromTo(f, t) => write!(fmt, "clock={f}-{t}"),
            UtcRange::To(t) => write!(fmt, "clock=-{t}"),
        }
    }
}

impl std::str::FromStr for UtcRange {
    type Err = HeaderParseError;

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        let s = s.strip_prefix("clock").ok_or_else(HeaderParseError::new)?;

        if s.is_empty() {
            return Ok(UtcRange::Empty);
        }

        let s = s.strip_prefix('=').ok_or_else(HeaderParseError::new)?;

        let (from, to) = s.split_once('-').ok_or_else(HeaderParseError::new)?;
        let from = if from.is_empty() { None } else { Some(from) };
        let to = if to.is_empty() { None } else { Some(to) };

        let from = from
            .map(|s| s.parse::<UtcTime>().map_err(|_| HeaderParseError::new()))
            .transpose()?;
        let to = to
            .map(|s| s.parse::<UtcTime>().map_err(|_| HeaderParseError::new()))
            .transpose()?;

        match (from, to) {
            (Some(from), Some(to)) => Ok(UtcRange::FromTo(from, to)),
            (None, Some(to)) => Ok(UtcRange::To(to)),
            (Some(from), None) => Ok(UtcRange::From(from)),
            (None, None) => Err(HeaderParseError::new()),
        }
    }
}

/// Absolute Time (UTC) Time ([RFC 7826 section 4.4.3](https://tools.ietf.org/html/rfc7826#section-4.4.3)).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
        let (date, time) = s.split_once('T').ok_or_else(HeaderParseError::new)?;
        let time = time.strip_suffix('Z').ok_or_else(HeaderParseError::new)?;

        // The date is YYYYMMDD and the time is HHMMSS with an optional
        // fractional second, per RFC 7826 sections 4.4.3 and 20.39.
        if date.len() != 8 || !date.bytes().all(|b| b.is_ascii_digit()) {
            return Err(HeaderParseError::new());
        }

        let (time, nanoseconds) = match time.split_once('.') {
            Some((time, subseconds)) => {
                let digits = subseconds.len();
                if digits > 9 || digits == 0 {
                    return Err(HeaderParseError::new());
                }
                let subseconds = subseconds
                    .parse::<u32>()
                    .map_err(|_| HeaderParseError::new())?;

                let nanoseconds = subseconds * u32::pow(10, 9 - digits as u32);

                (time, Some(nanoseconds))
            }
            None => (time, None),
        };

        if time.len() != 6 || !time.bytes().all(|b| b.is_ascii_digit()) {
            return Err(HeaderParseError::new());
        }

        let date = date.parse::<u32>().map_err(|_| HeaderParseError::new())?;
        let time = time.parse::<u32>().map_err(|_| HeaderParseError::new())?;

        // The date has to refer to a real day of a real month and the time
        // to a real time of day; use the `time` crate for the calendar
        // calculations (leap years, month lengths, value ranges).
        time::Date::from_calendar_date(
            (date / 10000) as i32,
            time::Month::try_from((date / 100 % 100) as u8).map_err(|_| HeaderParseError::new())?,
            (date % 100) as u8,
        )
        .map_err(|_| HeaderParseError::new())?;

        time::Time::from_hms(
            (time / 10000) as u8,
            (time / 100 % 100) as u8,
            (time % 100) as u8,
        )
        .map_err(|_| HeaderParseError::new())?;

        Ok(UtcTime {
            date,
            time,
            nanoseconds,
        })
    }
}

impl super::TypedHeader for Range {
    fn header_name() -> Option<HeaderName> {
        Some(RANGE.clone())
    }
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
                None,
            ),
            (
                "npt=123.0-456.000",
                Range::Npt(NptRange::FromTo(
                    NptTime::Seconds(123, Some(0)),
                    NptTime::Seconds(456, Some(0)),
                )),
                Some("npt=123-456"),
            ),
            (
                "npt=43:53:10-1:17:59.123",
                Range::Npt(NptRange::FromTo(
                    NptTime::Hms(43, 53, 10, None),
                    NptTime::Hms(1, 17, 59, Some(123_000_000)),
                )),
                Some("npt=43:53:10-01:17:59.123"),
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

            assert_eq!(range, *expected, "{header}");

            let request2 = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .typed_header(&range)
                .empty();

            let range = request2.header(&crate::headers::RANGE).unwrap();

            assert_eq!(range, serialized.unwrap_or(header), "{header}");
        }
    }

    #[test]
    fn test_range_extension_routing() {
        // Extension range specs must not be misrouted just because their
        // name starts with a known range type.
        assert_eq!(
            "nptx=1-2".parse::<Range>().unwrap(),
            Range::Other("nptx=1-2".into())
        );
        assert_eq!(
            "clockx=1".parse::<Range>().unwrap(),
            Range::Other("clockx=1".into())
        );
        assert_eq!(
            "asef".parse::<Range>().unwrap(),
            Range::Other("asef".into())
        );

        // Known range types still route to their parser.
        assert_eq!(
            "npt=1-2".parse::<Range>().unwrap(),
            Range::Npt(NptRange::FromTo(
                NptTime::Seconds(1, None),
                NptTime::Seconds(2, None)
            ))
        );
        assert_eq!("npt".parse::<Range>().unwrap(), Range::Npt(NptRange::Empty));
        assert_eq!(
            "clock".parse::<Range>().unwrap(),
            Range::Utc(UtcRange::Empty)
        );
        assert_eq!(
            "smpte".parse::<Range>().unwrap(),
            Range::Smpte(SmpteRange::Empty(SmpteType::Smpte))
        );
        // "smpte" followed by a token is a valid SMPTE type extension per
        // RFC 7826 section 20.39.
        assert!(matches!(
            "smptefoo=1:2:3-4:5:6".parse::<Range>().unwrap(),
            Range::Smpte(_)
        ));
    }

    #[test]
    fn test_range_time_validation() {
        let valid_headers = [
            "npt=0:59:59.5-1:00:00",
            "npt=1-99:59:59",
            "smpte=23:59:59:29.99-01:02:03",
            "smpte=10:07:00-10:07:33:05.01",
            "smpte-25=01:02:03:24-01:02:03:24",
            // For other types the frame rate is unknown, frames cannot be
            // checked.
            "smpteext=01:02:03:99-",
            "clock=20160229T235959.999999999Z-20160301T000000Z",
            "clock=20000229T000000Z-",
        ];

        for header in &valid_headers {
            assert!(header.parse::<Range>().is_ok(), "{} should parse", header);
        }

        let invalid_headers = [
            // NPT minutes and seconds are 0-59.
            "npt=0:60:00-",
            "npt=0:59:60-",
            "npt=1-0:0:60",
            "npt=0:99:99-",
            // SMPTE minutes and seconds are 0-59.
            "smpte=01:60:00-",
            "smpte=01:02:60-",
            "smpte=-23:00:60",
            // SMPTE time components are `1*2DIGIT`, so at most two digits
            // and in particular the hours are <= 99.
            "smpte=100:00:00-",
            "smpte=-01:02:03:100",
            "smpteext=01:02:03:100-",
            // Leading `+` signs are not digits.
            "npt=+5",
            "npt=+0:00:05-",
            "smpte=+1:02:03-",
            // The frames field is bounded by the frame rate of the type.
            "smpte=01:02:03:30-",
            "smpte=-01:02:03:30",
            "smpte-30-drop=01:02:03:30-",
            "smpte-25=01:02:03:25-01:02:03:24",
            // The UTC date has to be a valid calendar date and the time
            // valid.
            "clock=99999999T999999Z",
            "clock=20151301T120000Z",
            "clock=20150132T120000Z",
            "clock=20150100T120000Z",
            "clock=20150230T120000Z",
            "clock=19000229T120000Z",
            "clock=20150101T240000Z",
            "clock=20150101T126000Z",
            "clock=20150101T120060Z",
            "clock=20150101T1200Z",
            "clock=2015010T120000Z",
            "clock=2015-01-01T120000Z",
        ];

        for header in &invalid_headers {
            assert!(header.parse::<Range>().is_err(), "{} should fail", header);
        }
    }

    #[test]
    fn test_range_smpte() {
        let headers = [
            (
                "smpte",
                Range::Smpte(SmpteRange::Empty(SmpteType::Smpte)),
                None,
            ),
            (
                "smpte=1:2:3:4-5:6:7:8",
                Range::Smpte(SmpteRange::FromTo(
                    SmpteType::Smpte,
                    SmpteTime {
                        hours: 1,
                        minutes: 2,
                        seconds: 3,
                        frames: Some((4, None)),
                    },
                    SmpteTime {
                        hours: 5,
                        minutes: 6,
                        seconds: 7,
                        frames: Some((8, None)),
                    },
                )),
                Some("smpte=01:02:03:04-05:06:07:08"),
            ),
            (
                "smpte=1:2:3:4-",
                Range::Smpte(SmpteRange::From(
                    SmpteType::Smpte,
                    SmpteTime {
                        hours: 1,
                        minutes: 2,
                        seconds: 3,
                        frames: Some((4, None)),
                    },
                )),
                Some("smpte=01:02:03:04-"),
            ),
            (
                "smpte=-1:2:3:4",
                Range::Smpte(SmpteRange::To(
                    SmpteType::Smpte,
                    SmpteTime {
                        hours: 1,
                        minutes: 2,
                        seconds: 3,
                        frames: Some((4, None)),
                    },
                )),
                Some("smpte=-01:02:03:04"),
            ),
            (
                "smpte-25=01:02:03:04.05-05:06:07:08.09",
                Range::Smpte(SmpteRange::FromTo(
                    SmpteType::Smpte25,
                    SmpteTime {
                        hours: 1,
                        minutes: 2,
                        seconds: 3,
                        frames: Some((4, Some(5))),
                    },
                    SmpteTime {
                        hours: 5,
                        minutes: 6,
                        seconds: 7,
                        frames: Some((8, Some(9))),
                    },
                )),
                None,
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

            assert_eq!(range, *expected, "{header}");

            let request2 = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .typed_header(&range)
                .empty();

            let range = request2.header(&crate::headers::RANGE).unwrap();

            assert_eq!(range, serialized.unwrap_or(header), "{header}");
        }
    }
}
