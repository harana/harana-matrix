Add `Client::create_space`, which creates a room whose type is `m.space` and
raises `events_default` to 100 so ordinary members cannot post into the space
room itself. The specification leaves that to whoever creates the space, so
every client had to arrange it. The request is only filled in where the caller
left it empty: an explicit room type or power level override is used verbatim.
