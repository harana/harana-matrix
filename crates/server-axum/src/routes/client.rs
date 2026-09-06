//! The routes of the [client-server API].
//!
//! [client-server API]: https://spec.matrix.org/v1.19/client-server-api/
//!
//! Endpoints the specification deprecated, like the unauthenticated media ones,
//! are still served: clients that predate their replacement keep using them.
#![allow(deprecated)]

use crate::endpoint::Api;

endpoints! {
    api: Api::ClientServer,

    "client::account::add_3pid::v3" => ruma::api::client::account::add_3pid::v3::Request,
    "client::account::bind_3pid::v3" => ruma::api::client::account::bind_3pid::v3::Request,
    "client::account::change_password::v3" =>
        ruma::api::client::account::change_password::v3::Request,
    "client::account::check_registration_token_validity::v1" =>
        ruma::api::client::account::check_registration_token_validity::v1::Request,
    "client::account::deactivate::v3" => ruma::api::client::account::deactivate::v3::Request,
    "client::account::delete_3pid::v3" => ruma::api::client::account::delete_3pid::v3::Request,
    "client::account::get_3pids::v3" => ruma::api::client::account::get_3pids::v3::Request,
    "client::account::get_username_availability::v3" =>
        ruma::api::client::account::get_username_availability::v3::Request,
    "client::account::register::v3" => ruma::api::client::account::register::v3::Request,
    "client::account::request_3pid_management_token_via_email::v3" =>
        ruma::api::client::account::request_3pid_management_token_via_email::v3::Request,
    "client::account::request_3pid_management_token_via_msisdn::v3" =>
        ruma::api::client::account::request_3pid_management_token_via_msisdn::v3::Request,
    "client::account::request_openid_token::v3" =>
        ruma::api::client::account::request_openid_token::v3::Request,
    "client::account::request_password_change_token_via_email::v3" =>
        ruma::api::client::account::request_password_change_token_via_email::v3::Request,
    "client::account::request_password_change_token_via_msisdn::v3" =>
        ruma::api::client::account::request_password_change_token_via_msisdn::v3::Request,
    "client::account::request_registration_token_via_email::v3" =>
        ruma::api::client::account::request_registration_token_via_email::v3::Request,
    "client::account::request_registration_token_via_msisdn::v3" =>
        ruma::api::client::account::request_registration_token_via_msisdn::v3::Request,
    "client::account::unbind_3pid::v3" => ruma::api::client::account::unbind_3pid::v3::Request,
    "client::account::whoami::v3" => ruma::api::client::account::whoami::v3::Request,
    "client::admin::get_user_info::v3" => ruma::api::client::admin::get_user_info::v3::Request,
    "client::admin::is_user_locked::v1" => ruma::api::client::admin::is_user_locked::v1::Request,
    "client::admin::is_user_suspended::v1" =>
        ruma::api::client::admin::is_user_suspended::v1::Request,
    "client::admin::lock_user::v1" => ruma::api::client::admin::lock_user::v1::Request,
    "client::admin::suspend_user::v1" => ruma::api::client::admin::suspend_user::v1::Request,
    "client::alias::create_alias::v3" => ruma::api::client::alias::create_alias::v3::Request,
    "client::alias::delete_alias::v3" => ruma::api::client::alias::delete_alias::v3::Request,
    "client::alias::get_alias::v3" => ruma::api::client::alias::get_alias::v3::Request,
    "client::appservice::request_ping::v1" =>
        ruma::api::client::appservice::request_ping::v1::Request,
    "client::appservice::set_room_visibility::v3" =>
        ruma::api::client::appservice::set_room_visibility::v3::Request,
    "client::authenticated_media::get_content::v1" =>
        ruma::api::client::authenticated_media::get_content::v1::Request,
    "client::authenticated_media::get_content_as_filename::v1" =>
        ruma::api::client::authenticated_media::get_content_as_filename::v1::Request,
    "client::authenticated_media::get_content_thumbnail::v1" =>
        ruma::api::client::authenticated_media::get_content_thumbnail::v1::Request,
    "client::authenticated_media::get_media_config::v1" =>
        ruma::api::client::authenticated_media::get_media_config::v1::Request,
    "client::authenticated_media::get_media_preview::v1" =>
        ruma::api::client::authenticated_media::get_media_preview::v1::Request,
    "client::backup::add_backup_keys::v3" =>
        ruma::api::client::backup::add_backup_keys::v3::Request,
    "client::backup::add_backup_keys_for_room::v3" =>
        ruma::api::client::backup::add_backup_keys_for_room::v3::Request,
    "client::backup::add_backup_keys_for_session::v3" =>
        ruma::api::client::backup::add_backup_keys_for_session::v3::Request,
    "client::backup::create_backup_version::v3" =>
        ruma::api::client::backup::create_backup_version::v3::Request,
    "client::backup::delete_backup_keys::v3" =>
        ruma::api::client::backup::delete_backup_keys::v3::Request,
    "client::backup::delete_backup_keys_for_room::v3" =>
        ruma::api::client::backup::delete_backup_keys_for_room::v3::Request,
    "client::backup::delete_backup_keys_for_session::v3" =>
        ruma::api::client::backup::delete_backup_keys_for_session::v3::Request,
    "client::backup::delete_backup_version::v3" =>
        ruma::api::client::backup::delete_backup_version::v3::Request,
    "client::backup::get_backup_info::v3" =>
        ruma::api::client::backup::get_backup_info::v3::Request,
    "client::backup::get_backup_keys::v3" =>
        ruma::api::client::backup::get_backup_keys::v3::Request,
    "client::backup::get_backup_keys_for_room::v3" =>
        ruma::api::client::backup::get_backup_keys_for_room::v3::Request,
    "client::backup::get_backup_keys_for_session::v3" =>
        ruma::api::client::backup::get_backup_keys_for_session::v3::Request,
    "client::backup::get_latest_backup_info::v3" =>
        ruma::api::client::backup::get_latest_backup_info::v3::Request,
    "client::backup::update_backup_version::v3" =>
        ruma::api::client::backup::update_backup_version::v3::Request,
    "client::config::get_global_account_data::v3" =>
        ruma::api::client::config::get_global_account_data::v3::Request,
    "client::config::get_room_account_data::v3" =>
        ruma::api::client::config::get_room_account_data::v3::Request,
    "client::config::set_global_account_data::v3" =>
        ruma::api::client::config::set_global_account_data::v3::Request,
    "client::config::set_room_account_data::v3" =>
        ruma::api::client::config::set_room_account_data::v3::Request,
    "client::context::get_context::v3" => ruma::api::client::context::get_context::v3::Request,
    #[cfg(feature = "unstable-msc3814")]
    "client::dehydrated_device::delete_dehydrated_device::unstable" =>
        ruma::api::client::dehydrated_device::delete_dehydrated_device::unstable::Request,
    #[cfg(feature = "unstable-msc3814")]
    "client::dehydrated_device::get_dehydrated_device::unstable" =>
        ruma::api::client::dehydrated_device::get_dehydrated_device::unstable::Request,
    #[cfg(feature = "unstable-msc3814")]
    "client::dehydrated_device::get_events::unstable" =>
        ruma::api::client::dehydrated_device::get_events::unstable::Request,
    #[cfg(feature = "unstable-msc3814")]
    "client::dehydrated_device::put_dehydrated_device::unstable" =>
        ruma::api::client::dehydrated_device::put_dehydrated_device::unstable::Request,
    #[cfg(feature = "unstable-msc4140")]
    "client::delayed_events::delayed_message_event::unstable" =>
        ruma::api::client::delayed_events::delayed_message_event::unstable::Request,
    #[cfg(feature = "unstable-msc4140")]
    "client::delayed_events::delayed_state_event::unstable" =>
        ruma::api::client::delayed_events::delayed_state_event::unstable::Request,
    #[cfg(feature = "unstable-msc4140")]
    "client::delayed_events::get_all_delayed_events::unstable" =>
        ruma::api::client::delayed_events::get_all_delayed_events::unstable::Request,
    #[cfg(feature = "unstable-msc4140")]
    "client::delayed_events::get_delayed_event::unstable" =>
        ruma::api::client::delayed_events::get_delayed_event::unstable::Request,
    #[cfg(feature = "unstable-msc4140")]
    "client::delayed_events::send_delayed_event::unstable" =>
        ruma::api::client::delayed_events::send_delayed_event::unstable::Request,
    #[cfg(feature = "unstable-msc4140")]
    "client::delayed_events::update_delayed_event::unstable_v1" =>
        ruma::api::client::delayed_events::update_delayed_event::unstable_v1::Request,
    #[cfg(feature = "unstable-msc4140")]
    "client::delayed_events::update_delayed_event::unstable_v2" =>
        ruma::api::client::delayed_events::update_delayed_event::unstable_v2::Request,
    "client::device::delete_device::v3" => ruma::api::client::device::delete_device::v3::Request,
    "client::device::delete_devices::v3" => ruma::api::client::device::delete_devices::v3::Request,
    "client::device::get_device::v3" => ruma::api::client::device::get_device::v3::Request,
    "client::device::get_devices::v3" => ruma::api::client::device::get_devices::v3::Request,
    "client::device::update_device::v3" => ruma::api::client::device::update_device::v3::Request,
    "client::directory::get_public_rooms::v3" =>
        ruma::api::client::directory::get_public_rooms::v3::Request,
    "client::directory::get_public_rooms_filtered::v3" =>
        ruma::api::client::directory::get_public_rooms_filtered::v3::Request,
    "client::directory::get_room_visibility::v3" =>
        ruma::api::client::directory::get_room_visibility::v3::Request,
    "client::directory::set_room_visibility::v3" =>
        ruma::api::client::directory::set_room_visibility::v3::Request,
    "client::discovery::discover_homeserver" =>
        ruma::api::client::discovery::discover_homeserver::Request,
    "client::discovery::discover_policy_server" =>
        ruma::api::client::discovery::discover_policy_server::Request,
    "client::discovery::discover_support" =>
        ruma::api::client::discovery::discover_support::Request,
    "client::discovery::get_authorization_server_metadata::v1" =>
        ruma::api::client::discovery::get_authorization_server_metadata::v1::Request,
    "client::discovery::get_capabilities::v3" =>
        ruma::api::client::discovery::get_capabilities::v3::Request,
    "client::discovery::get_supported_versions" =>
        ruma::api::client::discovery::get_supported_versions::Request,
    "client::filter::create_filter::v3" => ruma::api::client::filter::create_filter::v3::Request,
    "client::filter::get_filter::v3" => ruma::api::client::filter::get_filter::v3::Request,
    "client::keys::claim_keys::v3" => ruma::api::client::keys::claim_keys::v3::Request,
    #[cfg(feature = "unstable-msc3983")]
    "client::keys::claim_keys::v4" => ruma::api::client::keys::claim_keys::v4::Request,
    "client::keys::get_key_changes::v3" => ruma::api::client::keys::get_key_changes::v3::Request,
    "client::keys::get_keys::v3" => ruma::api::client::keys::get_keys::v3::Request,
    "client::keys::upload_keys::v3" => ruma::api::client::keys::upload_keys::v3::Request,
    "client::keys::upload_signatures::v3" =>
        ruma::api::client::keys::upload_signatures::v3::Request,
    "client::keys::upload_signing_keys::v3" =>
        ruma::api::client::keys::upload_signing_keys::v3::Request,
    "client::knock::knock_room::v3" => ruma::api::client::knock::knock_room::v3::Request,
    "client::media::create_content::v3" => ruma::api::client::media::create_content::v3::Request,
    "client::media::create_content_async::v3" =>
        ruma::api::client::media::create_content_async::v3::Request,
    "client::media::create_mxc_uri::v1" => ruma::api::client::media::create_mxc_uri::v1::Request,
    "client::media::get_content::v3" => ruma::api::client::media::get_content::v3::Request,
    "client::media::get_content_as_filename::v3" =>
        ruma::api::client::media::get_content_as_filename::v3::Request,
    "client::media::get_content_thumbnail::v3" =>
        ruma::api::client::media::get_content_thumbnail::v3::Request,
    "client::media::get_media_config::v3" =>
        ruma::api::client::media::get_media_config::v3::Request,
    "client::media::get_media_preview::v3" =>
        ruma::api::client::media::get_media_preview::v3::Request,
    "client::membership::ban_user::v3" => ruma::api::client::membership::ban_user::v3::Request,
    "client::membership::forget_room::v3" =>
        ruma::api::client::membership::forget_room::v3::Request,
    "client::membership::get_member_events::v3" =>
        ruma::api::client::membership::get_member_events::v3::Request,
    "client::membership::invite_user::v3" =>
        ruma::api::client::membership::invite_user::v3::Request,
    "client::membership::join_room_by_id::v3" =>
        ruma::api::client::membership::join_room_by_id::v3::Request,
    "client::membership::join_room_by_id_or_alias::v3" =>
        ruma::api::client::membership::join_room_by_id_or_alias::v3::Request,
    "client::membership::joined_members::v3" =>
        ruma::api::client::membership::joined_members::v3::Request,
    "client::membership::joined_rooms::v3" =>
        ruma::api::client::membership::joined_rooms::v3::Request,
    "client::membership::kick_user::v3" => ruma::api::client::membership::kick_user::v3::Request,
    "client::membership::leave_room::v3" => ruma::api::client::membership::leave_room::v3::Request,
    "client::membership::mutual_rooms::unstable" =>
        ruma::api::client::membership::mutual_rooms::unstable::Request,
    "client::membership::mutual_rooms::v1" =>
        ruma::api::client::membership::mutual_rooms::v1::Request,
    "client::membership::unban_user::v3" => ruma::api::client::membership::unban_user::v3::Request,
    "client::message::get_message_events::v3" =>
        ruma::api::client::message::get_message_events::v3::Request,
    "client::message::send_message_event::v3" =>
        ruma::api::client::message::send_message_event::v3::Request,
    "client::peeking::get_current_state::v3" =>
        ruma::api::client::peeking::get_current_state::v3::Request,
    "client::peeking::listen_to_new_events::v3" =>
        ruma::api::client::peeking::listen_to_new_events::v3::Request,
    "client::presence::get_presence::v3" => ruma::api::client::presence::get_presence::v3::Request,
    "client::presence::set_presence::v3" => ruma::api::client::presence::set_presence::v3::Request,
    "client::profile::delete_profile_field::v3" =>
        ruma::api::client::profile::delete_profile_field::v3::Request,
    "client::profile::get_avatar_url::v3" =>
        ruma::api::client::profile::get_avatar_url::v3::Request,
    "client::profile::get_display_name::v3" =>
        ruma::api::client::profile::get_display_name::v3::Request,
    "client::profile::get_profile::v3" => ruma::api::client::profile::get_profile::v3::Request,
    "client::profile::get_profile_field::v3" =>
        ruma::api::client::profile::get_profile_field::v3::Request,
    "client::profile::set_avatar_url::v3" =>
        ruma::api::client::profile::set_avatar_url::v3::Request,
    "client::profile::set_display_name::v3" =>
        ruma::api::client::profile::set_display_name::v3::Request,
    "client::profile::set_profile_field::v3" =>
        ruma::api::client::profile::set_profile_field::v3::Request,
    "client::push::delete_pushrule::v3" => ruma::api::client::push::delete_pushrule::v3::Request,
    "client::push::get_notifications::v3" =>
        ruma::api::client::push::get_notifications::v3::Request,
    "client::push::get_pushers::v3" => ruma::api::client::push::get_pushers::v3::Request,
    "client::push::get_pushrule::v3" => ruma::api::client::push::get_pushrule::v3::Request,
    "client::push::get_pushrule_actions::v3" =>
        ruma::api::client::push::get_pushrule_actions::v3::Request,
    "client::push::get_pushrule_enabled::v3" =>
        ruma::api::client::push::get_pushrule_enabled::v3::Request,
    "client::push::get_pushrules_all::v3" =>
        ruma::api::client::push::get_pushrules_all::v3::Request,
    "client::push::get_pushrules_global_scope::v3" =>
        ruma::api::client::push::get_pushrules_global_scope::v3::Request,
    "client::push::set_pusher::v3" => ruma::api::client::push::set_pusher::v3::Request,
    "client::push::set_pushrule::v3" => ruma::api::client::push::set_pushrule::v3::Request,
    "client::push::set_pushrule_actions::v3" =>
        ruma::api::client::push::set_pushrule_actions::v3::Request,
    "client::push::set_pushrule_enabled::v3" =>
        ruma::api::client::push::set_pushrule_enabled::v3::Request,
    "client::read_marker::set_read_marker::v3" =>
        ruma::api::client::read_marker::set_read_marker::v3::Request,
    "client::receipt::create_receipt::v3" =>
        ruma::api::client::receipt::create_receipt::v3::Request,
    "client::redact::redact_event::v3" => ruma::api::client::redact::redact_event::v3::Request,
    "client::relations::get_relating_events::v1" =>
        ruma::api::client::relations::get_relating_events::v1::Request,
    "client::relations::get_relating_events_with_rel_type::v1" =>
        ruma::api::client::relations::get_relating_events_with_rel_type::v1::Request,
    "client::relations::get_relating_events_with_rel_type_and_event_type::v1" =>
        ruma::api::client::relations::get_relating_events_with_rel_type_and_event_type::v1::Request,
    #[cfg(any(feature = "unstable-msc4108", feature = "unstable-msc4388"))]
    #[cfg(feature = "unstable-msc4108")]
    "client::rendezvous::create_rendezvous_session::unstable_msc4108" =>
        ruma::api::client::rendezvous::create_rendezvous_session::unstable_msc4108::Request,
    #[cfg(any(feature = "unstable-msc4108", feature = "unstable-msc4388"))]
    #[cfg(feature = "unstable-msc4388")]
    "client::rendezvous::create_rendezvous_session::unstable_msc4388" =>
        ruma::api::client::rendezvous::create_rendezvous_session::unstable_msc4388::Request,
    #[cfg(any(feature = "unstable-msc4108", feature = "unstable-msc4388"))]
    #[cfg(feature = "unstable-msc4388")]
    "client::rendezvous::delete_rendezvous_session::unstable" =>
        ruma::api::client::rendezvous::delete_rendezvous_session::unstable::Request,
    #[cfg(any(feature = "unstable-msc4108", feature = "unstable-msc4388"))]
    #[cfg(feature = "unstable-msc4388")]
    "client::rendezvous::discover_rendezvous::unstable" =>
        ruma::api::client::rendezvous::discover_rendezvous::unstable::Request,
    #[cfg(any(feature = "unstable-msc4108", feature = "unstable-msc4388"))]
    #[cfg(feature = "unstable-msc4388")]
    "client::rendezvous::get_rendezvous_session::unstable" =>
        ruma::api::client::rendezvous::get_rendezvous_session::unstable::Request,
    #[cfg(any(feature = "unstable-msc4108", feature = "unstable-msc4388"))]
    #[cfg(feature = "unstable-msc4388")]
    "client::rendezvous::update_rendezvous_session::unstable" =>
        ruma::api::client::rendezvous::update_rendezvous_session::unstable::Request,
    "client::reporting::report_user::v3" => ruma::api::client::reporting::report_user::v3::Request,
    #[cfg(feature = "unstable-msc1763")]
    "client::retention::get_retention_configuration::unstable" =>
        ruma::api::client::retention::get_retention_configuration::unstable::Request,
    "client::room::aliases::v3" => ruma::api::client::room::aliases::v3::Request,
    "client::room::create_room::v3" => ruma::api::client::room::create_room::v3::Request,
    "client::room::get_event_by_timestamp::v1" =>
        ruma::api::client::room::get_event_by_timestamp::v1::Request,
    "client::room::get_room_event::v3" => ruma::api::client::room::get_room_event::v3::Request,
    "client::room::get_summary::v1" => ruma::api::client::room::get_summary::v1::Request,
    "client::room::report_content::v3" => ruma::api::client::room::report_content::v3::Request,
    "client::room::report_room::v3" => ruma::api::client::room::report_room::v3::Request,
    "client::room::upgrade_room::v3" => ruma::api::client::room::upgrade_room::v3::Request,
    #[cfg(feature = "unstable-msc4143")]
    "client::rtc::transports::v1" => ruma::api::client::rtc::transports::v1::Request,
    "client::search::search_events::v3" => ruma::api::client::search::search_events::v3::Request,
    "client::session::get_login_token::v1" =>
        ruma::api::client::session::get_login_token::v1::Request,
    "client::session::get_login_types::v3" =>
        ruma::api::client::session::get_login_types::v3::Request,
    "client::session::login::v3" => ruma::api::client::session::login::v3::Request,
    "client::session::login_fallback" => ruma::api::client::session::login_fallback::Request,
    "client::session::logout::v3" => ruma::api::client::session::logout::v3::Request,
    "client::session::logout_all::v3" => ruma::api::client::session::logout_all::v3::Request,
    "client::session::refresh_token::v3" => ruma::api::client::session::refresh_token::v3::Request,
    "client::session::sso_login::v3" => ruma::api::client::session::sso_login::v3::Request,
    "client::session::sso_login_with_provider::v3" =>
        ruma::api::client::session::sso_login_with_provider::v3::Request,
    "client::space::get_hierarchy::v1" => ruma::api::client::space::get_hierarchy::v1::Request,
    "client::state::get_state_event_for_key::v3" =>
        ruma::api::client::state::get_state_event_for_key::v3::Request,
    "client::state::get_state_events::v3" =>
        ruma::api::client::state::get_state_events::v3::Request,
    "client::state::send_state_event::v3" =>
        ruma::api::client::state::send_state_event::v3::Request,
    "client::sync::sync_events::v3" => ruma::api::client::sync::sync_events::v3::Request,
    #[cfg(feature = "unstable-msc4186")]
    "client::sync::sync_events::v5" => ruma::api::client::sync::sync_events::v5::Request,
    "client::tag::create_tag::v3" => ruma::api::client::tag::create_tag::v3::Request,
    "client::tag::delete_tag::v3" => ruma::api::client::tag::delete_tag::v3::Request,
    "client::tag::get_tags::v3" => ruma::api::client::tag::get_tags::v3::Request,
    "client::thirdparty::get_location_for_protocol::v3" =>
        ruma::api::client::thirdparty::get_location_for_protocol::v3::Request,
    "client::thirdparty::get_location_for_room_alias::v3" =>
        ruma::api::client::thirdparty::get_location_for_room_alias::v3::Request,
    "client::thirdparty::get_protocol::v3" =>
        ruma::api::client::thirdparty::get_protocol::v3::Request,
    "client::thirdparty::get_protocols::v3" =>
        ruma::api::client::thirdparty::get_protocols::v3::Request,
    "client::thirdparty::get_user_for_protocol::v3" =>
        ruma::api::client::thirdparty::get_user_for_protocol::v3::Request,
    "client::thirdparty::get_user_for_user_id::v3" =>
        ruma::api::client::thirdparty::get_user_for_user_id::v3::Request,
    #[cfg(feature = "unstable-msc4306")]
    "client::threads::get_thread_subscription::unstable" =>
        ruma::api::client::threads::get_thread_subscription::unstable::Request,
    #[cfg(feature = "unstable-msc4308")]
    "client::threads::get_thread_subscriptions_changes::unstable" =>
        ruma::api::client::threads::get_thread_subscriptions_changes::unstable::Request,
    "client::threads::get_threads::v1" => ruma::api::client::threads::get_threads::v1::Request,
    #[cfg(feature = "unstable-msc4306")]
    "client::threads::subscribe_thread::unstable" =>
        ruma::api::client::threads::subscribe_thread::unstable::Request,
    #[cfg(feature = "unstable-msc4306")]
    "client::threads::unsubscribe_thread::unstable" =>
        ruma::api::client::threads::unsubscribe_thread::unstable::Request,
    "client::to_device::send_event_to_device::v3" =>
        ruma::api::client::to_device::send_event_to_device::v3::Request,
    "client::typing::create_typing_event::v3" =>
        ruma::api::client::typing::create_typing_event::v3::Request,
    "client::uiaa::get_uiaa_fallback_page::v3" =>
        ruma::api::client::uiaa::get_uiaa_fallback_page::v3::Request,
    "client::user_directory::search_users::v3" =>
        ruma::api::client::user_directory::search_users::v3::Request,
    "client::voip::get_turn_server_info::v3" =>
        ruma::api::client::voip::get_turn_server_info::v3::Request,
}
