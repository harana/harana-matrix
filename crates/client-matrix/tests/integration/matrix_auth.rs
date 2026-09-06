use std::{collections::BTreeMap, sync::Mutex, time::Duration};

use assert_matches::assert_matches;
use client_matrix::{
    AuthApi, AuthSession, Client, HttpError, SessionTokens,
    authentication::matrix::MatrixSession,
    config::RequestConfig,
    executor::spawn,
    test_utils::{
        logged_in_client_with_server, mocks::MatrixMockServer, no_retry_test_client_with_server,
    },
};
use client_base::SessionMeta;
use common_test::{async_test, test_json};
use harana_matrix_common::{
    OwnedUserId,
    api::{
        self, MatrixVersion,
        client::{
            account::register::{RegistrationKind, v3::Request as RegistrationRequest},
            keys::upload_signatures::v3::SignedKeys,
            session::get_login_types::v3::LoginType,
            uiaa::{self, AuthData, MatrixUserIdentifier, UserIdentifier},
        },
        error::StandardErrorBody,
    },
    assign,
    encryption::CrossSigningKey,
    owned_device_id, owned_user_id,
    serde::Raw,
    user_id,
};
use serde_json::{from_value as from_json_value, json, to_value as to_json_value};
use url::Url;
use wiremock::{
    Mock, MockServer, Request, ResponseTemplate,
    matchers::{method, path},
};

#[async_test]
async fn test_restore_session() {
    let (client, _) = logged_in_client_with_server().await;
    let auth = client.matrix_auth();

    assert!(auth.logged_in(), "Client should be logged in with the MatrixAuth API");

    assert_matches!(client.auth_api(), Some(AuthApi::Matrix(_)));
    assert_matches!(client.session(), Some(AuthSession::Matrix(_)));
}

/// The session a client hands out names the homeserver it belongs to, so
/// restoring it later doesn't have to resolve the server name again.
#[async_test]
async fn test_the_session_carries_the_homeserver() {
    let (client, server) = logged_in_client_with_server().await;

    let session = client.matrix_auth().session().unwrap();

    assert_eq!(session.homeserver, Some(Url::parse(&server.uri()).unwrap()));
}

/// A restored session's homeserver wins over the URL the client was built
/// with: the tokens are only good against the server that issued them.
#[async_test]
async fn test_restoring_a_session_follows_its_homeserver() {
    let (_, server) = no_retry_test_client_with_server().await;

    let client = Client::builder()
        .homeserver_url("https://not.the.right.server.example.org")
        .request_config(RequestConfig::new().disable_retry())
        .build()
        .await
        .unwrap();

    let session_homeserver = Url::parse(&server.uri()).unwrap();
    client
        .matrix_auth()
        .restore_session(
            MatrixSession {
                meta: SessionMeta {
                    user_id: owned_user_id!("@example:localhost"),
                    device_id: owned_device_id!("DEVICEID"),
                },
                tokens: SessionTokens { access_token: "1234".to_owned(), refresh_token: None },
                homeserver: Some(session_homeserver.clone()),
            },
            Default::default(),
        )
        .await
        .unwrap();

    assert_eq!(client.homeserver(), session_homeserver);
}

/// A session without a homeserver, as saved before the field existed, leaves
/// the client where it was built.
#[async_test]
async fn test_restoring_a_session_without_a_homeserver_changes_nothing() {
    let (client, server) = no_retry_test_client_with_server().await;
    let built_with = client.homeserver();
    assert_eq!(built_with, Url::parse(&server.uri()).unwrap());

    client
        .matrix_auth()
        .restore_session(
            MatrixSession {
                meta: SessionMeta {
                    user_id: owned_user_id!("@example:localhost"),
                    device_id: owned_device_id!("DEVICEID"),
                },
                tokens: SessionTokens { access_token: "1234".to_owned(), refresh_token: None },
                homeserver: None,
            },
            Default::default(),
        )
        .await
        .unwrap();

    assert_eq!(client.homeserver(), built_with);
}

