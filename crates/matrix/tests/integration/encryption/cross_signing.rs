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

use assert_matches2::assert_let;
use matrix::{encryption::CrossSigningResetAuthType, test_utils::mocks::MatrixMockServer};
use ruma::api::{
    client::uiaa,
    error::{ErrorKind, StandardErrorBody},
};
use sdk_test::async_test;
use similar_asserts::assert_eq;

#[async_test]
async fn test_bootstrap_cross_signing_reports_what_it_did() {
    use matrix::encryption::CrossSigningBootstrapOutcome;

    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    server.mock_query_keys().ok().mount().await;
    server.mock_upload_keys().ok().mount().await;

    // Given a homeserver which wants user-interactive authentication for the keys
    let outcome = {
        let _guard =
            server.mock_upload_cross_signing_keys().uiaa().expect(1).mount_as_scoped().await;

        client
            .encryption()
            .bootstrap_cross_signing_if_needed_with_outcome(None)
            .await
            .expect("A homeserver asking for authentication is not a failure")
    };

    // Then we are told so, rather than getting an error
    assert_eq!(outcome, CrossSigningBootstrapOutcome::AuthenticationRequired);

    // When we answer the challenge and the homeserver accepts the keys
    server.mock_upload_cross_signing_keys().ok().expect(1).mount().await;
    server.mock_upload_cross_signing_signatures().ok().expect(1).mount().await;

    let user_id = client.user_id().expect("We should be able to access the user ID by now");
    let auth_data =
        uiaa::AuthData::Password(uiaa::Password::new(user_id.to_owned().into(), "1234".to_owned()));

    let outcome = client
        .encryption()
        .bootstrap_cross_signing_if_needed_with_outcome(Some(auth_data))
        .await
        .expect("We should be able to bootstrap cross-signing");

    // Then we are told that the identity was created
    assert_eq!(outcome, CrossSigningBootstrapOutcome::Created);
    assert!(client.encryption().cross_signing_status().await.unwrap().is_complete());
}

#[async_test]
async fn test_reset_legacy_auth() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let user_id = client.user_id().expect("We should be able to access the user ID by now");

    assert!(
        !client.encryption().cross_signing_status().await.unwrap().is_complete(),
        "Initially we shouldn't have any cross-signin keys",
    );

    server.mock_upload_keys().ok().mock_once().mount().await;

    let reset_handle = {
        let _guard =
            server.mock_upload_cross_signing_keys().uiaa().expect(1).mount_as_scoped().await;

        client
            .encryption()
            .reset_cross_signing()
            .await
            .unwrap()
            .expect("We should have received a reset handle")
    };

    server.mock_upload_cross_signing_keys().ok().expect(1).mount().await;
    server.mock_upload_cross_signing_signatures().ok().expect(1).mount().await;

    assert_let!(CrossSigningResetAuthType::Uiaa(uiaa_info) = reset_handle.auth_type());

    let mut password = uiaa::Password::new(user_id.to_owned().into(), "1234".to_owned());
    password.session = uiaa_info.session.clone();
    reset_handle
        .auth(Some(uiaa::AuthData::Password(password)))
        .await
        .expect("We should be able to reset the cross-signing keys using the reset handle");

    assert!(
        client.encryption().cross_signing_status().await.unwrap().is_complete(),
        "After the reset we have the cross-signing available.",
    );
}

