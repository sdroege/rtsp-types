// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::UtcTime;
use super::*;
use std::fmt;

/// `Media-Properties` header ([RFC 7826 section 18.29](https://tools.ietf.org/html/rfc7826#section-18.29)).
#[derive(Debug, Clone)]
pub struct MediaProperties(Vec<MediaProperty>);

/// Media properties.
#[derive(Debug, Clone, PartialEq)]
pub enum MediaProperty {
    /// Random access access is possible in given duration.
    RandomAccess(Option<f64>),
    /// Random access is only possible in the beginning.
    BeginningOnly,
    /// Seeking is not possible.
    NoSeeking,
    /// Content will not be changed during the lifetime of the RTSP session.
    Immutable,
    /// Content might be changed.
    Dynamic,
    /// Accessible media range progresses with wallclock.
    TimeProgressing,
    /// Content will be available for the whole lifetime of the RTSP session.
    Unlimited,
    /// Content will be available at least until the specific wallclock time.
    TimeLimited(UtcTime),
    /// Content will be available for the specific duration.
    TimeDuration(f64),
    /// Supported scales.
    Scales(Vec<ScaleRange>),
    /// Extension.
    Extension(String, Option<String>),
}

impl fmt::Display for MediaProperty {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use fmt::Write;

        match self {
            MediaProperty::RandomAccess(Some(dur)) => write!(f, "Random-Access={}", dur),
            MediaProperty::RandomAccess(None) => f.write_str("Random-Access"),
            MediaProperty::BeginningOnly => f.write_str("Beginning-Only"),
            MediaProperty::NoSeeking => f.write_str("No-Seeking"),
            MediaProperty::Immutable => f.write_str("Immutable"),
            MediaProperty::Dynamic => f.write_str("Dynamic"),
            MediaProperty::TimeProgressing => f.write_str("Time-Progressing"),
            MediaProperty::Unlimited => f.write_str("Unlimited"),
            MediaProperty::TimeLimited(time) => write!(f, "Time-Limited={}", time),
            MediaProperty::TimeDuration(dur) => write!(f, "Time-Duration={}", dur),
            MediaProperty::Scales(scales) => {
                let mut s = String::new();
                for scale in scales {
                    if !s.is_empty() {
                        s.push_str(", ");
                    }
                    write!(&mut s, "{}", scale).unwrap();
                }
                write!(f, "Scales=\"{}\"", s)
            }
            MediaProperty::Extension(key, Some(value)) => write!(f, "{}={}", key, value),
            MediaProperty::Extension(key, None) => f.write_str(key),
        }
    }
}

/// Scale range.
#[derive(Debug, Clone, PartialEq)]
pub enum ScaleRange {
    Scale(f64),
    Range(f64, f64),
}

impl fmt::Display for ScaleRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScaleRange::Scale(scale) => write!(f, "{}", scale),
            ScaleRange::Range(a, b) => write!(f, "{}:{}", a, b),
        }
    }
}

