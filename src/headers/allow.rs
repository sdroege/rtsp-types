// Copyright (C) 2020 Sebastian Dröge <sebastian@centricular.com>
//
// Licensed under the MIT license, see the LICENSE file or <http://opensource.org/licenses/MIT>

use super::*;
use crate::Method;

/// `Allow` header ([RFC 7826 section 18.6](https://tools.ietf.org/html/rfc7826#section-18.6)).
#[derive(Debug, Clone)]
pub struct Allow(Vec<Method>);

impl std::ops::Deref for Allow {
    type Target = Vec<Method>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Allow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl AsRef<Vec<Method>> for Allow {
    fn as_ref(&self) -> &Vec<Method> {
        &self.0
    }
}

impl AsMut<Vec<Method>> for Allow {
    fn as_mut(&mut self) -> &mut Vec<Method> {
        &mut self.0
    }
}

impl From<Vec<Method>> for Allow {
    fn from(v: Vec<Method>) -> Self {
        Allow(v)
    }
}

impl<'a> From<&'a [Method]> for Allow {
    fn from(v: &'a [Method]) -> Self {
        Allow(v.to_vec())
    }
}

impl Allow {
    /// Creates a new `Allow` header builder.
    pub fn builder() -> AllowBuilder {
        AllowBuilder(Vec::new())
    }
}

/// Builder for the 'Allow' header.
#[derive(Debug, Clone)]
pub struct AllowBuilder(Vec<Method>);

impl AllowBuilder {
    /// Add the provided method to the `Allow` header.
    pub fn method(mut self, method: Method) -> Self {
        self.0.push(method);
        self
    }

    /// Build the `Allow` header.
    pub fn build(self) -> Allow {
        Allow(self.0)
    }
}

impl super::TypedHeader for Allow {
    fn from_headers(headers: impl AsRef<Headers>) -> Result<Option<Self>, HeaderParseError> {
        let headers = headers.as_ref();

        let header = match headers.get(&ALLOW) {
            None => return Ok(None),
            Some(header) => header,
        };

        let mut allow = Vec::new();
        for method in header.as_str().split(',') {
            let method = method.trim();

            allow.push(method.into());
        }

        Ok(Some(Allow(allow)))
    }

    fn insert_into(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        let mut allow = String::new();
        for method in &self.0 {
            if !allow.is_empty() {
                allow.push_str(", ");
            }

            allow.push_str(method.into());
        }

        headers.insert(ALLOW, allow);
    }
}

impl super::TypedAppendableHeader for Allow {
    fn append_to(&self, mut headers: impl AsMut<Headers>) {
        let headers = headers.as_mut();

        let mut allow = String::new();
        for method in &self.0 {
            if !allow.is_empty() {
                allow.push_str(", ");
            }

            allow.push_str(method.into());
        }

        headers.append(ALLOW, allow);
    }
}
