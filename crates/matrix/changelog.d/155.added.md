Add `Client::get_room_preview_local_first`, which answers the preview of a room
the client already knows from local data, without waiting on the server, and
refreshes it from the server in the background. `Client::get_room_preview` is
unchanged, and now also saves what the server answered onto the local room, so
a later local preview is closer to the truth.
