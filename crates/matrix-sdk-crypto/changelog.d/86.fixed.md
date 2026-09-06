Olm sessions with a device are now expired instead of accumulating forever.
Every unwedging, every message from a device we had no session with, and every
one-time key claim that crossed with the peer's left another session behind,
and nothing ever removed one. The eight most recently used sessions per sender
key are kept, which is twice what the spec recommends, so a session the peer
may still be encrypting to is not dropped.
