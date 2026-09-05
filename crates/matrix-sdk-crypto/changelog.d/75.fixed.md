A secret is no longer shared twice with a device that asked for it more than
once while we had no Olm session with it. Requests still waiting for a session
are dropped once the secret has been sent, instead of being serviced again as
soon as a key claim establishes the session.
