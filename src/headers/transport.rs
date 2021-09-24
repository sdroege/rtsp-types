// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fmt;

/// `Transport` header ([RFC 7826 section 18.54](https://tools.ietf.org/html/rfc7826#section-18.54)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Transports(Vec<Transport>);

impl std::ops::Deref for Transports {
    type Target = Vec<Transport>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Transports {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Vec<Transport>> for Transports {
    fn as_ref(&self) -> &Vec<Transport> {
        &self.0
    }
}

impl AsMut<Vec<Transport>> for Transports {
    fn as_mut(&mut self) -> &mut Vec<Transport> {
        &mut self.0
    }
}

impl From<Vec<Transport>> for Transports {
    fn from(v: Vec<Transport>) -> Self {
        Transports(v)
    }
}

impl<'a> From<&'a [Transport]> for Transports {
    fn from(v: &'a [Transport]) -> Self {
        Transports(v.to_vec())
    }
}

/// Transport.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Transport {
    /// RTP media transport.
    Rtp(RtpTransport),
    /// Other transport.
    Other(OtherTransport),
}

/// RTP profiles.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RtpProfile {
    /// Audio/video profile.
    Avp,
    /// Audio/video profile with feedback (RTCP).
    AvpF,
    /// Secure (SRTP) audio/video profile.
    SAvp,
    /// Secure (SRTP) audio/video profile with feedback (RTCP).
    SAvpF,
    /// Other RTP profile.
    Other(String),
}

impl RtpProfile {
    /// Return profile as `&str`.
    pub fn as_str(&self) -> &str {
        match self {
            RtpProfile::Avp => "AVP",
            RtpProfile::AvpF => "AVPF",
            RtpProfile::SAvp => "SAVP",
            RtpProfile::SAvpF => "SAVPF",
            RtpProfile::Other(other) => other,
        }
    }
}

impl fmt::Display for RtpProfile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> From<&'a str> for RtpProfile {
    fn from(profile: &'a str) -> RtpProfile {
        match profile {
            "AVP" => RtpProfile::Avp,
            "AVPF" => RtpProfile::AvpF,
            "SAVP" => RtpProfile::SAvp,
            "SAVPF" => RtpProfile::SAvpF,
            other => RtpProfile::Other(other.into()),
        }
    }
}

/// RTP transport description.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RtpTransport {
    /// RTP profile.
    pub profile: RtpProfile,
    /// RTP lower transport.
    pub lower_transport: Option<RtpLowerTransport>,
    /// RTP transport parameters.
    pub params: RtpTransportParameters,
}

/// RTP transport parameters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RtpTransportParameters {
    /// Unicast transport.
    pub unicast: bool,
    /// Multicast transport.
    pub multicast: bool,
    /// TCP/interleaved transport channels.
    pub interleaved: Option<(u8, Option<u8>)>,
    /// Multicast packet time-to-live.
    pub ttl: Option<u8>,
    // TODO layers
    /// Stream SSRCs if known.
    pub ssrc: Vec<u32>,
    /// Transport mode.
    pub mode: Vec<TransportMode>,
    /// RTP and RTCP are muxed on the same transport channel.
    pub rtcp_mux: bool,
    /// Destination addresses. RTSP 2.0 only.
    pub dest_addr: Vec<String>,
    /// Source addresses. RTSP 2.0 only.
    pub src_addr: Vec<String>,
    /// Append to the resource. RTSP 1.0 RECORD mode only.
    pub append: bool,
    /// RTP/RTCP port for multicast. RTSP 1.0 only.
    pub port: Option<(u16, Option<u16>)>,
    /// Server RTP/RTCP port for unicast. RTSP 1.0 only.
    pub client_port: Option<(u16, Option<u16>)>,
    /// Server RTP/RTCP port for unicast. RTSP 1.0 only.
    pub server_port: Option<(u16, Option<u16>)>,
    /// Destination address. RTSP 1.0 only.
    pub destination: Option<String>,
    /// Source address. RTSP 1.0 only.
    pub source: Option<String>,
    // TODO: setup, connection
    // TODO mikey
    /// Other parameters.
    ///
    /// These are raw parameter strings, i.e. they might be quoted strings.
    pub others: BTreeMap<String, Option<String>>,
}

