// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

//! Crate for handling RTSP ([RFC 7826](https://tools.ietf.org/html/rfc7826))
//! messages, including a parser and serializer and support for parsing and generating
//! well-known headers.
//!
//! ## Creating an `OPTIONS` request
//!
//! ```rust
//! let request = rtsp_types::Request::builder(
//!         rtsp_types::Method::Options,
//!         rtsp_types::Version::V2_0
//!     )
//!     .header(rtsp_types::headers::CSEQ, "1")
//!     .empty();
//! ```
//!
//! This request contains an empty body.
//!
//! ## Creating a `SET_PARAMETER` request with a request body
//!
//! ```rust
//! let request = rtsp_types::Request::builder(
//!         rtsp_types::Method::SetParameter,
//!         rtsp_types::Version::V2_0
//!     )
//!     .request_uri(rtsp_types::Url::parse("rtsp://example.com/test").expect("Invalid URI"))
//!     .header(rtsp_types::headers::CSEQ, "2")
//!     .header(rtsp_types::headers::CONTENT_TYPE, "text/parameters")
//!     .build(Vec::from(&b"barparam: barstuff"[..]));
//! ```
//!
//! The body is passed to the `build()` function and a `Content-Length` header is automatically
//! inserted into the request headers.
//!
//! ## Creating an `OK` response
//!
//! ```rust
//! let response = rtsp_types::Response::builder(
//!         rtsp_types::Version::V2_0,
//!         rtsp_types::StatusCode::Ok,
//!     )
//!     .header(rtsp_types::headers::CSEQ, "1")
//!     .empty();
//! ```
//!
//! This response contains an empty body. A non-empty body can be added in the same way as for
//! requests.
//!
//! ## Creating a data message
//!
//! ```rust
//! let data = rtsp_types::Data::new(0, vec![0, 1, 2, 3, 4]);
//! ```
//!
//! This creates a new data message for channel id 0 with the given `Vec<u8>`.
//!
//! ## Parsing an RTSP message
//!
//! ```rust
//! let data = b"OPTIONS * RTSP/2.0\r\n\
//!              CSeq: 1\r\n\
//!              Supported: play.basic, play.scale\r\n\
//!              User-Agent: PhonyClient/1.2\r\n\
//!              \r\n";
//!
//! let (message, consumed): (rtsp_types::Message<Vec<u8>>, _) =
//!     rtsp_types::Message::parse(data).expect("Failed to parse data");
//!
//! assert_eq!(consumed, data.len());
//! match message {
//!     rtsp_types::Message::Request(ref request) => {
//!         assert_eq!(request.method(), rtsp_types::Method::Options);
//!     },
//!     _ => unreachable!(),
//! }
//! ```
//!
//! Messages can be parsed from any `AsRef<[u8]>` and to any borrowed or owned body type that
//! implements `From<&[u8]>`.
//!
//! More details about parsing can be found at [`Message::parse`](enum.Message.html#method.parse).
//!
//! ## Serializing an RTSP message
//!
//! ```rust
//! let request = rtsp_types::Request::builder(
//!         rtsp_types::Method::SetParameter,
//!         rtsp_types::Version::V2_0
//!     )
//!     .request_uri(rtsp_types::Url::parse("rtsp://example.com/test").expect("Invalid URI"))
//!     .header(rtsp_types::headers::CSEQ, "2")
//!     .header(rtsp_types::headers::CONTENT_TYPE, "text/parameters")
//!     .build(Vec::from(&b"barparam: barstuff"[..]));
//!
//!  let mut data = Vec::new();
//!  request.write(&mut data).expect("Failed to serialize request");
//!
//!  assert_eq!(
//!     data,
//!     b"SET_PARAMETER rtsp://example.com/test RTSP/2.0\r\n\
//!       Content-Length: 18\r\n\
//!       Content-Type: text/parameters\r\n\
//!       CSeq: 2\r\n\
//!       \r\n\
//!       barparam: barstuff",
//!  );
//! ```
//!
//! Serializing can be done to any type that implements `std::io::Write`.
//!
//! More details about serializing can be found at [`Message::write`](enum.Message.html#method.write).

