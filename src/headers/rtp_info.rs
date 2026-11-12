// Copyright (C) 2021 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use std::collections::BTreeMap;

/// `RTP-Info` header ([RFC 7826 section 18.45](https://tools.ietf.org/html/rfc7826#section-18.45)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum RtpInfos {
    V1(Vec<v1::RtpInfo>),
    V2(Vec<v2::RtpInfo>),
}

impl RtpInfos {
    /// Try converting into a RTSP 1.0 RTP-Info header.
    ///
    /// Note that this potentially loses extra information that can't be represented.
    pub fn try_into_v1(self) -> Result<Self, Self> {
        match self {
            RtpInfos::V1(v1) => Ok(RtpInfos::V1(v1)),
            RtpInfos::V2(v2) => {
                if v2.iter().any(|info| info.ssrc_infos.len() != 1) {
                    return Err(RtpInfos::V2(v2));
                }

                let infos = v2
                    .into_iter()
                    .map(|info| v1::RtpInfo {
                        uri: info.uri,
                        seq: info.ssrc_infos[0].seq,
                        rtptime: info.ssrc_infos[0].rtptime,
                    })
                    .collect();

                Ok(RtpInfos::V1(infos))
            }
        }
    }
}

pub mod v1 {
    use super::*;

    /// RTP-Info.
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct RtpInfo {
        /// Stream URI.
        pub uri: url::Url,
        /// Sequence number of the first packet that is a direct result of the request.
        pub seq: Option<u16>,
        /// RTP timestamp corresponding to the start time in the `Range` header.
        pub rtptime: Option<u32>,
    }

    pub(super) mod parser {
        use super::*;

        use super::parser_helpers::{backtrack, trim};
        use std::str;
        use winnow::combinator::{eof, separated, separated_pair};
        use winnow::token::{literal, take_while};
        use winnow::ModalResult;
        use winnow::Parser;

        fn param<'i>(input: &mut &'i [u8]) -> ModalResult<(&'i str, &'i str)> {
            separated_pair(
                trim(take_while(0.., |b: u8| b != b'=')).try_map(str::from_utf8),
                literal(b"="),
                trim(take_while(0.., |b: u8| b != b';')).try_map(str::from_utf8),
            )
            .parse_next(input)
        }

        fn rtp_info(input: &mut &[u8]) -> ModalResult<RtpInfo> {
            #[derive(Default)]
            #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
            struct Info<'a> {
                uri: Option<&'a str>,
                seq: Option<&'a str>,
                rtptime: Option<&'a str>,
            }

            let params: Vec<(&str, &str)> =
                separated(1.., param, literal(b";")).parse_next(input)?;

            let mut acc = Info::default();
            for (name, value) in params {
                match name {
                    "url" => acc.uri = Some(value),
                    "seq" => acc.seq = Some(value),
                    "rtptime" => acc.rtptime = Some(value),
                    _ => (),
                }
            }

            let info = (|| -> Result<_, HeaderParseError> {
                // Accept quoted URLs as well (seen in the wild).
                let uri = acc
                    .uri
                    .and_then(|uri| {
                        let uri = uri
                            .strip_prefix('"')
                            .and_then(|uri| uri.strip_suffix('"'))
                            .unwrap_or(uri);
                        url::Url::parse(uri).ok()
                    })
                    .ok_or_else(HeaderParseError::new)?;
                let seq = acc
                    .seq
                    .map(|s| s.parse::<u16>())
                    .transpose()
                    .map_err(|_| HeaderParseError::new())?;

                let rtptime = acc
                    .rtptime
                    .map(|s| s.parse::<u32>())
                    .transpose()
                    .map_err(|_| HeaderParseError::new())?;

                Ok(RtpInfo { uri, seq, rtptime })
            })()
            .map_err(|_| backtrack(input))?;