impl TryFrom<TransportParameters> for RtpTransportParameters {
    type Error = HeaderParseError;

    fn try_from(params: TransportParameters) -> Result<RtpTransportParameters, HeaderParseError> {
        let mut rtp_params = RtpTransportParameters::default();

        for (name, value) in params.0 {
            match name.as_str() {
                "unicast" => {
                    rtp_params.unicast = true;
                }
                "multicast" => {
                    rtp_params.multicast = true;
                }
                "interleaved" => {
                    let channels = value.ok_or(HeaderParseError)?;
                    let mut channels = channels.splitn(2, '-');

                    let channel_start = channels
                        .next()
                        .and_then(|s| s.parse::<u8>().ok())
                        .ok_or(HeaderParseError)?;

                    let channel_end = channels
                        .next()
                        .map(|s| s.parse::<u8>().map_err(|_| HeaderParseError))
                        .transpose()?;

                    rtp_params.interleaved = Some((channel_start, channel_end));
                }
                "ttl" => {
                    let ttl = value
                        .and_then(|s| s.parse::<u8>().ok())
                        .ok_or(HeaderParseError)?;

                    rtp_params.ttl = Some(ttl);
                }
                "ssrc" => {
                    let ssrc = value
                        .ok_or(HeaderParseError)?
                        .split('/')
                        .map(|s| u32::from_str_radix(s, 16).map_err(|_| HeaderParseError))
                        .collect::<Result<Vec<_>, _>>()?;

                    if ssrc.is_empty() {
                        return Err(HeaderParseError);
                    }

                    rtp_params.ssrc = ssrc;
                }
                "mode" => {
                    let modes = value.ok_or(HeaderParseError)?;

                    if !modes.starts_with('"') || !modes.ends_with('"') {
                        return Err(HeaderParseError);
                    }

                    let modes = &modes[1..(modes.len() - 1)];

                    let modes = modes
                        .split(',')
                        .map(TransportMode::from)
                        .collect::<Vec<_>>();

                    if modes.is_empty() {
                        return Err(HeaderParseError);
                    }

                    rtp_params.mode = modes;
                }
                "dest_addr" | "src_addr" => {
                    let addrs = value
                        .ok_or(HeaderParseError)?
                        .split('/')
                        .map(|s| {
                            if !s.starts_with('"') || !s.ends_with('"') {
                                return Err(HeaderParseError);
                            }

                            // Unescape quoted string
                            let mut res = Vec::with_capacity(s.len());
                            let mut s = s.as_bytes();
                            s = &s[1..(s.len() - 1)];
                            while !s.is_empty() {
                                if s.starts_with(b"\\") {
                                    assert!(s.len() >= 2);
                                    res.push(s[1]);
                                    s = &s[2..];
                                } else {
                                    res.push(s[0]);
                                    s = &s[1..];
                                }
                            }

                            // Can't really fail, must've been ASCII
                            Ok(String::from_utf8(res).expect("invalid UTF-8"))
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    if addrs.is_empty() {
                        return Err(HeaderParseError);
                    }

                    if name == "src_addr" {
                        rtp_params.src_addr = addrs;
                    } else {
                        rtp_params.dest_addr = addrs;
                    }
                }
                "port" | "server_port" | "client_port" => {
                    let ports = value.ok_or(HeaderParseError)?;
                    let mut ports = ports.splitn(2, '-');

                    let port_start = ports
                        .next()
                        .and_then(|s| s.parse::<u16>().ok())
                        .ok_or(HeaderParseError)?;

                    let port_end = ports
                        .next()
                        .map(|s| s.parse::<u16>().map_err(|_| HeaderParseError))
                        .transpose()?;

                    if name == "port" {
                        rtp_params.port = Some((port_start, port_end));
                    } else if name == "server_port" {
                        rtp_params.server_port = Some((port_start, port_end));
                    } else {
                        rtp_params.client_port = Some((port_start, port_end));
                    }
                }
                "destination" => {
                    rtp_params.destination = Some(value.ok_or(HeaderParseError)?);
                }
                "source" => {
                    rtp_params.source = Some(value.ok_or(HeaderParseError)?);
                }
                "append" => {
                    rtp_params.append = true;
                }
                "RTCP-mux" => {
                    rtp_params.rtcp_mux = true;
                }
                _ => {
                    rtp_params.others.insert(name, value);
                }
            }
        }

        Ok(rtp_params)
    }
}

/// Lower RTP transport protocol.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RtpLowerTransport {
    /// TCP.
    Tcp,
    /// UDP.
    Udp,
    /// Other transport protocol.
    Other(String),
}

impl<'a> From<&'a str> for RtpLowerTransport {
    fn from(lower_transport: &'a str) -> RtpLowerTransport {
        match lower_transport {
            "TCP" => RtpLowerTransport::Tcp,
            "UDP" => RtpLowerTransport::Udp,
            other => RtpLowerTransport::Other(other.into()),
        }
    }
}

impl RtpLowerTransport {
    /// Return RTP lower transport as `&str`.
    pub fn as_str(&self) -> &str {
        match self {
            RtpLowerTransport::Tcp => "TCP",
            RtpLowerTransport::Udp => "UDP",
            RtpLowerTransport::Other(other) => other,
        }
    }
}

impl fmt::Display for RtpLowerTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Transport mode.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransportMode {
    /// Play mode.
    Play,
    /// Record mode.
    Record,
    /// Other mode.
    Other(String),
}

impl<'a> From<&'a str> for TransportMode {
    fn from(transport_mode: &'a str) -> TransportMode {
        match transport_mode {
            "PLAY" => TransportMode::Play,
            "RECORD" => TransportMode::Record,
            other => TransportMode::Other(other.into()),
        }
    }
}

impl TransportMode {
    /// Return transport mode as `&str`.
    pub fn as_str(&self) -> &str {
        match self {
            TransportMode::Play => "PLAY",
            TransportMode::Record => "RECORD",
            TransportMode::Other(other) => other,
        }
    }
}

impl fmt::Display for TransportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Other transport description.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OtherTransport {
    /// Transport specification.
    pub spec: String,
    /// Other parameters.
    ///
    /// These are raw parameter strings, i.e. they might be quoted strings.
    pub params: TransportParameters,
}

/// Transport parameters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TransportParameters(pub BTreeMap<String, Option<String>>);

mod parser {
    use super::*;

