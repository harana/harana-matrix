Added `StoreCipherBackend` and `StoreCipherProvider`, which let a store be
encrypted with something other than the built-in `StoreCipher`: an OS keychain,
a Secure Enclave, an HSM or a KMS. `StoreCipher` implements
`StoreCipherBackend` and stays the default.

Added `StoreCodec`, which describes the serialization format a store writes to
disk, along with the `MessagePackCodec` and `JsonCodec` implementations the
SDK's stores have always used.

Added `CodecKind`, which a codec reports to say it is byte-identical to one of
the built-in formats. `StoreCodecExt` then serializes through that format's
crate directly rather than through `erased_serde`, whose dynamic dispatch costs
a virtual call per serde operation rather than per value. The two built-in
codecs report their kind, so the default store configuration pays nothing for
the codec being pluggable.