#[async_test]
async fn test_logout_stops_the_send_queue() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    client.send_queue().set_enabled(true).await;
    assert!(client.send_queue().is_enabled());

    server.mock_logout().ok().mock_once().mount().await;
    client.matrix_auth().logout().await.unwrap();

    // Queued requests belong to the session that queued them, so nothing more is
    // sent under the token we just invalidated.
    assert!(!client.send_queue().is_enabled());
}

#[async_test]
async fn test_the_session_carries_the_homeserver_it_belongs_to() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    let session = client.matrix_auth().session().expect("we are logged in");

    // Saving this session is enough to restore it later: the homeserver it was
    // opened against is part of it, so restoring it doesn't have to discover the
    // homeserver again.
    assert_eq!(session.homeserver, Some(client.homeserver()));
}

#[async_test]
async fn test_restoring_a_session_uses_the_homeserver_stored_with_it() {
    let previous_homeserver = MatrixMockServer::new().await;
    let homeserver = MatrixMockServer::new().await;

    // The client is built pointing at one server, and the session says it belongs
    // to another: this is the client that saved a session, was rebuilt from the
    // URL it happened to have, and would otherwise have to look the homeserver up
    // again.
    let client = previous_homeserver.client_builder().unlogged().build().await;

    let session = MatrixSession {
        meta: SessionMeta {
            user_id: owned_user_id!("@example:localhost"),
            device_id: owned_device_id!("DEVICEID"),
        },
        tokens: SessionTokens { access_token: "1234".to_owned(), refresh_token: None },
        homeserver: Some(Url::parse(&homeserver.uri()).unwrap()),
    };

    client
        .matrix_auth()
        .restore_session(session, client_matrix::store::RoomLoadSettings::default())
        .await
        .unwrap();

    assert_eq!(client.homeserver(), Url::parse(&homeserver.uri()).unwrap());

    // And the requests go there, rather than to the server the client was built
    // with.
    homeserver.mock_who_am_i().ok().mock_once().mount().await;
    client.whoami().await.unwrap();
}

#[async_test]
async fn test_restoring_a_session_without_a_homeserver_keeps_the_one_of_the_client() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().unlogged().build().await;
    let homeserver = client.homeserver();

    // A session stored before this SDK kept the homeserver with it deserializes
    // with none, and the client stays where it was pointed.
    let session = MatrixSession {
        meta: SessionMeta {
            user_id: owned_user_id!("@example:localhost"),
            device_id: owned_device_id!("DEVICEID"),
        },
        tokens: SessionTokens { access_token: "1234".to_owned(), refresh_token: None },
        homeserver: None,
    };

    client
        .matrix_auth()
        .restore_session(session, client_matrix::store::RoomLoadSettings::default())
        .await
        .unwrap();

    assert_eq!(client.homeserver(), homeserver);
}

#[async_test]
async fn test_a_session_stored_without_a_homeserver_still_deserializes() {
    let session: MatrixSession = from_json_value(json!({
        "access_token": "abcd",
        "user_id": "@user:localhost",
        "device_id": "HIJKLMN",
    }))
    .unwrap();

    assert_eq!(session.homeserver, None);
}

#[async_test]
async fn test_logout_cancels_the_requests_in_flight() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    // A request the homeserver takes its time answering. The test never waits for
    // that answer: logging out is what ends the request.
    Mock::given(method("GET"))
        .and(path("/_matrix/client/v3/account/whoami"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(&*test_json::WHOAMI)
                .set_delay(Duration::from_secs(60)),
        )
        .mount(server.server())
        .await;
    server.mock_logout().ok().mock_once().mount().await;

    let whoami = spawn({
        let client = client.clone();
        async move { client.whoami().await }
    });

    // Wait for the request to reach the homeserver, so it is really in flight when
    // the logout happens.
    while !server
        .server()
        .received_requests()
        .await
        .expect("the mock server records the requests it receives")
        .iter()
        .any(|request| request.url.path().ends_with("/account/whoami"))
    {
        tokio::task::yield_now().await;
    }

    client.matrix_auth().logout().await.unwrap();

    // The session is over, so waiting for that answer is pointless: the request is
    // dropped rather than left to finish under an invalidated token.
    assert_matches!(whoami.await.unwrap(), Err(HttpError::Cancelled));

    // Requests made after the logout are not cancelled: this ends a session, it
    // doesn't close the client.
    server.mock_versions().ok().mock_once().named("versions").mount().await;
    client.fetch_server_versions(None).await.unwrap();
}

