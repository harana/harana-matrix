-- Values stored under an arbitrary key, for data that is shared between the
-- processes opening this store and is not tied to a room.
CREATE TABLE "key_value" (
    "key" BLOB PRIMARY KEY NOT NULL,
    "value" BLOB NOT NULL
);
