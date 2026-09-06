// Copyright 2025 The Matrix.org Foundation C.I.C.
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
// See the License for that specific language governing permissions and
// limitations under the License.

use std::sync::{Arc, Mutex};

use language_tags::LanguageTag;
use client_matrix::widget::{MessageLikeEventFilter, StateEventFilter, ToDeviceEventFilter};
use client_common::{SendOutsideWasm, SyncOutsideWasm};
use common_ruma::UserId;
use tracing::error;

use crate::{error::ClientError, room::Room, runtime::get_runtime_handle};

#[derive(uniffi::Record)]
pub struct WidgetDriverAndHandle {
    pub driver: Arc<WidgetDriver>,
    pub handle: Arc<WidgetDriverHandle>,
}

#[client_matrix_ffi_macros::export]
pub fn make_widget_driver(settings: WidgetSettings) -> Result<WidgetDriverAndHandle, ParseError> {
    let (driver, handle) = client_matrix::widget::WidgetDriver::new(settings.try_into()?);
    Ok(WidgetDriverAndHandle {
        driver: Arc::new(WidgetDriver(Mutex::new(Some(driver)))),
        handle: Arc::new(WidgetDriverHandle(handle)),
    })
}

/// An object that handles all interactions of a widget living inside a webview
/// or IFrame with the Matrix world.
#[derive(uniffi::Object)]
pub struct WidgetDriver(Mutex<Option<client_matrix::widget::WidgetDriver>>);

#[client_matrix_ffi_macros::export]
impl WidgetDriver {
    pub async fn run(
        &self,
        room: Arc<Room>,
        capabilities_provider: Box<dyn WidgetCapabilitiesProvider>,
    ) {
        let Some(driver) = self.0.lock().unwrap().take() else {
            error!("Can't call run multiple times on a WidgetDriver");
            return;
        };

        let capabilities_provider = CapabilitiesProviderWrap(capabilities_provider.into());
        if let Err(()) = driver.run((*room.inner).clone(), capabilities_provider).await {
            // TODO
        }
    }
}

/// Information about a widget.
#[derive(uniffi::Record, Clone)]
pub struct WidgetSettings {
    /// Widget's unique identifier.
    pub widget_id: String,
    /// Whether or not the widget should be initialized on load message
    /// (`ContentLoad` message), or upon creation/attaching of the widget to
    /// the SDK's state machine that drives the API.
    pub init_after_content_load: bool,
    /// This contains the url from the widget state event.
    /// In this url placeholders can be used to pass information from the client
    /// to the widget. Possible values are: `$widgetId`, `$parentUrl`,
    /// `$userId`, `$lang`, `$fontScale`, `$analyticsID`.
    ///
    /// # Examples
    ///
    /// e.g `http://widget.domain?username=$userId`
    /// will become: `http://widget.domain?username=@user_matrix_id:server.domain`.
    raw_url: String,
}

impl TryFrom<WidgetSettings> for client_matrix::widget::WidgetSettings {
    type Error = ParseError;

    fn try_from(value: WidgetSettings) -> Result<Self, Self::Error> {
        let WidgetSettings { widget_id, init_after_content_load, raw_url } = value;
        Ok(client_matrix::widget::WidgetSettings::new(widget_id, init_after_content_load, &raw_url)?)
    }
}

impl From<client_matrix::widget::WidgetSettings> for WidgetSettings {
    fn from(value: client_matrix::widget::WidgetSettings) -> Self {
        WidgetSettings {
            widget_id: value.widget_id().to_owned(),
            init_after_content_load: value.init_on_content_load(),
            raw_url: value.raw_url().to_string(),
        }
    }
}

/// Create the actual url that can be used to setup the WebView or IFrame
/// that contains the widget.
///
/// # Arguments
/// * `widget_settings` - The widget settings to generate the url for.
/// * `room` - A Matrix room which is used to query the logged in username
/// * `props` - Properties from the client that can be used by a widget to adapt
///   to the client. e.g. language, font-scale...
#[client_matrix_ffi_macros::export]
pub async fn generate_webview_url(
    widget_settings: WidgetSettings,
    room: Arc<Room>,
    props: ClientProperties,
) -> Result<String, ParseError> {
    Ok(client_matrix::widget::WidgetSettings::generate_webview_url(
        &widget_settings.clone().try_into()?,
        &room.inner,
        props.into(),
    )
    .await
    .map(|url| url.to_string())?)
}