#[async_test]
async fn test_restore_session_with_access_token() {
    let (client, server) = no_retry_test_client_with_server().await;

    Mock::given(method("GET"))
        .and(path("/_matrix/client/r0/account/whoami"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user_id": "@example:localhost",
            "device_id": "MYDEVICEID",
        })))
        .mount(&server)
        .await;

    let session = client
        .restore_session_with_access_token(
            SessionTokens { access_token: "My-Token".to_owned(), refresh_token: None },
            Default::default(),
        )
        .await
        .unwrap();

    assert_eq!(session.meta.user_id, "@example:localhost");
    assert_eq!(session.meta.device_id, "MYDEVICEID");
    assert!(client.matrix_auth().logged_in());
}

#[async_test]
async fn test_restore_session_with_access_token_without_a_device_id() {
    let (client, server) = no_retry_test_client_with_server().await;

    // The device ID is optional in the specification, but the SDK needs one.
    Mock::given(method("GET"))
        .and(path("/_matrix/client/r0/account/whoami"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "user_id": "@example:localhost",
        })))
        .mount(&server)
        .await;

    let error = client
        .restore_session_with_access_token(
            SessionTokens { access_token: "My-Token".to_owned(), refresh_token: None },
            Default::default(),
        )
        .await
        .unwrap_err();

    assert_matches!(error, client_matrix::Error::MissingDeviceId);
    assert!(!client.matrix_auth().logged_in());
}

#[async_test]
async fn test_login_honours_the_request_config() {
    // The server rate-limits every login attempt.
    async fn rate_limited_server() -> (Client, MockServer) {
        let (client, server) = no_retry_test_client_with_server().await;

        Mock::given(method("POST"))
            .and(path("/_matrix/client/r0/login"))
            .respond_with(ResponseTemplate::new(429).set_body_json(json!({
                "errcode": "M_LIMIT_EXCEEDED",
                "error": "Too many requests",
                "retry_after_ms": 1,
            })))
            .up_to_n_times(5)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/_matrix/client/r0/login"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::LOGIN))
            .mount(&server)
            .await;

        (client, server)
    }

    // The default configuration gives up before the server stops rate-limiting.
    let (client, _server) = rate_limited_server().await;
    client.matrix_auth().login_username("example", "wordpass").send().await.unwrap_err();
    assert!(!client.matrix_auth().logged_in());

    // A configuration with a higher retry limit waits the rate limit out.
    let (client, _server) = rate_limited_server().await;
    client
        .matrix_auth()
        .login_username("example", "wordpass")
        .request_config(RequestConfig::new().retry_limit(10))
        .send()
        .await
        .unwrap();
    assert!(client.matrix_auth().logged_in());
}

#[async_test]
async fn test_login() {
    let (client, server) = no_retry_test_client_with_server().await;
    let homeserver = Url::parse(&server.uri()).unwrap();

    Mock::given(method("GET"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::LOGIN_TYPES))
        .mount(&server)
        .await;

    let can_password = client
        .matrix_auth()
        .get_login_types()
        .await
        .unwrap()
        .flows
        .iter()
        .any(|flow| matches!(flow, LoginType::Password(_)));
    assert!(can_password);

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::LOGIN))
        .mount(&server)
        .await;

    let auth = client.matrix_auth();
    auth.login_username("example", "wordpass").send().await.unwrap();

    assert!(client.is_active(), "Client should be active");
    assert!(auth.logged_in(), "Client should be logged in with the MatrixAuth API");

    assert_matches!(client.auth_api(), Some(AuthApi::Matrix(_)));
    assert_matches!(client.session(), Some(AuthSession::Matrix(_)));

    assert_eq!(client.homeserver(), homeserver);
}

