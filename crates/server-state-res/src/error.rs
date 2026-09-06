// Copyright 2026 The Harana Contributors
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

//! Errors from a check or resolution that could not be run to completion.

use crate::fetch::MAX_FETCH_ROUNDS;

/// A check or resolution that could not be run to completion.
///
/// A rejected event is not an error: authorization reports that as
/// [`AuthCheckOutcome::Deny`](crate::AuthCheckOutcome::Deny).
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// State resolution itself failed.
    #[error(transparent)]
    Resolution(#[from] harana_matrix_common::state_res::Error),

    /// The room version resolves state with the first version of the algorithm.
    ///
    /// Only room version 1 does, and Ruma implements the second version only.
    #[error("state resolution v1 is not implemented")]
    UnsupportedStateResolutionVersion,

    /// The algorithm kept asking for events the store had not been asked for.
    ///
    /// The seeds cover what the specification says each algorithm reads, so
    /// this means it read something entirely unanticipated.
    #[error("did not settle within {MAX_FETCH_ROUNDS} rounds of fetching")]
    FetchRoundsExhausted,
}
