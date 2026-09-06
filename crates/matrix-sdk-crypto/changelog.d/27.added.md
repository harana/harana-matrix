[MSC3879] (trusted key forwards). An `m.forwarded_room_key` now carries
`org.matrix.msc3879.trusted`, set when the forwarding device can tie the session
to its creator - it created the session, received it as an `m.room_key`, or
received it as a trusted forward itself. A session restored from a backup or a
file export is not vouched for.

On the receiving side, a forward marked trusted from another of our own devices
that we have verified - which the SDK already required before accepting a
forward at all - is attributed to the device the forwarder named, so a key we
asked our own device for is no longer shown as coming from nobody in particular.

[MSC3879]: https://github.com/matrix-org/matrix-spec-proposals/pull/3879
