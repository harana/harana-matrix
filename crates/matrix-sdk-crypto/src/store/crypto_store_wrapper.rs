use std::{
    collections::{BTreeMap, BTreeSet},
    future,
    ops::Deref,
    sync::Arc,
    time::Duration,
};

use futures_core::Stream;
use futures_util::StreamExt;
use matrix_sdk_common::{
    cross_process_lock::{CrossProcessLock, CrossProcessLockConfig},
    locks::RwLock as StdRwLock,
};
use ruma::{
    DeviceId, OwnedDeviceId, OwnedRoomId, OwnedUserId, RoomId, SecondsSinceUnixEpoch, UserId,
};
use tokio::sync::{Mutex, broadcast};
use tokio_stream::wrappers::{BroadcastStream, errors::BroadcastStreamRecvError};
use tracing::{debug, trace, warn};

use super::{
    DeviceChanges, IdentityChanges, LockableCryptoStore, caches::SessionStore,
    types::RoomKeyBundleInfo,
};
use crate::{
    CryptoStoreError, OwnUserIdentityData, Session, UserIdentityData,
    olm::InboundGroupSession,
    store,
    store::{
        Changes, DynCryptoStore, IntoCryptoStore, RoomKeyInfo, RoomKeyWithheldInfo,
        types::{RoomSettings, SecretsInboxItem},
    },
};

/// How many Olm sessions we keep per device once the older ones have aged out.
///
/// The spec has us keep more than one so that a peer encrypting on a session we
/// did not pick is still understood.
const MAX_SESSIONS_PER_DEVICE: usize = 4;

/// How long an Olm session has to have gone unused before it can be dropped.
///
/// Dropping a session the peer is still encrypting to would create exactly the
/// undecryptable messages that keeping several sessions is meant to avoid, so
/// the cap only applies to sessions nobody has touched for this long.
const SESSION_EVICTION_GRACE_PERIOD: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// A wrapper for crypto store implementations that adds update notifiers.
///
/// This is shared between [`StoreInner`] and
/// [`crate::verification::VerificationStore`].
#[derive(Debug)]
pub(crate) struct CryptoStoreWrapper {
    user_id: OwnedUserId,
    device_id: OwnedDeviceId,

    store: Arc<DynCryptoStore>,

    /// A cache for the Olm Sessions.
    sessions: SessionStore,

    /// Serialises the check-and-store of an inbound group session.
    ///
    /// Deciding whether a received room key is better than the one we already
    /// have is a read, a comparison and a write. Two of those interleaving —
    /// `import_room_keys` running while a sync delivers the same session, say —
    /// means both read "nothing stored", both decide to write, and the worse
    /// key can end up as the stored one with the comparison bypassed
    /// altogether.
    inbound_group_session_merge_lock: Mutex<()>,

    /// Serialises writes that replace a whole stored `DeviceData` or
    /// `UserIdentityData`.
    ///
    /// Those writes are read-modify-write on an object with several
    /// independent fields: `Device::set_local_trust` only means to change the
    /// trust, and `UserIdentity::pin` only means to change the pinned master
    /// key, but each writes the object it was built from in full. Without a
    /// lock, a `/keys/query` landing in between makes one of the two writes
    /// disappear — a pin can revert freshly received cross-signing keys, and a
    /// trust change can revert `deleted`, `olm_wedging_index` or
    /// `withheld_code_sent`.
    identity_update_lock: Mutex<()>,

    /// A cache for the per-room encryption settings.
    ///
    /// Reading these goes through a store lookup and a deserialization, and a
    /// client asks for them on the critical path of showing a room's encryption
    /// state, which made the call take seconds rather than milliseconds. The
    /// settings only change when we write them, so they can simply be
    /// remembered. `None` is cached as well: "this room has no settings" is the
    /// answer that gets asked for over and over.
    room_settings: StdRwLock<BTreeMap<OwnedRoomId, Option<RoomSettings>>>,

    /// The sender side of a broadcast stream that is notified whenever we get
    /// an update to an inbound group session.
    room_keys_received_sender: broadcast::Sender<Vec<RoomKeyInfo>>,

