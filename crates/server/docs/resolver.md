# `harana_matrix_server::resolver`

Resolution of a Matrix server name to the host, port, `Host` header and TLS
name a federation request uses, following the [server discovery] ladder of the
server-server specification.

The ladder is pure logic over two lookups, and this crate is the logic only:
[`resolve`] takes an async `.well-known` fetcher and an async SRV resolver and
returns a [`ResolvedServer`]. Which HTTP client and which DNS resolver perform
those lookups is the caller's choice, so nothing here dictates a runtime, a TLS
stack or a DNS implementation.

What the crate does supply is everything that is easy to get wrong:

- [`well_known_url`] and [`parse_well_known`] for the delegation document,
  along with [`WELL_KNOWN_MAX_BYTES`], the cap a fetcher should read to.
- [`srv_names`], the two SRV names to try in order — `_matrix-fed._tcp` first,
  then the deprecated `_matrix._tcp`.
- [`FedDest`], which distinguishes an IP literal from a named host and carries
  the port the ladder settled on.

A resolved server separates the three names that a naive implementation
conflates: where to connect (which an SRV record redirects), what to send as
`Host` (which delegation changes but SRV does not), and which name TLS is
validated against.

Ported from [tuwunel]'s `src/service/resolver`, without its DNS resolver,
destination cache, and IP-range denylist.

[server discovery]: https://spec.matrix.org/latest/server-server-api/#resolving-server-names
[tuwunel]: https://github.com/matrix-construct/tuwunel
