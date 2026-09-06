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
// The ladder is ported from tuwunel `src/service/resolver/actual.rs`, whose
// numbered steps follow the specification's own numbering.

#![doc = include_str!("../../docs/resolver.md")]
#![warn(missing_docs, missing_debug_implementations)]

mod dest;
mod well_known;

use std::future::Future;

use harana_matrix_common::ServerName;
use tracing::{debug, instrument};

pub use self::{
    dest::{DEFAULT_PORT, FedDest},
    well_known::{WELL_KNOWN_MAX_BYTES, parse_well_known, well_known_url},
};

/// An SRV record's target, as the ladder needs it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SrvTarget {
    /// The host the record points at.
    pub target: String,

    /// The port the record names.
    pub port: u16,
}

impl SrvTarget {
    /// Creates a target.
    #[must_use]
    pub fn new(target: impl Into<String>, port: u16) -> Self {
        Self { target: target.into(), port }
    }
}

/// A server name resolved to everything a federation request needs.
///
/// The three names differ, and conflating them is the usual bug: an SRV record
/// redirects where the request connects but changes neither the `Host` header
/// nor the name TLS validates, while delegation changes both of those but is
/// not itself a connection target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedServer {
    /// Where to connect.
    pub destination: FedDest,

    /// What to send as the `Host` header.
    pub host_header: String,

    /// The name the server's TLS certificate is validated against.
    pub tls_name: String,
}

/// The SRV names to query for a host, in the order they are tried.
///
/// `_matrix-fed._tcp` is the current name; `_matrix._tcp` is the deprecated one
/// that servers configured before it still answer on. A resolver stops at the
/// first name that answers.
#[must_use]
pub fn srv_names(host: &str) -> [String; 2] {
    [format!("_matrix-fed._tcp.{host}"), format!("_matrix._tcp.{host}")]
}

/// Resolves a server name to a federation destination.
///
/// `well_known` fetches the server's delegation document and returns the
/// delegated server name (see [`parse_well_known`]); `srv` resolves the SRV
/// names for a host (see [`srv_names`]). Both return `None` when there is
/// nothing to find, which is an ordinary answer: the ladder continues to its
/// next step.
///
/// The steps are the specification's, in its order:
///
/// 1. an IP literal, with the port it names or the default;
/// 2. a host name with an explicit port;
/// 3. a delegated name from `.well-known`, itself resolved by the same rules
///    (literal, explicit port, SRV, or the default port);
/// 4. an SRV record for the server name;
/// 5. the server name on the default port.
#[instrument(level = "debug", skip_all, fields(server_name = server_name.as_str()))]
pub async fn resolve<WellKnown, WellKnownFut, Srv, SrvFut>(
    server_name: &ServerName,
    well_known: WellKnown,
    srv: Srv,
) -> ResolvedServer
where
    WellKnown: Fn(String) -> WellKnownFut,
    WellKnownFut: Future<Output = Option<String>>,
    Srv: Fn(String) -> SrvFut,
    SrvFut: Future<Output = Option<SrvTarget>>,
{
    let name = server_name.as_str();

    // 1: IP literal with provided or default port.
    if let Some(destination) = FedDest::parse_literal(name) {
        debug!("1: IP literal");
        return ResolvedServer {
            destination,
            host_header: name.to_owned(),
            tls_name: server_name.host().to_owned(),
        };
    }

    // 2: hostname with included port.
    if let Some(port) = explicit_port(name) {
        debug!("2: hostname with explicit port");
        return ResolvedServer {
            destination: FedDest::Named(server_name.host().to_owned(), port),
            host_header: name.to_owned(),
            tls_name: server_name.host().to_owned(),
        };
    }

    // 3: a delegated destination from the well-known document.
    if let Some(delegated) = well_known(name.to_owned()).await {
        debug!(%delegated, "3: delegated by .well-known");
        return resolve_delegated(&delegated, srv).await;
    }

    // 4: an SRV record for the server name.
    if let Some(target) = srv(name.to_owned()).await {
        debug!(?target, "4: SRV record");
        return ResolvedServer {
            destination: FedDest::Named(target.target, target.port),
            host_header: name.to_owned(),
            tls_name: name.to_owned(),
        };
    }

    // 5: the server name itself, on the default port.
    debug!("5: no delegation and no SRV record");
    ResolvedServer {
        destination: FedDest::Named(name.to_owned(), DEFAULT_PORT),
        host_header: name.to_owned(),
        tls_name: name.to_owned(),
    }
}

