// Copyright 2023 The Matrix.org Foundation C.I.C.
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

//! Types and traits related to the capabilities that a widget can request from
//! a client.

use std::{fmt, future::Future};

use client_common::{SendOutsideWasm, SyncOutsideWasm};
use harana_matrix_common::{
    DeviceId, UserId,
    events::{MessageLikeEventType, StateEventType},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer, ser::SerializeSeq};
use tracing::{debug, warn};

use super::{
    MessageLikeEventFilter, StateEventFilter,
    filter::{Filter, FilterInput, ToDeviceEventFilter},
};

/// Must be implemented by a component that provides functionality of deciding
/// whether a widget is allowed to use certain capabilities (typically by
/// providing a prompt to the user).
pub trait CapabilitiesProvider: SendOutsideWasm + SyncOutsideWasm + 'static {
    /// Receives a request for given capabilities and returns the actual
    /// capabilities that the clients grants to a given widget (usually by
    /// prompting the user).
    fn acquire_capabilities(
        &self,
        capabilities: Capabilities,
    ) -> impl Future<Output = Capabilities> + SendOutsideWasm;
}

/// Capabilities that a widget can request from a client.
#[derive(Clone, Debug, Default)]
#[cfg_attr(test, derive(PartialEq))]
pub struct Capabilities {
    /// Types of the messages that a widget wants to be able to fetch.
    pub read: Vec<Filter>,
    /// Types of the messages that a widget wants to be able to send.
    pub send: Vec<Filter>,
    /// If this capability is requested by the widget, it can not operate
    /// separately from the Matrix client.
    ///
    /// This means clients should not offer to open the widget in a separate
    /// browser/tab/webview that is not connected to the postmessage widget-api.
    pub requires_client: bool,
    /// This allows the widget to ask the client to update delayed events.
    pub update_delayed_event: bool,
    /// This allows the widget to send events with a delay.
    pub send_delayed_event: bool,

    /// This allows the widget to download files as per MSC4039.
    pub download_file: bool,

    /// This allows the widget to discover the RTC transports advertised by the
    /// homeserver as per MSC4515.
    pub rtc_transports: bool,
}