#[async_test]
async fn test_login_with_discovery() {
    let (client, server) = no_retry_test_client_with_server().await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::LOGIN_WITH_DISCOVERY))
        .mount(&server)
        .await;

    client.matrix_auth().login_username("example", "wordpass").send().await.unwrap();

    assert!(client.is_active(), "Client should be active");
    assert_eq!(client.homeserver().as_str(), "https://example.org/");
}

#[async_test]
async fn test_login_no_discovery() {
    let (client, server) = no_retry_test_client_with_server().await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::LOGIN))
        .mount(&server)
        .await;

    client.matrix_auth().login_username("example", "wordpass").send().await.unwrap();

    assert!(client.is_active(), "Client should be active");
    assert_eq!(client.homeserver(), Url::parse(&server.uri()).unwrap());
}

#[async_test]
#[cfg(feature = "sso-login")]
async fn test_login_with_sso() {
    let (client, server) = no_retry_test_client_with_server().await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::LOGIN))
        .mount(&server)
        .await;

    let idp = api::client::session::get_login_types::v3::IdentityProvider::new(
        "some-id".to_owned(),
        "idp-name".to_owned(),
    );
    client
        .matrix_auth()
        .login_sso(|sso_url| async move {
            let sso_url = Url::parse(&sso_url).unwrap();

            let (_, redirect) =
                sso_url.query_pairs().find(|(key, _)| key == "redirectUrl").unwrap();

            let mut redirect_url = Url::parse(&redirect).unwrap();
            redirect_url.set_query(Some("loginToken=tinytoken"));

            reqwest::get(redirect_url.to_string()).await.unwrap();

            Ok(())
        })
        .identity_provider_id(&idp.id)
        .await
        .unwrap();

    assert!(client.is_active(), "Client should be active");
}

#[async_test]
async fn test_login_with_sso_token() {
    let (client, server) = no_retry_test_client_with_server().await;

    Mock::given(method("GET"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::LOGIN_TYPES))
        .mount(&server)
        .await;

    let auth = client.matrix_auth();
    let can_sso = auth
        .get_login_types()
        .await
        .unwrap()
        .flows
        .iter()
        .any(|flow| matches!(flow, LoginType::Sso(_)));
    assert!(can_sso);

    let sso_url = auth.get_sso_login_url("http://127.0.0.1:3030", None).await;
    sso_url.unwrap();

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::LOGIN))
        .mount(&server)
        .await;

    auth.login_token("averysmalltoken").send().await.unwrap();

    assert!(client.is_active(), "Client should be active");
}

#[async_test]
async fn test_login_with_sso_callback() {
    let (client, server) = no_retry_test_client_with_server().await;

    Mock::given(method("GET"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::LOGIN_TYPES))
        .mount(&server)
        .await;

    let auth = client.matrix_auth();
    let can_sso = auth
        .get_login_types()
        .await
        .unwrap()
        .flows
        .iter()
        .any(|flow| matches!(flow, LoginType::Sso(_)));
    assert!(can_sso);

    let sso_url = auth.get_sso_login_url("http://127.0.0.1:3030", None).await;
    sso_url.unwrap();

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::LOGIN))
        .mount(&server)
        .await;

    let callback_url = Url::parse("http://127.0.0.1:3030?loginToken=averysmalltoken").unwrap();
    auth.login_with_sso_callback(callback_url.into()).unwrap().await.unwrap();

    assert!(client.is_active(), "Client should be active");
}

#[async_test]
async fn test_login_error() {
    let (client, server) = no_retry_test_client_with_server().await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(403).set_body_json(&*test_json::LOGIN_RESPONSE_ERR))
        .mount(&server)
        .await;

    if let Err(err) = client.matrix_auth().login_username("example", "wordpass").send().await {
        if let Some(api_err) = err.as_client_api_error() {
            assert_eq!(api_err.status_code, http::StatusCode::from_u16(403).unwrap());

            if let api::error::ErrorBody::Standard(StandardErrorBody { kind, message, .. }) =
                &api_err.body
            {
                if !matches!(*kind, api::error::ErrorKind::Forbidden) {
                    panic!("found the wrong `ErrorKind` {kind:?}, expected `Forbidden");
                }

                assert_eq!(message, "Invalid password");
            } else {
                panic!("non-standard error body")
            }
        } else {
            panic!("found the wrong `Error` type {err:?}, expected `Error::RumaResponse");
        }
    } else {
        panic!("this request should return an `Err` variant")
    }
}

