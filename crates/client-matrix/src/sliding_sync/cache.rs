//! Cache utilities.
//!
//! A `SlidingSync` instance can be stored in a cache, and restored from the
//! same cache. It helps to define what it sometimes called a “cold start”, or a
//!  “fast start”.

#[cfg(feature = "e2e-encryption")]
use client_base::to_device_token;
use client_base::{StateStore, StoreError, event_cache::store::EventCacheStoreLockGuard};
use client_common::{cross_process_lock::MappedCrossProcessLockState, timer};
use harana_matrix_common::UserId;
use tracing::{trace, warn};

use super::{FrozenSlidingSyncList, SlidingSync, SlidingSyncPositionMarkers};
#[cfg(doc)]
use crate::sliding_sync::SlidingSyncList;
use crate::{Client, Result, sliding_sync::SlidingSyncListCachePolicy};

/// Be careful: as this is used as a storage key; changing it requires migrating
/// data!
pub(super) fn format_storage_key_prefix(id: &str, user_id: &UserId) -> String {
    format!("sliding_sync_store::{id}::{user_id}")
}

/// Be careful: as this is used as a storage key; changing it requires migrating
/// data!
fn format_storage_key_for_sliding_sync(storage_key: &str) -> String {
    format!("{storage_key}::instance")
}

/// Take the event cache store, whether or not another process wrote to it since
/// we last held the lock.
///
/// The `pos` is read back from the store rather than from memory, so a dirty
/// lock carries no extra work here.
async fn event_cache_store(client: &Client) -> Result<EventCacheStoreLockGuard> {
    Ok(match client.event_cache_store().lock().await? {
        MappedCrossProcessLockState::Clean(guard) | MappedCrossProcessLockState::Dirty(guard) => {
            guard
        }
    })
}

/// Be careful: as this is used as a storage key; changing it requires migrating
/// data!
fn format_storage_key_for_sliding_sync_list(storage_key: &str, list_name: &str) -> String {
    format!("{storage_key}::list::{list_name}")
}

/// Remove a previous [`SlidingSyncList`] cache entry from the state store.
async fn remove_cached_list(
    storage: &dyn StateStore<Error = StoreError>,
    storage_key: &str,
    list_name: &str,
) {
    let storage_key_for_list = format_storage_key_for_sliding_sync_list(storage_key, list_name);
    let _ = storage.remove_custom_value(storage_key_for_list.as_bytes()).await;
}

/// Store the `SlidingSync`'s state in the storage.
pub(super) async fn store_sliding_sync_state(
    sliding_sync: &SlidingSync,
    position: &SlidingSyncPositionMarkers,
) -> Result<()> {
    let storage_key = &sliding_sync.inner.storage_key;

    trace!(storage_key, "Saving a `SlidingSync` to the state store");
    let storage = sliding_sync.inner.client.state_store();

    // The `pos` is saved in the event cache store, which is shared between the
    // processes that open it and is guarded by a cross-process lock, so that a
    // second process resuming this sliding sync picks up where the first one
    // left off.
    {
        let instance_storage_key = format_storage_key_for_sliding_sync(storage_key);
        let pos_blob = serde_json::to_vec(&FrozenSlidingSyncPos { pos: position.pos.clone() })?;

        event_cache_store(&sliding_sync.inner.client)
            .await?
            .set_custom_value(instance_storage_key.as_bytes(), pos_blob)
            .await?;
    }

    // Write every `SlidingSyncList` that's configured for caching into the store.
    let frozen_lists = {
        sliding_sync
            .inner
            .lists
            .read()
            .await
            .iter()
            .filter(|(_, list)| matches!(list.cache_policy(), SlidingSyncListCachePolicy::Enabled))
            .map(|(list_name, list)| {
                Ok((
                    format_storage_key_for_sliding_sync_list(storage_key, list_name),
                    serde_json::to_vec(&FrozenSlidingSyncList::freeze(list))?,
                ))
            })
            .collect::<Result<Vec<_>, crate::Error>>()?
    };

    for (storage_key_for_list, frozen_list) in frozen_lists {
        trace!(storage_key_for_list, "Saving a `SlidingSyncList`");

        storage.set_custom_value(storage_key_for_list.as_bytes(), frozen_list).await?;
    }

    Ok(())
}

