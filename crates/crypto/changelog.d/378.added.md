`BackupMachine::encrypt_room_keys_for_room` and
`BackupMachine::encrypt_room_key` encrypt the keys of one room, or one session,
for upload to the active backup, so a caller can use the targeted
`/room_keys/keys/{roomId}` endpoints instead of the bulk one.
