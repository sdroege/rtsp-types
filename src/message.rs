// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Message<Body> {
    Request(Request<Body>),
    Response(Response<Body>),
    Data(Data<Body>),
}

impl<Body> Message<Body> {
    pub(crate) fn borrow(&self) -> MessageRef
    where
        Body: AsRef<[u8]>,
    {
        match self {
            Message::Request(request) => MessageRef::Request(request.borrow()),
            Message::Response(response) => MessageRef::Response(response.borrow()),
            Message::Data(data) => MessageRef::Data(data.borrow()),
        }
    }

    pub fn write<'b, W: std::io::Write + 'b>(&self, w: &'b mut W) -> Result<(), WriteError>
    where
        Body: AsRef<[u8]>,
    {
        self.borrow().write(w)
    }

    pub fn write_len(&self) -> u64
    where
        Body: AsRef<[u8]>,
    {
        self.borrow().write_len()
    }
}

impl<'a, T: From<&'a [u8]> + 'a> Message<T> {
    pub fn parse<B: AsRef<[u8]> + 'a>(buf: &'a B) -> Result<(Self, usize), ParseError> {
        let (msg, consumed) = MessageRef::parse(buf.as_ref())?;

        Ok((msg.to_owned()?, consumed))
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Method {
    Describe,
    GetParameter,
    Options,
    Pause,
    Play,
    PlayNotify,
    Redirect,
    Setup,
    SetParameter,
    Teardown,
    Extension(String),
}

impl Method {
    pub(crate) fn borrow(&self) -> MethodRef {
        match self {
            Method::Describe => MethodRef::Describe,
            Method::GetParameter => MethodRef::GetParameter,
            Method::Options => MethodRef::Options,
            Method::Pause => MethodRef::Pause,
            Method::Play => MethodRef::Play,
            Method::PlayNotify => MethodRef::PlayNotify,
            Method::Redirect => MethodRef::Redirect,
            Method::Setup => MethodRef::Setup,
            Method::SetParameter => MethodRef::SetParameter,
            Method::Teardown => MethodRef::Teardown,
            Method::Extension(s) => MethodRef::Extension(&s),
        }
    }
}

impl<'a> From<&'a str> for Method {
    fn from(v: &'a str) -> Self {
        MethodRef::from(v).to_owned()
    }
}

impl<'a> From<&'a Method> for &'a str {
    fn from(v: &'a Method) -> Self {
        match v {
            Method::Describe => "DESCRIBE",
            Method::GetParameter => "GET_PARAMETER",
            Method::Options => "OPTIONS",
            Method::Pause => "PAUSE",
            Method::Play => "PLAY",
            Method::PlayNotify => "PLAY_NOTIFY",
            Method::Redirect => "REDIRECT",
            Method::Setup => "SETUP",
            Method::SetParameter => "SET_PARAMETER",
            Method::Teardown => "TEARDOWN",
            Method::Extension(ref v) => v,
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Request<Body> {
    pub(crate) method: Method,
    pub(crate) request_uri: Option<Url>,
    pub(crate) version: Version,
    pub(crate) headers: Headers,
    pub(crate) body: Body,
}

impl<BodyA, BodyB: PartialEq<BodyA>> PartialEq<Request<BodyA>> for Request<BodyB> {
    fn eq(&self, other: &Request<BodyA>) -> bool {
        self.method == other.method
            && self.request_uri == other.request_uri
            && self.version == other.version
            && self.headers == other.headers
            && self.body == other.body
    }
}

impl Request<Empty> {
    pub fn builder(method: Method, version: Version) -> RequestBuilder {
        RequestBuilder::new(method, version)
    }
}

impl<Body> Request<Body> {
    pub(crate) fn borrow(&self) -> RequestRef
    where
        Body: AsRef<[u8]>,
    {
        let headers = self
            .headers
            .iter()
            .map(|(name, value)| HeaderRef {
                name: name.as_str(),
                value: value.as_str(),
            })
            .collect();

        RequestRef {
            method: self.method.borrow(),
            request_uri: self.request_uri.as_ref().map(|u| u.as_str()),
            version: self.version,
            headers,
            body: self.body.as_ref(),
        }
    }

    // Accessors
    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn uri(&self) -> Option<&Url> {
        self.request_uri.as_ref()
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    // Body API
    pub fn into_body(self) -> Body {
        self.body
    }

    pub fn map_body<NewBody, F: FnOnce(Body) -> NewBody>(self, func: F) -> Request<NewBody> {
        Request {
            method: self.method,
            request_uri: self.request_uri,
            version: self.version,
            headers: self.headers,
            body: func(self.body),
        }
    }

    pub fn replace_body<NewBody>(self, new_body: NewBody) -> Request<NewBody> {
        Request {
            method: self.method,
            request_uri: self.request_uri,
            version: self.version,
            headers: self.headers,
            body: new_body,
        }
    }

    // Header API
    pub fn append_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.append(name, value);
    }

    pub fn insert_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    pub fn remove_header(&mut self, name: &HeaderName) {
        self.headers.remove(name);
    }

    pub fn header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    pub fn header_mut(&mut self, name: &HeaderName) -> Option<&mut HeaderValue> {
        self.headers.get_mut(name)
    }

    pub fn headers(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.headers.iter()
    }

    pub fn header_names(&self) -> impl Iterator<Item = &HeaderName> {
        self.headers.names()
    }

    pub fn header_values(&self) -> impl Iterator<Item = &HeaderValue> {
        self.headers.values()
    }
}

impl<Body> AsRef<Headers> for Request<Body> {
    fn as_ref(&self) -> &Headers {
        &self.headers
    }
}

impl<Body> AsMut<Headers> for Request<Body> {
    fn as_mut(&mut self) -> &mut Headers {
        &mut self.headers
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct RequestBuilder(Request<Empty>);

impl RequestBuilder {
    fn new(method: Method, version: Version) -> Self {
        Self(Request {
            method,
            request_uri: None,
            version,
            headers: Headers::new(),
            body: Empty,
        })
    }

    pub fn uri<U: Into<Url>>(self, request_uri: U) -> Self {
        Self(Request {
            request_uri: Some(request_uri.into()),
            ..self.0
        })
    }

    pub fn header<N: Into<HeaderName>, V: Into<HeaderValue>>(mut self, name: N, value: V) -> Self {
        let name = name.into();
        let value = value.into();

        self.0.headers.append(name, value);

        self
    }

    pub fn empty(self) -> Request<Empty> {
        self.0
    }

    pub fn build<Body>(self, body: Body) -> Request<Body> {
        Request {
            method: self.0.method,
            request_uri: self.0.request_uri,
            version: self.0.version,
            headers: self.0.headers,
            body,
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Response<Body> {
    pub(crate) version: Version,
    pub(crate) status: StatusCode,
    pub(crate) reason_phrase: String,
    pub(crate) headers: Headers,
    pub(crate) body: Body,
}

impl<BodyA, BodyB: PartialEq<BodyA>> PartialEq<Response<BodyA>> for Response<BodyB> {
    fn eq(&self, other: &Response<BodyA>) -> bool {
        self.version == other.version
            && self.status == other.status
            && self.reason_phrase == other.reason_phrase
            && self.headers == other.headers
            && self.body == other.body
    }
}

impl Response<Empty> {
    pub fn builder<S: Into<String>>(
        version: Version,
        status: StatusCode,
        reason_phrase: S,
    ) -> ResponseBuilder {
        ResponseBuilder::new(version, status, reason_phrase.into())
    }
}

impl<Body> Response<Body> {
    pub(crate) fn borrow(&self) -> ResponseRef
    where
        Body: AsRef<[u8]>,
    {
        let headers = self
            .headers
            .iter()
            .map(|(name, value)| HeaderRef {
                name: name.as_str(),
                value: value.as_str(),
            })
            .collect();

        ResponseRef {
            version: self.version,
            status: self.status,
            reason_phrase: &self.reason_phrase,
            headers,
            body: self.body.as_ref(),
        }
    }

    // Accessors
    pub fn version(&self) -> Version {
        self.version
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn reason_phrase(&self) -> &str {
        self.reason_phrase.as_str()
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    pub fn set_status(&mut self, status: StatusCode) {
        self.status = status;
    }

    pub fn set_reason_phrase<S: Into<String>>(&mut self, reason_phrase: S) {
        self.reason_phrase = reason_phrase.into();
    }

    // Body API
    pub fn into_body(self) -> Body {
        self.body
    }

    pub fn map_body<NewBody, F: FnOnce(Body) -> NewBody>(self, func: F) -> Response<NewBody> {
        Response {
            version: self.version,
            status: self.status,
            reason_phrase: self.reason_phrase,
            headers: self.headers,
            body: func(self.body),
        }
    }

    pub fn replace_body<NewBody>(self, new_body: NewBody) -> Response<NewBody> {
        Response {
            version: self.version,
            status: self.status,
            reason_phrase: self.reason_phrase,
            headers: self.headers,
            body: new_body,
        }
    }

    // Header API
    pub fn append_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.append(name, value);
    }

    pub fn insert_header(&mut self, name: HeaderName, value: HeaderValue) {
        self.headers.insert(name, value);
    }

    pub fn remove_header(&mut self, name: &HeaderName) {
        self.headers.remove(name);
    }

    pub fn header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    pub fn header_mut(&mut self, name: &HeaderName) -> Option<&mut HeaderValue> {
        self.headers.get_mut(name)
    }

    pub fn headers(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.headers.iter()
    }

    pub fn header_names(&self) -> impl Iterator<Item = &HeaderName> {
        self.headers.names()
    }

    pub fn header_values(&self) -> impl Iterator<Item = &HeaderValue> {
        self.headers.values()
    }
}

impl<Body> AsRef<Headers> for Response<Body> {
    fn as_ref(&self) -> &Headers {
        &self.headers
    }
}

impl<Body> AsMut<Headers> for Response<Body> {
    fn as_mut(&mut self) -> &mut Headers {
        &mut self.headers
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ResponseBuilder(Response<Empty>);

impl ResponseBuilder {
    fn new(version: Version, status: StatusCode, reason_phrase: String) -> Self {
        let response = Response {
            version,
            status,
            reason_phrase,
            headers: Headers::new(),
            body: Empty,
        };

        Self(response)
    }

    pub fn header<N: Into<HeaderName>, V: Into<HeaderValue>>(mut self, name: N, value: V) -> Self {
        let name = name.into();
        let value = value.into();

        self.0.headers.append(name, value);

        self
    }

    pub fn empty(self) -> Response<Empty> {
        self.0
    }

    pub fn build<Body>(self, body: Body) -> Response<Body> {
        Response {
            version: self.0.version,
            status: self.0.status,
            reason_phrase: self.0.reason_phrase,
            headers: self.0.headers,
            body,
        }
    }
}

#[derive(Debug, Clone, Eq)]
pub struct Data<Body> {
    pub(crate) channel_id: u8,
    pub(crate) body: Body,
}

impl<BodyA, BodyB: PartialEq<BodyA>> PartialEq<Data<BodyA>> for Data<BodyB> {
    fn eq(&self, other: &Data<BodyA>) -> bool {
        self.channel_id == other.channel_id && self.body == other.body
    }
}

impl<Body> Data<Body> {
    pub(crate) fn borrow(&self) -> DataRef
    where
        Body: AsRef<[u8]>,
    {
        DataRef {
            channel_id: self.channel_id,
            body: self.body.as_ref(),
        }
    }

    pub fn new(channel_id: u8, body: Body) -> Self {
        Self { channel_id, body }
    }

    // Accessors
    pub fn channel_id(&self) -> u8 {
        self.channel_id
    }

    pub fn set_channel_id(&mut self, channel_id: u8) {
        self.channel_id = channel_id;
    }

    pub fn len(&self) -> usize
    where
        Body: AsRef<[u8]>,
    {
        self.body.as_ref().len()
    }

    pub fn is_empty(&self) -> bool
    where
        Body: AsRef<[u8]>,
    {
        self.body.as_ref().is_empty()
    }

    pub fn as_slice(&self) -> &[u8]
    where
        Body: AsRef<[u8]>,
    {
        self.body.as_ref()
    }

    // Body API
    pub fn into_body(self) -> Body {
        self.body
    }

    pub fn map_body<NewBody, F: FnOnce(Body) -> NewBody>(self, func: F) -> Data<NewBody> {
        Data {
            channel_id: self.channel_id,
            body: func(self.body),
        }
    }

    pub fn replace_body<NewBody>(self, new_body: NewBody) -> Data<NewBody> {
        Data {
            channel_id: self.channel_id,
            body: new_body,
        }
    }
}

impl Data<Vec<u8>> {
    pub fn from_vec(channel_id: u8, body: Vec<u8>) -> Self {
        Self { channel_id, body }
    }
}

impl<Body: AsRef<[u8]>> AsRef<[u8]> for Data<Body> {
    fn as_ref(&self) -> &[u8] {
        self.body.as_ref()
    }
}