    /// The sender side of a broadcast stream that is notified whenever we
    /// receive an `m.room_key.withheld` message.
    room_keys_withheld_received_sender: broadcast::Sender<Vec<RoomKeyWithheldInfo>>,

    /// The sender side of a broadcast channel which sends out secrets we
    /// received as a `m.secret.send` event.
    secrets_broadcaster: broadcast::Sender<SecretsInboxItem>,

    /// The sender side of a broadcast channel which sends out devices and user
    /// identities which got updated or newly created.
    identities_broadcaster:
        broadcast::Sender<(Option<OwnUserIdentityData>, IdentityChanges, DeviceChanges)>,

    /// The sender side of a broadcast channel which sends out information about
    /// historic room key bundles we have received.
    historic_room_key_bundles_broadcaster: broadcast::Sender<RoomKeyBundleInfo>,
}

impl CryptoStoreWrapper {
    pub(crate) fn new(user_id: &UserId, device_id: &DeviceId, store: impl IntoCryptoStore) -> Self {
        let room_keys_received_sender = broadcast::Sender::new(10);
        let room_keys_withheld_received_sender = broadcast::Sender::new(10);
        let secrets_broadcaster = broadcast::Sender::new(10);
        // The identities broadcaster is responsible for user identities as well as
        // devices, that's why we increase the capacity here.
        let identities_broadcaster = broadcast::Sender::new(20);
        let historic_room_key_bundles_broadcaster = broadcast::Sender::new(10);

        Self {
            user_id: user_id.to_owned(),
            device_id: device_id.to_owned(),
            store: store.into_crypto_store(),
            sessions: SessionStore::new(),
            inbound_group_session_merge_lock: Default::default(),
            identity_update_lock: Default::default(),
            room_settings: Default::default(),
            room_keys_received_sender,
            room_keys_withheld_received_sender,
            secrets_broadcaster,
            identities_broadcaster,
            historic_room_key_bundles_broadcaster,
        }
    }

    /// Get the encryption settings for the given room.
    ///
    /// The settings are cached in memory, so that the common case of asking
    /// for them repeatedly does not hit the store every time.
    pub(crate) async fn get_room_settings(
        &self,
        room_id: &RoomId,
    ) -> store::Result<Option<RoomSettings>> {
        if let Some(settings) = self.room_settings.read().get(room_id) {
            return Ok(settings.clone());
        }

        let settings = self.store.get_room_settings(room_id).await?;
        self.room_settings.write().insert(room_id.to_owned(), settings.clone());

        Ok(settings)
    }

