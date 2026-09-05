Saving an inbound group session no longer overwrites its `backed_up` column. The
column is the source of truth for whether a session has been uploaded to the
backup, and writing it back from a pickle read before a backup reset could
re-mark a session as backed up when it was not, so it was never uploaded and
future devices could not decrypt its messages. Importing sessions that came from
a backup now marks them explicitly instead.
