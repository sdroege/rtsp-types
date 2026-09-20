// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

/// `Content-Type` header ([RFC 7826 section 18.19](https://tools.ietf.org/html/rfc7826#section-18.19)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ContentType {
    /// Media type.
    pub media_type: super::MediaType,
    /// Media subtype.
    pub media_subtype: String,
    /// Optional media parameters.
    pub params: Vec<(String, Option<String>)>,
}

impl super::TypedHeader for ContentType {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        use super::parser_helpers::split_once;

        let headers = headers.as_ref();

        let header = match headers.get(&CONTENT_TYPE) {
            None => return Ok(None),
            Some(header) => header,
        };

        let content_type = header.as_str();

        let (media_type, params_string) = match split_once(content_type, ';') {
            None => (content_type, None),
            Some((media_type, params_string)) => (media_type, Some(params_string)),
        };

        let params = match params_string {
            None => Vec::new(),
            Some(params_string) => {
                let mut params = Vec::new();

                for param in split_unquoted(params_string, b';') {
                    let param = param.trim();
                    if param.is_empty() {
                        return Err(HeaderParseError);
                    }

                    let (param_name, param_value) = match split_once(param, '=') {
                        Some((param_name, param_value)) => (param_name, Some(param_value)),
                        None => (param, None),
                    };

                    let param_name = param_name.trim();
                    if param_name.is_empty() || param_name.bytes().any(|b| !is_token_char(b)) {
                        return Err(HeaderParseError);
                    }

                    let param_value = match param_value {
                        None => None,
                        Some(value) if value.trim().starts_with('"') => {
                            Some(parse_quoted_string(value.trim())?)
                        }
                        Some(value) => {
                            let value = value.trim();
                            if value.is_empty() || value.bytes().any(|b| !is_token_char(b)) {
                                return Err(HeaderParseError);
                            }
                            Some(String::from(value))
                        }
                    };

                    params.push((String::from(param_name), param_value));
                }

                params
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
            content_type.push_str(&param.0);
            if let Some(ref value) = param.1 {
                content_type.push('=');
                if value.is_empty() || value.bytes().any(|b| !is_token_char(b)) {
                    write_quoted_string(&mut content_type, value);
                } else {
                    content_type.push_str(value);
                }
            }
        }

        headers.insert(CONTENT_TYPE, content_type);
    }
}

/// Checks whether `byte` is a valid token character ([RFC 7826 section
/// 18.19](https://tools.ietf.org/html/rfc7826#section-18.19)).
fn is_token_char(byte: u8) -> bool {
    matches!(
        byte,
        b'!'
            | b'#'..=b'&'
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'0'..=b'9'
            | b'A'..=b'Z'
            | b'^'..=b'z'
            | b'|'
            | b'~'
    )
}

/// Splits `s` on `separator`, ignoring separators inside quoted strings.
fn split_unquoted(s: &str, separator: u8) -> Vec<&str> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    let mut escaped = false;

    for (index, byte) in s.bytes().enumerate() {
        if in_quotes {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_quotes = false;
            }
        } else if byte == b'"' {
            in_quotes = true;
        } else if byte == separator {
            result.push(&s[start..index]);
            start = index + 1;
        }
    }

    result.push(&s[start..]);

    result
}

/// Parses a `quoted-string` value, stripping the surrounding quotes and
/// unescaping quoted pairs.
fn parse_quoted_string(s: &str) -> Result<String, HeaderParseError> {
    let bytes = s.as_bytes();

    if bytes.first() != Some(&b'"') {
        return Err(HeaderParseError);
    }

    let mut value = Vec::new();
    let mut escaped = false;
    let mut closed = false;

    for (index, byte) in bytes[1..].iter().enumerate() {
        if escaped {
            if *byte != b'"' && *byte != b'\\' {
                return Err(HeaderParseError);
            }
            value.push(*byte);
            escaped = false;
        } else if *byte == b'\\' {
            escaped = true;
        } else if *byte == b'"' {
            if index + 1 != bytes.len() - 1 {
                return Err(HeaderParseError);
            }
            closed = true;
            break;
        } else if !(byte == &b' '
            || byte == &b'!'
            || (b'#'..=b'[').contains(byte)
            || (b']'..=b'~').contains(byte))
        {
            return Err(HeaderParseError);
        } else {
            value.push(*byte);
        }
    }

    if !closed || escaped {
        return Err(HeaderParseError);
    }

    String::from_utf8(value).map_err(|_| HeaderParseError)
}

