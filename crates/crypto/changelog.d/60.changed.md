`VerificationRequest` keeps the other party's device data once a verification
is done, so a client can name the device that was verified in a completion
dialog. `VerificationRequestState::Done` now carries `other_device_data`
(`None` when another of our own devices answered the request), a new
`VerificationRequest::other_device_data` accessor returns it, and
`VerificationRequest::other_device_id` no longer returns `None` in the done
state.
