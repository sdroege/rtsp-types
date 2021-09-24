// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::nom_extensions::many0_tinyvec;
use super::*;
use nom::branch::alt;
use nom::bytes::streaming::{tag, take, take_until, take_while, take_while_m_n};
use nom::character::streaming::{char, one_of};
use nom::character::{is_alphanumeric, is_digit, is_space};
use nom::combinator::{flat_map, map, map_res, opt, value};
use nom::multi::fold_many0;
use nom::number::streaming::be_u16;
use nom::sequence::{terminated, tuple};
use nom::{Err, IResult, Needed};
use std::str;
use tinyvec::TinyVec;

fn token(input: &[u8]) -> IResult<&[u8], &[u8]> {
    fn is_token_char(i: u8) -> bool {
        is_alphanumeric(i) || b"!#$%&'*+-.^_`|~".contains(&i)
    }

    take_while(is_token_char)(input)
}

// Matches and consumes one space
fn sp(input: &[u8]) -> IResult<&[u8], ()> {
    value((), char(' '))(input)
}

// Matches and consumes a CRLF
fn crlf(input: &[u8]) -> IResult<&[u8], ()> {
    value((), tag("\r\n"))(input)
}

// Matches a run of printable ASCII characters
fn vchar_1(input: &[u8]) -> IResult<&[u8], &[u8]> {
    fn is_vchar(i: u8) -> bool {
        i > 32 && i <= 126
    }

    take_while(is_vchar)(input)
}

// Parses an RTSP version, only accepts 1.0 and 2.0
fn rtsp_version(input: &[u8]) -> IResult<&[u8], Version> {
    map(
        tuple((tag("RTSP/"), one_of("12"), tag(".0"))),
        |(_, major, _)| {
            if major == '2' {
                Version::V2_0
            } else {
                Version::V1_0
            }
        },
    )(input)
}

fn request_line(input: &[u8]) -> IResult<&[u8], RequestLine> {
    map(
        tuple((
            map(map_res(token, str::from_utf8), MethodRef::from),
            sp,
            alt((
                value(None, char('*')),
                map(map_res(vchar_1, str::from_utf8), Some),
            )),
            sp,
            rtsp_version,
            crlf,
        )),
        |(method, _, request_uri, _, version, _)| RequestLine {
            method,
            request_uri,
            version,
        },
    )(input)
}

fn from_digit(input: &str) -> Result<u16, std::num::ParseIntError> {
    str::parse::<u16>(input)
}

fn status_line(input: &[u8]) -> IResult<&[u8], StatusLine> {
    map(
        tuple((
            rtsp_version,
            sp,
            map_res(
                map_res(take_while_m_n(3, 3, is_digit), str::from_utf8),
                from_digit,
            ),
            sp,
            map_res(take_until(&b"\r\n"[..]), str::from_utf8),
            crlf,
        )),
        |(version, _, status, _, reason_phrase, _)| StatusLine {
            version,
            status: status.into(),
            reason_phrase,
        },
    )(input)
}

fn header_value(i: &[u8]) -> IResult<&[u8], &[u8]> {
    // Header values can be split over multiple lines, in which case there
    // will be a CRLF followed by one or more spaces/tabs.
    let mut o = i;
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
            let (res, rem) = i.split_at(i.len() - o.len());
            return Ok((rem, res));
        }
    }

    Err(Err::Incomplete(Needed::Unknown))
}

fn message_header(input: &[u8]) -> IResult<&[u8], HeaderRef> {
    map(
        tuple((
            map_res(token, str::from_utf8),
            char(':'),
            opt(take_while(is_space)),
            map_res(header_value, str::from_utf8),
            crlf,
        )),
        |(name, _, _, value, _)| HeaderRef { name, value },
    )(input)
}

fn headers(input: &[u8]) -> IResult<&[u8], TinyVec<[HeaderRef; 16]>> {
    terminated(many0_tinyvec(message_header), crlf)(input)
}

fn content_length<'a>(
    headers: &[HeaderRef<'a>],
) -> Result<usize, nom::Err<nom::error::Error<&'a [u8]>>> {
    if let Some(h) = headers
        .iter()
        .find(|h| &h.name.to_ascii_uppercase() == "CONTENT-LENGTH")
    {
        return str::parse::<usize>(h.value).map_err(|_| {
            nom::Err::Failure(nom::error::Error::new(
                h.value.as_bytes(),
                nom::error::ErrorKind::MapRes,
            ))
        });
    }
    Ok(0)
}

fn request(input: &[u8]) -> IResult<&[u8], RequestRef> {
    let (input, request_line) = request_line(input)?;
    let (input, headers) = headers(input)?;
    let content_length = content_length(&headers)?;
    let (input, body) = take(content_length)(input)?;

    Ok((
        input,
        RequestRef {
            method: request_line.method,
            request_uri: request_line.request_uri,
            version: request_line.version,
            headers,
            body,
        },
    ))
}

fn response(input: &[u8]) -> IResult<&[u8], ResponseRef> {
    let (input, status_line) = status_line(input)?;
    let (input, headers) = headers(input)?;
    let content_length = content_length(&headers)?;
    let (input, body) = take(content_length)(input)?;

    Ok((
        input,
        ResponseRef {
            version: status_line.version,
            status: status_line.status,
            reason_phrase: status_line.reason_phrase,
            headers,
            body,
        },
    ))
}

fn data(input: &[u8]) -> IResult<&[u8], DataRef> {
    map(
        tuple((char('$'), take(1usize), flat_map(be_u16, take))),
        |(_, channel_id, body): (_, &[u8], _)| DataRef {
            channel_id: channel_id[0],
            body,
        },
    )(input)
}

pub(crate) fn message(input: &[u8]) -> IResult<&[u8], MessageRef> {
    flat_map(fold_many0(crlf, || (), |_acc, _item| ()), |_| {
        alt((
            map(data, MessageRef::Data),
            map(request, MessageRef::Request),
            map(response, MessageRef::Response),
        ))
    })(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    use tinyvec::tiny_vec;

    #[test]
    fn test_request_line() {
        assert_eq!(
            request_line(b"OPTIONS rtsp://media.example.com/movie/twister.3gp RTSP/2.0\r\n"),
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
            request_line(b"OPTIONS * RTSP/2.0\r\n"),
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
    fn test_status_line() {
        assert_eq!(
            status_line(b"RTSP/2.0 200 All Good\r\n"),
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
            request(
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
            request(
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
    fn test_options_without_uri() {
        assert_eq!(
            request(
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
            request(
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
            request(
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
            response(
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
            response(
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
            response(
                b"RTSP/2.0 200 All Good\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
Content-Length: bad\r\n\
\r\n\
0123456789\
REMAINDER"
            ),
            Err(nom::Err::Failure(_))
        ));
    }

    #[test]
    fn test_data() {
        assert_eq!(
            data(&[b'$', 12, 0, 10, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, b'a', b'b'][..]),
            Ok((
                &b"ab"[..],
                DataRef::from_slice(12, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
            ))
        );
    }
}
