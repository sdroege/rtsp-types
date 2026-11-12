// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

pub(crate) mod error;

use super::headers::parser_helpers::parse_digits;
use super::*;
use crate::parser::error::{
    Error, Input, MsgError, Result, EXPECTED_CHARACTER, EXPECTED_LITERAL, EXPECTED_NON_EMPTY,
    EXPECTED_ONE_OF, EXPECTED_VALUE,
};
use std::str;
use tinyvec::TinyVec;
use winnow::ascii::space0;
use winnow::binary::be_u16;
use winnow::combinator::{alt, cut_err, repeat, terminated};
use winnow::error::{ErrMode, Needed};
use winnow::token::{any, literal, one_of, take, take_until, take_while};
use winnow::Parser;

fn is_token_char(i: u8) -> bool {
    i.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&i)
}

fn token<'i>(input: &mut Input<'i>) -> Result<'i, &'i [u8]> {
    take_while(0.., is_token_char).parse_next(input)
}

fn token_1<'i>(input: &mut Input<'i>) -> Result<'i, &'i [u8]> {
    take_while(1.., is_token_char)
        .context(EXPECTED_NON_EMPTY)
        .parse_next(input)
}

// Converts a parsed byte slice into a string
fn utf8<'i>(bytes: &'i [u8]) -> Result<'i, &'i str> {
    str::from_utf8(bytes).map_err(|_| ErrMode::Backtrack(MsgError::at(bytes, EXPECTED_VALUE)))
}

// Matches and consumes one space
fn sp<'i>(input: &mut Input<'i>) -> Result<'i, ()> {
    literal(b' ')
        .context(EXPECTED_CHARACTER)
        .void()
        .parse_next(input)
}

// Matches and consumes a CRLF
fn crlf<'i>(input: &mut Input<'i>) -> Result<'i, ()> {
    literal(b"\r\n")
        .context(EXPECTED_LITERAL)
        .void()
        .parse_next(input)
}

// Parses an RTSP version, only accepts 1.0 and 2.0
fn rtsp_version<'i>(input: &mut Input<'i>) -> Result<'i, Version> {
    (
        literal(b"RTSP/").context(EXPECTED_LITERAL),
        one_of(b"12").context(EXPECTED_ONE_OF),
        literal(b".0").context(EXPECTED_LITERAL),
    )
        .map(|(_, major, _)| {
            if major == b'2' {
                Version::V2_0
            } else {
                Version::V1_0
            }
        })
        .parse_next(input)
}

// Matches a run of printable ASCII characters
fn vchar_1<'i>(input: &mut Input<'i>) -> Result<'i, &'i [u8]> {
    take_while(1.., |i: u8| i > 32 && i <= 126)
        .context(EXPECTED_NON_EMPTY)
        .parse_next(input)
}

fn request_line<'i>(input: &mut Input<'i>) -> Result<'i, RequestLine<'i>> {
    let method = token_1(input)?;
    let method = MethodRef::from(utf8(method)?);
    sp(input)?;
    let request_uri = alt((
        literal(b'*').context(EXPECTED_CHARACTER).map(|_| None),
        vchar_1.map(Some),
    ))
    .parse_next(input)?;
    let request_uri = request_uri.map(utf8).transpose()?;
    sp(input)?;
    let version = rtsp_version(input)?;
    crlf(input)?;

    Ok(RequestLine {
        method,
        request_uri,
        version,
    })
}

fn status_code<'i>(input: &mut Input<'i>) -> Result<'i, u16> {
    let digits = take_while(3..=3, |i: u8| i.is_ascii_digit()).parse_next(input)?;
    let digits = utf8(digits)?;

    digits
        .parse::<u16>()
        .map_err(|_| ErrMode::Backtrack(MsgError::at(digits.as_bytes(), EXPECTED_VALUE)))
}

