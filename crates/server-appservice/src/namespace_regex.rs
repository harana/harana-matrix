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
// Ported from tuwunel `src/service/appservice/namespace_regex.rs`.

//! The compiled regular expressions of one registration namespace.

use regex::{RegexSet, RegexSetBuilder};
use common_ruma::api::appservice::Namespace;

use crate::Error;

/// Compiled regular expressions for a namespace.
///
/// The exclusive and non-exclusive patterns are compiled separately, because
/// the two grant different rights: an exclusive match reserves the identifier
/// for this appservice, while a non-exclusive one only expresses interest in
/// it.
#[derive(Clone, Debug)]
pub struct NamespaceRegex {
    /// Patterns claiming their matches exclusively.
    pub exclusive: Option<RegexSet>,

    /// Patterns expressing interest without claiming their matches.
    pub non_exclusive: Option<RegexSet>,
}

impl NamespaceRegex {
    /// Compiles the patterns of one namespace.
    ///
    /// `case_sensitive` distinguishes room IDs, which are compared as written,
    /// from user IDs and aliases, whose localparts are case-insensitive.
    ///
    /// # Errors
    ///
    /// Returns an error if any pattern fails to compile.
    pub fn new<'a, I>(case_sensitive: bool, namespaces: I) -> Result<Self, Error>
    where
        I: Iterator<Item = &'a Namespace> + Clone,
    {
        let exclusive = namespaces
            .clone()
            .filter(|namespace| namespace.exclusive)
            .map(|namespace| namespace.regex.as_str());

        let non_exclusive = namespaces
            .filter(|namespace| !namespace.exclusive)
            .map(|namespace| namespace.regex.as_str());

        Ok(Self {
            exclusive: Self::build(case_sensitive, exclusive)?,
            non_exclusive: Self::build(case_sensitive, non_exclusive)?,
        })
    }

    /// Checks whether this namespace covers an identifier at all.
    #[inline]
    #[must_use]
    pub fn is_match(&self, input: &str) -> bool {
        self.is_exclusive_match(input)
            || self
                .non_exclusive
                .as_ref()
                .is_some_and(|non_exclusive| non_exclusive.is_match(input))
    }

    /// Checks whether this namespace claims an identifier exclusively.
    #[inline]
    #[must_use]
    pub fn is_exclusive_match(&self, input: &str) -> bool {
        self.exclusive.as_ref().is_some_and(|exclusive| exclusive.is_match(input))
    }

    /// Compiles a set of patterns, or `None` when there are none.
    ///
    /// An empty `RegexSet` matches nothing, so the distinction is not
    /// behavioral; it keeps an unused namespace from carrying a compiled set.
    fn build<'a, I>(case_sensitive: bool, patterns: I) -> Result<Option<RegexSet>, Error>
    where
        I: Iterator<Item = &'a str> + Clone,
    {
        if patterns.clone().next().is_none() {
            return Ok(None);
        }

        let set = RegexSetBuilder::new(patterns).case_insensitive(!case_sensitive).build()?;

        Ok(Some(set))
    }
}

#[cfg(test)]
mod tests {
    use common_ruma::api::appservice::Namespace;

    use super::NamespaceRegex;

    fn namespaces(patterns: &[(&str, bool)]) -> Vec<Namespace> {
        patterns
            .iter()
            .map(|(regex, exclusive)| Namespace::new(*exclusive, (*regex).to_owned()))
            .collect()
    }

    #[test]
    fn test_an_empty_namespace_matches_nothing() {
        let namespaces = namespaces(&[]);
        let regex = NamespaceRegex::new(false, namespaces.iter()).unwrap();

        assert!(regex.exclusive.is_none());
        assert!(regex.non_exclusive.is_none());
        assert!(!regex.is_match("@anyone:localhost"));
        assert!(!regex.is_exclusive_match("@anyone:localhost"));
    }

    #[test]
    fn test_an_exclusive_pattern_matches_both_ways() {
        let namespaces = namespaces(&[(r"@bridge_.*:localhost", true)]);
        let regex = NamespaceRegex::new(false, namespaces.iter()).unwrap();

        assert!(regex.is_match("@bridge_alice:localhost"));
        assert!(regex.is_exclusive_match("@bridge_alice:localhost"));
        assert!(!regex.is_match("@alice:localhost"));
    }

    #[test]
    fn test_a_non_exclusive_pattern_is_interest_without_a_claim() {
        let namespaces = namespaces(&[(r"@watched_.*:localhost", false)]);
        let regex = NamespaceRegex::new(false, namespaces.iter()).unwrap();

        assert!(regex.is_match("@watched_alice:localhost"));
        assert!(!regex.is_exclusive_match("@watched_alice:localhost"));
    }

    #[test]
    fn test_case_sensitivity_is_the_callers_choice() {
        let namespaces = namespaces(&[(r"@bridge_.*:localhost", true)]);

        let insensitive = NamespaceRegex::new(false, namespaces.iter()).unwrap();
        assert!(insensitive.is_match("@BRIDGE_alice:localhost"));

        let sensitive = NamespaceRegex::new(true, namespaces.iter()).unwrap();
        assert!(!sensitive.is_match("@BRIDGE_alice:localhost"));
        assert!(sensitive.is_match("@bridge_alice:localhost"));
    }

    #[test]
    fn test_an_invalid_pattern_is_an_error() {
        let namespaces = namespaces(&[("@bridge_[:localhost", true)]);

        assert!(NamespaceRegex::new(false, namespaces.iter()).is_err());
    }
}
