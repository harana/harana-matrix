// Copyright 2023 The Matrix.org Foundation C.I.C.
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

// The crate this module came from did not warn on these; the merged crate root
// does, because the SDK proper did.
#![allow(missing_docs, missing_debug_implementations)]
#![cfg_attr(target_family = "wasm", allow(clippy::arc_with_non_send_sync))]
// Async methods must hand back futures that can be spawned on a
// multi-threaded runtime, which is what consumers of this crate do with
// them. WASM has no threads and its host types are not `Send`, so the lint
// is only applied elsewhere.
#![cfg_attr(not(target_family = "wasm"), deny(clippy::future_not_send))]

pub use eyeball_im;
use harana_matrix_common::html::HtmlSanitizerMode;

pub mod encryption_sync_service;
pub mod notification_client;
pub mod room_list_service;
#[cfg(feature = "experimental-search")]
pub mod search_service;
pub mod spaces;
pub mod sync_service;
pub mod timeline;
pub mod unable_to_decrypt_hook;

pub use self::{room_list_service::RoomListService, timeline::Timeline};

/// The default sanitizer mode used when sanitizing HTML.
const DEFAULT_SANITIZER_MODE: HtmlSanitizerMode = HtmlSanitizerMode::Compat;
