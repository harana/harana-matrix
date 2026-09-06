Translating linked chunk updates into `VectorDiff`s no longer panics with "The
chunk is not found" when an update refers to a chunk the tracker does not know
about. This took down the whole application, for instance while
back-paginating a timeline. Such an update is now logged as an error and
skipped, so the process survives a bookkeeping inconsistency. This affects
both `AsVector` and the `OrderTracker`, which share the update mapper.
