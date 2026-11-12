// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

// TODO: Consider making this public at a later time

use super::headers::parser_helpers::parse_digits;
use super::parser::error::{Input, MsgError};
use super::*;
use winnow::error::{ErrMode, Needed};

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum MessageRef<'a> {
    Request(RequestRef<'a>),
    Response(ResponseRef<'a>),
    Data(DataRef<'a>),
}

impl<'a> MessageRef<'a> {
    pub fn to_owned<T: From<&'a [u8]>>(&self) -> Result<Message<T>, ParseError> {
        let owned = match self {
            MessageRef::Request(request) => Message::Request(request.to_owned()?),
            MessageRef::Response(response) => Message::Response(response.to_owned()?),
            MessageRef::Data(data) => Message::Data(data.to_owned()),
        };

        Ok(owned)
    }

    pub fn parse(buf: &'a [u8]) -> Result<(Self, usize), ParseError> {
        let mut input = Input::new(buf);
        let res = match parser::message(&mut input) {
            Ok(res) => res,
            Err(ErrMode::Incomplete(needed)) => {
                return Err(ParseError::Incomplete(match needed {
                    Needed::Size(needed_len) => Some(needed_len),
                    _ => None,
                }))
            }
            Err(ErrMode::Backtrack(err)) | Err(ErrMode::Cut(err)) => {
                return Err(ParseError::Invalid(syntax_error(buf, &err)))
            }
        };

        let consumed = buf.len() - input.into_inner().len();

        Ok((res, consumed))
    }

    pub fn write<'b, W: std::io::Write + 'b>(self, w: &'b mut W) -> Result<(), WriteError>
    where
        'b: 'a,
    {
        match cookie_factory::gen_simple(serializer::message(self), w) {
            Ok(_) => Ok(()),
            Err(cookie_factory::GenError::IoError(io)) => Err(WriteError::IoError(io)),
            Err(err) => Err(WriteError::Other(format!("{:?}", err))),
        }
    }

    pub fn write_len(&self) -> u64 {
        match self {
            MessageRef::Data(data) => data.write_len(),
            _ => cookie_factory::gen(serializer::message(self.clone()), std::io::sink())
                .map(|(_w, pos)| pos)
                .expect("request/response serialization to sink cannot fail"),
        }
    }
}

// Maximum length of the offending line included in a syntax error.
const MAX_ERROR_LINE_LEN: usize = 128;

fn syntax_error(buf: &[u8], err: &MsgError) -> ParseSyntaxError {
    // The parser only ever fails with a subslice of the input buffer.
    let offset = err.input().as_ptr() as usize - buf.as_ptr() as usize;
    debug_assert!(offset + err.input().len() <= buf.len());

    let line = error_line(buf, offset);
    let section = error_section(buf, offset);

    ParseSyntaxError::new(section, offset, line, err.expected())
}

fn error_line(buf: &[u8], offset: usize) -> String {
    let offset = offset.min(buf.len());
    let start = buf[..offset]
        .iter()
        .rposition(|&b| b == b'\n')
        .map_or(0, |i| i + 1);
    let end = buf[start..]
        .iter()
        .position(|&b| b == b'\n' || b == b'\r')
        .map_or(buf.len(), |i| start + i);

    let raw = &buf[start..end];
    let truncated = raw.len() > MAX_ERROR_LINE_LEN;
    let mut line = String::from_utf8_lossy(if truncated {
        &raw[..MAX_ERROR_LINE_LEN]
    } else {
        raw
    })
    .into_owned();
    if truncated {
        line.push_str("...");
    }
    line
}

fn error_section(buf: &[u8], offset: usize) -> ParseSyntaxSection {
    let mut start = 0;
    while buf[start..].starts_with(b"\r\n") {
        start += 2;
    }

    // Interleaved data frames have no textual lines.
    if buf[start..].starts_with(b"$") {
        return ParseSyntaxSection::Body;
    }

    let line_end = find_crlf(&buf[start..]).map_or(buf.len(), |i| start + i);
    if offset <= line_end {
        return if buf[start..].starts_with(b"RTSP/") {
            ParseSyntaxSection::StatusLine
        } else {
            ParseSyntaxSection::RequestLine
        };
    }

    let mut pos = line_end + 2;
    while pos < buf.len() && !buf[pos..].starts_with(b"\r\n") {
        match find_crlf(&buf[pos..]) {
            Some(i) if offset <= pos + i => return ParseSyntaxSection::HeaderLine,
            Some(i) => pos += i + 2,
            None => return ParseSyntaxSection::HeaderLine,
        }
    }

    ParseSyntaxSection::Body
}

fn find_crlf(input: &[u8]) -> Option<usize> {
    input.windows(2).position(|w| w == b"\r\n")
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum MethodRef<'a> {
    Describe,
    GetParameter,
    Options,
    Pause,
    Play,
    PlayNotify,
    Redirect,
    Setup,
    SetParameter,
    Announce,
    Record,
    Teardown,
    Extension(&'a str),
}

impl<'a> MethodRef<'a> {
    pub fn to_owned(&self) -> Method {
        match *self {
            MethodRef::Describe => Method::Describe,
            MethodRef::GetParameter => Method::GetParameter,
            MethodRef::Options => Method::Options,
            MethodRef::Pause => Method::Pause,
            MethodRef::Play => Method::Play,
            MethodRef::PlayNotify => Method::PlayNotify,
            MethodRef::Redirect => Method::Redirect,
            MethodRef::Setup => Method::Setup,
            MethodRef::SetParameter => Method::SetParameter,
            MethodRef::Announce => Method::Announce,
            MethodRef::Record => Method::Record,
            MethodRef::Teardown => Method::Teardown,
            MethodRef::Extension(s) => Method::Extension(s.into()),
        }
    }
}

impl<'a> From<&'a str> for MethodRef<'a> {
    fn from(v: &'a str) -> Self {
        match v {
            "DESCRIBE" => MethodRef::Describe,
            "GET_PARAMETER" => MethodRef::GetParameter,
            "OPTIONS" => MethodRef::Options,
            "PAUSE" => MethodRef::Pause,
            "PLAY" => MethodRef::Play,
            "PLAY_NOTIFY" => MethodRef::PlayNotify,
            "REDIRECT" => MethodRef::Redirect,
            "SETUP" => MethodRef::Setup,
            "SET_PARAMETER" => MethodRef::SetParameter,
            "ANNOUNCE" => MethodRef::Announce,
            "RECORD" => MethodRef::Record,
            "TEARDOWN" => MethodRef::Teardown,
            v => MethodRef::Extension(v),
        }
    }
}

impl<'a> From<&'a MethodRef<'a>> for &'a str {
    fn from(v: &'a MethodRef<'a>) -> Self {
        match v {
            MethodRef::Describe => "DESCRIBE",
            MethodRef::GetParameter => "GET_PARAMETER",
            MethodRef::Options => "OPTIONS",
            MethodRef::Pause => "PAUSE",
            MethodRef::Play => "PLAY",
            MethodRef::PlayNotify => "PLAY_NOTIFY",
            MethodRef::Redirect => "REDIRECT",
            MethodRef::Setup => "SETUP",
            MethodRef::SetParameter => "SET_PARAMETER",
            MethodRef::Announce => "ANNOUNCE",
            MethodRef::Record => "RECORD",
            MethodRef::Teardown => "TEARDOWN",
            MethodRef::Extension(v) => v,
        }
    }
}

/// Ensure the `Content-Length` header, if any, matches the body.
fn check_content_length<'a>(headers: &[HeaderRef<'a>], body: &'a [u8]) -> Result<(), WriteError> {
    let mut content_length = None;

    for h in headers
        .iter()
        .filter(|h| h.name.eq_ignore_ascii_case("Content-Length"))
    {
        let v = parse_digits::<usize>(h.value).map_err(|_| WriteError::InvalidContentLength {
            content_length: None,
            body_len: body.len(),
        })?;

        // Multiple non-identical `Content-Length` fields are malformed.
        match content_length {
            None => content_length = Some(v),
            Some(prev) if prev != v => {
                return Err(WriteError::InvalidContentLength {
                    content_length: Some(v),
                    body_len: body.len(),
                })
            }
            Some(_) => (),
        }
    }

    if content_length == Some(body.len()) || (content_length.is_none() && body.is_empty()) {
        return Ok(());
    }

    Err(WriteError::InvalidContentLength {
        content_length,
        body_len: body.len(),
    })
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RequestRef<'a> {
    pub(crate) method: MethodRef<'a>,
    pub(crate) request_uri: Option<&'a str>,
    pub(crate) version: Version,
    pub(crate) headers: TinyVec<[HeaderRef<'a>; 16]>,
    pub(crate) body: &'a [u8],
}

impl<'a> RequestRef<'a> {
    pub fn to_owned<T: From<&'a [u8]>>(&self) -> Result<Request<T>, ParseError> {
        Ok(Request {
            method: self.method.to_owned(),
            request_uri: self
                .request_uri
                .map(Url::parse)
                .transpose()
                .map_err(|_| ParseError::Error)?,
            version: self.version,
            headers: Headers::from_headers_ref(&self.headers)?,
            body: self.body.into(),
        })
    }

    pub fn write<'b, W: std::io::Write + 'b>(self, w: &'b mut W) -> Result<(), WriteError>
    where
        'b: 'a,
    {
        check_content_length(&self.headers, self.body)?;

        match cookie_factory::gen_simple(serializer::request(self), w) {
            Ok(_) => Ok(()),
            Err(cookie_factory::GenError::IoError(io)) => Err(WriteError::IoError(io)),
            Err(err) => Err(WriteError::Other(format!("{:?}", err))),
        }
    }

    pub fn write_len(&self) -> u64 {
        cookie_factory::gen(serializer::request(self.clone()), std::io::sink())
            .map(|(_w, pos)| pos)
            .expect("serialization to sink cannot fail")
    }

    #[allow(dead_code)]
    pub fn method(&self) -> &MethodRef<'a> {
        &self.method
    }

    #[allow(dead_code)]
    pub fn uri(&self) -> Option<&'a str> {
        self.request_uri
    }

    #[allow(dead_code)]
    pub fn version(&self) -> Version {
        self.version
    }

    #[allow(dead_code)]
    pub fn body(&self) -> &'a [u8] {
        self.body
    }

    #[allow(dead_code)]
    pub fn headers(&self) -> impl Iterator<Item = &HeaderRef<'_>> {
        self.headers.iter()
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct RequestLine<'a> {
    pub(crate) method: MethodRef<'a>,
    pub(crate) request_uri: Option<&'a str>,
    pub(crate) version: Version,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ResponseRef<'a> {
    pub(crate) version: Version,
    pub(crate) status: StatusCode,
    pub(crate) reason_phrase: &'a str,
    pub(crate) headers: TinyVec<[HeaderRef<'a>; 16]>,
    pub(crate) body: &'a [u8],
}

