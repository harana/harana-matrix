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

#![doc = include_str!("../README.md")]
#![warn(missing_docs, missing_debug_implementations)]

mod auth;
mod error;
mod fetch;
mod resolve;

#[doc(no_inline)]
pub use ruma::state_res::{
    Error as ResolutionError, Event, StateMap, auth_types_for_event, check_pdu_format, events,
    reverse_topological_power_sort, utils,
};

pub use self::{
    auth::{
        AuthCheckOutcome, auth_check, check_state_dependent_auth_rules,
        check_state_independent_auth_rules,
    },
    error::Error,
    fetch::MAX_FETCH_ROUNDS,
    resolve::resolve,
};
