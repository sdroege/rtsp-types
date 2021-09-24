// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use std::borrow::{Borrow, Cow};
use std::collections::BTreeMap;
use std::convert::TryFrom;
use std::error;
use std::fmt;

use crate::message_ref::HeaderRef;

/// A collection of RTSP headers together with their values.
///
/// [`Request`](../struct.Request.html) and [`Response`](../struct.Response.html) implement
/// `AsRef<Headers>` and `AsMut<Headers>, which allows functions working with headers to be
/// implemented generically over those traits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Headers(pub(crate) BTreeMap<HeaderName, HeaderValue>);

impl Headers {
    pub(crate) fn new() -> Headers {
        Headers(BTreeMap::new())
    }

    pub(crate) fn from_headers_ref<'a, V: AsRef<[HeaderRef<'a>]>>(headers: V) -> Headers {
        let headers = headers.as_ref();
        let mut owned_headers = Headers::new();

        for header in headers.iter() {
            // Header values can be split over multiple lines, in which case there
            // will be a CRLF followed by one or more spaces/tabs. Here we replace
            // the CRLF and spaces/tabs with a single space.
            let mut value = Vec::with_capacity(header.value.len());
            let mut raw_value = header.value.as_bytes();
            while !raw_value.is_empty() {
                if raw_value.starts_with(b"\r\n") {
                    raw_value = raw_value.split_at(2).1;
                    if let Some((non_space_pos, _)) = raw_value
                        .iter()
                        .enumerate()
                        .find(|(_, b)| **b != b' ' && **b != b'\t')
                    {
                        value.push(b' ');
                        raw_value = raw_value.split_at(non_space_pos).1;
                    } else {
                        raw_value = &[];
                    }
                } else {
                    value.push(raw_value[0]);
                    raw_value = raw_value.split_at(1).1;
                }
            }

            /* This is both already checked when parsing */
            let name = HeaderName::try_from(header.name).expect("Non-ASCII characters");
            let value = String::from_utf8(value).expect("Non-UTF8 characters");

            owned_headers.append(name, HeaderValue::from(value));
        }

        owned_headers
    }

    /// Insert an RTSP header with its value.
    ///
    /// If a header with the same name already exists then its value will be replaced.
    ///
    /// See [`append`](#method.append) for appending additional values to a header.
    pub fn insert<V: Into<HeaderValue>>(&mut self, name: HeaderName, value: V) {
        let value = value.into();
        self.0.insert(name, value);
    }

    /// Appends a value to an existing RTSP header or inserts it.
    ///
    /// Additional values are comma separated as defined in [RFC 7826 section 5.2](https://tools.ietf.org/html/rfc7826#section-5.2).
    pub fn append<V: Into<HeaderValue>>(&mut self, name: HeaderName, value: V) {
        let value = value.into();
        self.0
            .entry(name)
            .and_modify(|old_value| {
                old_value.0.push_str(", ");
                old_value.0.push_str(&value.0);
            })
            .or_insert(value);
    }

    /// Insert a typed RTSP header.
    ///
    /// If a header with the same name already exists then its value will be replaced.
    pub fn insert_typed<H: TypedHeader>(&mut self, header: &H) {
        header.insert_into(self);
    }

    /// Append a typed RTSP header.
    ///
    /// This is only defined for header types that can be appended.
    pub fn append_typed<H: TypedAppendableHeader>(&mut self, header: &H) {
        header.append_to(self);
    }

    /// Removes and RTSP header if it exists.
    pub fn remove(&mut self, name: &HeaderName) {
        self.0.remove(name);
    }

    /// Gets an RTSP header value if it exists.
    pub fn get(&self, name: &HeaderName) -> Option<&HeaderValue> {
        self.0.get(name)
    }

    /// Gets a typed RTSP header value if it exists.
    pub fn get_typed<H: TypedHeader>(&self) -> Result<Option<H>, HeaderParseError> {
        H::from_headers(self)
    }

    /// Gets a mutable reference to an RTSP header value if it exists.
    pub fn get_mut(&mut self, name: &HeaderName) -> Option<&mut HeaderValue> {
        self.0.get_mut(name)
    }

    /// Iterator over all header name and value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &HeaderValue)> {
        self.0.iter()
    }

    /// Iterator over all header names.
    pub fn names(&self) -> impl Iterator<Item = &HeaderName> {
        self.0.keys()
    }

    /// Iterator over all header values.
    pub fn values(&self) -> impl Iterator<Item = &HeaderValue> {
        self.0.values()
    }
}