/// Try to restore a single [`SlidingSyncList`] from the cache.
///
/// If it fails to deserialize for some reason, invalidate the cache entry.
pub(super) async fn restore_sliding_sync_list(
    storage: &dyn StateStore<Error = StoreError>,
    storage_key: &str,
    list_name: &str,
) -> Result<Option<FrozenSlidingSyncList>> {
    let _timer = timer!(format!("loading list from DB {list_name}"));

    let storage_key_for_list = format_storage_key_for_sliding_sync_list(storage_key, list_name);

    match storage
        .get_custom_value(storage_key_for_list.as_bytes())
        .await?
        .map(|custom_value| serde_json::from_slice::<FrozenSlidingSyncList>(&custom_value))
    {
        Some(Ok(frozen_list)) => {
            // List has been found and successfully deserialized.
            trace!(list_name, "successfully read the list from cache");
            return Ok(Some(frozen_list));
        }

        Some(Err(_)) => {
            // List has been found, but it wasn't possible to deserialize it. It's declared
            // as obsolete. The main reason might be that the internal representation of a
            // `SlidingSyncList` might have changed. Instead of considering this as a strong
            // error, we remove the entry from the cache and keep the list in its initial
            // state.
            warn!(
                list_name,
                "failed to deserialize the list from the cache, it is obsolete; removing the cache entry!"
            );
            // Let's clear the list and stop here.
            remove_cached_list(storage, storage_key, list_name).await;
        }

        None => {
            // A missing cache doesn't make anything obsolete.
            // We just do nothing here.
            trace!(list_name, "failed to find the list in the cache");
        }
    }

    Ok(None)
}

/// Fields restored during [`restore_sliding_sync_state`].
#[derive(Default)]
pub(super) struct RestoredFields {
    pub to_device_token: Option<String>,
    pub pos: Option<String>,
}

/// A sliding sync position marker that can be persisted or restored from a
/// store.
#[derive(serde::Serialize, serde::Deserialize)]
struct FrozenSlidingSyncPos {
    #[serde(skip_serializing_if = "Option::is_none")]
    pos: Option<String>,
}