impl Capabilities {
    /// The capabilities an [Element Call] widget needs to run.
    ///
    /// A client can hand these back from
    /// [`CapabilitiesProvider::acquire_capabilities`] as they are, even when
    /// the widget asked for something else, or extend them where it has to.
    ///
    /// `own_user_id` and `own_device_id` are the ones of the session running
    /// the call: several of the capabilities are state keys derived from them.
    ///
    /// [Element Call]: https://github.com/element-hq/element-call
    pub fn element_call_required(own_user_id: &UserId, own_device_id: &DeviceId) -> Self {
        let read_send = vec![
            // To read and send rageshake requests from other room members
            Filter::MessageLike(MessageLikeEventFilter::WithType(
                "org.matrix.rageshake_request".into(),
            )),
            // To read and send encryption keys
            Filter::ToDevice(ToDeviceEventFilter::new("io.element.call.encryption_keys".into())),
            // TODO change this to the appropriate to-device version once ready
            // remove this once all matrixRTC call apps supports to-device encryption.
            Filter::MessageLike(MessageLikeEventFilter::WithType(
                "io.element.call.encryption_keys".into(),
            )),
            // To read and send custom EC reactions. They are different to normal `m.reaction`
            // because they can be send multiple times to the same event.
            Filter::MessageLike(MessageLikeEventFilter::WithType(
                "io.element.call.reaction".into(),
            )),
            // This allows send raise hand reactions.
            Filter::MessageLike(MessageLikeEventFilter::WithType(MessageLikeEventType::Reaction)),
            // This allows to detect if someone does not raise their hand anymore.
            Filter::MessageLike(MessageLikeEventFilter::WithType(
                MessageLikeEventType::RoomRedaction,
            )),
            // This allows declining an incoming call and detect if someone declines a call.
            Filter::MessageLike(MessageLikeEventFilter::WithType(MessageLikeEventType::RtcDecline)),
        ];

        Self {
            read: vec![
                // To compute the current state of the matrixRTC session.
                Filter::State(StateEventFilter::WithType(StateEventType::CallMember)),
                // To display the name of the room.
                Filter::State(StateEventFilter::WithType(StateEventType::RoomName)),
                // To detect leaving/kicked room members during a call.
                Filter::State(StateEventFilter::WithType(StateEventType::RoomMember)),
                // To decide whether to encrypt the call streams based on the room encryption
                // setting.
                Filter::State(StateEventFilter::WithType(StateEventType::RoomEncryption)),
                // This allows the widget to check the room version, so it can know about
                // version-specific auth rules (namely MSC3779).
                Filter::State(StateEventFilter::WithType(StateEventType::RoomCreate)),
            ]
            .into_iter()
            .chain(read_send.clone())
            .collect(),
            send: vec![
                // To notify other users that a call has started.
                Filter::MessageLike(MessageLikeEventFilter::WithType(
                    MessageLikeEventType::RtcNotification,
                )),
                // Also for call notifications, except this is the deprecated fallback type which
                // Element Call still sends.
                // Deprecated for now, kept for backward compatibility as widgets will send both
                // CallNotify and RtcNotification.
                Filter::MessageLike(MessageLikeEventFilter::WithType(
                    MessageLikeEventType::CallNotify,
                )),
                // To send the call participation state event (main MatrixRTC event).
                // This is required for legacy state events (using only one event for all devices
                // with a membership array). TODO: remove once legacy call member
                // events are sunset.
                Filter::State(StateEventFilter::WithTypeAndStateKey(
                    StateEventType::CallMember,
                    own_user_id.to_string(),
                )),
                // `delayed_event` version for session memberhips
                // [MSC3779](https://github.com/matrix-org/matrix-spec-proposals/pull/3779), with no leading underscore.
                Filter::State(StateEventFilter::WithTypeAndStateKey(
                    StateEventType::CallMember,
                    format!("{own_user_id}_{own_device_id}"),
                )),
                // Same as above for [MSC3779] and [MSC4143](https://github.com/matrix-org/matrix-spec-proposals/pull/4143),
                // with application suffix
                Filter::State(StateEventFilter::WithTypeAndStateKey(
                    StateEventType::CallMember,
                    format!("{own_user_id}_{own_device_id}_m.call"),
                )),
                // The same as above but with an underscore.
                // To work around the issue that state events starting with `@` have to be Matrix
                // id's but we use mxId+deviceId.
                Filter::State(StateEventFilter::WithTypeAndStateKey(
                    StateEventType::CallMember,
                    format!("_{own_user_id}_{own_device_id}"),
                )),
                // Same as above for [MSC4143], with application suffix
                Filter::State(StateEventFilter::WithTypeAndStateKey(
                    StateEventType::CallMember,
                    format!("_{own_user_id}_{own_device_id}_m.call"),
                )),
            ]
            .into_iter()
            .chain(read_send)
            .collect(),
            requires_client: true,
            update_delayed_event: true,
            send_delayed_event: true,
            download_file: true,
            rtc_transports: true,
        }
    }

