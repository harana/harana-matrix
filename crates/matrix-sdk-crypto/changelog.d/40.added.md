`CrossSigningStatus` now reports `is_published`, saying whether the identity's
public keys actually reached the homeserver, and `is_usable()` combines it with
`is_complete()`. Nothing retries a failed identity upload, so a client used to
believe it was cross-signed while everyone else still saw its device as
unverified, with no way to tell.
