`CryptoStore::clear_received_room_key_bundle_data` forgets the room key bundle
data received from a user for a room, implemented for the memory, SQLite and
IndexedDB backends. A bundle names an encrypted file on the media repository and
carries the key to decrypt it, and was kept for the lifetime of the account.
