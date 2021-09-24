// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use crate::headers::{TypedAppendableHeader, TypedHeader};

/// Enum holding all possible RTSP message types.
///
/// A `Message` can be a [`Request`](struct.Request.html), a [`Response`](struct.Response.html) or
/// [`Data`](struct.Data.html).
///
/// The body of the message is generic and usually a type that implements `AsRef<[u8]>`. For empty
/// bodies there also exists the [`Empty`](struct.Empty.html) type.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Message<Body> {
    /// Request message
    Request(Request<Body>),
    /// Response message
    Response(Response<Body>),
    /// Data message
    Data(Data<Body>),
}

impl<Body> From<Request<Body>> for Message<Body> {
    fn from(v: Request<Body>) -> Self {
        Message::Request(v)
    }
}

impl<Body> From<Response<Body>> for Message<Body> {
    fn from(v: Response<Body>) -> Self {
        Message::Response(v)
    }
}

impl<Body> From<Data<Body>> for Message<Body> {
    fn from(v: Data<Body>) -> Self {
        Message::Data(v)
    }
}

impl<Body: AsRef<[u8]>> Message<Body> {
    pub(crate) fn borrow(&self) -> MessageRef {
        match self {
            Message::Request(request) => MessageRef::Request(request.borrow()),
            Message::Response(response) => MessageRef::Response(response.borrow()),
            Message::Data(data) => MessageRef::Data(data.borrow()),
        }
    }

    /// Serialize the message to any `std::io::Write`.
    ///
    /// Resuming writing after `std::io::ErrorKind::WouldBlock` is not supported. Any previously
    /// written data will have to be discarded for resuming.
    ///
    /// ## Serializing an RTSP message
    ///
    /// ```rust
    /// let request = rtsp_types::Request::builder(
    ///         rtsp_types::Method::SetParameter,
    ///         rtsp_types::Version::V2_0
    ///     )
    ///     .request_uri(rtsp_types::Url::parse("rtsp://example.com/test").expect("Invalid URI"))
    ///     .header(rtsp_types::headers::CSEQ, "2")
    ///     .header(rtsp_types::headers::CONTENT_TYPE, "text/parameters")
    ///     .build(Vec::from(&b"barparam: barstuff"[..]));
    ///
    ///  let mut data = Vec::new();
    ///  request.write(&mut data).expect("Failed to serialize request");
    ///
    ///  assert_eq!(
    ///     data,
    ///     b"SET_PARAMETER rtsp://example.com/test RTSP/2.0\r\n\
    ///       Content-Length: 18\r\n\
    ///       Content-Type: text/parameters\r\n\
    ///       CSeq: 2\r\n\
    ///       \r\n\
    ///       barparam: barstuff",
    ///  );
    /// ```
    pub fn write<'b, W: std::io::Write + 'b>(&self, w: &'b mut W) -> Result<(), WriteError> {
        self.borrow().write(w)
    }

    /// Calculate the number of bytes needed to serialize the message.
    pub fn write_len(&self) -> u64 {
        self.borrow().write_len()
    }
}

impl<'a, T: From<&'a [u8]>> Message<T> {
    /// Try parse a message from a `&[u8]` and also return how many bytes were consumed.
    ///
    /// The body type of the returned message can be any type that implements `From<&[u8]>`. This
    /// includes `Vec<u8>` and `&[u8]` among others.
    ///
    /// If parsing the message succeeds, in addition to the message itself also the number of bytes
    /// that were consumed from the input data are returned. This allows the caller to advance to
    /// the next message.
    ///
    /// If parsing the message fails with [`ParseError::Incomplete`](enum.ParseError.html#variant.Incomplete)
    /// then the caller has to provide more data for successfully parsing the message.
    ///
    /// Otherwise, if parsing the message fails with [`ParseError::Error`](enum.ParseError.html#variant.Error)
    /// then the message can't be parsed and the caller can try skipping over some data until
    /// parsing succeeds again.
    ///
    /// ## Parsing an RTSP message
    ///
    /// ```rust
    /// let data = b"OPTIONS * RTSP/2.0\r\n\
    ///              CSeq: 1\r\n\
    ///              Supported: play.basic, play.scale\r\n\
    ///              User-Agent: PhonyClient/1.2\r\n\
    ///              \r\n";
    ///
    /// let (message, consumed): (rtsp_types::Message<Vec<u8>>, _) =
    ///     rtsp_types::Message::parse(data).expect("Failed to parse data");
    ///
    /// assert_eq!(consumed, data.len());
    /// match message {
    ///     rtsp_types::Message::Request(ref request) => {
    ///         assert_eq!(request.method(), rtsp_types::Method::Options);
    ///     },
    ///     _ => unreachable!(),
    /// }
    /// ```
    pub fn parse<B: AsRef<[u8]> + 'a + ?Sized>(buf: &'a B) -> Result<(Self, usize), ParseError> {
        let buf = buf.as_ref();
        let (msg, consumed) = MessageRef::parse(buf)?;

        Ok((msg.to_owned()?, consumed))
    }
}

