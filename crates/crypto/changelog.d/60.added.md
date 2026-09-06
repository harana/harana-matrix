`VerificationRequest::other_device_data` returns the device that took part in a
verification, and `VerificationRequestState::Done` now carries it too. The
information used to be dropped when the request reached the done state, so a
client could not name the device it had just verified.
