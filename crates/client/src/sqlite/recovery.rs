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

//! Recovering from an unreadable SQLite database.
//!
//! A database file can end up corrupted for reasons entirely outside of the
//! SDK's control: a device losing power mid-write, a filesystem bug, a backup
//! restored halfway. SQLite then refuses every statement with `database disk
//! image is malformed`, and a client that treats that as a fatal error is
//! permanently broken with no recovery path short of a reinstall.
//!
//! The stores this module serves are caches: the event cache and the media
//! cache can both be refilled from the homeserver. So rather than failing to
//! open, they throw the unreadable file away and start over.

use std::{io, path::Path};

use rusqlite::ErrorCode;
use tracing::warn;

use crate::{
    common::executor::spawn_blocking,
    sqlite::{OpenStoreError, connection::PoolError, error::Error},
};

/// The sidecar files SQLite keeps next to the database itself. They are all
/// derived from the database, and a stale one left next to a fresh database is
/// itself a source of corruption, so they go away together.
const SIDECAR_SUFFIXES: &[&str] = &["-wal", "-shm", "-journal"];

/// Whether this SQLite error means the file cannot be read as a database at
/// all.
///
/// Both codes are terminal for a given file: SQLite reports them when the
/// bytes on disk do not describe a database it can make sense of, and no
/// amount of retrying changes that.
fn is_corruption(error: &rusqlite::Error) -> bool {
    matches!(error.sqlite_error_code(), Some(ErrorCode::DatabaseCorrupt | ErrorCode::NotADatabase))
}

/// Digs the underlying SQLite error out of a store error, if there is one.
fn sqlite_error(error: &Error) -> Option<&rusqlite::Error> {
    match error {
        Error::Sqlite(error) => Some(error),
        Error::Pool(PoolError::Backend(error)) => Some(error),
        _ => None,
    }
}

impl OpenStoreError {
    /// Whether opening the store failed because the database file is corrupted
    /// beyond repair.
    pub(crate) fn is_database_corrupted(&self) -> bool {
        let error = match self {
            Self::LoadVersion(error) | Self::LoadCipher(error) | Self::SaveCipher(error) => error,
            Self::Pool(PoolError::Backend(error)) => error,
            Self::Migration(error) => {
                let Some(error) = sqlite_error(error) else { return false };
                error
            }
            _ => return false,
        };

        is_corruption(error)
    }
}

/// Delete the database at `db_path`, along with the sidecar files SQLite keeps
/// next to it, so that the next open starts from an empty database.
///
/// A file that is already missing is not an error: the point is for none of
/// them to exist afterwards.
pub(crate) async fn delete_database(db_path: &Path) -> Result<(), io::Error> {
    let db_path = db_path.to_owned();

    spawn_blocking(move || {
        let mut paths = vec![db_path.clone()];

        for suffix in SIDECAR_SUFFIXES {
            let mut sidecar = db_path.clone().into_os_string();
            sidecar.push(suffix);
            paths.push(sidecar.into());
        }

        for path in paths {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }

        Ok(())
    })
    .await
    .expect("Deleting a file should never panic")
}

/// Open a store, recreating its database from scratch if the existing one
/// turns out to be corrupted.
///
/// `open` is called a second time after the corrupted database has been
/// deleted; if that second attempt fails too, its error is returned as-is.
pub(crate) async fn open_or_recreate<S, F, Fut>(
    store_name: &str,
    db_path: &Path,
    open: F,
) -> Result<S, OpenStoreError>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<S, OpenStoreError>>,
{
    let error = match open().await {
        Ok(store) => return Ok(store),
        Err(error) => error,
    };

    if !error.is_database_corrupted() {
        return Err(error);
    }

    warn!(
        sentry = true,
        ?db_path,
        "The {store_name} database is corrupted ({error}); recreating it from scratch"
    );

    delete_database(db_path).await.map_err(OpenStoreError::DeleteCorrupted)?;

    open().await
}

#[cfg(test)]
mod tests {
    use std::io::ErrorKind;

    use rusqlite::ffi::Error as FfiError;
    use tempfile::TempDir;

    use super::{delete_database, is_corruption};
    use crate::test::async_test;

    /// The SQLite result codes this module keys off, from
    /// <https://www.sqlite.org/rescode.html>.
    const SQLITE_BUSY: i32 = 5;
    const SQLITE_CORRUPT: i32 = 11;
    const SQLITE_NOTADB: i32 = 26;

    #[test]
    fn test_is_corruption() {
        let corrupt = rusqlite::Error::SqliteFailure(
            FfiError::new(SQLITE_CORRUPT),
            Some("database disk image is malformed".to_owned()),
        );
        assert!(is_corruption(&corrupt));

        let not_a_database = rusqlite::Error::SqliteFailure(
            FfiError::new(SQLITE_NOTADB),
            Some("file is not a database".to_owned()),
        );
        assert!(is_corruption(&not_a_database));

        // A transient failure is not corruption, and must not cost the user
        // their database.
        let busy = rusqlite::Error::SqliteFailure(
            FfiError::new(SQLITE_BUSY),
            Some("database is locked".to_owned()),
        );
        assert!(!is_corruption(&busy));

        assert!(!is_corruption(&rusqlite::Error::QueryReturnedNoRows));
    }

    #[async_test]
    async fn test_delete_database_removes_sidecars() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("matrix.sqlite3");

        let sidecars = [
            dir.path().join("matrix.sqlite3-wal"),
            dir.path().join("matrix.sqlite3-shm"),
            dir.path().join("matrix.sqlite3-journal"),
        ];

        for path in std::iter::once(&db_path).chain(sidecars.iter()) {
            std::fs::write(path, b"whatever").unwrap();
        }

        // A file that isn't ours is left alone.
        let unrelated = dir.path().join("matrix.sqlite3.backup");
        std::fs::write(&unrelated, b"whatever").unwrap();

        delete_database(&db_path).await.unwrap();

        for path in std::iter::once(&db_path).chain(sidecars.iter()) {
            assert_eq!(std::fs::metadata(path).unwrap_err().kind(), ErrorKind::NotFound);
        }
        assert!(std::fs::metadata(&unrelated).is_ok());

        // Deleting again is fine: the files are already gone.
        delete_database(&db_path).await.unwrap();
    }
}
