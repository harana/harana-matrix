`OlmMachine::retry_decryption_of_bundled_events` retries the bundled
aggregations - an edit, or a thread's latest event - of an event we have already
decrypted. A bundled aggregation is a separate encrypted event, so it can be a
UTD while the event carrying it is not, and nothing about the outer event
changes when its room key finally arrives.
