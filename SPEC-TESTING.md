# Matrix spec conformance testing

This document is the map for the spec conformance test suite. The work itself
is tracked in GitHub, under the epic
[#423](https://github.com/harana/harana-matrix/issues/423): 10 area meta issues
and 158 leaf issues, carrying 3,733 requirement rows.

The goal is narrow and checkable: every normative statement in the Matrix
specification has at least one executing test in this workspace, on the client
side and on the server side.

## The corpus

Everything is measured against
[matrix-org/matrix-spec](https://github.com/matrix-org/matrix-spec) at commit
`0dfc691` (2026-08-26, changelog through v1.19). The pin matters: a spec bump
surfaces new requirements as uncovered rows rather than as silent drift.

| Part | Source | Size |
| --- | --- | --- |
| Client-Server API | `content/client-server-api/` | 166 operations, 45 module documents |
| Server-Server API | `content/server-server-api.md` | 36 operations |
| Identity Service API | `content/identity-service-api.md` | 23 operations |
| Application Service API | `content/application-service-api.md` | 9 operations |
| Push Gateway API | `content/push-gateway-api.md` | 1 operation |
| Room versions | `content/rooms/v1.md` to `v12.md` | 12 versions |
| Appendices | `content/appendices.md` | canonical JSON, signing, grammars |
| Event schemas | `data/event-schemas/` | 94 schema files |

Across `content/` and `data/` the spec uses 878 normative keywords: 372 MUST,
63 MUST NOT, 287 SHOULD, 32 SHOULD NOT, 122 MAY, 2 REQUIRED. That count is a
lower bound on requirements, because one sentence can carry several
obligations. The 3,733 requirement rows in the issues are the real
decomposition.

## Areas

| # | Area | Meta issue | Leaves | Requirement rows |
| --- | --- | --- | --- | --- |
| 1 | Harness, corpus tooling, coverage ledger, CI | [#424](https://github.com/harana/harana-matrix/issues/424) | 13 | tooling |
| 2 | Client-Server standards, discovery, authentication, capabilities | [#425](https://github.com/harana/harana-matrix/issues/425) | 16 | 522 |
| 3 | Client-Server events, syncing, rooms, membership | [#430](https://github.com/harana/harana-matrix/issues/430) | 18 | 450 |
| 4 | Client-Server modules: messaging, relations, room metadata | [#441](https://github.com/harana/harana-matrix/issues/441) | 18 | 415 |
| 5 | End-to-end encryption, device management, secrets | [#429](https://github.com/harana/harana-matrix/issues/429) | 18 | 443 |
| 6 | Push rules, notifications, Push Gateway API | [#432](https://github.com/harana/harana-matrix/issues/432) | 12 | 204 |
| 7 | Content repository, VoIP, ephemeral events | [#435](https://github.com/harana/harana-matrix/issues/435) | 13 | 262 |
| 8 | Server-Server (federation) API | [#439](https://github.com/harana/harana-matrix/issues/439) | 18 | 617 |
| 9 | Room versions, auth rules, state resolution, appendices | [#449](https://github.com/harana/harana-matrix/issues/449) | 18 | 462 |
| 10 | Application Service and Identity Service APIs | [#474](https://github.com/harana/harana-matrix/issues/474) | 14 | 358 |

## How a requirement becomes a test

1. The extractor walks `content/**/*.md` and `data/**/*.yaml` and emits a stable
   requirement ID per normative statement: spec file, anchor, keyword, sentence
   hash.
2. A test claims one or more requirement IDs through the `#[spec(...)]`
   attribute.
3. The ledger joins the two and reports three buckets: covered, uncovered, and
   parked.
4. CI fails when a requirement loses its last test, and reports the uncovered
   count with a trend.

Test kinds used across the issues:

| Kind | Meaning |
| --- | --- |
| `unit` | a pure function, called directly |
| `serde` | serialise and deserialise round trip |
| `schema` | validate against the OpenAPI definition or the event schema |
| `mock-http` | against a wiremock server standing in for a homeserver |
| `integration` | against a real homeserver, or two crypto machines talking |
| `property` | generated inputs with an invariant, usually grammars |
| `vector` | fixed inputs and outputs taken from the spec |
| `differential` | compared against another implementation |

## What is not testable yet

There is no homeserver in this workspace, only the `server-*` building blocks,
so 276 requirement rows start parked: 196 in federation, 80 in the application
service and identity service areas. Every parked row carries a reason and, where
one exists, what would unblock it. No requirement was dropped.

The conversion of parked rows tracks the growth of the `server-*` crates, under
[#482](https://github.com/harana/harana-matrix/issues/482).

## Sequencing

1. Area 1 first. The corpus pin, the requirement extractor, and the coverage
   ledger are what let the other areas count as coverage rather than as
   assertions.
2. Areas 9 and 5 next. Room version algorithms and cryptographic vectors are
   pure functions over fixed inputs, so they need no harness beyond fixtures.
3. Areas 2, 3, 4, 6, 7 and 10 follow the mock homeserver harness.
4. Area 8 runs in parallel for the 421 rows that are testable today against
   `common-federation-api`, `common-signatures` and `server-resolver`.
