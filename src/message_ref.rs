// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

// TODO: Consider making this public at a later time

use super::*;

#[derive(Debug, PartialEq, Clone)]
pub enum MessageRef<'a> {
    Request(RequestRef<'a>),
    Response(ResponseRef<'a>),
    Data(DataRef<'a>),
}

impl<'a> MessageRef<'a> {
    pub fn to_owned<T: From<&'a [u8]>>(&self) -> Result<Message<T>, ParseError> {
        let owned = match self {
            MessageRef::Request(request) => Message::Request(request.to_owned()?),
            MessageRef::Response(response) => Message::Response(response.to_owned()),
            MessageRef::Data(data) => Message::Data(data.to_owned()),
        };

        Ok(owned)
    }

    pub fn parse(buf: &'a [u8]) -> Result<(Self, usize), ParseError> {
        let (remainder, res) = match parser::message(buf) {
            Ok(res) => res,
            Err(nom::Err::Incomplete(..)) => return Err(ParseError::Incomplete),
            Err(_) => return Err(ParseError::Error),
        };

        let consumed = buf.len() - remainder.len();

        Ok((res, consumed))
    }

    pub fn write<'b, W: std::io::Write + 'b>(self, w: &'b mut W) -> Result<(), WriteError>
    where
        'b: 'a,
    {
        match cookie_factory::gen_simple(serializer::message(self), w) {
            Ok(_) => Ok(()),
            Err(cookie_factory::GenError::IoError(io)) => Err(WriteError::IoError(io)),
            // This case can't really happen with our serializer!
            Err(err) => panic!("Failed to write message: {:?}", err),
        }
    }

    pub fn write_len(&self) -> u64 {
        match cookie_factory::gen(serializer::message(self.clone()), std::io::sink()) {
            Ok((_w, pos)) => pos,
            // This case can't really happen with our serializer!
            Err(err) => panic!("Failed to calculate write length: {:?}", err),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
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

#[derive(Debug, PartialEq, Clone)]
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
            headers: Headers::from_headers_ref(&self.headers),
            body: self.body.into(),
        })
    }

    pub fn write<'b, W: std::io::Write + 'b>(self, w: &'b mut W) -> Result<(), WriteError>
    where
        'b: 'a,
    {
        match cookie_factory::gen_simple(serializer::request(self), w) {
            Ok(_) => Ok(()),
            Err(cookie_factory::GenError::IoError(io)) => Err(WriteError::IoError(io)),
            // This case can't really happen with our serializer!
            Err(err) => panic!("Failed to write message: {:?}", err),
        }
    }

    pub fn write_len(&self) -> u64 {
        match cookie_factory::gen(serializer::request(self.clone()), std::io::sink()) {
            Ok((_w, pos)) => pos,
            // This case can't really happen with our serializer!
            Err(err) => panic!("Failed to calculate write length: {:?}", err),
        }
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
    pub fn headers(&self) -> impl Iterator<Item = &HeaderRef> {
        self.headers.iter()
    }
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) struct RequestLine<'a> {
    pub(crate) method: MethodRef<'a>,
    pub(crate) request_uri: Option<&'a str>,
    pub(crate) version: Version,
}

#[derive(Debug, PartialEq, Clone)]
pub struct ResponseRef<'a> {
    pub(crate) version: Version,
    pub(crate) status: StatusCode,
    pub(crate) reason_phrase: &'a str,
    pub(crate) headers: TinyVec<[HeaderRef<'a>; 16]>,
    pub(crate) body: &'a [u8],
}

impl<'a> ResponseRef<'a> {
    pub fn to_owned<T: From<&'a [u8]>>(&self) -> Response<T> {
        Response {
            version: self.version,
            status: self.status,
            reason_phrase: self.reason_phrase.into(),
            headers: Headers::from_headers_ref(&self.headers),
            body: self.body.into(),
        }
    }

    pub fn write<'b, W: std::io::Write + 'b>(self, w: &'b mut W) -> Result<(), WriteError>
    where
        'b: 'a,
    {
        match cookie_factory::gen_simple(serializer::response(self), w) {
            Ok(_) => Ok(()),
            Err(cookie_factory::GenError::IoError(io)) => Err(WriteError::IoError(io)),
            // This case can't really happen with our serializer!
            Err(err) => panic!("Failed to write message: {:?}", err),
        }
    }

    pub fn write_len(&self) -> u64 {
        match cookie_factory::gen(serializer::response(self.clone()), std::io::sink()) {
            Ok((_w, pos)) => pos,
            // This case can't really happen with our serializer!
            Err(err) => panic!("Failed to calculate write length: {:?}", err),
        }
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
    pub fn headers(&self) -> impl Iterator<Item = &HeaderRef> {
        self.headers.iter()
    }
}

#[derive(Debug, PartialEq)]
pub(crate) struct StatusLine<'a> {
    pub(crate) version: Version,
    pub(crate) status: StatusCode,
    pub(crate) reason_phrase: &'a str,
}

#[derive(Debug, PartialEq, Clone)]
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
        match cookie_factory::gen_simple(serializer::data(self), w) {
            Ok(_) => Ok(()),
            Err(cookie_factory::GenError::IoError(io)) => Err(WriteError::IoError(io)),
            // This case can't really happen with our serializer!
            Err(err) => panic!("Failed to write message: {:?}", err),
        }
    }

    pub fn write_len(&self) -> u64 {
        match cookie_factory::gen(serializer::data(self.clone()), std::io::sink()) {
            Ok((_w, pos)) => pos,
            // This case can't really happen with our serializer!
            Err(err) => panic!("Failed to calculate write length: {:?}", err),
        }
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

#[derive(Debug, PartialEq, Clone)]
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

impl<'a> Default for HeaderRef<'a> {
    fn default() -> Self {
        HeaderRef {
            name: "",
            value: "",
        }
    }
}