impl AsRef<Headers> for Headers {
    fn as_ref(&self) -> &Headers {
        self
    }
}

impl AsMut<Headers> for Headers {
    fn as_mut(&mut self) -> &mut Headers {
        self
    }
}

/// Representation of an RTSP header name.
///
/// This ensures that the header name only contains ASCII characters and comparisons on it are
/// case-insensitive as required by the RTSP RFC.
///
/// RTSP headers are not normalized to a specific case but stored in here as created.
#[derive(Debug, Clone, Eq)]
pub struct HeaderName(Cow<'static, str>);

impl HeaderName {
    /// Get a `&str` representation of the header.
    pub fn as_str(&self) -> &str {
        self.0.borrow()
    }

    /// Convert a static `&str` to a header name.
    ///
    /// This does not involve any heap allocations.
    pub fn from_static_str(v: &'static str) -> Result<HeaderName, AsciiError> {
        if !v.is_ascii() {
            return Err(AsciiError);
        }

        Ok(HeaderName(Cow::Borrowed(v)))
    }

    pub(crate) const fn from_static_str_unchecked(v: &'static str) -> HeaderName {
        Self(Cow::Borrowed(v))
    }
}

/// Create a header name from a `&[u8]`.
impl<'a> TryFrom<&'a [u8]> for HeaderName {
    type Error = AsciiError;

    fn try_from(v: &'a [u8]) -> Result<HeaderName, AsciiError> {
        if !v.is_ascii() {
            return Err(AsciiError);
        }

        let v = String::from_utf8(v.into()).map_err(|_| AsciiError)?;

        Ok(HeaderName(Cow::Owned(v)))
    }
}

/// Create a header name from a `&str`.
impl<'a> TryFrom<&'a str> for HeaderName {
    type Error = AsciiError;

    fn try_from(v: &'a str) -> Result<HeaderName, AsciiError> {
        Self::try_from(v.as_bytes())
    }
}

/// Create a header name from a `String`.
///
/// This takes ownership of the passed in `String` and does not involve an additional heap
/// allocation.
impl<'a> TryFrom<String> for HeaderName {
    type Error = AsciiError;

    fn try_from(v: String) -> Result<HeaderName, AsciiError> {
        if !v.is_ascii() {
            return Err(AsciiError);
        }

        Ok(HeaderName(Cow::Owned(v)))
    }
}

/// Case-insensitive comparison of header names.
impl PartialEq for HeaderName {
    fn eq(&self, other: &Self) -> bool {
        self.eq(other.as_str())
    }
}

/// Case-insensitive ordering of header names.
impl PartialOrd for HeaderName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Case-insensitive ordering of header names.
impl Ord for HeaderName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let s = self.0.as_bytes();
        let o = other.0.as_bytes();

        let len = std::cmp::min(s.len(), o.len());

        let s = &s[..len];
        let o = &o[..len];

        for (s, o) in Iterator::zip(s.iter(), o.iter()) {
            let mut s = *s;
            let mut o = *o;

            s.make_ascii_lowercase();
            o.make_ascii_lowercase();

            match s.cmp(&o) {
                std::cmp::Ordering::Equal => (),
                non_eq => return non_eq,
            }
        }

        s.len().cmp(&o.len())
    }
}

/// Case-insensitive hashing of header names.
impl std::hash::Hash for HeaderName {
    fn hash<H>(&self, h: &mut H)
    where
        H: std::hash::Hasher,
    {
        for b in self.0.as_bytes() {
            b.hash(h)
        }
    }
}