fn status_line<'i>(input: &mut Input<'i>) -> Result<'i, StatusLine<'i>> {
    let version = rtsp_version(input)?;
    sp(input)?;
    let status = status_code(input)?;
    sp(input)?;
    let reason_phrase = take_until(0.., &b"\r\n"[..]).parse_next(input)?;
    let reason_phrase = utf8(reason_phrase)?;
    crlf(input)?;

    Ok(StatusLine {
        version,
        status: status.into(),
        reason_phrase,
    })
}

fn header_value<'i>(input: &mut Input<'i>) -> Result<'i, &'i [u8]> {
    // Header values can be split over multiple lines, in which case there
    // will be a CRLF followed by one or more spaces/tabs.
    let start = input.into_inner();
    let mut o = start;
    while !o.is_empty() {
        if o.len() >= 3 && o.starts_with(b"\r\n") && (o[2] == b' ' || o[2] == b'\t') {
            if let Some((no_space_pos, _)) = o
                .iter()
                .enumerate()
                .skip(3)
                .find(|(_, b)| **b != b' ' && **b != b'\t')
            {
                // Header continues on the next line
                o = &o[no_space_pos..];
            } else {
                // Incomplete
                o = &[];
            }
        } else if !o.starts_with(b"\r\n") {
            // Normal header character
            o = &o[1..];
        } else {
            // Not a header character
            let (res, rem) = start.split_at(start.len() - o.len());
            let res = res.trim_ascii();

            *input = Input::new(rem);

            return Ok(res);
        }
    }

    Err(ErrMode::Incomplete(Needed::Unknown))
}

fn message_header<'i>(input: &mut Input<'i>) -> Result<'i, HeaderRef<'i>> {
    let name = token(input)?;
    let name = utf8(name)?;
    literal(b':')
        .context(EXPECTED_CHARACTER)
        .parse_next(input)?;
    space0(input)?;
    let value = header_value(input)?;
    let value = utf8(value)?;
    crlf(input)?;

    Ok(HeaderRef { name, value })
}

fn headers<'i>(input: &mut Input<'i>) -> Result<'i, TinyVec<[HeaderRef<'i>; 16]>> {
    terminated(
        repeat(0.., message_header).fold(
            TinyVec::new,
            |mut acc: TinyVec<[HeaderRef<'i>; 16]>, h| {
                acc.push(h);
                acc
            },
        ),
        crlf,
    )
    .parse_next(input)
}

fn content_length<'a>(headers: &[HeaderRef<'a>]) -> core::result::Result<usize, Error<'a>> {
    let mut value = None;

    for h in headers
        .iter()
        .filter(|h| &h.name.to_ascii_uppercase() == "CONTENT-LENGTH")
    {
        // A sender MUST NOT generate multiple Content-Length header fields with
        // non-identical values, in which case the message is malformed.
        let v = parse_digits::<usize>(h.value)
            .map_err(|_| ErrMode::Cut(MsgError::at(h.value.as_bytes(), EXPECTED_VALUE)))?;

        match value {
            None => value = Some(v),
            Some(prev) if prev != v => {
                return Err(ErrMode::Cut(MsgError::at(
                    h.value.as_bytes(),
                    EXPECTED_VALUE,
                )))
            }
            Some(_) => (),
        }
    }

    Ok(value.unwrap_or(0))
}

fn request<'i>(input: &mut Input<'i>) -> Result<'i, RequestRef<'i>> {
    let request_line = request_line(input)?;
    let headers = headers(input)?;
    let content_length = content_length(&headers)?;
    let body = take(content_length).parse_next(input)?;

    Ok(RequestRef {
        method: request_line.method,
        request_uri: request_line.request_uri,
        version: request_line.version,
        headers,
        body,
    })
}

fn response<'i>(input: &mut Input<'i>) -> Result<'i, ResponseRef<'i>> {
    let status_line = status_line(input)?;
    let headers = headers(input)?;
    let content_length = content_length(&headers)?;
    let body = take(content_length).parse_next(input)?;

    Ok(ResponseRef {
        version: status_line.version,
        status: status_line.status,
        reason_phrase: status_line.reason_phrase,
        headers,
        body,
    })
}