    use super::parser_helpers::{cond_parser, rtsp_unreserved, token, trim};
    use nom::branch::alt;
    use nom::bytes::complete::tag;
    use nom::combinator::{all_consuming, map_res};
    use nom::multi::{fold_many0, separated_list1};
    use nom::sequence::{preceded, tuple};
    use nom::{Err, IResult, Needed};
    use std::str;

    // Check for `"[spaces]/[spaces]"` and return how much to skip
    fn is_address_list_separator(i: &[u8]) -> Option<usize> {
        let mut o = i;

        if !o.starts_with(b"\"") {
            return None;
        }
        o = &o[1..];

        while o.starts_with(b" ") || o.starts_with(b"\t") {
            o = &o[1..];
        }

        if !o.starts_with(b"/") {
            return None;
        }
        o = &o[1..];

        while o.starts_with(b" ") || o.starts_with(b"\t") {
            o = &o[1..];
        }

        if o.starts_with(b"\"") {
            Some(i.len() - o.len() + 1)
        } else {
            None
        }
    }

    fn quoted_string_or_address_list(input: &[u8]) -> IResult<&[u8], &[u8]> {
        use std::num::NonZeroUsize;

        if !input.starts_with(b"\"") {
            return Err(Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Tag,
            )));
        }

        let i = &input[1..];
        let mut o = i;

