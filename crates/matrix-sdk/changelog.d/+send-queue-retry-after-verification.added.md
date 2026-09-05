`SendQueue::retry_requests_blocked_on_verification()` puts back in the queue the
requests that failed because the room contained insecure devices, because a
previously verified user changed identity, or because our own session wasn't
verified, once that is no longer the case. Such a request used to stay wedged
forever: verifying is what the user is expected to do about it, but nothing
looked at those requests again afterwards. It runs on its own whenever a
`/keys/query` response updates what we know about device keys.