fn data<'i>(input: &mut Input<'i>) -> Result<'i, DataRef<'i>> {
    (
        literal(b'$').context(EXPECTED_CHARACTER),
        any,
        be_u16.flat_map(|len: u16| take(len as usize)),
    )
        .map(|(_, channel_id, body): (_, u8, _)| DataRef { channel_id, body })
        .parse_next(input)
}

pub(crate) fn message<'i>(input: &mut Input<'i>) -> Result<'i, MessageRef<'i>> {
    repeat(0.., crlf)
        .fold(|| (), |(), ()| ())
        .parse_next(input)?;

    if input.into_inner().is_empty() {
        return Err(ErrMode::Incomplete(Needed::Unknown));
    }

    // The message type is determined by the first byte(s), so errors can
    // never be recovered from and must not be replaced by errors from
    // another message type.
    if input.into_inner().starts_with(b"$") {
        Ok(MessageRef::Data(cut_err(data).parse_next(input)?))
    } else if input.into_inner().starts_with(b"RTSP/") {
        Ok(MessageRef::Response(cut_err(response).parse_next(input)?))
    } else {
        Ok(MessageRef::Request(cut_err(request).parse_next(input)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use tinyvec::tiny_vec;

    fn parse<'i, O>(
        mut parser: impl Parser<Input<'i>, O, Error<'i>>,
        input: &'i [u8],
    ) -> Result<'i, (&'i [u8], O)> {
        parser
            .parse_peek(Input::new(input))
            .map(|(rest, res)| (rest.into_inner(), res))
    }

    #[test]
    fn test_request_line() {
        assert_eq!(
            parse(
                request_line,
                b"OPTIONS rtsp://media.example.com/movie/twister.3gp RTSP/2.0\r\n"
            ),
            Ok((
                &b""[..],
                RequestLine {
                    method: MethodRef::Options,
                    request_uri: Some("rtsp://media.example.com/movie/twister.3gp"),
                    version: Version::V2_0,
                }
            ))
        );
    }

    #[test]
    fn test_request_line_without_uri() {
        assert_eq!(
            parse(request_line, b"OPTIONS * RTSP/2.0\r\n"),
            Ok((
                &b""[..],
                RequestLine {
                    method: MethodRef::Options,
                    request_uri: None,
                    version: Version::V2_0,
                }
            ))
        );
    }

    #[test]
    fn test_request_line_with_empty_method() {
        assert!(parse(request_line, b" rtsp://media.example.com RTSP/2.0\r\n").is_err());
    }

    #[test]
    fn test_request_line_with_empty_uri() {
        assert!(parse(request_line, b"OPTIONS  RTSP/2.0\r\n").is_err());
    }

    #[test]
    fn test_status_line() {
        assert_eq!(
            parse(status_line, b"RTSP/2.0 200 All Good\r\n"),
            Ok((
                &b""[..],
                StatusLine {
                    version: Version::V2_0,
                    status: StatusCode::Ok,
                    reason_phrase: "All Good",
                }
            ))
        );
    }

    #[test]
    fn test_options() {
        assert_eq!(
            parse(
                request,
                b"OPTIONS rtsp://media.example.com/movie/twister.3gp RTSP/2.0\r\n\
        CSeq: 1\r\n\
        Supported: play.basic, play.scale\r\n\
        User-Agent: PhonyClient/1.2\r\n\
        \r\n\
        REMAINDER"
            ),
            Ok((
                &b"REMAINDER"[..],
                RequestRef {
                    method: MethodRef::Options,
                    version: Version::V2_0,
                    request_uri: Some("rtsp://media.example.com/movie/twister.3gp"),
                    headers: tiny_vec!(
                        HeaderRef {
                            name: "CSeq",
                            value: "1"
                        },
                        HeaderRef {
                            name: "Supported",
                            value: "play.basic, play.scale"
                        },
                        HeaderRef {
                            name: "User-Agent",
                            value: "PhonyClient/1.2"
                        }
                    ),
                    body: &[],
                }
            ))
        );
    }

    #[test]
    fn test_options_multiline_header() {
        assert_eq!(
            parse(
                request,
                b"OPTIONS rtsp://media.example.com/movie/twister.3gp RTSP/2.0\r\n\
CSeq: 1\r\n\
Supported: play.basic,\r\n play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
\r\n\
REMAINDER"
            )
            .map(|(rem, req)| (rem, RequestRef::to_owned(&req).unwrap())),
            Ok((
                &b"REMAINDER"[..],
                Request::builder(Method::Options, Version::V2_0)
                    .request_uri(Url::parse("rtsp://media.example.com/movie/twister.3gp").unwrap())
                    .header(crate::headers::CSEQ, "1")
                    .header(crate::headers::SUPPORTED, "play.basic, play.scale")
                    .header(crate::headers::USER_AGENT, "PhonyClient/1.2")
                    .build(vec![])
            ))
        );
    }

    #[test]
    fn test_trailing_whitespace() {
        assert_eq!(
            parse(
                request,
                b"OPTIONS rtsp://media.example.com/movie/twister.3gp RTSP/2.0\r\n\
CSeq: 1 \r\n\
Supported: \tplay.basic,\r\n play.scale\r\n\
User-Agent: PhonyClient/1.2\t\r\n\
\r\n\
REMAINDER"
            )
            .map(|(rem, req)| (rem, RequestRef::to_owned(&req).unwrap())),
            Ok((
                &b"REMAINDER"[..],
                Request::builder(Method::Options, Version::V2_0)
                    .request_uri(Url::parse("rtsp://media.example.com/movie/twister.3gp").unwrap())
                    .header(crate::headers::CSEQ, "1")
                    .header(crate::headers::SUPPORTED, "play.basic, play.scale")
                    .header(crate::headers::USER_AGENT, "PhonyClient/1.2")
                    .build(vec![])
            ))
        );
    }

    #[test]
    fn test_options_without_uri() {
        assert_eq!(
            parse(
                request,
                b"OPTIONS * RTSP/2.0\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
\r\n\
REMAINDER"
            ),
            Ok((
                &b"REMAINDER"[..],
                RequestRef {
                    method: MethodRef::Options,
                    version: Version::V2_0,
                    request_uri: None,
                    headers: tiny_vec!(
                        HeaderRef {
                            name: "CSeq",
                            value: "1"
                        },
                        HeaderRef {
                            name: "Supported",
                            value: "play.basic, play.scale"
                        },
                        HeaderRef {
                            name: "User-Agent",
                            value: "PhonyClient/1.2"
                        }
                    ),
                    body: &[],
                }
            ))
        );
    }

    #[test]
    fn test_options_with_body() {
        assert_eq!(
            parse(
                request,
                b"OPTIONS rtsp://media.example.com/movie/twister.3gp RTSP/2.0\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
Content-Length: 10\r\n\
\r\n\
0123456789\
REMAINDER"
            ),
            Ok((
                &b"REMAINDER"[..],
                RequestRef {
                    method: MethodRef::Options,
                    version: Version::V2_0,
                    request_uri: Some("rtsp://media.example.com/movie/twister.3gp"),
                    headers: tiny_vec!(
                        HeaderRef {
                            name: "CSeq",
                            value: "1"
                        },
                        HeaderRef {
                            name: "Supported",
                            value: "play.basic, play.scale"
                        },
                        HeaderRef {
                            name: "User-Agent",
                            value: "PhonyClient/1.2"
                        },
                        HeaderRef {
                            name: "Content-Length",
                            value: "10"
                        }
                    ),
                    body: &b"0123456789"[..],
                }
            ))
        );
    }

    #[test]
    fn test_options_with_body_without_uri() {
        assert_eq!(
            parse(
                request,
                b"OPTIONS * RTSP/2.0\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
Content-Length: 10\r\n\
\r\n\
0123456789\
REMAINDER"
            ),
            Ok((
                &b"REMAINDER"[..],
                RequestRef {
                    method: MethodRef::Options,
                    version: Version::V2_0,
                    request_uri: None,
                    headers: tiny_vec![
                        HeaderRef {
                            name: "CSeq",
                            value: "1"
                        },
                        HeaderRef {
                            name: "Supported",
                            value: "play.basic, play.scale"
                        },
                        HeaderRef {
                            name: "User-Agent",
                            value: "PhonyClient/1.2"
                        },
                        HeaderRef {
                            name: "Content-Length",
                            value: "10"
                        }
                    ],
                    body: &b"0123456789"[..],
                }
            ))
        );
    }

    #[test]
    fn test_ok() {
        assert_eq!(
            parse(
                response,
                b"RTSP/2.0 200 All Good\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
\r\n\
REMAINDER"
            ),
            Ok((
                &b"REMAINDER"[..],
                ResponseRef {
                    version: Version::V2_0,
                    status: StatusCode::Ok,
                    reason_phrase: "All Good",
                    headers: tiny_vec![
                        HeaderRef {
                            name: "CSeq",
                            value: "1"
                        },
                        HeaderRef {
                            name: "Supported",
                            value: "play.basic, play.scale"
                        },
                        HeaderRef {
                            name: "User-Agent",
                            value: "PhonyClient/1.2"
                        }
                    ],
                    body: &[],
                }
            ))
        );
    }

    #[test]
    fn test_ok_with_body() {
        assert_eq!(
            parse(
                response,
                b"RTSP/2.0 200 All Good\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
Content-Length: 10\r\n\
\r\n\
0123456789\
REMAINDER"
            ),
            Ok((
                &b"REMAINDER"[..],
                ResponseRef {
                    version: Version::V2_0,
                    status: StatusCode::Ok,
                    reason_phrase: "All Good",
                    headers: tiny_vec!(
                        HeaderRef {
                            name: "CSeq",
                            value: "1"
                        },
                        HeaderRef {
                            name: "Supported",
                            value: "play.basic, play.scale"
                        },
                        HeaderRef {
                            name: "User-Agent",
                            value: "PhonyClient/1.2"
                        },
                        HeaderRef {
                            name: "Content-Length",
                            value: "10"
                        }
                    ),
                    body: &b"0123456789"[..],
                }
            ))
        );
    }

    #[test]
    fn test_bad_content_length() {
        assert!(matches!(
            parse(
                response,
                b"RTSP/2.0 200 All Good\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
Content-Length: bad\r\n\
\r\n\
0123456789\
REMAINDER"
            ),
            Err(ErrMode::Cut(_))
        ));
    }

    #[test]
    fn test_duplicate_content_length() {
        // Identical duplicates are fine.
        assert!(parse(
            response,
            b"RTSP/2.0 200 All Good\r\n\
Content-Length: 10\r\n\
Content-Length: 10\r\n\
\r\n\
0123456789\
REMAINDER"
        )
        .is_ok());

        // Differing duplicates make the message malformed.
        assert!(matches!(
            parse(
                response,
                b"RTSP/2.0 200 All Good\r\n\
Content-Length: 10\r\n\
Content-Length: 5\r\n\
\r\n\
0123456789\
REMAINDER"
            ),
            Err(ErrMode::Cut(_))
        ));
    }

    #[test]
    fn test_data() {
        assert_eq!(
            parse(
                data,
                &[b'$', 12, 0, 10, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, b'a', b'b'][..]
            ),
            Ok((
                &b"ab"[..],
                DataRef::from_slice(12, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
            ))
        );
    }

    #[test]
    fn test_options_with_invalid_content_length() {
        assert!(matches!(
            parse(
                request,
                b"OPTIONS rtsp://media.example.com/movie/twister.3gp RTSP/2.0\r\n\
 CSeq: 1\r\n\
 Content-Length: +10\r\n\
 \r\n\
 0123456789"
            ),
            Err(ErrMode::Cut(_))
        ));
    }
}
