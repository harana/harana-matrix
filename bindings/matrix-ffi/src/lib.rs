#![recursion_limit = "256"]
#![allow(unused_qualifications, clippy::new_without_default)]
// Needed because uniffi macros contain empty lines after docs.
#![allow(clippy::empty_line_after_doc_comments)]
// Needed because uniffi generates a big const array.
#![allow(clippy::large_const_arrays)]

mod authentication;
mod chunk_iterator;
mod client;
mod client_builder;
mod content_scanner;
mod encryption;
mod error;
mod event;
mod helpers;
mod identity_status_change;
mod live_locations_observer;
mod notification;
mod notification_settings;
mod password_strength;
mod platform;
mod qr_code;
mod room;
mod room_alias;
mod room_directory_search;
mod room_list;
mod room_member;
mod room_preview;
mod ruma;
mod runtime;
#[cfg(feature = "experimental-search")]
mod search_service;
mod session_verification;
mod spaces;
mod store;
mod sync_service;
mod sync_v2;
mod task_handle;
mod timeline;
mod user_id;
mod utd;
mod utils;
mod widget;

use matrix::ruma::events::room::message::RoomMessageEventContentWithoutRelation;

use self::{
    error::ClientError,
    ruma::{Mentions, RoomMessageEventContentWithoutRelationExt},
    task_handle::TaskHandle,
};

uniffi::include_scaffolding!("api");

/// The seams a Rust application embedding these bindings can use to replace
/// parts of what the bindings otherwise decide for it.
///
/// None of this is exported through `uniffi`: every item here carries a Rust
/// trait object, which cannot cross the FFI boundary. Swift and Kotlin callers
/// keep using the built-in SQLite, IndexedDB and in-memory stores, and the
/// built-in text and Sentry log sinks.
pub mod pluggable {
    pub use matrix::StoreProvider;

    pub use crate::{
        client_builder::ClientBuilder,
        platform::telemetry::{TelemetryLayer, TelemetryProvider, set_telemetry_providers},
    };
}

#[matrix_ffi_macros::export]
fn sdk_git_sha() -> String {
    env!("VERGEN_GIT_SHA").to_owned()
}

/// The worked examples under `bindings/examples` are written against this
/// crate's API but are not compiled by CI: no Swift or Kotlin toolchain runs
/// there, and building them means building the bindings first.
///
/// These tests are the guard rail instead. They fail when an API an example
/// demonstrates is renamed or removed, which is when an example silently stops
/// working.
#[cfg(test)]
mod examples {
    use std::{fs::read_to_string, path::PathBuf};

    /// The API the examples walk through, as the bindings name it (snake case)
    /// paired with the name the generated Swift and Kotlin use (camel case).
    const DEMONSTRATED_API: &[(&str, &str)] = &[
        ("fn init_platform(", "initPlatform("),
        ("fn set_log_event_listener(", "setLogEventListener("),
        ("fn set_panic_listener(", "setPanicListener("),
        ("fn server_name_from_user_id(", "serverNameFromUserId("),
        ("fn session_paths(", "sessionPaths("),
        ("fn user_agent(", "userAgent("),
        ("fn login(", "login("),
        ("fn session(", "session("),
        ("fn restore_session(", "restoreSession("),
        ("fn bootstrap_cross_signing(", "bootstrapCrossSigning("),
        ("fn sync_service(", "syncService("),
        ("fn get_room(", "getRoom("),
        ("fn state_event(", "stateEvent("),
        ("fn state_events(", "stateEvents("),
        ("fn account_data(", "accountData("),
        ("fn set_account_data(", "setAccountData("),
        ("fn set_topic(", "setTopic("),
        ("fn invite_user_by_id(", "inviteUserById("),
        ("fn join_room_by_id(", "joinRoomById("),
    ];

    fn crate_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
    }

    fn examples_dir() -> PathBuf {
        crate_root().join("../examples")
    }

    /// Every source file of this crate, concatenated.
    fn crate_sources() -> String {
        fn read_dir_recursively(dir: &std::path::Path, into: &mut String) {
            for entry in std::fs::read_dir(dir).expect("the source directory is readable") {
                let path = entry.expect("the directory entry is readable").path();

                if path.is_dir() {
                    read_dir_recursively(&path, into);
                } else if path.extension().is_some_and(|extension| extension == "rs") {
                    into.push_str(&read_to_string(&path).expect("the source file is readable"));
                }
            }
        }

        let mut sources = String::new();
        read_dir_recursively(&crate_root().join("src"), &mut sources);
        sources
    }

    #[test]
    fn test_the_examples_are_written_against_an_api_that_exists() {
        let sources = crate_sources();

        for (rust_name, _) in DEMONSTRATED_API {
            assert!(
                sources.contains(rust_name),
                "`{rust_name}` is demonstrated in bindings/examples but no longer exists; \
                 update the examples along with the API"
            );
        }
    }

    #[test]
    fn test_the_examples_still_demonstrate_that_api() {
        let swift = read_to_string(examples_dir().join("swift/Example.swift"))
            .expect("the Swift example is where the README says it is");
        let kotlin = read_to_string(examples_dir().join("kotlin/Example.kt"))
            .expect("the Kotlin example is where the README says it is");

        for (rust_name, foreign_name) in DEMONSTRATED_API {
            assert!(
                swift.contains(foreign_name),
                "the Swift example no longer calls `{foreign_name}` ({rust_name})"
            );
            assert!(
                kotlin.contains(foreign_name),
                "the Kotlin example no longer calls `{foreign_name}` ({rust_name})"
            );
        }
    }

    /// The bindings README points at the examples; a moved file breaks that
    /// link silently.
    #[test]
    fn test_the_readme_points_at_the_examples() {
        let readme = read_to_string(crate_root().join("../README.md"))
            .expect("the bindings README is readable");

        assert!(readme.contains("./examples"), "the bindings README no longer links the examples");
        assert!(examples_dir().join("README.md").is_file());
    }
}
