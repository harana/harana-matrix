A data migration framework for the crypto store, in `store::migrations`. A
migration of the store's *contents* - recomputing a field on stored sessions,
say - has nothing to do with SQLite or IndexedDB, but had to be written once per
backend alongside their schema migrations. Migrations here run above the
backends against the ordinary `Store` API, so they are written once, and the
store remembers how far it has got. The ad-hoc verified-latch migration, which
carried a `FIXME` asking for exactly this, is now the framework's first
migration; stores that already ran it under the old flag are recognised as
already migrated.
