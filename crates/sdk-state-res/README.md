# sdk-state-res

Asynchronous, store-backed adapters over [`ruma::state_res`], the Matrix state
resolution and event authorization algorithms.

Ruma implements both algorithms against *synchronous* lookup closures: a
`fetch_event(&EventId) -> Option<E>` and a
`fetch_state(&StateEventType, &str) -> Option<E>`. A consumer whose events live
in an async store (an SDK store, a database, the network) cannot answer those
closures without blocking, so it must know in advance which events the
algorithm will ask for.

This crate removes that requirement. Its entry points take async fetchers and
drive the synchronous algorithm underneath:

- [`auth_check`], [`check_state_independent_auth_rules`] and
  [`check_state_dependent_auth_rules`] authorize one event.
- [`resolve`] resolves conflicted state maps.

Each seeds a cache with the events the specification says the check needs, runs
the synchronous algorithm against that cache, and — if the algorithm asks for
something the cache does not hold — fetches the missing entries and runs it
again, up to [`MAX_FETCH_ROUNDS`] times. A key that was fetched and found absent
is cached as absent, so a genuinely missing power-levels or join-rules event
resolves in one round rather than looping.

Authorization returns an [`AuthCheckOutcome`] rather than a `Result`: a denial
is an ordinary outcome of a check that ran to completion, not a failure to run
it.

The async-fetch shape is taken from [tuwunel]'s fork of Ruma's state resolution;
the algorithms themselves are Ruma's, and are re-exported here unchanged.

[tuwunel]: https://github.com/matrix-construct/tuwunel
