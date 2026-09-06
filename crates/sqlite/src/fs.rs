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

//! The handful of async filesystem operations this crate needs, run on the
//! blocking pool of whichever runtime the SDK was configured with.

use std::{io, path::Path};

use sdk_common::executor::spawn_blocking;

/// Recursively create a directory and all of its parents if they are missing.
pub(crate) async fn create_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref().to_owned();

    spawn_blocking(move || std::fs::create_dir_all(path))
        .await
        .expect("Creating a directory should never panic")
}

/// Remove a directory and everything it contains.
#[cfg(test)]
pub(crate) async fn remove_dir_all(path: impl AsRef<Path>) -> io::Result<()> {
    let path = path.as_ref().to_owned();

    spawn_blocking(move || std::fs::remove_dir_all(path))
        .await
        .expect("Removing a directory should never panic")
}
