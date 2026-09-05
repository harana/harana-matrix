The notification client's sliding sync request no longer uses the `$ME` magic
state key, which was removed from MSC4186; the current user's full user ID is
sent instead.