    /// Checks if a given event is allowed to be forwarded to the widget.
    ///
    /// - `event_filter_input` is a minimized event representation that contains
    ///   only the information needed to check if the widget is allowed to
    ///   receive the event. (See [`FilterInput`])
    pub(super) fn allow_reading<'a>(
        &self,
        event_filter_input: impl TryInto<FilterInput<'a>>,
    ) -> bool {
        match &event_filter_input.try_into() {
            Err(_) => {
                warn!("Failed to convert event into filter input for `allow_reading`.");
                false
            }
            Ok(filter_input) => self.read.iter().any(|f| f.matches(filter_input)),
        }
    }

    /// Checks if a given event is allowed to be sent by the widget.
    ///
    /// - `event_filter_input` is a minimized event representation that contains
    ///   only the information needed to check if the widget is allowed to send
    ///   the event to a matrix room. (See [`FilterInput`])
    pub(super) fn allow_sending<'a>(
        &self,
        event_filter_input: impl TryInto<FilterInput<'a>>,
    ) -> bool {
        match &event_filter_input.try_into() {
            Err(_) => {
                warn!("Failed to convert event into filter input for `allow_sending`.");
                false
            }
            Ok(filter_input) => self.send.iter().any(|f| f.matches(filter_input)),
        }
    }

    /// Checks if a filter exists for the given event type, useful for
    /// optimization. Avoids unnecessary read event requests when no matching
    /// filter is present.
    pub(super) fn has_read_filter_for_type(&self, event_type: &str) -> bool {
        self.read.iter().any(|f| f.filter_event_type() == event_type)
    }
}

pub(super) const SEND_EVENT: &str = "org.matrix.msc2762.send.event";
pub(super) const READ_EVENT: &str = "org.matrix.msc2762.receive.event";
pub(super) const SEND_STATE: &str = "org.matrix.msc2762.send.state_event";
pub(super) const READ_STATE: &str = "org.matrix.msc2762.receive.state_event";
pub(super) const SEND_TODEVICE: &str = "org.matrix.msc3819.send.to_device";
pub(super) const READ_TODEVICE: &str = "org.matrix.msc3819.receive.to_device";
pub(super) const REQUIRES_CLIENT: &str = "io.element.requires_client";
pub(super) const SEND_DELAYED_EVENT: &str = "org.matrix.msc4157.send.delayed_event";
pub(super) const UPDATE_DELAYED_EVENT: &str = "org.matrix.msc4157.update_delayed_event";

pub(super) const DOWNLOAD_FILE: &str = "org.matrix.msc4039.download_file";

pub(super) const RTC_TRANSPORTS: &str = "org.matrix.msc4515.rtc_transports";

impl Serialize for Capabilities {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        struct PrintEventFilter<'a>(&'a Filter);
        impl fmt::Display for PrintEventFilter<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    Filter::MessageLike(filter) => PrintMessageLikeEventFilter(filter).fmt(f),
                    Filter::State(filter) => PrintStateEventFilter(filter).fmt(f),
                    Filter::ToDevice(filter) => {
                        // As per MSC 3819 https://github.com/matrix-org/matrix-spec-proposals/pull/3819
                        // ToDevice capabilities is in the form of `m.send.to_device:<event type>`
                        // or `m.receive.to_device:<event type>`
                        write!(f, "{}", filter.event_type)
                    }
                }
            }
        }

        struct PrintMessageLikeEventFilter<'a>(&'a MessageLikeEventFilter);
        impl fmt::Display for PrintMessageLikeEventFilter<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                match self.0 {
                    MessageLikeEventFilter::WithType(event_type) => {
                        // TODO: escape `#` as `\#` and `\` as `\\` in event_type
                        write!(f, "{event_type}")
                    }
                    MessageLikeEventFilter::RoomMessageWithMsgtype(msgtype) => {
                        write!(f, "m.room.message#{msgtype}")
                    }
                }
            }
        }

        struct PrintStateEventFilter<'a>(&'a StateEventFilter);
        impl fmt::Display for PrintStateEventFilter<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                // TODO: escape `#` as `\#` and `\` as `\\` in event_type
                match self.0 {
                    StateEventFilter::WithType(event_type) => write!(f, "{event_type}"),
                    StateEventFilter::WithTypeAndStateKey(event_type, state_key) => {
                        write!(f, "{event_type}#{state_key}")
                    }
                }
            }
        }

        let mut seq = serializer.serialize_seq(None)?;

        if self.requires_client {
            seq.serialize_element(REQUIRES_CLIENT)?;
        }
        if self.update_delayed_event {
            seq.serialize_element(UPDATE_DELAYED_EVENT)?;
        }
        if self.send_delayed_event {
            seq.serialize_element(SEND_DELAYED_EVENT)?;
        }
        if self.download_file {
            seq.serialize_element(DOWNLOAD_FILE)?;
        }
        if self.rtc_transports {
            seq.serialize_element(RTC_TRANSPORTS)?;
        }
        for filter in &self.read {
            let name = match filter {
                Filter::MessageLike(_) => READ_EVENT,
                Filter::State(_) => READ_STATE,
                Filter::ToDevice(_) => READ_TODEVICE,
            };
            seq.serialize_element(&format!("{name}:{}", PrintEventFilter(filter)))?;
        }
        for filter in &self.send {
            let name = match filter {
                Filter::MessageLike(_) => SEND_EVENT,
                Filter::State(_) => SEND_STATE,
                Filter::ToDevice(_) => SEND_TODEVICE,
            };
            seq.serialize_element(&format!("{name}:{}", PrintEventFilter(filter)))?;
        }

        seq.end()
    }
}

