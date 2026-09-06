// Copyright 2022 The Matrix.org Foundation C.I.C.
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

use std::{error::Error, fmt, future::IntoFuture, time::Duration};

use futures_util::future::{Either, select};

use crate::sleep::sleep;

/// Error type notifying that a timeout has elapsed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ElapsedError();

impl fmt::Display for ElapsedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "time waiting for future has elapsed!")
    }
}

impl Error for ElapsedError {}

/// Wait for `future` to be completed. `future` needs to return
/// a `Result`.
///
/// If the given timeout has elapsed the method will stop waiting and return
/// an error.
///
/// The timer comes from whichever runtime the SDK was configured with; see
/// [`crate::runtime`].
pub async fn timeout<F>(future: F, duration: Duration) -> Result<F::Output, ElapsedError>
where
    F: IntoFuture,
{
    match select(std::pin::pin!(future.into_future()), std::pin::pin!(sleep(duration))).await {
        Either::Left((result, _)) => Ok(result),
        Either::Right((_, _)) => Err(ElapsedError()),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::{future, time::Duration};

    use harana_matrix_macros::async_test;

    use super::timeout;

    #[cfg(target_family = "wasm")]
    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[async_test]
    async fn test_without_timeout() {
        timeout(future::ready(()), Duration::from_millis(100))
            .await
            .expect("future should have completed without ElapsedError");
    }

    #[async_test]
    async fn test_with_timeout() {
        timeout(future::pending::<()>(), Duration::from_millis(100))
            .await
            .expect_err("future should return an ElapsedError");
    }
}
