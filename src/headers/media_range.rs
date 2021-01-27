// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::Range;
use super::*;

/// `Media-Range` header ([RFC 7826 section 18.30](https://tools.ietf.org/html/rfc7826#section-18.30)).
#[derive(Debug, Clone)]
pub struct MediaRange(Vec<Range>);

impl std::ops::Deref for MediaRange {
    type Target = Vec<Range>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for MediaRange {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Vec<Range>> for MediaRange {
    fn as_ref(&self) -> &Vec<Range> {
        &self.0
    }
}

impl AsMut<Vec<Range>> for MediaRange {
    fn as_mut(&mut self) -> &mut Vec<Range> {
        &mut self.0
    }
}

impl From<Vec<Range>> for MediaRange {
    fn from(v: Vec<Range>) -> Self {
        MediaRange(v)
    }
}

impl<'a> From<&'a [Range]> for MediaRange {
    fn from(v: &'a [Range]) -> Self {
        MediaRange(v.to_vec())
    }
}

impl MediaRange {
    /// Creates a new `Media-Range` header builder.
    pub fn builder() -> MediaRangeBuilder {
        MediaRangeBuilder(Vec::new())
    }
}

/// Builder for the 'Media-Range' header.
#[derive(Debug, Clone)]
pub struct MediaRangeBuilder(Vec<Range>);

impl MediaRangeBuilder {
    /// Add the provided range to the `Media-Range` header.
    pub fn range(mut self, range: Range) -> Self {
        self.0.push(range);
        self
    }

    /// Build the `Media-Range` header.
    pub fn build(self) -> MediaRange {
        MediaRange(self.0)
    }
}

impl super::TypedHeader for MediaRange {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&MEDIA_RANGE) {
            None => return Ok(None),
            Some(header) => header,
        };

        let mut ranges = Vec::new();
        for range in header.as_str().split(',') {
            let range = range.trim();

            ranges.push(range.parse()?);
        }

        Ok(Some(MediaRange(ranges)))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        let mut ranges = String::new();
        for range in &self.0 {
            if !ranges.is_empty() {
                ranges.push_str(", ");
            }

            ranges.push_str(&range.to_string());
        }

        headers.insert(MEDIA_RANGE, ranges);
    }
}

impl super::TypedAppendableHeader for MediaRange {
    fn append_to(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        let mut ranges = String::new();
        for range in &self.0 {
            if !ranges.is_empty() {
                ranges.push_str(", ");
            }

            ranges.push_str(&range.to_string());
        }

        headers.append(MEDIA_RANGE, ranges);
    }
}
