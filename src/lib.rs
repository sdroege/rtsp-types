// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

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

use smallvec::SmallVec;
use std::fmt;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Version {
    V1_0,
    V2_0,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusCode {
    Continue,
    Ok,
    MovedPermanently,
    Found,
    SeeOther,
    NotModified,
    UseProxy,
    BadRequest,
    Unauthorized,
    PaymentRequired,
    Forbidden,
    NotFound,
    MethodNotAllowed,
    NotAcceptable,
    ProxyAuthenticationRequired,
    RequestTimeout,
    Gone,
    PreconditionFailed,
    RequestMessageBodyTooLarge,
    RequestURITooLong,
    UnsupportedMediaType,
    ParameterNotUnderstood,
    Reserved,
    NotEnoughBandwidth,
    SessionNotFound,
    MethodNotValidInThisState,
    HeaderFieldNotValidForResource,
    InvalidRange,
    ParameterIsReadOnly,
    AggregateOperationNotAllowed,
    OnlyAggregateOperationAllowed,
    UnsupportedTransport,
    DestinationUnreachable,
    DestinationProhibited,
    DataTransportNotReadyYet,
    NotificationReasonUnknown,
    KeyManagementError,
    ConnectionAuthorizationRequired,
    ConnectionCredentialsNotAccepted,
    FailureToEstablishSecureConnection,
    InternalServerError,
    NotImplemented,
    BadGateway,
    ServiceUnavailable,
    GatewayTimeout,
    RTSPVersionNotSupported,
    OptionNotSupported,
    ProxyUnavailable,
    Extension(u16),
}

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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Empty;

impl AsRef<[u8]> for Empty {
    fn as_ref(&self) -> &[u8] {
        &[]
    }
}

// TODO: Distinguish more errors and provide more information!
#[derive(Debug)]
pub enum ParseError {
    Error,
    Incomplete,
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match *self {
            ParseError::Error => write!(f, "Parse Error"),
            ParseError::Incomplete => write!(f, "Incomplete"),
        }
    }
}

// TODO: Distinguish more errors and provide more information!
#[derive(Debug)]
pub enum WriteError {
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
            WriteError::IoError(ref error) => write!(f, "Write error: {}", error),
        }
    }
}

impl From<std::io::Error> for WriteError {
    fn from(v: std::io::Error) -> Self {
        WriteError::IoError(v)
    }
}
