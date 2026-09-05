An `m.room.encrypted` event that was redacted before we saw it is no longer
reported as an unsupported-algorithm decryption failure. A redaction strips the
`algorithm` field, which made these events indistinguishable from genuinely
undecryptable ones and inflated UTD rates. They now fail with
`MegolmError::RedactedEvent`, mapped to `UnableToDecryptReason::Redacted`, and
are left out of UTD reports.
