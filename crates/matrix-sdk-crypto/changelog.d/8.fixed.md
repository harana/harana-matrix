Room events are now checked against a replay of the Megolm ratchet index they
decrypt at. A ciphertext says nothing about the event it arrived in, so anyone
who can inject events into a room could take one the server had already
delivered and send it again under a new event ID or timestamp; it decrypted
cleanly and showed a second time, attributed to its original sender. Decrypting
the *same* event again, as happens when a timeline is rebuilt or a key arrives
late, is unaffected.