/// RTSP method.
///
/// See [RFC 7826 section 13](https://tools.ietf.org/html/rfc7826#section-13) for the details about
/// each method.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Method {
    /// Describe
    Describe,
    /// Get parameter
    GetParameter,
    /// Options
    Options,
    /// Pause
    Pause,
    /// Play
    Play,
    /// Play notify (only RTSP 2.0)
    PlayNotify,
    /// Redirect
    Redirect,
    /// Setup
    Setup,
    /// Set parameter
    SetParameter,
    /// Announce (only RTSP 1.0)
    Announce,
    /// Record (only RTSP 1.0)
    Record,
    /// Teardown
    Teardown,
    /// Extension method
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
            Method::Announce => MethodRef::Announce,
            Method::Record => MethodRef::Record,
            Method::Teardown => MethodRef::Teardown,
            Method::Extension(s) => MethodRef::Extension(s),
        }
    }
}

/// Parses a method from a `&str`.
impl<'a> From<&'a str> for Method {
    fn from(v: &'a str) -> Self {
        MethodRef::from(v).to_owned()
    }
}

/// Converts a method into a `&str`.
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
            Method::Announce => "ANNOUNCE",
            Method::Record => "RECORD",
            Method::Teardown => "TEARDOWN",
            Method::Extension(ref v) => v,
        }
    }
}

impl PartialEq<Method> for &Method {
    fn eq(&self, other: &Method) -> bool {
        (*self).eq(other)
    }
}