    /// Save the set of changes to the store.
    ///
    /// Also responsible for sending updates to the broadcast streams such as
    /// `room_keys_received_sender` and `secrets_broadcaster`.
    ///
    /// # Arguments
    ///
    /// * `changes` - The set of changes that should be stored.
    pub async fn save_changes(&self, changes: Changes) -> store::Result<()> {
        let room_key_updates: Vec<_> =
            changes.inbound_group_sessions.iter().map(RoomKeyInfo::from).collect();

        let withheld_session_updates: Vec<_> = changes
            .withheld_session_info
            .iter()
            .flat_map(|(room_id, session_map)| {
                session_map.iter().map(|(session_id, withheld_event)| RoomKeyWithheldInfo {
                    room_id: room_id.to_owned(),
                    session_id: session_id.to_owned(),
                    withheld_event: withheld_event.clone(),
                })
            })
            .collect();

        // If our own identity verified status changes we need to do some checks on
        // other identities. So remember the verification status before
        // processing the changes
        let own_identity_was_verified_before_change = self
            .store
            .get_user_identity(self.user_id.as_ref())
            .await?
            .as_ref()
            .and_then(|i| i.own())
            .is_some_and(|own| own.is_verified());

        let secrets = changes.secrets.to_owned();
        let devices = changes.devices.to_owned();
        let identities = changes.identities.to_owned();
        let room_key_bundle_updates: Vec<_> =
            changes.received_room_key_bundles.iter().map(RoomKeyBundleInfo::from).collect();

        // The sender keys whose Olm sessions this write touches, so that the
        // per-device cap can be applied to them once the write went through.
        let mut touched_sender_keys: BTreeSet<String> = BTreeSet::new();

        if devices
            .changed
            .iter()
            .any(|d| d.user_id() == self.user_id && d.device_id() == self.device_id)
        {
            // If our own device key changes, we need to clear the
            // session cache because the sessions contain a copy of our
            // device key.
            self.sessions.clear().await;
        } else {
            // Otherwise add the sessions to the cache.
            for session in &changes.sessions {
                touched_sender_keys.insert(session.sender_key.to_base64());
                self.sessions.add(session.clone()).await;
            }
        }

        // Keep the room settings cache in step with what we are about to write. Doing
        // it before the write and again after would leave a window where a
        // concurrent read repopulates the cache with the old value, so clear
        // the affected entries first and fill them in from the changes once the
        // write went through.
        let room_settings_updates = changes.room_settings.clone();

        if !room_settings_updates.is_empty() {
            let mut cache = self.room_settings.write();

            for room_id in room_settings_updates.keys() {
                cache.remove(room_id);
            }
        }

        self.store.save_changes(changes).await?;

        for sender_key in touched_sender_keys {
            self.expire_old_sessions(&sender_key).await?;
        }

        if !room_settings_updates.is_empty() {
            let mut cache = self.room_settings.write();

            for (room_id, settings) in room_settings_updates {
                cache.insert(room_id, Some(settings));
            }
        }

        // If we updated our own public identity, log it for debugging purposes
        if tracing::level_enabled!(tracing::Level::DEBUG) {
            for updated_identity in
                identities.new.iter().chain(identities.changed.iter()).filter_map(|id| id.own())
            {
                let master_key = updated_identity.master_key().get_first_key();
                let user_signing_key = updated_identity.user_signing_key().get_first_key();
                let self_signing_key = updated_identity.self_signing_key().get_first_key();

                debug!(
                    ?master_key,
                    ?user_signing_key,
                    ?self_signing_key,
                    previously_verified = updated_identity.was_previously_verified(),
                    verified = updated_identity.is_verified(),
                    "Stored our own identity"
                );
            }
        }

        if !room_key_updates.is_empty() {
            // Ignore the result. It can only fail if there are no listeners.
            let _ = self.room_keys_received_sender.send(room_key_updates);
        }

        if !withheld_session_updates.is_empty() {
            let _ = self.room_keys_withheld_received_sender.send(withheld_session_updates);
        }

        for secret in secrets {
            let _ = self.secrets_broadcaster.send(secret);
        }

        for bundle_info in room_key_bundle_updates {
            let _ = self.historic_room_key_bundles_broadcaster.send(bundle_info);
        }

        if !devices.is_empty() || !identities.is_empty() {
            // Mapping the devices and user identities from the read-only variant to one's
            // that contain side-effects requires our own identity. This is
            // guaranteed to be up-to-date since we just persisted it.
            let maybe_own_identity =
                self.store.get_user_identity(&self.user_id).await?.and_then(|i| i.into_own());

            // If our identity was not verified before the change and is now, that means
            // this could impact the verification chain of other known
            // identities.
            if let Some(own_identity_after) = maybe_own_identity.as_ref() {
                // Only do this if our identity is passing from not verified to verified,
                // the previously_verified can only change in that case.
                let own_identity_is_verified = own_identity_after.is_verified();

                if !own_identity_was_verified_before_change && own_identity_is_verified {
                    debug!(
                        "Own identity is now verified, check all known identities for verification status changes"
                    );
                    // We need to review all the other identities to see if they are verified now
                    // and mark them as such
                    self.check_all_identities_and_update_was_previously_verified_flag_if_needed(
                        own_identity_after,
                    )
                    .await?;
                } else if own_identity_was_verified_before_change != own_identity_is_verified {
                    // Log that the verification state of the identity changed.
                    debug!(
                        own_identity_is_verified,
                        "The verification state of our own identity has changed",
                    );
                }
            }

            let _ = self.identities_broadcaster.send((maybe_own_identity, identities, devices));
        }

        Ok(())
    }

