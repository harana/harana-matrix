// Copyright 2024 The Matrix.org Foundation C.I.C.
// Copyright 2024 Hanadi Tamimi
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

//! High-level pusher API.

use harana_matrix_common::{
    api::client::push::{PusherIds, set_pusher},
    push::HttpPusherData,
};

use crate::{Client, Result};

/// The name of the field carrying the [MSC4076] `disable_badge_count` flag in
/// the data of an HTTP pusher.
///
/// A client that computes its own badge counts can set this flag so the
/// homeserver stops sending high-priority pushes whose only purpose is to
/// update the unread count.
///
/// [MSC4076]: https://github.com/matrix-org/matrix-spec-proposals/pull/4076
pub const DISABLE_BADGE_COUNT_FIELD: &str = "org.matrix.msc4076.disable_badge_count";

/// Sets or clears the [MSC4076] `disable_badge_count` flag on the data of an
/// HTTP pusher.
///
/// The field is only written when `disable_badge_count` is `true`, since a
/// homeserver that doesn't know about the flag treats its absence and `false`
/// the same way.
///
/// [MSC4076]: https://github.com/matrix-org/matrix-spec-proposals/pull/4076
pub fn set_disable_badge_count(data: &mut HttpPusherData, disable_badge_count: bool) {
    if disable_badge_count {
        data.data.insert(DISABLE_BADGE_COUNT_FIELD.to_owned(), true.into());
    } else {
        data.data.remove(DISABLE_BADGE_COUNT_FIELD);
    }
}

/// A high-level API to interact with the pusher API.
///
/// All the methods in this struct send a request to the homeserver.
#[derive(Debug, Clone)]
pub struct Pusher {
    /// The underlying HTTP client.
    client: Client,
}

impl Pusher {
    pub(crate) fn new(client: Client) -> Self {
        Self { client }
    }

    /// Sets a given pusher.
    pub async fn set(
        &self,
        pusher: harana_matrix_common::api::client::push::Pusher,
        append: bool,
    ) -> Result<()> {
        let mut request = set_pusher::v3::Request::post(pusher);
        if let set_pusher::v3::PusherAction::Post(data) = &mut request.action {
            data.append = append;
        }
        self.client.send(request).await?;
        Ok(())
    }

    /// Deletes a pusher by its ids
    pub async fn delete(&self, pusher_ids: PusherIds) -> Result<()> {
        let request = set_pusher::v3::Request::delete(pusher_ids);
        self.client.send(request).await?;
        Ok(())
    }
}

// The http mocking library is not supported for wasm32
#[cfg(all(test, not(target_family = "wasm")))]
mod tests {
    use harana_matrix_common::{
        api::client::push::{PusherIds, PusherInit, PusherKind},
        push::HttpPusherData,
    };
    use serde_json::json;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{body_partial_json, method, path},
    };

    use super::{DISABLE_BADGE_COUNT_FIELD, set_disable_badge_count};
    use crate::{
        test::{async_test, test_json},
        test_utils::logged_in_client,
    };

    async fn mock_api(server: MockServer) {
        Mock::given(method("POST"))
            .and(path("_matrix/client/r0/pushers/set"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::EMPTY))
            .mount(&server)
            .await;
    }

    fn dummy_pusher() -> PusherInit {
        PusherInit {
            ids: PusherIds::new("pushKey".to_owned(), "app_id".to_owned()),
            app_display_name: "name".to_owned(),
            kind: PusherKind::Http(HttpPusherData::new("dummy".to_owned())),
            lang: "EN".to_owned(),
            device_display_name: "name".to_owned(),
            profile_tag: None,
        }
    }

    #[async_test]
    async fn test_set_pusher() {
        let server = MockServer::start().await;
        let client = logged_in_client(Some(server.uri())).await;
        mock_api(server).await;

        let response = client.pusher().set(dummy_pusher().into(), false).await;

        assert!(response.is_ok());
    }

    #[async_test]
    async fn test_set_pusher_forwards_append_flag() {
        let server = MockServer::start().await;
        let client = logged_in_client(Some(server.uri())).await;

        Mock::given(method("POST"))
            .and(path("_matrix/client/r0/pushers/set"))
            .and(body_partial_json(json!({ "append": true })))
            .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::EMPTY))
            .expect(1)
            .mount(&server)
            .await;

        let response = client.pusher().set(dummy_pusher().into(), true).await;

        assert!(response.is_ok());
    }

    #[async_test]
    async fn test_set_pusher_forwards_disable_badge_count_flag() {
        let server = MockServer::start().await;
        let client = logged_in_client(Some(server.uri())).await;

        Mock::given(method("POST"))
            .and(path("_matrix/client/r0/pushers/set"))
            .and(body_partial_json(
                json!({ "data": { "org.matrix.msc4076.disable_badge_count": true } }),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::EMPTY))
            .expect(1)
            .mount(&server)
            .await;

        let mut pusher = dummy_pusher();
        let PusherKind::Http(data) = &mut pusher.kind else { panic!("expected an HTTP pusher") };
        set_disable_badge_count(data, true);

        let response = client.pusher().set(pusher.into(), false).await;

        assert!(response.is_ok());
    }

    #[test]
    fn test_set_disable_badge_count_removes_the_field_when_disabled() {
        let mut data = HttpPusherData::new("dummy".to_owned());

        set_disable_badge_count(&mut data, true);
        assert_eq!(data.data.get(DISABLE_BADGE_COUNT_FIELD), Some(&json!(true)));

        set_disable_badge_count(&mut data, false);
        assert!(data.data.get(DISABLE_BADGE_COUNT_FIELD).is_none());
    }

    #[async_test]
    async fn test_delete_pusher() {
        let server = MockServer::start().await;
        let client = logged_in_client(Some(server.uri())).await;
        mock_api(server).await;

        // prepare pusher ids
        let pusher_ids = PusherIds::new("pushKey".to_owned(), "app_id".to_owned());

        let response = client.pusher().delete(pusher_ids).await;

        assert!(response.is_ok());
    }
}
