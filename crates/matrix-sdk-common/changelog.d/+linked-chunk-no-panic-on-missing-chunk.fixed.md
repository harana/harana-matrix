`LinkedChunk::items` and `LinkedChunk::ritems` no longer resolve the starting
chunk by identifier, which walks the chunk chain backwards from the last chunk
and panicked with "cannot fail because at least one empty chunk must exist"
when the ends bookkeeping was inconsistent. They now iterate the chunk chain
directly, which cannot fail.
