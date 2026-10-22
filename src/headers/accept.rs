// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::parser_helpers;
use super::*;

use std::fmt;

/// `Accept` header ([RFC 7826 section 18.1](https://tools.ietf.org/html/rfc7826#section-18.1)).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Accept(Vec<MediaTypeRange>);

/// Media type range.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MediaTypeRange {
    /// Media type.
    ///
    /// `None` for `*`.
    pub type_: Option<MediaType>,
    /// Optional media sub-type.
    ///
    /// `None` for `*`.
    pub subtype: Option<String>,
    /// Media type parameters.
    pub params: Vec<(String, Option<String>)>,
}

/// Media type.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MediaType {
    Text,
    Image,
    Audio,
    Video,
    Application,
    Message,
    Multipart,
    Extension(String),
}

impl MediaType {
    pub fn as_str(&self) -> &str {
        match self {
            MediaType::Text => "text",
            MediaType::Image => "image",
            MediaType::Audio => "audio",
            MediaType::Video => "video",
            MediaType::Application => "application",
            MediaType::Message => "message",
            MediaType::Multipart => "multipart",
            MediaType::Extension(ref s) => s.as_str(),
        }
    }
}

impl fmt::Display for MediaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for MediaType {
    type Err = HeaderParseError;

    /// Media types are case-insensitive, unknown ones are kept as-is.
    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        let media_type = match s.to_ascii_lowercase().as_str() {
            "text" => MediaType::Text,
            "image" => MediaType::Image,
            "audio" => MediaType::Audio,
            "video" => MediaType::Video,
            "application" => MediaType::Application,
            "message" => MediaType::Message,
            "multipart" => MediaType::Multipart,
            _ => return Ok(MediaType::Extension(String::from(s))),
        };

        Ok(media_type)
    }
}

impl std::ops::Deref for Accept {
    type Target = Vec<MediaTypeRange>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Accept {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Vec<MediaTypeRange>> for Accept {
    fn as_ref(&self) -> &Vec<MediaTypeRange> {
        &self.0
    }
}

impl AsMut<Vec<MediaTypeRange>> for Accept {
    fn as_mut(&mut self) -> &mut Vec<MediaTypeRange> {
        &mut self.0
    }
}

impl From<Vec<MediaTypeRange>> for Accept {
    fn from(v: Vec<MediaTypeRange>) -> Self {
        Accept(v)
    }
}

impl<'a> From<&'a [MediaTypeRange]> for Accept {
    fn from(v: &'a [MediaTypeRange]) -> Self {
        Accept(v.to_vec())
    }
}

impl Accept {
    /// Creates a new `Accept` header builder.
    pub fn builder() -> AcceptBuilder {
        AcceptBuilder(Vec::new())
    }
}

/// Builder for the 'Accept' header.
#[derive(Debug, Clone)]
pub struct AcceptBuilder(Vec<MediaTypeRange>);

impl AcceptBuilder {
    /// Add the provided media type to the `Accept` header.
    pub fn media_type(mut self, media_type: MediaTypeRange) -> Self {
        self.0.push(media_type);
        self
    }

    /// Build the `Accept` header.
    pub fn build(self) -> Accept {
        Accept(self.0)
    }
}

impl super::TypedHeader for Accept {
    fn header_name() -> Option<HeaderName> {
        Some(ACCEPT.clone())
    }
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&ACCEPT) {
            None => return Ok(None),
            Some(header) => header,
        };

        let mut media_types = Vec::new();
        for media_type_range in parser_helpers::split_unquoted(header.as_str(), b',') {
            let mut params_iter =
                parser_helpers::split_unquoted(media_type_range, b';').into_iter();
            let media_type_range = params_iter.next().ok_or_else(HeaderParseError::new)?.trim();

            if media_type_range.is_empty() {
                continue;
            }

            let (media_type, media_subtype) = media_type_range
                .split_once('/')
                .ok_or_else(HeaderParseError::new)?;
            let media_type = media_type.trim();
            let media_subtype = media_subtype.trim();

            let media_type = if media_type == "*" {
                None
            } else if media_type.is_empty() {
                return Err(HeaderParseError::new());
            } else {
                Some(
                    media_type
                        .parse::<MediaType>()
                        .map_err(|_| HeaderParseError::new())?,
                )
            };
            let media_subtype = if media_subtype == "*" {
                None
            } else if media_subtype.is_empty() {
                return Err(HeaderParseError::new());
            } else {
                Some(String::from(media_subtype))
            };

            let mut params = Vec::new();
            for param in params_iter {
                let param = param.trim();
                if param.is_empty() {
                    return Err(HeaderParseError::new());
                }

                let (param_name, param_value) = match param.split_once('=') {
                    Some((param_name, param_value)) => (param_name, Some(param_value)),
                    None => (param, None),
                };

                let param_name = param_name.trim();
                if param_name.is_empty()
                    || param_name
                        .bytes()
                        .any(|b| !parser_helpers::is_token_char(b))
                {
                    return Err(HeaderParseError::new());
                }

                let param_value = match param_value {
                    None => None,
                    Some(value) if value.trim().starts_with('"') => {
                        Some(parser_helpers::parse_quoted_string(value.trim())?)
                    }
                    Some(value) => {
                        let value = value.trim();
                        if value.is_empty()
                            || value.bytes().any(|b| !parser_helpers::is_token_char(b))
                        {
                            return Err(HeaderParseError::new());
                        }
                        Some(String::from(value))
                    }
                };

                params.push((String::from(param_name), param_value));
            }

            media_types.push(MediaTypeRange {
                type_: media_type,
                subtype: media_subtype,
                params,
            });
        }

        Ok(Some(Accept(media_types)))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        if self.0.is_empty() {
            // An empty list cannot be represented. Remove any existing
            // header instead of writing an empty header value.
            headers.remove(&ACCEPT);
            return;
        }

        headers.insert(ACCEPT, format_media_type_ranges(&self.0));
    }
}

