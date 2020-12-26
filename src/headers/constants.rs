// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::HeaderName;

pub const ACCEPT: HeaderName = HeaderName::from_static_str_unchecked("Accept");
pub const ACCEPT_CREDENTIALS: HeaderName =
    HeaderName::from_static_str_unchecked("Accept-Credentials");
pub const ACCEPT_ENCODING: HeaderName = HeaderName::from_static_str_unchecked("Accept-Encoding");
pub const ACCEPT_LANGUAGE: HeaderName = HeaderName::from_static_str_unchecked("Accept-Language");
pub const ACCEPT_RANGES: HeaderName = HeaderName::from_static_str_unchecked("Accept-Ranges");
pub const ALLOW: HeaderName = HeaderName::from_static_str_unchecked("Allow");
pub const AUTHENTICATION_INFO: HeaderName =
    HeaderName::from_static_str_unchecked("Authentication-Info");
pub const AUTHORIZATION: HeaderName = HeaderName::from_static_str_unchecked("Authorization");
pub const BANDWIDTH: HeaderName = HeaderName::from_static_str_unchecked("Bandwidth");
pub const BLOCKSIZE: HeaderName = HeaderName::from_static_str_unchecked("Blocksize");
pub const CACHE_CONTROL: HeaderName = HeaderName::from_static_str_unchecked("Cache-Control");
pub const CONNECTION: HeaderName = HeaderName::from_static_str_unchecked("Connection");
pub const CONNECTION_CREDENTIALS: HeaderName =
    HeaderName::from_static_str_unchecked("Connection-Credentials");
pub const CONTENT_BASE: HeaderName = HeaderName::from_static_str_unchecked("Content-Base");
pub const CONTENT_ENCODING: HeaderName = HeaderName::from_static_str_unchecked("Content-Encoding");
pub const CONTENT_LANGUAGE: HeaderName = HeaderName::from_static_str_unchecked("Content-Language");
pub const CONTENT_LENGTH: HeaderName = HeaderName::from_static_str_unchecked("Content-Length");
pub const CONTENT_LOCATION: HeaderName = HeaderName::from_static_str_unchecked("Content-Location");
pub const CONTENT_TYPE: HeaderName = HeaderName::from_static_str_unchecked("Content-Type");
pub const CSEQ: HeaderName = HeaderName::from_static_str_unchecked("CSeq");
pub const DATE: HeaderName = HeaderName::from_static_str_unchecked("Date");
pub const EXPIRES: HeaderName = HeaderName::from_static_str_unchecked("Expires");
pub const FROM: HeaderName = HeaderName::from_static_str_unchecked("From");
pub const IF_MATCH: HeaderName = HeaderName::from_static_str_unchecked("If-Match");
pub const IF_MODIFIED_SINCE: HeaderName =
    HeaderName::from_static_str_unchecked("If-Modified-Since");
pub const IF_NONE_MATCH: HeaderName = HeaderName::from_static_str_unchecked("If-None-Match");
pub const LAST_MODIFIED: HeaderName = HeaderName::from_static_str_unchecked("Last-Modified");
pub const LOCATION: HeaderName = HeaderName::from_static_str_unchecked("Location");
pub const MEDIA_PROPERTIES: HeaderName = HeaderName::from_static_str_unchecked("Media-Properties");
pub const MEDIA_RANGE: HeaderName = HeaderName::from_static_str_unchecked("Media-Range");
pub const MTAG: HeaderName = HeaderName::from_static_str_unchecked("MTag");
pub const NOTIFY_REASON: HeaderName = HeaderName::from_static_str_unchecked("Notify-Reason");
pub const PIPELINED_REQUESTS: HeaderName =
    HeaderName::from_static_str_unchecked("Pipelined-Requests");
pub const PROXY_AUTHENTICATE: HeaderName =
    HeaderName::from_static_str_unchecked("Proxy-Authenticate");
pub const PROXY_AUTHENTICATION_INFO: HeaderName =
    HeaderName::from_static_str_unchecked("Proxy-Authentication-Info");
pub const PROXY_AUTHORIZATION: HeaderName =
    HeaderName::from_static_str_unchecked("Proxy-Authorization");
pub const PROXY_REQUIRE: HeaderName = HeaderName::from_static_str_unchecked("Proxy-Require");
pub const PROXY_SUPPORTED: HeaderName = HeaderName::from_static_str_unchecked("Proxy-Supported");
pub const PUBLIC: HeaderName = HeaderName::from_static_str_unchecked("Public");
pub const RANGE: HeaderName = HeaderName::from_static_str_unchecked("Range");
pub const REFERRER: HeaderName = HeaderName::from_static_str_unchecked("Referrer");
pub const REQUEST_STATUS: HeaderName = HeaderName::from_static_str_unchecked("Request-Status");
pub const REQUIRE: HeaderName = HeaderName::from_static_str_unchecked("Require");
pub const RETRY_AFTER: HeaderName = HeaderName::from_static_str_unchecked("Retry-After");
pub const RTP_INFO: HeaderName = HeaderName::from_static_str_unchecked("RTP-Info");
pub const SCALE: HeaderName = HeaderName::from_static_str_unchecked("Scale");
pub const SEEK_STYLE: HeaderName = HeaderName::from_static_str_unchecked("Seek-Style");
pub const SERVER: HeaderName = HeaderName::from_static_str_unchecked("Server");
pub const SESSION: HeaderName = HeaderName::from_static_str_unchecked("Session");
pub const SPEED: HeaderName = HeaderName::from_static_str_unchecked("Speed");
pub const SUPPORTED: HeaderName = HeaderName::from_static_str_unchecked("Supported");
pub const TERMINATE_REASON: HeaderName = HeaderName::from_static_str_unchecked("Terminate-Reason");
pub const TIMESTAMP: HeaderName = HeaderName::from_static_str_unchecked("Timestamp");
pub const TRANSPORT: HeaderName = HeaderName::from_static_str_unchecked("Transport");
pub const UNSUPPORTED: HeaderName = HeaderName::from_static_str_unchecked("Unsupported");
pub const USER_AGENT: HeaderName = HeaderName::from_static_str_unchecked("User-Agent");
pub const VIA: HeaderName = HeaderName::from_static_str_unchecked("Via");
pub const WWW_AUTHENTICATE: HeaderName = HeaderName::from_static_str_unchecked("WWW-Authenticate");
