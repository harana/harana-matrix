Added `client_common::bloom_filter`, a Scalable Bloom Filter implementation
vendored from the `growable-bloom-filter` crate, which is no longer a
dependency. `GrowableBloom` and `GrowableBloomBuilder` keep the same API and the
same serialized representation, so filters persisted by earlier versions still
deserialize.
