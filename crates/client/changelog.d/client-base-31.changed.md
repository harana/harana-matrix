`StateChanges::add_redaction` no longer takes the redacted event's ID: it reads
it off the redaction itself, given the room version's `RedactionRules`, which
say whether it sits at the top level of the event or in its content. Callers
had to extract and pass back data the `Raw` argument already carried.