mod message;
pub use message::*;
// TODO: Maybe make this public at a later time
mod message_ref;
pub(crate) use message_ref::*;
mod nom_extensions;
mod parser;
mod serializer;

pub mod headers;
pub use headers::{HeaderName, HeaderValue, Headers};

pub use url::Url;

use std::fmt;
use tinyvec::TinyVec;

/// RTSP protocol version of the message.
///
/// RTSP 1.0 is defined in [RFC 2326](https://tools.ietf.org/html/rfc2326), RTSP 2.0 is defined in
/// [RFC 7826](https://tools.ietf.org/html/rfc7826). Check the RFCs for the differences between the
/// two versions.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Version {
    /// RTSP/1.0
    V1_0,
    /// RTSP/2.0
    V2_0,
}

/// RTSP response status codes.
///
/// These are defined in [RFC 7826 section 17](https://tools.ietf.org/html/rfc7826#section-17)
/// together with their semantics for the different requests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusCode {
    /// Continue
    Continue,
    /// Ok
    Ok,
    /// Moved permanently
    MovedPermanently,
    /// Found
    Found,
    /// See other
    SeeOther,
    /// Not modified
    NotModified,
    /// Use proxy
    UseProxy,
    /// Bad request
    BadRequest,
    /// Unauthorized
    Unauthorized,
    /// Payment required
    PaymentRequired,
    /// Forbidden
    Forbidden,
    /// Not found
    NotFound,
    /// Method not allowed
    MethodNotAllowed,
    /// Not acceptable
    NotAcceptable,
    /// Proxy authentication required
    ProxyAuthenticationRequired,
    /// Request timeout
    RequestTimeout,
    /// Gone
    Gone,
    /// Precondition failed
    PreconditionFailed,
    /// Request message body too large
    RequestMessageBodyTooLarge,
    /// Request URI too long
    RequestURITooLong,
    /// Unsupported media type
    UnsupportedMediaType,
    /// Parameter not understood
    ParameterNotUnderstood,
    /// Reserved
    Reserved,
    /// Not enough bandwidth
    NotEnoughBandwidth,
    /// Session not found
    SessionNotFound,
    /// Method not valid in this state
    MethodNotValidInThisState,
    /// Header field not valid for resource
    HeaderFieldNotValidForResource,
    /// Invalid range
    InvalidRange,
    /// Parameter is read-only
    ParameterIsReadOnly,
    /// Aggregate operation not allowed
    AggregateOperationNotAllowed,
    /// Only aggregate operation allowed
    OnlyAggregateOperationAllowed,
    /// Unsupported transport
    UnsupportedTransport,
    /// Destination unreachable
    DestinationUnreachable,
    /// Destination prohibited
    DestinationProhibited,
    /// Data transport not ready yet
    DataTransportNotReadyYet,
    /// Notification reason unknown
    NotificationReasonUnknown,
    /// Key management error
    KeyManagementError,
    /// Connection authorization required
    ConnectionAuthorizationRequired,
    /// Connection credentials not accepted
    ConnectionCredentialsNotAccepted,
    /// Failure to establish secure connection
    FailureToEstablishSecureConnection,
    /// Internal server error
    InternalServerError,
    /// Not implemented
    NotImplemented,
    /// Bad gateway
    BadGateway,
    /// Service unavailable
    ServiceUnavailable,
    /// Gateway timeout
    GatewayTimeout,
    /// RTSP version not supported
    RTSPVersionNotSupported,
    /// Option not supported
    OptionNotSupported,
    /// Proxy unavailable
    ProxyUnavailable,
    /// Extension status code
    Extension(u16),
}

impl StatusCode {
    /// Returns `true` if the status code is `1xx`.
    pub fn is_informational(self) -> bool {
        let val = u16::from(self);

        (100..200).contains(&val)
    }

