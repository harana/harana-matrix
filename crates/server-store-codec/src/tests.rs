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

//! Round trips and layout guarantees of the record codec.

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{Ignore, Interfix, Json, SEP, from_slice, serialize_to_vec};

#[test]
fn test_a_string_round_trips() {
    // A bare string is rejected at the top level: a caller with nothing but a
    // string to store has nothing to encode, and the codec says so rather than
    // pretending otherwise. One inside a tuple is the ordinary case.
    let encoded = serialize_to_vec(("@alice:localhost",)).unwrap();

    assert_eq!(encoded, b"@alice:localhost");
    assert_eq!(from_slice::<(&str,)>(&encoded).unwrap(), ("@alice:localhost",));
    assert_eq!(from_slice::<(String,)>(&encoded).unwrap(), ("@alice:localhost".to_owned(),));
}

#[test]
fn test_a_tuple_separates_its_elements() {
    let encoded = serialize_to_vec(("@alice:localhost", "!room:localhost")).unwrap();

    let mut expected = b"@alice:localhost".to_vec();
    expected.push(SEP);
    expected.extend_from_slice(b"!room:localhost");
    assert_eq!(encoded, expected);

    let (user, room): (&str, &str) = from_slice(&encoded).unwrap();
    assert_eq!((user, room), ("@alice:localhost", "!room:localhost"));
}

#[test]
fn test_integers_are_written_big_endian() {
    let encoded = serialize_to_vec(1_u64).unwrap();

    assert_eq!(encoded, 1_u64.to_be_bytes());
    assert_eq!(from_slice::<u64>(&encoded).unwrap(), 1);

    // Big-endian is what makes the encoded order the numeric order, which is
    // the whole reason a store can range-scan these keys.
    assert!(serialize_to_vec(1_u64).unwrap() < serialize_to_vec(2_u64).unwrap());
    assert!(serialize_to_vec(255_u64).unwrap() < serialize_to_vec(256_u64).unwrap());
}

#[test]
fn test_an_i64_round_trips_but_does_not_sort() {
    for value in [i64::MIN, -1, 0, 1, i64::MAX] {
        let encoded = serialize_to_vec(value).unwrap();
        assert_eq!(from_slice::<i64>(&encoded).unwrap(), value, "{value} did not round trip");
    }

    // A signed integer is written as-is, so its sign bit makes every negative
    // value sort above every positive one. Encode an offset unsigned value in a
    // key whose order matters.
    assert!(serialize_to_vec(0_i64).unwrap() < serialize_to_vec(1_i64).unwrap());
    assert!(serialize_to_vec(-1_i64).unwrap() > serialize_to_vec(1_i64).unwrap());
}

#[test]
fn test_a_prefix_of_a_key_is_a_scannable_prefix() {
    let key = serialize_to_vec(("@alice:localhost", "!room:localhost", 7_u64)).unwrap();
    let prefix = serialize_to_vec(("@alice:localhost", "!room:localhost", Interfix)).unwrap();

    // Interfix finalizes the prefix including its trailing separator, so the
    // prefix cannot match a longer user or room whose name merely starts the
    // same way.
    assert!(key.starts_with(&prefix), "{key:?} does not start with {prefix:?}");
    assert_eq!(prefix.last(), Some(&SEP));

    let other = serialize_to_vec(("@alice:localhost2", "!room:localhost", 7_u64)).unwrap();
    assert!(!other.starts_with(&prefix));
}

#[test]
fn test_a_json_payload_carries_a_shape_the_compact_rules_cannot() {
    #[derive(Debug, Deserialize, PartialEq, Serialize)]
    struct Content {
        body: String,
        count: u64,
    }

    let content = Content { body: "hello".to_owned(), count: 3 };
    let encoded = serialize_to_vec(Json(&content)).unwrap();

    assert_eq!(encoded, br#"{"body":"hello","count":3}"#);

    let Json(decoded): Json<Content> = from_slice(&encoded).unwrap();
    assert_eq!(decoded, content);
}

#[test]
fn test_a_json_value_round_trips() {
    let value = json!({ "membership": "join", "displayname": "Alice" });
    let encoded = serialize_to_vec(Json(&value)).unwrap();

    let Json(decoded): Json<serde_json::Value> = from_slice(&encoded).unwrap();
    assert_eq!(decoded, value);
}

#[test]
fn test_a_key_can_be_read_back_a_field_at_a_time() {
    let encoded = serialize_to_vec(("@alice:localhost", "!room:localhost", 7_u64)).unwrap();

    // A reader that wants only the count skips what precedes it.
    let (_, _, count): (Ignore, Ignore, u64) = from_slice(&encoded).unwrap();
    assert_eq!(count, 7);
}

#[test]
fn test_a_trailing_field_decodes_from_a_shorter_record() {
    // A record written before the tuple gained its last element.
    let encoded = serialize_to_vec(("@alice:localhost", "!room:localhost")).unwrap();

    let (user, room, added): (&str, &str, Option<u64>) = from_slice(&encoded).unwrap();

    assert_eq!((user, room), ("@alice:localhost", "!room:localhost"));
    assert_eq!(added, None, "a field the stored record predates is absent, not an error");
}

#[test]
fn test_a_separator_cannot_occur_inside_encoded_text() {
    // The separator is invalid UTF-8, so no encoded string can contain it and
    // splitting on it can never cut a string in half.
    assert!(std::str::from_utf8(&[SEP]).is_err());

    let encoded = serialize_to_vec(("a string with \u{fffd} replacement characters",)).unwrap();
    assert!(!encoded.contains(&SEP));
}
