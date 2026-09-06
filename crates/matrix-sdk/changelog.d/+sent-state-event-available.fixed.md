A state event is readable as soon as the send that wrote it returns. Sending
only gave back the event ID the server had assigned to it, and the event itself
reached the store when the sync echoed it back, so reading the state just
written gave the previous value until then, and every consumer had to work
around that gap on its own. Message-like events already had this guarantee: the
send queue puts them in the event cache as soon as the server accepts them.
