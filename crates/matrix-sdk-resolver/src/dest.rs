// Copyright 2025 Tuwunel Contributors
// Copyright 2026 The Harana Contributors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
//
// Ported from tuwunel `src/service/resolver/fed.rs`, with the port held as a
// `u16` rather than a formatted string.

//! Where a federation request connects to.

use std::{
    fmt,
    net::{IpAddr, SocketAddr},
};

/// The default federation port, used whenever nothing names another.
pub const DEFAULT_PORT: u16 = 8448;

/// A resolved federation destination.
///
/// The distinction is kept because it decides what happens next: an IP literal
/// is connected to as written and is never resolved further, while a named host
/// goes through DNS.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FedDest {
    /// An address that needs no name resolution.
    Literal(SocketAddr),

    /// A host name and the port the ladder settled on.
    Named(String, u16),
}

impl FedDest {
    /// Parses an IP literal, with or without a port.
    ///
    /// Returns `None` for anything that is not an IP literal, including a host
    /// name that merely looks like one.
    #[must_use]
    pub fn parse_literal(dest: &str) -> Option<Self> {
        if let Ok(addr) = dest.parse::<SocketAddr>() {
            return Some(Self::Literal(addr));
        }

        dest.parse::<IpAddr>().ok().map(|addr| Self::Literal(SocketAddr::new(addr, DEFAULT_PORT)))
    }

    /// Splits a `host:port` pair, defaulting the port when there is none.
    ///
    /// An IPv6 literal is not a host name, so this returns `None` for one;
    /// [`Self::parse_literal`] handles those.
    #[must_use]
    pub fn parse_named(dest: &str) -> Option<Self> {
        if dest.starts_with('[') {
            return None;
        }

        match dest.split_once(':') {
            None => Some(Self::Named(dest.to_owned(), DEFAULT_PORT)),
            Some((host, port)) => {
                let port = port.parse().ok()?;
                Some(Self::Named(host.to_owned(), port))
            }
        }
    }

    /// The host part, without a port.
    #[must_use]
    pub fn hostname(&self) -> String {
        match self {
            Self::Literal(addr) => addr.ip().to_string(),
            Self::Named(host, _) => host.clone(),
        }
    }

    /// The port a request connects to.
    #[must_use]
    pub fn port(&self) -> u16 {
        match self {
            Self::Literal(addr) => addr.port(),
            Self::Named(_, port) => *port,
        }
    }

    /// The `https://` URL a federation request is sent to.
    #[must_use]
    pub fn https_url(&self) -> String {
        format!("https://{self}")
    }
}

impl fmt::Display for FedDest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(addr) => write!(f, "{addr}"),
            Self::Named(host, port) => write!(f, "{host}:{port}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::SocketAddr;

    use super::{DEFAULT_PORT, FedDest};

    #[test]
    fn test_an_ip_literal_defaults_its_port() {
        assert_eq!(
            FedDest::parse_literal("1.2.3.4"),
            Some(FedDest::Literal(SocketAddr::from(([1, 2, 3, 4], DEFAULT_PORT))))
        );
        assert_eq!(
            FedDest::parse_literal("1.2.3.4:1234"),
            Some(FedDest::Literal(SocketAddr::from(([1, 2, 3, 4], 1234))))
        );
    }

    #[test]
    fn test_an_ipv6_literal_is_recognized_in_both_forms() {
        assert_eq!(FedDest::parse_literal("::1").map(|dest| dest.port()), Some(DEFAULT_PORT));
        assert_eq!(FedDest::parse_literal("[::1]:1234").map(|dest| dest.port()), Some(1234));

        // Bracketed with no port is not a socket address and not an IP address.
        assert_eq!(FedDest::parse_literal("[::1]"), None);
    }

    #[test]
    fn test_a_host_name_is_not_a_literal() {
        assert_eq!(FedDest::parse_literal("matrix.org"), None);
        assert_eq!(FedDest::parse_literal("1.2.3.4.example.com"), None);
    }

    #[test]
    fn test_a_named_host_takes_its_port_or_the_default() {
        assert_eq!(
            FedDest::parse_named("matrix.org"),
            Some(FedDest::Named("matrix.org".to_owned(), DEFAULT_PORT))
        );
        assert_eq!(
            FedDest::parse_named("matrix.org:1234"),
            Some(FedDest::Named("matrix.org".to_owned(), 1234))
        );

        // A port that is not a number is not a port.
        assert_eq!(FedDest::parse_named("matrix.org:https"), None);
    }

    #[test]
    fn test_the_url_carries_the_settled_port() {
        assert_eq!(
            FedDest::Named("matrix.org".to_owned(), 8448).https_url(),
            "https://matrix.org:8448"
        );
        assert_eq!(FedDest::parse_literal("[::1]:1234").unwrap().https_url(), "https://[::1]:1234");
    }
}
