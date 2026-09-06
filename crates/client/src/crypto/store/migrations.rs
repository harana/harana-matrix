// Copyright 2026 The Matrix.org Foundation C.I.C.
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

//! Migrations of the *contents* of the crypto store.
//!
//! Each [`CryptoStore`](super::CryptoStore) backend carries its own schema
//! migrations: SQLite has a numbered list of `.sql` files, IndexedDB a numbered
//! list of object-store rewrites. Those are about how a backend lays its data
//! out, and belong there.
//!
//! A change to what is *in* the store is a different thing. Recomputing a field
//! on every stored session, or marking every tracked user as needing a fresh
//! key query, has nothing to do with SQLite or IndexedDB, and writing it once
//! per backend means writing it twice, keeping the two in step forever, and
//! getting no migration at all in a backend somebody else wrote.
//!
//! The migrations here run above the backends, against the ordinary
//! [`Store`] API, so a data migration is written once and every backend gets
//! it. The store remembers the highest version it has run under a custom value,
//! and a migration that fails leaves that version where it was, so the next
//! start tries again.
//!
//! # Adding a migration
//!
//! Implement [`DataMigration`] with the next unused [`DataMigration::version`],
//! and add it to [`builtin_data_migrations`]. A migration must be safe to run
//! against a store that has already had it applied: a crash partway through
//! leaves the version unchanged, so it will be run again.

use std::fmt;

use async_trait::async_trait;
use tracing::{debug, info, instrument};

use super::{Result, Store};
use crate::crypto::identities::manager::IdentityManager;

/// The custom value under which the store remembers how far its data
/// migrations have got.
const DATA_MIGRATION_VERSION_KEY: &str = "crypto-store-data-migration-version";

/// The custom value the pre-framework verified-latch migration used as its
/// "already done" flag.
///
/// A store written by an older version of the SDK has this set and no migration
/// version, and must not have [`PostVerifiedLatchSupport`] run against it a
/// second time.
const HAS_MIGRATED_VERIFICATION_LATCH: &str = "HAS_MIGRATED_VERIFICATION_LATCH";

/// What a [`DataMigration`] is given to do its work.
pub(crate) struct DataMigrationContext<'a> {
    /// The store whose contents are being migrated.
    pub store: &'a Store,

    /// The identity manager, for migrations that need to disturb the device
    /// and identity tracking.
    pub identity_manager: &'a IdentityManager,
}

/// A migration of the contents of the crypto store.
///
/// See the [module documentation](self) for how these relate to the backends'
/// own schema migrations.
#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
pub(crate) trait DataMigration: fmt::Debug + Send + Sync {
    /// The version this migration brings the store to.
    ///
    /// Must be greater than zero, and unique across all migrations. Migrations
    /// run in ascending order of version.
    fn version(&self) -> u32;

    /// A short name for this migration, used in logs.
    fn name(&self) -> &'static str;

    /// Apply the migration.
    ///
    /// Must tolerate being run against a store it has already been run
    /// against: a crash before the version is written means it runs again.
    async fn migrate(&self, context: &DataMigrationContext<'_>) -> Result<()>;
}

/// The migrations that ship with the SDK, in ascending version order.
pub(crate) fn builtin_data_migrations() -> Vec<Box<dyn DataMigration>> {
    vec![Box::new(PostVerifiedLatchSupport)]
}

/// Run every migration the store has not run yet.
///
/// Returns the version the store is at afterwards.
#[instrument(skip_all)]
pub(crate) async fn run_data_migrations(
    context: &DataMigrationContext<'_>,
    migrations: &[Box<dyn DataMigration>],
) -> Result<u32> {
    let mut version = stored_version(context.store).await?;

    let mut migrations: Vec<_> = migrations.iter().collect();
    migrations.sort_by_key(|migration| migration.version());

    for migration in migrations {
        debug_assert_ne!(migration.version(), 0, "A data migration must have a non-zero version");

        if migration.version() <= version {
            continue;
        }

        info!(
            version = migration.version(),
            name = migration.name(),
            "Running a crypto store data migration",
        );

        migration.migrate(context).await?;

        // Written per migration rather than once at the end, so a failure
        // halfway through a list only replays the migration that failed.
        set_stored_version(context.store, migration.version()).await?;
        version = migration.version();
    }

    Ok(version)
}

/// How far the store's data migrations have got.
async fn stored_version(store: &Store) -> Result<u32> {
    if let Some(value) = store.get_custom_value(DATA_MIGRATION_VERSION_KEY).await? {
        let bytes: [u8; 4] = value.as_slice().try_into().unwrap_or_else(|_| {
            // Nothing but this module writes the key, so this cannot happen
            // without the store handing back something it was not given.
            tracing::error!(
                length = value.len(),
                "The stored data migration version is not a 32 bit number, starting over",
            );
            [0; 4]
        });

        return Ok(u32::from_le_bytes(bytes));
    }

    // A store from before this framework existed: its verified-latch flag, if
    // set, means it is already past version 1.
    if store.get_custom_value(HAS_MIGRATED_VERIFICATION_LATCH).await?.is_some() {
        debug!("Adopting the pre-framework verified latch flag as data migration version 1");
        return Ok(PostVerifiedLatchSupport.version());
    }

    Ok(0)
}

async fn set_stored_version(store: &Store, version: u32) -> Result<()> {
    store.set_custom_value(DATA_MIGRATION_VERSION_KEY, version.to_le_bytes().to_vec()).await
}

/// Mark every tracked user as needing a fresh `/keys/query`.
///
/// The SDK gained detection of a verified identity changing, which added a
/// local `verified_latch` flag on `OtherUserIdentityData`. A store written
/// before that has the flag unset on every identity, and the only way to fill
/// it in is to fetch the identities again.
#[derive(Debug)]
struct PostVerifiedLatchSupport;

#[cfg_attr(target_family = "wasm", async_trait(?Send))]
#[cfg_attr(not(target_family = "wasm"), async_trait)]
impl DataMigration for PostVerifiedLatchSupport {
    fn version(&self) -> u32 {
        1
    }

