// Copyright 2025 Tuwunel Contributors
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

#![doc = include_str!("../../docs/store_codec.md")]
#![warn(missing_docs, missing_debug_implementations)]

mod de;
mod error;
mod ser;

#[cfg(test)]
mod tests;

pub use self::{
    de::{Ignore, IgnoreAll, from_slice},
    error::{Error, Result},
    ser::{Interfix, Json, SEP, Separator, serialize, serialize_to, serialize_to_vec},
};
