# Changelog

All notable changes to this project will be documented in this file.

This crate is the merge of what used to be the `client-*` crates. The
sections below are their changelogs, unchanged, under the name each had.

<!-- changelog start -->

---

## `client-matrix`


All notable changes to this project will be documented in this file.


## Unreleased

### Added

- Add `ClientBuilder::discovery_cache_timeout`, which sets how long the
  homeserver discovery data stays usable before the client refreshes it, and
  `Client::rediscover`, which expires the stored copies and fetches them
  again rather than waiting for the timeout.
  ([#36](https://github.com/harana/harana-matrix/issues/36))

### Fixed

- Persist the sliding sync `pos` in the event cache store rather than the
  crypto store. A client built without encryption could not resume from its
  previous position, because the crypto store it was kept in did not exist. A
  `pos` written by an older version is migrated out of the crypto store the
  first time it is read.
  ([#250](https://github.com/harana/harana-matrix/issues/250))
- Stop retrying a `/keys/upload` the server rejects with "One time key ...
  already exists". The request is now marked as sent, so it no longer blocks
  everything queued behind it, cross-signing bootstrap included.
  ([#191](https://github.com/harana/harana-matrix/issues/191),
  [#259](https://github.com/harana/harana-matrix/issues/259))
- Document on `Client::restore_session` that the store has to be persisted
  separately. ([#58](https://github.com/harana/harana-matrix/issues/58))

## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

### Added

- Add `Client::tile_server` and a `TileServerInfo` struct to expose the
  homeserver-advertised map tile server (`tile_server` field of the matrix
  client well-known,
  [MSC3488](https://github.com/matrix-org/matrix-spec-proposals/pull/3488)).
  Returns `None` when the homeserver hasn't advertised one or the well-known is
  unavailable.
  ([#6610](https://github.com/matrix-org/matrix-rust-sdk/pulls/6610))

### Changed

- [**breaking**] `RumaApiError` is now a type alias for `UiaaResponse`, because
  they have similar variants containing the same data. The `ClientApi` variant
  is now `MatrixError`, and the `Uiaa` variant is `AuthResponse`.
  ([#6574](https://github.com/matrix-org/matrix-rust-sdk/pulls/6574))
- [**breaking**] `Pusher::set` now takes an `append: bool` parameter, forwarded
  to the homeserver on `POST /_matrix/client/v3/pushers/set`. Pass `true` to
  keep an existing pusher with the same `app_id` and `pushkey` registered for
  other users (e.g. multi-profile clients on a single device); pass `false` to
  preserve the previous default behaviour.
  ([#6600](https://github.com/matrix-org/matrix-rust-sdk/pulls/6600))

### Fixed

- Upgrade Ruma to 0.16.0, fixing a deserialization issue for
  `m.key.verification.accept` events.
  ([#6628](https://github.com/matrix-org/matrix-rust-sdk/pulls/6628))
- A cyclic reference of `Client` has been detected in
  `ThreadSubscriptionCatchup`, preventing `Client` to drop correctly. This is
  now fixed, removing a memory leak about `Client`.
  ([#6594](https://github.com/matrix-org/matrix-rust-sdk/pulls/6594))
- Fix a panic due to non-deterministic sorting of pinned events.
  ([#6595](https://github.com/matrix-org/matrix-rust-sdk/pulls/6595))

## [0.17.0] - 2026-05-08

### Security fixes

- Reject invalid edits as candidates for the latest event.
  ([#6454](https://github.com/matrix-org/matrix-rust-sdk/pull/6454), Moderate,
  [CVE-2026-45057](https://www.cve.org/CVERecord?id=CVE-2026-45057),
  [GHSA-h97m-27fx-42rx](https://github.com/matrix-org/matrix-rust-sdk/security/advisories/GHSA-h97m-27fx-42rx))

### Features

- [**breaking**] `Room::is_dm` was renamed to `Room::compute_is_dm` to match its
  behavior, since it'll now compute and cache the result. A new _synchronous_
  `Room::is_dm` function was added which centralizes the logic of checking if
  something is a DM based on that cached value and the provided
  `DmRoomDefinition`. `Room::sync_members` will also now compute active service
  members. ([#6537](https://github.com/matrix-org/matrix-rust-sdk/pull/6537))
- [**breaking**] Enforce atomic and synchronized updates to `RoomInfo`. Requires
  `StateStore::save_changes` to acquire state store lock and replaces
  `Room::set_room_info` with an atomic version, `Room::update_room_info`, which
  is also synchronized by the state store lock.
  ([#6478](https://github.com/matrix-org/matrix-rust-sdk/pull/6478))
- Added `beacon` and `beacon_info` fields to `RoomPowerLevelChanges`, allowing
  callers to read and update the power levels required to send beacon (live
  location) message events and beacon info state events respectively.
  ([#6540](https://github.com/matrix-org/matrix-rust-sdk/pull/6540))
- Added `DmRoomDefinition` as a parameter of `ClientBuilder` so we can specify
  it when creating a Client. Also added a `Room::is_dm` method and added some
  logic to use the new DM definitions in `Client::get_dm_rooms` and when using
  message search.
  ([#6490](https://github.com/matrix-org/matrix-rust-sdk/pull/6490))
- Sharing encrypted history on room invite, per
  [MSC4268](https://github.com/matrix-org/matrix-spec-proposals/pull/4268) is
  now enabled by default (though can still be disabled via
  `ClientBuilder::with_enable_share_history_on_invite`).
  ([#6497](https://github.com/matrix-org/matrix-rust-sdk/pull/6497))
- Add `Client::get_dm_rooms` function to get an iterator with the DMs for the
  provided user id.
  ([#6487](https://github.com/matrix-org/matrix-rust-sdk/pull/6487))
- Support the stable `m.key_backup` prefix for MSC4287: Sharing key backup
  preference between clients.
  ([#6410](https://github.com/matrix-org/matrix-rust-sdk/pull/6410))
- Add new high-level search helpers in `ui::search` to perform
  searches for messages in a room or across all rooms.
  ([#6394](https://github.com/matrix-org/matrix-rust-sdk/pull/6394))
- Latest Event does not emit an update when it computes the same value as the
  previous Latest Event.
  ([#6396](https://github.com/matrix-org/matrix-rust-sdk/pull/6396))
- Add support for pushing the backup key to other clients, and receiving a
  pushed backup key from other clients
  ([MSC4385](https://github.com/matrix-org/matrix-spec-proposals/pull/4385)),
  gated behind the `experimental-push-secrets` feature.
  ([#6432](https://github.com/matrix-org/matrix-rust-sdk/pull/6432))
- Add `room_versions()` & `account_moderation()` to `HomeserverCapabilities`.
  ([#6413](https://github.com/matrix-org/matrix-rust-sdk/pull/6413))
- Enable sending redaction events through the send queue via
  `RoomSendQueue::redact`. This includes local echoes for redaction events
  through the new `LocalEchoContent::Redaction` variant.
  ([#6250](https://github.com/matrix-org/matrix-rust-sdk/pull/6250))
- [**breaking**] Remove support for `native-tls` and remove all feature
  flags for selecting TLS backend, as `rustls` is the now the only supported
  TLS backend.
  ([#6409](https://github.com/matrix-org/matrix-rust-sdk/pull/6409))
- [**breaking**] Added `HomeserverCapabilities` and
  `Client::homeserver_capabilities()` to get the capabilities of the homeserver.
  This replaces `Client::get_capabilities()`.
  ([#6371](https://github.com/matrix-org/matrix-rust-sdk/pull/6371))
- [**breaking**] `matrix::error::Error` has a new variant `Timeout` which
  occurs when a cross-signing reset does not succeed after some period of time.
  ([#6325](https://github.com/matrix-org/matrix-rust-sdk/pull/6325))
- The `beacon_info` start event
  ([MSC3672](https://github.com/matrix-org/matrix-spec-proposals/pull/3672)) is
  now included when computing the latest event for a room, so live location
  sharing sessions can be surfaced as a room's most recent activity.
  ([#6295](https://github.com/matrix-org/matrix-rust-sdk/pull/6295))
- [**breaking**] The `EventCacheError` is now `Clone`able, which implied marking
  a few other error types as `Clone`able, and wrapping a few other error
  variants with `Arc`.
  ([#6305](https://github.com/matrix-org/matrix-rust-sdk/pull/6305))
- [**breaking**]: The unread count computation has now moved from the sliding
  sync processing, to the event cache. As a result, it is necessary to enable
  the event cache if you want to keep a precise unread counts, using
  `Client::event_cache().subscribe()`. The unread counts will now also be
  available even if you used a previous version of sync (v2), as long as you've
  enabled the event cache beforehand.
  ([#6253](https://github.com/matrix-org/matrix-rust-sdk/pull/6253))
- [**breaking**] `room::reply::Event` has a new field `add_mentions` which is
  passed forward in `room::reply::make_reply_event`.
  ([#6270](https://github.com/matrix-org/matrix-rust-sdk/pull/6270))
- Add `Recovery::recover_and_fix_backup` to automatically fix key storage backup
  if the private backup decryption key is missing, invalid or inconsistent with
  the public key.
  ([#6252](https://github.com/matrix-org/matrix-rust-sdk/pull/6252))
- Attempt to import stored room key bundles for rooms awaiting bundles at
  client startup.
  ([#6215](https://github.com/matrix-org/matrix-rust-sdk/pull/6215))
- Add `OAuth::cached_server_metadata()` that caches the authorization server
  metadata for a while.
  ([#6217](https://github.com/matrix-org/matrix-rust-sdk/pull/6217))
- Add `QRCodeGrantLoginError::SecureChannel` for secure channel errors
  ([#6141](https://github.com/matrix-org/matrix-rust-sdk/pull/6141)
- Add `QRCodeGrantLoginError::UnexpectedMessage` for protocol message errors
  ([#6141](https://github.com/matrix-org/matrix-rust-sdk/pull/6141)
- Add `QRCodeGrantLoginError::LoginFailure` for login failure errors received
  from the other device
  ([#6141](https://github.com/matrix-org/matrix-rust-sdk/pull/6141)
- Add `QRCodeGrantLoginError::DeviceNotFound` for when the requested device was
  not returned by the homeserver
  ([#6141](https://github.com/matrix-org/matrix-rust-sdk/pull/6141)
- Add `Client::subscribe_to_duplicate_key_upload_errors` for listening to
  duplicate key upload errors from `/keys/upload`.
  ([#6135](https://github.com/matrix-org/matrix-rust-sdk/pull/6135/))
- Add `Room::pin_event` and `Room::unpin_event`, which allow pinning and
  unpinning events from a room. These were extracted from the `ui`
  crate, with no changes in functionality.
  ([#6106](https://github.com/matrix-org/matrix-rust-sdk/pull/6106))
- `LatestEventValue::RemoteInvite` is added to handle a Latest Event for invite
  room. ([#6056](https://github.com/matrix-org/matrix-rust-sdk/pull/6056))
- Add `Room::set_own_member_display_name` to set the current user's display name
  within only the one single room (can be used for /myroomnick functionality).
  [#5981](https://github.com/matrix-org/matrix-rust-sdk/pull/5981)
- Sending `MessageLike` and `RawMessageLike` events through a `Room` now returns
  the used `EncryptionInfo`, if any.
  ([#5936](https://github.com/matrix-org/matrix-rust-sdk/pull/5936))
- [**breaking**]: The new Latest Event API replaces the old API. All the
  `new_` prefixes have been removed, thus `Room::new_latest_event` becomes
  and overwrites the `Room::latest_event` value. The new Latest Event values
  stored in `RoomInfo` are also erased once during the first update of the
  SDK. The new values will be re-calculated. The following types or functions
  are removed: `PossibleLatestEvent`, `is_suitable_for_latest_event`, and
  `LatestEvent` (replaced by `LatestEventValue`). See the documentation of
  `matrix::latest_event` to learn about the new API.
  ([#5624](https://github.com/matrix-org/matrix-rust-sdk/pull/5624/))
- Expose a new method `RoomEventCache::find_event_relations` for loading
  events relating to a specific event ID from the cache.
  ([#5930](https://github.com/matrix-org/matrix-rust-sdk/pull/5930/))
- Replace in-memory stores with IndexedDB implementations when initializing
  `Client` with `BuilderStoreConfig::IndexedDb`.
  ([#5946](https://github.com/matrix-org/matrix-rust-sdk/pull/5946))
- Call: Add support for the new Intents for voice only calls
  `Intent.StartCallDmVoice` and `Intent.JoinExistingDmVoice`.
  ([#6003](https://github.com/matrix-org/matrix-rust-sdk/pull/6003))
- Add `SlidingSync::unsubscribe_to_rooms` and
  `SlidingSync::clear_and_subscribe_to_rooms`.
  ([#6012](https://github.com/matrix-org/matrix-rust-sdk/pull/6012))
- [**breaking**] Sliding Sync has a new `PollTimeout` type, used by
  `SlidingSyncBuilder::requires_timeout`.
  ([#6005](https://github.com/matrix-org/matrix-rust-sdk/pull/6005))
- Inviting a user to a room with `Client::enable_share_history_on_invite` set
  to true will now trigger a download of all historical keys for the room in
  question from the client's key backup.
  ([#6017](https://github.com/matrix-org/matrix-rust-sdk/pull/6017))
- Add widget partial support for MSC4039. Allows widgets to download
  non-encrypted files from the content repository (like avatars).
  ([#6354](https://github.com/matrix-org/matrix-rust-sdk/pull/6354))

### Breaking changes

- [**breaking**] `LiveLocationShares` has been renamed to
  `LiveLocationsObserver` and `Room::live_location_shares` to
  `Room::live_locations_observer`.
  ([#6446](https://github.com/matrix-org/matrix-rust-sdk/pull/6446))

- `Room::observe_live_location_shares` has been replaced by
  `Room::live_locations_observer`. The new API returns a `LiveLocationsObserver`
  struct with a `subscribe()` method that provides an initial snapshot
  (`Vector<LiveLocationShare>`) and a batched stream of `VectorDiff` updates,
  instead of emitting individual `LiveLocationShare` items as beacon events
  arrive. The initial snapshot is loaded from the event cache on creation,
  includes the own user's shares (previously excluded), and properly handles
  share start/stop by listening to beacon_info state events.
  ([#6385](https://github.com/matrix-org/matrix-rust-sdk/pull/6385))

### Bugfix

- Add the `session` key in `OAuthCrossSigningResetInfo`, allowing to provide
  `AuthData::OAuth` in `CrossSigningResetHandle::auth()`, to match the behavior
  described in the Matrix spec.
  ([#6525](https://github.com/matrix-org/matrix-rust-sdk/pull/6525))
- When threads are enabled, a focused event timeline is used and the focused
  event is not part of a thread, hide other threaded events by default like it
  happens on the live focus timeline.
  ([#6519](https://github.com/matrix-org/matrix-rust-sdk/pull/6519))
- Add a recursion limit attribute that raises it from the default value of 128
  to 256. ([#6489](https://github.com/matrix-org/matrix-rust-sdk/pull/6489))
- Fix an infinite loop when loading pinned events from the storage.
  ([#6453](https://github.com/matrix-org/matrix-rust-sdk/pull/6453))
- `beacon_info` stop events (`live: false`,
  [MSC3672](https://github.com/matrix-org/matrix-spec-proposals/pull/3672)) are
  now also eligible as the latest event for a room, preventing the live location
  sharing item from disappearing from the room list summary once the session
  ends. ([#6373](https://github.com/matrix-org/matrix-rust-sdk/pull/6373))
- Android: add back custom certificates and disabling SSL verification options
  in `ClientBuilder` using the previous `webkpi` verifier instead of platform
  verifier, otherwise these features will fail.
  ([#6328](https://github.com/matrix-org/matrix-rust-sdk/pull/6328))
- Room keys are now rotated whenever the client receives an `m.room.member`
  event not belonging to the current user with non-`join` membership in order to
  prevent
  [MSC4268](https://github.com/matrix-org/matrix-spec-proposals/pull/4268) from
  leaking room keys in an unintuitive manner.
  ([#6292](https://github.com/matrix-org/matrix-rust-sdk/pull/6292))
  ([#6457](https://github.com/matrix-org/matrix-rust-sdk/pull/6457))
- Only share historic room keys on invite if the current room history is shared.
  ([#6275](https://github.com/matrix-org/matrix-rust-sdk/pull/6275))
- The event cache's thread subscriptions background task won't enable if the
  server doesn't advertise support for the experimental thread subscription
  feature. In the past, this would result in sending spurious requests that
  aren't supported by the user's homeserver.
  ([#6245](https://github.com/matrix-org/matrix-rust-sdk/pull/6245))
- Handle race between send queue update and remote echo in latest event
  computation.
  ([#6220](https://github.com/matrix-org/matrix-rust-sdk/pull/6220))
- Return `QRCodeGrantLoginError::DeviceNotFound` instead of
  `QRCodeGrantLoginError::DeviceIDAlreadyInUse` for when the new device is not
  returned by the homeserver.
  ([#6141](https://github.com/matrix-org/matrix-rust-sdk/pull/6141)
- Latest Event is correctly computed when multiple edits exist for the same
  event candidate.
  ([#6096](https://github.com/matrix-org/matrix-rust-sdk/pull/6096))
- Restrict which `m.room.member` can be a `LatestEventValue` candidate by
  relying on `MembershipChange` for more control.
  ([#6143](https://github.com/matrix-org/matrix-rust-sdk/pull/6143))
- Add manual WAL checkpoints when opening Sqlite DBs and when vacuuming them,
  since the WAL files aren't automatically shrinking.
  ([#6004](https://github.com/matrix-org/matrix-rust-sdk/pull/6004))
- Use the server name extracted from the user id in
  `Client::fetch_client_well_known` as a fallback value. Otherwise, sometimes
  the server name is not available and we can't reload the well-known contents.
  ([#5996](https://github.com/matrix-org/matrix-rust-sdk/pull/5996))
- Latest Event is lazier: a `RoomLatestEvents` can be registered even if its
  associated `RoomEventCache` isn't created yet.
  ([#5947](https://github.com/matrix-org/matrix-rust-sdk/pull/5947))
- Allow granting of QR login to a new client whose device ID is not a base64
  encoded Curve25519 public key.
  ([#5940](https://github.com/matrix-org/matrix-rust-sdk/pull/5940))
- Remove an unwrap in `SlidingSync::send_sync_request` when an asynchronous task
  panics or is canceled.
  ([#6316](https://github.com/matrix-org/matrix-rust-sdk/pull/6316))

### Refactor

- [**breaking**] Upgrade Ruma to 0.15.1.
  ([#6503](https://github.com/matrix-org/matrix-rust-sdk/pull/6503))
- Revert back to determining lock dirtiness in
  `Encryption::{spin_lock_store, try_lock_once_store}` through logic defined in
  `OlmMachine`, rather than `CrossProcessLock`.
  ([#6496](https://github.com/matrix-org/matrix-rust-sdk/pull/6496))
- [**breaking**] Update `Encryption::{spin_lock_store, try_lock_once_store}` so
  that lock dirtiness is determined entirely by `CrossProcessLock`, rather than
  logic defined by `OlmMachine`. Also enforce that lock generation is opaque by
  removing `CrossProcessLockStoreGuardWithGeneration`.
  ([#6326](https://github.com/matrix-org/matrix-rust-sdk/pull/6326))
- [**breaking**] The `EventCache` now owns pagination tasks, and will run them
  to completion, even if a manual caller stopped polling the called future.
  ([#6304](https://github.com/matrix-org/matrix-rust-sdk/pull/6304))
- [**breaking**] `RoomEventCache::thread_pagination` is now async and fallible.
  ([#6280](https://github.com/matrix-org/matrix-rust-sdk/pull/6280))
- [**breaking**] The `UrlOrQuery` enum was moved from the
  `authentication::oauth` module to the `utils` module. It can also be converted
  from a `QueryString`.
  ([#6224](https://github.com/matrix-org/matrix-rust-sdk/pull/6224))
- [**breaking**] `MatrixAuth::login_with_sso_callback()` takes a `UrlOrQuery`
  instead of a `Url`, to make it more convenient to use with
  `LocalServerBuilder` / `LocalServerRedirectHandle`.
  ([#6224](https://github.com/matrix-org/matrix-rust-sdk/pull/6224))
- [**breaking**] `Room::report_content()` no longer takes a `score` argument,
  because it was removed from the Matrix specification. The
  `ReportedContentScore` type was removed too.
  ([#6256](https://github.com/matrix-org/matrix-rust-sdk/pull/6256))
- [**breaking**] `Client::enabled_thread_subscriptions()` is now async and
  fallible, as it will check for both static enablement of the thread
  subscription feature as well as dynamically checking that the user's
  homeserver supports it.
- [**breaking**] `SessionChange::UnknownToken` is now a tuple variant containing
  an `UnknownTokenErrorData`.
  ([#6241](https://github.com/matrix-org/matrix-rust-sdk/pull/6241))
- [**breaking**] `EventCacheError::BackPaginationError` has been renamed
  `PaginationError`.
  ([#6239](https://github.com/matrix-org/matrix-rust-sdk/pull/6239))
- [**breaking**] The functions on the `OAuth` API to access the account
  management URL and its actions were removed. The methods available on the
  `AuthorizationServerMetadata` should be used instead.
  ([#6217](https://github.com/matrix-org/matrix-rust-sdk/pull/6217))
- [**breaking**] `QRCodeGrantLoginError::UnableToCreateDevice` has been removed
  ([#6141](https://github.com/matrix-org/matrix-rust-sdk/pull/6141)
- The `RoomEventCache::paginate_thread_backwards` method is replaced by
  `RoomEventCache::thread_pagination` which returns a new `ThreadPagination`
  type, similar to `RoomPagination`.
  ([#6174](https://github.com/matrix-org/matrix-rust-sdk/pull/6174))

  Before:

  ```rust
  room_event_cache.paginate_thread_backwards(thread_id, 42).await
  ```

  After:

  ```rust
  room_event_cache.thread_pagination(thread_id).run_backwards_once(42).await
  ```

- `RoomPaginationStatus` is renamed to `PaginationStatus`.
  ([#6174](https://github.com/matrix-org/matrix-rust-sdk/pull/6174/))
- [**breaking**] Replaced `ClientBuilder::cross_process_store_locks_holder_name`
  with `ClientBuilder::cross_process_store_config` to allow specifying the
  configuration for the cross-process lock and whether it should act as a no-op
  (client used in a single process) or we should keep the previous behavior
  (client used in multiple processes).
  ([#6160](https://github.com/matrix-org/matrix-rust-sdk/pull/6160))

## [0.16.1] - 2026-05-08

- Add a recursion limit attribute that raises it from the default value of 128
  to 256. ([#6489](https://github.com/matrix-org/matrix-rust-sdk/pull/6489))
- Reject invalid edits as candidates for the latest event.
  ([#6454](https://github.com/matrix-org/matrix-rust-sdk/pull/6454))

## [0.16.0] - 2025-12-04

### Features

- Add `Client::get_store_sizes()` so to query the size of the existing stores,
  if available.
  ([#5911](https://github.com/matrix-org/matrix-rust-sdk/pull/5911))
- Add `QRCodeLoginError::NotFound` for non-existing / expired rendezvous
  sessions ([#5898](https://github.com/matrix-org/matrix-rust-sdk/pull/5898))
- Add `QRCodeGrantLoginError::NotFound` for non-existing / expired rendezvous
  sessions ([#5898](https://github.com/matrix-org/matrix-rust-sdk/pull/5898))
- Improve logging around key history bundles when joining a room.
  ([#5866](https://github.com/matrix-org/matrix-rust-sdk/pull/5866))
- Expose the power level required to modify `m.space.child` on
  `room::power_levels::RoomPowerLevelChanges`.
  ([#5857](https://github.com/matrix-org/matrix-rust-sdk/pull/5857))
- Add the `Client::server_versions_cached()` method.
  ([#5853](https://github.com/matrix-org/matrix-rust-sdk/pull/5853))
- Extend `authentication::oauth::OAuth::grant_login_with_qr_code` to support
  granting login by scanning a QR code on the existing device.
  ([#5818](https://github.com/matrix-org/matrix-rust-sdk/pull/5818))
- Add a new `RequestConfig::skip_auth()` option. This is useful to ensure that
  certain request won't ever include an authorization header.
  ([#5822](https://github.com/matrix-org/matrix-rust-sdk/pull/5822))
- Add support for extended profile fields with
  `Account::fetch_profile_field_of()`,
  `Account::fetch_profile_field_of_static()`, `Account::set_profile_field()` and
  `Account::delete_profile_field()`.
  ([#5771](https://github.com/matrix-org/matrix-rust-sdk/pull/5771))
- [**breaking**] Remove the `crypto` re-export.
  ([#5769](https://github.com/matrix-org/matrix-rust-sdk/pull/5769))
- Allow `Client::get_dm_room()` to be called without the `e2e-encryption` crate
  feature. ([#5787](https://github.com/matrix-org/matrix-rust-sdk/pull/5787))
- [**breaking**] Add
  `encryption::secret_storage::SecretStorageError::ImportError` to indicate an
  error that occurred when importing a secret from secret storage.
  ([#5647](https://github.com/matrix-org/matrix-rust-sdk/pull/5647))
- [**breaking**] Add
  `authentication::oauth::qrcode::login::LoginProgress::SyncingSecrets` to
  indicate that secrets are being synced between the two devices.
  ([#5760](https://github.com/matrix-org/matrix-rust-sdk/pull/5760))
- Add `authentication::oauth::OAuth::grant_login_with_qr_code` to reciprocate a
  login by generating a QR code on the existing device.
  ([#5801](https://github.com/matrix-org/matrix-rust-sdk/pull/5801))
- [**breaking**] `OAuth::login_with_qr_code` now returns a builder that allows
  performing the flow with either the current device scanning or generating the
  QR code. Additionally, new errors `SecureChannelError::CannotReceiveCheckCode`
  and `QRCodeLoginError::ServerReset` were added.
  ([#5711](https://github.com/matrix-org/matrix-rust-sdk/pull/5711))
- [**breaking**] `ThreadedEventsLoader::new` now takes optional `tokens`
  parameter to customise where the pagination begins
  ([#5678](https://github.com/matrix-org/matrix-rust-sdk/pull/5678).
- Make `PaginationTokens` `pub`, as well as its `previous` and `next` tokens so
  they can be assigned from other files
  ([#5678](https://github.com/matrix-org/matrix-rust-sdk/pull/5678).
- Add new API to decline calls
  ([MSC4310](https://github.com/matrix-org/matrix-spec-proposals/pull/4310)):
  `Room::make_decline_call_event` and `Room::subscribe_to_call_decline_events`
  ([#5614](https://github.com/matrix-org/matrix-rust-sdk/pull/5614))
- Use `StateStore::upsert_thread_subscriptions()` to bulk process thread
  subscription updates received via the sync response or from the MSC4308
  companion endpoint.
  ([#5848](https://github.com/matrix-org/matrix-rust-sdk/pull/5848))

### Refactor

- [**breaking**]: `Client::server_vendor_info()` requires to enable the
  `federation-api` feature.
  ([#5912](https://github.com/matrix-org/matrix-rust-sdk/pull/5912))
- [**breaking**]: `Client::reset_server_info()` has been split into
  `reset_supported_versions()` and `reset_well_known()`.
  ([#5910](https://github.com/matrix-org/matrix-rust-sdk/pull/5910))
- [**breaking**]: `Client::send()` has extra bounds where
  `Request::Authentication: AuthScheme<Input<'a> = SendAccessToken<'a>>` and
  `Request::PathBuilder: SupportedPathBuilder`. This method should still work
  for any request to the Client-Server API. This allows to drop the
  `HttpError::NotClientRequest` error in favor of a compile-time error.
  ([#5781](https://github.com/matrix-org/matrix-rust-sdk/pull/5781),
  [#5789](https://github.com/matrix-org/matrix-rust-sdk/pull/5789),
  [#5815](https://github.com/matrix-org/matrix-rust-sdk/pull/5815))
- [**breaking**]: The `waveform` field was moved from `AttachmentInfo::Voice` to
  `BaseAudioInfo`, allowing to set it for any audio message. Its format also
  changed, and it is now a list of `f32` between 0 and 1.
  ([#5732](https://github.com/matrix-org/matrix-rust-sdk/pull/5732))
- [**breaking**] The `caption` and `formatted_caption` fields and methods of
  `AttachmentConfig`, `GalleryConfig` and `GalleryItemInfo` have been merged
  into a single field that uses `TextMessageEventContent`.
  ([#5733](https://github.com/matrix-org/matrix-rust-sdk/pull/5733))
- The Matrix SDK crate now uses the 2024 edition of Rust.
  ([#5677](https://github.com/matrix-org/matrix-rust-sdk/pull/5677))
- [**breaking**] Make `LoginProgress::EstablishingSecureChannel` generic in
  order to reuse it for the currently missing QR login flow.
  ([#5750](https://github.com/matrix-org/matrix-rust-sdk/pull/5750))
- [**breaking**] The `new_virtual_element_call_widget` now uses a `props` and a
  `config` parameter instead of only `props`. This splits the configuration of
  the widget into required properties ("widget_id", "parent_url"...) so the
  widget can work and optional config parameters ("skip_lobby", "header",
  "..."). The config option should in most cases only provide the `"intent"`
  property. All other config options will then be chosen by EC based on platform
  - `intent`.

  Before:

  ```rust
  new_virtual_element_call_widget(
    VirtualElementCallWidgetProperties {
      widget_id: "my_widget_id", // required property
      skip_lobby: Some(true), // optional configuration
      preload: Some(true), // optional configuration
      // ...
    }
  )
  ```

  Now:

  ```rust
  new_virtual_element_call_widget(
    VirtualElementCallWidgetProperties {
      widget_id: "my_widget_id", // required property
      // ... only required properties
    },
    VirtualElementCallWidgetConfig {
      intend: Intend.StartCallDM, // defines the default values for all other configuration
      skip_lobby: Some(false), // overwrite a specific default value
      ..VirtualElementCallWidgetConfig::default() // set all other config options to `None`. Use defaults from intent.
    }
  )
  ```

  ([#5560](https://github.com/matrix-org/matrix-rust-sdk/pull/5560))

### Bugfix

- A new local `LatestEventValue` was always created as `LocalIsSending`. It
  must be created as `LocalCannotBeSent` if a previous local `LatestEventValue`
  exists and is `LocalCannotBeSent`.
  ([#5908](https://github.com/matrix-org/matrix-rust-sdk/pull/5908))
- Switch QR login implementation from `std::time::Instant` to
  `ruma::time::Instant` which is compatible with Wasm.
  ([#5889](https://github.com/matrix-org/matrix-rust-sdk/pull/5889))

## [0.14.0] - 2025-09-04

### Features

- `Client::fetch_thread_subscriptions` implements support for the companion
  endpoint of the experimental MSC4308, allowing to fetch thread subscriptions
  for a given range, as specified by the MSC.
  ([#5590](https://github.com/matrix-org/matrix-rust-sdk/pull/5590))
- Add a `Client::joined_space_rooms` method that allows retrieving the list of
  joined spaces.
  ([#5592](https://github.com/matrix-org/matrix-rust-sdk/pull/5592))
- `Room::enable_encryption` and
  `Room::enable_encryption_with_state_event_encryption` will poll the encryption
  state for up to 3 seconds, rather than checking once after a single sync has
  completed. ([#5559](https://github.com/matrix-org/matrix-rust-sdk/pull/5559))
- Add `Room::enable_encryption_with_state` to enable E2E encryption with
  encrypted state event support, gated behind the
  `experimental-encrypted-state-events` feature.
  ([#5557](https://github.com/matrix-org/matrix-rust-sdk/pull/5557))
- Add `ignore_timeout_on_first_sync` to the `SyncSettings`, which should allow
  to have a quicker first response when using one of the `sync`,
  `sync_with_callback`, `sync_with_result_callback` or `sync_stream` methods on
  `Client`, if the response is empty.
  ([#5481](https://github.com/matrix-org/matrix-rust-sdk/pull/5481))
- The methods to use the `/v3/sync` endpoint set the `use_state_after` field,
  which means that, if the server supports it, the response will contain the
  state changes between the last sync and the end of the timeline.
  ([#5488](https://github.com/matrix-org/matrix-rust-sdk/pull/5488))
- Add experimental support for
  [MSC4306](https://github.com/matrix-org/matrix-spec-proposals/pull/4306), with
  the `Room::fetch_thread_subscription()`, `Room::subscribe_thread()` and
  `Room::unsubscribe_thread()` methods.
  ([#5439](https://github.com/matrix-org/matrix-rust-sdk/pull/5439))
- [**breaking**] `RoomMemberRole` has a new `Creator` variant, that
  differentiates room creators with infinite power levels, as introduced in room
  version 12.
  ([#5436](https://github.com/matrix-org/matrix-rust-sdk/pull/5436))
- Add `Account::fetch_account_data_static` to fetch account data from the server
  with a statically-known type, with a signature similar to
  `Account::account_data`.
  ([#5424](https://github.com/matrix-org/matrix-rust-sdk/pull/5424))
- Add support to accept historic room key bundles that arrive out of order, i.e.
  the bundle arrives after the invite has already been accepted.
  ([#5322](https://github.com/matrix-org/matrix-rust-sdk/pull/5322))
- [**breaking**] `OAuth::login` now allows requesting additional scopes for the
  authorization code grant.
  ([#5395](https://github.com/matrix-org/matrix-rust-sdk/pull/5395))

### Refactor

- [**breaking**] Upgrade ruma to 0.13.0
  ([#5623](https://github.com/matrix-org/matrix-rust-sdk/pull/5623))
- [**breaking**] `SyncSettings` token is now `SyncToken` enum type which has
  default behaviour of `SyncToken::ReusePrevious` token. This breaks
  `Client::sync_once`. For old behaviour, set the token to `SyncToken::NoToken`
  with the usual `SyncSettings::token` setter.
  ([#5522](https://github.com/matrix-org/matrix-rust-sdk/pull/5522))
- [**breaking**] Change the upload_encrypted_file and make it clone the client
  instead of owning it. The lifetime of the `UploadEncryptedFile` request
  returned by `Client::upload_encrypted_file()` only depends on the request
  lifetime now.
  ([#5470](https://github.com/matrix-org/matrix-rust-sdk/pull/5470))
- [**breaking**] Add an `IsPrefix = False` bound to the `account_data()` and
  `fetch_account_data_static()` methods of `Account`. These methods only worked
  for events where the full event type is statically-known, and this is now
  enforced at compile-time. `account_data_raw()` and `fetch_account_data()`
  respectively can be used instead for event types with a variable suffix.
  ([#5444](https://github.com/matrix-org/matrix-rust-sdk/pull/5444))
- [**breaking**] `RoomMemberRole::suggested_role_for_power_level()` and
  `RoomMemberRole::suggested_power_level()` now use `UserPowerLevel` to
  represent power levels instead of `i64` to differentiate the infinite power
  level of creators, as introduced in room version 12.
  ([#5436](https://github.com/matrix-org/matrix-rust-sdk/pull/5436))
- [**breaking**] The `reason` argument of `Room::report_room()` is now required,
  due to a clarification in the spec.
  ([#5337](https://github.com/matrix-org/matrix-rust-sdk/pull/5337))
- [**breaking**] The `join_rule` field of `RoomPreview` is now a
  `JoinRuleSummary`. It has the same variants as `SpaceRoomJoinRule` but
  contains as summary of the allow rules for the restricted variants.
  ([#5337](https://github.com/matrix-org/matrix-rust-sdk/pull/5337))
- [**breaking**] The MSRV has been bumped to Rust 1.88.
  ([#5431](https://github.com/matrix-org/matrix-rust-sdk/pull/5431))
- [**breaking**] `Room::send_call_notification` and
  `Room::send_call_notification_if_needed` have been removed, since the event
  type they send is outdated, and `Client` is not actually supposed to be able
  to join MatrixRTC sessions (yet). In practice, users of these methods probably
  already rely on another MatrixRTC implementation to participate in sessions,
  and such an implementation should be capable of sending notifications itself.
  ([#5452](https://github.com/matrix-org/matrix-rust-sdk/pull/5452))

### Bugfix

- The event handlers APIs now properly support events whose type is not fully
  statically-known. Before, those events would never trigger an event handler.
  ([#5444](https://github.com/matrix-org/matrix-rust-sdk/pull/5444))
- All HTTP requests now have a default `read_timeout` of 60s, which means
  they'll disconnect if the connection stalls.
`RequestConfig::timeout` is now optional and can be disabled on a per-request
basis. This will be done for the requests used to download media, so they don't
get cancelled after the default 30s timeout for no good reason.
([#5437](https://github.com/matrix-org/matrix-rust-sdk/pull/5437))

## [0.13.0] - 2025-07-10

### Security fixes

- Fix SQL injection vulnerability in `EventCache`
  ([d0c0100](https://github.com/matrix-org/matrix-rust-sdk/commit/d0c01006e4808db5eb96ad5c496416f284d8bd3c),
  Moderate, [CVE-2025-53549](https://www.cve.org/CVERecord?id=CVE-2025-53549),
  [GHSA-275g-g844-73jh](https://github.com/matrix-org/matrix-rust-sdk/security/advisories/GHSA-275g-g844-73jh))

### Bug fixes

- `Room.leave()` will now attempt to leave all reachable predecessors too.
  ([#5381](https://github.com/matrix-org/matrix-rust-sdk/pull/5381))
- When joining a room via `Client::join_room_by_id()`, if the client has
  `enable_share_history_on_invite` enabled, we will correctly check for received
  room key bundles. Previously this was only done when calling `Room::join`.
  ([#5043](https://github.com/matrix-org/matrix-rust-sdk/pull/5043))
- `m.room.avatar` has been added as required state for sliding sync until
  [the existing backend issue](https://github.com/element-hq/synapse/issues/18598)
causing deleted room avatars to not be flagged is fixed.
([#5293](https://github.com/matrix-org/matrix-rust-sdk/pull/5293))

### Features

- Add `Client::supported_versions()`, which returns the results of both
  `Client::server_versions()` and `Client::unstable_features()` with a single
  call. ([#5357](https://github.com/matrix-org/matrix-rust-sdk/pull/5357))
- `WidgetDriver::send_to_device` Now supports sending encrypted to-device
  messages. ([#5252](https://github.com/matrix-org/matrix-rust-sdk/pull/5252))
- `Client::add_event_handler`: Set `Option<EncryptionInfo>` in
  `EventHandlerData` for to-device messages. If the to-device message was
  encrypted, the `EncryptionInfo` will be set. If it is `None` the message was
  sent in clear.
  ([#5099](https://github.com/matrix-org/matrix-rust-sdk/pull/5099))
- `EventCache::subscribe_to_room_generic_updates` is added to subscribe to _all_
  room updates without having to subscribe to all rooms individually
  ([#5247](https://github.com/matrix-org/matrix-rust-sdk/pull/5247))
- [**breaking**] The element call widget URL configuration struct uses the new
  `header` url parameter instead of the now deprecated `hideHeader` parameter.
  This is only compatible with EC v0.13.0 or newer.
- [**breaking**] `RoomEventCacheGenericUpdate` gains a new `Clear` variant, and
  sees its `TimelineUpdated` variant being renamed to `UpdateTimeline`.
  ([#5363](https://github.com/matrix-org/matrix-rust-sdk/pull/5363/))
- [**breaking**]: The element call widget URL configuration struct uses the new
  `header` url parameter instead of the now deprecated `hideHeader` parameter.
  This is only compatible with EC v0.13.0 or newer.
- [**breaking**]: The experimental `Encryption::encrypt_and_send_raw_to_device`
  function now takes a `share_strategy` parameter, and will not send to devices
  that do not satisfy the given share strategy.
  ([#5457](https://github.com/matrix-org/matrix-rust-sdk/pull/5457/))

### Refactor

- [**breaking**]: `Client::unstable_features()` returns a
  `BTreeSet<FeatureFlag>`, containing only the features whose value was set to
  true in the response to the `/versions` endpoint.
  ([#5357](https://github.com/matrix-org/matrix-rust-sdk/pull/5357))
- [**breaking**]: The family of `Room::can_user_*` methods has been removed. The
  same functionality can be accessed using the `RoomPowerLevels::user_can_*`
  family of methods. The `RoomPowerLevels` object can be accessed using the
  `Room::power_levels()` method.
  ([#5250](https://github.com/matrix-org/matrix-rust-sdk/pull/5250/))
- `ClientServerCapabilities` has been renamed to `ClientServerInfo`. Alongside
  this, `Client::reset_server_info` is now `Client::reset_server_info` and
  `Client::fetch_server_capabilities` is now `Client::fetch_server_versions`,
  returning the server versions response directly.
  ([#5167](https://github.com/matrix-org/matrix-rust-sdk/pull/5167))
- `RoomEventCacheListener` is renamed `RoomEventCacheSubscriber`
  ([#5269](https://github.com/matrix-org/matrix-rust-sdk/pull/5269))
- `RoomPreview::join_rule` is now optional, and will be set to `None` if the
  join rule state event is missing for a given room.
  ([#5278](https://github.com/matrix-org/matrix-rust-sdk/pull/5278))

## [0.12.0] - 2025-06-10

### Features

- `Client::send_call_notification_if_needed` now returns `Result<bool>` instead
  of `Result<()>` so we can check if the event was sent.
  ([#5171](https://github.com/matrix-org/matrix-rust-sdk/pull/5171))
- Added `SendMediaUploadRequest` wrapper for `SendRequest`, which checks the
  size of the request to upload making sure it doesn't exceed the
  `m.upload.size` value that can be fetched through
  `Client::load_or_fetch_max_upload_size`.
  ([#5119](https://github.com/matrix-org/matrix-rust-sdk/pull/5119))
- Add `ClientBuilder::with_enable_share_history_on_invite` to enable
  experimental support for sharing encrypted room history on invite, per
  [MSC4268](https://github.com/matrix-org/matrix-spec-proposals/pull/4268).
  ([#5141](https://github.com/matrix-org/matrix-rust-sdk/pull/5141))
- `Room::list_threads()` is a new method to list all the threads in a room.
  ([#4973](https://github.com/matrix-org/matrix-rust-sdk/pull/4973))
- `Room::relations()` is a new method to list all the events related to another
  event ("relations"), with additional filters for relation type or relation
  type + event type.
  ([#4973](https://github.com/matrix-org/matrix-rust-sdk/pull/4973))
- The `EventCache`'s persistent storage has been enabled by default. This means
  that all the events received by sync or back-paginations will be stored, in
  memory or on disk, by default, as soon as `EventCache::subscribe()` has been
  called (which happens automatically if you're using the
  `ui::Timeline`). This offers offline access and super quick
  back-paginations (when the cache has been filled) whenever the event cache is
  enabled. It's also not possible to disable the persistent storage anymore.
  Note that by default, the event cache store uses an in-memory store, so the
  events will be lost when the process exits. To store the events on disk, you
  need to use the sqlite event cache store.
  ([#4308](https://github.com/matrix-org/matrix-rust-sdk/pull/4308))
- `Room::set_unread_flag()` now sets the stable `m.marked_unread` room account
  data, which was stabilized in Matrix 1.12. `Room::is_marked_unread()` also
  ignores the unstable `com.famedly.marked_unread` room account data if the
  stable variant is present.
  ([#5034](https://github.com/matrix-org/matrix-rust-sdk/pull/5034))
- `Encryption::encrypt_and_send_raw_to_device`: Introduced as an experimental
  method for sending custom encrypted to-device events. This feature is gated
  behind the `experimental-send-custom-to-device` flag, as it remains under
  active development and may undergo changes.
  ([4998](https://github.com/matrix-org/matrix-rust-sdk/pull/4998))
- `Room::send_single_receipt()` and `Room::send_multiple_receipts()` now also
  unset the unread flag of the room if an unthreaded read receipt is sent.
  ([#5055](https://github.com/matrix-org/matrix-rust-sdk/pull/5055))
- `Client::is_user_ignored(&UserId)` can be used to check if a user is currently
  ignored. ([#5081](https://github.com/matrix-org/matrix-rust-sdk/pull/5081))
- `RoomSendQueue::send_gallery` has been added to allow sending MSC4274-style
  media galleries via the send queue under the `unstable-msc4274` feature.
  ([#4977](https://github.com/matrix-org/matrix-rust-sdk/pull/4977))

### Bug fixes

- A invited DM room joined with `Client::join_room_by_id()` or
  `Client::join_room_by_id_or_alias()` will now be correctly marked as a DM.
  ([#5043](https://github.com/matrix-org/matrix-rust-sdk/pull/5043))
- API responses with an HTTP status code `520` won't be retried anymore, as this
  is used by some proxies (including Cloudflare) to warn that an unknown error
  has happened in the actual server.
  ([#5105](https://github.com/matrix-org/matrix-rust-sdk/pull/5105))

### Refactor

- Support for the deprecated `GET /auth_issuer` endpoint was removed in the
  `OAuth` API. Only the `GET /auth_metadata` endpoint is used now.
  ([#5302](https://github.com/matrix-org/matrix-rust-sdk/pull/5302))
- `Room::push_context()` has been renamed into
  `Room::push_condition_room_ctx()`. The newer `Room::push_context` now returns
  a `matrix::Room::PushContext`, which can be used to compute the push
  actions for any event.
  ([#4962](https://github.com/matrix-org/matrix-rust-sdk/pull/4962))
- `Room::decrypt_event()` now requires an extra `matrix::Room::PushContext`
  parameter to compute the push notifications for the decrypted event.
  ([#4962](https://github.com/matrix-org/matrix-rust-sdk/pull/4962))
- `SlidingSyncRoom` has been removed. With it, the `SlidingSync::get_room`,
  `get_all_rooms`, `get_rooms`, `get_number_of_rooms`, and
  `FrozenSlidingSync` methods and type have been removed.
  ([#5047](https://github.com/matrix-org/matrix-rust-sdk/pull/5047))
- `Room::set_unread_flag()` is now a no-op if the unread flag already has the
  wanted value.
  ([#5055](https://github.com/matrix-org/matrix-rust-sdk/pull/5055))

## [0.11.0] - 2025-04-11

### Features

- `Room::load_or_fetch_event()` is a new method that will find an event in the
  event cache (if enabled), or using network like `Room::event()` does.
  ([#4837](https://github.com/matrix-org/matrix-rust-sdk/pull/4837))
- [**breaking**]: The element call widget URL configuration struct
  (`VirtualElementCallWidgetOptions`) and URL generation have changed.
  - It supports the new fields: `hide_screensharing`, `posthog_api_host`,
    `posthog_api_key`,
  `rageshake_submit_url`, `sentry_dsn`, `sentry_environment`.
  - The widget URL will no longer automatically add `/room` to the base domain.
    For backward compatibility
  the app itself would need to add `/room` to the `element_call_url`.
  - And replaced:
    - `analytics_id` -> `posthog_user_id` (The widget URL query parameters will
      include `analytics_id` & `posthog_user_id` for backward compatibility)
    - `skip_lobby` -> `intent` (`Intent.StartCall`, `Intent.JoinExisting`.
      The widget URL query parameters will include `skip_lobby` if `intent` is
      `Intent.StartCall` for backward compatibility)
  - `VirtualElementCallWidgetOptions` now implements `Default`.
  ([#4822](https://github.com/matrix-org/matrix-rust-sdk/pull/4822))
- [**breaking**]: The `RoomPagination::run_backwards` method has been removed
  and replaced by two
simpler methods:
  - `RoomPagination::run_backwards_until()`, which will retrigger
    back-paginations until a certain number of events have been received (and
    retry if the timeline has been reset in the background).
  - `RoomPagination::run_backwards_once()`, which will run a single
    back-pagination (and retry if
  the timeline has been reset in the background).
  ([#4689](https://github.com/matrix-org/matrix-rust-sdk/pull/4689))
- [**breaking**]: The `OAuth::account_management_url` method now caches the
  result of a call, subsequent calls to the method will not contact the server
  for a while, instead the cached URI will be returned. If caching of this URI
  is not desirable, the `OAuth::fetch_account_management_url` method can be
  used. ([#4663](https://github.com/matrix-org/matrix-rust-sdk/pull/4663))
- The `MediaRetentionPolicy` can now trigger regular cleanups with its new
  `cleanup_frequency` setting.
  ([#4603](https://github.com/matrix-org/matrix-rust-sdk/pull/4603))
- [**breaking**] The HTTP client only allows TLS 1.2 or newer, as recommended by
  [BCP 195](https://datatracker.ietf.org/doc/bcp195/).
  ([#4647](https://github.com/matrix-org/matrix-rust-sdk/pull/4647))
- Add `Room::report_room` api.
  ([#4713](https://github.com/matrix-org/matrix-rust-sdk/pull/4713))
- `Client::notification_client` will create a copy of the existing `Client`,
  but now it'll make sure it doesn't handle any verification events to
  avoid an issue with these events being received and processed twice if
  `NotificationProcessSetup` was `SingleSetup`.
- [**breaking**] `Room::is_encrypted` is replaced by
  `Room::latest_encryption_state` which returns a value of the new
  `EncryptionState` enum; another `Room::encryption_state` non-async and
  infallible method is added to get the `EncryptionState` without calling
  `Room::request_encryption_state`. This latter method is also now public.
  ([#4777](https://github.com/matrix-org/matrix-rust-sdk/pull/4777)). One can
  safely replace:

  ```rust
  room.is_encrypted().await?
  ```

  by

  ```rust
  room.latest_encryption_state().await?.is_encrypted()
  ```

- `LocalServerBuilder`, behind the `local-server` feature, can be used to spawn
  a server when the end-user needs to be redirected to an address on localhost.
  It was used for `SsoLoginBuilder` and can now be used in other cases, like for
  login with the OAuth 2.0 API.
  ([#4804](https://github.com/matrix-org/matrix-rust-sdk/pull/4804)
- The `OAuth` api is no longer gated behind the `experimental-oidc` cargo
  feature.
  ([#4830](https://github.com/matrix-org/matrix-rust-sdk/pull/4830))
- Re-export `SqliteStoreConfig` and add
  `ClientBuilder::sqlite_store_with_config_and_cache_path` to configure the
  SQLite store with the new `SqliteStoreConfig` structure
  ([#4870](https://github.com/matrix-org/matrix-rust-sdk/pull/4870))
- Add `Client::logout()` that allows to log out regardless of the `AuthApi` that
  is used for the session.
  ([#4886](https://github.com/matrix-org/matrix-rust-sdk/pull/4886))

### Bug fixes

- Ensure all known secrets are removed from secret storage when invoking the
  `Recovery::disable()` method. While the server is not guaranteed to delete
  these secrets, making an attempt to remove them is considered good practice.
  Note that all secrets are uploaded to the server in an encrypted form.
  ([#4629](https://github.com/matrix-org/matrix-rust-sdk/pull/4629))
- Most of the features in the `OAuth` API should now work under WASM
  ([#4830](https://github.com/matrix-org/matrix-rust-sdk/pull/4830))

### Refactor

- [**breaking**] Switched from the unmaintained backoff crate to the
  [backon](https://docs.rs/backon/1.5.0/backon/) crate. As part of this change,
  the `RequestConfig::retry_limit` method was renamed to
  `RequestConfig::max_retry_time` and the parameter for the method was updated
  from a `u64` to a `usize`.
  ([#4916](https://github.com/matrix-org/matrix-rust-sdk/pull/4916))
- [**breaking**] We now require Rust 1.85 as the minimum supported Rust version
  to compile. Yay for async closures!
  ([#4745](https://github.com/matrix-org/matrix-rust-sdk/pull/4745))
- [**breaking**] The `server_url` and `server_response` methods of
  `SsoLoginBuilder` are replaced by `server_builder()`, which allows more
  fine-grained settings for the server.
  ([#4804](https://github.com/matrix-org/matrix-rust-sdk/pull/4804)
- [**breaking**]: `OidcSessionTokens` and `MatrixSessionTokens` have been merged
  into `SessionTokens`. Methods to get and watch session tokens are now
  available directly on `Client`.
  `(MatrixAuth/Oidc)::session_tokens_stream()`, can be replaced by
  `Client::subscribe_to_session_changes()` and then calling
  `Client::session_tokens()` on a `SessionChange::TokenRefreshed`.
  ([#4772](https://github.com/matrix-org/matrix-rust-sdk/pull/4772))
- [**breaking**] `Oidc::url_for_oidc()` doesn't take the
  `VerifiedClientMetadata` to register as an argument, the one in
  `OidcRegistrations` is used instead. However it now takes the redirect URI to
  use, instead of always using the first one in the client metadata.
  ([#4771](https://github.com/matrix-org/matrix-rust-sdk/pull/4771))
- [**breaking**] The `server_url` and `server_response` methods of
  `SsoLoginBuilder` are replaced by `server_builder()`, which allows more
  fine-grained settings for the server.
- [**breaking**]: Rename the `Oidc` API to `OAuth`, since it's using almost
  exclusively OAuth 2.0 rather than OpenID Connect.
  ([#4805](https://github.com/matrix-org/matrix-rust-sdk/pull/4805))
  - The `oidc` module was renamed to `oauth`.
  - `Client::oidc()` was renamed to `Client::oauth()` and the `AuthApi::Oidc`
    variant was renamed to `AuthApi::OAuth`.
  - `OidcSession` was renamed to `OAuthSession` and the `AuthSession::Oidc`
    variant was renamed to `AuthSession::OAuth`.
  - `OidcAuthCodeUrlBuilder` and `OidcAuthorizationData` were renamed to
    `OAuthAuthCodeUrlBuilder` and `OAuthAuthorizationData`.
  - `OidcError` was renamed to `OAuthError` and the `RefreshTokenError::Oidc`
    variant was renamed to `RefreshTokenError::OAuth`.
  - `Oidc::provider_metadata()` was renamed to `OAuth::server_metadata()`.
- [**breaking**]: `OAuth::finish_login()` must always be called, instead of
  `OAuth::finish_authorization()`
  ([#4817](https://github.com/matrix-org/matrix-rust-sdk/pull/4817))
  - `OAuth::abort_authorization()` was renamed to `OAuth::abort_login()`.
  - `OAuth::finish_login()` can be called several times for the same session,
    but it will return an error if it is called with a new session.
  - `OAuthError::MissingDeviceId` was removed, it cannot occur anymore.
- [**breaking**] `OidcRegistrations` was renamed to `OAuthRegistrationStore`.
  ([#4814](https://github.com/matrix-org/matrix-rust-sdk/pull/4814))
  - `OidcRegistrationsError` was renamed to `OAuthRegistrationStoreError`.
  - The `registrations` module was renamed and is now private.
    `OAuthRegistrationStore` and `ClientId` are exported from `oauth`, and
    `OAuthRegistrationStoreError` is exported from `oauth::error`.
  - All the methods of `OAuthRegistrationStore` are now `async` and return a
    `Result`: errors when reading the file are no longer ignored, and blocking
    I/O is performed in a separate thread.
  - `OAuthRegistrationStore::new()` takes a `PathBuf` instead of a `Path`.
  - `OAuthRegistrationStore::new()` no longer takes a `static_registrations`
    parameter. It should be provided if needed with
    `OAuthRegistrationStore::with_static_registrations()`.
- [**breaking**] Allow to use any registration method with `OAuth::login()` and
  `OAuth::login_with_qr_code()`.
  ([#4827](https://github.com/matrix-org/matrix-rust-sdk/pull/4827))
  - `OAuth::login` takes an optional `ClientRegistrationData` to be able to
    register and login with a single function call.
  - `OAuth::url_for_oidc()` was removed, it can be replaced by a call to
    `OAuth::login()`.
  - `OAuth::login_with_qr_code()` takes an optional `ClientRegistrationData`
    instead of the client metadata.
  - `OAuth::finish_login` takes a `UrlOrQuery` instead of an
    `AuthorizationCode`. The deserialization of the query string will occur
    inside the method and eventual errors will be handled.
  - `OAuth::login_with_oidc_callback()` was removed, it can be replaced by a
    call to `OAuth::finish_login()`.
  - `AuthorizationResponse`, `AuthorizationCode` and `AuthorizationError` are
    now private.
- [**breaking**] - `OAuth::account_management_url()` and
  `OAuth::fetch_account_management_url()` don't take an action anymore but
  return an `AccountManagementUrlBuilder`. The final URL can be obtained with
  `AccountManagementUrlBuilder::build()`.
  ([#4831](https://github.com/matrix-org/matrix-rust-sdk/pull/4831))
- [**breaking**] `Client::store` is renamed `state_store`
  ([#4851](https://github.com/matrix-org/matrix-rust-sdk/pull/4851))
- [**breaking**] The parameters `event_id` and `enforce_thread` on
  [`Room::make_reply_event()`] have been wrapped in a `reply` struct parameter.
  ([#4880](https://github.com/matrix-org/matrix-rust-sdk/pull/4880/))
- [**breaking**]: The `Oidc` API was updated to match the latest version of the
  next-gen auth MSCs. The most notable change is that these MSCs are now based
  on OAuth 2.0 rather then OpenID Connect. To reflect that, most types have been
  renamed, with the `Oidc` prefix changed to `OAuth`. The API has also been
  cleaned up, it is now simpler and has fewer methods while keeping most of the
  available features. Here is a detailed list of changes:
  - Rename the `Oidc` API to `OAuth`, since it's using almost exclusively OAuth
    2.0 rather than OpenID Connect.
    ([#4805](https://github.com/matrix-org/matrix-rust-sdk/pull/4805))
    - The `oidc` module was renamed to `oauth`.
    - `Client::oidc()` was renamed to `Client::oauth()` and the `AuthApi::Oidc`
      variant was renamed to `AuthApi::OAuth`.
    - `OidcSession` was renamed to `OAuthSession` and the `AuthSession::Oidc`
      variant was renamed to `AuthSession::OAuth`.
    - `OidcAuthCodeUrlBuilder` and `OidcAuthorizationData` were renamed to
      `OAuthAuthCodeUrlBuilder` and `OAuthAuthorizationData`.
    - `OidcError` was renamed to `OAuthError` and the `RefreshTokenError::Oidc`
      variant was renamed to `RefreshTokenError::OAuth`.
    - `Oidc::provider_metadata()` was renamed to `OAuth::server_metadata()`.
  - The `authentication::qrcode` module was moved inside
    `authentication::oauth`, because it is only available through the `OAuth`
    API. ([#4687](https://github.com/matrix-org/matrix-rust-sdk/pull/4687/))
  - The `OAuth` API only supports public clients, i.e. clients
    without a secret.
    ([#4634](https://github.com/matrix-org/matrix-rust-sdk/pull/4634))
    - `OAuth::restore_registered_client()` takes a `ClientId` instead of
      `ClientCredentials`
    - `OAuth::restore_registered_client()` must NOT be called after
      `OAuth::register_client()` anymore.
  - `Oidc::authorize_scope()` was removed because it has no use
    case anymore, according to the latest version of
    [MSC2967](https://github.com/matrix-org/matrix-spec-proposals/pull/2967).
    ([#4664](https://github.com/matrix-org/matrix-rust-sdk/pull/4664))
  - The `OAuth` API uses the `GET /auth_metadata` endpoint from the latest
    version of
    [MSC2965](https://github.com/matrix-org/matrix-spec-proposals/pull/2965) by
    default. The previous `GET /auth_issuer` endpoint is still supported as a
    fallback for now.
    ([#4673](https://github.com/matrix-org/matrix-rust-sdk/pull/4673))
    - It is not possible to provide a custom issuer anymore:
      `Oidc::given_provider_metadata()` was removed, and the parameter was
      removed from `OAuth::register_client()`.
    - `Oidc::fetch_authentication_issuer()` was removed. To check if the
      homeserver supports OAuth 2.0, use `OAuth::server_metadata()`.
    - `OAuth::server_metadata()` returns an `OAuthDiscoveryError`. It has a
      `NotSupported` variant and an `is_not_supported()` method to check if the
      error is due to the server not supporting OAuth 2.0.
    - `OAuthError::MissingAuthenticationIssuer` was removed.
  - The behavior of `OAuth::logout()` is now aligned with
    [MSC4254](https://github.com/matrix-org/matrix-spec-proposals/pull/4254)
    ([#4674](https://github.com/matrix-org/matrix-rust-sdk/pull/4674))
    - Support for
      [RP-Initiated Logout](https://openid.net/specs/openid-connect-rpinitiated-1_0.html)
      was removed, so it doesn't return an `OidcEndSessionUrlBuilder` anymore.
    - Only one request is made to revoke the access token, since the server is
      supposed to revoke both the access token and the associated refresh token
      when the request is made.
  - Remove most of the parameter methods of `OAuthAuthCodeUrlBuilder`, since
    they were parameters defined in OpenID Connect. Only the `prompt` and
    `user_id_hint` parameters are still supported.
    ([#4699](https://github.com/matrix-org/matrix-rust-sdk/pull/4699))
  - Remove support for ID tokens in the `OAuth` API.
    ([#4726](https://github.com/matrix-org/matrix-rust-sdk/pull/4726))
    - `OAuth::restore_registered_client()` doesn't take a
      `VerifiedClientMetadata` anymore.
    - `Oidc::latest_id_token()` and `Oidc::client_metadata()` were removed.
  - The `OAuth` API makes use of the oauth2 and ruma crates rather than
    mas-oidc-client.
    ([#4761](https://github.com/matrix-org/matrix-rust-sdk/pull/4761))
    ([#4789](https://github.com/matrix-org/matrix-rust-sdk/pull/4789))
    - `ClientId` is a different type reexported from the oauth2 crate.
    - The error types that were in the `oauth` module have been moved to the
      `oauth::error` module.
    - The `device_id` parameter of `OAuth::login` is now an
      `Option<OwnedDeviceId>`.
    - The `state` field of `OAuthAuthorizationData` and the parameter of the
      same name in `OAuth::abort_login()` now use `CsrfToken`.
    - The `types` and `requests` modules are gone and the necessary types are
      exported from the `oauth` module or available from `ruma`.
    - `AccountManagementUrlFull` now takes an `OwnedDeviceId` when a device ID
      is required.
    - `(Verified)ProviderMetadata` was replaced by
      `AuthorizationServerMetadata`.
    - `OAuth::register_client()` doesn't accept a software statement anymore.
    - `(Verified)ClientMetadata` was replaced by `Raw<ClientMetadata>`.
      `ClientMetadata` is an opinionated type that only supports the fields
      required for the `OAuth` API, however any type can be used to construct
      the metadata by serializing it to JSON and converting it.
  - `OAuth::finish_login()` must always be called, instead of
    `OAuth::finish_authorization()`
    ([#4817](https://github.com/matrix-org/matrix-rust-sdk/pull/4817))
    - `OAuth::abort_authorization()` was renamed to `OAuth::abort_login()`.
    - `OAuth::finish_login()` can be called several times for the same session,
      but it will return an error if it is called with a new session.
    - `OAuthError::MissingDeviceId` was removed, it cannot occur anymore.
  - Allow to use any registration method with `OAuth::login()` and
    `OAuth::login_with_qr_code()`.
    ([#4827](https://github.com/matrix-org/matrix-rust-sdk/pull/4827))
    - `OAuth::login` takes an optional `ClientRegistrationData` to be able to
      register and login with a single function call.
    - `OAuth::url_for_oidc()` was removed, it can be replaced by a call to
      `OAuth::login()`.
    - `OAuth::login_with_qr_code()` takes an optional `ClientRegistrationData`
      instead of the client metadata.
    - `OAuth::finish_login` takes a `UrlOrQuery` instead of an
      `AuthorizationCode`. The deserialization of the query string will occur
      inside the method and eventual errors will be handled.
    - `OAuth::login_with_oidc_callback()` was removed, it can be replaced by a
      call to `OAuth::finish_login()`.
    - `AuthorizationResponse`, `AuthorizationCode` and `AuthorizationError` are
      now private.
  - `OAuth::account_management_url()` and
    `OAuth::fetch_account_management_url()` don't take an action anymore but
    return an `AccountManagementUrlBuilder`. The final URL can be obtained with
    `AccountManagementUrlBuilder::build()`.
    ([#4831](https://github.com/matrix-org/matrix-rust-sdk/pull/4831))
  - `OidcRegistrations` was removed. Clients are supposed to re-register with
    the homeserver for every login.
    ([#4879](https://github.com/matrix-org/matrix-rust-sdk/pull/4879))
  - `OAuth::restore_registered_client()` doesn't take an `issuer` anymore.
    ([#4879](https://github.com/matrix-org/matrix-rust-sdk/pull/4879))
    - `Oidc::issuer()` was removed.
    - The `issuer` field of `UserSession` was removed.
- `SendHandle::media_handles` was generalized into a vector
  ([#4898](https://github.com/matrix-org/matrix-rust-sdk/pull/4898))

## [0.10.0] - 2025-02-04

### Features

- Allow to set and check whether an image is animated via its `ImageInfo`.
  ([#4503](https://github.com/matrix-org/matrix-rust-sdk/pull/4503))

- Implement `Default` for `BaseImageInfo`, `BaseVideoInfo`, `BaseAudioInfo` and
  `BaseFileInfo`.
  ([#4503](https://github.com/matrix-org/matrix-rust-sdk/pull/4503))

- Expose `Client::server_versions()` publicly to allow users of the library to
  get the versions of Matrix supported by the homeserver.
  ([#4519](https://github.com/matrix-org/matrix-rust-sdk/pull/4519))

- Create `RoomPrivacySettings` helper to group room settings functionality
  related to room access and visibility.
  ([#4401](https://github.com/matrix-org/matrix-rust-sdk/pull/4401))

- Enable HTTP/2 support in the HTTP client.
  ([#4566](https://github.com/matrix-org/matrix-rust-sdk/pull/4566))

- Add support for creating custom conditional push rules in
  `NotificationSettings::create_custom_conditional_push_rule`.
  ([#4587](https://github.com/matrix-org/matrix-rust-sdk/pull/4587))

- The media contents stored in the media cache can now be controlled with a
  `MediaRetentionPolicy` and the new `Media` methods `media_retention_policy()`,
  `set_media_retention_policy()`, `clean_up_media_cache()`.
  ([#4571](https://github.com/matrix-org/matrix-rust-sdk/pull/4571))

- Add support for creating custom conditional push rules in
  `NotificationSettings::create_custom_conditional_push_rule`.
  ([#4587](https://github.com/matrix-org/matrix-rust-sdk/pull/4587))

### Refactor

- [**breaking**]: The `RoomEventCacheUpdate::Clear` variant has been removed, as
  it is redundant with the `RoomEventCacheUpdate::UpdateTimelineEvents { diffs:
  Vec<VectorDiff<_>>, .. }` where `VectorDiff` has its own `Clear` variant.
  ([#4627](https://github.com/matrix-org/matrix-rust-sdk/pull/4627))

- Improve the performance of `EventCache` (approximately 4.5 times faster).
  ([#4616](https://github.com/matrix-org/matrix-rust-sdk/pull/4616))

- [**breaking**]: The reexported types `SyncTimelineEvent` and `TimelineEvent`
  have been fused into a single type `TimelineEvent`, and its field
  `push_actions` has been made `Option`al (it is set to `None` when we couldn't
  compute the push actions, because we lacked some information).
  ([#4568](https://github.com/matrix-org/matrix-rust-sdk/pull/4568))

- [**breaking**] Move the optional `RequestConfig` argument of the
  `Client::send()` method to the `with_request_config()` builder method. You
  should call `Client::send(request).with_request_config(request_config).await`
  now instead.
  ([#4443](https://github.com/matrix-org/matrix-rust-sdk/pull/4443))

- [**breaking**] Remove the `AttachmentConfig::with_thumbnail()` constructor and
  replace it with the `AttachmentConfig::thumbnail()` builder method. You should
  call `AttachmentConfig::new().thumbnail(thumbnail)` now instead.
  ([#4452](https://github.com/matrix-org/matrix-rust-sdk/pull/4452))

- [**breaking**] `Room::send_attachment()` and
  `RoomSendQueue::send_attachment()` now take any type that implements
  `Into<String>` for the filename.
  ([#4451](https://github.com/matrix-org/matrix-rust-sdk/pull/4451))

- [**breaking**] `Recovery::are_we_the_last_man_standing()` has been renamed to
  `is_last_device()`.
  ([#4522](https://github.com/matrix-org/matrix-rust-sdk/pull/4522))

- [**breaking**] The `matrix_auth` module is now at `authentication::matrix`.
  ([#4575](https://github.com/matrix-org/matrix-rust-sdk/pull/4575))

- [**breaking**] The `oidc` module is now at `authentication::oidc`.
  ([#4575](https://github.com/matrix-org/matrix-rust-sdk/pull/4575))

## [0.9.0] - 2024-12-18

### Bug fixes

- Use the inviter's server name and the server name from the room alias as
  fallback values for the via parameter when requesting the room summary from
  the homeserver. This ensures requests succeed even when the room being
  previewed is hosted on a federated server.
  ([#4357](https://github.com/matrix-org/matrix-rust-sdk/pull/4357))

- Do not use the encrypted original file's content type as the encrypted
  thumbnail's content type.
  ([#ecf4434](https://github.com/matrix-org/matrix-rust-sdk/commit/ecf44348cf6a872b843fb7d7af1a88f724c58c3e))

### Features

- Enable persistent storage for the `EventCache`. This allows events received
  through the `/sync` endpoint or backpagination to be stored persistently,
  enabling client applications to restore a room's view, including events,
  without requiring server communication.
  ([#4347](https://github.com/matrix-org/matrix-rust-sdk/pull/4347))

- [**breaking**] Make all fields of Thumbnail required
  ([#4324](https://github.com/matrix-org/matrix-rust-sdk/pull/4324))

- `Backups::exists_on_server`, which always fetches up-to-date information from
  the server about whether a key storage backup exists, was renamed to
  `fetch_exists_on_the_server`, and a new implementation of `exists_on_server`
  which caches the most recent answer is now provided.

## [0.8.0] - 2024-11-19

### Bug fixes

- Add more invalid characters for room aliases.
- Match the right status code in `Client::is_room_alias_available`.
- Fix a bug where room keys were considered to be downloaded before backups were
  enabled. This bug only affects the
  `BackupDownloadStrategy::AfterDecryptionFailure`, where no attempt would be
  made to download a room key, if a decryption failure with a given room key
  would have been encountered before the backups were enabled.

### Documentation

- Improve documentation of `Client::observe_events`.

### Features

- Add `create_room_alias` function.
- `Client::cross_process_store_locks_holder_name` is used everywhere:
- `StoreConfig::new()` now takes a
  `cross_process_store_locks_holder_name` argument.
- `StoreConfig` no longer implements `Default`.
- `BaseClient::new()` has been removed.
- `BaseClient::clone_with_in_memory_state_store()` now takes a
  `cross_process_store_locks_holder_name` argument.
- `BaseClient` no longer implements `Default`.
- `EventCacheStoreLock::new()` no longer takes a `key` argument.
- `BuilderStoreConfig` no longer has
  `cross_process_store_locks_holder_name` field for `Sqlite` and
  `IndexedDb`.

- `EncryptionSyncService` and `Notification` are using
  `Client::cross_process_store_locks_holder_name`.

- Allow passing a custom `RequestConfig` to an upload request.
- Retry uploads if they've failed with transient errors.
- Implement `EventHandlerContext` for tuples.
- Introduce a mechanism similar to `Client::add_event_handler` and
  `Client::add_room_event_handler` but with a reactive programming pattern. Add
  `Client::observe_events` and `Client::observe_room_events`.

  ```rust
  // Get an observer.
  let observer =
     client.observe_events::<SyncRoomMessageEvent, (Room, Vec<Action>)>();

  // Subscribe to the observer.
  let mut subscriber = observer.subscribe();

  // Use the subscriber as a `Stream`.
  let (message_event, (room, push_actions)) = subscriber.next().await.unwrap();
  ```

  When calling `observe_events`, one has to specify the type of event (in the
  example, `SyncRoomMessageEvent`) and a context (in the example, `(Room,
  Vec<Action>)`, respectively for the room and the push actions).

- Implement unwedging for media uploads.
- Send state from state sync and not from timeline to widget
  ([#4254](https://github.com/matrix-org/matrix-rust-sdk/pull/4254))

- Allow aborting media uploads.
- Add `RoomPreviewInfo::num_active_members`.
- Use room directory search as another data source.
- Check if the user is allowed to do a room mention before trying to send a call
  notify event.
  ([#4271](https://github.com/matrix-org/matrix-rust-sdk/pull/4271))

- Add `Client::cross_process_store_locks_holder_name()`.
- Add a `PreviouslyVerified` variant to `VerificationLevel` indicating that the
  identity is unverified and previously it was verified.

- New `UserIdentity::pin` method.
- New `ClientBuilder::with_decryption_trust_requirement` method.
- New `ClientBuilder::with_room_key_recipient_strategy` method
- New `Room.set_account_data` and `Room.set_account_data_raw` RoomAccountData
  setters, analogous to the GlobalAccountData

- New `RequestConfig.max_concurrent_requests` which allows to limit the maximum
  number of concurrent requests the internal HTTP client issues (all others have
  to wait until the number drops below that threshold again)

- Implement proper redact handling in the widget driver. This allows the Rust
  SDK widget driver to support widgets that rely on redacting.

### Refactor

- [**breaking**] Rename `DisplayName` to `RoomDisplayName`.
- Improve `is_room_alias_format_valid` so it's more strict.
- Remove duplicated fields in media event contents.
- Use `SendHandle` for media uploads too.
- Move `event_cache_store/` to `event_cache/store/` in `base`.
- Move `linked_chunk` from `matrix` to `sdk-common`.
- Move `Event` and `Gap` into `base::event_cache`.
- Move `formatted_caption_from` to the SDK, rename it.
- Tidy up and start commenting the widget code.
- Get rid of `ProcessingContext` and inline it in its callers.
- Get rid of unused `limits` parameter when constructing a `WidgetMachine`.
- Use a specialized mutex for locking access to the state store and
  `being_sent`.

- Renamed `VerificationLevel::PreviouslyVerified` to
  `VerificationLevel::VerificationViolation`.

- [**breaking**] Replace the `Notification` type from Ruma in `SyncResponse` and
  `Client::register_notification_handler` by a custom one.

- [**breaking**] The ambiguity maps in `SyncResponse` are moved to `JoinedRoom`
  and `LeftRoom`.

- [**breaking**] `Room::can_user_redact` and `Member::can_redact` are split
  between `*_redact_own` and `*_redact_other`.

- [**breaking**] `AmbiguityCache` contains the room member's user ID.
- [**breaking**] Replace `impl MediaEventContent` with `&impl MediaEventContent`
  in
  `Media::get_file`/`Media::remove_file`/`Media::get_thumbnail`/`Media::remove_thumbnail`

- [**breaking**] A custom sliding sync proxy set with
  `ClientBuilder::sliding_sync_proxy` now takes precedence over a discovered
  proxy.

- [**breaking**] `Client::get_profile` was moved to `Account` and renamed to
  `Account::fetch_user_profile_of`. `Account::get_profile` was renamed to
  `Account::fetch_user_profile`.

- [**breaking**] The `HttpError::UnableToCloneRequest` error variant has been
  removed because it was never used or generated by the SDK.

- [**breaking**] The `Error::InconsistentState` error variant has been removed
  because it was never used or generated by the SDK.

- [**breaking**] The widget capabilities in the FFI now need two additional
  flags: `update_delayed_event`, `send_delayed_event`.

- [**breaking**] `Room::event` now takes an optional `RequestConfig` to allow
  for tweaking the network behavior.

- [**breaking**] The `instant` module was removed, use the `ruma::time` module
  instead.

- [**breaking**] Add `ClientBuilder::sqlite_store_with_cache_path` to build a
  client that stores caches in a different directory to state/crypto.

- [**breaking**] The `body` parameter in `get_media_file` has been replaced with
  a `filename` parameter now that Ruma has a `filename()` method.

## 0.7.0

Breaking changes:

- The `Client::sync_token` accessor function is no longer public. If you were
  using this for `Client::sync_once()`, you can get the token from the result of
  the `Client::sync_once()` method instead
  ([#1216](https://github.com/matrix-org/matrix-rust-sdk/pull/1216)).
- `Common::members` and `Common::members_no_sync` take a `RoomMemberships` to be
  able to filter the results by any membership state.
  - `Common::active_members(_no_sync)` and `Common::joined_members(_no_sync)`
    are deprecated.
- `sqlite` is the new default store implementation outside of WASM,
  behind the `sqlite` feature.
  - The `sled` feature was removed. The `matrix-sled` crate is deprecated
    and no longer maintained.
- Replace `Client::authentication_issuer` with
  `Client::authentication_server_info` that contains all the fields discovered
  from the homeserver for authenticating with OIDC
- Remove `HttpSend` trait in favor of allowing a custom `reqwest::Client`
  instance to be supplied
- Move all the types and methods using the native Matrix login and registration
  APIs from `Client` to the new `matrix_auth::MatrixAuth` API that is accessible
  via `Client::matrix_auth()`.
- Move `Session` and `SessionTokens` to the `matrix_auth` module.
  - Move the session methods on `Client` to the `MatrixAuth` API.
  - Split `Session`'s content into several types. Its (de)serialization is still
    backwards compatible.
- The room API has been simplified
  - Removed the previous `Room`, `Joined`, `Invited` and `Left` types
  - Merged all of the functionality from `Joined`, `Invited` and `Left` into
    `room::Common`
  - Renamed `room::Common` to just `Room` and made it accessible as
    `matrix::Room`
- Event handler closures now need to implement `FnOnce` + `Clone` instead of
  `Fn`
  - As a consequence, you no longer need to explicitly need to `clone` variables
    they capture before constructing an `async move {}` block inside
- `Room::sync_members` doesn't return the underlying Ruma response anymore. If
  you need to get the room members, you can use `Room::members` or
  `Room::get_member` which will make sure that the members are up to date.
- The `transaction_id` parameter of `Room::{send, send_raw}` was removed
  - Instead, both methods now return types that implement `IntoFuture` (so can
    be awaited like before) and have a `with_transaction_id` builder-style
    method
- The parameter order of `Room::{send_raw, send_state_event_raw}` has changed,
  `content` is now last
  - The parameter type of `content` has also changed to a generic;
    `serde_json::Value` arguments are still allowed, but so are other types like
    `Box<serde_json::value::RawValue>`
- All "named futures" (structs implementing `IntoFuture`) are now exported from
  modules named `futures` instead of directly in the respective parent module
- `Verification` is non-exhaustive, to make the `qrcode` cargo feature additive

Bug fixes:

- `Client::rooms` now returns all rooms, even invited, as advertised.

Additions:

- Add secret storage support, the secret store can be opened using the
  `Client::encryption()::open_secret_store()` method, which allows you to import
  or export secrets from the account-data backed secret-store.

- Add `VerificationRequest::state` and `VerificationRequest::changes` to check
  and listen to changes in the state of the `VerificationRequest`. This removes
  the need to listen to individual matrix events once the `VerificationRequest`
  object has been acquired.
- The `Room` methods to retrieve state events can now return a sync or stripped
  event, so they can be used for invited rooms too.
- Add `Client::subscribe_to_room_updates` and
  `room::Common::subscribe_to_updates`
- Add `Client::rooms_filtered`
- Add methods on `Client` that can handle several authentication APIs.
- Add new method `force_discard_session` on `Room` that allows to discard the
  current outbound session (room key) for that room. Can be used by clients for
  the `/discardsession` command.

## 0.6.2

- Fix the access token being printed in tracing span fields.

## 0.6.1

- Fixes a bug where the access token used for Matrix requests was added as a
  field to a tracing span.

---

## `client-common`


All notable changes to this project will be documented in this file.


## Unreleased

### Added

- Add `TtlValue::has_expired_after`, which checks expiry against a
  caller-provided time-to-live rather than the default threshold.
  ([#36](https://github.com/harana/harana-matrix/issues/36))

## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

No significant changes.

## [0.17.0] - 2026-05-08

### Features

- [**breaking**] Change to the stable identifiers for `m.history_not_shared`.
  We still support reading the unstable identifier.
  ([#6467](https://github.com/matrix-org/matrix-rust-sdk/pull/6467))
- Add a method to check the validity of edits.
  ([#6454](https://github.com/matrix-org/matrix-rust-sdk/pull/6454))
- A background task monitor has been added, that can spawn background tasks and
  monitor their execution on a separate channel. Such tasks can run forever, or
  they can run for one-shot jobs.
  ([#6075](https://github.com/matrix-org/matrix-rust-sdk/pull/6075) &&
  [#6421](https://github.com/matrix-org/matrix-rust-sdk/pull/6421))
- Add `AcquireCrossProcessLockResult` and `AcquireCrossProcessLockFn`
  for convenience in generalizing cross-process lock acquisition.
  ([#6326](https://github.com/matrix-org/matrix-rust-sdk/pull/6326))
- Add support in the `MemoryStore`'s implementation of `EventCacheStore` for
  having duplicate events in a room, where each duplicate is in a different
  `LinkedChunk`. This is useful, e.g., when an event is in a room and a
  thread in that room.
- [**breaking**] In order to support having duplicate events in the same room
  (in different `LinkedChunk`'s) a few functions were changed in
  `RelationalLinkedChunk`. The items in the `Iterator` returned by
  `RelationalLinkedChunk::items` now also include the `LinkedChunkId` in which
  the `Item` was found. Additionally, `RelationalLinkedChunk::save_item` now
  requires the `Item` to be `Clone` as it may be stored in multiple
  `LinkedChunk`s.
  (#[6200](https://github.com/matrix-org/matrix-rust-sdk/pull/6200))
- [**breaking**] Added `CrossProcessLockConfig`, which can be used to configure
  the behavior of the cross-process lock. `CrossProcessLock` now takes a
  `CrossProcessLockConfig` as an argument to its constructor instead of a
  `lock_holder` value.
  ([#6160](https://github.com/matrix-org/matrix-rust-sdk/pull/6160))
- [**breaking**] `ShieldStateCode` no longer includes
  `SentInClear`. `VerificationState::to_shield_state_{lax,strict}` never
  returned that code, and so having it in the enum was somewhat misleading.
  ([#5959](https://github.com/matrix-org/matrix-rust-sdk/pull/5959))
- Add field `forwarder` of type `ForwarderInfo` to `EncryptionInfo`, which
  exposes information about the forwarder of the keys with which an event was
  encrypted if they were shared as part of an
  [MSC4268](https://github.com/matrix-org/matrix-spec-proposals/pull/4268) room
  key bundle.
  ([#5945](https://github.com/matrix-org/matrix-rust-sdk/pull/5945)).

### Bug fixes

- Fix an off-by-one check for `Error:InvalidItemIndex` in
  `LinkedChunk::remove_item_at`.
  ([#6057](https://github.com/matrix-org/matrix-rust-sdk/pull/6057))
- Fix `TimelineEvent::from_bundled_latest_event` sometimes removing the
  `session_id` of UTDs. This broken event could later be saved to the event
  cache and become an unresolvable UTD.
  ([#5970](https://github.com/matrix-org/matrix-rust-sdk/pull/5970)).

### Refactor

- [**breaking**] Remove `ttl_cache::TtlCache` because it is now unused.
  ([#6484](https://github.com/matrix-org/matrix-rust-sdk/pull/6484))

## [0.16.1] - 2026-05-08

### Features

- Add a method to check the validity of edits.
  ([#6454](https://github.com/matrix-org/matrix-rust-sdk/pull/6454))

## [0.16.0] - 2025-12-04

### Features

- [**breaking**] Cross-process lock can be dirty. The
  `CrossProcess::try_lock_once` now returns a new type `CrossProcessResult`,
  which is an enum with `Clean`, `Dirty` or `Unobtained` variants. When the lock
  is dirty it means it's been acquired once, then acquired another time from
  another holder, so the current holder may want to refresh its internal state.
  ([#5672](https://github.com/matrix-org/matrix-rust-sdk/pull/5672)).

## [0.14.0] - 2025-09-04

### Features

- Tracing subscribers created via
  [`sdk_common::js_tracing::MakeJsLogWriter`] or
  [`make_tracing_subscriber`] will now drop log events at the `TRACE` level.
  Previously `TRACE` logs were treated the same as `DEBUG` logs.
  ([#5590](https://github.com/matrix-org/matrix-rust-sdk/pull/5590)).

- [**breaking**] Use `Raw<AnyTimelineEvent>` in place of
  `Raw<AnyMessageLikeEvent>` in `DecryptedRoomEvent::event`.
  ([#5512](https://github.com/matrix-org/matrix-rust-sdk/pull/5512)). Affects
  the following functions:
  - `OlmMachine::decrypt_room_event` - existing matches on the result's event
    field should be updated to
    `AnyTimelineEvent::MessageLike(AnyMessageLikeEvent::...)`

## [0.13.0] - 2025-07-10

### Features

- Expose the `ROOM_VERSION_RULES_FALLBACK` that should be used when the rules of
  a room are unknown.
  ([#5337](https://github.com/matrix-org/matrix-rust-sdk/pull/5337))
- Expose the `ROOM_VERSION_FALLBACK` that should be used when the version of a
  room is unknown.
  ([#5306](https://github.com/matrix-org/matrix-rust-sdk/pull/5306))

### Refactor

- [**breaking**] `extract_bundled_thread_summary()` returns a
  `Raw<AnySyncMessageLikeEvent>` for the latest event instead of a
  `Raw<AnyMessageLikeEvent>`.
  ([#5337](https://github.com/matrix-org/matrix-rust-sdk/pull/5337))

## [0.12.0] - 2025-06-10

No notable changes in this release.

## [0.11.0] - 2025-04-11

### Features

- Add a simple TTL cache implementation. The `TtlCache` struct can be used as a
  key/value map that expires items after 15 minutes.
  ([#4663](https://github.com/matrix-org/matrix-rust-sdk/pull/4663))

## [0.10.0] - 2025-02-04

- [**breaking**]: `SyncTimelineEvent` and `TimelineEvent` have been
  fused into a single type `TimelineEvent`, and its field `push_actions`
  has been made `Option`al (it is set to `None` when we couldn't
  compute the push actions, because we lacked some information).
  ([#4568](https://github.com/matrix-org/matrix-rust-sdk/pull/4568))

## [0.9.0] - 2024-12-18

### Bug fixes

- Change the behavior of `LinkedChunk::new_with_update_history()` to emit an
  `Update::NewItemsChunk` when a new, initial empty, chunk is created.
  ([#4327](https://github.com/matrix-org/matrix-rust-sdk/pull/4321))

- [**breaking**] Make `Room::history_visibility()` return an Option, and
  introduce `Room::history_visibility_or_default()` to return a better
  sensible default, according to the spec.
  ([#4325](https://github.com/matrix-org/matrix-rust-sdk/pull/4325))

- Clear the internal state of the `AsVector` struct if an `Update::Clear`
  state has been received.
  ([#4321](https://github.com/matrix-org/matrix-rust-sdk/pull/4321))

### Documentation

- Document that a decrypted raw event always has a room id.
  ([#728e1fd](https://github.com/matrix-org/matrix-rust-sdk/commit/728e1fda2ae9f1bfa87df162aa553040be705223))

## [0.8.0] - 2024-11-19

### Refactor

- Move `linked_chunk` from `matrix` to `sdk-common`.

---

## `client-base`


All notable changes to this project will be documented in this file.


## Unreleased

### Added

- Add `EventCacheStore::get_custom_value`, `set_custom_value` and
  `remove_custom_value`, a key/value area for cross-process data that is not
  tied to a room. ([#250](https://github.com/harana/harana-matrix/issues/250))
- Add `StateStoreExt::get_serialized_custom_value`,
  `set_serialized_custom_value`, `set_serialized_custom_value_no_read` and
  `remove_serialized_custom_value`, so structured data can be kept in the
  custom-value store without hand-rolling serialization.
  ([#16](https://github.com/harana/harana-matrix/issues/16))

### Fixed

- Drop the sync token when the ignored user list changes, so the next sync is
  an initial sync. Ignoring or unignoring a user changes which events the
  server returns, and an incremental sync leaves the events it will no longer
  send cached locally.
  ([#42](https://github.com/harana/harana-matrix/issues/42))

## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

### Added

- Add `Room::compute_joined_service_members` to compute the number of joined
  service members in a room. This is needed for calculating display names of
  `SpaceRoom`s with service members.
  ([#6561](https://github.com/matrix-org/matrix-rust-sdk/pulls/6561))
- Add `RoomInfo::fully_read_event_id` and `Room::fully_read_event_id` to expose
  the user's `m.fully_read` event ID.
  ([#6569](https://github.com/matrix-org/matrix-rust-sdk/pulls/6569))

### Changed

- `Client::sync_once` acquires the state store lock when processing a sync and
  response and holds it until processing has completed. This mimics the behavior
  of `SlidingSync::sync_once`.
  ([#6555](https://github.com/matrix-org/matrix-rust-sdk/pulls/6555))
- [**breaking**] `RoomInfoNotableUpdateReasons` is now a `u16` to include a
  `FULLY_READ` flag to notify on changes of the `m.fully_read` marker.
  ([#6569](https://github.com/matrix-org/matrix-rust-sdk/pulls/6569))

## [0.17.0] - 2026-05-08

### Bug fixes

- Filter out service members from `Room::heroes`. This _should_ be done by the
  homeservers, but some don't.
  ([#6535](https://github.com/matrix-org/matrix-rust-sdk/pull/6535))
- Room keys are now rotated whenever the client fully reloads the member list by
  making a request to `/members`, which prevents clients using keys that may
  have been shared under
  [MSC4268](https://github.com/matrix-org/matrix-spec-proposals/pull/4268) even
  if a gappy sync occurs.
  ([#6339](https://github.com/matrix-org/matrix-rust-sdk/pull/6339))

- Fix invited/knocked rooms disappearing from the room list after
  join → leave/kick → re-invite when using Sliding Sync. The SDK now always
  emits a room update so the room is surfaced correctly again.
  ([#6126](https://github.com/matrix-org/matrix-rust-sdk/pull/6126))

- [**breaking**] `BaseClient::room_info_notable_update_sender` has
  moved into `BaseStateStore`. `BaseStateStore::derive_from_other`
  and `BaseStateStore::get_or_create_room` no longer takes a
  `room_info_notable_update_sender` argument.
  ([#6130](https://github.com/matrix-org/matrix-rust-sdk/pull/6130))
- [**breaking**] New `LatestEventValue::LocalHasBeenSent` variant to represent
  a local event that has been sent successfully.
  ([#5968](https://github.com/matrix-org/matrix-rust-sdk/pull/5968))

### Features

- [**breaking**] Add `RoomSummary::active_service_members` field to act as a
  cached value that will be computed when we sync members. Rename `Room::is_dm`
  to `Room::compute_is_dm` since it will now also store the computed active
  service members count in the new cached field. `Room::active_service_members`
  is now `Room::update_active_service_members` for the same reason.
  ([#6537](https://github.com/matrix-org/matrix-rust-sdk/pull/6537))
- [**breaking**] Enforce atomic and synchronized updates to `RoomInfo`. Requires
  `StateStore::save_changes` to acquire state store lock and replaces
  `Room::set_room_info` with an atomic version, `Room::update_room_info`, which
  is also synchronized by the state store lock.
  ([#6478](https://github.com/matrix-org/matrix-rust-sdk/pull/6478))
- Add `RoomMember::is_service_member` that automatically checks the room info
  and retrieves this info.
  ([#6536](https://github.com/matrix-org/matrix-rust-sdk/pull/6536))
- [**breaking**] Add `DmRoomDefinition` enum, allowing clients to specify what a
  DM room should look like. A `Room::is_dm` method was added to check if a room
  is a DM room too, using this definition.
  ([#6490](https://github.com/matrix-org/matrix-rust-sdk/pull/6490))
- Add `Room::active_room_members`, returning a list of all the service room
  members that are active in the room.
  ([#6843](https://github.com/matrix-org/matrix-rust-sdk/pull/6483))
- Add support in the `MemoryStore`'s implementation of `EventCacheStore` for
  having duplicate events in a room, where each duplicate is in a different
  `LinkedChunk`. This is useful, e.g., when an event is in a room and a
  thread in that room.
  (#[6200](https://github.com/matrix-org/matrix-rust-sdk/pull/6200))
- Add `StateStore::upsert_thread_subscriptions()` method for bulk upserts.
  ([#5848](https://github.com/matrix-org/matrix-rust-sdk/pull/5848))
- The `LatestEventValue::LocalHasBeenSent` variant gains a new `event_id:
  OwnedEventId` field.
  ([#5977](https://github.com/matrix-org/matrix-rust-sdk/pull/5977))
- [**breaking**] `RelationalLinkedChunk::apply_updates` returns an error rather
  than panicking. This is necessary in order to ensure certain behaviors are
  disallowed. ([#6061](https://github.com/matrix-org/matrix-rust-sdk/pull/6061))
- Add `RoomInfo::active_room_call_consensus_intent()` method to get the call
  intent for the current call, based on what members are advertising.
  ([#6274](https://github.com/matrix-org/matrix-rust-sdk/pull/6274))
- Add `Room::is_call` to check for Call rooms (MSC3417)
  ([#6315](https://github.com/matrix-org/matrix-rust-sdk/pull/6315))

### Refactor

- [**breaking**] `TtlStoreValue` was moved and renamed to
  `sdk_common::ttl::TtlValue`.
  ([#6463](https://github.com/matrix-org/matrix-rust-sdk/pull/6463),
  [#6484](https://github.com/matrix-org/matrix-rust-sdk/pull/6484))
- [**breaking**] `Gap::prev_token` has been renamed to `Gap::token` since it's
  now used for both the previous batch token and the next batch token.
  ([#6236](https://github.com/matrix-org/matrix-rust-sdk/pull/6236))
- [**breaking**] Invite acceptance details are no longer stored in `RoomInfo`,
  and the accessors `RoomInfo.invite_acceptance_details()` and
  `Room::invite_acceptance_details` have been removed. Instead, equivalent
  details are stored in the Crypto store, and, provided the `e2e-encryption`
  feature is enabled, are accessible via
  `BaseClient::get_pending_key_bundle_details_for_room`.
  ([#6199](https://github.com/matrix-org/matrix-rust-sdk/pull/6199))
- [**breaking**] `once_cell` is no longer reexported from this crate. The types
  that were stabilized in the Rust standard library can be used instead in most
  cases. ([#6194](https://github.com/matrix-org/matrix-rust-sdk/pull/6194))
- [**breaking**] All the `*StoreLock` structs use a `CrossProcessLockConfig` now <!-- rumdl-disable-line MD013 -->
  instead of the previous `holder` value and so does `StoreConfig` and
  `BaseClient::clone_with_in_memory_state_store. Passing a `CrossProcessLockConfig::MultiProcess`
  will keep the same behaviour we had where the client uses the cross process
  lock and using `CrossProcessLockConfig::SingleProcess` will disable the cross process lock.
  ([#6061](https://github.com/matrix-org/matrix-rust-sdk/pull/6061))
- [**breaking**] The `StateStore::upsert_thread_subscription` method has been
  removed in favor of a bulk method `StateStore::upsert_thread_subscriptions`.
- [**breaking**] The `message-ids` feature has been removed. It was already a
  no-op and has now been eliminated entirely.
  ([#5963](https://github.com/matrix-org/matrix-rust-sdk/pull/5963))

## [0.16.1] - 2026-05-08

No notable changes in this release.

## [0.16.0] - 2025-12-04

### Security fixes

- Skip the serialization of custom join rules in the `RoomInfo` which prevented
  the processing of sync responses containing events with custom join rules.
  ([#5924](https://github.com/matrix-org/matrix-rust-sdk/pull/5924), Low,
  [CVE-2025-66622](https://www.cve.org/CVERecord?id=CVE-2025-66622),
  [GHSA-jj6p-3m75-g2p3](https://github.com/matrix-org/matrix-rust-sdk/security/advisories/GHSA-jj6p-3m75-g2p3)).

### Refactor

- [**breaking**] `ServerInfo` has been renamed to `SupportedVersionsResponse`,
  and its `well_known` field has been removed. It is also wrapped in a
  `TtlStoreValue` that handles the expiration of the data, rather than calling
  `maybe_decode()`. Its constructor has been removed since all its fields are
  now public.
  ([#5910](https://github.com/matrix-org/matrix-rust-sdk/pull/5910))
  - `StateStoreData(Key/Value)::ServerInfo` has been split into the
    `SupportedVersions` and `WellKnown` variants.
- [**breaking**] Upgrade Ruma to version 0.14.0.
  ([#5882](https://github.com/matrix-org/matrix-rust-sdk/pull/5882))
- `Client::sync_lock` has been renamed `Client::state_store_lock`.
  ([#5707](https://github.com/matrix-org/matrix-rust-sdk/pull/5707))

### Features

- [**breaking**] The `EventCacheStore::get_room_events()` method has received
  two new arguments. This allows users to load only events of a certain event
  type and events that were encrypted using a certain room key identified by its
  session ID.
  ([#5817](https://github.com/matrix-org/matrix-rust-sdk/pull/5817))
- `ComposerDraft` can now store attachments alongside text messages.
  ([#5794](https://github.com/matrix-org/matrix-rust-sdk/pull/5794))

## [0.14.1] - 2025-09-10

### Security fixes

- Fix a panic in the `RoomMember::normalized_power_level` method.
  ([#5635](https://github.com/matrix-org/matrix-rust-sdk/pull/5635)) ( Low,
  [CVE-2025-59047](https://www.cve.org/CVERecord?id=CVE-2025-59047),
  [GHSA-qhj8-q5r6-8q6j](https://github.com/matrix-org/matrix-rust-sdk/security/advisories/GHSA-qhj8-q5r6-8q6j)).

## [0.14.0] - 2025-09-04

### Features

- Add `SyncResponse::RoomUpdates::is_empty` to check if there were any room
  updates. ([#5593](https://github.com/matrix-org/matrix-rust-sdk/pull/5593))
- Add `EncryptionState::StateEncrypted` to represent rooms supporting encrypted
  state events. Feature-gated behind `experimental-encrypted-state-events`.
  ([#5523](https://github.com/matrix-org/matrix-rust-sdk/pull/5523))
- [**breaking**] The `state` field of `JoinedRoomUpdate` and `LeftRoomUpdate`
  now uses the `State` enum, depending on whether the state changes were
  received in the `state` field or the `state_after` field.
  ([#5488](https://github.com/matrix-org/matrix-rust-sdk/pull/5488))
- [**breaking**] `RoomCreateWithCreatorEventContent` has a new field
  `additional_creators` that allows to specify additional room creators beside
  the user sending the `m.room.create` event, introduced with room version 12.
  ([#5436](https://github.com/matrix-org/matrix-rust-sdk/pull/5436))
- [**breaking**] The `RoomInfo` method now remembers the inviter at the time
  when the `BaseClient::room_joined()` method was called. The caller is
  responsible to remember the inviter before a server request to join the room
  is made. The `RoomInfo::invite_accepted_at` method was removed, the
  `RoomInfo::invite_details` method returns both the timestamp and the
  inviter.
  ([#5390](https://github.com/matrix-org/matrix-rust-sdk/pull/5390))

### Refactor

- [**breaking**] The `Stripped` variants of `RawAnySyncOrStrippedTimelineEvent`,
  `RawAnySyncOrStrippedState` and `AnySyncOrStrippedState` use `StrippedState`
  instead of `AnyStrippedStateEvent`.
  ([#5473](https://github.com/matrix-org/matrix-rust-sdk/pull/5473))
- [**breaking**] The `stripped_state` field of `StateChanges` uses
  `StrippedState` instead of `AnyStrippedStateEvent`.
  ([#5473](https://github.com/matrix-org/matrix-rust-sdk/pull/5473))
- [**breaking**] `RelationalLinkedChunk::items` now takes a `RoomId` instead of
  an `&OwnedLinkedChunkId` parameter.
  ([#5445](https://github.com/matrix-org/matrix-rust-sdk/pull/5445))
- [**breaking**] Add an `IsPrefix = False` bound to the
  `get_state_event_static()`, `get_state_event_static_for_key()` and
  `get_state_events_static()`, `get_account_data_event_static()` and
  `get_room_account_data_event_static` methods of `StateStoreExt`. These methods
  only worked for events where the full event type is statically-known, and this
  is now enforced at compile-time. The matching non-`static` methods of
  `StateStore` can be used instead for event types with a variable suffix.
  ([#5444](https://github.com/matrix-org/matrix-rust-sdk/pull/5444))
- [**breaking**]
  `SyncOrStrippedState<RoomPowerLevelsEventContent>::power_levels()` takes
  `AuthorizationRules` and a list of creators, because creators can have
  infinite power levels, as introduced in room version 12.
  ([#5436](https://github.com/matrix-org/matrix-rust-sdk/pull/5436))
- [**breaking**] `RoomMember::power_level()` and
  `RoomMember::normalized_power_level()` now use `UserPowerLevel` to represent
  power levels instead of `i64` to differentiate the infinite power level of
  creators, as introduced in room version 12.
  ([#5436](https://github.com/matrix-org/matrix-rust-sdk/pull/5436))
- [**breaking**] The `creator()` methods of `Room` and `RoomInfo` have been
  renamed to `creators()` and can now return a list of user IDs, to reflect that
  a room can have several creators, as introduced in room version 12.
  ([#5436](https://github.com/matrix-org/matrix-rust-sdk/pull/5436))
- [**breaking**] `RoomInfo::room_version_or_default()` was replaced with
  `room_version_rules_or_default()`. The room version should only be used for
  display purposes. The rules contain flags for all the differences in behavior
  between all known room versions.
  ([#5337](https://github.com/matrix-org/matrix-rust-sdk/pull/5337))
- [**breaking**] `MinimalStateEvent::redact()` takes `RedactionRules` instead of
  a `RoomVersionId`.
  ([#5337](https://github.com/matrix-org/matrix-rust-sdk/pull/5337))
- [**breaking**] The `event_id` field of `PredecessorRoom` was removed, due to
  its removal in the Matrix specification with MSC4291.
  ([#5419](https://github.com/matrix-org/matrix-rust-sdk/pull/5419))

## [0.13.0] - 2025-07-10

### Features

- The `RoomInfo` now remembers when an invite was explicitly accepted when the
  `BaseClient::room_joined()` method was called. A new getter for this
  timestamp exists, the `RoomInfo::invite_accepted_at()` method returns this
  timestamp.
  ([#5333](https://github.com/matrix-org/matrix-rust-sdk/pull/5333))
- [**breaking**] The `BaseClient::new()` method now takes an additional
  `ThreadingSupport` parameter controlling whether the client is supposed to do
  extra processing for threads. Right now, it controls whether to exclude
  in-thread events from the room unread counts, but it may be expanded in the
  future to support more threading-related features.
  ([#5325](https://github.com/matrix-org/matrix-rust-sdk/pull/5325))

### Refactor

- The cached `ServerCapabilities` has been renamed to `ServerInfo` and
  additionally contains the well-known response alongside the existing server
  versions. Despite the old name, it does not contain the server capabilities.
  ([#5167](https://github.com/matrix-org/matrix-rust-sdk/pull/5167))
- `Room::join_rule` and `Room::is_public` now return an `Option` to reflect that
  the join rule state event might be missing, in which case they will return
  `None`. ([#5278](https://github.com/matrix-org/matrix-rust-sdk/pull/5278))

## [0.12.0] - 2025-06-10

No notable changes in this release.

## [0.11.0] - 2025-04-11

### Features

- [**breaking**] The `Client::subscribe_to_ignore_user_list_changes()`
  method will now only trigger whenever the ignored user list has
  changed from what was previously known, instead of triggering
  every time an ignore-user-list event has been received from sync.
  ([#4779](https://github.com/matrix-org/matrix-rust-sdk/pull/4779))
- [**breaking**] The `MediaRetentionPolicy` can now trigger regular cleanups
  with its new `cleanup_frequency` setting.
  ([#4603](https://github.com/matrix-org/matrix-rust-sdk/pull/4603))
  - `Clone` is a supertrait of `EventCacheStoreMedia`.
  - `EventCacheStoreMedia` has a new method `last_media_cleanup_time_inner`
  - There are new `'static` bounds in `MediaService` for the media cache stores
- `event_cache::store::MemoryStore` implements `Clone`.
- `BaseClient` now has a `handle_verification_events` field which is `true` by
  default and can be negated so the `NotificationClient` won't handle received
  verification events too, causing errors in the `VerificationMachine`.
- [**breaking**] `Room::is_encryption_state_synced` has been removed
  ([#4777](https://github.com/matrix-org/matrix-rust-sdk/pull/4777))
- [**breaking**] `Room::is_encrypted` is replaced by `Room::encryption_state`
  which returns a value of the new `EncryptionState` enum
  ([#4777](https://github.com/matrix-org/matrix-rust-sdk/pull/4777))

### Refactor

- [**breaking**] `BaseClient::store` is renamed `state_store`
  ([#4851](https://github.com/matrix-org/matrix-rust-sdk/pull/4851))
- [**breaking**] `BaseClient::with_store_config` is renamed `new`
  ([#4847](https://github.com/matrix-org/matrix-rust-sdk/pull/4847))
- [**breaking**] `BaseClient::set_session_metadata` is renamed
  `activate`, and `BaseClient::logged_in` is renamed `is_activated`
  ([#4850](https://github.com/matrix-org/matrix-rust-sdk/pull/4850))
- [**breaking] `DependentQueuedRequestKind::UploadFileWithThumbnail` was renamed
  to `DependentQueuedRequestKind::UploadFileOrThumbnail`. Under the
  `unstable-msc4274` feature,
  `DependentQueuedRequestKind::UploadFileOrThumbnail` and `SentMediaInfo` were
  generalized to allow chaining multiple dependent file / thumbnail uploads.
  ([#4897](https://github.com/matrix-org/matrix-rust-sdk/pull/4897))
- [**breaking**] `RoomInfo::prev_state` has been removed due to being useless.
  ([#5054](https://github.com/matrix-org/matrix-rust-sdk/pull/5054))

## [0.10.0] - 2025-02-04

### Features

- [**breaking**] `EventCacheStore` allows to control which media content is
  allowed in the media cache, and how long it should be kept, with a
  `MediaRetentionPolicy`:
  - `EventCacheStore::add_media_content()` has an extra argument,
    `ignore_policy`, which decides whether a media content should ignore the
    `MediaRetentionPolicy`. It should be stored alongside the media content.
  - `EventCacheStore` has four new methods: `media_retention_policy()`,
    `set_media_retention_policy()`, `set_ignore_media_retention_policy()` and
    `clean_up_media_cache()`.
  - `EventCacheStore` implementations should delegate media cache methods to the
    methods of the same name of `MediaService` to use the
    `MediaRetentionPolicy`. They need to implement the `EventCacheStoreMedia`
    trait that can be tested with the
    `event_cache_store_media_integration_tests!` macro.
    ([#4571](https://github.com/matrix-org/matrix-rust-sdk/pull/4571))

### Refactor

- [**breaking**] Replaced `Room::compute_display_name` with the reintroduced
  `Room::display_name()`. The new method computes a display name, or return a
  cached value from the previous successful computation. If you need a sync
  variant, consider using `Room::cached_display_name()`.
  ([#4470](https://github.com/matrix-org/matrix-rust-sdk/pull/4470))
- [**breaking**]: The reexported types `SyncTimelineEvent` and `TimelineEvent`
  have been fused into a single type `TimelineEvent`, and its field
  `push_actions` has been made `Option`al (it is set to `None` when we couldn't
  compute the push actions, because we lacked some information).
  ([#4568](https://github.com/matrix-org/matrix-rust-sdk/pull/4568))

## [0.9.0] - 2024-12-18

### Features

- Introduced support for
  [MSC4171](https://github.com/matrix-org/matrix-rust-sdk/pull/4335), enabling
  the designation of certain users as service members. These flagged users are
  excluded from the room display name calculation.
  ([#4335](https://github.com/matrix-org/matrix-rust-sdk/pull/4335))

### Bug fixes

- Fix an off-by-one error in the `ObservableMap` when the `remove()` method is
  called. Previously, items following the removed item were not shifted left by
  one position, leaving them at incorrect indices.
  ([#4346](https://github.com/matrix-org/matrix-rust-sdk/pull/4346))

## [0.8.0] - 2024-11-19

### Bug fixes

- Add more invalid characters for room aliases.
- Use the `DisplayName` struct to protect against homoglyph attacks.

### Features

- Add `BaseClient::room_key_recipient_strategy` field
- `AmbiguityCache` contains the room member's user ID.
- [**breaking**] `Media::get_thumbnail` and `MediaFormat::Thumbnail` allow to
  request an animated thumbnail They both take a `MediaThumbnailSettings`
  instead of `MediaThumbnailSize`.

- Consider knocked members to be part of the room for display name
  disambiguation.

- `Client::cross_process_store_locks_holder_name` is used everywhere:
- `StoreConfig::new()` now takes a
  `cross_process_store_locks_holder_name` argument.
- `StoreConfig` no longer implements `Default`.
- `BaseClient::new()` has been removed.
- `BaseClient::clone_with_in_memory_state_store()` now takes a
  `cross_process_store_locks_holder_name` argument.
- `BaseClient` no longer implements `Default`.
- `EventCacheStoreLock::new()` no longer takes a `key` argument.
- `BuilderStoreConfig` no longer has
  `cross_process_store_locks_holder_name` field for `Sqlite` and
  `IndexedDb`.

- Make `ObservableMap::stream` works on `wasm32-unknown-unknown`.
- Allow aborting media uploads.
- Replace the `Notification` type from Ruma in `SyncResponse` and `StateChanges`
  by a custom one.

- Introduce a `DisplayName` struct which normalizes and sanitizes
  display names.

### Refactor

- [**breaking**] Rename `DisplayName` to `RoomDisplayName`.
- Rename `AmbiguityMap` to `DisplayNameUsers`.
- Move `event_cache_store/` to `event_cache/store/` in `base`.
- Move `linked_chunk` from `matrix` to `sdk-common`.
- Move `Event` and `Gap` into `base::event_cache`.
- The ambiguity maps in `SyncResponse` are moved to `JoinedRoom` and `LeftRoom`.
- `Store::get_rooms` and `Store::get_rooms_filtered` are way faster because they
  don't acquire the lock for every room they read.

- `Store::get_rooms`, `Store::get_rooms_filtered` and `Store::get_room` are
  renamed `Store::rooms`, `Store::rooms_filtered` and `Store::room`.

- [**breaking**] `Client::get_rooms` and `Client::get_rooms_filtered` are
  renamed `Client::rooms` and `Client::rooms_filtered`.

- [**breaking**] `Client::get_stripped_rooms` has finally been removed.
- [**breaking**] The `StateStore` methods to access data in the media cache
  where moved to a separate `EventCacheStore` trait.

- [**breaking**] The `instant` module was removed, use the `ruma::time` module
  instead.

## 0.7.0

- Rename `RoomType` to `RoomState`
- Add `RoomInfo::state` accessor
- Remove `members` and `stripped_members` fields in `StateChanges`. Room member
  events are now with other state events in `state` and `stripped_state`.
- `StateStore::get_user_ids` takes a `RoomMemberships` to be able to filter the
  results by any membership state.
  - `StateStore::get_joined_user_ids` and `StateStore::get_invited_user_ids` are
    deprecated.
- `Room::members` takes a `RoomMemberships` to be able to filter the results by
  any membership state.
  - `Room::active_members` and `Room::joined_members` are deprecated.
- `RoomMember` has new methods:
  - `can_ban`
  - `can_invite`
  - `can_kick`
  - `can_redact`
  - `can_send_message`
  - `can_send_state`
  - `can_trigger_room_notification`
- Move `StateStore::get_member_event` to `StateStoreExt`
- `StateStore::get_stripped_room_infos` is deprecated. All room infos should now
  be returned by `get_room_infos`.
- `BaseClient::get_stripped_rooms` is deprecated. Use `get_rooms_filtered` with
  `RoomStateFilter::INVITED` instead.
- Add methods to `StateStore` to be able to retrieve data in batch
  - `get_state_events_for_keys`
  - `get_profiles`
  - `get_presence_events`
  - `get_users_with_display_names`
- Move `Session`, `SessionTokens` and associated methods to the `matrix`
  crate.
- Add `Room::subscribe_info`

## 0.5.1

### Bug fixes

- #664: Fix regression with push rules being applied to the own user_id only
  instead of all but the own user_id

## 0.5.0

---

## `client-crypto`


All notable changes to this project will be documented in this file.


## Unreleased

### Added

- [**breaking**] Add `CryptoStore::delete_sessions`, so Olm sessions can be
  removed from a store. Third-party `CryptoStore` implementations need to
  implement it. ([#86](https://github.com/harana/harana-matrix/issues/86))
- Add `SenderData::legacy_session` and `SenderData::with_legacy_session`, to
  read and carry over the legacy flag when a session's sender data is
  recomputed. ([#178](https://github.com/harana/harana-matrix/issues/178))

### Fixed

- Cap the number of Olm sessions kept per sender key at 8 and drop the least
  recently used ones beyond that. Sessions were only ever added, so a device
  we repeatedly failed to decrypt from grew the store without a bound.
  ([#86](https://github.com/harana/harana-matrix/issues/86))
- Keep the legacy flag on an inbound group session restored from a backup when
  a `/keys/query` later tells us about the sending device. The session used to
  be downgraded, hiding its messages wherever insecure devices are excluded.
  ([#178](https://github.com/harana/harana-matrix/issues/178))
- `Device::set_local_trust` and `OtherUserIdentity::pin_current_master_key` /
  `withdraw_verification` no longer write a stale in-memory object back over
  the store, reverting fields (or cross-signing keys) that changed in the
  meantime. ([#128](https://github.com/harana/harana-matrix/issues/128),
  [#129](https://github.com/harana/harana-matrix/issues/129))
- Unwedge a device whose first Olm session wedged before it was ever persisted;
  it used to stay wedged forever.
  ([#103](https://github.com/harana/harana-matrix/issues/103))
- Clamp Olm session timestamps read back from a pickle that lie in the future,
  so a session cannot claim to be the freshest one we own.
  ([#87](https://github.com/harana/harana-matrix/issues/87))

### Performance

- Only write the crypto `Account` back to the store when something actually
  changed it. ([#70](https://github.com/harana/harana-matrix/issues/70))

## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

### Fixed

- Upgrade Ruma to 0.16.0, fixing a deserialization issue for
  `m.key.verification.accept` events.
  ([#6628](https://github.com/matrix-org/matrix-rust-sdk/pulls/6628))

## [0.17.0] - 2026-05-08

### Security fixes

- Check the user ID in the `sender_device_keys` property of Olm-encrypted
to-device events to prevent sender spoofing by homeserver owners.
([#6553](https://github.com/matrix-org/matrix-rust-sdk/pull/6553))

  Resolves: [GHSA-wfq4-36m3-9g42](https://github.com/matrix-org/matrix-rust-sdk/security/advisories/GHSA-wfq4-36m3-9g42) / [CVE-2026-45056](https://www.cve.org/CVERecord?id=CVE-2026-45056).

### Features

- [**breaking**] Change to the stable identifiers for `m.room_key_bundle`,
  `m.history_not_shared` and `m.shared_history`. We still support reading the
  unstable identifiers.
  ([#6467](https://github.com/matrix-org/matrix-rust-sdk/pull/6467))
- Add support for MSC4385.
  ([#6164](https://github.com/matrix-org/matrix-rust-sdk/pull/6164))
  - Add new method `OlmMachine::push_secret_to_verified_devices`.
  - Pushed secrets that we receive from verified devices are added to the
    secrets inbox.
- Add `Store::{store,clear}_room_pending_key_bundle`,
  `CryptoStore::get_pending_key_bundle_details_for_room` and
  `CryptoStore::get_all_rooms_pending_key_bundle`, which can be used by
  applications to track whether they are expecting an
  [MSC4268](https://github.com/matrix-org/matrix-spec-proposals/pull/4268) key
  bundle. ([#6199](https://github.com/matrix-org/matrix-rust-sdk/pull/6199)),
  ([#6233](https://github.com/matrix-org/matrix-rust-sdk/pull/6233)),
- Add MSC4388 support to the QrcodeData struct.
  ([#6089](https://github.com/matrix-org/matrix-rust-sdk/pull/6089))
- Improved logging when we are sending secrets in `GossipMachine`.
  ([#6074](https://github.com/matrix-org/matrix-rust-sdk/pull/6074))
  ([#6083](https://github.com/matrix-org/matrix-rust-sdk/pull/6083))
- Added a new field `forwarder` to `InboundGroupSession` of type
  `ForwarderData`, which stores information about the forwarder of a session
  shared in a room key bundle under
  [MSC4268](https://github.com/matrix-org/matrix-spec-proposals/pull/4268).
  ([#5980])([https://github.com/matrix-org/matrix-rust-sdk/pull/5980][https-github-com-matrix-org-matrix-rust-sdk-pull-5980])
- The `OutboundGroupSession` and `OlmMachine` now return the `EncryptionInfo`
  used when encrypting raw events.
  ([#5936](https://github.com/matrix-org/matrix-rust-sdk/pull/5936))
- Expose a new method `CryptoStore::has_downloaded_all_room_keys`, used to track
  whether the client has previously downloaded historical room keys for a given
  room from key backup prior to building an
  [MSC4268](https://github.com/matrix-org/matrix-spec-proposals/pull/4268) room
  key bundle. ([#6017](https://github.com/matrix-org/matrix-rust-sdk/pull/6017))
  ([#6044](https://github.com/matrix-org/matrix-rust-sdk/pull/6044))

### Refactor

- Re-introduce cross-process lock generation logic in `OlmMachine`
  ([#6496](https://github.com/matrix-org/matrix-rust-sdk/pull/6496))
- [**breaking**] The `MegolmV1BackupKey::encrypt` now returns a `Result`
  ([#6477](https://github.com/matrix-org/matrix-rust-sdk/pull/6477))
- [**breaking**] `CryptoStore::get_secrets_from_inbox` now returns a `Vec` of
  the secrets as strings, rather than a `Vec` of `GossippedSecret` structs.
  ([#6164](https://github.com/matrix-org/matrix-rust-sdk/pull/6164))
- [**breaking**] `store::types::Changes::sessions` now stores a `Vec` of
  `SecretsInboxItem`.
  ([#6164](https://github.com/matrix-org/matrix-rust-sdk/pull/6164))
- **breaking** The `BackupDecryptionKey::new` and `DehydratedDeviceKey::new`
  methods became infallible, they don't return a `Result` anymore.
  ([#5502](https://github.com/matrix-org/matrix-rust-sdk/pull/5502))
- [**breaking**] Remove cross-process lock generation logic from `OlmMachine`,
  which is now implemented more generally in
  `sdk_common::cross_process_lock::CrossProcessLock`.
  ([#6326](https://github.com/matrix-org/matrix-rust-sdk/pull/6326))
- [**breaking**] The `MediaEncryptionInfo` fields changed to match the new
  fields of `EncryptedFile` from Ruma. The serialized JSON format did not change
  and still matches the format of `EncryptedFile` defined in the spec, without
  the `url` field. The `DecryptorError::KeyNonceLength` variant was removed
  because the length of the key and nonce are now enforced in
  `MediaEncryptionInfo`.
  ([#6346](https://github.com/matrix-org/matrix-rust-sdk/pull/6346))
- [**breaking**] Removed `WithLocking` from `EncryptionSyncService` and replaced
  it with `CrossProcessLockConfig`.
  ([#6160](https://github.com/matrix-org/matrix-rust-sdk/pull/6160))
- [**breaking**] The QrcodeData struct has been reworked in preparation to
  support MSC4388. The fields of the QrcodeData struct are not anymore publicly
  accessible. The `mode_data()` method has been renamed to `intent_data()` and
  returns an MSC-specific struct now. The `rendezvous_url()` method has been
  removed.
  ([#6081](https://github.com/matrix-org/matrix-rust-sdk/pull/6081))
- [**breaking**] The `message-ids` feature has been removed. It was already a
  no-op and has now been eliminated entirely.
  ([#5963](https://github.com/matrix-org/matrix-rust-sdk/pull/5963))

## [0.16.1] - 2026-05-08

### Bug fixes

- Check the user ID in the `sender_device_keys` property of Olm-encrypted
to-device events to prevent sender spoofing by homeserver owners.
([#6553](https://github.com/matrix-org/matrix-rust-sdk/pull/6553))

## [0.16.0] - 2025-12-04

### Features

- When we receive an inbound Megolm session from two different sources, merge
  the two copies together to get the best of both.
  ([#5865](https://github.com/matrix-org/matrix-rust-sdk/pull/5865)
- When constructing a key bundle for history sharing, if we had received a key
  bundle ourselves, in which one or more sessions was marked as "history not
  shared", pass that on to the new user.
  ([#5820](https://github.com/matrix-org/matrix-rust-sdk/pull/5820)
- Expose new method `CryptoStore::get_withheld_sessions_by_room_id`.
  ([#5819](https://github.com/matrix-org/matrix-rust-sdk/pull/5819))
- Use new withheld code in key bundles for sessions not marked as
  `shared_history`.
  ([#5807](https://github.com/matrix-org/matrix-rust-sdk/pull/5807),
  ([#5834](https://github.com/matrix-org/matrix-rust-sdk/pull/5834))
- Improve feedback support for shared history when downloading room key
  bundles.
  ([#5737](https://github.com/matrix-org/matrix-rust-sdk/pull/5737))
  - Add `RoomKeyWithheldEntry` enum, wrapping either a received to-device
    `m.room_key.withheld` event or its content, if derived from a downloaded
    room key bundle.
  - `OlmMachine::receive_room_key_bundle` now appends withheld key information
    to the store.
  - [**breaking**] `Changes::withheld_session_info` now stores a
    `RoomKeyWithheldEntry` in each `room-id`-`session-id` entry.
  - [**breaking**] `CryptoStore::get_withheld_info` now returns
    `Result<Option<RoomKeyWithheldEntry>>`. This change also affects
    `MemoryStore`.
- [**breaking**] Add `name` fields to some of the variants of
  `store::SecretImportError` to indicate what secret was being imported when the
  error occurred.
  ([#5647](https://github.com/matrix-org/matrix-rust-sdk/pull/5647))

### Bug fixes

- Fix a bug which caused encrypted to-device messages from unknown devices to be
  ignored. ([#5763](https://github.com/matrix-org/matrix-rust-sdk/pull/5763))
- Fix a bug which caused history shared on invite to be ignored when "exclude
  insecure devices" was enabled.
  ([#5763](https://github.com/matrix-org/matrix-rust-sdk/pull/5763))
- Fix a bug introduced in 0.14.0 which meant that the serialization of the value
  returned by `OtherUserIdentity::verification_request_content` did not include
  a `msgtype` field.
  ([#5642](https://github.com/matrix-org/matrix-rust-sdk/pull/5642))

## [0.14.0] - 2025-09-04

### Features

- Log message index for Megolm sessions received over encrypted to-device
  messages. ([#5599](https://github.com/matrix-org/matrix-rust-sdk/pull/5599))
- Add `RoomSettings::encrypt_state_events` flag.
  ([#5511](https://github.com/matrix-org/matrix-rust-sdk/pull/5511))
- Make sure to accept historic room key bundles only if the sender is trusted
  enough.
  ([#5510](https://github.com/matrix-org/matrix-rust-sdk/pull/5510))
- [**breaking**]: When in "exclude insecure devices" mode, refuse to decrypt
  incoming to-device messages from unverified devices, except for some
  exceptions for certain event types. To support this, a new variant has been
  added to `ProcessedToDeviceEvent`: `UnverifiedSender`, which is returned from
  `OlmMachine::receive_sync_changes` when we are excluding insecure devices and
  the sender's device is not verified. Also, several methods now take a
  `DecryptionSettings` argument to allow controlling the processing of to-device
  events based on those settings. To recreate the previous behaviour pass in:
  `DecryptionSettings { sender_device_trust_requirement: TrustRequirement::Untrusted }`. <!-- rumdl-disable-line MD013 -->
  Affected methods are `OlmMachine::receive_sync_changes`,
  `RehydratedDevice::receive_events`, and several internal methods.
  ([#5319](https://github.com/matrix-org/matrix-rust-sdk/pull/5319))
- [**breaking**] The `Device::encrypt_event_raw` and (experimental)
  `OlmMachine::encrypt_content_for_devices` have new `share_strategy` parameters
  to ensure that the recipients are sufficiently trusted.
  ([#5457](https://github.com/matrix-org/matrix-rust-sdk/pull/5457/))

### Refactor

- [**breaking**] The `sender_key` and `device_id` fields of
  `encrypted::MegolmV1AesSha2Content` and
  `room_key_request::MegolmV1AesSha2Content` are now optional. The have been
  deprecated in Matrix 1.3 and are no longer required.
  ([#5489](https://github.com/matrix-org/matrix-rust-sdk/pull/5489))

## [0.13.0] - 2025-07-10

### Features

- [**breaking**] Add a new `VerificationLevel::MismatchedSender` to indicate
  that the sender of an event appears to have been tampered with.
  ([#5219](https://github.com/matrix-org/matrix-rust-sdk/pull/5219))

### Refactor

- [**breaking**] The `PendingChanges`, `Changes`, `StoredRoomKeyBundleData`,
  `TrackedUser`, `IdentityChanges`, `DeviceChanges`, `DeviceUpdates`,
  `IdentityUpdates`, `BackupDecryptionKey`, `DehydratedDeviceKey`,
  `RoomKeyCounts`, `BackupKeys`, `CrossSigningKeyExport`, `UserKeyQueryResult`,
  `RoomSettings`, `RoomKeyInfo`, and `RoomKeyWithheldInfo` types have been moved
  from the `store` module into a new `store/types` module.
  ([#5177](https://github.com/matrix-org/matrix-rust-sdk/pull/5177))

## [0.12.0] - 2025-06-10

### Features

- [**breaking**] The `ProcessedToDeviceEvent::Decrypted` variant now also have
  an `EncryptionInfo` field. Format changed from
  `Decrypted(Raw<AnyToDeviceEvent>)` to
  `Decrypted { raw: Raw<AnyToDeviceEvent>, encryption_info: EncryptionInfo) }`
  ([#5074](https://github.com/matrix-org/matrix-rust-sdk/pull/5074))

- [**breaking**] Move `session_id` from `EncryptionInfo` to `AlgorithmInfo` as
  it is megolm specific. Use `EncryptionInfo::session_id()` helper for quick
  access. ([#4981](https://github.com/matrix-org/matrix-rust-sdk/pull/4981))

- Send stable identifier `sender_device_keys` for MSC4147 (Including device
  keys with Olm-encrypted events).
  ([#4964](https://github.com/matrix-org/matrix-rust-sdk/pull/4964))

- Add experimental APIs for sharing encrypted room key history with new members,
  `Store::build_room_key_bundle` and `OlmMachine::share_room_key_bundle_data`.
  ([#4775](https://github.com/matrix-org/matrix-rust-sdk/pull/4775),
  [#4864](https://github.com/matrix-org/matrix-rust-sdk/pull/4864))

- Check the `sender_device_keys` field on _all_ incoming Olm-encrypted to-device
  messages and ignore any to-device messages which include the field but whose
  data is invalid (as per
  [MSC4147](https://github.com/matrix-org/matrix-spec-proposals/pull/4147)).
  ([#4922](https://github.com/matrix-org/matrix-rust-sdk/pull/4922))

- Fix bug which caused room keys to be unnecessarily rotated on every send in
  the presence of blacklisted/withheld devices in the room.
  ([#4954](https://github.com/matrix-org/matrix-rust-sdk/pull/4954))

- Fix [#2729](https://github.com/matrix-org/matrix-rust-sdk/issues/2729) which
  in rare cases can cause room key oversharing.
  ([#4975](https://github.com/matrix-org/matrix-rust-sdk/pull/4975))

- [**breaking**] `OlmMachine.receive_sync_changes` returns now a list of
  `ProcessedToDeviceEvent` instead of a list of `Raw<AnyToDeviceEvent>`. With
  variants like `Decrypted`|`UnableToDecrypt`|`PlainText`|`NotProcessed`. This
  allows for example to make the difference between an event sent in clear and
  an event successfully decrypted. For quick compatibility a helper
  `ProcessedToDeviceEvent::to_raw` allows to map back to the previous behaviour.
  ([#4935](https://github.com/matrix-org/matrix-rust-sdk/pull/4935))

## [0.11.1] - 2025-06-10

### Security fixes

- Check the sender of an event matches owner of session, preventing sender
  spoofing by homeserver owners.
  [13c1d20](https://github.com/matrix-org/matrix-rust-sdk/commit/13c1d2048286bbabf5e7bc6b015aafee98f04d55)
  (High, [CVE-2025-48937](https://www.cve.org/CVERecord?id=CVE-2025-48937),
  [GHSA-x958-rvg6-956w](https://github.com/matrix-org/matrix-rust-sdk/security/advisories/GHSA-x958-rvg6-956w)).

### Bug fixes

- Remove a wildcard enum variant import which breaks compilation if used with
  `tracing-attributes` version `0.1.29`. This is a workaround for a bug in
  `tracing-attributes`.
  ([#5190](https://github.com/matrix-org/matrix-rust-sdk/issues/5190))
  ([#5191](https://github.com/matrix-org/matrix-rust-sdk/issues/5191))
  ([#5193](https://github.com/matrix-org/matrix-rust-sdk/issues/5193))

## [0.11.0] - 2025-04-11

### Features

- [**breaking**] Add support for the shared history flag defined in
  [MSC3061](https://github.com/matrix-org/matrix-spec-proposals/pull/3061).
  The shared history flag is now respected when room keys are received as an
  `m.room_key` event as well as when they are imported from a backup or a file
  export. We also ensure to set the flag when we send out room keys. Due to
  this, a new argument to the constructor for `room_key::MegolmV1AesSha2Content`
  has been added and `PickledInboundGroupSession` has received a new
  `shared_history` field that defaults to `false.`
  ([#4700](https://github.com/matrix-org/matrix-rust-sdk/pull/4700))

- Have the `RoomIdentityProvider` return processing changes when identities
  transition to `IdentityState::Verified` too.
  ([#4670](https://github.com/matrix-org/matrix-rust-sdk/pull/4670))

## [0.10.0] - 2025-02-04

### Features

- [**breaking**] `CollectStrategy::DeviceBasedStrategy` is now split into three
  separate strategies (`AllDevices`, `ErrorOnVerifiedUserProblem`,
  `OnlyTrustedDevices`), to make the behaviour clearer.
  ([#4581](https://github.com/matrix-org/matrix-rust-sdk/pull/4581))

- Accept stable identifier `sender_device_keys` for MSC4147 (Including device
  keys with Olm-encrypted events).
  ([#4420](https://github.com/matrix-org/matrix-rust-sdk/pull/4420))

- Room keys are not shared with unsigned dehydrated devices.
  ([#4551](https://github.com/matrix-org/matrix-rust-sdk/pull/4551))

## [0.9.0] - 2024-12-18

### Features

- [**breaking**] Expose new API
  `DehydratedDevices::get_dehydrated_device_pickle_key`,
  `DehydratedDevices::save_dehydrated_device_pickle_key` and
  `DehydratedDevices::delete_dehydrated_device_pickle_key` to store/load the
  dehydrated device pickle key. This allows client to automatically rotate
  the dehydrated device to avoid one-time-keys exhaustion and to_device
  accumulation.
  `DehydratedDevices::keys_for_upload` and
  `DehydratedDevices::rehydrate` now use the `DehydratedDeviceKey` as parameter
  instead of a raw byte array. Use `DehydratedDeviceKey::from_bytes` to migrate.
  ([#4383](https://github.com/matrix-org/matrix-rust-sdk/pull/4383))

- Add extra logging in `OtherUserIdentity::pin_current_master_key` and
  `OtherUserIdentity::withdraw_verification`.
  ([#4415](https://github.com/matrix-org/matrix-rust-sdk/pull/4415))

- Added new `UtdCause` variants `WithheldForUnverifiedOrInsecureDevice` and
  `WithheldBySender`. These variants provide clearer categorization for expected
  Unable-To-Decrypt (UTD) errors when the sender either did not wish to share or
  was unable to share the room_key.
  ([#4305](https://github.com/matrix-org/matrix-rust-sdk/pull/4305))

- `UtdCause` has two new variants that replace the existing `HistoricalMessage`:
  `HistoricalMessageAndBackupIsDisabled` and
  `HistoricalMessageAndDeviceIsUnverified`. These give more detail about what
  went wrong and allow us to suggest to users what actions they can take to fix
  the problem. See the doc comments on these variants for suggested wording.
  ([#4384](https://github.com/matrix-org/matrix-rust-sdk/pull/4384))

## [0.8.0] - 2024-11-19

### Features

- Pin identity when we withdraw verification.
- Expose new method `OlmMachine::room_keys_withheld_received_stream`, to allow
  applications to receive notifications about received `m.room_key.withheld`
  events.
  ([#3660](https://github.com/matrix-org/matrix-rust-sdk/pull/3660)),
  ([#3674](https://github.com/matrix-org/matrix-rust-sdk/pull/3674))

- Expose new method `OlmMachine::clear_crypto_cache()`, with FFI bindings.
  ([#3462](https://github.com/matrix-org/matrix-rust-sdk/pull/3462))

- Expose new method `OlmMachine::upload_device_keys()`.
  ([#3457](https://github.com/matrix-org/matrix-rust-sdk/pull/3457))

- Expose new method `CryptoStore::import_room_keys`.
  ([#3448](https://github.com/matrix-org/matrix-rust-sdk/pull/3448))

- Expose new method `BackupMachine::backup_version`.
  ([#3320](https://github.com/matrix-org/matrix-rust-sdk/pull/3320))

- Add data types to parse the QR code data for the QR code login defined in.
  [MSC4108](https://github.com/matrix-org/matrix-spec-proposals/pull/4108)

- Expose new method `CryptoStore::clear_caches`.
  ([#3338](https://github.com/matrix-org/matrix-rust-sdk/pull/3338))

- Expose new method `OlmMachine::device_creation_time`.
  ([#3275](https://github.com/matrix-org/matrix-rust-sdk/pull/3275))

- Log more details about the Olm session after encryption and decryption.
  ([#3242](https://github.com/matrix-org/matrix-rust-sdk/pull/3242))

- When Olm message decryption fails, report the error code(s) from the failure.
  ([#3212](https://github.com/matrix-org/matrix-rust-sdk/pull/3212))

- Expose new methods `OlmMachine::set_room_settings` and
  `OlmMachine::get_room_settings`.
  ([#3042](https://github.com/matrix-org/matrix-rust-sdk/pull/3042))

- Add new properties `session_rotation_period` and
  `session_rotation_period_msgs` to `store::RoomSettings`.
  ([#3042](https://github.com/matrix-org/matrix-rust-sdk/pull/3042))

- Fix bug which caused `SecretStorageKey` to incorrectly reject secret storage
  keys whose metadata lacked check fields.
  ([#3046](https://github.com/matrix-org/matrix-rust-sdk/pull/3046))

- Add new API `Device::encrypt_event_raw` that allows
  to encrypt an event to a specific device.
  ([#3091](https://github.com/matrix-org/matrix-rust-sdk/pull/3091))

- Add new API `store::Store::export_room_keys_stream` that provides room
  keys on demand.

- Include event timestamps on logs from event decryption.
  ([#3194](https://github.com/matrix-org/matrix-rust-sdk/pull/3194))

### Refactor

- Fix [#4424](https://github.com/matrix-org/matrix-rust-sdk/issues/4424) Failed
  storage upgrade for "PreviouslyVerifiedButNoLonger". This bug caused errors to
  occur when loading crypto information from storage, which typically prevented
  apps from starting correctly.
  ([#4430](https://github.com/matrix-org/matrix-rust-sdk/pull/4430))

- Add new method `OlmMachine::try_decrypt_room_event`.
  ([#4116](https://github.com/matrix-org/matrix-rust-sdk/pull/4116))

- Add reason code to
  `sdk_common::deserialized_responses::UnableToDecryptInfo`.
  ([#4116](https://github.com/matrix-org/matrix-rust-sdk/pull/4116))

- [**breaking**] The `UserIdentity` struct has been renamed to
  `OtherUserIdentity`.
  ([#4036](https://github.com/matrix-org/matrix-rust-sdk/pull/4036]))

- [**breaking**] The `UserIdentities` enum has been renamed to `UserIdentity`.
  ([#4036](https://github.com/matrix-org/matrix-rust-sdk/pull/4036]))

- Change the withheld code for keys not shared due to the
  `IdentityBasedStrategy`, from `m.unauthorised` to `m.unverified`.
  ([#3985](https://github.com/matrix-org/matrix-rust-sdk/pull/3985))

- Improve logging for undecryptable Megolm events.
  ([#3989](https://github.com/matrix-org/matrix-rust-sdk/pull/3989))

- Miscellaneous improvements to logging for verification and `OwnUserIdentity`
  updates.
  ([#3949](https://github.com/matrix-org/matrix-rust-sdk/pull/3949))

- Update `SenderData` on existing inbound group sessions when we receive
  updates via `/keys/query`.
  ([#3849](https://github.com/matrix-org/matrix-rust-sdk/pull/3849))

- Add message IDs to all outgoing to-device messages encrypted by
  `crypto`. The `message-ids` feature of `crypto` and
  `base` is now a no-op.
  ([#3776](https://github.com/matrix-org/matrix-rust-sdk/pull/3776))

- Log the content of received `m.room_key.withheld` to-device events.
  ([#3591](https://github.com/matrix-org/matrix-rust-sdk/pull/3591))

- Attempt to decrypt bundled events (reactions and the latest thread reply) if
  they are found in the unsigned part of an event.
  ([#3468](https://github.com/matrix-org/matrix-rust-sdk/pull/3468))

- Sign the device keys with the user-identity (i.e. cross-signing keys) if
  we're uploading the device keys and if the cross-signing keys are available.
  This approach eliminates the need to upload signatures in a separate request,
  ensuring that other users/devices will never encounter this device without a
  signature from their user identity. Consequently, they will never see the
  device as unverified.
  ([#3453](https://github.com/matrix-org/matrix-rust-sdk/pull/3453))

- Avoid emitting entries from `identities_stream_raw` and `devices_stream` when
  we receive a `/keys/query` response which shows that no devices changed.
  ([#3442](https://github.com/matrix-org/matrix-rust-sdk/pull/3442))

- Fallback keys are rotated in a time-based manner, instead of waiting for the
  server to tell us that a fallback key got used.
  ([#3151](https://github.com/matrix-org/matrix-rust-sdk/pull/3151))

Breaking changes:

- [**breaking**] `VerificationRequestState::Transitioned` now includes a new
  field `other_device_data` of type `DeviceData`.
  ([#4153](https://github.com/matrix-org/matrix-rust-sdk/pull/4153))

- [**breaking**] `OlmMachine::decrypt_room_event` now returns a
  `DecryptedRoomEvent` type, instead of the more generic `TimelineEvent` type.

- [**breaking**] **NOTE**: this version causes changes to the format of the
  serialised data in the CryptoStore, meaning that, once upgraded, it will not
  be possible to roll back applications to earlier versions without breaking
  user sessions.

- [**breaking**] Renamed `VerificationLevel::PreviouslyVerified` to
  `VerificationLevel::VerificationViolation`.

- [**breaking**] `OlmMachine::decrypt_room_event` now takes a
  `DecryptionSettings` argument, which includes a `TrustRequirement` indicating
  the required trust level for the sending device. When it is called with
  `TrustRequirement` other than `TrustRequirement::Unverified`, it may return
  the new `MegolmError::SenderIdentityNotTrusted` variant if the sending device
  does not satisfy the required trust level.
  ([#3899](https://github.com/matrix-org/matrix-rust-sdk/pull/3899))

- [**breaking**] Change the structure of the `SenderData` enum to separate
  variants for previously-verified, unverified and verified.
  ([#3877](https://github.com/matrix-org/matrix-rust-sdk/pull/3877))

- [**breaking**] Where `EncryptionInfo` is returned it may include the new
  `PreviouslyVerified` variant of `VerificationLevel` to indicate that the user
  was previously verified and is no longer verified.
  ([#3877](https://github.com/matrix-org/matrix-rust-sdk/pull/3877))

- [**breaking**] Expose new methods `OwnUserIdentity::was_previously_verified`,
  `OwnUserIdentity::withdraw_verification`, and
  `OwnUserIdentity::has_verification_violation`, which track whether our own
  identity was previously verified.
  ([#3846](https://github.com/matrix-org/matrix-rust-sdk/pull/3846))

- [**breaking**] Add a new `error_on_verified_user_problem` property to
  `CollectStrategy::DeviceBasedStrategy`, which, when set, causes
  `OlmMachine::share_room_key` to fail with an error if any verified users on
  the recipient list have unsigned devices, or are no longer verified.

  When `CallectStrategy::IdentityBasedStrategy` is used,
  `OlmMachine::share_room_key` will fail with an error if any verified users on
  the recipient list are no longer verified, or if our own device is not
  properly cross-signed.

  Also remove `CollectStrategy::new_device_based`: callers should construct a
  `CollectStrategy::DeviceBasedStrategy` directly.

  `EncryptionSettings::new` now takes a `CollectStrategy` argument, instead of a
  list of booleans.
  ([#3810](https://github.com/matrix-org/matrix-rust-sdk/pull/3810))
  ([#3816](https://github.com/matrix-org/matrix-rust-sdk/pull/3816))
  ([#3896](https://github.com/matrix-org/matrix-rust-sdk/pull/3896))

- [**breaking**] Remove the method `OlmMachine::clear_crypto_cache()`, crypto
  stores are not supposed to have any caches anymore.

- [**breaking**] Add a `custom_account` argument to the
  `OlmMachine::with_store()` method, this allows users to learn their identity
  keys before they get access to the user and device ID.
  ([#3451](https://github.com/matrix-org/matrix-rust-sdk/pull/3451))

- [**breaking**] Add a `backup_version` argument to `CryptoStore`'s
  `inbound_group_sessions_for_backup`,
  `mark_inbound_group_sessions_as_backed_up` and `inbound_group_session_counts`
  methods. ([#3253](https://github.com/matrix-org/matrix-rust-sdk/pull/3253))

- [**breaking**] Rename the `OlmMachine::invalidate_group_session` method to
  `OlmMachine::discard_room_key`.

- [**breaking**] Move `OlmMachine::export_room_keys` to
  `crypto::store::Store`. (Call it with
  `olm_machine.store().export_room_keys(...)`.)

- [**breaking**] Add new `dehydrated` property to
  `olm::account::PickledAccount`.
  ([#3164](https://github.com/matrix-org/matrix-rust-sdk/pull/3164))

- [**breaking**] Remove deprecated `OlmMachine::import_room_keys`.
  ([#3448](https://github.com/matrix-org/matrix-rust-sdk/pull/3448))

- [**breaking**] Add the `SasState::Created` variant to differentiate the state
  between the party that sent the verification start and the party that received
  it.

- [**breaking**] Deprecate `BackupMachine::import_backed_up_room_keys`.
  ([#3448](https://github.com/matrix-org/matrix-rust-sdk/pull/3448))

## 0.7.2

### Security fixes

- Fix `UserIdentity::is_verified` to take into account our own identity
  [#d8d9dae](https://github.com/matrix-org/matrix-rust-sdk/commit/d8d9dae9d77bee48a2591b9aad9bd2fa466354cc)
  (Moderate,
  [GHSA-4qg4-cvh2-crgg](https://github.com/matrix-org/matrix-rust-sdk/security/advisories/GHSA-4qg4-cvh2-crgg)).

## 0.7.1

### Security fixes

- Don't log the private part of the backup key, introduced in
  [#71136e4](https://github.com/matrix-org/matrix-rust-sdk/commit/71136e44c03c79f80d6d1a2446673bc4d53a2067).

## 0.7.0

- Add method to mark a list of inbound group sessions as backed up:
  `CryptoStore::mark_inbound_group_sessions_as_backed_up`

- `OlmMachine::toggle_room_key_forwarding` is replaced by two separate methods:

  - `OlmMachine::set_room_key_requests_enabled`, which controls whether
    outgoing room key requests are enabled, and:

  - `OlmMachine::set_room_key_forwarding_enabled`, which controls whether we
    automatically reply to incoming room key requests.

  `OlmMachine::is_room_key_forwarding_enabled` is updated to return the setting
  of `OlmMachine::set_room_key_forwarding_enabled`, while
  `OlmMachine::are_room_key_requests_enabled` is added to return the setting of
  `OlmMachine::set_room_key_requests_enabled`.

  ([#2902](https://github.com/matrix-org/matrix-rust-sdk/pull/2902))

- Improve performance of `share_room_key`.
  ([#2862](https://github.com/matrix-org/matrix-rust-sdk/pull/2862))

- `get_missing_sessions`: Don't block waiting for `/keys/query` requests on
  blacklisted servers, and improve performance.
  ([#2845](https://github.com/matrix-org/matrix-rust-sdk/pull/2845))

- Generalize `olm::Session::encrypt` to accept any value implementing
  `Serialize` for the `value` parameter, instead of specifically
  `serde_json::Value`. Note that references to `Serialize`-implementing types
  themselves implement `Serialize`.

- Change the argument to `OlmMachine::receive_sync_changes` to be an
  `EncryptionSyncChanges` struct packing all the arguments instead of many
  single arguments. The new `next_batch_token` field there should be the
  `next_batch` value read from the latest sync response.

- Handle missing devices in `/keys/claim` responses.
  ([#2805](https://github.com/matrix-org/matrix-rust-sdk/pull/2805))

- Add the higher level decryption method `decrypt_session_data` to the
  `BackupDecryptionKey` type.

- Add a higher level method to create signatures for the backup info. The
  `OlmMachine::backup_machine()::sign_backup()` method can be used to add
  signatures to a `RoomKeyBackupInfo`.

- Remove the `backups_v1` feature, backups support is now enabled by default.
- Use the `Signatures` type as the return value for the
  `MegolmV1BackupKey::signatures()` method.

- Add two new methods to import room keys,
  `OlmMachine::store()::import_exported_room_keys()` for file exports and
  `OlmMachine::backup_machine()::import_backed_up_room_keys()` for backups. The
  `OlmMachine::import_room_keys()` method is now deprecated.

- The parameter order of `OlmMachine::encrypt_room_event_raw` and
  `OutboundGroupSession::encrypt` has changed, `content` is now last
  - The parameter type of `content` has also changed, from `serde_json::Value`
    to `&Raw<AnyMessageLikeEventContent>`

- Change the return value of `bootstrap_cross_signing` so it returns an extra
  keys upload request. The three requests must be sent in the order they
  appear in the return tuple.

- Stop logging large quantities of data about the `Store` during olm
  decryption.

- Remove spurious "Unknown outgoing secret request" warning which was logged
  for every outgoing secret request.

- Clean up the logging of to-device messages in `share_room_key`.
- Expose new `OlmMachine::get_room_event_encryption_info` method.
- Add support for secret storage.
- Add initial support for MSC3814 - dehydrated devices.
- Mark our `OwnUserIdentity` as verified if we successfully import the matching
  private keys.

- The `OlmMachine::export_cross_signing_keys()` method now returns a `Result`.
  This removes an `unwrap()` from the codebase.

- Add support for the `hkdf-hmac-sha256.v2` SAS message authentication code.
- Ensure that the correct short authentication strings are used when accepting a
  SAS verification with the `Sas::accept()` method.

- Add a new optional `message-ids` feature which adds a unique ID to the content
  of `m.room.encrypted` event contents which get sent out.

- Disable the automatic-key-forwarding feature by default.
- Add a new variant to the `VerificationRequestState` enum called
  `Transitioned`. This enum variant is used when a `VerificationRequest`
  transitions into a concrete `Verification` object. The concrete `Verification`
  object is given as associated data in the `Transitioned` enum variant.

- Replace the libolm backup encryption code with a native Rust version. This
  adds WASM support to the backups_v1 feature.

- Add new API `store::Store::room_keys_received_stream` to provide
  updates of room keys being received.

- Add new method `identities::device::Device::first_time_seen_ts`
  that allows to get a local timestamp of when the device was first seen by
  the sdk (in seconds since epoch).

- When rejecting a key-verification request over to-device messages, send the
  `m.key.verification.cancel` to the device that made the request, rather than
  broadcasting to all devices.

- Expose `VerificationRequest::time_remaining`.
- For verification-via-emojis, return the word "Aeroplane" rather than
  "Airplane", for consistency with the Matrix spec.

- Fix handling of SAS verification start events once we have shown a QR code.
- Fix a bug which could cause generated one-time-keys not to be persisted.
- Fix parsing error for `POST /_matrix/client/v3/keys/signatures/upload`
  responses generated by Synapse.

- Add new API `OlmMachine::query_keys_for_users` for generating out-of-band key
  queries.

- Rename "recovery key" to "backup decryption key" to avoid confusion with the
  secret-storage key which is also known as a recovery key.

  This affects the `crypto::store::RecoveryKey` struct itself (now
  renamed to `BackupDecryptionKey`, as well as
  `BackupMachine::save_recovery_key` (now `save_decryption_key`).

- Change the returned success value type of `BackupMachine::backup` from
  `OutgoingRequest` to `(OwnedTransactionId, KeysBackupRequest)`.

[https-github-com-matrix-org-matrix-rust-sdk-pull-5980]: https://github.com/matrix-org/matrix-rust-sdk/pull/5980

---

## `client-qrcode`


All notable changes to this project will be documented in this file.


## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

No significant changes.

## [0.17.0] - 2026-05-08

No notable changes in this release.

## [0.16.1] - 2026-05-08

No notable changes in this release.

## [0.16.0] - 2025-12-04

No notable changes in this release.

## [0.14.0] - 2025-09-04

No notable changes in this release.

## [0.13.0] - 2025-07-10

No notable changes in this release.

## [0.12.0] - 2025-06-10

No notable changes in this release.

## [0.11.0] - 2025-04-11

No notable changes in this release.

## [0.10.0] - 2025-02-04

No notable changes in this release.

## [0.9.0] - 2024-12-18

No notable changes in this release.

## [0.8.0] - 2024-11-19

No notable changes in this release.

---

## `client-store-encryption`


All notable changes to this project will be documented in this file.


## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

No significant changes.

## [0.17.0] - 2026-05-08

### Refactor

- **breaking** The `Random` error variant has been removed. An infallible random
  number generator is used in the crate.
  ([#5502](https://github.com/matrix-org/matrix-rust-sdk/pull/5502))

## [0.16.1] - 2026-05-08

No notable changes in this release.

## [0.16.0] - 2025-12-04

No notable changes in this release.

## [0.14.0] - 2025-09-04

No notable changes in this release.

## [0.13.0] - 2025-07-10

No notable changes in this release.

## [0.12.0] - 2025-06-10

No notable changes in this release.

## [0.11.0] - 2025-04-11

No notable changes in this release.

## [0.10.0] - 2025-02-04

### Bug fixes

- Remove the usage of an unwrap in the `StoreCipher::import_with_key` method.
  This could have lead to panics if the second argument was an invalid
  `StoreCipher` export.
  ([#4506](https://github.com/matrix-org/matrix-rust-sdk/pull/4506))

## [0.9.0] - 2024-12-18

No notable changes in this release.

## [0.8.0] - 2024-11-19

No notable changes in this release.

---

## `client-sqlite`


All notable changes to this project will be documented in this file.


## Unreleased

### Added

- Add `OpenStoreError::is_database_corruption` and the
  `OpenStoreError::RemoveCorruptedDatabase` variant.
  ([#244](https://github.com/harana/harana-matrix/issues/244))

### Fixed

- Recreate the media store and event cache databases from scratch when they
  turn out to be corrupted, instead of failing every open with "database disk
  image is malformed". ([#244](https://github.com/harana/harana-matrix/issues/244))

## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

No significant changes.

## [0.17.0] - 2026-05-08

### Features

- Implement `CryptoStore::get_pending_key_bundle_details_for_room` and
  `CryptoStore::get_all_rooms_pending_key_bundle`, and process
  `rooms_pending_key_bundle` field in `Changes`.
  ([#6199](https://github.com/matrix-org/matrix-rust-sdk/pull/6199)),
  ([#6233](https://github.com/matrix-org/matrix-rust-sdk/pull/6233))
- Implement new method `CryptoStore::has_downloaded_all_room_keys`, and process
  `room_key_backups_fully_downloaded` field in `Changes`.
  ([#6017](https://github.com/matrix-org/matrix-rust-sdk/pull/6017))
  ([#6044](https://github.com/matrix-org/matrix-rust-sdk/pull/6044))
- [**breaking**] In `EventCacheStore::handle_linked_chunk_updates`, new chunks
  may no longer reference chunk identifiers which do not yet exist in the store
  ([#6061](https://github.com/matrix-org/matrix-rust-sdk/pull/6061))

### Bug fixes

- Fix a panic when the SQLite connection is aborted.
  ([#6091](https://github.com/matrix-org/matrix-rust-sdk/pull/6091))

### Refactor

- Add migration to `SqliteCryptoStore` that removes cross-process lock
  generation key from `kv` table, as this is tracked in `lease_locks` table.
  ([#6326](https://github.com/matrix-org/matrix-rust-sdk/pull/6326))

## [0.16.1] - 2026-05-08

No notable changes in this release.

## [0.16.0] - 2025-12-04

### Features

- Implement new method `CryptoStore::get_withheld_sessions_by_room_id`.
  ([#5819](https://github.com/matrix-org/matrix-rust-sdk/pull/5819))
- [**breaking**] `SqliteCryptoStore::get_withheld_info` now returns
  `Result<Option<RoomKeyWithheldEntry>>`.
  ([#5737](https://github.com/matrix-org/matrix-rust-sdk/pull/5737))
- Implement a new constructor that allows to open `SqliteCryptoStore` with a
  cryptographic key
  ([#5472](https://github.com/matrix-org/matrix-rust-sdk/pull/5472))
- Implement `StateStore::upsert_thread_subscriptions()` method for bulk upserts.
  ([#5848](https://github.com/matrix-org/matrix-rust-sdk/pull/5848))

### Refactor

- [breaking] Change the logic for opening a store so as to use a `Secret` enum
  in the function `open_with_pool` instead of a `passphrase`
  ([#5472](https://github.com/matrix-org/matrix-rust-sdk/pull/5472))

## [0.14.0] - 2025-09-04

No notable changes in this release.

## [0.13.0] - 2025-07-10

### Security fixes

- Fix SQL injection vulnerability in `find_event_relations()`.
  ([d0c0100](https://github.com/matrix-org/matrix-rust-sdk/commit/d0c01006e4808db5eb96ad5c496416f284d8bd3c),
  Moderate, [CVE-2025-53549](https://www.cve.org/CVERecord?id=CVE-2025-53549),
  [GHSA-275g-g844-73jh](https://github.com/matrix-org/matrix-rust-sdk/security/advisories/GHSA-275g-g844-73jh))

## [0.12.0] - 2025-06-10

### Bug fixes

- Fix a `UNIQUE` constraint violation in the event cache store
  ([#5001](https://github.com/matrix-org/matrix-rust-sdk/pull/5001))

## [0.11.0] - 2025-04-11

### Features

- Implement the new method of `EventCacheStoreMedia` for
  `SqliteEventCacheStore`.
  ([#4603](https://github.com/matrix-org/matrix-rust-sdk/pull/4603))
- Defragment an sqlite state store after removing a room.
  ([#4651](https://github.com/matrix-org/matrix-rust-sdk/pull/4651))
- Add `SqliteStoreConfig` and the `open_with_config` constructor on all the
  stores, it allows to control the maximum size of the pool of connections to
  SQLite for example.
  ([#4826](https://github.com/matrix-org/matrix-rust-sdk/pull/4826))
- Add `SqliteStoreConfig::path()` to override the path given to the constructor
  ([#4870](https://github.com/matrix-org/matrix-rust-sdk/pull/4870/))
- Implement `Clone` and `Debug` on `SqliteStoreConfig`
  ([#4870](https://github.com/matrix-org/matrix-rust-sdk/pull/4870/))
- Add `SqliteStoreConfig::with_low_memory_config` constructor
  ([#4894](https://github.com/matrix-org/matrix-rust-sdk/pull/4894))

## [0.10.0] - 2025-02-04

### Features

- [**breaking**] `SqliteEventCacheStore` implements the new APIs of
  `EventCacheStore` for `MediaRetentionPolicy`. See the changelog of
  `base` for more details.
  ([#4571](https://github.com/matrix-org/matrix-rust-sdk/pull/4571))
- The SQLite databases are optimized during the construction of the stores. It
  should improve the performance of the queries.
  ([#4602](https://github.com/matrix-org/matrix-rust-sdk/pull/4602))
- The size of the WAL files is now limited to 10MB. This avoids cases where the
  WAL file takes as much space as the database.
  ([#4602](https://github.com/matrix-org/matrix-rust-sdk/pull/4602))

## [0.9.0] - 2024-12-18

### Features

- Add support for persisting LinkedChunks in the SQLite store. This is a step
  towards implementing event cache support, enabling a persisted cache of
  events. ([#4340](https://github.com/matrix-org/matrix-rust-sdk/pull/4340))
  ([#4362](https://github.com/matrix-org/matrix-rust-sdk/pull/4362))

## [0.8.0] - 2024-11-19

### Bug fixes

- Use the `DisplayName` struct to protect against homoglyph attacks.

### Refactor

- Move `event_cache_store/` to `event_cache/store/` in `base`.

---

## `client-indexeddb`


All notable changes to this project will be documented in this file.


## Unreleased

### Fixed

- Commit the transactions that carry Olm session or account state with strict
  durability, so a crash or power cut cannot leave the persisted session behind
  the one the peer has already ratcheted past.
  ([#99](https://github.com/harana/harana-matrix/issues/99))

## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

No significant changes.

## [0.17.0] - 2026-05-08

### Features

- Add support in the implementation of `EventCacheStore` for
  having duplicate events in a room, where each duplicate is in a different
  `LinkedChunk`. This is useful, e.g., when an event is in a room and a
  thread in that room. The change involves a database migration where
  the `EVENTS` object store is cleared and then modified so that the
  `ROOM` index no longer requires keys to be unique.
  ([#6200](https://github.com/matrix-org/matrix-rust-sdk/pull/6200))
- Implement `CryptoStore::get_pending_key_bundle_details_for_room` and
  `CryptoStore::get_all_rooms_pending_key_bundle`, and process
  `rooms_pending_key_bundle` field in `Changes`.
  ([#6199](https://github.com/matrix-org/matrix-rust-sdk/pull/6199)),
  ([#6233](https://github.com/matrix-org/matrix-rust-sdk/pull/6233))
- Expose implementations of `EventCacheStore` and `MediaStore` and add a
  composite type for initializing all stores with a single function - i.e.,
  `IndexeddbStores::open`. Additionally, allow feature flags for each of the
  stores to be used independent of and in combination with the others.
  ([#5946](https://github.com/matrix-org/matrix-rust-sdk/pull/5946))
- Implement new method `CyptoStore::has_downloaded_all_room_keys`, and process
  `room_key_backups_fully_downloaded` field in `Changes`.
  ([#6017](https://github.com/matrix-org/matrix-rust-sdk/pull/6017))
  ([#6044](https://github.com/matrix-org/matrix-rust-sdk/pull/6044))
- [**breaking**] In `EventCacheStore::handle_linked_chunk_updates`, new chunks
  may no longer reference chunk identifiers which do not yet exist in the store
  ([#6061](https://github.com/matrix-org/matrix-rust-sdk/pull/6061))

### Bug fixes

- Ensure that encrypted tests are run with a `StoreCipher`. This happened to
  reveal tests which fail in an encrypted `EventCacheStore`, which required
  fixing queries for all events in a room.
  ([#5933](https://github.com/matrix-org/matrix-rust-sdk/pull/5933))

### Refactor

- Add migration to `IndexeddbCryptoStore` that removes cross-process lock
  generation key from `CORE` object store, as this is tracked in `LEASE_LOCKS`
  object store.
  ([#6326](https://github.com/matrix-org/matrix-rust-sdk/pull/6326))

## [0.16.1] - 2026-05-08

No notable changes in this release.

## [0.16.0] - 2025-12-04

### Features

- Implement new method `CryptoStore::get_withheld_sessions_by_room_id`.
  ([#5819](https://github.com/matrix-org/matrix-rust-sdk/pull/5819))
- [**breaking**] `IndexeddbCryptoStore::get_withheld_info` now returns
  `Result<Option<RoomKeyWithheldEntry>, ...>`.
  ([#5737](https://github.com/matrix-org/matrix-rust-sdk/pull/5737))
- Implement `StateStore::upsert_thread_subscriptions()` method for bulk upserts.
  ([#5848](https://github.com/matrix-org/matrix-rust-sdk/pull/5848))

### Performance

- Improve performance of certain media queries in `MediaStore` implementation by
  storing media content and media metadata in separate object stores in
  IndexedDB (see
  [#5795](https://github.com/matrix-org/matrix-rust-sdk/pull/5795)).

## [0.14.0] - 2025-09-04

No notable changes in this release.

## [0.13.0] - 2025-07-10

### Features

- Add support for received room key bundle data, as required by encrypted
  history sharing
  ((MSC4268)[[https://github.com/matrix-org/matrix-spec-proposals/pull/4268][https-github-com-matrix-org-matrix-spec-proposals-pull-4268])).
  ([#5276](https://github.com/matrix-org/matrix-rust-sdk/pull/5276))

## [0.12.0] - 2025-06-10

No notable changes in this release.

## [0.11.0] - 2025-04-11

No notable changes in this release.

## [0.10.0] - 2025-02-04

No notable changes in this release.

## [0.9.0] - 2024-12-18

No notable changes in this release.

## [0.8.0] - 2024-11-19

### Features

- Improve the efficiency of objects stored in the crypto store.
  ([#3645](https://github.com/matrix-org/matrix-rust-sdk/pull/3645),
  [#3651](https://github.com/matrix-org/matrix-rust-sdk/pull/3651))

- Add new method `IndexeddbCryptoStore::open_with_key`.
  ([#3423](https://github.com/matrix-org/matrix-rust-sdk/pull/3423))

- `save_change` performance improvement, all encryption and serialization
  is done now outside of the db transaction.

### Bug fixes

- Use the `DisplayName` struct to protect against homoglyph attacks.

[https-github-com-matrix-org-matrix-spec-proposals-pull-4268]: https://github.com/matrix-org/matrix-spec-proposals/pull/4268

---

## `client-search`


All notable changes to this project will be documented in this file.


## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

No significant changes.

## [0.17.0] - 2026-05-08

No notable changes in this release.

## [0.16.1] - 2026-05-08

No notable changes in this release.

## [0.16.0] - 2025-12-04

No notable changes in this release.

## [0.14.0] - 2025-09-04

Initial release of the search crate

---

## `client-ui`


All notable changes to this project will be documented in this file.


## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

### Changed

- [**breaking**] `SpaceRoom::new_from_known` and `SpaceRoom::new_from_summary`
  are now asynchronous so we can properly check if they are DMs on demand
  instead of trusting the pre-computed value. Some other related functions are
  now `async` too.
  ([#6561](https://github.com/matrix-org/matrix-rust-sdk/pulls/6561))

### Fixed

- Remove the ability to reply to live location events.
  ([#6563](https://github.com/matrix-org/matrix-rust-sdk/pulls/6563))

## [0.17.0] - 2026-05-08

### Security fixes

- Reject invalid edits as candidates for timeline updates.
  ([#6454](https://github.com/matrix-org/matrix-rust-sdk/pull/6454), Moderate,
  [CVE-2026-45057](https://www.cve.org/CVERecord?id=CVE-2026-45057),
  [GHSA-h97m-27fx-42rx](https://github.com/matrix-org/matrix-rust-sdk/security/advisories/GHSA-h97m-27fx-42rx))

### Bug fixes

- Fix a possible panic in `RoomList::entries_with_dynamic_adapters`.
  ([#6459](https://github.com/matrix-org/matrix-rust-sdk/pull/6459))
- Keep stopped `beacon_info` live location sessions visible in
  `Room::latest_event()`, so room summaries still show the last live location
  sharing session after it ends.
  ([#6437](https://github.com/matrix-org/matrix-rust-sdk/pull/6437))
- Allow setting a custom Sliding Sync connection ID and timeline limit on
  `RoomListService`.
  ([#6289](https://github.com/matrix-org/matrix-rust-sdk/pull/6289))
- Don't show a "sent in clear" shield on live location timeline items in
  encrypted rooms, since `beacon_info` is a state event that cannot be
  encrypted by design.
  ([#6308](https://github.com/matrix-org/matrix-rust-sdk/pull/6308))
- Include secondary relations when re-initializing a threaded timeline after a
  lag. ([#6209](https://github.com/matrix-org/matrix-rust-sdk/pull/6209))
- Ensure that the display name of a `Room` in a `NotificationStatus` coming
  from a `NotificationClient` excludes service members.
  ([#6136](https://github.com/matrix-org/matrix-rust-sdk/pull/6136))
- Fix the `is_last_admin` check in `LeaveSpaceRoom` since it was not
  accounting for the membership state.
  [#6032](https://github.com/matrix-org/matrix-rust-sdk/pull/6032)
- [**breaking**] `LatestEventValue::Local { is_sending: bool }` is replaced
  by [`state: LatestEventValueLocalState`] to represent 3 states: `IsSending`,
  `HasBeenSent` and `CannotBeSent`.
  ([#5968](https://github.com/matrix-org/matrix-rust-sdk/pull/5968/))
- Fix the redecryption of events in timelines built using the
  `TimelineFocus` of `PinnedEvents`, `Thread`, `Event`.
  ([#5955](https://github.com/matrix-org/matrix-rust-sdk/pull/5955))

### Features

- `SpaceRoom` and `NotificationItem` now have an `is_dm` field to indicate
  whether the room is a DM room.
  ([#6537](https://github.com/matrix-org/matrix-rust-sdk/pull/6537))
- Add a list of `declined_by: Vec<OwnedUserId>` to the
  `TimelineItemContent::RtcNotification`, this will contain the list of users
  that have declined the call.
  ([#6494](https://github.com/matrix-org/matrix-rust-sdk/pull/6494))
- [**breaking**] Add the `suggested` field to the `SpaceRoom` struct, which
  indicates whether a space's admins have marked that sub-space/room as a
  "suggested" one to join.
  ([6417](https://github.com/matrix-org/matrix-rust-sdk/pull/6417))
- Handle local echoes of redactions in the timeline.
  ([#6250](https://github.com/matrix-org/matrix-rust-sdk/pull/6250))
- [**breaking**] Remove support for `native-tls` and remove all feature
  flags for selecting TLS backend, as `rustls` is the now the only supported
  TLS backend.
  ([#6409](https://github.com/matrix-org/matrix-rust-sdk/pull/6409))
- Add `call_intent` to `TimelineItemContent::RtcNotification`
  ([#6412](https://github.com/matrix-org/matrix-rust-sdk/pull/6412))
- Introduce a `ThreadListService` which offers reactive interfaces for rendering
  and managing the list of threads from a particular room.
  ([#6311](https://github.com/matrix-org/matrix-rust-sdk/pull/6311))
- [**breaking**] Remove the `Room::load_thread_list` in favor of the new
  `ThreadListService`
  ([#6311](https://github.com/matrix-org/matrix-rust-sdk/pull/6311))
- Add support for
  [MSC3489](https://github.com/matrix-org/matrix-spec-proposals/pull/3489) live
  location sharing through a new `TimelineItemContent::LiveLocation` variant.
- The internal timeline unique ID may be recycled when an event is deduplicated
  from the timeline, so that embedders can notice that it's the same item and
  avoid unnecessary re-rendering.
  ([#6228](https://github.com/matrix-org/matrix-rust-sdk/pull/6228))
- [**breaking**] Add `NotificationState.EventRedacted` enum value, to handle the
  case where a notification resolves to a redacted event.
  ([#6203](https://github.com/matrix-org/matrix-rust-sdk/pull/6203))
- [**breaking**] Extend `TimelineFocus::Event` to allow marking the target
  event as the root of a thread.
  ([#6050](https://github.com/matrix-org/matrix-rust-sdk/pull/6050))
- [**breaking**] Remove `TimelineEventTypeFilter` which has been replaced by
  the more generic `TimelineEventFilter`.
  ([#6070](https://github.com/matrix-org/matrix-rust-sdk/pull/6070/))
- Add `TimelineEventFilter` for filtering events based on their type or
  content. For content filtering, only membership and profile change filters
  are available as of now.
  ([#6048](https://github.com/matrix-org/matrix-rust-sdk/pull/6048/))
- Introduce `SpaceFilter`s as a mechanism for narrowing down what's displayed in
  the room list
  ([#6025](https://github.com/matrix-org/matrix-rust-sdk/pull/6025))
- Utilize the cache and include common relations when focusing a timeline on an
  event without requestion context.
  ([#5858](https://github.com/matrix-org/matrix-rust-sdk/pull/5858))
- [**breaking**] `EventTimelineItem::get_shield` now returns a new type,
  `TimelineEventShieldState`, which extends the old `ShieldState` with a code
  for `SentInClear`, now that the latter has been removed from `ShieldState`.
  ([#5959](https://github.com/matrix-org/matrix-rust-sdk/pull/5959))
- Add `SpaceService::get_space_room` to get a space
  given its id from the space graph if available.
  ([#5944](https://github.com/matrix-org/matrix-rust-sdk/pull/5944))
- [**breaking**]: The new Latest Event API replaces the old API. All the
  `new_` prefixes have been removed. The following methods are removed:
  `EventTimelineItem::from_latest_event`, and `Timeline::latest_event`. See the
  documentation of `matrix::latest_event` to learn about the new API.
  ([#5624](https://github.com/matrix-org/matrix-rust-sdk/pull/5624/))
- `Room::load_event_with_relations` now also calls `/relations` to fetch related
  events when falling back to network mode after a cache miss.
  ([#5930](https://github.com/matrix-org/matrix-rust-sdk/pull/5930))
- Expose `EventTimelineItem::forwarder` and `forwarder_profile`, which, if
  present, provide the ID and profile of the user who forwarded the keys used to
  decrypt the event as part of an
  [MSC4268](https://github.com/matrix-org/matrix-spec-proposals/pull/4268) key
  bundle. ([#6000](https://github.com/matrix-org/matrix-rust-sdk/pull/6000))

### Refactor

- Use `DmRoomDefinition` to check if a room should be considered part of the
  `RoomCategory::People` or `RoomCategory::Room` when using room list filters.
  ([#6490](https://github.com/matrix-org/matrix-rust-sdk/pull/6490))
- [**breaking**] `AnyOtherStateEventContentChange::RoomAliases` was removed.
  This state event type was removed from the Matrix specification a while ago,
  and support for it has been removed in Ruma.
  ([#6414](https://github.com/matrix-org/matrix-rust-sdk/pull/6414))
- [**breaking**] Move `LiveLocation` out of `TimelineItemContent` and into
  `MsgLikeKind` so it has access to `MsgLikeContent` `reactions`.
  ([#6286](https://github.com/matrix-org/matrix-rust-sdk/pull/6286))
- [**breaking**] Rename `AnyOtherFullStateEventContent` to
  `AnyOtherStateEventContentChange` to match the name change in the upstream
  types. ([#6218](https://github.com/matrix-org/matrix-rust-sdk/pull/6218))
- [**breaking**] Remove `WithLocking` from `EncryptionSyncService`, the locking
  mechanism will be taken from the parent `Client` with
  `Client::cross_process_store_config`.
  ([#6160](https://github.com/matrix-org/matrix-rust-sdk/pull/6160))
- [**breaking**] The [`Timeline::pin_event`] and [`Timeline::unpin_event`]
  methods have been moved to the SDK crate, in the `Room` object. Users can
  replace previous uses with `timeline.room().pin_event()` etc.
  ([#6106](https://github.com/matrix-org/matrix-rust-sdk/pull/6106))
- [**breaking**] Refactored `is_last_admin` to `is_last_owner` the check will
  now account also for v12 rooms, where creators and users with PL 150 matter.
  ([#6036](https://github.com/matrix-org/matrix-rust-sdk/pull/6036))
- [**breaking**] The `SpaceService` will no longer auto-subscribe to required
  client events when invoking the `subscribe_to_joined_spaces` but instead do it
  through its, now async, constructor.
  ([#5972](https://github.com/matrix-org/matrix-rust-sdk/pull/5972))
- [**breaking**] The `SpaceService`'s `joined_spaces` method has been renamed
  `top_level_joined_spaces` and `subscribe_to_joined_spaces` to
  `space_service.subscribe_to_top_level_joined_spaces`
  ([#5972](https://github.com/matrix-org/matrix-rust-sdk/pull/5972))
- `RoomListService::subscribe_to_rooms` now forgets previous subscriptions.
  ([#6012](https://github.com/matrix-org/matrix-rust-sdk/pull/6012))

## [0.16.1] - 2026-05-08

No notable changes in this release.

## [0.16.0] - 2025-12-04

### Features

- [**breaking**] `TimelineBuilder::track_read_marker_and_receipts` now takes a
  parameter to allow tracking to be enabled for all events (like before) or only
  for message-like events (which prevents read receipts from being placed on
  state events).
  ([#5900](https://github.com/matrix-org/matrix-rust-sdk/pull/5900))

## [0.15.0] - 2025-11-27

### Features

- Expose `is_space` in `NotificationItem`, allowing clients to determine if the
  room that triggered the notification is a space.
- [**breaking**] The `LatestEventValue::Local` type gains 2 new fields: `sender`
  and `profile`.
  ([#5885](https://github.com/matrix-org/matrix-rust-sdk/pull/5885))
- Add push actions to `NotificationItem`.
  ([#5835](https://github.com/matrix-org/matrix-rust-sdk/pull/5835))
- Add support for top level space ordering through
  [MSC3230](https://github.com/matrix-org/matrix-spec-proposals/pull/3230) and
  `m.space_order` room account data fields
  ([#5799](https://github.com/matrix-org/matrix-rust-sdk/pull/5799))

### Refactor

- `Timeline::latest_event` will return the latest event in the timeline, not the
  latest item of the timeline if it's an event.
- `TimelineFocusKind::Event` can now handle both the existing event pagination
  and thread pagination if the focused event is part of a thread
  ([#5678](https://github.com/matrix-org/matrix-rust-sdk/pull/5678)).
- [**breaking**] The `Room` type in `room_list_service` is renamed to
  `RoomListItem`.
  ([#5684](https://github.com/matrix-org/matrix-rust-sdk/pull/5684))

### Bug fixes

- `Timeline::latest_event_id` won't take threaded events into account on
  live/event focused timelines if `hide_threaded_events` is enabled. This fixes
  a bug in `Timeline::mark_as_read` that incorrectly tried to send a read
  receipt for threaded events that aren't really part of those timelines.
  ([#5864](https://github.com/matrix-org/matrix-rust-sdk/pull/5864/))
- Avoid replacing timeline items when the encryption info is unchanged.
  ([#5660](https://github.com/matrix-org/matrix-rust-sdk/pull/5660))
- Improvement performance of `RoomList` by introducing a new `RoomListItem` type
  (that replaces the `Room` type).
  ([#5684](https://github.com/matrix-org/matrix-rust-sdk/pull/5684))

## [0.14.0] - 2025-09-04

### Features

- Add a new [`SpaceService`] that provides high level reactive interfaces for
  listing the user's joined top level spaces as long as their children.
  ([#5509](https://github.com/matrix-org/matrix-rust-sdk/pull/5509))
- Add `new_filter_low_priority` and `new_filter_non_low_priority` filters to the
  room list filtering system, allowing clients to filter rooms based on their
  low priority status. The filters use the `Room::is_low_priority()` method
  which checks for the `m.lowpriority` room tag.
  ([#5508](https://github.com/matrix-org/matrix-rust-sdk/pull/5508))
- [**breaking**] Refactor the `non_space` filter into a `space` filter,
  favouring its use in combination with the `not` filter.
  ([#5508](https://github.com/matrix-org/matrix-rust-sdk/pull/5508))
- [**breaking**] Space rooms are now being retrieved through sliding sync and
  the newly introduced [`room_list_service::filters::new_filter_non_space`]
  filter should be used to exclude them from any room list.
  ([5479](https://github.com/matrix-org/matrix-rust-sdk/pull/5479))
- [**breaking**] [`Timeline::send_gallery()`] now automatically fills in the
  thread relationship, based on the timeline focus. As a result, the
  `GalleryConfig::reply()` builder method has been replaced with
  `GalleryConfig::in_reply_to`, and only takes an optional event id (the event
  that is effectively replied to) instead of the `Reply` type. The proper way to
  start a thread with a gallery event is now thus to create a threaded-focused
  timeline, and then use `Timeline::send_gallery()`.
  ([5427](https://github.com/matrix-org/matrix-rust-sdk/pull/5427))
- [**breaking**] [`Timeline::send_attachment()`] now automatically fills in the
  thread relationship, based on the timeline focus. As a result, there's a new
  `ui::timeline::AttachmentConfig` type in town, that has a
  simplified optional parameter `replied_to` of type `OwnedEventId` instead of
  the `Reply` type and that must be used in place of
  `matrix::attachment::AttachmentConfig`. The proper way to start a thread
  with a media attachment is now thus to create a threaded-focused timeline, and
  then use `Timeline::send_attachment()`.
  ([5427](https://github.com/matrix-org/matrix-rust-sdk/pull/5427))
- [**breaking**] [`Timeline::send_reply()`] now automatically fills in the
  thread relationship, based on the timeline focus. As a result, it only takes
  an `OwnedEventId` parameter, instead of the `Reply` type. The proper way to
  start a thread is now thus to create a threaded-focused timeline, and then use
  `Timeline::send()`.
  ([5427](https://github.com/matrix-org/matrix-rust-sdk/pull/5427))
- `Timeline::send()` will now automatically fill the thread relationship, if the
  timeline has a thread focus, and the sent event doesn't have a prefilled
  `relates_to` field (i.e. a relationship).
  ([5427](https://github.com/matrix-org/matrix-rust-sdk/pull/5427))

### Refactor

- [**breaking**] The MSRV has been bumped to Rust 1.88.
  ([#5431](https://github.com/matrix-org/matrix-rust-sdk/pull/5431))

### Bug fixes

- Correctly remove unable-to-decrypt items that have been decrypted but contain
  unsupported event types.
  ([#5463](https://github.com/matrix-org/matrix-rust-sdk/pull/5463))

## [0.13.0] - 2025-07-10

### Features

- Infer timeline read receipt threads for the `send_single_receipt` method from
  the focus mode and associated `hide_threaded_events` flag.
  ([5325](https://github.com/matrix-org/matrix-rust-sdk/pull/5325))
- Add `NotificationItem::room_topic` to the `NotificationItem` struct, which
  contains the topic of the room. This is useful for displaying the room topic
  in notifications.
  ([#5300](https://github.com/matrix-org/matrix-rust-sdk/pull/5300))
- Add `EmbeddedEvent::timestamp` and `EmbeddedEvent::identifier` which are
  already available in regular timeline items.
  ([#5331](https://github.com/matrix-org/matrix-rust-sdk/pull/5331))
- `RoomListService::subscribe_to_rooms` becomes `async` and automatically calls
  `matrix::latest_events::LatestEvents::listen_to_room`
  ([#5369](https://github.com/matrix-org/matrix-rust-sdk/pull/5369))

### Refactor

- [**breaking**] The function provided to `TimelineBuilder::event_filter()`
  must take `RoomVersionRules` as second argument instead of a `RoomVersionId`.
  The `default_event_filter()` reflects that change.
  ([#5337](https://github.com/matrix-org/matrix-rust-sdk/pull/5337))

## [0.12.0] - 2025-06-10

### Refactor

- [**breaking**] [`TimelineItemContent::reactions()`] returns an
  `Option<&ReactionsByKeyBySender>` instead of `ReactionsByKeyBySender`. This
  reflects the fact that some timeline items cannot hold reactions at all.
- `NotificationItem::room_join_rule` is now optional to reflect that the join
  rule state event might be missing, in which case it will be set to `None`. The
  `NotificationItem::is_public` field has been replaced with a method that
  returns an `Option<bool>`, based on the same logic.
  ([#5278](https://github.com/matrix-org/matrix-rust-sdk/pull/5278))

### Bug fixes

- Introduce `Timeline` regions, which helps to remove a class of bugs in the
  `Timeline` where items could be inserted in the wrong _regions_, such as
  a remote timeline item before the `TimelineStart` virtual timeline item.
  ([#5000](https://github.com/matrix-org/matrix-rust-sdk/pull/5000))
- `NotificationClient` will filter out events sent by ignored users on
  `get_notification` and `get_notifications`.
  ([#5081](https://github.com/matrix-org/matrix-rust-sdk/pull/5081))

### Features

- `Timeline::send_single_receipt()` and `Timeline::send_multiple_receipts()` now
  also unset the unread flag of the room if an unthreaded read receipt is sent.
  ([#5055](https://github.com/matrix-org/matrix-rust-sdk/pull/5055))
- `Timeline::mark_as_read()` unsets the unread flag of the room if it was set.
  ([#5055](https://github.com/matrix-org/matrix-rust-sdk/pull/5055))
- Add new method `Timeline::send_gallery` to allow sending MSC4274-style
  galleries.
  ([#5125](https://github.com/matrix-org/matrix-rust-sdk/pull/5125))

## [0.11.0] - 2025-04-11

### Bug fixes

### Features

- [**breaking**] Optionally allow starting threads with `Timeline::send_reply`.
  ([#4819](https://github.com/matrix-org/matrix-rust-sdk/pull/4819))
- [**breaking**] Push `RepliedToInfo`, `ReplyContent`, `EnforceThread` and
  `UnsupportedReplyItem` (becoming `ReplyError`) down into matrix.
  [`Timeline::send_reply()`] now takes an event ID rather than a
  `RepliedToInfo`. `Timeline::replied_to_info_from_event_id` has been made
  private in `matrix`.
  ([#4842](https://github.com/matrix-org/matrix-rust-sdk/pull/4842))
- Allow sending media as (thread) replies. The reply behaviour can be configured
  through new fields on [`AttachmentConfig`].
  ([#4852](https://github.com/matrix-org/matrix-rust-sdk/pull/4852))

### Refactor

- [**breaking**] Reactions on a given timeline item have been moved from
  [`EventTimelineItem::reactions()`] to [`TimelineItemContent::reactions()`];
  they're thus available from an [`EventTimelineItem`] by calling
  `.content().reactions()`. They're also returned by ownership (cloned) instead
  of by reference.
  ([#4576](https://github.com/matrix-org/matrix-rust-sdk/pull/4576))
- [**breaking**] The parameters `event_id` and `enforce_thread` on
  [`Timeline::send_reply()`] have been wrapped in a `reply` struct parameter.
  ([#4880](https://github.com/matrix-org/matrix-rust-sdk/pull/4880/))

## [0.10.0] - 2025-02-04

### Bug fixes

- Don't consider rooms in the banned state to be non-left rooms. This bug was
  introduced due to the introduction of the banned state for rooms, and the
  non-left room filter did not take the new room state into account.
  ([#4448](https://github.com/matrix-org/matrix-rust-sdk/pull/4448))

- Fix `EventTimelineItem::latest_edit_json()` when it is populated by a live
  edit. ([#4552](https://github.com/matrix-org/matrix-rust-sdk/pull/4552))

- Fix our own explicit read receipt being ignored when loading it from the
  state store, which resulted in our own read receipt being wrong sometimes.
  ([#4600](https://github.com/matrix-org/matrix-rust-sdk/pull/4600))

### Features

- [**breaking**] `Timeline::send_attachment()` now takes a type that implements
  `Into<AttachmentSource>` instead of a type that implements `Into<PathBuf>`.
  `AttachmentSource` allows to send an attachment either from a file, or with
  the bytes and the filename of the attachment. Note that all types that
  implement `Into<PathBuf>` also implement `Into<AttachmentSource>`.
  ([#4451](https://github.com/matrix-org/matrix-rust-sdk/pull/4451))

- [**breaking**] Add an "offline" mode to the `SyncService`. This allows the
  `SyncService` to attempt to restart the sync automatically. It can be enabled
  with the `SyncServiceBuilder::with_offline_mode` method. Due to this addition,
  the `SyncService::stop` method has been made infallible.
  ([#4592](https://github.com/matrix-org/matrix-rust-sdk/pull/4592))

### Refactor

- Drastically improve the performance of the `Timeline` when it receives
  hundreds and hundreds of events (approximately 10 times faster).
  ([#4601](https://github.com/matrix-org/matrix-rust-sdk/pull/4601),
  [#4608](https://github.com/matrix-org/matrix-rust-sdk/pull/4608),
  [#4612](https://github.com/matrix-org/matrix-rust-sdk/pull/4612))

- [**breaking**] `Timeline::paginate_forwards` and
  `Timeline::paginate_backwards` are unified to work on a live or focused
  timeline. `Timeline::live_paginate_*` and `Timeline::focused_paginate_*` have
  been removed
  ([#4584](https://github.com/matrix-org/matrix-rust-sdk/pull/4584)).

- [**breaking**] `Timeline::subscribe_batched` replaces
  `Timeline::subscribe`. `subscribe` has been removed in
  [#4567](https://github.com/matrix-org/matrix-rust-sdk/pull/4567),
  and `subscribe_batched` has been renamed to `subscribe` in
  [#4585](https://github.com/matrix-org/matrix-rust-sdk/pull/4585).

## [0.9.0] - 2024-12-18

### Bug fixes

- Add the `m.room.create` and the `m.room.history_visibility` state events to
  the required state for the sync. These two state events are required to
  properly compute the room preview of a joined room.
  ([#4325](https://github.com/matrix-org/matrix-rust-sdk/pull/4325))

### Features

- Introduce a new variant to the `UtdCause` enum tailored for device-historical
  messages. These messages cannot be decrypted unless the client regains access
  to message history through key storage (e.g., room key backups).
  ([#4375](https://github.com/matrix-org/matrix-rust-sdk/pull/4375))

## [0.8.0] - 2024-11-19

### Bug fixes

- Disable `share_pos()` inside `RoomListService`.
- `UtdHookManager` no longer re-reports UTD events as late decryptions.
  ([#3480](https://github.com/matrix-org/matrix-rust-sdk/pull/3480))

- Messages that we were unable to decrypt no longer display a red padlock.
  ([#3956](https://github.com/matrix-org/matrix-rust-sdk/issues/3956))

- `UtdHookManager` no longer reports UTD events that were already reported in a
  previous session.
  ([#3519](https://github.com/matrix-org/matrix-rust-sdk/pull/3519))

### Features

- Add `m.room.join_rules` to the required state.
- `EncryptionSyncService` and `Notification` are using
  `Client::cross_process_store_locks_holder_name`.

### Refactor

- [**breaking**] `Timeline::edit` now takes a
  `RoomMessageEventContentWithoutRelation`.

- [**breaking**] `Timeline::send_attachment` now takes an `impl Into<PathBuf>`
  for the path of the file to send.

- [**breaking**] `Timeline::item_by_transaction_id` has been renamed to
  `Timeline::local_item_by_transaction_id` (always returns local echoes).

## 0.7.0

Initial release

---

## `common-test`


All notable changes to this project will be documented in this file.


## [0.18.0](https://github.com/matrix-org/matrix-rust-sdk/tree/0.18.0) - 2026-06-02

No significant changes.

## [0.17.0] - 2026-05-08

No notable changes in this release.

## [0.16.1] - 2026-05-08

No notable changes in this release.

## [0.16.0] - 2025-12-04

No notable changes in this release.

## [0.14.0] - 2025-09-04

No notable changes in this release.

## [0.13.0] - 2025-07-10

No notable changes in this release.

## [0.12.0] - 2025-06-10

No notable changes in this release.

## [0.11.0] - 2025-04-11

No notable changes in this release.

## [0.10.0] - 2025-02-04

No notable changes in this release.

