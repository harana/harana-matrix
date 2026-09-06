A device we could not establish an Olm session with when a room key was shared
now gets the session from the point at which that happened, rather than from
wherever the ratchet has reached by the time it asks. The withheld notice we
send it records the session's message index, and a later `m.room_key_request`
from that device is answered from there; previously it was refused outright,
because the records showed the session had never been shared with it, so every
message sent between the failure and the recovery stayed undecryptable for it.