impl<'de> Deserialize<'de> for Capabilities {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        enum Permission {
            RequiresClient,
            UpdateDelayedEvent,
            SendDelayedEvent,
            DownloadFile,
            RtcTransports,
            Read(Filter),
            Send(Filter),
            Unknown,
        }

        impl<'de> Deserialize<'de> for Permission {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                let s = harana_matrix_common::serde::deserialize_cow_str(deserializer)?;
                if s == REQUIRES_CLIENT {
                    return Ok(Self::RequiresClient);
                }
                if s == UPDATE_DELAYED_EVENT {
                    return Ok(Self::UpdateDelayedEvent);
                }
                if s == SEND_DELAYED_EVENT {
                    return Ok(Self::SendDelayedEvent);
                }
                if s == DOWNLOAD_FILE {
                    return Ok(Self::DownloadFile);
                }
                if s == RTC_TRANSPORTS {
                    return Ok(Self::RtcTransports);
                }

                match s.split_once(':') {
                    Some((READ_EVENT, filter_s)) => Ok(Permission::Read(Filter::MessageLike(
                        parse_message_event_filter(filter_s),
                    ))),
                    Some((SEND_EVENT, filter_s)) => Ok(Permission::Send(Filter::MessageLike(
                        parse_message_event_filter(filter_s),
                    ))),
                    Some((READ_STATE, filter_s)) => {
                        Ok(Permission::Read(Filter::State(parse_state_event_filter(filter_s))))
                    }
                    Some((SEND_STATE, filter_s)) => {
                        Ok(Permission::Send(Filter::State(parse_state_event_filter(filter_s))))
                    }
                    Some((READ_TODEVICE, filter_s)) => Ok(Permission::Read(Filter::ToDevice(
                        parse_to_device_event_filter(filter_s),
                    ))),
                    Some((SEND_TODEVICE, filter_s)) => Ok(Permission::Send(Filter::ToDevice(
                        parse_to_device_event_filter(filter_s),
                    ))),
                    _ => {
                        debug!("Unknown capability `{s}`");
                        Ok(Self::Unknown)
                    }
                }
            }
        }

        fn parse_message_event_filter(s: &str) -> MessageLikeEventFilter {
            match s.strip_prefix("m.room.message#") {
                Some(msgtype) => MessageLikeEventFilter::RoomMessageWithMsgtype(msgtype.to_owned()),
                // TODO: Replace `\\` by `\` and `\#` by `#`, enforce no unescaped `#`
                None => MessageLikeEventFilter::WithType(s.into()),
            }
        }

        fn parse_state_event_filter(s: &str) -> StateEventFilter {
            // TODO: Search for un-escaped `#` only, replace `\\` by `\` and `\#` by `#`
            match s.split_once('#') {
                Some((event_type, state_key)) => {
                    StateEventFilter::WithTypeAndStateKey(event_type.into(), state_key.to_owned())
                }
                None => StateEventFilter::WithType(s.into()),
            }
        }

        fn parse_to_device_event_filter(s: &str) -> ToDeviceEventFilter {
            ToDeviceEventFilter::new(s.into())
        }

        let mut capabilities = Capabilities::default();
        for capability in Vec::<Permission>::deserialize(deserializer)? {
            match capability {
                Permission::RequiresClient => capabilities.requires_client = true,
                Permission::Read(filter) => capabilities.read.push(filter),
                Permission::Send(filter) => capabilities.send.push(filter),
                // ignore unknown capabilities
                Permission::Unknown => {}
                Permission::UpdateDelayedEvent => capabilities.update_delayed_event = true,
                Permission::SendDelayedEvent => capabilities.send_delayed_event = true,
                Permission::DownloadFile => capabilities.download_file = true,
                Permission::RtcTransports => capabilities.rtc_transports = true,
            }
        }

        Ok(capabilities)
    }
}

