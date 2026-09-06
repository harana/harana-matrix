Ephemeral events are now deserialized exactly once per sync, at the top of the
event cache's update pipeline, and the parsed read receipts are passed to the
room cache and every thread cache. Previously the same raw events were parsed
again for each cache, and a second time by the thread aggregator.