impl<'a> ResponseRef<'a> {
    pub fn to_owned<T: From<&'a [u8]>>(&self) -> Result<Response<T>, ParseError> {
        Ok(Response {
            version: self.version,
            status: self.status,
            reason_phrase: self.reason_phrase.into(),
            headers: Headers::from_headers_ref(&self.headers)?,
            body: self.body.into(),
        })
    }

    pub fn write<'b, W: std::io::Write + 'b>(self, w: &'b mut W) -> Result<(), WriteError>
    where
        'b: 'a,
    {
        check_content_length(&self.headers, self.body)?;

        match cookie_factory::gen_simple(serializer::response(self), w) {
            Ok(_) => Ok(()),
            Err(cookie_factory::GenError::IoError(io)) => Err(WriteError::IoError(io)),
            Err(err) => Err(WriteError::Other(format!("{:?}", err))),
        }
    }

    pub fn write_len(&self) -> u64 {
        cookie_factory::gen(serializer::response(self.clone()), std::io::sink())
            .map(|(_w, pos)| pos)
            .expect("serialization to sink cannot fail")
    }

    #[allow(dead_code)]
    pub fn version(&self) -> Version {
        self.version
    }

    #[allow(dead_code)]
    pub fn status(&self) -> StatusCode {
        self.status
    }

    #[allow(dead_code)]
    pub fn reason_phrase(&self) -> &'a str {
        self.reason_phrase
    }

    #[allow(dead_code)]
    pub fn body(&self) -> &'a [u8] {
        self.body
    }

    #[allow(dead_code)]
    pub fn headers(&self) -> impl Iterator<Item = &HeaderRef<'_>> {
        self.headers.iter()
    }
}