#[cfg(test)]
mod tests {
    use harana_matrix_common::{device_id, events::StateEventType, user_id};

    use super::*;
    use crate::widget::filter::ToDeviceEventFilter;

    #[test]
    fn element_call_required_capabilities_are_complete() {
        let capabilities = Capabilities::element_call_required(
            user_id!("@my_user:my-domain.org"),
            device_id!("ABCDEFGHI"),
        );

        // Serializing gives the capability strings a widget is granted, in a
        // list whose order is not part of the contract.
        let serialized = serde_json::to_string(&capabilities).unwrap();
        let granted: Vec<String> = serde_json::from_str(&serialized).unwrap();

        let assert_granted = |capability: &str| {
            assert!(
                granted.contains(&capability.to_owned()),
                "the \"{capability}\" capability is missing from the Element Call list"
            );
        };

        assert_granted("io.element.requires_client");
        assert_granted("org.matrix.msc4157.update_delayed_event");
        assert_granted("org.matrix.msc4157.send.delayed_event");
        assert_granted("org.matrix.msc2762.receive.state_event:org.matrix.msc3401.call.member");
        assert_granted("org.matrix.msc2762.receive.state_event:m.room.name");
        assert_granted("org.matrix.msc2762.receive.state_event:m.room.member");
        assert_granted("org.matrix.msc2762.receive.state_event:m.room.encryption");
        assert_granted("org.matrix.msc2762.receive.state_event:m.room.create");
        assert_granted("org.matrix.msc2762.receive.event:org.matrix.rageshake_request");
        assert_granted("org.matrix.msc2762.receive.event:io.element.call.encryption_keys");
        assert_granted("org.matrix.msc2762.send.event:org.matrix.rageshake_request");
        assert_granted("org.matrix.msc2762.send.event:io.element.call.encryption_keys");

        // The state keys the session's own membership events are sent under.
        for state_key in [
            "@my_user:my-domain.org",
            "@my_user:my-domain.org_ABCDEFGHI",
            "@my_user:my-domain.org_ABCDEFGHI_m.call",
            "_@my_user:my-domain.org_ABCDEFGHI",
            "_@my_user:my-domain.org_ABCDEFGHI_m.call",
        ] {
            assert_granted(&format!(
                "org.matrix.msc2762.send.state_event:org.matrix.msc3401.call.member#{state_key}"
            ));
        }

        // RTC decline
        assert_granted("org.matrix.msc2762.receive.event:org.matrix.msc4310.rtc.decline");
        assert_granted("org.matrix.msc2762.send.event:org.matrix.msc4310.rtc.decline");

        // Download avatars
        assert_granted("org.matrix.msc4039.download_file");
    }