        while !o.is_empty() {
            if o.len() >= 2 && o.starts_with(b"\\") {
                o = &o[2..];
            } else if o.starts_with(b"\\") {
                return Err(Err::Incomplete(Needed::Size(NonZeroUsize::new(1).unwrap())));
            } else if !o.starts_with(b"\"") {
                o = &o[1..];
            } else if let Some(skip) = is_address_list_separator(o) {
                // Address list
                o = &o[skip..];
            } else {
                // Closing quote, also include it
                o = &o[1..];
                break;
            }
        }

        let (fst, snd) = input.split_at(input.len() - o.len());
        // Did not end with a quote
        if !fst.ends_with(b"\"") {
            return Err(Err::Incomplete(Needed::Size(NonZeroUsize::new(1).unwrap())));
        }

        // Must have the starting quote
        assert!(fst.starts_with(b"\""));

        Ok((snd, fst))
    }

    fn parameter(input: &[u8]) -> IResult<&[u8], (&str, Option<&str>)> {
        if input.is_empty() {
            return Err(Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }

        tuple((
            trim(map_res(token, str::from_utf8)),
            cond_parser(
                tag(b"="),
                trim(map_res(
                    alt((quoted_string_or_address_list, rtsp_unreserved)),
                    str::from_utf8,
                )),
            ),
        ))(input)
    }

    fn parameters(input: &[u8]) -> IResult<&[u8], TransportParameters> {
        fold_many0(
            preceded(trim(tag(b";")), parameter),
            || TransportParameters(BTreeMap::new()),
            |mut acc, (name, value)| {
                // FIXME: We assume each parameter appears only once
                acc.0.insert(name.into(), value.map(String::from));
                acc
            },
        )(input)
    }

    fn spec(input: &[u8]) -> IResult<&[u8], Vec<&str>> {
        separated_list1(tag(b"/"), map_res(trim(token), str::from_utf8))(input)
    }

    fn transport(input: &[u8]) -> IResult<&[u8], Transport> {
        map_res(tuple((spec, parameters)), |(spec, params)| {
            match spec.as_slice() {
                ["RTP", profile, lower_transport] => {
                    let profile = RtpProfile::from(*profile);
                    let lower_transport = Some(RtpLowerTransport::from(*lower_transport));
                    let params = match RtpTransportParameters::try_from(params) {
                        Ok(params) => params,
                        Err(err) => return Err(err),
                    };

                    Ok(Transport::Rtp(RtpTransport {
                        profile,
                        lower_transport,
                        params,
                    }))
                }
                ["RTP", profile] => {
                    let profile = RtpProfile::from(*profile);
                    let params = RtpTransportParameters::try_from(params)?;

                    Ok(Transport::Rtp(RtpTransport {
                        profile,
                        lower_transport: None,
                        params,
                    }))
                }
                other => Ok(Transport::Other(OtherTransport {
                    spec: other.iter().copied().map(String::from).collect(),
                    params,
                })),
            }
        })(input)
    }

    pub(super) fn transports(input: &[u8]) -> IResult<&[u8], Vec<Transport>> {
        all_consuming(separated_list1(tag(b","), transport))(input)
    }
}