#[async_test]
async fn test_reset_legacy_auth_invalid_password() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;
    let user_id = client.user_id().expect("We should be able to access the user ID by now");

    assert!(
        !client.encryption().cross_signing_status().await.unwrap().is_complete(),
        "Initially we shouldn't have any cross-signin keys",
    );

    server.mock_upload_keys().ok().mock_once().mount().await;

    let reset_handle = {
        let _guard =
            server.mock_upload_cross_signing_keys().uiaa().expect(1).mount_as_scoped().await;

        client
            .encryption()
            .reset_cross_signing()
            .await
            .unwrap()
            .expect("We should have received a reset handle")
    };

    server.mock_upload_cross_signing_keys().uiaa_invalid_password().expect(1).mount().await;

    assert_let!(CrossSigningResetAuthType::Uiaa(uiaa_info) = reset_handle.auth_type());

    let mut password = uiaa::Password::new(user_id.to_owned().into(), "wrong-password".to_owned());
    password.session = uiaa_info.session.clone();
    reset_handle
        .auth(Some(uiaa::AuthData::Password(password)))
        .await
        .expect_err("Resetting with the wrong password should return the error");
}

#[async_test]
async fn test_reset_unstable_oauth() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().logged_in_with_oauth().build().await;

    assert!(
        !client.encryption().cross_signing_status().await.unwrap().is_complete(),
        "Initially we shouldn't have any cross-signing keys",
    );

    server.mock_upload_keys().ok().expect(1).named("Initial device keys upload").mount().await;

    // Return the UIAA response 5 times.
    server
        .mock_upload_cross_signing_keys()
        .uiaa_unstable_oauth()
        .up_to_n_times(5)
        .expect(5)
        .named("Trying to upload the cross-signing keys with UIAA response")
        .mount()
        .await;

    // And finally succeed.
    // This works because the first mocked endpoint that matches the path is used
    // until it is invalidated by `up_to_n_times`.
    server
        .mock_upload_cross_signing_keys()
        .ok()
        .expect(1)
        .named("Succeeding to upload the cross-signing keys")
        .mount()
        .await;

    server
        .mock_upload_cross_signing_signatures()
        .ok()
        .expect(1)
        .named("Final signatures upload")
        .mount()
        .await;

    // First requests gives us a reset handle.
    let reset_handle = client
        .encryption()
        .reset_cross_signing()
        .await
        .unwrap()
        .expect("We should have received a reset handle");

    assert_let!(CrossSigningResetAuthType::OAuth(oauth_info) = reset_handle.auth_type());
    assert_eq!(
        oauth_info.approval_url.as_str(),
        format!("{}/account/?action=org.matrix.cross_signing_reset", server.uri())
    );

    // Then it retries until it succeeds.
    reset_handle.auth(None).await.expect("We should be able to reset the cross-signing keys after some attempts, waiting for the auth issue to allow us to upload");

    assert!(
        client.encryption().cross_signing_status().await.unwrap().is_complete(),
        "After the reset we have the cross-signing available.",
    );
}

#[async_test]
async fn test_reset_stable_oauth() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().logged_in_with_oauth().build().await;

    assert!(
        !client.encryption().cross_signing_status().await.unwrap().is_complete(),
        "Initially we shouldn't have any cross-signing keys",
    );

    let session = "oauth_session";
    let mut expected_oauth = uiaa::OAuth::new();
    expected_oauth.session = Some(session.to_owned());
    let expected_auth_data = uiaa::AuthData::OAuth(expected_oauth);

    server.mock_upload_keys().ok().expect(1).named("Initial device keys upload").mount().await;

    // First, return the UIAA response without expecting the UIAA auth data in the
    // request.
    server
        .mock_upload_cross_signing_keys()
        .uiaa_stable_oauth(session, None)
        .mock_once()
        .named("Trying to upload the cross-signing keys with UIAA response without auth data")
        .mount()
        .await;

    // Then return the UIAA response 5 times while expecting the UIAA auth data in
    // the request.
    let extra_error =
        StandardErrorBody::new(ErrorKind::Forbidden, "Stage not completed".to_owned());
    server
        .mock_upload_cross_signing_keys()
        .expect_uiaa_auth_data(&expected_auth_data)
        .uiaa_stable_oauth(session, Some(&extra_error))
        .up_to_n_times(5)
        .expect(5)
        .named("Trying to upload the cross-signing keys with UIAA response and auth data")
        .mount()
        .await;

    // And finally succeed.
    // This works because the first mocked endpoint that matches the path is used
    // until it is invalidated by `up_to_n_times`.
    server
        .mock_upload_cross_signing_keys()
        .expect_uiaa_auth_data(&expected_auth_data)
        .ok()
        .expect(1)
        .named("Succeeding to upload the cross-signing keys")
        .mount()
        .await;

    server
        .mock_upload_cross_signing_signatures()
        .ok()
        .expect(1)
        .named("Final signatures upload")
        .mount()
        .await;

    // First requests gives us a reset handle.
    let reset_handle = client
        .encryption()
        .reset_cross_signing()
        .await
        .unwrap()
        .expect("We should have received a reset handle");

    assert_let!(CrossSigningResetAuthType::OAuth(oauth_info) = reset_handle.auth_type());
    assert_eq!(
        oauth_info.approval_url.as_str(),
        format!("{}/account/?action=org.matrix.cross_signing_reset", server.uri())
    );

    // Then it retries until it succeeds.
    let mut oauth = uiaa::OAuth::new();
    oauth.session = oauth_info.session.clone();
    reset_handle.auth(Some(uiaa::AuthData::OAuth(oauth))).await.expect("We should be able to reset the cross-signing keys after some attempts, waiting for the auth issue to allow us to upload");

    assert!(
        client.encryption().cross_signing_status().await.unwrap().is_complete(),
        "After the reset we have the cross-signing available.",
    );
}

