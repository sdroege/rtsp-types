// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use std::fmt;

/// `Accept` header ([RFC 7826 section 18.1](https://tools.ietf.org/html/rfc7826#section-18.1)).
#[derive(Debug, Clone)]
pub struct Accept(Vec<MediaTypeRange>);

/// Media type range.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
            MediaType::Multipart => "Multipart",
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

    fn from_str(s: &str) -> Result<Self, HeaderParseError> {
        match s {
            "text" => Ok(MediaType::Text),
            "image" => Ok(MediaType::Image),
            "audio" => Ok(MediaType::Audio),
            "video" => Ok(MediaType::Video),
            "application" => Ok(MediaType::Application),
            "message" => Ok(MediaType::Message),
            "multipart" => Ok(MediaType::Multipart),
            _ => Ok(MediaType::Extension(String::from(s))),
        }
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
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        use super::parser_helpers::split_once;

        let headers = headers.as_ref();

        let header = match headers.get(&ACCEPT) {
            None => return Ok(None),
            Some(header) => header,
        };

        let mut media_types = Vec::new();
        for media_type_range in header.as_str().split(',') {
            let media_type_range = media_type_range.trim();

            let mut iter = media_type_range.split(';');
            let media_type = iter.next().ok_or(HeaderParseError)?.trim();
            let (media_type, media_subtype) =
                split_once(media_type, '/').ok_or(HeaderParseError)?;

            let media_type = if media_type == "*" {
                None
            } else {
                Some(media_type)
            };
            let media_subtype = if media_subtype == "*" {
                None
            } else {
                Some(media_subtype)
            };

            let mut params = Vec::new();
            for param in iter {
                let param = param.trim();
                if let Some((param, value)) = split_once(param, '=') {
                    params.push((String::from(param), Some(String::from(value))));
                } else {
                    params.push((String::from(param), None));
                }
            }

            media_types.push(MediaTypeRange {
                type_: media_type
                    .map(|s| s.parse())
                    .transpose()
                    .map_err(|_| HeaderParseError)?,
                subtype: media_subtype.map(String::from),
                params,
            });
        }

        Ok(Some(Accept(media_types)))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        use std::fmt::Write;

        let headers = headers.as_mut();

        let mut media_types = String::new();
        for media_type in &self.0 {
            if !media_types.is_empty() {
                media_types.push_str(", ");
            }

            if let Some(ref t) = media_type.type_ {
                write!(&mut media_types, "{}", t).unwrap();
            } else {
                media_types.push('*');
            }
            media_types.push('/');
            if let Some(ref t) = media_type.subtype {
                media_types.push_str(t);
            } else {
                media_types.push('*');
            }

            for param in &media_type.params {
                media_types.push(';');
                if let Some(ref value) = param.1 {
                    write!(&mut media_types, "{}={}", param.0, value).unwrap();
                } else {
                    media_types.push_str(&param.0);
                }
            }
        }

        headers.insert(ACCEPT, media_types);
    }
}

impl super::TypedAppendableHeader for Accept {
    fn append_to(&self, mut headers: impl AsMut<Headers>) {
        use std::fmt::Write;

        let headers = headers.as_mut();

        let mut media_types = String::new();
        for media_type in &self.0 {
            if !media_types.is_empty() {
                media_types.push_str(", ");
            }

            if let Some(ref t) = media_type.type_ {
                write!(&mut media_types, "{}", t).unwrap();
            } else {
                media_types.push('*');
            }
            media_types.push('/');
            if let Some(ref t) = media_type.subtype {
                media_types.push_str(t);
            } else {
                media_types.push('*');
            }

            for param in &media_type.params {
                media_types.push(';');
                if let Some(ref value) = param.1 {
                    write!(&mut media_types, "{}={}", param.0, value).unwrap();
                } else {
                    media_types.push_str(&param.0);
                }
            }
        }

        headers.append(ACCEPT, media_types);
    }
}