#[derive(Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub(crate) struct StatusLine<'a> {
    pub(crate) version: Version,
    pub(crate) status: StatusCode,
    pub(crate) reason_phrase: &'a str,
}

#[derive(Debug, PartialEq, Eq, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DataRef<'a> {
    pub(crate) channel_id: u8,
    pub(crate) body: &'a [u8],
}

impl<'a> DataRef<'a> {
    pub fn to_owned<T: From<&'a [u8]>>(&self) -> Data<T> {
        Data {
            channel_id: self.channel_id,
            body: self.body.into(),
        }
    }

    pub fn write<'b, W: std::io::Write + 'b>(self, w: &'b mut W) -> Result<(), WriteError>
    where
        'b: 'a,
    {
        let len = self.len();
        match cookie_factory::gen_simple(serializer::data(self), w) {
            Ok(_) => Ok(()),
            Err(cookie_factory::GenError::IoError(io)) => Err(WriteError::IoError(io)),
            Err(cookie_factory::GenError::CustomError(_)) => Err(WriteError::DataTooLarge(len)),
            Err(err) => Err(WriteError::Other(format!("{:?}", err))),
        }
    }

    pub fn write_len(&self) -> u64 {
        // '$' + channel + 16-bit length
        4 + self.len() as u64
    }

    #[allow(dead_code)]
    pub fn channel_id(&self) -> u8 {
        self.channel_id
    }

    #[allow(dead_code)]
    pub fn set_channel_id(&mut self, channel_id: u8) {
        self.channel_id = channel_id;
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.body.len()
    }

    #[allow(dead_code)]
    pub fn as_slice(&self) -> &'a [u8] {
        self.body
    }

    #[allow(dead_code)]
    pub fn set_body(&mut self, body: &'a [u8]) {
        self.body = body;
    }

    #[allow(dead_code)]
    pub fn from_slice(channel_id: u8, body: &'a [u8]) -> Self {
        Self { channel_id, body }
    }
}

