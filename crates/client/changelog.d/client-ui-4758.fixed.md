Added a regression test covering that subscribing to a timeline that already
holds items delivers those items only in the initial snapshot, and never again
as a diff on the update stream.
