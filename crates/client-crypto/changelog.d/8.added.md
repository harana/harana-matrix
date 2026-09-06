Room event decryption now detects replayed events: an event which reuses the
Megolm message index of an event we have already decrypted is refused with
`MegolmError::ReplayedMessageIndex` instead of being shown as a new message.
Decrypting the same event again, which happens routinely, is unaffected.
