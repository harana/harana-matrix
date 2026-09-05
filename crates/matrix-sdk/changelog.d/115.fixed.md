`Client::create_room` no longer retries the `/createRoom` request. That
endpoint has no transaction ID, so it is not idempotent: when the server
answered with a 5xx error, for instance because it failed to invite a user that
doesn't exist, the default retry policy sent the request again and created a
room on every attempt. The error is now returned to the caller instead.