            Ok(info)
        }

        fn element(input: &mut &[u8]) -> ModalResult<RtpInfo> {
            // Parse each list element only up to the next ',' so that the
            // value parsers cannot consume past the element boundary.
            let segment = take_while(0.., |b: u8| b != b',').parse_next(input)?;

            rtp_info(&mut &segment[..])
        }

        pub(crate) fn rtp_infos(input: &mut &[u8]) -> ModalResult<Vec<RtpInfo>> {
            let infos = separated(1.., element, literal(b",")).parse_next(input)?;
            eof(input)?;

            Ok(infos)
        }
    }
}

pub mod v2 {
    use super::*;

    /// RTP-Info.
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct RtpInfo {
        /// Stream URI.
        pub uri: url::Url,
        /// SSRC information.
        ///
        /// Entries without any SSRC information cannot be represented in the
        /// serialized form and are dropped when serializing.
        pub ssrc_infos: Vec<SsrcInfo>,
    }

    /// SSRC Information.
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    pub struct SsrcInfo {
        /// SSRC of this stream.
        pub ssrc: u32,
        /// Sequence number of the first packet that is a direct result of the request.
        pub seq: Option<u16>,
        /// RTP timestamp corresponding to the start time in the `Range` header.
        pub rtptime: Option<u32>,
        /// Other parameters.
        pub others: BTreeMap<String, Option<String>>,
    }

    pub(super) mod parser {
        use super::*;

        use super::parser_helpers::{
            backtrack, cond_parser, parse_quoted_string, quoted_string, token, trim,
        };
        use std::str;
        use winnow::combinator::{alt, eof, repeat, separated};
        use winnow::token::{literal, take};
        use winnow::ModalResult;
        use winnow::Parser;

        fn param_value(input: &mut &[u8]) -> ModalResult<String> {
            let value = alt((quoted_string, token)).parse_next(input)?;
            let value = str::from_utf8(value).map_err(|_| backtrack(input))?;

            if value.starts_with('"') {
                parse_quoted_string(value).map_err(|_| backtrack(input))
            } else {
                Ok(String::from(value))
            }
        }

        fn param<'i>(input: &mut &'i [u8]) -> ModalResult<(&'i str, Option<String>)> {
            if input.is_empty() {
                return Err(backtrack(input));
            }

            (
                trim(token).try_map(str::from_utf8),
                cond_parser(literal(b"="), trim(param_value)),
            )
                .parse_next(input)
        }

        fn param_list<'i>(input: &mut &'i [u8]) -> ModalResult<Vec<(&'i str, Option<String>)>> {
            separated(1.., param, literal(b";")).parse_next(input)
        }

        fn ssrc_info(input: &mut &[u8]) -> ModalResult<SsrcInfo> {
            let ssrc_info = (
                trim(literal(b"ssrc")),
                trim(literal(b"=")),
                take(8usize)
                    .try_map(str::from_utf8)
                    .try_map(|s| u32::from_str_radix(s, 16)),
                cond_parser(trim(literal(b":")), param_list),
            )
                .try_map(|(_, _, ssrc, params)| -> Result<_, HeaderParseError> {
                    let mut others: BTreeMap<String, Option<String>> = BTreeMap::new();
                    for (name, value) in params.unwrap_or_default() {
                        others.insert(String::from(name), value);
                    }

                    let seq = if let Some((_, Some(seq))) = others.remove_entry("seq") {
                        Some(seq.parse::<u16>().map_err(|_| HeaderParseError::new())?)
                    } else {
                        None
                    };

                    let rtptime = if let Some((_, Some(rtptime))) = others.remove_entry("rtptime") {
                        Some(
                            rtptime
                                .parse::<u32>()
                                .map_err(|_| HeaderParseError::new())?,
                        )
                    } else {
                        None
                    };

                    Ok(SsrcInfo {
                        ssrc,
                        seq,
                        rtptime,
                        others,
                    })
                })
                .parse_next(input)?;

            Ok(ssrc_info)
        }

        fn parse_url(input: &mut &[u8]) -> ModalResult<url::Url> {
            let quoted = trim(quoted_string).parse_next(input)?;
            let quoted = str::from_utf8(quoted).map_err(|_| backtrack(input))?;
            let quoted = parse_quoted_string(quoted).map_err(|_| backtrack(input))?;
            let uri = url::Url::parse(&quoted).map_err(|_| backtrack(input))?;

            Ok(uri)
        }

        fn rtp_info(input: &mut &[u8]) -> ModalResult<RtpInfo> {
            let (_, _, uri, ssrc_infos) = (
                trim(literal(b"url")),
                trim(literal(b"=")),
                parse_url,
                repeat(1.., trim(ssrc_info)),
            )
                .parse_next(input)?;

            Ok(RtpInfo { uri, ssrc_infos })
        }

        pub(crate) fn rtp_infos(input: &mut &[u8]) -> ModalResult<Vec<RtpInfo>> {
            let infos = separated(1.., rtp_info, literal(b",")).parse_next(input)?;
            eof(input)?;

            Ok(infos)
        }
    }
}