#[async_test]
async fn test_bootstrap_records_that_the_keys_reached_the_homeserver() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    server.mock_upload_keys().ok().mount().await;
    server.mock_upload_cross_signing_keys().ok().expect(1).mount().await;
    server.mock_upload_cross_signing_signatures().ok().expect(1).mount().await;

    client.encryption().bootstrap_cross_signing(None).await.unwrap();

    let status = client.encryption().cross_signing_status().await.unwrap();
    assert!(status.is_complete(), "we hold the whole identity");
    assert!(status.is_published, "the homeserver accepted the keys we uploaded");
    assert!(status.is_usable(), "an identity everybody else can see is a usable one");
}

#[async_test]
async fn test_an_identity_that_never_reached_the_homeserver_is_published_again() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    server.mock_upload_keys().ok().mount().await;

    // The upload fails, so the keys exist here and nowhere else. Every other
    // device goes on seeing this one as unverified.
    {
        let _guard = server.mock_upload_cross_signing_keys().error500().mount_as_scoped().await;

        client
            .encryption()
            .bootstrap_cross_signing(None)
            .await
            .expect_err("the homeserver rejected the upload");
    }

    let status = client.encryption().cross_signing_status().await.unwrap();
    assert!(status.is_complete(), "the keys were created before the upload failed");
    assert!(!status.is_published, "but nothing of them reached the homeserver");
    assert!(!status.is_usable());

    // Having the identity locally is not enough to consider the work done: the
    // upload is tried again rather than left for nobody to retry.
    server.mock_upload_cross_signing_keys().ok().expect(1).mount().await;
    server.mock_upload_cross_signing_signatures().ok().expect(1).mount().await;

    client.encryption().bootstrap_cross_signing_if_needed(None).await.unwrap();

    let status = client.encryption().cross_signing_status().await.unwrap();
    assert!(status.is_usable(), "the second attempt got the keys published");
}

#[async_test]
async fn test_a_published_identity_is_not_published_again() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    server.mock_upload_keys().ok().mount().await;
    server.mock_upload_cross_signing_keys().ok().expect(1).mount().await;
    server.mock_upload_cross_signing_signatures().ok().expect(1).mount().await;

    client.encryption().bootstrap_cross_signing(None).await.unwrap();

    // The keys are on the homeserver, so there is nothing to redo: the mocks
    // above expect exactly one call each.
    client.encryption().bootstrap_cross_signing_if_needed(None).await.unwrap();

    assert!(client.encryption().cross_signing_status().await.unwrap().is_usable());
}