impl std::ops::Deref for MediaProperties {
    type Target = Vec<MediaProperty>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for MediaProperties {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Vec<MediaProperty>> for MediaProperties {
    fn as_ref(&self) -> &Vec<MediaProperty> {
        &self.0
    }
}

impl AsMut<Vec<MediaProperty>> for MediaProperties {
    fn as_mut(&mut self) -> &mut Vec<MediaProperty> {
        &mut self.0
    }
}

impl From<Vec<MediaProperty>> for MediaProperties {
    fn from(v: Vec<MediaProperty>) -> Self {
        MediaProperties(v)
    }
}

impl<'a> From<&'a [MediaProperty]> for MediaProperties {
    fn from(v: &'a [MediaProperty]) -> Self {
        MediaProperties(v.to_vec())
    }
}

impl MediaProperties {
    /// Creates a new `Media-Properties` header builder.
    pub fn builder() -> MediaPropertiesBuilder {
        MediaPropertiesBuilder(Vec::new())
    }
}

/// Builder for the 'Media-Properties' header.
#[derive(Debug, Clone)]
pub struct MediaPropertiesBuilder(Vec<MediaProperty>);

impl MediaPropertiesBuilder {
    /// Add the provided media property to the `Media-Properties` header.
    pub fn property(mut self, property: MediaProperty) -> Self {
        self.0.push(property);
        self
    }

    /// Build the `Media-Properties` header.
    pub fn build(self) -> MediaProperties {
        MediaProperties(self.0)
    }
}

pub(super) mod parser {
    use super::*;

    // FIXME: Remove once str::split_once is stabilized
    fn split_once(s: &str, d: char) -> Option<(&str, &str)> {
        let idx = s.find(d)?;
        let (fst, snd) = s.split_at(idx);

        let (_, snd) = snd.split_at(snd.char_indices().nth(1).map(|(idx, _c)| idx).unwrap_or(1));

        Some((fst, snd))
    }

    use nom::branch::alt;
    use nom::bytes::complete::{tag, take_while};
    use nom::character::complete::space0;
    use nom::character::is_alphanumeric;
    use nom::combinator::{all_consuming, cond, flat_map, map, map_res, opt};
    use nom::multi::separated_list1;
    use nom::sequence::tuple;
    use nom::{Err, IResult, Needed};
    use std::str;

    fn token(input: &[u8]) -> IResult<&[u8], &[u8]> {
        fn is_token_char(i: u8) -> bool {
            is_alphanumeric(i) || b"!#$%&'*+-.^_`|~".contains(&i)
        }

        take_while(is_token_char)(input)
    }

    fn rtsp_unreserved(input: &[u8]) -> IResult<&[u8], &[u8]> {
        fn is_rtsp_unreserved_char(i: u8) -> bool {
            // rtsp_unreserved
            is_alphanumeric(i) || b"$-_.+!*'()".contains(&i)
        }

        take_while(is_rtsp_unreserved_char)(input)
    }
    fn quoted_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
        use std::num::NonZeroUsize;

        if !input.starts_with(b"\"") {
            return Err(Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }

        let i = &input[1..];
        let mut o = i;

        while !o.is_empty() {
            if o.len() >= 2 && o.starts_with(b"\\") {
                o = &o[2..];
            } else if o.starts_with(b"\\") {
                return Err(Err::Incomplete(Needed::Size(NonZeroUsize::new(1).unwrap())));
            } else if !o.starts_with(b"\"") {
                o = &o[1..];
            } else {
                // Closing quote, also include it
                o = &o[1..];
                break;
            }
        }

        let (fst, snd) = input.split_at(input.len() - o.len());
        // Did not end with a quote
        if !fst.ends_with(b"\"") {
            return Err(Err::Incomplete(Needed::Size(NonZeroUsize::new(1).unwrap())));
        }

        // Must have the starting quote
        assert!(fst.starts_with(b"\""));

        Ok((snd, fst))
    }

    fn param(input: &[u8]) -> IResult<&[u8], (&str, Option<&str>)> {
        if input.is_empty() {
            return Err(Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }

        map(
            tuple((
                space0,
                map_res(token, str::from_utf8),
                space0,
                flat_map(opt(tag(b"=")), |res| {
                    cond(
                        res.is_some(),
                        map_res(alt((quoted_string, rtsp_unreserved)), str::from_utf8),
                    )
                }),
                space0,
            )),
            |(_, name, _, value, _)| (name, value),
        )(input)
    }

    fn media_property(input: &[u8]) -> IResult<&[u8], MediaProperty> {
        map_res(param, |p| -> Result<_, HeaderParseError> {
            dbg!(&p);
            match p {
                ("Random-Access", None) => Ok(MediaProperty::RandomAccess(None)),
                ("Random-Access", Some(dur)) => {
                    let dur = dur.parse().map_err(|_| HeaderParseError)?;
                    Ok(MediaProperty::RandomAccess(Some(dur)))
                }
                ("Beginning-Only", None) => Ok(MediaProperty::BeginningOnly),
                ("No-Seeking", None) => Ok(MediaProperty::NoSeeking),
                ("Immutable", None) => Ok(MediaProperty::Immutable),
                ("Dynamic", None) => Ok(MediaProperty::Dynamic),
                ("Time-Progressing", None) => Ok(MediaProperty::TimeProgressing),
                ("Unlimited", None) => Ok(MediaProperty::Unlimited),
                ("Time-Limited", Some(time)) => {
                    let time = time.parse().map_err(|_| HeaderParseError)?;
                    Ok(MediaProperty::TimeLimited(time))
                }
                ("Time-Duration", Some(dur)) => {
                    let dur = dur.parse().map_err(|_| HeaderParseError)?;
                    Ok(MediaProperty::TimeDuration(dur))
                }
                ("Scales", Some(scales)) => {
                    if !scales.starts_with('"') || !scales.ends_with('"') {
                        return Err(HeaderParseError);
                    }

                    let mut s = Vec::new();
                    for scale in scales[1..(scales.len() - 1)].split(',') {
                        let scale = scale.trim();
                        if let Some((a, b)) = split_once(scale, ':') {
                            let a = a.parse().map_err(|_| HeaderParseError)?;
                            let b = b.parse().map_err(|_| HeaderParseError)?;
                            s.push(ScaleRange::Range(a, b));
                        } else {
                            let a = scale.parse().map_err(|_| HeaderParseError)?;
                            s.push(ScaleRange::Scale(a));
                        }
                    }

                    Ok(MediaProperty::Scales(s))
                }
                (key, value) => Ok(MediaProperty::Extension(
                    key.into(),
                    value.map(String::from),
                )),
            }
        })(input)
    }

    pub(crate) fn media_properties(input: &[u8]) -> IResult<&[u8], Vec<MediaProperty>> {
        all_consuming(separated_list1(tag(","), media_property))(input)
    }
}

impl super::TypedHeader for MediaProperties {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&MEDIA_PROPERTIES) {
            None => return Ok(None),
            Some(header) => header,
        };

        let (_rem, properties) =
            parser::media_properties(header.as_str().as_bytes()).map_err(|_| HeaderParseError)?;

        Ok(Some(properties.into()))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        let mut properties = String::new();
        for property in &self.0 {
            if !properties.is_empty() {
                properties.push_str(", ");
            }

            properties.push_str(&property.to_string());
        }

        headers.insert(MEDIA_PROPERTIES, properties);
    }
}

impl super::TypedAppendableHeader for MediaProperties {
    fn append_to(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        let mut properties = String::new();
        for property in &self.0 {
            if !properties.is_empty() {
                properties.push_str(", ");
            }

            properties.push_str(&property.to_string());
        }

        headers.append(MEDIA_PROPERTIES, properties);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_properties() {
        let header = "Random-Access=2.5, Unlimited, Immutable, Scales=\"-20, -10, -4, 0.5:1.5, 4, 8, 10, 15, 20\"";
        let response = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .header(crate::headers::MEDIA_PROPERTIES, header)
            .empty();

        let props = response
            .typed_header::<super::MediaProperties>()
            .unwrap()
            .unwrap();

        assert_eq!(
            &*props,
            &[
                MediaProperty::RandomAccess(Some(2.5)),
                MediaProperty::Unlimited,
                MediaProperty::Immutable,
                MediaProperty::Scales(vec![
                    ScaleRange::Scale(-20.0),
                    ScaleRange::Scale(-10.0),
                    ScaleRange::Scale(-4.0),
                    ScaleRange::Range(0.5, 1.5),
                    ScaleRange::Scale(4.0),
                    ScaleRange::Scale(8.0),
                    ScaleRange::Scale(10.0),
                    ScaleRange::Scale(15.0),
                    ScaleRange::Scale(20.0),
                ]),
            ]
        );

        let response2 = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .typed_header(&props)
            .empty();
        assert_eq!(response, response2);
    }
}