    #[test]
    fn deserialization_of_no_capabilities() {
        let capabilities_str = r#"[]"#;

        let parsed = serde_json::from_str::<Capabilities>(capabilities_str).unwrap();
        let expected = Capabilities::default();

        assert_eq!(parsed, expected);
    }

    #[test]
    fn deserialization_of_capabilities() {
        let capabilities_str = r#"[
            "m.always_on_screen",
            "io.element.requires_client",
            "org.matrix.msc2762.receive.event:org.matrix.rageshake_request",
            "org.matrix.msc2762.receive.state_event:m.room.member",
            "org.matrix.msc2762.receive.state_event:org.matrix.msc3401.call.member",
            "org.matrix.msc3819.receive.to_device:io.element.call.encryption_keys",
            "org.matrix.msc2762.send.event:org.matrix.rageshake_request",
            "org.matrix.msc2762.send.state_event:org.matrix.msc3401.call.member#@user:matrix.server",
            "org.matrix.msc3819.send.to_device:io.element.call.encryption_keys",
            "org.matrix.msc4157.send.delayed_event",
            "org.matrix.msc4157.update_delayed_event",
            "org.matrix.msc4039.download_file",
            "org.matrix.msc4515.rtc_transports"
        ]"#;

        let parsed = serde_json::from_str::<Capabilities>(capabilities_str).unwrap();
        let expected = Capabilities {
            read: vec![
                Filter::MessageLike(MessageLikeEventFilter::WithType(
                    "org.matrix.rageshake_request".into(),
                )),
                Filter::State(StateEventFilter::WithType(StateEventType::RoomMember)),
                Filter::State(StateEventFilter::WithType("org.matrix.msc3401.call.member".into())),
                Filter::ToDevice(ToDeviceEventFilter::new(
                    "io.element.call.encryption_keys".into(),
                )),
            ],
            send: vec![
                Filter::MessageLike(MessageLikeEventFilter::WithType(
                    "org.matrix.rageshake_request".into(),
                )),
                Filter::State(StateEventFilter::WithTypeAndStateKey(
                    "org.matrix.msc3401.call.member".into(),
                    "@user:matrix.server".into(),
                )),
                Filter::ToDevice(ToDeviceEventFilter::new(
                    "io.element.call.encryption_keys".into(),
                )),
            ],
            requires_client: true,
            update_delayed_event: true,
            send_delayed_event: true,
            download_file: true,
            rtc_transports: true,
        };

        assert_eq!(parsed, expected);
    }

    #[test]
    fn serialization_and_deserialization_are_symmetrical() {
        let capabilities = Capabilities {
            read: vec![
                Filter::MessageLike(MessageLikeEventFilter::WithType("io.element.custom".into())),
                Filter::State(StateEventFilter::WithType(StateEventType::RoomMember)),
                Filter::State(StateEventFilter::WithTypeAndStateKey(
                    "org.matrix.msc3401.call.member".into(),
                    "@user:matrix.server".into(),
                )),
                Filter::ToDevice(ToDeviceEventFilter::new(
                    "io.element.call.encryption_keys".into(),
                )),
            ],
            send: vec![
                Filter::MessageLike(MessageLikeEventFilter::WithType("io.element.custom".into())),
                Filter::State(StateEventFilter::WithTypeAndStateKey(
                    "org.matrix.msc3401.call.member".into(),
                    "@user:matrix.server".into(),
                )),
                Filter::ToDevice(ToDeviceEventFilter::new("my.org.other.to_device_event".into())),
            ],
            requires_client: true,
            update_delayed_event: false,
            send_delayed_event: false,
            download_file: false,
            rtc_transports: true,
        };

        let capabilities_str = serde_json::to_string(&capabilities).unwrap();
        let parsed = serde_json::from_str::<Capabilities>(&capabilities_str).unwrap();
        assert_eq!(parsed, capabilities);
    }
}