#[async_test]
async fn test_register_error() {
    let (client, server) = no_retry_test_client_with_server().await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/register"))
        .respond_with(
            ResponseTemplate::new(403).set_body_json(&*test_json::REGISTRATION_RESPONSE_ERR),
        )
        .mount(&server)
        .await;

    let user = assign!(RegistrationRequest::new(), {
        username: Some("user".to_owned()),
        password: Some("password".to_owned()),
        auth: Some(AuthData::FallbackAcknowledgement(
            uiaa::FallbackAcknowledgement::new("foobar".to_owned()),
        )),
        kind: RegistrationKind::User,
    });

    if let Err(err) = client.matrix_auth().register(user).await {
        if let Some(api_err) = err.as_client_api_error() {
            assert_eq!(api_err.status_code, http::StatusCode::from_u16(403).unwrap());
            if let api::error::ErrorBody::Standard(StandardErrorBody { kind, message, .. }) =
                &api_err.body
            {
                if !matches!(*kind, api::error::ErrorKind::Forbidden) {
                    panic!("found the wrong `ErrorKind` {kind:?}, expected `Forbidden");
                }

                assert_eq!(message, "Invalid password");
            } else {
                panic!("non-standard error body")
            }
        } else {
            panic!("found the wrong `Error` type {err:#?}, expected `UiaaResponse`");
        }
    } else {
        panic!("this request should return an `Err` variant")
    }
}

#[test]
fn test_deserialize_session() {
    // First version, or second version without refresh token.
    let json = json!({
        "access_token": "abcd",
        "user_id": "@user:localhost",
        "device_id": "EFGHIJ",
    });
    let session: MatrixSession = from_json_value(json).unwrap();
    assert_eq!(session.tokens.access_token, "abcd");
    assert_eq!(session.meta.user_id, "@user:localhost");
    assert_eq!(session.meta.device_id, "EFGHIJ");
    assert_eq!(session.tokens.refresh_token, None);

    // Second version with refresh_token.
    let json = json!({
        "access_token": "abcd",
        "refresh_token": "wxyz",
        "user_id": "@user:localhost",
        "device_id": "EFGHIJ",
    });
    let session: MatrixSession = from_json_value(json).unwrap();
    assert_eq!(session.tokens.access_token, "abcd");
    assert_eq!(session.meta.user_id, "@user:localhost");
    assert_eq!(session.meta.device_id, "EFGHIJ");
    assert_eq!(session.tokens.refresh_token.as_deref(), Some("wxyz"));
}

#[test]
fn test_serialize_session() {
    // Without refresh token.
    let mut session = MatrixSession {
        homeserver: None,
        meta: SessionMeta {
            user_id: owned_user_id!("@user:localhost"),
            device_id: owned_device_id!("EFGHIJ"),
        },
        tokens: SessionTokens { access_token: "abcd".to_owned(), refresh_token: None },
    };
    assert_eq!(
        to_json_value(session.clone()).unwrap(),
        json!({
            "access_token": "abcd",
            "user_id": "@user:localhost",
            "device_id": "EFGHIJ",
        })
    );

    // With refresh_token.
    session.tokens.refresh_token = Some("wxyz".to_owned());
    assert_eq!(
        to_json_value(session).unwrap(),
        json!({
            "access_token": "abcd",
            "refresh_token": "wxyz",
            "user_id": "@user:localhost",
            "device_id": "EFGHIJ",
        })
    );
}

