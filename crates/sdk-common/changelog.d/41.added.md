`UnableToDecryptReason::Redacted` distinguishes an event that was redacted
before we could decrypt it from a genuine decryption failure, and
`UnableToDecryptReason::is_expected` reports whether a reason should be counted
towards UTD rates at all.
