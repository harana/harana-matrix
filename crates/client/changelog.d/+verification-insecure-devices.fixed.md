In-room verification is no longer blocked by
`CollectStrategy::ErrorOnVerifiedUserProblem`. That strategy refuses to share a
room key when a verified user has a device their own identity hasn't signed,
which in an encrypted room also blocked the `m.key.verification.*` events, so
the one thing that resolves the situation could never be started. Sharing the
room key for a verification event now retries with
`CollectStrategy::AllDevices`, and the outbound session is discarded on both
sides of that retry so widening who receives a room key doesn't widen who can
read the rest of the room.
