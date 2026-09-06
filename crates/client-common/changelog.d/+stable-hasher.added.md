Added `client_common::stable_hasher`, a `Hasher` whose digest is identical on
every platform: integers are hashed little-endian and `usize`/`isize` are
widened to 64 bits, on top of xxh3-64 with the default seed. This makes hashes
safe to persist and to compare across 32-bit (wasm) and 64-bit targets.
