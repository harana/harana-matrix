`BaseClient::receive_sent_state_event()` and
`receive_sent_room_account_data()` apply an event we have just sent
successfully, the way the sync would, so a successful send is immediately
followed by a store that knows about it instead of the previous value being read
back until the sync echoes the change.

[**breaking**] `BaseClient::share_room_key()` takes an optional `CollectStrategy`
that replaces the client's configured one for that sharing only, for the rare
events that must reach devices the configured strategy would refuse to share
with.