impl PartialEq<HeaderName> for &HeaderName {
    fn eq(&self, other: &HeaderName) -> bool {
        (*self).eq(other)
    }
}

impl PartialOrd<HeaderName> for &HeaderName {
    fn partial_cmp(&self, other: &HeaderName) -> Option<std::cmp::Ordering> {
        (*self).partial_cmp(other)
    }
}

impl PartialEq<String> for HeaderName {
    fn eq(&self, other: &String) -> bool {
        self.eq(other.as_str())
    }
}

impl PartialEq<str> for HeaderName {
    fn eq(&self, other: &str) -> bool {
        if self.0.len() != other.len() {
            return false;
        }

        for (s, o) in Iterator::zip(self.0.as_bytes().iter(), other.as_bytes().iter()) {
            let mut s = *s;
            let mut o = *o;

            s.make_ascii_lowercase();
            o.make_ascii_lowercase();

            if s != o {
                return false;
            }
        }

        true
    }
}

impl fmt::Display for HeaderName {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(self.as_str())
    }
}

/// Representation of a header value.
///
/// This is equivalent to a `String`.
// The following are OK because the explicit impls below are only for references
// and map to the derived impls.
#[allow(clippy::derive_ord_xor_partial_ord)]
#[allow(clippy::derive_hash_xor_eq)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeaderValue(String);

impl HeaderValue {
    /// Get a `&str` for the header value.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for HeaderValue {
    fn from(v: String) -> HeaderValue {
        HeaderValue(v)
    }
}

impl<'a> From<&'a str> for HeaderValue {
    fn from(v: &'a str) -> HeaderValue {
        HeaderValue(String::from(v))
    }
}

impl<'a> TryFrom<&'a [u8]> for HeaderValue {
    type Error = Utf8Error;

    fn try_from(v: &'a [u8]) -> Result<HeaderValue, Utf8Error> {
        std::str::from_utf8(v)
            .map(|s| HeaderValue::from(String::from(s)))
            .map_err(|_| Utf8Error)
    }
}

impl<'a> TryFrom<Vec<u8>> for HeaderValue {
    type Error = Utf8Error;

    fn try_from(v: Vec<u8>) -> Result<HeaderValue, Utf8Error> {
        String::from_utf8(v).map(HeaderValue).map_err(|_| Utf8Error)
    }
}

impl PartialEq<HeaderValue> for &HeaderValue {
    fn eq(&self, other: &HeaderValue) -> bool {
        (*self).eq(other)
    }
}

impl PartialOrd<HeaderValue> for &HeaderValue {
    fn partial_cmp(&self, other: &HeaderValue) -> Option<std::cmp::Ordering> {
        (*self).partial_cmp(other)
    }
}

impl PartialEq<String> for HeaderValue {
    fn eq(&self, other: &String) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<str> for HeaderValue {
    fn eq(&self, other: &str) -> bool {
        self.0.eq(other)
    }
}

impl fmt::Display for HeaderValue {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.write_str(self.0.as_str())
    }
}

/// Trait for typed headers.
pub trait TypedHeader: Sized {
    /// Parses the header from headers.
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError>;
    /// Inserts the header into headers.
    fn insert_into(&self, headers: impl AsMut<Headers>);
}

/// Trait for typed headers that can be appended.
pub trait TypedAppendableHeader: TypedHeader {
    /// Appends the header to headers.
    fn append_to(&self, headers: impl AsMut<Headers>);
}

/// Parsing a `HeaderName` failed because it contained non-ASCII characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsciiError;

impl error::Error for AsciiError {}

impl fmt::Display for AsciiError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Invalid ASCII")
    }
}

/// Parsing a `HeaderValue` failed because it contained invalid UTF-8 characters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Utf8Error;

impl error::Error for Utf8Error {}

impl fmt::Display for Utf8Error {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Invalid UTF-8")
    }
}

/// Parsing a `HeaderValue` failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderParseError;

impl error::Error for HeaderParseError {}

impl fmt::Display for HeaderParseError {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(fmt, "Error parsing error")
    }
}
