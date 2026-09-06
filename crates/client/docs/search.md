# `client-search`

This crate implements a searchable index for messages in a Matrix client.

## Usage

The recommended way to use this crate is to include `matrix` as a dependency
in your `Cargo.toml` file with the `experimental-search` feature flag turned on.

## Stand-alone usage

Constructing a `client_search::index::RoomIndex` is done with the
`client_search::index::builder::RoomIndexBuilder`.

```rust
use std::path::PathBuf;
use client_search::{
    error::IndexError,
    index::{
        RoomIndex,
        builder::RoomIndexBuilder,
    },
};
use harana_matrix_common::RoomId;

fn create_index(path: PathBuf, room_id: &RoomId) -> Result<RoomIndex, IndexError> {
    RoomIndexBuilder::new_on_disk(path, room_id).unencrypted().build()
}
```

The search crate accepts index operations through
`client_search::index::RoomIndex::execute()` which takes a
`client_search::index::RoomIndexOperation`.

```rust
use client_search::index::{
    IndexableEvent, RoomIndex, RoomIndexOperation,
    builder::RoomIndexBuilder
};

async fn add_event(index: &mut RoomIndex, event: IndexableEvent) {
    index.execute(RoomIndexOperation::Add(event));
}
```

Some method(s) for creating these operations will need to be implemented. There
is an example of handling `harana_matrix_common::events::AnySyncTimelineEvents` in
`harana_matrix_client::search_index`.
