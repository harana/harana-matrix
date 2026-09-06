The long-lived background tasks of the latest-events subsystem (the update
listener and the latest-event computation) and of encryption (room key backup
upload, room key backup download, and the historic room key bundle receiver)
are now spawned through the client's task monitor. A panic or an unexpected
early exit in any of them is reported as a background task failure instead of
going unnoticed. Tasks that were aborted when their owner was dropped keep
that behaviour, and are now also aborted on WebAssembly targets, where the
previous manual abort was compiled out.