    /// Returns `true` if the status code is `2xx`.
    pub fn is_success(self) -> bool {
        let val = u16::from(self);

        (200..300).contains(&val)
    }

    /// Returns `true` if the status code is `3xx`.
    pub fn is_redirection(self) -> bool {
        let val = u16::from(self);

        (300..400).contains(&val)
    }

    /// Returns `true` if the status code is `4xx`.
    pub fn is_client_error(self) -> bool {
        let val = u16::from(self);

        (400..500).contains(&val)
    }

    /// Returns `true` if the status code is `5xx`.
    pub fn is_server_error(self) -> bool {
        let val = u16::from(self);

        (500..600).contains(&val)
    }
}

/// Converts from the numeric value of a `StatusCode`.
impl From<u16> for StatusCode {
    fn from(v: u16) -> Self {
        match v {
            100 => StatusCode::Continue,
            200 => StatusCode::Ok,
            301 => StatusCode::MovedPermanently,
            302 => StatusCode::Found,
            303 => StatusCode::SeeOther,
            304 => StatusCode::NotModified,
            305 => StatusCode::UseProxy,
            400 => StatusCode::BadRequest,
            401 => StatusCode::Unauthorized,
            402 => StatusCode::PaymentRequired,
            403 => StatusCode::Forbidden,
            404 => StatusCode::NotFound,
            405 => StatusCode::MethodNotAllowed,
            406 => StatusCode::NotAcceptable,
            407 => StatusCode::ProxyAuthenticationRequired,
            408 => StatusCode::RequestTimeout,
            410 => StatusCode::Gone,
            412 => StatusCode::PreconditionFailed,
            413 => StatusCode::RequestMessageBodyTooLarge,
            414 => StatusCode::RequestURITooLong,
            415 => StatusCode::UnsupportedMediaType,
            451 => StatusCode::ParameterNotUnderstood,
            452 => StatusCode::Reserved,
            453 => StatusCode::NotEnoughBandwidth,
            454 => StatusCode::SessionNotFound,
            455 => StatusCode::MethodNotValidInThisState,
            456 => StatusCode::HeaderFieldNotValidForResource,
            457 => StatusCode::InvalidRange,
            458 => StatusCode::ParameterIsReadOnly,
            459 => StatusCode::AggregateOperationNotAllowed,
            460 => StatusCode::OnlyAggregateOperationAllowed,
            461 => StatusCode::UnsupportedTransport,
            462 => StatusCode::DestinationUnreachable,
            463 => StatusCode::DestinationProhibited,
            464 => StatusCode::DataTransportNotReadyYet,
            465 => StatusCode::NotificationReasonUnknown,
            466 => StatusCode::KeyManagementError,
            470 => StatusCode::ConnectionAuthorizationRequired,
            471 => StatusCode::ConnectionCredentialsNotAccepted,
            472 => StatusCode::FailureToEstablishSecureConnection,
            500 => StatusCode::InternalServerError,
            501 => StatusCode::NotImplemented,
            502 => StatusCode::BadGateway,
            503 => StatusCode::ServiceUnavailable,
            504 => StatusCode::GatewayTimeout,
            505 => StatusCode::RTSPVersionNotSupported,
            551 => StatusCode::OptionNotSupported,
            553 => StatusCode::ProxyUnavailable,
            v => StatusCode::Extension(v),
        }
    }
}