/// Resolves the name a `.well-known` document delegated to.
///
/// The delegated name is resolved by the same rules as the original, except
/// that it cannot delegate again: a `.well-known` document is not fetched for
/// it.
async fn resolve_delegated<Srv, SrvFut>(delegated: &str, srv: Srv) -> ResolvedServer
where
    Srv: Fn(String) -> SrvFut,
    SrvFut: Future<Output = Option<SrvTarget>>,
{
    // 3.1: IP literal in the .well-known file.
    if let Some(destination) = FedDest::parse_literal(delegated) {
        debug!("3.1: IP literal");
        return ResolvedServer {
            destination,
            host_header: delegated.to_owned(),
            tls_name: hostname_of(delegated),
        };
    }

    // 3.2: hostname with port in the .well-known file.
    if let Some(port) = explicit_port(delegated) {
        debug!("3.2: hostname with explicit port");
        return ResolvedServer {
            destination: FedDest::Named(hostname_of(delegated), port),
            host_header: delegated.to_owned(),
            tls_name: hostname_of(delegated),
        };
    }

    // 3.3: an SRV record for the delegated hostname.
    if let Some(target) = srv(delegated.to_owned()).await {
        debug!(?target, "3.3: SRV record");
        return ResolvedServer {
            destination: FedDest::Named(target.target, target.port),
            host_header: delegated.to_owned(),
            tls_name: delegated.to_owned(),
        };
    }

    // 3.4: the delegated hostname on the default port.
    debug!("3.4: no SRV record for the delegated hostname");
    ResolvedServer {
        destination: FedDest::Named(delegated.to_owned(), DEFAULT_PORT),
        host_header: delegated.to_owned(),
        tls_name: delegated.to_owned(),
    }
}

/// The port a destination states outright, if it states one.
fn explicit_port(dest: &str) -> Option<u16> {
    dest.rsplit_once(':').and_then(|(_, port)| port.parse().ok())
}

