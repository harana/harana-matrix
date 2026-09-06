Room encryption settings are now cached in memory rather than read and
deserialized from the crypto store on every call. `getRoomSettings` sits on the
critical path for showing a room's encryption state and was taking seconds
rather than milliseconds.
