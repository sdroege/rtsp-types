// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::parser_helpers::{join_list, split_list};
use super::*;
use crate::Method;

/// `Public` header ([RFC 7826 section 18.39](https://tools.ietf.org/html/rfc7826#section-18.39)).
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Public(Vec<Method>);

impl std::ops::Deref for Public {
    type Target = Vec<Method>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Public {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Vec<Method>> for Public {
    fn as_ref(&self) -> &Vec<Method> {
        &self.0
    }
}

impl AsMut<Vec<Method>> for Public {
    fn as_mut(&mut self) -> &mut Vec<Method> {
        &mut self.0
    }
}

impl From<Vec<Method>> for Public {
    fn from(v: Vec<Method>) -> Self {
        Public(v)
    }
}

impl<'a> From<&'a [Method]> for Public {
    fn from(v: &'a [Method]) -> Self {
        Public(v.to_vec())
    }
}

impl Public {
    /// Creates a new `Public` header builder.
    pub fn builder() -> PublicBuilder {
        PublicBuilder(Vec::new())
    }
}

/// Builder for the `Public` header.
#[derive(Debug, Clone)]
pub struct PublicBuilder(Vec<Method>);

impl PublicBuilder {
    /// Add the provided method to the `Public` header.
    pub fn method(mut self, method: Method) -> Self {
        self.0.push(method);
        self
    }

    /// Build the `Public` header.
    pub fn build(self) -> Public {
        Public(self.0)
    }
}

impl super::TypedHeader for Public {
    fn header_name() -> Option<HeaderName> {
        Some(PUBLIC.clone())
    }
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&PUBLIC) {
            None => return Ok(None),
            Some(header) => header,
        };

        let public = split_list(header.as_str()).map(Method::from).collect();

        Ok(Some(Public(public)))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        if self.0.is_empty() {
            // An empty list cannot be represented. Remove any existing
            // header instead of writing an empty header value.
            headers.remove(&PUBLIC);
            return;
        }

        headers.insert(PUBLIC, join_list(self.0.iter().map(|method| method.into())));
    }
}

impl super::TypedAppendableHeader for Public {
    fn append_to(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        if self.0.is_empty() {
            return;
        }

        headers.append(PUBLIC, join_list(self.0.iter().map(|method| method.into())));
    }
}