#[cfg(feature = "e2e-encryption")]
#[async_test]
async fn test_login_with_cross_signing_bootstrapping() {
    use assert_matches2::assert_let;

    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/keys/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_keys": {
                "@alice:example.org": {}
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/keys/upload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "one_time_key_counts": {}
        })))
        .mount(&server)
        .await;

    let num_calls = Mutex::new(0);
    Mock::given(method("POST"))
        .and(path("/_matrix/client/unstable/keys/device_signing/upload"))
        .respond_with(move |req: &Request| {
            #[derive(Debug, serde::Deserialize)]
            struct Parameters {
                auth: Option<AuthData>,
                master_key: Option<Raw<CrossSigningKey>>,
                self_signing_key: Option<Raw<CrossSigningKey>>,
                user_signing_key: Option<Raw<CrossSigningKey>>,
            }

            let params: Parameters = req.body_json().unwrap();

            {
                let mut num_calls = num_calls.lock().unwrap();
                if *num_calls == 0 {
                    // First time, we use a password.
                    assert_let!(Some(AuthData::Password(password)) = &params.auth);
                    assert_eq!(
                        password.identifier,
                        UserIdentifier::Matrix(MatrixUserIdentifier::new("example".to_owned()))
                    );
                    assert_eq!(password.password, "hunter2");

                    *num_calls += 1;
                } else {
                    // Second time, we use a login token. Pretend MSC3967 is enabled and require an
                    // empty auth.
                    assert!(params.auth.is_none());
                }
            }

            assert!(params.master_key.is_some());
            assert!(params.self_signing_key.is_some());
            assert!(params.user_signing_key.is_some());

            ResponseTemplate::new(200).set_body_json(json!({}))
        })
        .expect(2)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/unstable/keys/signatures/upload"))
        .respond_with(|req: &Request| {
            #[derive(Debug, serde::Deserialize)]
            #[serde(transparent)]
            struct Parameters(BTreeMap<OwnedUserId, SignedKeys>);

            let params: Parameters = req.body_json().unwrap();
            assert!(params.0.contains_key(user_id!("@alice:example.org")));

            ResponseTemplate::new(200).set_body_json(json!({
                "failures": {}
            }))
        })
        .mount(&server)
        .await;

    {
        // Login with username and password.
        let _guard = Mock::given(method("POST"))
            .and(path("/_matrix/client/r0/login"))
            .respond_with(|req: &Request| {
                #[derive(serde::Deserialize)]
                struct Parameters {
                    r#type: String,
                    password: String,
                }

                let params: Parameters = req.body_json().unwrap();
                assert_eq!(params.r#type, "m.login.password");
                assert_eq!(params.password, "hunter2");

                ResponseTemplate::new(200).set_body_json(json!({
                    "access_token": "abc123",
                    "device_id": "GHTYAJCE",
                    "home_server": "example.org",
                    "user_id": "@alice:example.org"
                }))
            })
            .mount_as_scoped(&server)
            .await;

        let client = Client::builder()
            .homeserver_url(server.uri())
            .server_versions([MatrixVersion::V1_0])
            .with_encryption_settings(client_matrix::encryption::EncryptionSettings {
                auto_enable_cross_signing: true,
                ..Default::default()
            })
            .request_config(RequestConfig::new().disable_retry())
            .build()
            .await
            .unwrap();

        let auth = client.matrix_auth();
        auth.login_username("example", "hunter2").send().await.unwrap();

        assert!(client.is_active(), "Client should be active");
        assert!(auth.logged_in(), "Client should be logged in with the MatrixAuth API");

        client.encryption().wait_for_e2ee_initialization_tasks().await;

        let me = client.user_id().expect("we are now logged in");
        let own_identity =
            client.encryption().get_user_identity(me).await.expect("succeeds").expect("is present");

        assert_eq!(own_identity.user_id(), me);
        assert!(own_identity.is_verified());
    }

    {
        // Login with a token.
        let _guard = Mock::given(method("POST"))
            .and(path("/_matrix/client/r0/login"))
            .respond_with(|req: &Request| {
                #[derive(serde::Deserialize)]
                struct Parameters {
                    r#type: String,
                    token: String,
                }

                let params: Parameters = req.body_json().unwrap();
                assert_eq!(params.r#type, "m.login.token");
                assert_eq!(params.token, "HUNTER2");

                ResponseTemplate::new(200).set_body_json(json!({
                    "access_token": "abc123",
                    "device_id": "GHTYAJCE",
                    "home_server": "example.org",
                    "user_id": "@alice:example.org"
                }))
            })
            .mount_as_scoped(&server)
            .await;

        let client = Client::builder()
            .homeserver_url(server.uri())
            .server_versions([MatrixVersion::V1_0])
            .with_encryption_settings(client_matrix::encryption::EncryptionSettings {
                auto_enable_cross_signing: true,
                ..Default::default()
            })
            .request_config(RequestConfig::new().disable_retry())
            .build()
            .await
            .unwrap();

        let auth = client.matrix_auth();
        auth.login_token("HUNTER2").send().await.unwrap();

        assert!(client.is_active(), "Client should be active");
        assert!(auth.logged_in(), "Client should be logged in with the MatrixAuth API");

        client.encryption().wait_for_e2ee_initialization_tasks().await;

        let me = client.user_id().expect("we are now logged in");
        let own_identity =
            client.encryption().get_user_identity(me).await.expect("succeeds").expect("is present");

        assert_eq!(own_identity.user_id(), me);
        assert!(own_identity.is_verified());
    }

    server.verify().await;
}

#[cfg(feature = "e2e-encryption")]
#[async_test]
async fn test_login_doesnt_fail_if_cross_signing_bootstrapping_failed() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/keys/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_keys": {
                "@alice:example.org": {}
            }
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/unstable/keys/device_signing/upload"))
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({})))
        .mount(&server)
        .await;

    // Login with username and password.
    let _guard = Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(|req: &Request| {
            #[derive(serde::Deserialize)]
            struct Parameters {
                r#type: String,
                password: String,
            }

            let params: Parameters = req.body_json().unwrap();
            assert_eq!(params.r#type, "m.login.password");
            assert_eq!(params.password, "hunter2");

            ResponseTemplate::new(200).set_body_json(json!({
                "access_token": "abc123",
                "device_id": "GHTYAJCE",
                "home_server": "example.org",
                "user_id": "@alice:example.org"
            }))
        })
        .mount_as_scoped(&server)
        .await;

    let client = Client::builder()
        .homeserver_url(server.uri())
        .server_versions([MatrixVersion::V1_0])
        .with_encryption_settings(client_matrix::encryption::EncryptionSettings {
            auto_enable_cross_signing: true,
            ..Default::default()
        })
        .request_config(RequestConfig::new().disable_retry())
        .build()
        .await
        .unwrap();

    let auth = client.matrix_auth();
    auth.login_username("example", "hunter2").send().await.unwrap();

    assert!(client.is_active(), "Client should be active");
    assert!(auth.logged_in(), "Client should be logged in with the MatrixAuth API");

    let me = client.user_id().expect("we are now logged in");

    client.encryption().wait_for_e2ee_initialization_tasks().await;

    let own_identity = client.encryption().get_user_identity(me).await.expect("succeeds");
    let identity = own_identity.expect("created local default identity");
    assert!(identity.is_verified());
}

