A `/members` response no longer overwrites member events that a sync wrote
while the request was in flight. `Room::start_members_request` marks a request
as in flight, syncs record the member events they write for as long as one is,
and `BaseClient::receive_all_members` leaves those users alone instead of
taking their membership backwards.
