`OlmMachine::outgoing_requests()` no longer hands out a second `/keys/upload`
request carrying the same one-time keys under a new ID when it is polled again
before the first is marked as sent. Sending both made the homeserver reject the
second with a 400 for a key it already had, and the collision reproduced on
every retry, blocking cross-signing bootstrap.