mod parser {
    use super::*;

    use winnow::ModalResult;

    pub(super) fn rtp_infos(input: &mut &[u8]) -> ModalResult<RtpInfos> {
        fn is_v2_rtpinfo(mut i: &[u8]) -> bool {
            while i.starts_with(b" ") || i.starts_with(b"\t") {
                i = &i[1..];
            }

            if !i.starts_with(b"url") {
                return false;
            }
            i = &i[3..];

            while i.starts_with(b" ") || i.starts_with(b"\t") {
                i = &i[1..];
            }

            if !i.starts_with(b"=") {
                return false;
            }
            i = &i[1..];

            while i.starts_with(b" ") || i.starts_with(b"\t") {
                i = &i[1..];
            }

            i.starts_with(b"\"")
        }

        let start = *input;

        if is_v2_rtpinfo(start) {
            match v2::parser::rtp_infos(input) {
                Ok(infos) => Ok(RtpInfos::V2(infos)),
                // Fall back to the other parser in case the heuristic
                // misdetected the version.
                Err(err) => {
                    let mut rem = start;
                    match v1::parser::rtp_infos(&mut rem) {
                        Ok(infos) => {
                            *input = rem;

                            Ok(RtpInfos::V1(infos))
                        }
                        Err(_) => Err(err),
                    }
                }
            }
        } else {
            match v1::parser::rtp_infos(input) {
                Ok(infos) => Ok(RtpInfos::V1(infos)),
                Err(err) => {
                    let mut rem = start;
                    match v2::parser::rtp_infos(&mut rem) {
                        Ok(infos) => {
                            *input = rem;

                            Ok(RtpInfos::V2(infos))
                        }
                        Err(_) => Err(err),
                    }
                }
            }
        }
    }
}

