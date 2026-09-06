Sharing room history on invite now sends an `m.no_olm` withheld notice to the
invitee's devices we could not establish an Olm session with, instead of
silently leaving them out. Without it the invitee is left with a room whose
history does not decrypt on that device and no reason given, which is
indistinguishable from history never having been shared.
`RoomKeyWithheldContent::no_olm` builds that content without the room and
session IDs an `m.no_olm` does not carry.
