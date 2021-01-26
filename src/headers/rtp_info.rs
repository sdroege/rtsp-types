// Copyright (C) 2021 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;

use std::collections::BTreeMap;

/// `RTP-Info` header ([RFC 7826 section 18.45](https://tools.ietf.org/html/rfc7826#section-18.45)).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
    /// RTP-Info.
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

        use crate::nom_extensions::separated_list1_fold;
        use nom::bytes::complete::{tag, take_while};
        use nom::combinator::{all_consuming, map, map_parser, map_res};
        use nom::multi::separated_list1;
        use nom::sequence::separated_pair;
        use nom::{Err, IResult};
        use std::str;

        fn param(input: &[u8]) -> IResult<&[u8], (&str, &str)> {
            map(
                separated_pair(
                    map_res(take_while(|b| b != b'='), str::from_utf8),
                    tag("="),
                    map_res(take_while(|b| b != b';'), str::from_utf8),
                ),
                |(name, value)| (name, value),
            )(input)
        }

        fn rtp_info(input: &[u8]) -> IResult<&[u8], RtpInfo> {
            #[derive(Clone, Default)]
            struct Info<'a> {
                uri: Option<&'a str>,
                seq: Option<&'a str>,
                rtptime: Option<&'a str>,
            }

            map_res(
                separated_list1_fold(tag(b";"), param, Info::default(), |mut acc, param| {
                    match param.0 {
                        "url" => acc.uri = Some(param.1),
                        "seq" => acc.seq = Some(param.1),
                        "rtptime" => acc.rtptime = Some(param.1),
                        _ => (),
                    }

                    acc
                }),
                |info| {
                    let uri = match info.uri {
                        None => {
                            return Err(Err::Error(nom::error::Error::new(
                                input,
                                nom::error::ErrorKind::Tag,
                            )));
                        }
                        Some(uri) => url::Url::parse(uri).map_err(|err| {
                            use nom::error::FromExternalError;

                            Err::Error(nom::error::Error::from_external_error(
                                input,
                                nom::error::ErrorKind::MapRes,
                                err,
                            ))
                        })?,
                    };

                    let seq = info
                        .seq
                        .map(|s| s.parse::<u16>())
                        .transpose()
                        .map_err(|err| {
                            use nom::error::FromExternalError;

                            Err::Error(nom::error::Error::from_external_error(
                                input,
                                nom::error::ErrorKind::MapRes,
                                err,
                            ))
                        })?;

                    let rtptime =
                        info.rtptime
                            .map(|s| s.parse::<u32>())
                            .transpose()
                            .map_err(|err| {
                                use nom::error::FromExternalError;

                                Err::Error(nom::error::Error::from_external_error(
                                    input,
                                    nom::error::ErrorKind::MapRes,
                                    err,
                                ))
                            })?;

                    Ok(RtpInfo { uri, seq, rtptime })
                },
            )(input)
        }

        pub(crate) fn rtp_infos(input: &[u8]) -> IResult<&[u8], Vec<RtpInfo>> {
            all_consuming(separated_list1(
                tag(","),
                map_parser(take_while(|b| b != b','), rtp_info),
            ))(input)
        }
    }
}

pub mod v2 {
    use super::*;

    /// RTP-Info.
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct RtpInfo {
        /// Stream URI.
        pub uri: url::Url,
        /// SSRC information.
        pub ssrc_infos: Vec<SsrcInfo>,
    }

    /// SSRC Information.
    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

        use crate::nom_extensions::separated_list1_fold;
        use nom::branch::alt;
        use nom::bytes::complete::{tag, take, take_while};
        use nom::character::is_alphanumeric;
        use nom::combinator::{all_consuming, cond, flat_map, map, map_res, opt};
        use nom::multi::separated_list1;
        use nom::sequence::tuple;
        use nom::{Err, IResult, Needed};
        use std::str;

        fn token(input: &[u8]) -> IResult<&[u8], &[u8]> {
            fn is_token_char(i: u8) -> bool {
                is_alphanumeric(i) || b"!#$%&'*+-.^_`|~".contains(&i)
            }

            take_while(is_token_char)(input)
        }

