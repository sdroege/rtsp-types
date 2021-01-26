// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

/// `Content-Type` header ([RFC 7826 section 18.19](https://tools.ietf.org/html/rfc7826#section-18.19)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ContentType {
    /// Media type.
    pub media_type: super::MediaType,
    /// Media subtype.
    pub media_subtype: String,
    /// Optional media parameters.
    pub params: Vec<(String, Option<String>)>,
}

// FIXME: Remove once str::split_once is stabilized
fn split_once(s: &str, d: char) -> Option<(&str, &str)> {
    let idx = s.find(d)?;
    let (fst, snd) = s.split_at(idx);

    let (_, snd) = snd.split_at(snd.char_indices().nth(1).map(|(idx, _c)| idx).unwrap_or(1));

    Some((fst, snd))
}

impl super::TypedHeader for ContentType {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&CONTENT_TYPE) {
            None => return Ok(None),
            Some(header) => header,
        };

        let content_type = header.as_str();

        let (media_type, params) = match split_once(content_type, ';') {
            None => (content_type, Vec::new()),
            Some((media_type, params_string)) => {
                let mut params = Vec::new();
                for param in params_string.split(';') {
                    let param = param.trim();
                    if let Some((param, value)) = split_once(param, '=') {
                        params.push((String::from(param), Some(String::from(value))));
                    } else {
                        params.push((String::from(param), None));
                    }
                }

                (media_type, params)
            }
        };

        let (media_type, media_subtype) = split_once(media_type, '/').ok_or(HeaderParseError)?;
        let media_type = media_type.parse().map_err(|_| HeaderParseError)?;

        Ok(Some(ContentType {
            media_type,
            media_subtype: media_subtype.into(),
            params,
        }))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        use std::fmt::Write;

        let headers = headers.as_mut();

        let mut content_type = String::new();
        write!(
            &mut content_type,
            "{}/{}",
            self.media_type, self.media_subtype
        )
        .unwrap();

        for param in &self.params {
            content_type.push(';');
            if let Some(ref value) = param.1 {
                write!(&mut content_type, "{}={}", param.0, value).unwrap();
            } else {
                content_type.push_str(&param.0);
            }
        }

        headers.insert(CONTENT_TYPE, content_type);
    }
}