/// Restore the `SlidingSync`'s state from what is stored in the storage.
///
/// If one cache is obsolete (corrupted, and cannot be deserialized or
/// anything), the entire `SlidingSync` cache is removed.
pub(super) async fn restore_sliding_sync_state(
    client: &Client,
    storage_key: &str,
) -> Result<Option<RestoredFields>> {
    let _timer = timer!(format!("loading sliding sync {storage_key} state from DB"));

    let mut restored_fields = RestoredFields::default();
    let instance_storage_key = format_storage_key_for_sliding_sync(storage_key);

    #[cfg(feature = "e2e-encryption")]
    if let Some(olm_machine) = &*client.olm_machine().await {
        match olm_machine.store().next_batch_token().await? {
            // Only resume from a token that a sliding sync response produced. An
            // untagged value is a sync v2 `next_batch` (or a token stored before
            // the tagging existed), and sending it as the to-device `since` makes
            // the server reject every sync.
            Some(stored_value) => match to_device_token::untag(&stored_value) {
                Some(token) => restored_fields.to_device_token = Some(token.to_owned()),
                None => trace!(
                    "Ignoring the to-device token from the crypto store: it doesn't come \
                     from a sliding sync response"
                ),
            },
            None => trace!("Couldn't read the previous to-device token from the crypto store"),
        }
    }

    let store = event_cache_store(client).await?;

    if let Ok(Some(blob)) = store.get_custom_value(instance_storage_key.as_bytes()).await
        && let Ok(frozen_pos) = serde_json::from_slice::<FrozenSlidingSyncPos>(&blob)
    {
        trace!("Successfully read the `Sliding Sync` pos from the event cache store");
        restored_fields.pos = frozen_pos.pos;

        return Ok(Some(restored_fields));
    }

    // Older versions kept the `pos` in the crypto store, because it was the only
    // store that was cross-process safe at the time. Move it over, so a client
    // that upgrades resumes from where it was rather than starting a new sliding
    // sync.
    #[cfg(feature = "e2e-encryption")]
    if let Some(olm_machine) = &*client.olm_machine().await
        && let Ok(Some(blob)) = olm_machine.store().get_custom_value(&instance_storage_key).await
    {
        if let Ok(frozen_pos) = serde_json::from_slice::<FrozenSlidingSyncPos>(&blob) {
            trace!("Migrating the `Sliding Sync` pos out of the crypto store");
            restored_fields.pos = frozen_pos.pos;

            store.set_custom_value(instance_storage_key.as_bytes(), blob).await?;
        }

        olm_machine.store().remove_custom_value(&instance_storage_key).await?;
    }

    Ok(Some(restored_fields))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, RwLock};

    use common_test::async_test;

    #[cfg(feature = "e2e-encryption")]
    use super::restore_sliding_sync_state;
    use super::{
        super::SlidingSyncList, FrozenSlidingSyncPos, event_cache_store,
        format_storage_key_for_sliding_sync, format_storage_key_for_sliding_sync_list,
        format_storage_key_prefix, store_sliding_sync_state,
    };
    use crate::{Result, test_utils::logged_in_client};

    #[allow(clippy::await_holding_lock)]
    #[async_test]
    async fn test_sliding_sync_can_be_stored_and_restored() -> Result<()> {
        let client = logged_in_client(Some("https://foo.bar".to_owned())).await;

        let store = client.state_store();

        let sync_id = "test-sync-id";
        let storage_key = format_storage_key_prefix(sync_id, client.user_id().unwrap());

        // Store entries don't exist.
        assert!(
            store
                .get_custom_value(
                    format_storage_key_for_sliding_sync_list(&storage_key, "list_foo").as_bytes()
                )
                .await?
                .is_none()
        );

        assert!(
            store
                .get_custom_value(
                    format_storage_key_for_sliding_sync_list(&storage_key, "list_bar").as_bytes()
                )
                .await?
                .is_none()
        );

        // Create a new `SlidingSync` instance, and store it.
        let storage_key = {
            let sliding_sync = client
                .sliding_sync(sync_id)?
                .add_cached_list(SlidingSyncList::builder("list_foo"))
                .await?
                .add_list(SlidingSyncList::builder("list_bar"))
                .build()
                .await?;

            // Modify both lists, so we can check expected caching behavior later.
            {
                let lists = sliding_sync.inner.lists.write().await;

                let list_foo = lists.get("list_foo").unwrap();
                list_foo.set_maximum_number_of_rooms(Some(42));

                let list_bar = lists.get("list_bar").unwrap();
                list_bar.set_maximum_number_of_rooms(Some(1337));
            }

            let position_guard = sliding_sync.inner.position.lock().await;
            assert!(sliding_sync.cache_to_storage(&position_guard).await.is_ok());

            storage_key
        };

        // Store entries now exist for `list_foo`.
        assert!(
            store
                .get_custom_value(
                    format_storage_key_for_sliding_sync_list(&storage_key, "list_foo").as_bytes()
                )
                .await?
                .is_some()
        );

        // But not for `list_bar`.
        assert!(
            store
                .get_custom_value(
                    format_storage_key_for_sliding_sync_list(&storage_key, "list_bar").as_bytes()
                )
                .await?
                .is_none()
        );

        // Create a new `SlidingSync`, and it should be read from the cache.
        let max_number_of_room_stream = Arc::new(RwLock::new(None));
        let cloned_stream = max_number_of_room_stream.clone();
        let sliding_sync = client
            .sliding_sync(sync_id)?
            .add_cached_list(SlidingSyncList::builder("list_foo").once_built(move |list| {
                // In the `once_built()` handler, nothing has been read from the cache yet.
                assert_eq!(list.maximum_number_of_rooms(), None);

                let mut stream = cloned_stream.write().unwrap();
                *stream = Some(list.maximum_number_of_rooms_stream());
                list
            }))
            .await?
            .add_list(SlidingSyncList::builder("list_bar"))
            .build()
            .await?;

        // Check the list' state.
        {
            let lists = sliding_sync.inner.lists.read().await;

            // This one was cached.
            let list_foo = lists.get("list_foo").unwrap();
            assert_eq!(list_foo.maximum_number_of_rooms(), Some(42));

            // This one wasn't.
            let list_bar = lists.get("list_bar").unwrap();
            assert_eq!(list_bar.maximum_number_of_rooms(), None);
        }

        // The maximum number of rooms reloaded from the cache should have been
        // published.
        {
            let mut stream =
                max_number_of_room_stream.write().unwrap().take().expect("stream must be set");
            let initial_max_number_of_rooms =
                stream.next().await.expect("stream must have emitted something");
            assert_eq!(initial_max_number_of_rooms, Some(42));
        }

        Ok(())
    }

    #[cfg(feature = "e2e-encryption")]
    #[async_test]
    async fn test_sliding_sync_high_level_cache_and_restore() -> Result<()> {
        let client = logged_in_client(Some("https://foo.bar".to_owned())).await;

        let sync_id = "test-sync-id";
        let storage_key_prefix = format_storage_key_prefix(sync_id, client.user_id().unwrap());
        let full_storage_key = format_storage_key_for_sliding_sync(&storage_key_prefix);
        let sliding_sync = client.sliding_sync(sync_id)?.build().await?;

        // At first, there's nothing in both stores.
        if let Some(olm_machine) = &*client.base_client().olm_machine().await {
            let store = olm_machine.store();
            assert!(store.next_batch_token().await?.is_none());
        }

        let state_store = client.state_store();
        assert!(state_store.get_custom_value(full_storage_key.as_bytes()).await?.is_none());

        // Emulate some data to be cached.
        let pos = "pos".to_owned();
        {
            let mut position_guard = sliding_sync.inner.position.lock().await;
            position_guard.pos = Some(pos.clone());

            // Then, we can correctly cache the sliding sync instance.
            store_sliding_sync_state(&sliding_sync, &position_guard).await?;
        }

        // Ok, forget about the sliding sync, let's recreate one from scratch.
        drop(sliding_sync);

        let restored_fields = restore_sliding_sync_state(&client, &storage_key_prefix)
            .await?
            .expect("must have restored sliding sync fields");

        // After restoring, to-device token could be read.
        assert_eq!(restored_fields.pos.unwrap(), pos);

        // Test the "migration" path: assume a missing to-device token in crypto store,
        // but present in a former state store.

        // For our sanity, check no to-device token has been saved in the database.
        {
            let olm_machine = client.base_client().olm_machine().await;
            let olm_machine = olm_machine.as_ref().unwrap();
            assert!(olm_machine.store().next_batch_token().await?.is_none());
        }

        Ok(())
    }

    #[async_test]
    async fn test_the_pos_is_stored_in_the_event_cache_store() -> Result<()> {
        let client = logged_in_client(Some("https://foo.bar".to_owned())).await;

        let sync_id = "test-sync-id";
        let storage_key_prefix = format_storage_key_prefix(sync_id, client.user_id().unwrap());
        let full_storage_key = format_storage_key_for_sliding_sync(&storage_key_prefix);
        let sliding_sync = client.sliding_sync(sync_id)?.build().await?;

        {
            let mut position_guard = sliding_sync.inner.position.lock().await;
            position_guard.pos = Some("pos".to_owned());

            store_sliding_sync_state(&sliding_sync, &position_guard).await?;
        }

        // The `pos` is in the event cache store, which every client has, and not in
        // the crypto store, which only a client built with encryption has.
        let blob = event_cache_store(&client)
            .await?
            .get_custom_value(full_storage_key.as_bytes())
            .await?
            .expect("the pos must be in the event cache store");
        let frozen: FrozenSlidingSyncPos = serde_json::from_slice(&blob)?;
        assert_eq!(frozen.pos.as_deref(), Some("pos"));

        #[cfg(feature = "e2e-encryption")]
        {
            let olm_machine = client.base_client().olm_machine().await;
            let olm_machine = olm_machine.as_ref().unwrap();
            assert!(olm_machine.store().get_custom_value(&full_storage_key).await?.is_none());
        }

        Ok(())
    }

    #[cfg(feature = "e2e-encryption")]
    #[async_test]
    async fn test_a_pos_from_the_crypto_store_is_migrated() -> Result<()> {
        let client = logged_in_client(Some("https://foo.bar".to_owned())).await;

        let sync_id = "test-sync-id";
        let storage_key_prefix = format_storage_key_prefix(sync_id, client.user_id().unwrap());
        let full_storage_key = format_storage_key_for_sliding_sync(&storage_key_prefix);

        // A client that ran an older version of the SDK left its `pos` in the crypto
        // store.
        {
            let olm_machine = client.base_client().olm_machine().await;
            let olm_machine = olm_machine.as_ref().unwrap();
            let blob = serde_json::to_vec(&FrozenSlidingSyncPos { pos: Some("older".to_owned()) })?;
            olm_machine.store().set_custom_value(&full_storage_key, blob).await?;
        }

        let restored_fields = restore_sliding_sync_state(&client, &storage_key_prefix)
            .await?
            .expect("must have restored sliding sync fields");

        // It resumes from that position, rather than starting a new sliding sync.
        assert_eq!(restored_fields.pos.as_deref(), Some("older"));

        // And the value moved to the event cache store, with nothing left behind.
        let blob = event_cache_store(&client)
            .await?
            .get_custom_value(full_storage_key.as_bytes())
            .await?
            .expect("the pos must have been migrated");
        let frozen: FrozenSlidingSyncPos = serde_json::from_slice(&blob)?;
        assert_eq!(frozen.pos.as_deref(), Some("older"));

        {
            let olm_machine = client.base_client().olm_machine().await;
            let olm_machine = olm_machine.as_ref().unwrap();
            assert!(olm_machine.store().get_custom_value(&full_storage_key).await?.is_none());
        }

        // Reading it again takes the event cache store path.
        let restored_fields = restore_sliding_sync_state(&client, &storage_key_prefix)
            .await?
            .expect("must have restored sliding sync fields");
        assert_eq!(restored_fields.pos.as_deref(), Some("older"));

        Ok(())
    }
}
