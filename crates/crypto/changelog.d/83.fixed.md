An `m.key.verification.ready` from a device we don't know about yet is no longer
dropped. Accepting a verification and immediately calling `start_sas()` or
`generate_qr_code()` used to race with the in-flight `/keys/query` that
announces the accepting device, leaving the request stuck in the `Created`
state. The acceptance is now kept and replayed once the device list catches up,
using the same mechanism as for incoming verification requests.
