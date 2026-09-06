An incoming verification request from a device we don't know about yet is no
longer dropped. Such a request commonly arrives in the same sync response as (or
before) the device list update announcing the sending device, whose keys are only
downloaded afterwards; this is frequent after the app has been backgrounded. The
request is now kept and handled once the `/keys/query` response lands, or
discarded when it expires.