/// Writes `value` as a quoted string to `f`, escaping quotes and
/// backslashes.
fn write_quoted_string(f: &mut String, value: &str) {
    f.push('"');
    for char in value.chars() {
        if char == '"' || char == '\\' {
            f.push('\\');
        }
        f.push(char);
    }
    f.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string_params(pairs: &[(&str, &str)]) -> Vec<(String, Option<String>)> {
        pairs
            .iter()
            .map(|(name, value)| (String::from(*name), Some(String::from(*value))))
            .collect()
    }

    #[test]
    fn test_content_type() {
        let headers = [
            (
                "text/plain",
                ContentType {
                    media_type: MediaType::Text,
                    media_subtype: "plain".into(),
                    params: Vec::new(),
                },
                None,
            ),
            (
                "text/plain;foo=bar",
                ContentType {
                    media_type: MediaType::Text,
                    media_subtype: "plain".into(),
                    params: string_params(&[("foo", "bar")]),
                },
                None,
            ),
            (
                "text/plain;flag",
                ContentType {
                    media_type: MediaType::Text,
                    media_subtype: "plain".into(),
                    params: vec![("flag".into(), None)],
                },
                None,
            ),
            (
                "text/plain; charset=\"utf-8\"",
                ContentType {
                    media_type: MediaType::Text,
                    media_subtype: "plain".into(),
                    params: string_params(&[("charset", "utf-8")]),
                },
                Some("text/plain;charset=utf-8"),
            ),
            (
                "text/plain; foo = bar",
                ContentType {
                    media_type: MediaType::Text,
                    media_subtype: "plain".into(),
                    params: string_params(&[("foo", "bar")]),
                },
                Some("text/plain;foo=bar"),
            ),
            (
                "text/plain;charset=\"utf;8\"",
                ContentType {
                    media_type: MediaType::Text,
                    media_subtype: "plain".into(),
                    params: string_params(&[("charset", "utf;8")]),
                },
                None,
            ),
            (
                "text/plain;charset=\"utf-8\";profile=1",
                ContentType {
                    media_type: MediaType::Text,
                    media_subtype: "plain".into(),
                    params: string_params(&[("charset", "utf-8"), ("profile", "1")]),
                },
                Some("text/plain;charset=utf-8;profile=1"),
            ),
            (
                "text/plain;foo=\"bar\\\\\"",
                ContentType {
                    media_type: MediaType::Text,
                    media_subtype: "plain".into(),
                    params: string_params(&[("foo", "bar\\")]),
                },
                None,
            ),
        ];

        for (header, expected, serialized) in &headers {
            let request = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .header(crate::headers::CONTENT_TYPE, *header)
                .empty();

            let content_type = request
                .typed_header::<super::ContentType>()
                .unwrap_or_else(|_| panic!("couldn't parse {}", header))
                .unwrap();

            assert_eq!(content_type, *expected, "{header}");

            let request2 = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .typed_header(&content_type)
                .empty();

            let content_type = request2.header(&crate::headers::CONTENT_TYPE).unwrap();

            assert_eq!(content_type, serialized.unwrap_or(header), "{header}");
        }
    }

    #[test]
    fn test_content_type_invalid() {
        let headers = [
            "text/plain;=bar",
            "text/plain;foo=",
            "text/plain;foo bar=baz",
            "text/plain;foo=b ar",
            "text/plain;foo=\"bar",
            "text/plain;foo=\"bar\"baz",
            "text/plain;foo=bar;",
            "text/plain;foo=bar;;baz=qux",
        ];

        for header in &headers {
            let request = crate::Request::builder(crate::Method::Play, crate::Version::V2_0)
                .header(crate::headers::CONTENT_TYPE, *header)
                .empty();

            assert!(
                request.typed_header::<super::ContentType>().is_err(),
                "invalid header accepted: {}",
                header
            );
        }
    }
}
