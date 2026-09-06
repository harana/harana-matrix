# `harana_matrix_server::store_codec`

A compact binary codec for the keys and values of a key-value store, ported
from [tuwunel]'s database layer.

A store backend keyed by tuples — `(user_id, room_id)`, `(room_id, event_id)`,
`(user_id, room_id, "m.receipt")` — needs an encoding where those tuples sort
the way the code expects and where a prefix of a key is itself a valid prefix to
scan by. This crate is that encoding, as a Serde format:

- Tuples and sequences write [`SEP`] between adjacent elements, a byte that
  cannot occur inside encoded UTF-8, so a decoder can split records without
  escaping and a prefix scan cannot match a partial component.
- Integers are written big-endian, so their byte order is their numeric order.
- [`Interfix`] as a tuple's last element finalizes a prefix *including* its
  trailing separator, which is what a range scan over `(user_id, room_id, ..)`
  needs.
- [`Separator`] writes one separator where a layout needs it outside a container
  boundary.
- [`Json`] wraps a value whose shape the compact rules cannot express — a map, a
  `serde_json::Value`, anything self-describing — and delegates it to
  `serde_json`.

A trailing element that accepts empty input decodes from an older, shorter
record, so a tuple can gain a field without rewriting what is already stored.

Two limits are worth knowing before designing a key around this format. A signed
integer is written as-is, so its sign bit puts every negative value above every
positive one: a key whose order matters wants an unsigned value, offset if it
must represent negatives. And a bare string cannot be encoded at the top
level, only inside a tuple, since a caller holding nothing but a string has
nothing for the codec to do with it.

The format covers what a store's keys and values need and nothing more: a value
whose shape it cannot represent panics rather than encoding something that will
not decode. Tuwunel's CBOR wrapper is not ported, so a payload needing a
self-describing format uses `Json`.

[tuwunel]: https://github.com/matrix-construct/tuwunel
