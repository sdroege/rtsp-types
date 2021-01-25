// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

//! RTSP header definitions.
//!
//! See [RFC 7826 section 18](https://tools.ietf.org/html/rfc7826#section-18) for the standardized
//! headers and their semantics.

mod types;
pub use types::*;

mod constants;
pub use constants::*;

pub mod accept;
pub mod accept_ranges;
pub mod allow;
pub mod cseq;
pub mod features;
pub mod pipelined_requests;
pub mod public;
pub mod range;
pub mod require;
pub mod rtp_info;
pub mod session;
pub mod supported;
pub mod transport;
pub mod unsupported;

pub use accept::{Accept, MediaType, MediaTypeRange};
pub use accept_ranges::{AcceptRanges, RangeUnit};
pub use allow::Allow;
pub use cseq::CSeq;
pub use pipelined_requests::PipelinedRequests;
pub use public::Public;
pub use range::{NptRange, NptTime, Range, SmpteRange, SmpteTime, SmpteType, UtcRange, UtcTime};
pub use require::Require;
pub use rtp_info::RtpInfos;
pub use session::Session;
pub use supported::Supported;
pub use transport::{
    OtherTransport, RtpLowerTransport, RtpProfile, RtpTransport, RtpTransportParameters, Transport,
    TransportMode, TransportParameters, Transports,
};
pub use unsupported::Unsupported;