/// RTSP Request.
///
/// Represents an RTSP request and providers functions to construct, modify and read requests.
///
/// See [RFC 7826 section 7](https://tools.ietf.org/html/rfc7826#section-7) for the details about
/// methods.
///
/// ## Creating an `OPTIONS` request
///
/// ```rust
/// let request = rtsp_types::Request::builder(
///         rtsp_types::Method::Options,
///         rtsp_types::Version::V2_0
///     )
///     .header(rtsp_types::headers::CSEQ, "1")
///     .empty();
/// ```
///
/// This request contains an empty body.
///
/// ## Creating a `SET_PARAMETER` request with a request body
///
/// ```rust
/// let request = rtsp_types::Request::builder(
///         rtsp_types::Method::SetParameter,
///         rtsp_types::Version::V2_0
///     )
///     .request_uri(rtsp_types::Url::parse("rtsp://example.com/test").expect("Invalid URI"))
///     .header(rtsp_types::headers::CSEQ, "2")
///     .header(rtsp_types::headers::CONTENT_TYPE, "text/parameters")
///     .build(Vec::from(&b"barparam: barstuff"[..]));
/// ```
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
    /// Build a new `Request` for a given method and RTSP version.
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

    /// Serialize the request to any `std::io::Write`.
    ///
    /// Resuming writing after `std::io::ErrorKind::WouldBlock` is not supported. Any previously
    /// written data will have to be discarded for resuming.
    pub fn write<'b, W: std::io::Write + 'b>(&self, w: &'b mut W) -> Result<(), WriteError>
    where
        Body: AsRef<[u8]>,
    {
        self.borrow().write(w)
    }

    /// Calculate the number of bytes needed to serialize the request.
    pub fn write_len(&self) -> u64
    where
        Body: AsRef<[u8]>,
    {
        self.borrow().write_len()
    }

    // Accessors
    /// Get the method of the request.
    pub fn method(&self) -> &Method {
        &self.method
    }

    /// Set the method of the request.
    pub fn set_method(&mut self, method: Method) {
        self.method = method;
    }

    /// Get the request URI of the request.
    pub fn request_uri(&self) -> Option<&Url> {
        self.request_uri.as_ref()
    }

    /// Set the request URI of the request.
    pub fn set_request_uri(&mut self, request_uri: Option<Url>) {
        self.request_uri = request_uri;
    }

    /// Get the version of the request.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Set the version of the request.
    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    /// Get the body of the request.
    pub fn body(&self) -> &Body {
        &self.body
    }

    // Body API
    /// Convert the request into its body.
    pub fn into_body(self) -> Body {
        self.body
    }

    /// Modify the body of the request with a closure.
    ///
    /// This replaces the `Content-Length` header of the message with the length of the new body.
    pub fn map_body<NewBody: AsRef<[u8]>, F: FnOnce(Body) -> NewBody>(
        self,
        func: F,
    ) -> Request<NewBody> {
        let Request {
            method,
            request_uri,
            version,
            mut headers,
            body,
        } = self;

        let new_body = func(body);

        {
            let new_body = new_body.as_ref();
            if new_body.is_empty() {
                headers.remove(&crate::headers::CONTENT_LENGTH);
            } else {
                headers.insert(
                    crate::headers::CONTENT_LENGTH,
                    HeaderValue::from(format!("{}", new_body.len())),
                );
            }
        }

        Request {
            method,
            request_uri,
            version,
            headers,
            body: new_body,
        }
    }

    /// Replace the body of the request with a different body.
    ///
    /// This replaces the `Content-Length` header of the message with the length of the new body.
    pub fn replace_body<NewBody: AsRef<[u8]>>(self, new_body: NewBody) -> Request<NewBody> {
        let Request {
            method,
            request_uri,
            version,
            mut headers,
            body: _body,
        } = self;

        {
            let new_body = new_body.as_ref();
            if new_body.is_empty() {
                headers.remove(&crate::headers::CONTENT_LENGTH);
            } else {
                headers.insert(
                    crate::headers::CONTENT_LENGTH,
                    HeaderValue::from(format!("{}", new_body.len())),
                );
            }
        }

        Request {
            method,
            request_uri,
            version,
            headers,
            body: new_body,
        }
    }

    // Header API
    /// Appends a value to an existing RTSP header or inserts it.
    ///
    /// Additional values are comma separated as defined in [RFC 7826 section 5.2](https://tools.ietf.org/html/rfc7826#section-5.2).
    pub fn append_header<V: Into<HeaderValue>>(&mut self, name: HeaderName, value: V) {
        let value = value.into();
        self.headers.append(name, value);
    }

    /// Insert an RTSP header with its value.
    ///
    /// If a header with the same name already exists then its value will be replaced.
    ///
    /// See [`append`](#method.append) for appending additional values to a header.
    pub fn insert_header<V: Into<HeaderValue>>(&mut self, name: HeaderName, value: V) {
        let value = value.into();
        self.headers.insert(name, value);
    }

    /// Append a typed RTSP header with its value.
    pub fn append_typed_header<H: TypedAppendableHeader>(&mut self, header: &H) {
        self.headers.append_typed(header);
    }

    /// Insert a typed RTSP header with its value.
    ///
    /// If a header with the same name already exists then its value will be replaced.
    pub fn insert_typed_header<H: TypedHeader>(&mut self, header: &H) {
        self.headers.insert_typed(header);
    }

    /// Removes and RTSP header if it exists.
    pub fn remove_header(&mut self, name: &HeaderName) {
        self.headers.remove(name);
    }

    /// Gets an RTSP header value if it exists.
    pub fn header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Gets a typed RTSP header value if it exists.
    pub fn typed_header<H: TypedHeader>(&self) -> Result<Option<H>, headers::HeaderParseError> {
        self.headers.get_typed()
    }

    /// Gets a mutable reference to an RTSP header value if it exists.
    pub fn header_mut(&mut self, name: &HeaderName) -> Option<&mut HeaderValue> {
        self.headers.get_mut(name)
    }

    /// Iterator over all header name and value pairs.
    pub fn headers(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.headers.iter()
    }

    /// Iterator over all header names.
    pub fn header_names(&self) -> impl Iterator<Item = &HeaderName> {
        self.headers.names()
    }

    /// Iterator over all header values.
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

/// RTSP request builder.
///
/// See [`Request::builder`](struct.Request.html#method.builder) for details.
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

    /// Set the optional request URI.
    pub fn request_uri<U: Into<Url>>(self, request_uri: U) -> Self {
        Self(Request {
            request_uri: Some(request_uri.into()),
            ..self.0
        })
    }

    /// Append a header to the request.
    pub fn header<V: Into<HeaderValue>>(mut self, name: HeaderName, value: V) -> Self {
        let value = value.into();

        self.0.headers.append(name, value);

        self
    }

    /// Append a typed header to the request.
    pub fn typed_header<H: TypedHeader>(mut self, header: &H) -> Self {
        self.0.headers.insert_typed(header);

        self
    }

    /// Build a request with an empty body.
    pub fn empty(self) -> Request<Empty> {
        self.0
    }

    /// Build a request with a provided body.
    ///
    /// This inserts the `Content-Length` header with the length of the body if it is not empty.
    pub fn build<Body: AsRef<[u8]>>(mut self, body: Body) -> Request<Body> {
        {
            let body = body.as_ref();
            if !body.is_empty() {
                self.0.headers.insert(
                    crate::headers::CONTENT_LENGTH,
                    HeaderValue::from(format!("{}", body.len())),
                );
            }
        }

        Request {
            method: self.0.method,
            request_uri: self.0.request_uri,
            version: self.0.version,
            headers: self.0.headers,
            body,
        }
    }
}

