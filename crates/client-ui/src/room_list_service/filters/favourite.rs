// Copyright 2024 The Matrix.org Foundation C.I.C.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::{super::RoomListItem, Filter};

fn matches(is_favourite: fn(&RoomListItem) -> bool, room: &RoomListItem) -> bool {
    is_favourite(room)
}

/// Create a new filter that will filter out rooms that are not marked as
/// favourite (see [`client_base::Room::is_favourite`]).
pub fn new_filter() -> impl Filter {
    |room| -> bool { matches(|room: &RoomListItem| room.is_favourite(), room) }
}

#[cfg(test)]
mod tests {
    use std::ops::Not;

    use client_matrix::test_utils::mocks::MatrixMockServer;
    use common_test::async_test;
    use harana_matrix_common::room_id;

    use super::{super::new_rooms, *};

    #[async_test]
    async fn test_is_favourite() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        let [room] = new_rooms([room_id!("!a:b.c")], &client, &server).await;

        assert!(matches(|_: &RoomListItem| true, &room));
    }

    #[async_test]
    async fn test_is_not_favourite() {
        let server = MatrixMockServer::new().await;
        let client = server.client_builder().build().await;
        let [room] = new_rooms([room_id!("!a:b.c")], &client, &server).await;

        assert!(matches(|_: &RoomListItem| false, &room).not());
    }
}
