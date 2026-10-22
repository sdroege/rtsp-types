// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use super::parser_helpers::parse_digits;
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::fmt;

/// `Transport` header ([RFC 7826 section 18.54](https://tools.ietf.org/html/rfc7826#section-18.54)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[allow(clippy::large_enum_variant)]
pub enum Transport {
    /// RTP media transport.
    Rtp(RtpTransport),
    /// Other transport.
    Other(OtherTransport),
}

/// RTP profiles.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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

/// Transport connection setup direction. RTSP 2.0 only.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TransportSetup {
    /// The client initiates an outgoing connection.
    Active,
    /// The client accepts an incoming connection.
    Passive,
    /// The client accepts an incoming or initiates an outgoing connection.
    Actpass,
}

impl TransportSetup {
    /// Return setup as `&str`.
    pub fn as_str(&self) -> &str {
        match self {
            TransportSetup::Active => "active",
            TransportSetup::Passive => "passive",
            TransportSetup::Actpass => "actpass",
        }
    }
}

impl fmt::Display for TransportSetup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> TryFrom<&'a str> for TransportSetup {
    type Error = HeaderParseError;

    fn try_from(setup: &'a str) -> Result<TransportSetup, HeaderParseError> {
        match setup {
            "active" => Ok(TransportSetup::Active),
            "passive" => Ok(TransportSetup::Passive),
            "actpass" => Ok(TransportSetup::Actpass),
            _ => Err(HeaderParseError::new()),
        }
    }
}

/// Transport connection reuse preference. RTSP 2.0 only.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum TransportConnection {
    /// Create a new connection.
    New,
    /// Reuse an existing connection.
    Existing,
}

impl TransportConnection {
    /// Return connection as `&str`.
    pub fn as_str(&self) -> &str {
        match self {
            TransportConnection::New => "new",
            TransportConnection::Existing => "existing",
        }
    }
}

impl fmt::Display for TransportConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'a> TryFrom<&'a str> for TransportConnection {
    type Error = HeaderParseError;

    fn try_from(connection: &'a str) -> Result<TransportConnection, HeaderParseError> {
        match connection {
            "new" => Ok(TransportConnection::New),
            "existing" => Ok(TransportConnection::Existing),
            _ => Err(HeaderParseError::new()),
        }
    }
}