/// The host part of a destination that may carry a port.
fn hostname_of(dest: &str) -> String {
    match dest.rsplit_once(':') {
        Some((host, port)) if port.parse::<u16>().is_ok() => host.to_owned(),
        _ => dest.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use harana_matrix_common::{OwnedServerName, ServerName};
    use harana_matrix_macros::async_test;

    use super::{DEFAULT_PORT, FedDest, ResolvedServer, SrvTarget, resolve, srv_names};

    /// A resolver that finds nothing.
    async fn nothing(_: String) -> Option<String> {
        None
    }

    /// An SRV resolver that finds nothing.
    async fn no_srv(_: String) -> Option<SrvTarget> {
        None
    }

    fn server_name(name: &str) -> OwnedServerName {
        ServerName::parse(name).unwrap()
    }

    #[test]
    fn test_the_current_srv_name_is_tried_first() {
        assert_eq!(
            srv_names("matrix.org"),
            ["_matrix-fed._tcp.matrix.org".to_owned(), "_matrix._tcp.matrix.org".to_owned(),]
        );
    }

    #[async_test]
    async fn test_step_1_an_ip_literal_is_connected_to_directly() {
        let resolved = resolve(&server_name("1.2.3.4:1234"), nothing, no_srv).await;

        assert_eq!(
            resolved,
            ResolvedServer {
                destination: FedDest::parse_literal("1.2.3.4:1234").unwrap(),
                host_header: "1.2.3.4:1234".to_owned(),
                tls_name: "1.2.3.4".to_owned(),
            }
        );
    }

    #[async_test]
    async fn test_step_2_an_explicit_port_is_used_as_given() {
        let resolved = resolve(&server_name("matrix.org:1234"), nothing, no_srv).await;

        assert_eq!(
            resolved,
            ResolvedServer {
                destination: FedDest::Named("matrix.org".to_owned(), 1234),
                host_header: "matrix.org:1234".to_owned(),
                tls_name: "matrix.org".to_owned(),
            }
        );
    }

    #[async_test]
    async fn test_step_2_does_not_consult_delegation() {
        // A server name with a port is resolved as written, so a delegation
        // document must not be fetched for it.
        let resolved = resolve(
            &server_name("matrix.org:1234"),
            |_| async { panic!("well-known must not be fetched") },
            no_srv,
        )
        .await;

        assert_eq!(resolved.destination, FedDest::Named("matrix.org".to_owned(), 1234));
    }

    #[async_test]
    async fn test_step_3_2_delegation_to_a_host_and_port() {
        let resolved = resolve(
            &server_name("matrix.org"),
            |_| async { Some("matrix.matrix.org:443".to_owned()) },
            no_srv,
        )
        .await;

        assert_eq!(
            resolved,
            ResolvedServer {
                destination: FedDest::Named("matrix.matrix.org".to_owned(), 443),
                host_header: "matrix.matrix.org:443".to_owned(),
                tls_name: "matrix.matrix.org".to_owned(),
            }
        );
    }

    #[async_test]
    async fn test_step_3_3_a_delegated_host_with_an_srv_record() {
        let resolved = resolve(
            &server_name("matrix.org"),
            |_| async { Some("matrix.matrix.org".to_owned()) },
            |host| async move {
                assert_eq!(host, "matrix.matrix.org");
                Some(SrvTarget::new("backend.example.com", 8449))
            },
        )
        .await;

        // The request connects to the SRV target, but is addressed to the
        // delegated name in both the Host header and TLS.
        assert_eq!(
            resolved,
            ResolvedServer {
                destination: FedDest::Named("backend.example.com".to_owned(), 8449),
                host_header: "matrix.matrix.org".to_owned(),
                tls_name: "matrix.matrix.org".to_owned(),
            }
        );
    }

    #[async_test]
    async fn test_step_3_4_a_delegated_host_without_an_srv_record() {
        let resolved = resolve(
            &server_name("matrix.org"),
            |_| async { Some("matrix.matrix.org".to_owned()) },
            no_srv,
        )
        .await;

        assert_eq!(
            resolved,
            ResolvedServer {
                destination: FedDest::Named("matrix.matrix.org".to_owned(), DEFAULT_PORT),
                host_header: "matrix.matrix.org".to_owned(),
                tls_name: "matrix.matrix.org".to_owned(),
            }
        );
    }

    #[async_test]
    async fn test_step_4_an_srv_record_for_the_server_name() {
        let resolved = resolve(&server_name("matrix.org"), nothing, |host| async move {
            assert_eq!(host, "matrix.org");
            Some(SrvTarget::new("backend.example.com", 8449))
        })
        .await;

        assert_eq!(
            resolved,
            ResolvedServer {
                destination: FedDest::Named("backend.example.com".to_owned(), 8449),
                host_header: "matrix.org".to_owned(),
                tls_name: "matrix.org".to_owned(),
            }
        );
    }

    #[async_test]
    async fn test_step_5_the_server_name_on_the_default_port() {
        let resolved = resolve(&server_name("matrix.org"), nothing, no_srv).await;

        assert_eq!(
            resolved,
            ResolvedServer {
                destination: FedDest::Named("matrix.org".to_owned(), DEFAULT_PORT),
                host_header: "matrix.org".to_owned(),
                tls_name: "matrix.org".to_owned(),
            }
        );
    }

    #[async_test]
    async fn test_a_delegated_name_does_not_delegate_again() {
        let fetches = AtomicUsize::new(0);
        let resolved = resolve(
            &server_name("matrix.org"),
            |name| {
                fetches.fetch_add(1, Ordering::SeqCst);
                async move {
                    assert_eq!(name, "matrix.org");
                    Some("matrix.matrix.org".to_owned())
                }
            },
            no_srv,
        )
        .await;

        assert_eq!(fetches.load(Ordering::SeqCst), 1);
        assert_eq!(resolved.host_header, "matrix.matrix.org");
    }
}