/// Converts to the numeric value of a `StatusCode`.
impl From<StatusCode> for u16 {
    fn from(v: StatusCode) -> Self {
        match v {
            StatusCode::Continue => 100,
            StatusCode::Ok => 200,
            StatusCode::MovedPermanently => 301,
            StatusCode::Found => 302,
            StatusCode::SeeOther => 303,
            StatusCode::NotModified => 304,
            StatusCode::UseProxy => 305,
            StatusCode::BadRequest => 400,
            StatusCode::Unauthorized => 401,
            StatusCode::PaymentRequired => 402,
            StatusCode::Forbidden => 403,
            StatusCode::NotFound => 404,
            StatusCode::MethodNotAllowed => 405,
            StatusCode::NotAcceptable => 406,
            StatusCode::ProxyAuthenticationRequired => 407,
            StatusCode::RequestTimeout => 408,
            StatusCode::Gone => 410,
            StatusCode::PreconditionFailed => 412,
            StatusCode::RequestMessageBodyTooLarge => 413,
            StatusCode::RequestURITooLong => 414,
            StatusCode::UnsupportedMediaType => 415,
            StatusCode::ParameterNotUnderstood => 451,
            StatusCode::Reserved => 452,
            StatusCode::NotEnoughBandwidth => 453,
            StatusCode::SessionNotFound => 454,
            StatusCode::MethodNotValidInThisState => 455,
            StatusCode::HeaderFieldNotValidForResource => 456,
            StatusCode::InvalidRange => 456,
            StatusCode::ParameterIsReadOnly => 458,
            StatusCode::AggregateOperationNotAllowed => 459,
            StatusCode::OnlyAggregateOperationAllowed => 460,
            StatusCode::UnsupportedTransport => 461,
            StatusCode::DestinationUnreachable => 462,
            StatusCode::DestinationProhibited => 463,
            StatusCode::DataTransportNotReadyYet => 464,
            StatusCode::NotificationReasonUnknown => 465,
            StatusCode::KeyManagementError => 466,
            StatusCode::ConnectionAuthorizationRequired => 470,
            StatusCode::ConnectionCredentialsNotAccepted => 471,
            StatusCode::FailureToEstablishSecureConnection => 472,
            StatusCode::InternalServerError => 500,
            StatusCode::NotImplemented => 501,
            StatusCode::BadGateway => 502,
            StatusCode::ServiceUnavailable => 503,
            StatusCode::GatewayTimeout => 504,
            StatusCode::RTSPVersionNotSupported => 505,
            StatusCode::OptionNotSupported => 551,
            StatusCode::ProxyUnavailable => 553,
            StatusCode::Extension(v) => v,
        }
    }
}