/// RTP transport description.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RtpTransportParameters {
    /// Unicast transport.
    pub unicast: bool,
    /// Multicast transport.
    pub multicast: bool,
    /// TCP/interleaved transport channels.
    pub interleaved: Option<(u8, Option<u8>)>,
    /// Multicast packet time-to-live.
    pub ttl: Option<u8>,
    /// Multicast layers. RTSP 2.0 only.
    pub layers: Option<u32>,
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
    /// Connection setup direction. RTSP 2.0 only.
    pub setup: Option<TransportSetup>,
    /// Connection reuse preference. RTSP 2.0 only.
    pub connection: Option<TransportConnection>,
    /// MIKEY keying data, base64-encoded. RTSP 2.0 only.
    pub mikey: Option<String>,
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
                    let channels = value.ok_or_else(HeaderParseError::new)?;
                    let mut channels = channels.splitn(2, '-');

                    let channel_start = channels
                        .next()
                        .and_then(|s| s.parse::<u8>().ok())
                        .ok_or_else(HeaderParseError::new)?;

                    let channel_end = channels
                        .next()
                        .map(|s| s.parse::<u8>().map_err(|_| HeaderParseError::new()))
                        .transpose()?;

                    rtp_params.interleaved = Some((channel_start, channel_end));
                }
                "ttl" => {
                    let ttl = value
                        .and_then(|s| s.parse::<u8>().ok())
                        .ok_or_else(HeaderParseError::new)?;

                    rtp_params.ttl = Some(ttl);
                }
                "layers" => {
                    let layers = value
                        .as_deref()
                        .and_then(|s| parse_digits::<u32>(s).ok())
                        .ok_or_else(HeaderParseError::new)?;

                    rtp_params.layers = Some(layers);
                }
                "ssrc" => {
                    let ssrc = value
                        .ok_or_else(HeaderParseError::new)?
                        .split('/')
                        .map(|s| {
                            // ssrc = 8HEX
                            if s.len() != 8 {
                                return Err(HeaderParseError::new());
                            }

                            u32::from_str_radix(s, 16).map_err(|_| HeaderParseError::new())
                        })
                        .collect::<Result<Vec<_>, _>>()?;

                    if ssrc.is_empty() {
                        return Err(HeaderParseError::new());
                    }

                    rtp_params.ssrc = ssrc;
                }
                "mode" => {
                    let modes = value.ok_or_else(HeaderParseError::new)?;
                    let modes = match (modes.starts_with('"'), modes.ends_with('"')) {
                        (true, true) if modes.len() > 1 => &modes[1..(modes.len() - 1)],
                        (false, false) => &modes,
                        _ => return Err(HeaderParseError::new()),
                    };
                    let modes = modes
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(TransportMode::from)
                        .collect::<Vec<_>>();

                    if modes.is_empty() {
                        return Err(HeaderParseError::new());
                    }

                    rtp_params.mode = modes;
                }
                "dest_addr" | "src_addr" => {
                    let addrs = value
                        .ok_or_else(HeaderParseError::new)?
                        .split('/')
                        .map(|s| {
                            if !s.starts_with('"') || !s.ends_with('"') {
                                return Err(HeaderParseError::new());
                            }

                            // Unescape quoted string
                            let mut res = Vec::with_capacity(s.len());
                            let mut s = s.as_bytes();
                            s = &s[1..(s.len() - 1)];
                            while !s.is_empty() {
                                if s.starts_with(b"\\") {
                                    if s.len() < 2 {
                                        return Err(HeaderParseError::new());
                                    }
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
                        return Err(HeaderParseError::new());
                    }

                    if name == "src_addr" {
                        rtp_params.src_addr = addrs;
                    } else {
                        rtp_params.dest_addr = addrs;
                    }
                }
                "port" | "server_port" | "client_port" => {
                    let ports = value.ok_or_else(HeaderParseError::new)?;
                    let mut ports = ports.splitn(2, '-');

                    let port_start = ports
                        .next()
                        .and_then(|s| s.parse::<u16>().ok())
                        .ok_or_else(HeaderParseError::new)?;

                    let port_end = ports
                        .next()
                        .map(|s| s.parse::<u16>().map_err(|_| HeaderParseError::new()))
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
                    rtp_params.destination = Some(value.ok_or_else(HeaderParseError::new)?);
                }
                "source" => {
                    rtp_params.source = Some(value.ok_or_else(HeaderParseError::new)?);
                }
                "append" => {
                    rtp_params.append = true;
                }
                "RTCP-mux" => {
                    rtp_params.rtcp_mux = true;
                }
                "setup" => {
                    let setup = value.ok_or_else(HeaderParseError::new)?;
                    rtp_params.setup = Some(TransportSetup::try_from(setup.as_str())?);
                }
                "connection" => {
                    let connection = value.ok_or_else(HeaderParseError::new)?;
                    rtp_params.connection =
                        Some(TransportConnection::try_from(connection.as_str())?);
                }
                "MIKEY" => {
                    let mikey = value.ok_or_else(HeaderParseError::new)?;

                    if !is_valid_base64(&mikey) {
                        return Err(HeaderParseError::new());
                    }

                    rtp_params.mikey = Some(mikey);
                }
                _ => {
                    rtp_params.others.insert(name, value);
                }
            }
        }

        Ok(rtp_params)
    }
}

/// Check for the `base64` rule of RFC 7826 section 20.39: `*base64-unit
/// [base64-pad]` with `base64-unit = 4base64-char` and `base64-pad =
/// (2base64-char "==") / (3base64-char "=")`.
fn is_valid_base64(s: &str) -> bool {
    let (data, padding) = match s.find('=') {
        Some(i) => s.split_at(i),
        None => (s, ""),
    };

    if !data
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/')
    {
        return false;
    }

    match padding {
        "" => data.len() % 4 == 0,
        "==" => data.len() % 4 == 2,
        "=" => data.len() % 4 == 3,
        _ => false,
    }
}

