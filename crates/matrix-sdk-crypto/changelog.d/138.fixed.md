The check that decides whether a received room key is better than the one we
already have is now atomic with the write that acts on it. `import_room_keys`
and the sync key handler could both find nothing stored, both decide to write,
and leave the worse key stored with the comparison bypassed.