impl super::TypedHeader for RtpInfos {
    fn header_name() -> Option<HeaderName> {
        Some(RTP_INFO.clone())
    }
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&RTP_INFO) {
            None => return Ok(None),
            Some(header) => header,
        };

        let rtp_info = parser::rtp_infos(&mut header.as_str().as_bytes())
            .map_err(|_| HeaderParseError::new())?;

        Ok(Some(rtp_info))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        use std::fmt::Write;

        let headers = headers.as_mut();

        let mut infos = String::new();

        match self {
            RtpInfos::V1(v1) => {
                for info in v1 {
                    if !infos.is_empty() {
                        infos.push(',');
                    }

                    write!(&mut infos, "url={}", info.uri).unwrap();

                    if let Some(seq) = info.seq {
                        write!(&mut infos, ";seq={seq}").unwrap();
                    }

                    if let Some(rtptime) = info.rtptime {
                        write!(&mut infos, ";rtptime={rtptime}").unwrap();
                    }
                }
            }
            RtpInfos::V2(v2) => {
                for info in v2 {
                    if info.ssrc_infos.is_empty() {
                        continue;
                    }

                    if !infos.is_empty() {
                        infos.push(',');
                    }

                    write!(&mut infos, "url=\"{}\"", info.uri).unwrap();
                    for ssrc in &info.ssrc_infos {
                        write!(&mut infos, " ssrc={:08X}", ssrc.ssrc).unwrap();
                        if ssrc.seq.is_none() && ssrc.rtptime.is_none() && ssrc.others.is_empty() {
                            continue;
                        }
                        infos.push(':');

                        let mut need_semi = false;

                        if let Some(seq) = ssrc.seq {
                            write!(&mut infos, "seq={seq}").unwrap();
                            need_semi = true;
                        }

                        if let Some(rtptime) = ssrc.rtptime {
                            if need_semi {
                                infos.push(';');
                            }
                            write!(&mut infos, "rtptime={rtptime}").unwrap();
                            need_semi = true;
                        }

                        for (name, value) in &ssrc.others {
                            if need_semi {
                                infos.push(';');
                            }
                            if let Some(value) = value {
                                infos.push_str(name);
                                infos.push('=');
                                if value.is_empty()
                                    || value.bytes().any(|b| !parser_helpers::is_token_char(b))
                                {
                                    parser_helpers::write_quoted_string(&mut infos, value);
                                } else {
                                    infos.push_str(value);
                                }
                            } else {
                                write!(&mut infos, "{name}").unwrap();
                            }
                            need_semi = true;
                        }
                    }
                }
            }
        }

        if infos.is_empty() {
            // An empty list cannot be represented. Remove any existing
            // header instead of writing an empty header value.
            headers.remove(&RTP_INFO);
            return;
        }

        headers.insert(RTP_INFO, infos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_info() {
        let header =
            "url=\"rtsp://example.com/foo/audio\" ssrc=0A13C760:seq=45102;rtptime=12345678";
        let response = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header)
            .empty();

        let infos = response.typed_header::<super::RtpInfos>().unwrap().unwrap();

        assert_eq!(
            infos,
            RtpInfos::V2(vec![v2::RtpInfo {
                uri: url::Url::parse("rtsp://example.com/foo/audio").unwrap(),
                ssrc_infos: vec![v2::SsrcInfo {
                    ssrc: 0x0A13C760,
                    seq: Some(45102),
                    rtptime: Some(12345678),
                    others: BTreeMap::new()
                }],
            }])
        );

        let response2 = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .typed_header(&infos)
            .empty();

        assert_eq!(response, response2);
    }

    #[test]
    fn test_info_multiple_ssrc() {
        let header =
            "url=\"rtsp://example.com/foo/audio\" ssrc=0A13C760:seq=45102;rtptime=12345678 ssrc=9A9DE123:seq=30211;rtptime=29567112";
        let response = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header)
            .empty();

        let infos = response.typed_header::<super::RtpInfos>().unwrap().unwrap();

        assert_eq!(
            infos,
            RtpInfos::V2(vec![v2::RtpInfo {
                uri: url::Url::parse("rtsp://example.com/foo/audio").unwrap(),
                ssrc_infos: vec![
                    v2::SsrcInfo {
                        ssrc: 0x0A13C760,
                        seq: Some(45102),
                        rtptime: Some(12345678),
                        others: BTreeMap::new()
                    },
                    v2::SsrcInfo {
                        ssrc: 0x9A9DE123,
                        seq: Some(30211),
                        rtptime: Some(29567112),
                        others: BTreeMap::new()
                    }
                ],
            }])
        );

        let response2 = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .typed_header(&infos)
            .empty();

        assert_eq!(response, response2);
    }

    #[test]
    fn test_multiple_infos() {
        let header = "url=\"rtsp://example.com/foo/audio\" ssrc=0A13C760:seq=45102;rtptime=12345678,url=\"rtsp://example.com/foo/video\" ssrc=9A9DE123:seq=30211;rtptime=29567112";
        let response = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header)
            .empty();

        let infos = response.typed_header::<super::RtpInfos>().unwrap().unwrap();

        assert_eq!(
            infos,
            RtpInfos::V2(vec![
                v2::RtpInfo {
                    uri: url::Url::parse("rtsp://example.com/foo/audio").unwrap(),
                    ssrc_infos: vec![v2::SsrcInfo {
                        ssrc: 0x0A13C760,
                        seq: Some(45102),
                        rtptime: Some(12345678),
                        others: BTreeMap::new()
                    }],
                },
                v2::RtpInfo {
                    uri: url::Url::parse("rtsp://example.com/foo/video").unwrap(),
                    ssrc_infos: vec![v2::SsrcInfo {
                        ssrc: 0x9A9DE123,
                        seq: Some(30211),
                        rtptime: Some(29567112),
                        others: BTreeMap::new()
                    }],
                }
            ])
        );

        let response2 = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .typed_header(&infos)
            .empty();

        assert_eq!(response, response2);
    }

    #[test]
    fn test_info_v2_quoted_string_escaped_quote_end() {
        // A quoted string ending in an escaped quote is not terminated.
        let header = "url=\"rtsp://example.com/foo/audio\" ssrc=0A13C760:seq=45102;other=\"a\\\"";
        let response = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header)
            .empty();

        assert!(response.typed_header::<super::RtpInfos>().is_err());
    }

    #[test]
    fn test_info_v1() {
        let header = "url=rtsp://example.com/foo/audio;seq=45102;rtptime=12345678";
        let response = crate::Response::builder(crate::Version::V1_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header)
            .empty();

        let infos = response.typed_header::<super::RtpInfos>().unwrap().unwrap();

        assert_eq!(
            infos,
            RtpInfos::V1(vec![v1::RtpInfo {
                uri: url::Url::parse("rtsp://example.com/foo/audio").unwrap(),
                seq: Some(45102),
                rtptime: Some(12345678),
            }])
        );

        let response2 = crate::Response::builder(crate::Version::V1_0, crate::StatusCode::Ok)
            .typed_header(&infos)
            .empty();

        assert_eq!(response, response2);
    }

    #[test]
    fn test_multiple_infos_v1() {
        let header = "url=rtsp://example.com/foo/audio;seq=45102;rtptime=12345678,url=rtsp://example.com/foo/video;seq=30211;rtptime=29567112";
        let response = crate::Response::builder(crate::Version::V1_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header)
            .empty();

        let infos = response.typed_header::<super::RtpInfos>().unwrap().unwrap();

        assert_eq!(
            infos,
            RtpInfos::V1(vec![
                v1::RtpInfo {
                    uri: url::Url::parse("rtsp://example.com/foo/audio").unwrap(),
                    seq: Some(45102),
                    rtptime: Some(12345678),
                },
                v1::RtpInfo {
                    uri: url::Url::parse("rtsp://example.com/foo/video").unwrap(),
                    seq: Some(30211),
                    rtptime: Some(29567112),
                }
            ])
        );

        let response2 = crate::Response::builder(crate::Version::V1_0, crate::StatusCode::Ok)
            .typed_header(&infos)
            .empty();

        assert_eq!(response, response2);
    }

    #[test]
    fn test_info_v1_quoted_url() {
        let header = "url=\"rtsp://example.com/foo/audio\";seq=45102;rtptime=12345678";
        let response = crate::Response::builder(crate::Version::V1_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header)
            .empty();

        let infos = response.typed_header::<super::RtpInfos>().unwrap().unwrap();

        assert_eq!(
            infos,
            RtpInfos::V1(vec![v1::RtpInfo {
                uri: url::Url::parse("rtsp://example.com/foo/audio").unwrap(),
                seq: Some(45102),
                rtptime: Some(12345678),
            }])
        );
    }

    #[test]
    fn test_multiple_infos_v1_quoted_url() {
        let header = "url=\"rtsp://example.com/foo/audio\";seq=45102;rtptime=12345678,url=\"rtsp://example.com/foo/video\";seq=30211;rtptime=29567112";
        let response = crate::Response::builder(crate::Version::V1_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header)
            .empty();

        let infos = response.typed_header::<super::RtpInfos>().unwrap().unwrap();

        assert_eq!(
            infos,
            RtpInfos::V1(vec![
                v1::RtpInfo {
                    uri: url::Url::parse("rtsp://example.com/foo/audio").unwrap(),
                    seq: Some(45102),
                    rtptime: Some(12345678),
                },
                v1::RtpInfo {
                    uri: url::Url::parse("rtsp://example.com/foo/video").unwrap(),
                    seq: Some(30211),
                    rtptime: Some(29567112),
                }
            ])
        );
    }

    #[test]
    fn test_quoted_params() {
        let header =
            "url=\"rtsp://example.com/foo/audio\" ssrc=0A13C760:x=\"a;b=c\";y=\"p\\\"q\";plain";
        let response = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header)
            .empty();

        let infos = response.typed_header::<super::RtpInfos>().unwrap().unwrap();

        let mut others: BTreeMap<String, Option<String>> = BTreeMap::new();
        others.insert("x".to_string(), Some("a;b=c".to_string()));
        others.insert("y".to_string(), Some("p\"q".to_string()));
        others.insert("plain".to_string(), None);

        assert_eq!(
            infos,
            RtpInfos::V2(vec![v2::RtpInfo {
                uri: url::Url::parse("rtsp://example.com/foo/audio").unwrap(),
                ssrc_infos: vec![v2::SsrcInfo {
                    ssrc: 0x0A13C760,
                    seq: None,
                    rtptime: None,
                    others,
                }],
            }])
        );

        // The serialized form has a different parameter order, so check that
        // it parses back to the same value.
        let response2 = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .typed_header(&infos)
            .empty();
        let header2 = response2
            .header(&crate::headers::RTP_INFO)
            .unwrap()
            .to_string();
        let response3 = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header2.as_str())
            .empty();

        assert_eq!(
            response3
                .typed_header::<super::RtpInfos>()
                .unwrap()
                .unwrap(),
            infos
        );
    }

    #[test]
    fn test_escaped_quote_in_url() {
        let header = "url=\"rtsp://example.com/foo\\\"bar\" ssrc=0A13C760:seq=45102";
        let response = crate::Response::builder(crate::Version::V2_0, crate::StatusCode::Ok)
            .header(crate::headers::RTP_INFO, header)
            .empty();

        let infos = response.typed_header::<super::RtpInfos>().unwrap().unwrap();

        assert_eq!(
            infos,
            RtpInfos::V2(vec![v2::RtpInfo {
                uri: url::Url::parse("rtsp://example.com/foo\"bar").unwrap(),
                ssrc_infos: vec![v2::SsrcInfo {
                    ssrc: 0x0A13C760,
                    seq: Some(45102),
                    rtptime: None,
                    others: BTreeMap::new(),
                }],
            }])
        );
    }

    #[test]
    fn test_empty_ssrc_infos_dropped() {
        let infos = RtpInfos::V2(vec![v2::RtpInfo {
            uri: url::Url::parse("rtsp://example.com/foo/audio").unwrap(),
            ssrc_infos: Vec::new(),
        }]);

        let mut headers = Headers::new();
        headers.insert(
            crate::headers::RTP_INFO,
            "url=\"rtsp://example.com/x\" ssrc=00000000",
        );
        infos.insert_into(&mut headers);

        assert!(headers.get(&crate::headers::RTP_INFO).is_none());
    }
}