/// Lower RTP transport protocol.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
            "PLAY" | "play" => TransportMode::Play,
            "RECORD" | "record" => TransportMode::Record,
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TransportParameters(pub BTreeMap<String, Option<String>>);

mod parser {
    use super::*;

    use super::parser_helpers::{cond_parser, rtsp_unreserved, token, trim};
    use nom::branch::alt;
    use nom::bytes::complete::{tag, take_while};
    use nom::combinator::{all_consuming, map_res};
    use nom::multi::{many0, separated_list1};
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
        let mut closed = false;

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
                closed = true;
                break;
            }
        }

        let (fst, snd) = input.split_at(input.len() - o.len());
        // The input did not contain the closing quote, e.g. it ended in an escaped
        // quote.
        if !closed {
            return Err(Err::Incomplete(Needed::Size(NonZeroUsize::new(1).unwrap())));
        }

        // Must have the starting quote
        assert!(fst.starts_with(b"\""));

        Ok((snd, fst))
    }

    // MIKEY carries base64 data, which may contain '/' characters and
    // trailing '=' padding that are not rtsp-unreserved characters.
    fn mikey_value(input: &[u8]) -> IResult<&[u8], &[u8]> {
        take_while(|b: u8| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')(input)
    }

    fn parameter(input: &[u8]) -> IResult<&[u8], (&str, Option<&str>)> {
        if input.is_empty() {
            return Err(Err::Error(nom::error::Error::new(
                input,
                nom::error::ErrorKind::Eof,
            )));
        }

        let (input, name) = trim(map_res(token, str::from_utf8))(input)?;

        type ValueParser = fn(&[u8]) -> IResult<&[u8], &[u8]>;

        let value_parser: ValueParser = if name == "MIKEY" {
            mikey_value
        } else {
            rtsp_unreserved
        };

        let (input, value) = cond_parser(
            tag(b"="),
            trim(map_res(
                alt((quoted_string_or_address_list, value_parser)),
                str::from_utf8,
            )),
        )(input)?;

        Ok((input, (name, value)))
    }

    fn parameters(input: &[u8]) -> IResult<&[u8], TransportParameters> {
        let (input, params) = many0(preceded(trim(tag(b";")), parameter))(input)?;

        // A parameter shall appear at most once within one transport-spec;
        // duplicates with identical values are tolerated, conflicting ones
        // are rejected.
        let mut map = BTreeMap::new();
        for (name, value) in params {
            let value = value.map(String::from);
            match map.get(name) {
                Some(previous) if *previous != value => {
                    return Err(Err::Failure(nom::error::Error::new(
                        input,
                        nom::error::ErrorKind::MapRes,
                    )));
                }
                Some(_) => (),
                None => {
                    map.insert(name.into(), value);
                }
            }
        }

        Ok((input, TransportParameters(map)))
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
                    spec: other.join("/"),
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
    fn header_name() -> Option<HeaderName> {
        Some(TRANSPORT.clone())
    }
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&TRANSPORT) {
            None => return Ok(None),
            Some(header) => header,
        };

        let (_rem, transport) =
            parser::transports(header.as_str().as_bytes()).map_err(|_| HeaderParseError::new())?;

        Ok(Some(transport.into()))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        use std::fmt::Write;

        let headers = headers.as_mut();

        if self.0.is_empty() {
            // An empty list cannot be represented. Remove any existing
            // header instead of writing an empty (and unparseable) header
            // value.
            headers.remove(&TRANSPORT);
            return;
        }

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
                        write!(&mut transports, "interleaved={channel_start}").unwrap();
                        if let Some(channel_end) = channel_end {
                            write!(&mut transports, "-{channel_end}").unwrap();
                        }
                    }

                    if let Some(ttl) = rtp.params.ttl {
                        transports.push(';');
                        write!(&mut transports, "ttl={ttl}").unwrap();
                    }

                    if let Some(layers) = rtp.params.layers {
                        transports.push(';');
                        write!(&mut transports, "layers={layers}").unwrap();
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

                            write!(&mut transports, "{ssrc:08X}").unwrap();
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

                            // Escape quote and backslash characters.
                            let addr = addr.replace('\\', "\\\\").replace('"', "\\\"");
                            write!(&mut transports, "\"{addr}\"").unwrap()
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

                            // Escape quote and backslash characters.
                            let addr = addr.replace('\\', "\\\\").replace('"', "\\\"");
                            write!(&mut transports, "\"{addr}\"").unwrap()
                        }
                    }

                    if rtp.params.append {
                        transports.push(';');
                        transports.push_str("append");
                    }

                    if let Some((port_start, port_end)) = rtp.params.port {
                        transports.push(';');
                        write!(&mut transports, "port={port_start}").unwrap();
                        if let Some(port_end) = port_end {
                            write!(&mut transports, "-{port_end}").unwrap();
                        }
                    }

                    if let Some((port_start, port_end)) = rtp.params.client_port {
                        transports.push(';');
                        write!(&mut transports, "client_port={port_start}").unwrap();
                        if let Some(port_end) = port_end {
                            write!(&mut transports, "-{port_end}").unwrap();
                        }
                    }

                    if let Some((port_start, port_end)) = rtp.params.server_port {
                        transports.push(';');
                        write!(&mut transports, "server_port={port_start}").unwrap();
                        if let Some(port_end) = port_end {
                            write!(&mut transports, "-{port_end}").unwrap();
                        }
                    }

                    if let Some(ref destination) = rtp.params.destination {
                        transports.push(';');
                        write!(&mut transports, "destination={destination}").unwrap();
                    }

                    if let Some(ref source) = rtp.params.source {
                        transports.push(';');
                        write!(&mut transports, "source={source}").unwrap();
                    }

                    if !rtp.params.mode.is_empty() {
                        transports.push(';');

                        transports.push_str("mode=\"");
                        let mut first = true;
                        for mode in &rtp.params.mode {
                            if first {
                                first = false;
                            } else {
                                transports.push(',');
                            }

                            transports.push_str(mode.as_str());
                        }

                        transports.push('"');
                    }

                    if rtp.params.rtcp_mux {
                        transports.push(';');
                        transports.push_str("RTCP-mux");
                    }

                    if let Some(ref setup) = rtp.params.setup {
                        transports.push(';');
                        write!(&mut transports, "setup={setup}").unwrap();
                    }

                    if let Some(ref connection) = rtp.params.connection {
                        transports.push(';');
                        write!(&mut transports, "connection={connection}").unwrap();
                    }

                    if let Some(ref mikey) = rtp.params.mikey {
                        transports.push(';');
                        write!(&mut transports, "MIKEY={mikey}").unwrap();
                    }

                    for (name, value) in &rtp.params.others {
                        transports.push(';');

                        if let Some(value) = value {
                            write!(&mut transports, "{name}={value}").unwrap();
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
                            write!(&mut transports, "{name}={value}").unwrap();
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

    #[test]
    fn test_transport_dest_addr_escaped_quote_end() {
        // A quoted string ending in an escaped quote is not terminated.
        let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .header(crate::headers::TRANSPORT, "RTP/AVP;dest_addr=\"a\\\"")
            .empty();

        assert!(request.typed_header::<super::Transports>().is_err());
    }

    #[test]
    fn test_transport_dest_addr_escaped_quote() {
        let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .header(crate::headers::TRANSPORT, "RTP/AVP;dest_addr=\"a\\\"b\"")
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
                    dest_addr: vec!["a\"b".into()],
                    ..Default::default()
                },
            })]
            .into()
        );
    }

    #[test]
    fn test_transport_addr_escaping_roundtrip() {
        let header = "RTP/AVP;dest_addr=\"a\\\"b\"/\"c\\\\d\";src_addr=\"e\\\"f\"";
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
                    dest_addr: vec!["a\"b".into(), "c\\d".into()],
                    src_addr: vec!["e\"f".into()],
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
    fn test_transport_multiple_modes_roundtrip() {
        let header = "RTP/AVP;mode=\"PLAY,RECORD\"";
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
                    mode: vec![TransportMode::Play, TransportMode::Record],
                    ..Default::default()
                },
            })]
            .into()
        );

        let request2 = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .typed_header(&transports)
            .empty();

        assert_eq!(request2.header(&crate::headers::TRANSPORT).unwrap(), header);
    }

    #[test]
    fn test_transport_other_spec() {
        let header = "MP2T/TS/UDP;unicast;ttl=32";
        let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .header(crate::headers::TRANSPORT, header)
            .empty();

        let transports = request
            .typed_header::<super::Transports>()
            .unwrap()
            .unwrap();

        assert_eq!(
            transports,
            vec![Transport::Other(super::OtherTransport {
                spec: "MP2T/TS/UDP".into(),
                params: TransportParameters(std::collections::BTreeMap::from([
                    ("unicast".to_string(), None),
                    ("ttl".to_string(), Some("32".to_string())),
                ])),
            })]
            .into()
        );

        let request2 = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .typed_header(&transports)
            .empty();

        assert_eq!(
            request2.typed_header::<super::Transports>().unwrap(),
            Some(transports)
        );
    }

    #[test]
    fn test_transport_mode_no_quotes() {
        let header = "RTP/AVP;multicast;mode=PLAY,RTP/AVP;unicast;dest_addr=\"192.0.2.5:3456\"/\"192.0.2.5:3457\";mode=\"play\"";
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
    }
    #[test]
    fn test_insert_into_empty_list() {
        let transports = Transports(Vec::new());

        let mut test_headers = Headers::new();
        test_headers.insert(TRANSPORT, "RTP/AVP;unicast");
        transports.insert_into(&mut test_headers);

        assert_eq!(test_headers.get(&TRANSPORT), None);
    }

    #[test]
    fn test_transport_ssrc() {
        let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .header(crate::headers::TRANSPORT, "RTP/AVP;ssrc=0D12F123")
            .empty();

        let transports = request
            .typed_header::<super::Transports>()
            .unwrap()
            .unwrap();

        let [Transport::Rtp(rtp)] = &transports.0[..] else {
            panic!("unexpected transports: {:?}", transports);
        };

        assert_eq!(rtp.params.ssrc, vec![0x0D12F123]);
    }

    #[test]
    fn test_rtp_transport_parameters_mode_lone_quote() {
        // A lone quote is not a valid quoted string, it must not panic.
        let mut params = std::collections::BTreeMap::new();
        params.insert("mode".to_string(), Some("\"".to_string()));

        assert!(
            super::RtpTransportParameters::try_from(super::TransportParameters(params)).is_err()
        );
    }

    #[test]
    fn test_transport_invalid_ssrc() {
        // ssrc = 8HEX.
        let bad_headers = [
            "RTP/AVP;ssrc=0D12F12",
            "RTP/AVP;ssrc=0D12F1234",
            "RTP/AVP;ssrc=0D12F12G",
            "RTP/AVP;ssrc=+0D12F123",
        ];

        for header in bad_headers {
            let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
                .header(crate::headers::TRANSPORT, header)
                .empty();

            assert!(
                request
                    .typed_header::<super::Transports>()
                    .map_or(true, |transports| transports.is_none()),
                "{} should fail",
                header
            );
        }
    }

    #[test]
    fn test_transport_duplicate_parameters() {
        // Duplicate parameters with identical values within one transport
        // spec are accepted, conflicting ones are rejected. Duplicates
        // across separate transport specs are always fine.
        let ok_headers = [
            "RTP/AVP;unicast;client_port=5000-5001;client_port=5000-5001",
            "RTP/AVP;unicast;unicast",
            "RTP/AVP;unicast;client_port=5000-5001,RTP/AVP;unicast;client_port=6000-6001",
        ];
        let bad_headers = [
            "RTP/AVP;unicast;client_port=5000-5001;client_port=6000-6001",
            "RTP/AVP;unicast;ssrc=0D12F123;ssrc=1D12F123",
            "RTP/AVP;unicast;unknown=1;unknown=2",
        ];

        for header in ok_headers {
            let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
                .header(crate::headers::TRANSPORT, header)
                .empty();

            assert!(
                request
                    .typed_header::<super::Transports>()
                    .unwrap_or_else(|_| panic!("couldn't parse {}", header))
                    .is_some(),
                "{} should parse",
                header
            );
        }

        for header in bad_headers {
            let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
                .header(crate::headers::TRANSPORT, header)
                .empty();

            assert!(
                request.typed_header::<super::Transports>().is_err(),
                "{} should fail",
                header
            );
        }
    }

    #[test]
    fn test_transport_setup_connection_layers() {
        let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .header(
                crate::headers::TRANSPORT,
                "RTP/AVP/TCP;setup=actpass;connection=new",
            )
            .empty();

        let transports = request
            .typed_header::<super::Transports>()
            .unwrap()
            .unwrap();

        let [Transport::Rtp(rtp)] = &transports.0[..] else {
            panic!("unexpected transports: {:?}", transports);
        };

        assert_eq!(rtp.params.setup, Some(super::TransportSetup::Actpass));
        assert_eq!(rtp.params.connection, Some(super::TransportConnection::New));

        let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .header(crate::headers::TRANSPORT, "RTP/AVP;multicast;layers=3")
            .empty();

        let transports = request
            .typed_header::<super::Transports>()
            .unwrap()
            .unwrap();

        let [Transport::Rtp(rtp)] = &transports.0[..] else {
            panic!("unexpected transports: {:?}", transports);
        };

        assert_eq!(rtp.params.layers, Some(3));
    }

    #[test]
    fn test_transport_mikey() {
        // The RFC 7826 Appendix example MIKEY values contain base64
        // characters outside the rtsp-unreserved set.
        let ok_values = [
            "TUlLRVkgZm9yIHN0cmVhbSB0d2lzdGVyLjNncC90cmFja0lEPTQ=",
            "ab==",
            "abc=",
            "",
        ];

        for value in ok_values {
            let header = format!("RTP/SAVP;MIKEY={}", value);
            let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
                .header(crate::headers::TRANSPORT, header.as_str())
                .empty();

            let transports = request
                .typed_header::<super::Transports>()
                .unwrap_or_else(|_| panic!("couldn't parse {}", header))
                .unwrap();

            let [Transport::Rtp(rtp)] = &transports.0[..] else {
                panic!("unexpected transports: {:?}", transports);
            };

            assert_eq!(rtp.params.mikey.as_deref(), Some(value));
        }
    }

    #[test]
    fn test_transport_invalid_setup_connection_layers_mikey() {
        let bad_headers = [
            // "holdconn" is not defined for RTSP (RFC 7826 section 18.54).
            "RTP/AVP/TCP;setup=holdconn",
            "RTP/AVP/TCP;setup=ACTIVE",
            "RTP/AVP/TCP;setup=",
            "RTP/AVP/TCP;setup",
            "RTP/AVP/TCP;connection=x",
            "RTP/AVP/TCP;connection",
            // layers = 1*DIGIT.
            "RTP/AVP;layers=abc",
            "RTP/AVP;layers=+1",
            "RTP/AVP;layers=-1",
            "RTP/AVP;layers=",
            "RTP/AVP;layers=4294967296",
            // MIKEY-Value = base64, padding only at the end.
            "RTP/AVP;MIKEY=abcd=",
            "RTP/AVP;MIKEY=a=",
            "RTP/AVP;MIKEY=ab=cd",
            "RTP/AVP;MIKEY=ab*cd",
        ];

        for header in bad_headers {
            let request = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
                .header(crate::headers::TRANSPORT, header)
                .empty();

            assert!(
                request.typed_header::<super::Transports>().is_err(),
                "{} should fail",
                header
            );
        }
    }

    #[test]
    fn test_transport_new_params_roundtrip() {
        let header = "RTP/AVP;layers=3;setup=passive;connection=existing;MIKEY=abcd//==";
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
                    layers: Some(3),
                    setup: Some(super::TransportSetup::Passive),
                    connection: Some(super::TransportConnection::Existing),
                    mikey: Some("abcd//==".into()),
                    ..Default::default()
                },
            })]
            .into()
        );

        let request2 = crate::Request::builder(crate::Method::Setup, crate::Version::V2_0)
            .typed_header(&transports)
            .empty();

        assert_eq!(request2.header(&crate::headers::TRANSPORT).unwrap(), header);
    }
}