impl<'a> AsRef<[u8]> for DataRef<'a> {
    fn as_ref(&self) -> &[u8] {
        self.body
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct HeaderRef<'a> {
    pub(crate) name: &'a str,
    pub(crate) value: &'a str,
}

impl<'a> HeaderRef<'a> {
    #[allow(dead_code)]
    pub fn name(&self) -> &'a str {
        self.name
    }

    #[allow(dead_code)]
    pub fn value(&self) -> &'a str {
        self.value
    }

    #[allow(dead_code)]
    pub fn set_value(&mut self, value: &'a str) {
        self.value = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_content_length_mismatch() {
        let mut request =
            crate::Request::builder(crate::Method::SetParameter, crate::Version::V2_0)
                .build(b"123");
        *request.header_mut(&crate::headers::CONTENT_LENGTH).unwrap() = "100".into();

        let mut v = vec![];

        match request.write(&mut v) {
            Err(WriteError::InvalidContentLength {
                content_length,
                body_len,
            }) => {
                assert_eq!(content_length, Some(100));
                assert_eq!(body_len, 3);
            }
            other => panic!("Expected InvalidContentLength, got: {:?}", other),
        }
        assert!(v.is_empty());
    }

    #[test]
    fn test_write_len_oversized_data() {
        // write() fails for data larger than the 16-bit length field can
        // hold, but the length itself can still be calculated.
        let body = vec![0; 65536];
        let data = DataRef::from_slice(0, &body);

        assert_eq!(data.write_len(), 4 + body.len() as u64);
    }

    #[test]
    fn test_to_owned_invalid_header_name() {
        // An empty header name parses at the message level (the token
        // grammar is matched leniently) but must fail the conversion to
        // an owned message instead of panicking.
        let msg = b"RTSP/2.0 200 OK\r\n: foo\r\n\r\n";

        let (parsed, _) = MessageRef::parse(msg).expect("should parse");

        assert!(parsed.to_owned::<Vec<u8>>().is_err());
    }

    #[test]
    fn test_parse_error_request_line() {
        let err = MessageRef::parse(b"DESCRIBE rtsp://example.com RTSP/1.3\r\n\r\n").unwrap_err();

        match err {
            ParseError::Invalid(err) => {
                assert_eq!(err.section(), ParseSyntaxSection::RequestLine);
                assert_eq!(err.offset(), 34);
                assert_eq!(err.line(), "DESCRIBE rtsp://example.com RTSP/1.3");
                assert_eq!(err.expected(), "literal");
            }
            other => panic!("expected invalid parse error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_error_status_line() {
        let err = MessageRef::parse(b"RTSP/2.0 2XX OK\r\n\r\n").unwrap_err();

        match err {
            ParseError::Invalid(err) => {
                assert_eq!(err.section(), ParseSyntaxSection::StatusLine);
                assert!(err.offset() > 0);
                assert_eq!(err.line(), "RTSP/2.0 2XX OK");
                assert!(!err.expected().is_empty());
            }
            other => panic!("expected invalid parse error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_error_header_line() {
        let err =
            MessageRef::parse(b"OPTIONS * RTSP/2.0\r\nContent-Length: abc\r\n\r\n").unwrap_err();

        match err {
            ParseError::Invalid(err) => {
                assert_eq!(err.section(), ParseSyntaxSection::HeaderLine);
                assert_eq!(err.offset(), 36);
                assert_eq!(err.line(), "Content-Length: abc");
                assert_eq!(err.expected(), "valid value");
                assert_eq!(
                    err.to_string(),
                    "syntax error in header line at offset 36, expected valid value, in line 'Content-Length: abc'"
                );
            }
            other => panic!("expected invalid parse error, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_error_line_truncation() {
        let mut msg = b"OPTIONS * RTSP/2.0\r\nContent-Length: ".to_vec();
        msg.extend(std::iter::repeat_n(b'a', 1000));
        msg.extend_from_slice(b"\r\n\r\n");

        let err = MessageRef::parse(&msg).unwrap_err();

        match err {
            ParseError::Invalid(err) => {
                assert_eq!(
                    err.line(),
                    "Content-Length: ".to_string() + &"a".repeat(112) + "..."
                );
            }
            other => panic!("expected invalid parse error, got {:?}", other),
        }
    }
}
