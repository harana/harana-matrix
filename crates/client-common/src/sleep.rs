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

use std::time::Duration;

use crate::runtime;

/// Sleep for the specified duration.
///
/// The timer comes from whichever runtime the SDK was configured with, so this
/// works the same on native targets, on Wasm, and on any runtime installed with
/// [`crate::runtime::set_runtime`].
pub async fn sleep(duration: Duration) {
    runtime::runtime().sleep(duration).await;
}

#[cfg(test)]
mod tests {
    use harana_matrix_macros::async_test;

    use super::*;

    #[cfg(target_family = "wasm")]
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[async_test]
    async fn test_sleep() {
        // Just test that it doesn't panic
        sleep(Duration::from_millis(1)).await;
    }
}