impl super::TypedHeader for Transports {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&TRANSPORT) {
            None => return Ok(None),
            Some(header) => header,
        };

        let (_rem, transport) =
            parser::transports(header.as_str().as_bytes()).map_err(|_| HeaderParseError)?;

        Ok(Some(transport.into()))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        use std::fmt::Write;

        let headers = headers.as_mut();

        let mut transports = String::new();
        for transport in &self.0 {
            if !transports.is_empty() {
                transports.push(',');
            }

            match transport {
                Transport::Rtp(rtp) => {
                    transports.push_str("RTP/");
                    transports.push_str(rtp.profile.as_str());
                    if let Some(lower_transport) = &rtp.lower_transport {
                        transports.push('/');
                        transports.push_str(lower_transport.as_str());
                    }

                    if rtp.params.unicast {
                        transports.push(';');
                        transports.push_str("unicast");
                    }

                    if rtp.params.multicast {
                        transports.push(';');
                        transports.push_str("multicast");
                    }

                    if let Some((channel_start, channel_end)) = &rtp.params.interleaved {
                        transports.push(';');
                        write!(&mut transports, "interleaved={}", channel_start).unwrap();
                        if let Some(channel_end) = channel_end {
                            write!(&mut transports, "-{}", channel_end).unwrap();
                        }
                    }

                    if let Some(ttl) = rtp.params.ttl {
                        transports.push(';');
                        write!(&mut transports, "ttl={}", ttl).unwrap();
                    }

                    if !rtp.params.ssrc.is_empty() {
                        transports.push(';');

                        transports.push_str("ssrc=");
                        let mut first = true;
                        for ssrc in &rtp.params.ssrc {
                            if first {
                                first = false;
                            } else {
                                transports.push('/');
                            }

                            write!(&mut transports, "{:08X}", ssrc).unwrap();
                        }
                    }

                    if !rtp.params.dest_addr.is_empty() {
                        transports.push(';');

                        transports.push_str("dest_addr=");
                        let mut first = true;
                        for addr in &rtp.params.dest_addr {
                            if first {
                                first = false;
                            } else {
                                transports.push('/');
                            }

                            write!(&mut transports, "\"{}\"", addr).unwrap()
                        }
                    }

                    if !rtp.params.src_addr.is_empty() {
                        transports.push(';');

                        transports.push_str("src_addr=");
                        let mut first = true;
                        for addr in &rtp.params.src_addr {
                            if first {
                                first = false;
                            } else {
                                transports.push('/');
                            }

                            write!(&mut transports, "\"{}\"", addr).unwrap()
                        }
                    }

                    if rtp.params.append {
                        transports.push(';');
                        transports.push_str("append");
                    }

                    if let Some((port_start, port_end)) = rtp.params.port {
                        transports.push(';');
                        write!(&mut transports, "port={}", port_start).unwrap();
                        if let Some(port_end) = port_end {
                            write!(&mut transports, "-{}", port_end).unwrap();
                        }
                    }

                    if let Some((port_start, port_end)) = rtp.params.client_port {
                        transports.push(';');
                        write!(&mut transports, "client_port={}", port_start).unwrap();
                        if let Some(port_end) = port_end {
                            write!(&mut transports, "-{}", port_end).unwrap();
                        }
                    }

                    if let Some((port_start, port_end)) = rtp.params.server_port {
                        transports.push(';');
                        write!(&mut transports, "server_port={}", port_start).unwrap();
                        if let Some(port_end) = port_end {
                            write!(&mut transports, "-{}", port_end).unwrap();
                        }
                    }

                    if let Some(ref destination) = rtp.params.destination {
                        transports.push(';');
                        write!(&mut transports, "destination={}", destination).unwrap();
                    }

                    if let Some(ref source) = rtp.params.source {
                        transports.push(';');
                        write!(&mut transports, "source={}", source).unwrap();
                    }

                    if !rtp.params.mode.is_empty() {
                        transports.push(';');

                        transports.push_str("mode=\"");
                        let mut first = true;
                        for mode in &rtp.params.mode {
                            if first {
                                first = false;
                            } else {
                                transports.push_str(", ");
                            }

                            transports.push_str(mode.as_str());
                        }

                        transports.push('"');
                    }

                    if rtp.params.rtcp_mux {
                        transports.push(';');
                        transports.push_str("RTCP-mux");
                    }

                    for (name, value) in &rtp.params.others {
                        transports.push(';');

                        if let Some(value) = value {
                            write!(&mut transports, "{}={}", name, value).unwrap();
                        } else {
                            transports.push_str(name);
                        }
                    }
                }
                Transport::Other(other) => {
                    transports.push_str(&other.spec);

                    for (name, value) in &other.params.0 {
                        transports.push(';');

                        if let Some(value) = value {
                            write!(&mut transports, "{}={}", name, value).unwrap();
                        } else {
                            transports.push_str(name);
                        }
                    }
                }
            }
        }

        headers.insert(TRANSPORT, transports);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport() {
        let header = "RTP/AVP;unicast;dest_addr=\"192.0.2.5:3456\"/\"192.0.2.5:3457\";src_addr=\"192.0.2.224:6256\"/\"192.0.2.224:6257\";mode=\"PLAY\"";
        let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .header(crate::headers::TRANSPORT, header)
            .empty();

        let transports = request
            .typed_header::<super::Transports>()
            .unwrap()
            .unwrap();

        assert_eq!(
            transports,
            vec![Transport::Rtp(RtpTransport {
                profile: super::RtpProfile::Avp,
                lower_transport: None,
                params: RtpTransportParameters {
                    unicast: true,
                    multicast: false,
                    dest_addr: vec!["192.0.2.5:3456".into(), "192.0.2.5:3457".into()],
                    src_addr: vec!["192.0.2.224:6256".into(), "192.0.2.224:6257".into()],
                    mode: vec![TransportMode::Play],
                    ..Default::default()
                },
            })]
            .into()
        );

        let request2 = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .typed_header(&transports)
            .empty();

        assert_eq!(request, request2);
    }

    #[test]
    fn test_transport_multicast() {
        let header = "RTP/AVP;multicast";
        let request = crate::Request::builder(crate::Method::Setup, crate::Version::V1_0)
            .header(crate::headers::TRANSPORT, header)
            .empty();

        let transports = request
            .typed_header::<super::Transports>()
            .unwrap()
            .unwrap();

        assert_eq!(
            transports,
            vec![Transport::Rtp(RtpTransport {
                profile: super::RtpProfile::Avp,
                lower_transport: None,
                params: RtpTransportParameters {
                    unicast: false,
                    multicast: true,
                    ..Default::default()
                },
            })]
            .into()
        );

        let request2 = crate::Request::builder(crate::Method::Setup, crate::Version::V1_0)
            .typed_header(&transports)
            .empty();

        assert_eq!(request, request2);
    }

    #[test]
    fn test_transport_v1() {
        let header = "RTP/AVP;unicast;client_port=42860-42861";
        let request = crate::Request::builder(crate::Method::Setup, crate::Version::V1_0)
            .header(crate::headers::TRANSPORT, header)
            .empty();

        let transports = request
            .typed_header::<super::Transports>()
            .unwrap()
            .unwrap();

        assert_eq!(
            transports,
            vec![Transport::Rtp(RtpTransport {
                profile: super::RtpProfile::Avp,
                lower_transport: None,
                params: RtpTransportParameters {
                    unicast: true,
                    multicast: false,
                    client_port: Some((42860, Some(42861))),
                    ..Default::default()
                },
            })]
            .into()
        );

        let request2 = crate::Request::builder(crate::Method::Setup, crate::Version::V1_0)
            .typed_header(&transports)
            .empty();

        assert_eq!(request, request2);
    }

    #[test]
    fn test_multiple_transports() {
        let header = "RTP/AVP;multicast;mode=\"PLAY\",RTP/AVP;unicast;dest_addr=\"192.0.2.5:3456\"/\"192.0.2.5:3457\";mode=\"PLAY\"";
        let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .header(crate::headers::TRANSPORT, header)
            .empty();

        let transports = request
            .typed_header::<super::Transports>()
            .unwrap()
            .unwrap();

        assert_eq!(
            transports,
            vec![
                Transport::Rtp(RtpTransport {
                    profile: super::RtpProfile::Avp,
                    lower_transport: None,
                    params: RtpTransportParameters {
                        multicast: true,
                        unicast: false,
                        mode: vec![TransportMode::Play],
                        ..Default::default()
                    },
                }),
                Transport::Rtp(RtpTransport {
                    profile: super::RtpProfile::Avp,
                    lower_transport: None,
                    params: RtpTransportParameters {
                        unicast: true,
                        multicast: false,
                        dest_addr: vec!["192.0.2.5:3456".into(), "192.0.2.5:3457".into()],
                        mode: vec![TransportMode::Play],
                        ..Default::default()
                    },
                })
            ]
            .into()
        );

        let request2 = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .typed_header(&transports)
            .empty();

        assert_eq!(request, request2);
    }
}