        fn quoted_string(input: &[u8]) -> IResult<&[u8], &[u8]> {
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
                } else {
                    break;
                }
            }

            // Did not end with a quote
            if o.is_empty() {
                return Err(Err::Incomplete(Needed::Size(NonZeroUsize::new(1).unwrap())));
            }

            let (fst, snd) = i.split_at(i.len() - o.len());
            if fst.is_empty() {
                return Err(Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::TakeWhile1,
                )));
            }

            Ok((snd, fst))
        }

        fn param(input: &[u8]) -> IResult<&[u8], (&str, Option<&str>)> {
            if input.is_empty() {
                return Err(Err::Error(nom::error::Error::new(
                    input,
                    nom::error::ErrorKind::Eof,
                )));
            }

            map(
                tuple((
                    map_res(token, str::from_utf8),
                    flat_map(opt(tag(b"=")), |res| {
                        cond(
                            res.is_some(),
                            map_res(alt((quoted_string, token)), str::from_utf8),
                        )
                    }),
                )),
                |(name, value)| (name, value),
            )(input)
        }

        fn ssrc_info(input: &[u8]) -> IResult<&[u8], SsrcInfo> {
            map_res(
                tuple((
                    tag(b"ssrc="),
                    map_res(map_res(take(8usize), str::from_utf8), |s| {
                        u32::from_str_radix(s, 16)
                    }),
                    flat_map(opt(tag(b":")), |res| {
                        cond(
                            res.is_some(),
                            separated_list1_fold(
                                tag(b";"),
                                param,
                                BTreeMap::new(),
                                |mut acc, param| {
                                    acc.insert(String::from(param.0), param.1.map(String::from));

                                    acc
                                },
                            ),
                        )
                    }),
                )),
                |(_, ssrc, params)| {
                    let mut params = params.unwrap_or_default();

                    let seq = if let Some((_, Some(seq))) = params.remove_entry("seq") {
                        match seq.parse::<u16>() {
                            Ok(seq) => Some(seq),
                            Err(err) => {
                                use nom::error::FromExternalError;

                                return Err(Err::Error(nom::error::Error::from_external_error(
                                    input,
                                    nom::error::ErrorKind::MapRes,
                                    err,
                                )));
                            }
                        }
                    } else {
                        None
                    };

                    let rtptime = if let Some((_, Some(rtptime))) = params.remove_entry("rtptime") {
                        Some(rtptime.parse::<u32>().map_err(|err| {
                            use nom::error::FromExternalError;

                            Err::Error(nom::error::Error::from_external_error(
                                input,
                                nom::error::ErrorKind::MapRes,
                                err,
                            ))
                        })?)
                    } else {
                        None
                    };

                    Ok(SsrcInfo {
                        ssrc,
                        seq,
                        rtptime,
                        others: params,
                    })
                },
            )(input)
        }

        fn rtp_info(input: &[u8]) -> IResult<&[u8], RtpInfo> {
            map(
                tuple((
                    tag(b"url=\""),
                    map_res(
                        map_res(take_while(|b| b != b'"'), str::from_utf8),
                        url::Url::parse,
                    ),
                    tag(b"\" "),
                    separated_list1(tag(b" "), ssrc_info),
                )),
                |(_, uri, _, ssrc_infos)| RtpInfo { uri, ssrc_infos },
            )(input)
        }

        pub(crate) fn rtp_infos(input: &[u8]) -> IResult<&[u8], Vec<RtpInfo>> {
            all_consuming(separated_list1(tag(","), rtp_info))(input)
        }
    }
}

mod parser {
    use super::*;

    use nom::IResult;

    pub(super) fn rtp_infos(input: &[u8]) -> IResult<&[u8], RtpInfos> {
        if input.starts_with(b"url=\"") {
            let (rem, infos) = v2::parser::rtp_infos(input)?;
            Ok((rem, RtpInfos::V2(infos)))
        } else {
            let (rem, infos) = v1::parser::rtp_infos(input)?;
            Ok((rem, RtpInfos::V1(infos)))
        }
    }
}

impl super::TypedHeader for RtpInfos {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&RTP_INFO) {
            None => return Ok(None),
            Some(header) => header,
        };

        let (_rem, rtp_info) =
            parser::rtp_infos(header.as_str().as_bytes()).map_err(|_| HeaderParseError)?;

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
                        write!(&mut infos, ";seq={}", seq).unwrap();
                    }

                    if let Some(rtptime) = info.rtptime {
                        write!(&mut infos, ";rtptime={}", rtptime).unwrap();
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
                            write!(&mut infos, "seq={}", seq).unwrap();
                            need_semi = true;
                        }

                        if let Some(rtptime) = ssrc.rtptime {
                            if need_semi {
                                infos.push(';');
                            }
                            write!(&mut infos, "rtptime={}", rtptime).unwrap();
                            need_semi = true;
                        }

                        for (name, value) in &ssrc.others {
                            if need_semi {
                                infos.push(';');
                            }
                            if let Some(value) = value {
                                write!(&mut infos, "{}={}", name, value).unwrap();
                            } else {
                                write!(&mut infos, "{}", name).unwrap();
                            }
                            need_semi = true;
                        }
                    }
                }
            }
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
}