    async fn check_all_identities_and_update_was_previously_verified_flag_if_needed(
        &self,
        own_identity_after: &OwnUserIdentityData,
    ) -> Result<(), CryptoStoreError> {
        let tracked_users = self.store.load_tracked_users().await?;
        let mut updated_identities: Vec<UserIdentityData> = Default::default();
        for tracked_user in tracked_users {
            if let Some(other_identity) = self
                .store
                .get_user_identity(tracked_user.user_id.as_ref())
                .await?
                .as_ref()
                .and_then(|i| i.other())
                && !other_identity.was_previously_verified()
                && own_identity_after.is_identity_signed(other_identity)
            {
                trace!(?tracked_user.user_id, "Marking set verified_latch to true.");
                other_identity.mark_as_previously_verified();
                updated_identities.push(other_identity.clone().into());
            }
        }

        if !updated_identities.is_empty() {
            let identity_changes =
                IdentityChanges { changed: updated_identities, ..Default::default() };
            self.store
                .save_changes(Changes {
                    identities: identity_changes.clone(),
                    ..Default::default()
                })
                .await?;

            let _ = self.identities_broadcaster.send((
                Some(own_identity_after.clone()),
                identity_changes,
                DeviceChanges::default(),
            ));
        }

        Ok(())
    }

    /// Drop Olm sessions with the given device beyond the per-device cap.
    ///
    /// Olm sessions are never replaced, only added: every unwedging, every
    /// message from a device we had no session with, every one-time key claim
    /// that crossed with theirs leaves another session behind, and they were
    /// kept forever. The spec has us keep several — the peer may still be
    /// encrypting to one we did not pick — but not all of them.
    ///
    /// The sessions kept are the [`MAX_SESSIONS_PER_DEVICE`] most recently
    /// used, plus any session used within [`SESSION_EVICTION_GRACE_PERIOD`]
    /// however many that is. The grace period is what keeps this from causing
    /// the undecryptable messages it is meant to avoid: a session the peer is
    /// still sending on gets used, and a session nobody has touched in a month
    /// is one nobody is going to send on.
    async fn expire_old_sessions(&self, sender_key: &str) -> store::Result<()> {
        let Some(sessions) = self.get_sessions(sender_key).await? else {
            return Ok(());
        };

        let mut sessions = sessions.lock().await;

        if sessions.len() <= MAX_SESSIONS_PER_DEVICE {
            return Ok(());
        }

        // Most recently used last, so the tail is what we keep.
        sessions.sort_by_key(|s| (s.last_use_time, s.creation_time));

        let cutoff = SecondsSinceUnixEpoch::now()
            .to_system_time()
            .and_then(|now| now.checked_sub(SESSION_EVICTION_GRACE_PERIOD))
            .and_then(SecondsSinceUnixEpoch::from_system_time);

        let evict_up_to = sessions.len() - MAX_SESSIONS_PER_DEVICE;
        let evicted: Vec<String> = sessions[..evict_up_to]
            .iter()
            .filter(|session| cutoff.is_none_or(|cutoff| session.last_use_time < cutoff))
            .map(|session| session.session_id().to_owned())
            .collect();

        if evicted.is_empty() {
            return Ok(());
        }

        debug!(
            sender_key,
            evicted = ?evicted,
            remaining = sessions.len() - evicted.len(),
            "Expiring Olm sessions that have not been used in a long time",
        );

        sessions.retain(|session| !evicted.iter().any(|id| id == session.session_id()));
        drop(sessions);

        self.store.delete_sessions(sender_key, &evicted).await?;

        Ok(())
    }

    pub async fn get_sessions(
        &self,
        sender_key: &str,
    ) -> store::Result<Option<Arc<Mutex<Vec<Session>>>>> {
        let sessions = self.sessions.get(sender_key).await;

        let sessions = if sessions.is_none() {
            let mut entries = self.sessions.entries.write().await;

            let sessions = entries.get(sender_key);

            if sessions.is_some() {
                sessions.cloned()
            } else {
                let sessions = self.store.get_sessions(sender_key).await?;
                let sessions = Arc::new(Mutex::new(sessions.unwrap_or_default()));

                entries.insert(sender_key.to_owned(), sessions.clone());

                Some(sessions)
            }
        } else {
            sessions
        };

        Ok(sessions)
    }

