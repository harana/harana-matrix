A sync that brings back events the event cache already knows about no longer
removes them and pushes them back at the end when they were already there, in
that order. Observers used to see a remove/insert pair for an item that never
moved, which made a just-sent message visibly bounce in the timeline: the send
queue puts the event in the event cache, then the sync copy took it out and put
it back. Such events now only produce a `VectorDiff::Set`.