    fn name(&self) -> &'static str {
        "post verified latch support"
    }

    async fn migrate(&self, context: &DataMigrationContext<'_>) -> Result<()> {
        context.identity_manager.mark_all_tracked_users_as_dirty(context.store.cache().await?).await
    }
}

/// Mark every built-in data migration as already applied on a store.
///
/// For tests that build an `OlmMachine` over a store they have set up by hand
/// and do not want the migrations disturbing.
#[cfg(test)]
pub(crate) async fn mark_all_data_migrations_as_done<S>(store: &S) -> Result<()>
where
    S: super::CryptoStore,
    S::Error: Into<super::CryptoStoreError>,
{
    let version =
        builtin_data_migrations().iter().map(|migration| migration.version()).max().unwrap_or(0);

    store
        .set_custom_value(DATA_MIGRATION_VERSION_KEY, version.to_le_bytes().to_vec())
        .await
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use async_trait::async_trait;
    use harana_matrix_common::{device_id, user_id};

    use super::{
        DataMigration, DataMigrationContext, HAS_MIGRATED_VERIFICATION_LATCH,
        PostVerifiedLatchSupport, run_data_migrations, stored_version,
    };
    use crate::{
        crypto::{OlmMachine, store::Result},
        test::async_test,
    };

    /// A migration that counts how many times it ran, and can be made to fail.
    #[derive(Debug)]
    struct CountingMigration {
        version: u32,
        runs: Arc<AtomicUsize>,
        fails: bool,
    }

    impl CountingMigration {
        fn new(version: u32) -> (Self, Arc<AtomicUsize>) {
            let runs = Arc::new(AtomicUsize::new(0));
            (Self { version, runs: runs.clone(), fails: false }, runs)
        }

        fn failing(version: u32) -> (Self, Arc<AtomicUsize>) {
            let (mut migration, runs) = Self::new(version);
            migration.fails = true;
            (migration, runs)
        }
    }

    #[cfg_attr(target_family = "wasm", async_trait(?Send))]
    #[cfg_attr(not(target_family = "wasm"), async_trait)]
    impl DataMigration for CountingMigration {
        fn version(&self) -> u32 {
            self.version
        }

        fn name(&self) -> &'static str {
            "counting"
        }

        async fn migrate(&self, _: &DataMigrationContext<'_>) -> Result<()> {
            self.runs.fetch_add(1, Ordering::SeqCst);

            if self.fails {
                Err(crate::crypto::store::CryptoStoreError::UnsupportedDatabaseVersion(0, 0))
            } else {
                Ok(())
            }
        }
    }

    async fn machine() -> OlmMachine {
        OlmMachine::new(user_id!("@alice:localhost"), device_id!("DEVICEID")).await
    }

    #[async_test]
    async fn test_migrations_run_once_and_in_order() {
        let machine = machine().await;
        let context = DataMigrationContext {
            store: machine.store(),
            identity_manager: machine.identity_manager(),
        };

        let (second, second_runs) = CountingMigration::new(3);
        let (first, first_runs) = CountingMigration::new(2);

        // Deliberately out of order.
        let migrations: Vec<Box<dyn DataMigration>> = vec![Box::new(second), Box::new(first)];

        assert_eq!(run_data_migrations(&context, &migrations).await.unwrap(), 3);
        assert_eq!(first_runs.load(Ordering::SeqCst), 1);
        assert_eq!(second_runs.load(Ordering::SeqCst), 1);

        // Running again does nothing.
        assert_eq!(run_data_migrations(&context, &migrations).await.unwrap(), 3);
        assert_eq!(first_runs.load(Ordering::SeqCst), 1);
        assert_eq!(second_runs.load(Ordering::SeqCst), 1);
    }

    #[async_test]
    async fn test_a_failing_migration_leaves_the_version_behind_it() {
        let machine = machine().await;
        let context = DataMigrationContext {
            store: machine.store(),
            identity_manager: machine.identity_manager(),
        };

        let (ok, ok_runs) = CountingMigration::new(2);
        let (bad, bad_runs) = CountingMigration::failing(3);
        let (later, later_runs) = CountingMigration::new(4);

        let migrations: Vec<Box<dyn DataMigration>> =
            vec![Box::new(ok), Box::new(bad), Box::new(later)];

        run_data_migrations(&context, &migrations)
            .await
            .expect_err("The failing migration should abort the run");

        assert_eq!(ok_runs.load(Ordering::SeqCst), 1);
        assert_eq!(bad_runs.load(Ordering::SeqCst), 1);
        assert_eq!(later_runs.load(Ordering::SeqCst), 0, "Later migrations must not run");
        assert_eq!(
            stored_version(machine.store()).await.unwrap(),
            2,
            "The store should be left at the last migration that did work",
        );
    }

    #[async_test]
    async fn test_the_old_verified_latch_flag_counts_as_version_one() {
        let machine = machine().await;
        let store = machine.store();

        // A store written by a version of the SDK from before this framework, which
        // had already run the verified latch migration.
        store.set_custom_value(HAS_MIGRATED_VERIFICATION_LATCH, vec![0]).await.unwrap();

        assert_eq!(stored_version(store).await.unwrap(), PostVerifiedLatchSupport.version());
    }
}