/// `WidgetSettings` are usually created from a state event.
/// (currently unimplemented)
///
/// In some cases the client wants to create custom `WidgetSettings`
/// for specific rooms based on other conditions.
/// This function returns a `WidgetSettings` object which can be used
/// to setup a widget using `run_client_widget_api`
/// and to generate the correct url for the widget.
///
/// # Arguments
///
/// * `props` - A struct containing the configuration parameters for a element
///   call widget.
#[client_matrix_ffi_macros::export]
pub fn new_virtual_element_call_widget(
    props: client_matrix::widget::VirtualElementCallWidgetProperties,
    config: client_matrix::widget::VirtualElementCallWidgetConfig,
) -> Result<WidgetSettings, ParseError> {
    Ok(client_matrix::widget::WidgetSettings::new_virtual_element_call_widget(props, config)
        .map(|w| w.into())?)
}

/// The Capabilities required to run a element call widget.
///
/// This is intended to be used in combination with: `acquire_capabilities` of
/// the `CapabilitiesProvider`.
///
/// `acquire_capabilities` can simply return the `WidgetCapabilities` from this
/// function. Even if there are non intersecting permissions to what the widget
/// requested.
///
/// Editing and extending the capabilities from this function is also possible,
/// but should only be done as temporal workarounds until this function is
/// adjusted
///
/// The list itself lives in the SDK, as
/// [`client_matrix::widget::Capabilities::element_call_required`], so it can be
/// shared with consumers that do not go through the bindings.
#[client_matrix_ffi_macros::export]
pub fn get_element_call_required_permissions(
    own_user_id: String,
    own_device_id: String,
) -> Result<WidgetCapabilities, ClientError> {
    let own_user_id = UserId::parse(own_user_id)?;

    Ok(client_matrix::widget::Capabilities::element_call_required(
        &own_user_id,
        own_device_id.as_str().into(),
    )
    .into())
}

#[derive(uniffi::Record)]
pub struct ClientProperties {
    /// The client_id provides the widget with the option to behave differently
    /// for different clients. e.g org.example.ios.
    client_id: String,
    /// The language tag the client is set to e.g. en-us. (Undefined and invalid
    /// becomes: `en-US`)
    language_tag: Option<String>,
    /// A string describing the theme (dark, light) or org.example.dark.
    /// (default: `light`)
    theme: Option<String>,
}

impl From<ClientProperties> for client_matrix::widget::ClientProperties {
    fn from(value: ClientProperties) -> Self {
        let ClientProperties { client_id, language_tag, theme } = value;
        let language_tag = language_tag.and_then(|l| LanguageTag::parse(&l).ok());
        Self::new(&client_id, language_tag, theme)
    }
}

/// A handle that encapsulates the communication between a widget driver and the
/// corresponding widget (inside a webview or IFrame).
#[derive(uniffi::Object)]
pub struct WidgetDriverHandle(client_matrix::widget::WidgetDriverHandle);

#[client_matrix_ffi_macros::export]
impl WidgetDriverHandle {
    /// Receive a message from the widget driver.
    ///
    /// The message must be passed on to the widget.
    ///
    /// Returns `None` if the widget driver is no longer running.
    pub async fn recv(&self) -> Option<String> {
        self.0.recv().await
    }

    //// Send a message from the widget to the widget driver.
    ///
    /// Returns `false` if the widget driver is no longer running.
    pub async fn send(&self, msg: String) -> bool {
        self.0.send(msg).await
    }
}

/// Capabilities that a widget can request from a client.
#[derive(uniffi::Record)]
pub struct WidgetCapabilities {
    /// Types of the messages that a widget wants to be able to fetch.
    pub read: Vec<WidgetEventFilter>,
    /// Types of the messages that a widget wants to be able to send.
    pub send: Vec<WidgetEventFilter>,
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
    /// This allows the widget to download files (avatars)
    pub download_files: bool,
    /// This allows the widget to discover the RTC transports advertised by the
    /// homeserver (MSC4515).
    pub rtc_transports: bool,
}

impl From<WidgetCapabilities> for client_matrix::widget::Capabilities {
    fn from(value: WidgetCapabilities) -> Self {
        Self {
            read: value.read.into_iter().map(Into::into).collect(),
            send: value.send.into_iter().map(Into::into).collect(),
            requires_client: value.requires_client,
            update_delayed_event: value.update_delayed_event,
            send_delayed_event: value.send_delayed_event,
            download_file: value.download_files,
            rtc_transports: value.rtc_transports,
        }
    }
}

impl From<client_matrix::widget::Capabilities> for WidgetCapabilities {
    fn from(value: client_matrix::widget::Capabilities) -> Self {
        Self {
            read: value.read.into_iter().map(Into::into).collect(),
            send: value.send.into_iter().map(Into::into).collect(),
            requires_client: value.requires_client,
            update_delayed_event: value.update_delayed_event,
            send_delayed_event: value.send_delayed_event,
            download_files: value.download_file,
            rtc_transports: value.rtc_transports,
        }
    }
}