/// Provides the default reason phrase for the `StatusCode`.
impl fmt::Display for StatusCode {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StatusCode::Continue => write!(fmt, "Continue"),
            StatusCode::Ok => write!(fmt, "Ok"),
            StatusCode::MovedPermanently => write!(fmt, "Moved Permanently"),
            StatusCode::Found => write!(fmt, "Found"),
            StatusCode::SeeOther => write!(fmt, "See Other"),
            StatusCode::NotModified => write!(fmt, "Not Modified"),
            StatusCode::UseProxy => write!(fmt, "Use Proxy"),
            StatusCode::BadRequest => write!(fmt, "Bad Request"),
            StatusCode::Unauthorized => write!(fmt, "Unauthorized"),
            StatusCode::PaymentRequired => write!(fmt, "Payment Required"),
            StatusCode::Forbidden => write!(fmt, "Forbidden"),
            StatusCode::NotFound => write!(fmt, "Not Found"),
            StatusCode::MethodNotAllowed => write!(fmt, "Method Not Allowed"),
            StatusCode::NotAcceptable => write!(fmt, "Not Acceptable"),
            StatusCode::ProxyAuthenticationRequired => write!(fmt, "Proxy Authentication Required"),
            StatusCode::RequestTimeout => write!(fmt, "Request Timeout"),
            StatusCode::Gone => write!(fmt, "Gone"),
            StatusCode::PreconditionFailed => write!(fmt, "Precondition Failed"),
            StatusCode::RequestMessageBodyTooLarge => write!(fmt, "Request Message Body Too Large"),
            StatusCode::RequestURITooLong => write!(fmt, "Request URI Too Long"),
            StatusCode::UnsupportedMediaType => write!(fmt, "Unsupported Media Type"),
            StatusCode::ParameterNotUnderstood => write!(fmt, "Parameter Not Understood"),
            StatusCode::Reserved => write!(fmt, "Reserved"),
            StatusCode::NotEnoughBandwidth => write!(fmt, "Not Enough Bandwidth"),
            StatusCode::SessionNotFound => write!(fmt, "Session Not Found"),
            StatusCode::MethodNotValidInThisState => write!(fmt, "Method Not Valid In This State"),
            StatusCode::HeaderFieldNotValidForResource => {
                write!(fmt, "Header Field Not Valid For Resource")
            }
            StatusCode::InvalidRange => write!(fmt, "Invalid Range"),
            StatusCode::ParameterIsReadOnly => write!(fmt, "Parameter Is Read-Only"),
            StatusCode::AggregateOperationNotAllowed => {
                write!(fmt, "Aggregate Operation Not Allowed")
            }
            StatusCode::OnlyAggregateOperationAllowed => {
                write!(fmt, "Only Aggregate Operation ALlowed")
            }
            StatusCode::UnsupportedTransport => write!(fmt, "Unsupported Transport"),
            StatusCode::DestinationUnreachable => write!(fmt, "Destination Unreachable"),
            StatusCode::DestinationProhibited => write!(fmt, "Destination Prohibited"),
            StatusCode::DataTransportNotReadyYet => write!(fmt, "Data Transport Not Ready Yet"),
            StatusCode::NotificationReasonUnknown => write!(fmt, "Notification Reason Unknown"),
            StatusCode::KeyManagementError => write!(fmt, "Key Management Error"),
            StatusCode::ConnectionAuthorizationRequired => {
                write!(fmt, "Connection Authorization Required")
            }
            StatusCode::ConnectionCredentialsNotAccepted => {
                write!(fmt, "Connection Credentials Not Accepted")
            }
            StatusCode::FailureToEstablishSecureConnection => {
                write!(fmt, "Failure To Establish Secure Connection")
            }
            StatusCode::InternalServerError => write!(fmt, "Internal Server Error"),
            StatusCode::NotImplemented => write!(fmt, "Not Implemented"),
            StatusCode::BadGateway => write!(fmt, "Bad Gateway"),
            StatusCode::ServiceUnavailable => write!(fmt, "Service Unavailable"),
            StatusCode::GatewayTimeout => write!(fmt, "Gateway Timeout"),
            StatusCode::RTSPVersionNotSupported => write!(fmt, "RTSP Version Not Supported"),
            StatusCode::OptionNotSupported => write!(fmt, "Option Not Supported"),
            StatusCode::ProxyUnavailable => write!(fmt, "Proxy Unavailable"),
            StatusCode::Extension(v) => write!(fmt, "Extension {}", v),
        }
    }
}

/// Empty body.
///
/// This can be used as the `Response` or `Request` body in place of a `&[]`
/// to signal an empty body.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Empty;

impl AsRef<[u8]> for Empty {
    fn as_ref(&self) -> &[u8] {
        &[]
    }
}

/// Message parsing error.
// TODO: Distinguish more errors and provide more information!
#[derive(Debug)]
pub enum ParseError {
    /// Parsing failed irrecoverably.
    Error,
    /// Message was not complete and more data is required.
    Incomplete,
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match *self {
            ParseError::Error => write!(f, "Parse Error"),
            ParseError::Incomplete => write!(f, "Incomplete message"),
        }
    }
}

/// Serialization write error.
// TODO: Distinguish more errors and provide more information!
#[derive(Debug)]
pub enum WriteError {
    /// Error reported by the underlying IO type
    IoError(std::io::Error),
}

impl std::error::Error for WriteError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WriteError::IoError(ref err) => Some(err),
        }
    }
}

impl std::fmt::Display for WriteError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match *self {
            WriteError::IoError(ref error) => write!(f, "Write IO error: {}", error),
        }
    }
}

impl From<std::io::Error> for WriteError {
    fn from(v: std::io::Error) -> Self {
        WriteError::IoError(v)
    }
}