#[cfg(feature = "e2e-encryption")]
#[async_test]
async fn test_login_with_cross_signing_bootstrapping_already_bootstrapped() {
    // Even if we enabled cross-signing bootstrap for another device, it won't
    // restart the procedure.
    let (builder, server) = client_matrix::test_utils::test_client_builder_with_server().await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "access_token": "abc123",
            "device_id": "FEJILWLI",
            "home_server": "example.org",
            "user_id": "@alice:example.org"
        })))
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/_matrix/client/r0/keys/query"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "device_keys": {
                "@alice:example.org": {
                    "GHTYAJCE": {
                      "user_id": "@alice:example.org",
                      "device_id": "GHTYAJCE",
                      "algorithms": [
                        "m.olm.v1.curve25519-aes-sha2",
                        "m.megolm.v1.aes-sha2"
                      ],
                      "keys": {
                        "curve25519:GHTYAJCE": "okg/vMIocD10QuctIUhBOk9ccrrNLUtBRzTDSJlVRw4",
                        "ed25519:GHTYAJCE": "MxZSkgCAPVM4KZ3VCy0zG88vYp7Z+jjy8l5z1Ji3B7Y"
                      },
                      "signatures": {
                        "@alice:example.org": {
                          "ed25519:784pBUxon7VPcJJs69XkvN+AbC1ks07bvMh4qOPnVgY": "369BRaMHLW4nwrpy34eBYl0TpUeZoCs+IFXvTWJUBAv8Va4iqgB07Wi7XcJ+mmE4M7asyKnf5f7Zh4kGjOoNAQ"
                        }
                      }
                    }
                }
            },
            "failures": {},
            "master_keys": {
                "@alice:example.org": {
                    "user_id": "@alice:example.org",
                    "usage": [
                      "master"
                    ],
                    "keys": {
                      "ed25519:qGlcu2K7qaDn6wBG3DHOtnOeTgu6Dj1QLsxHSEGtODg": "qGlcu2K7qaDn6wBG3DHOtnOeTgu6Dj1QLsxHSEGtODg"
                    },
                    "signatures": {
                      "@alice:example.org": {
                        "ed25519:GHTYAJCE": "L3v/GSbEN+qO/vJipVupW6j3fHFn1CPSt8w5Ob0IpByM+LOuxKTc60kpisl94cueQZnl40mnKEFoYzI0JZWTDA",
                        "ed25519:qGlcu2K7qaDn6wBG3DHOtnOeTgu6Dj1QLsxHSEGtODg": "rb1Y9O5nfF0bU2p7aWF+I4095C4sm3uc/IWxdC55Q8GtrGFNsiR+YTvi3tJahMLDxYOCzgXl7dJ1mXsvzRNwBA"
                      }
                    }
                }
            },
            "self_signing_keys": {
                "@alice:example.org": {
                    "user_id": "@alice:example.org",
                    "usage": [
                      "self_signing"
                    ],
                    "keys": {
                      "ed25519:784pBUxon7VPcJJs69XkvN+AbC1ks07bvMh4qOPnVgY": "784pBUxon7VPcJJs69XkvN+AbC1ks07bvMh4qOPnVgY"
                    },
                    "signatures": {
                      "@alice:example.org": {
                        "ed25519:qGlcu2K7qaDn6wBG3DHOtnOeTgu6Dj1QLsxHSEGtODg": "TQQOP7BYFB6aZ/cVOa2qOzmzsap2kTpCLMEI1U8nO1kVtGRjXMGU+xoJ43DDWEgRvy2iUA7AMQpC1yCxo79BBA"
                      }
                    }
                }
            },
            "user_signing_keys": {
                "@alice:example.org": {
                    "user_id": "@alice:example.org",
                    "usage": [
                      "user_signing"
                    ],
                    "keys": {
                      "ed25519:D5nFYOzvmWUab4084Tahqhe4NgfQnuJ2XvdETSbOqrs": "D5nFYOzvmWUab4084Tahqhe4NgfQnuJ2XvdETSbOqrs"
                    },
                    "signatures": {
                      "@alice:example.org": {
                        "ed25519:qGlcu2K7qaDn6wBG3DHOtnOeTgu6Dj1QLsxHSEGtODg": "fFf76W6aPyxiwrINjlEjYxTIvC+35uth/WK7mzNLtQgHCGyzhJqRZECvHVQ4slr/oSu1EAAYJbAkq/QU0bniDg"
                      }
                    }
                }
            },
        })))
        .mount(&server)
        .await;

    let client = builder
        .with_encryption_settings(client_matrix::encryption::EncryptionSettings {
            auto_enable_cross_signing: true,
            ..Default::default()
        })
        .request_config(RequestConfig::new().disable_retry())
        .build()
        .await
        .unwrap();

    let auth = client.matrix_auth();
    auth.login_username("example", "hunter2").send().await.unwrap();

    assert!(client.is_active(), "Client should be active");
    assert!(auth.logged_in(), "Client should be logged in with the MatrixAuth API");
}