/// Different kinds of filters that could be applied to the timeline events.
#[derive(uniffi::Enum, Clone)]
pub enum WidgetEventFilter {
    /// Matches message-like events with the given `type`.
    MessageLikeWithType { event_type: String },
    /// Matches `m.room.message` events with the given `msgtype`.
    RoomMessageWithMsgtype { msgtype: String },
    /// Matches state events with the given `type`, regardless of `state_key`.
    StateWithType { event_type: String },
    /// Matches state events with the given `type` and `state_key`.
    StateWithTypeAndStateKey { event_type: String, state_key: String },
    /// Matches to-device events with the given `event_type`.
    ToDevice { event_type: String },
}

impl From<WidgetEventFilter> for client_matrix::widget::Filter {
    fn from(value: WidgetEventFilter) -> Self {
        match value {
            WidgetEventFilter::MessageLikeWithType { event_type } => {
                Self::MessageLike(MessageLikeEventFilter::WithType(event_type.into()))
            }
            WidgetEventFilter::RoomMessageWithMsgtype { msgtype } => {
                Self::MessageLike(MessageLikeEventFilter::RoomMessageWithMsgtype(msgtype))
            }
            WidgetEventFilter::StateWithType { event_type } => {
                Self::State(StateEventFilter::WithType(event_type.into()))
            }
            WidgetEventFilter::StateWithTypeAndStateKey { event_type, state_key } => {
                Self::State(StateEventFilter::WithTypeAndStateKey(event_type.into(), state_key))
            }
            WidgetEventFilter::ToDevice { event_type } => {
                Self::ToDevice(ToDeviceEventFilter { event_type: event_type.into() })
            }
        }
    }
}

impl From<client_matrix::widget::Filter> for WidgetEventFilter {
    fn from(value: client_matrix::widget::Filter) -> Self {
        use client_matrix::widget::Filter as F;

        match value {
            F::MessageLike(MessageLikeEventFilter::WithType(event_type)) => {
                Self::MessageLikeWithType { event_type: event_type.to_string() }
            }
            F::MessageLike(MessageLikeEventFilter::RoomMessageWithMsgtype(msgtype)) => {
                Self::RoomMessageWithMsgtype { msgtype }
            }
            F::State(StateEventFilter::WithType(event_type)) => {
                Self::StateWithType { event_type: event_type.to_string() }
            }
            F::State(StateEventFilter::WithTypeAndStateKey(event_type, state_key)) => {
                Self::StateWithTypeAndStateKey { event_type: event_type.to_string(), state_key }
            }
            F::ToDevice(ToDeviceEventFilter { event_type }) => {
                Self::ToDevice { event_type: event_type.to_string() }
            }
        }
    }
}

#[client_matrix_ffi_macros::export(callback_interface)]
pub trait WidgetCapabilitiesProvider: SendOutsideWasm + SyncOutsideWasm {
    fn acquire_capabilities(&self, capabilities: WidgetCapabilities) -> WidgetCapabilities;
}

struct CapabilitiesProviderWrap(Arc<dyn WidgetCapabilitiesProvider>);

impl client_matrix::widget::CapabilitiesProvider for CapabilitiesProviderWrap {
    async fn acquire_capabilities(
        &self,
        capabilities: client_matrix::widget::Capabilities,
    ) -> client_matrix::widget::Capabilities {
        let this = self.0.clone();
        // This could require a prompt to the user. Ideally the callback
        // interface would just be async, but that's not supported yet so use
        // one of tokio's blocking task threads instead.
        get_runtime_handle()
            .spawn_blocking(move || this.acquire_capabilities(capabilities.into()).into())
            .await
            // propagate panics from the blocking task
            .unwrap()
    }
}

#[derive(Debug, thiserror::Error, uniffi::Error)]
#[uniffi(flat_error)]
pub enum ParseError {
    #[error("empty host")]
    EmptyHost,
    #[error("invalid international domain name")]
    IdnaError,
    #[error("invalid port number")]
    InvalidPort,
    #[error("invalid IPv4 address")]
    InvalidIpv4Address,
    #[error("invalid IPv6 address")]
    InvalidIpv6Address,
    #[error("invalid domain character")]
    InvalidDomainCharacter,
    #[error("relative URL without a base")]
    RelativeUrlWithoutBase,
    #[error("relative URL with a cannot-be-a-base base")]
    RelativeUrlWithCannotBeABaseBase,
    #[error("a cannot-be-a-base URL doesn’t have a host to set")]
    SetHostOnCannotBeABaseUrl,
    #[error("URLs more than 4 GB are not supported")]
    Overflow,
    #[error("unknown URL parsing error")]
    Other,
}

