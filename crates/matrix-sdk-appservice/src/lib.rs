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
//
// Ported from tuwunel
// `src/service/appservice/{namespace_regex,registration_info}.rs`.

#![doc = include_str!("../README.md")]
#![warn(missing_docs, missing_debug_implementations)]

mod namespace_regex;
mod registration_info;
mod registrations;

pub use self::{
    namespace_regex::NamespaceRegex, registration_info::RegistrationInfo,
    registrations::Registrations,
};

/// A registration whose namespaces could not be compiled.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A namespace declared a regular expression that does not compile.
    #[error(transparent)]
    Regex(#[from] regex::Error),

    /// The registration's `sender_localpart` does not make a valid user ID for
    /// the server it is registered on.
    #[error(transparent)]
    SenderLocalpart(#[from] ruma::IdParseError),
}
