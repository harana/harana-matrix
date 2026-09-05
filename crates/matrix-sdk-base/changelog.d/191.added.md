Restoring a session that has synced before with a crypto store that holds no
account for it is now reported as an error. This is what happens when the crypto
store isn't persistent or isn't the one the session was created with, and it
used to be silent, showing up only as a device key upload that never succeeded.