impl From<url::ParseError> for ParseError {
    fn from(value: url::ParseError) -> Self {
        match value {
            url::ParseError::EmptyHost => Self::EmptyHost,
            url::ParseError::IdnaError => Self::IdnaError,
            url::ParseError::InvalidPort => Self::InvalidPort,
            url::ParseError::InvalidIpv4Address => Self::InvalidIpv4Address,
            url::ParseError::InvalidIpv6Address => Self::InvalidIpv6Address,
            url::ParseError::InvalidDomainCharacter => Self::InvalidDomainCharacter,
            url::ParseError::RelativeUrlWithoutBase => Self::RelativeUrlWithoutBase,
            url::ParseError::RelativeUrlWithCannotBeABaseBase => {
                Self::RelativeUrlWithCannotBeABaseBase
            }
            url::ParseError::SetHostOnCannotBeABaseUrl => Self::SetHostOnCannotBeABaseUrl,
            url::ParseError::Overflow => Self::Overflow,
            _ => Self::Other,
        }
    }
}

#[cfg(test)]
mod tests {
    use client_matrix::widget::Capabilities;

    use super::get_element_call_required_permissions;

    #[test]
    fn element_call_permissions_are_correct() {
        let widget_cap = get_element_call_required_permissions(
            "@my_user:my-domain.org".to_owned(),
            "ABCDEFGHI".to_owned(),
        )
        .expect("the user ID is a valid one");

        // We test two things:

        // Converting the WidgetCapability (ffi struct) to Capabilities (rust sdk
        // struct)
        let cap = Into::<Capabilities>::into(widget_cap);
        // Converting Capabilities (rust sdk struct) to a json list.
        let cap_json_repr = serde_json::to_string(&cap).unwrap();

        // Converting to a Vec<String> allows to check if the required elements exist
        // without breaking the test each time the order of permissions might
        // change.
        let permission_array: Vec<String> = serde_json::from_str(&cap_json_repr).unwrap();

        let cap_assert = |capability: &str| {
            assert!(
                permission_array.contains(&capability.to_owned()),
                "The \"{capability}\" capability was missing from the element call capability list."
            );
        };

        cap_assert("io.element.requires_client");
        cap_assert("org.matrix.msc4157.update_delayed_event");
        cap_assert("org.matrix.msc4157.send.delayed_event");
        cap_assert("org.matrix.msc2762.receive.state_event:org.matrix.msc3401.call.member");
        cap_assert("org.matrix.msc2762.receive.state_event:m.room.name");
        cap_assert("org.matrix.msc2762.receive.state_event:m.room.member");
        cap_assert("org.matrix.msc2762.receive.state_event:m.room.encryption");
        cap_assert("org.matrix.msc2762.receive.event:org.matrix.rageshake_request");
        cap_assert("org.matrix.msc2762.receive.event:io.element.call.encryption_keys");
        cap_assert("org.matrix.msc2762.receive.state_event:m.room.create");
        cap_assert(
            "org.matrix.msc2762.send.state_event:org.matrix.msc3401.call.member#@my_user:my-domain.org",
        );
        cap_assert(
            "org.matrix.msc2762.send.state_event:org.matrix.msc3401.call.member#@my_user:my-domain.org_ABCDEFGHI",
        );
        cap_assert(
            "org.matrix.msc2762.send.state_event:org.matrix.msc3401.call.member#@my_user:my-domain.org_ABCDEFGHI_m.call",
        );
        cap_assert(
            "org.matrix.msc2762.send.state_event:org.matrix.msc3401.call.member#_@my_user:my-domain.org_ABCDEFGHI",
        );
        cap_assert(
            "org.matrix.msc2762.send.state_event:org.matrix.msc3401.call.member#_@my_user:my-domain.org_ABCDEFGHI_m.call",
        );
        cap_assert("org.matrix.msc2762.send.event:org.matrix.rageshake_request");
        cap_assert("org.matrix.msc2762.send.event:io.element.call.encryption_keys");

        // RTC decline
        cap_assert("org.matrix.msc2762.receive.event:org.matrix.msc4310.rtc.decline");
        cap_assert("org.matrix.msc2762.send.event:org.matrix.msc4310.rtc.decline");

        // Download avatars
        cap_assert("org.matrix.msc4039.download_file");
    }
}