/// RTSP Response.
///
/// Represents an RTSP response and providers functions to construct, modify and read responses.
///
/// See [RFC 7826 section 8](https://tools.ietf.org/html/rfc7826#section-8) for the details about
/// methods.
///
/// ## Creating an `OK` response
///
/// ```rust
/// let response = rtsp_types::Response::builder(
///         rtsp_types::Version::V2_0,
///         rtsp_types::StatusCode::Ok,
///     )
///     .header(rtsp_types::headers::CSEQ, "1")
///     .empty();
/// ```
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
    /// Build a new `Response` for a given RTSP version and status code.
    pub fn builder(version: Version, status: StatusCode) -> ResponseBuilder {
        ResponseBuilder::new(version, status)
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

    /// Serialize the response to any `std::io::Write`.
    ///
    /// Resuming writing after `std::io::ErrorKind::WouldBlock` is not supported. Any previously
    /// written data will have to be discarded for resuming.
    pub fn write<'b, W: std::io::Write + 'b>(&self, w: &'b mut W) -> Result<(), WriteError>
    where
        Body: AsRef<[u8]>,
    {
        self.borrow().write(w)
    }

    /// Calculate the number of bytes needed to serialize the response.
    pub fn write_len(&self) -> u64
    where
        Body: AsRef<[u8]>,
    {
        self.borrow().write_len()
    }

    // Accessors
    /// Get the version of the response.
    pub fn version(&self) -> Version {
        self.version
    }

    /// Set the version of the response.
    pub fn set_version(&mut self, version: Version) {
        self.version = version;
    }

    /// Get the status code of the response.
    pub fn status(&self) -> StatusCode {
        self.status
    }

    /// Set the status code of the response.
    pub fn set_status(&mut self, status: StatusCode) {
        self.status = status;
    }

    /// Get the reason phrase of the response.
    pub fn reason_phrase(&self) -> &str {
        self.reason_phrase.as_str()
    }

    /// Set the reason phrase of the response.
    pub fn set_reason_phrase<S: Into<String>>(&mut self, reason_phrase: S) {
        self.reason_phrase = reason_phrase.into();
    }

    /// Get the body of the response.
    pub fn body(&self) -> &Body {
        &self.body
    }

    // Body API
    /// Convert the response into its body.
    pub fn into_body(self) -> Body {
        self.body
    }

    /// Modify the body of the response with a closure.
    ///
    /// This replaces the `Content-Length` header of the message with the length of the new body.
    pub fn map_body<NewBody: AsRef<[u8]>, F: FnOnce(Body) -> NewBody>(
        self,
        func: F,
    ) -> Response<NewBody> {
        let Response {
            version,
            status,
            reason_phrase,
            mut headers,
            body,
        } = self;

        let new_body = func(body);

        {
            let new_body = new_body.as_ref();
            if new_body.is_empty() {
                headers.remove(&crate::headers::CONTENT_LENGTH);
            } else {
                headers.insert(
                    crate::headers::CONTENT_LENGTH,
                    HeaderValue::from(format!("{}", new_body.len())),
                );
            }
        }

        Response {
            version,
            status,
            reason_phrase,
            headers,
            body: new_body,
        }
    }

    /// Replace the body of the response with a different body.
    ///
    /// This replaces the `Content-Length` header of the message with the length of the new body.
    pub fn replace_body<NewBody: AsRef<[u8]>>(self, new_body: NewBody) -> Response<NewBody> {
        let Response {
            version,
            status,
            reason_phrase,
            mut headers,
            body: _body,
        } = self;

        {
            let new_body = new_body.as_ref();
            if new_body.is_empty() {
                headers.remove(&crate::headers::CONTENT_LENGTH);
            } else {
                headers.insert(
                    crate::headers::CONTENT_LENGTH,
                    HeaderValue::from(format!("{}", new_body.len())),
                );
            }
        }

        Response {
            version,
            status,
            reason_phrase,
            headers,
            body: new_body,
        }
    }

    // Header API
    /// Appends a value to an existing RTSP header or inserts it.
    ///
    /// Additional values are comma separated as defined in [RFC 7826 section 5.2](https://tools.ietf.org/html/rfc7826#section-5.2).
    pub fn append_header<V: Into<HeaderValue>>(&mut self, name: HeaderName, value: V) {
        let value = value.into();
        self.headers.append(name, value);
    }

    /// Insert an RTSP header with its value.
    ///
    /// If a header with the same name already exists then its value will be replaced.
    ///
    /// See [`append`](#method.append) for appending additional values to a header.
    pub fn insert_header<V: Into<HeaderValue>>(&mut self, name: HeaderName, value: V) {
        let value = value.into();
        self.headers.insert(name, value);
    }

    /// Append a typed RTSP header with its value.
    pub fn append_typed_header<H: TypedAppendableHeader>(&mut self, header: &H) {
        self.headers.append_typed(header);
    }

    /// Insert a typed RTSP header with its value.
    ///
    /// If a header with the same name already exists then its value will be replaced.
    pub fn insert_typed_header<H: TypedHeader>(&mut self, header: &H) {
        self.headers.insert_typed(header);
    }

    /// Removes and RTSP header if it exists.
    pub fn remove_header(&mut self, name: &HeaderName) {
        self.headers.remove(name);
    }

    /// Gets an RTSP header value if it exists.
    pub fn header(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.headers.get(name)
    }

    /// Gets a typed RTSP header value if it exists.
    pub fn typed_header<H: TypedHeader>(&self) -> Result<Option<H>, headers::HeaderParseError> {
        self.headers.get_typed()
    }

    /// Gets a mutable reference to an RTSP header value if it exists.
    pub fn header_mut(&mut self, name: &HeaderName) -> Option<&mut HeaderValue> {
        self.headers.get_mut(name)
    }

    /// Iterator over all header name and value pairs.
    pub fn headers(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.headers.iter()
    }

    /// Iterator over all header names.
    pub fn header_names(&self) -> impl Iterator<Item = &HeaderName> {
        self.headers.names()
    }

    /// Iterator over all header values.
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

/// RTSP response builder.
///
/// See [`Response::builder`](struct.Response.html#method.builder) for details.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ResponseBuilder(Response<Empty>, Option<String>);

impl ResponseBuilder {
    fn new(version: Version, status: StatusCode) -> Self {
        let response = Response {
            version,
            status,
            reason_phrase: String::new(),
            headers: Headers::new(),
            body: Empty,
        };

        Self(response, None)
    }

    /// Set the reason phrase of the response.
    ///
    /// If not set then a default reason phrase will be used based on the status code.
    pub fn reason_phrase<S: Into<String>>(mut self, reason_phrase: S) -> Self {
        let reason_phrase = reason_phrase.into();

        self.1 = Some(reason_phrase);

        self
    }

    /// Append a header to the response.
    pub fn header<V: Into<HeaderValue>>(mut self, name: HeaderName, value: V) -> Self {
        let value = value.into();

        self.0.headers.append(name, value);

        self
    }

    /// Append a typed header to the response.
    pub fn typed_header<H: TypedHeader>(mut self, header: &H) -> Self {
        self.0.headers.insert_typed(header);

        self
    }

    /// Build a response with an empty body.
    pub fn empty(self) -> Response<Empty> {
        let ResponseBuilder(mut response, reason_phrase) = self;

        response.reason_phrase = reason_phrase.unwrap_or_else(|| response.status.to_string());

        response
    }

    /// Build a response with a provided body.
    ///
    /// This inserts the `Content-Length` header with the length of the body if it is not empty.
    pub fn build<Body: AsRef<[u8]>>(self, body: Body) -> Response<Body> {
        let ResponseBuilder(mut response, reason_phrase) = self;

        {
            let body = body.as_ref();
            if !body.is_empty() {
                response.headers.insert(
                    crate::headers::CONTENT_LENGTH,
                    HeaderValue::from(format!("{}", body.len())),
                );
            }
        }

        let reason_phrase = reason_phrase.unwrap_or_else(|| response.status.to_string());

        Response {
            version: response.version,
            status: response.status,
            reason_phrase,
            headers: response.headers,
            body,
        }
    }
}

/// RTSP data message.
///
/// See [RFC 7826 section 14](https://tools.ietf.org/html/rfc7826#section-14) for details about the
/// data message.
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

    /// Create a new data message for a given channel id and body.
    pub fn new(channel_id: u8, body: Body) -> Self {
        Self { channel_id, body }
    }

    /// Serialize the data to any `std::io::Write`.
    ///
    /// Resuming writing after `std::io::ErrorKind::WouldBlock` is not supported. Any previously
    /// written data will have to be discarded for resuming.
    pub fn write<'b, W: std::io::Write + 'b>(&self, w: &'b mut W) -> Result<(), WriteError>
    where
        Body: AsRef<[u8]>,
    {
        self.borrow().write(w)
    }

    /// Calculate the number of bytes needed to serialize the data.
    pub fn write_len(&self) -> u64
    where
        Body: AsRef<[u8]>,
    {
        self.borrow().write_len()
    }

    // Accessors
    /// Get the channel id of the data message.
    pub fn channel_id(&self) -> u8 {
        self.channel_id
    }

    /// Set the channel id of the data message.
    pub fn set_channel_id(&mut self, channel_id: u8) {
        self.channel_id = channel_id;
    }

    /// Get the length of the data message.
    pub fn len(&self) -> usize
    where
        Body: AsRef<[u8]>,
    {
        self.body.as_ref().len()
    }

    /// Check if the body of the data message is empty.
    pub fn is_empty(&self) -> bool
    where
        Body: AsRef<[u8]>,
    {
        self.body.as_ref().is_empty()
    }

    /// Get a `&[u8]` slice for the body of the data message.
    pub fn as_slice(&self) -> &[u8]
    where
        Body: AsRef<[u8]>,
    {
        self.body.as_ref()
    }

    // Body API
    /// Convert the data message into its body.
    pub fn into_body(self) -> Body {
        self.body
    }

    /// Modify the body of the data message with a closure.
    pub fn map_body<NewBody, F: FnOnce(Body) -> NewBody>(self, func: F) -> Data<NewBody> {
        Data {
            channel_id: self.channel_id,
            body: func(self.body),
        }
    }

    /// Replace the body of the data message with a different body.
    pub fn replace_body<NewBody>(self, new_body: NewBody) -> Data<NewBody> {
        Data {
            channel_id: self.channel_id,
            body: new_body,
        }
    }
}

impl Data<Vec<u8>> {
    /// Create a new data message from a `Vec<u8>`.
    pub fn from_vec(channel_id: u8, body: Vec<u8>) -> Self {
        Self { channel_id, body }
    }
}

impl<Body: AsRef<[u8]>> AsRef<[u8]> for Data<Body> {
    fn as_ref(&self) -> &[u8] {
        self.body.as_ref()
    }
}
