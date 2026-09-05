`OlmMachine::receive_keys_query()` feeds a `/keys/query` response the machine
didn't ask for into the store, for users it isn't tracking. Callers used to have
to invent a request ID and go through `mark_request_as_sent()`, which could race
the machine's own device list tracking; users the machine already tracks are
skipped.