    /// Save a list of inbound group sessions to the store.
    ///
    /// # Arguments
    ///
    /// * `sessions` - The sessions to be saved.
    /// * `backed_up_to_version` - If the keys should be marked as having been
    ///   backed up, the version of the backup.
    ///
    /// Note: some implementations ignore `backup_version` and assume the
    /// current backup version, which is normally the same.
    pub async fn save_inbound_group_sessions(
        &self,
        sessions: Vec<InboundGroupSession>,
        backed_up_to_version: Option<&str>,
    ) -> store::Result<()> {
        let room_key_updates: Vec<_> = sessions.iter().map(RoomKeyInfo::from).collect();
        self.store.save_inbound_group_sessions(sessions, backed_up_to_version).await?;

        if !room_key_updates.is_empty() {
            // Ignore the result. It can only fail if there are no listeners.
            let _ = self.room_keys_received_sender.send(room_key_updates);
        }
        Ok(())
    }

    /// Take the lock that serialises the check-and-store of inbound group
    /// sessions.
    ///
    /// Must be held from the moment a caller asks whether a received session is
    /// better than the stored one until it has written its answer, otherwise
    /// two callers can both find nothing stored and both write.
    pub(crate) async fn lock_inbound_group_session_merge(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.inbound_group_session_merge_lock.lock().await
    }

    /// Take the lock that serialises whole-object writes of devices and user
    /// identities.
    ///
    /// Must be held across the read, the change and the write.
    pub(crate) async fn lock_identity_update(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.identity_update_lock.lock().await
    }

    /// Receive notifications of room keys being received as a [`Stream`].
    ///
    /// Each time a room key is updated in any way, an update will be sent to
    /// the stream. Updates that happen at the same time are batched into a
    /// [`Vec`].
    ///
    /// If the reader of the stream lags too far behind an error will be sent to
    /// the reader.
    pub fn room_keys_received_stream(
        &self,
    ) -> impl Stream<Item = Result<Vec<RoomKeyInfo>, BroadcastStreamRecvError>> + use<> {
        BroadcastStream::new(self.room_keys_received_sender.subscribe())
    }

    /// Receive notifications of received `m.room_key.withheld` messages.
    ///
    /// Each time an `m.room_key.withheld` is received and stored, an update
    /// will be sent to the stream. Updates that happen at the same time are
    /// batched into a [`Vec`].
    ///
    /// If the reader of the stream lags too far behind, a warning will be
    /// logged and items will be dropped.
    pub fn room_keys_withheld_received_stream(
        &self,
    ) -> impl Stream<Item = Vec<RoomKeyWithheldInfo>> + use<> {
        let stream = BroadcastStream::new(self.room_keys_withheld_received_sender.subscribe());
        Self::filter_errors_out_of_stream(stream, "room_keys_withheld_received_stream")
    }

    /// Receive notifications of gossipped secrets being received and stored in
    /// the secret inbox as a [`Stream`].
    pub fn secrets_stream(&self) -> impl Stream<Item = SecretsInboxItem> + use<> {
        let stream = BroadcastStream::new(self.secrets_broadcaster.subscribe());
        Self::filter_errors_out_of_stream(stream, "secrets_stream")
    }

    /// Receive notifications of historic room key bundles being received and
    /// stored in the store as a [`Stream`].
    pub fn historic_room_key_stream(&self) -> impl Stream<Item = RoomKeyBundleInfo> + use<> {
        let stream = BroadcastStream::new(self.historic_room_key_bundles_broadcaster.subscribe());
        Self::filter_errors_out_of_stream(stream, "bundle_stream")
    }

    /// Returns a stream of newly created or updated cryptographic identities.
    ///
    /// This is just a helper method which allows us to build higher level
    /// device and user identity streams.
    pub(super) fn identities_stream(
        &self,
    ) -> impl Stream<Item = (Option<OwnUserIdentityData>, IdentityChanges, DeviceChanges)> + use<>
    {
        let stream = BroadcastStream::new(self.identities_broadcaster.subscribe());
        Self::filter_errors_out_of_stream(stream, "identities_stream")
    }

