The trust shield on already received messages now follows the sender's
verification state. A sender's state can improve without any of their devices
changing - we cross-sign them, or a verification violation is withdrawn - and
only a device change used to trigger a recompute, so messages received before
the change kept the shield they were decrypted with while messages received a
moment later got a better one. `/keys/query` responses carrying an identity, and
`withdraw_verification()` on either kind of identity, now recompute the
`SenderData` of that user's sessions and notify listeners.
