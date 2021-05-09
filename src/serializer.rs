// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;
use cf::bytes::{be_u16, be_u8};
use cf::combinator::{slice, string};
use cf::sequence::tuple;
use cf::{GenError, SerializeFn, WriteContext};
use cookie_factory as cf;
use std::io::Write;

fn rtsp_version<W: Write>(version: Version) -> impl SerializeFn<W> {
    move |out: WriteContext<W>| match version {
        Version::V1_0 => string("RTSP/1.0")(out),
        Version::V2_0 => string("RTSP/2.0")(out),
    }
}

fn method<W: Write>(method: MethodRef<'_>) -> impl SerializeFn<W> + '_ {
    move |out: WriteContext<W>| match method {
        MethodRef::Describe => string("DESCRIBE")(out),
        MethodRef::GetParameter => string("GET_PARAMETER")(out),
        MethodRef::Options => string("OPTIONS")(out),
        MethodRef::Pause => string("PAUSE")(out),
        MethodRef::Play => string("PLAY")(out),
        MethodRef::PlayNotify => string("PLAY_NOTIFY")(out),
        MethodRef::Redirect => string("REDIRECT")(out),
        MethodRef::Setup => string("SETUP")(out),
        MethodRef::SetParameter => string("SET_PARAMETER")(out),
        MethodRef::Announce => string("ANNOUNCE")(out),
        MethodRef::Record => string("RECORD")(out),
        MethodRef::Teardown => string("TEARDOWN")(out),
        MethodRef::Extension(s) => string(s)(out),
    }
}

fn header<'a, W: Write + 'a>(header: HeaderRef<'a>) -> impl SerializeFn<W> + 'a {
    tuple((string(header.name), string(": "), string(header.value)))
}

fn headers<'a, W: Write + 'a>(headers: TinyVec<[HeaderRef<'a>; 16]>) -> impl SerializeFn<W> + 'a {
    move |mut w: WriteContext<W>| {
        let headers = headers.clone();
        for h in headers {
            w = header(h)(w)?;
            w = string("\r\n")(w)?;
        }

        Ok(w)
    }
}

fn request_line<'a, W: Write + 'a>(request_line: RequestLine<'a>) -> impl SerializeFn<W> + 'a {
    tuple((
        method(request_line.method),
        string(" "),
        string(request_line.request_uri.unwrap_or("*")),
        string(" "),
        rtsp_version(request_line.version),
        string("\r\n"),
    ))
}

pub(crate) fn request<'a, W: Write + 'a>(request: RequestRef<'a>) -> impl SerializeFn<W> + 'a {
    tuple((
        request_line(RequestLine {
            method: request.method,
            request_uri: request.request_uri,
            version: request.version,
        }),
        headers(request.headers),
        string("\r\n"),
        slice(request.body),
    ))
}

fn status_code<W: Write>(status: StatusCode) -> impl SerializeFn<W> {
    move |mut w: WriteContext<W>| match write!(w, "{}", u16::from(status)) {
        Err(io) => Err(GenError::IoError(io)),
        Ok(()) => Ok(w),
    }
}

fn status_line<'a, W: Write + 'a>(status_line: StatusLine<'a>) -> impl SerializeFn<W> + 'a {
    tuple((
        rtsp_version(status_line.version),
        string(" "),
        status_code(status_line.status),
        string(" "),
        string(status_line.reason_phrase),
        string("\r\n"),
    ))
}

pub(crate) fn response<'a, W: Write + 'a>(response: ResponseRef<'a>) -> impl SerializeFn<W> + 'a {
    tuple((
        status_line(StatusLine {
            version: response.version,
            status: response.status,
            reason_phrase: response.reason_phrase,
        }),
        headers(response.headers),
        string("\r\n"),
        slice(response.body),
    ))
}

pub(crate) fn data<'a, W: Write + 'a>(data: DataRef<'a>) -> impl SerializeFn<W> + 'a {
    tuple((
        string("$"),
        be_u8(data.channel_id),
        be_u16(data.len() as u16),
        slice(data.as_slice()),
    ))
}

pub(crate) fn message<'a, W: Write + 'a>(message: MessageRef<'a>) -> impl SerializeFn<W> + 'a {
    move |w: WriteContext<W>| {
        let message = message.clone();
        match message {
            MessageRef::Request(req) => request(req)(w),
            MessageRef::Response(resp) => response(resp)(w),
            MessageRef::Data(d) => data(d)(w),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tinyvec::tiny_vec;

    #[test]
    fn test_request_line() {
        let mut v = vec![];
        cf::gen_simple(
            request_line(RequestLine {
                method: MethodRef::Options,
                request_uri: Some("rtsp://media.example.com/movie/twister.3gp"),
                version: Version::V2_0,
            }),
            &mut v,
        )
        .unwrap();

        assert_eq!(
            v,
            &b"OPTIONS rtsp://media.example.com/movie/twister.3gp RTSP/2.0\r\n"[..],
        );
    }

    #[test]
    fn test_request_line_without_uri() {
        let mut v = vec![];
        cf::gen_simple(
            request_line(RequestLine {
                method: MethodRef::Options,
                request_uri: None,
                version: Version::V2_0,
            }),
            &mut v,
        )
        .unwrap();

        assert_eq!(v, &b"OPTIONS * RTSP/2.0\r\n"[..],);
    }

    #[test]
    fn test_status_line() {
        let mut v = vec![];
        cf::gen_simple(
            status_line(StatusLine {
                version: Version::V2_0,
                status: StatusCode::Ok,
                reason_phrase: "All Good",
            }),
            &mut v,
        )
        .unwrap();

        assert_eq!(v, &b"RTSP/2.0 200 All Good\r\n"[..],);
    }

    #[test]
    fn test_options() {
        let mut v = vec![];
        cf::gen_simple(
            request(RequestRef {
                method: MethodRef::Options,
                version: Version::V2_0,
                request_uri: Some("rtsp://media.example.com/movie/twister.3gp"),
                headers: tiny_vec!(
                    HeaderRef {
                        name: "CSeq",
                        value: "1"
                    },
                    HeaderRef {
                        name: "Supported",
                        value: "play.basic, play.scale"
                    },
                    HeaderRef {
                        name: "User-Agent",
                        value: "PhonyClient/1.2"
                    }
                ),
                body: &[],
            }),
            &mut v,
        )
        .unwrap();

        assert_eq!(
            v,
            &b"OPTIONS rtsp://media.example.com/movie/twister.3gp RTSP/2.0\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
\r\n"[..]
        );
    }

    #[test]
    fn test_options_without_uri() {
        let mut v = vec![];
        cf::gen_simple(
            request(RequestRef {
                method: MethodRef::Options,
                version: Version::V2_0,
                request_uri: None,
                headers: tiny_vec!(
                    HeaderRef {
                        name: "CSeq",
                        value: "1"
                    },
                    HeaderRef {
                        name: "Supported",
                        value: "play.basic, play.scale"
                    },
                    HeaderRef {
                        name: "User-Agent",
                        value: "PhonyClient/1.2"
                    }
                ),
                body: &[],
            }),
            &mut v,
        )
        .unwrap();

        assert_eq!(
            v,
            &b"OPTIONS * RTSP/2.0\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
\r\n"[..],
        );
    }

    #[test]
    fn test_options_with_body() {
        let mut v = vec![];
        cf::gen_simple(
            request(RequestRef {
                method: MethodRef::Options,
                version: Version::V2_0,
                request_uri: Some("rtsp://media.example.com/movie/twister.3gp"),
                headers: tiny_vec!(
                    HeaderRef {
                        name: "CSeq",
                        value: "1"
                    },
                    HeaderRef {
                        name: "Supported",
                        value: "play.basic, play.scale"
                    },
                    HeaderRef {
                        name: "User-Agent",
                        value: "PhonyClient/1.2"
                    },
                    HeaderRef {
                        name: "Content-Length",
                        value: "10"
                    }
                ),
                body: &b"0123456789"[..],
            }),
            &mut v,
        )
        .unwrap();

        assert_eq!(
            v,
            &b"OPTIONS rtsp://media.example.com/movie/twister.3gp RTSP/2.0\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
Content-Length: 10\r\n\
\r\n\
0123456789"[..]
        );
    }

    #[test]
    fn test_options_with_body_without_uri() {
        let mut v = vec![];
        cf::gen_simple(
            request(RequestRef {
                method: MethodRef::Options,
                version: Version::V2_0,
                request_uri: None,
                headers: tiny_vec!(
                    HeaderRef {
                        name: "CSeq",
                        value: "1"
                    },
                    HeaderRef {
                        name: "Supported",
                        value: "play.basic, play.scale"
                    },
                    HeaderRef {
                        name: "User-Agent",
                        value: "PhonyClient/1.2"
                    },
                    HeaderRef {
                        name: "Content-Length",
                        value: "10"
                    }
                ),
                body: &b"0123456789"[..],
            }),
            &mut v,
        )
        .unwrap();
        assert_eq!(
            v,
            &b"OPTIONS * RTSP/2.0\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
Content-Length: 10\r\n\
\r\n\
0123456789"[..]
        );
    }

    #[test]
    fn test_ok() {
        let mut v = vec![];
        cf::gen_simple(
            response(ResponseRef {
                version: Version::V2_0,
                status: StatusCode::Ok,
                reason_phrase: "All Good",
                headers: tiny_vec!(
                    HeaderRef {
                        name: "CSeq",
                        value: "1"
                    },
                    HeaderRef {
                        name: "Supported",
                        value: "play.basic, play.scale"
                    },
                    HeaderRef {
                        name: "User-Agent",
                        value: "PhonyClient/1.2"
                    }
                ),
                body: &[],
            }),
            &mut v,
        )
        .unwrap();

        assert_eq!(
            v,
            &b"RTSP/2.0 200 All Good\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
\r\n"[..]
        );
    }

    #[test]
    fn test_ok_with_body() {
        let mut v = vec![];
        cf::gen_simple(
            response(ResponseRef {
                version: Version::V2_0,
                status: StatusCode::Ok,
                reason_phrase: "All Good",
                headers: tiny_vec!(
                    HeaderRef {
                        name: "CSeq",
                        value: "1"
                    },
                    HeaderRef {
                        name: "Supported",
                        value: "play.basic, play.scale"
                    },
                    HeaderRef {
                        name: "User-Agent",
                        value: "PhonyClient/1.2"
                    },
                    HeaderRef {
                        name: "Content-Length",
                        value: "10"
                    }
                ),
                body: &b"0123456789"[..],
            }),
            &mut v,
        )
        .unwrap();

        assert_eq!(
            v,
            &b"RTSP/2.0 200 All Good\r\n\
CSeq: 1\r\n\
Supported: play.basic, play.scale\r\n\
User-Agent: PhonyClient/1.2\r\n\
Content-Length: 10\r\n\
\r\n\
0123456789"[..]
        );
    }

    #[test]
    fn test_data() {
        let mut v = vec![];
        cf::gen_simple(
            data(DataRef::from_slice(12, &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9])),
            &mut v,
        )
        .unwrap();

        assert_eq!(v, &[b'$', 12, 0, 10, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9][..],);
    }
}