    /// Helper for *_stream functions: filters errors out of the stream,
    /// creating a new Stream.
    ///
    /// `BroadcastStream`s gives us `Result`s which can fail with
    /// `BroadcastStreamRecvError` if the reader falls behind. That's annoying
    /// to work with, so here we just emit a warning and drop the errors.
    fn filter_errors_out_of_stream<ItemType>(
        stream: BroadcastStream<ItemType>,
        stream_name: &str,
    ) -> impl Stream<Item = ItemType> + use<ItemType>
    where
        ItemType: 'static + Clone + Send,
    {
        let stream_name = stream_name.to_owned();
        stream.filter_map(move |result| {
            future::ready(match result {
                Ok(r) => Some(r),
                Err(BroadcastStreamRecvError::Lagged(lag)) => {
                    warn!("{stream_name} missed {lag} updates");
                    None
                }
            })
        })
    }

    /// Creates a [`CrossProcessLock`] for this store, that will contain the
    /// given key and value when hold.
    pub(crate) fn create_store_lock(
        &self,
        lock_key: String,
        config: CrossProcessLockConfig,
    ) -> CrossProcessLock<LockableCryptoStore> {
        CrossProcessLock::new(LockableCryptoStore(self.store.clone()), lock_key, config)
    }
}

impl Deref for CryptoStoreWrapper {
    type Target = DynCryptoStore;

    fn deref(&self) -> &Self::Target {
        self.store.deref()
    }
}

#[cfg(test)]
mod test {
    use matrix_sdk_test::async_test;
    use ruma::user_id;

    use super::*;
    use crate::machine::test_helpers::get_machine_pair_with_setup_sessions_test_helper;

    #[async_test]
    async fn test_cache_cleared_after_device_update() {
        let user_id = user_id!("@alice:example.com");
        let (first, second) =
            get_machine_pair_with_setup_sessions_test_helper(user_id, user_id, false).await;

        let sender_key = second.identity_keys().curve25519.to_base64();

        first
            .store()
            .inner
            .store
            .sessions
            .get(&sender_key)
            .await
            .expect("We should have a session in the cache.");

        let device_data = first
            .get_device(user_id, first.device_id(), None)
            .await
            .unwrap()
            .expect("We should have access to our own device.")
            .inner;

        // When we save a new version of our device keys
        first
            .store()
            .save_changes(Changes {
                devices: DeviceChanges { changed: vec![device_data], ..Default::default() },
                ..Default::default()
            })
            .await
            .unwrap();

        // Then the session is no longer in the cache
        assert!(
            first.store().inner.store.sessions.get(&sender_key).await.is_none(),
            "The session should no longer be in the cache after our own device keys changed"
        );
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, sync::Arc};

    use matrix_sdk_test::async_test;
    use ruma::{device_id, room_id, user_id};

    use super::CryptoStoreWrapper;
    use crate::store::{
        CryptoStore, MemoryStore,
        types::{Changes, RoomSettings},
    };

    /// Reading a room's encryption settings went to the store and
    /// deserialized on every call, on the critical path of showing that
    /// room's encryption state. They only ever change when we write them, so
    /// they are remembered instead (#143).
    #[async_test]
    async fn test_room_settings_are_cached_and_updated_on_write() {
        let user_id = user_id!("@alice:localhost");
        let device_id = device_id!("ALICEDEVICE");
        let room_id = room_id!("!test:localhost");

        let memory_store = Arc::new(MemoryStore::new());
        let wrapper = CryptoStoreWrapper::new(user_id, device_id, memory_store.clone());

        // Nothing is stored to begin with. "No settings for this room" is the answer
        // that gets asked for over and over, so it is remembered as well.
        assert!(wrapper.get_room_settings(room_id).await.unwrap().is_none());

        // A write that goes around the wrapper isn't picked up, which is what tells us
        // the answer came from the cache rather than from the store.
        let settings = RoomSettings { only_allow_trusted_devices: true, ..Default::default() };
        let changes = || Changes {
            room_settings: HashMap::from([(room_id.to_owned(), settings.clone())]),
            ..Default::default()
        };
        CryptoStore::save_changes(&*memory_store, changes()).await.unwrap();

        assert!(wrapper.get_room_settings(room_id).await.unwrap().is_none());

        // A write that goes through the wrapper is: the cache is kept in step with
        // what we write.
        wrapper.save_changes(changes()).await.unwrap();

        assert_eq!(wrapper.get_room_settings(room_id).await.unwrap().as_ref(), Some(&settings));
    }
}