impl super::TypedAppendableHeader for Accept {
    fn append_to(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        if self.0.is_empty() {
            return;
        }

        headers.append(ACCEPT, format_media_type_ranges(&self.0));
    }
}

fn format_media_type_ranges(media_types: &[MediaTypeRange]) -> String {
    use std::fmt::Write;

    let mut result = String::new();
    for media_type in media_types {
        if !result.is_empty() {
            result.push_str(", ");
        }

        if let Some(ref t) = media_type.type_ {
            write!(&mut result, "{t}").unwrap();
        } else {
            result.push('*');
        }
        result.push('/');
        if let Some(ref t) = media_type.subtype {
            result.push_str(t);
        } else {
            result.push('*');
        }

        for (name, value) in &media_type.params {
            result.push(';');
            result.push_str(name);
            if let Some(ref value) = value {
                result.push('=');
                if value.is_empty() || value.bytes().any(|b| !parser_helpers::is_token_char(b)) {
                    parser_helpers::write_quoted_string(&mut result, value);
                } else {
                    result.push_str(value);
                }
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_type_multipart_roundtrip() {
        let media_type: MediaType = "multipart".parse().unwrap();

        assert_eq!(media_type, MediaType::Multipart);
        assert_eq!(media_type.as_str(), "multipart");
    }

    #[test]
    fn test_accept_multipart_roundtrip() {
        let mut test_headers = Headers::new();
        test_headers.insert(ACCEPT, "multipart/*");
        let accept = Accept::from_headers(&test_headers)
            .expect("should not error")
            .expect("should be Some");

        let mut out_headers = Headers::new();
        accept.insert_into(&mut out_headers);

        assert_eq!(
            out_headers.get(&ACCEPT).expect("should be Some").as_str(),
            "multipart/*"
        );
    }

    #[test]
    fn test_insert_into_empty_list() {
        let accept = Accept(Vec::new());

        let mut test_headers = Headers::new();
        test_headers.insert(ACCEPT, "video/*");
        accept.insert_into(&mut test_headers);

        assert_eq!(test_headers.get(&ACCEPT), None);
    }

    #[test]
    fn test_append_to_empty_list() {
        let accept = Accept(Vec::new());

        let mut test_headers = Headers::new();
        test_headers.insert(ACCEPT, "video/*");
        accept.append_to(&mut test_headers);

        assert_eq!(
            test_headers.get(&ACCEPT).expect("should be Some").as_str(),
            "video/*"
        );
    }

    fn parse_accept(header: &str) -> Accept {
        let mut headers = Headers::new();
        headers.insert(ACCEPT, header);
        Accept::from_headers(&headers).unwrap().unwrap()
    }

    fn range(
        type_: MediaType,
        subtype: Option<&str>,
        params: &[(&str, Option<&str>)],
    ) -> MediaTypeRange {
        MediaTypeRange {
            type_: Some(type_),
            subtype: subtype.map(String::from),
            params: params
                .iter()
                .map(|(name, value)| (String::from(*name), value.map(String::from)))
                .collect(),
        }
    }

    #[test]
    fn test_media_type_case_insensitive() {
        assert_eq!("AUDIO".parse::<MediaType>().unwrap(), MediaType::Audio);
        assert_eq!(
            "Multipart".parse::<MediaType>().unwrap(),
            MediaType::Multipart
        );
        assert_eq!(
            "X-Custom".parse::<MediaType>().unwrap(),
            MediaType::Extension("X-Custom".into())
        );
    }

    #[test]
    fn test_accept_parsing() {
        let cases = [
            (
                "AUDIO/*;q=0.5, Video/Mpeg4",
                vec![
                    range(MediaType::Audio, None, &[("q", Some("0.5"))]),
                    range(MediaType::Video, Some("Mpeg4"), &[]),
                ],
                "audio/*;q=0.5, video/Mpeg4",
            ),
            (
                "*/*",
                vec![MediaTypeRange {
                    type_: None,
                    subtype: None,
                    params: Vec::new(),
                }],
                "*/*",
            ),
            (
                "audio/x-pn-realaudio;Params=\"a;b=c\";flag",
                vec![range(
                    MediaType::Audio,
                    Some("x-pn-realaudio"),
                    &[("Params", Some("a;b=c")), ("flag", None)],
                )],
                "audio/x-pn-realaudio;Params=\"a;b=c\";flag",
            ),
            (
                "audio/x;foo=\"a,b\",video/*",
                vec![
                    range(MediaType::Audio, Some("x"), &[("foo", Some("a,b"))]),
                    range(MediaType::Video, None, &[]),
                ],
                "audio/x;foo=\"a,b\", video/*",
            ),
            (
                "audio/x;foo=\"a\\\"b\"",
                vec![range(MediaType::Audio, Some("x"), &[("foo", Some("a\"b"))])],
                "audio/x;foo=\"a\\\"b\"",
            ),
        ];

        for (header, expected, serialized) in &cases {
            let accept = parse_accept(header);

            assert_eq!(&*accept, expected, "{}", header);

            let mut out_headers = Headers::new();
            accept.insert_into(&mut out_headers);

            assert_eq!(
                out_headers.get(&ACCEPT).expect("should be Some").as_str(),
                *serialized,
                "{}",
                header
            );

            assert_eq!(&*parse_accept(serialized), expected, "{}", serialized);
        }
    }

    #[test]
    fn test_accept_invalid() {
        let headers = [
            "audio",
            "/mpeg",
            "audio/",
            "audio/*;foo=\"unterminated",
            "audio/*;foo=a b",
            "audio/*;foo=",
            "audio/*;=bar",
            "audio/*;",
        ];

        for header in &headers {
            assert!(
                parse_accept_err(header).is_err(),
                "invalid header accepted: {}",
                header
            );
        }
    }

    fn parse_accept_err(header: &str) -> Result<Option<Accept>, HeaderParseError> {
        let mut headers = Headers::new();
        headers.insert(ACCEPT, header);
        Accept::from_headers(&headers)
    }
}
