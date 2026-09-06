//! The routes of the [server-server API], including the [server discovery]
//! endpoints.
//!
//! [server-server API]: https://spec.matrix.org/v1.19/server-server-api/
//! [server discovery]: https://spec.matrix.org/v1.19/server-server-api/#server-discovery

use crate::endpoint::Api;

endpoints! {
    api: Api::Federation,

    "federation::authenticated_media::get_content::v1" =>
        ruma::api::federation::authenticated_media::get_content::v1::Request,
    "federation::authenticated_media::get_content_thumbnail::v1" =>
        ruma::api::federation::authenticated_media::get_content_thumbnail::v1::Request,
    "federation::authorization::get_event_authorization::v1" =>
        ruma::api::federation::authorization::get_event_authorization::v1::Request,
    "federation::backfill::get_backfill::v1" =>
        ruma::api::federation::backfill::get_backfill::v1::Request,
    "federation::device::get_devices::v1" =>
        ruma::api::federation::device::get_devices::v1::Request,
    "federation::directory::get_public_rooms::v1" =>
        ruma::api::federation::directory::get_public_rooms::v1::Request,
    "federation::directory::get_public_rooms_filtered::v1" =>
        ruma::api::federation::directory::get_public_rooms_filtered::v1::Request,
    "federation::discovery::discover_homeserver" =>
        ruma::api::federation::discovery::discover_homeserver::Request,
    "federation::discovery::get_remote_server_keys::v2" =>
        ruma::api::federation::discovery::get_remote_server_keys::v2::Request,
    "federation::discovery::get_remote_server_keys_batch::v2" =>
        ruma::api::federation::discovery::get_remote_server_keys_batch::v2::Request,
    "federation::discovery::get_server_keys::v2" =>
        ruma::api::federation::discovery::get_server_keys::v2::Request,
    "federation::discovery::get_server_version::v1" =>
        ruma::api::federation::discovery::get_server_version::v1::Request,
    #[cfg(feature = "unstable-msc3723")]
    "federation::discovery::get_server_versions::msc3723" =>
        ruma::api::federation::discovery::get_server_versions::msc3723::Request,
    "federation::event::get_event::v1" => ruma::api::federation::event::get_event::v1::Request,
    "federation::event::get_event_by_timestamp::v1" =>
        ruma::api::federation::event::get_event_by_timestamp::v1::Request,
    "federation::event::get_missing_events::v1" =>
        ruma::api::federation::event::get_missing_events::v1::Request,
    "federation::event::get_room_state::v1" =>
        ruma::api::federation::event::get_room_state::v1::Request,
    "federation::event::get_room_state_ids::v1" =>
        ruma::api::federation::event::get_room_state_ids::v1::Request,
    "federation::keys::claim_keys::v1" => ruma::api::federation::keys::claim_keys::v1::Request,
    "federation::keys::get_keys::v1" => ruma::api::federation::keys::get_keys::v1::Request,
    "federation::membership::create_invite::v1" =>
        ruma::api::federation::membership::create_invite::v1::Request,
    "federation::membership::create_invite::v2" =>
        ruma::api::federation::membership::create_invite::v2::Request,
    "federation::membership::create_join_event::v2" =>
        ruma::api::federation::membership::create_join_event::v2::Request,
    "federation::membership::create_knock_event::v1" =>
        ruma::api::federation::membership::create_knock_event::v1::Request,
    "federation::membership::create_leave_event::v2" =>
        ruma::api::federation::membership::create_leave_event::v2::Request,
    "federation::membership::prepare_join_event::v1" =>
        ruma::api::federation::membership::prepare_join_event::v1::Request,
    "federation::membership::prepare_knock_event::v1" =>
        ruma::api::federation::membership::prepare_knock_event::v1::Request,
    "federation::membership::prepare_leave_event::v1" =>
        ruma::api::federation::membership::prepare_leave_event::v1::Request,
    "federation::openid::get_openid_userinfo::v1" =>
        ruma::api::federation::openid::get_openid_userinfo::v1::Request,
    "federation::policy::sign_event::v1" => ruma::api::federation::policy::sign_event::v1::Request,
    "federation::query::get_custom_information::v1" =>
        ruma::api::federation::query::get_custom_information::v1::Request,
    #[cfg(feature = "unstable-msc4495")]
    "federation::query::get_presence_recipients::msc4495" =>
        ruma::api::federation::query::get_presence_recipients::msc4495::Request,
    "federation::query::get_profile_information::v1" =>
        ruma::api::federation::query::get_profile_information::v1::Request,
    "federation::query::get_room_information::v1" =>
        ruma::api::federation::query::get_room_information::v1::Request,
    #[cfg(feature = "unstable-msc3843")]
    "federation::room::report_content::msc3843" =>
        ruma::api::federation::room::report_content::msc3843::Request,
    "federation::space::get_hierarchy::v1" =>
        ruma::api::federation::space::get_hierarchy::v1::Request,
    "federation::thirdparty::bind_callback::v1" =>
        ruma::api::federation::thirdparty::bind_callback::v1::Request,
    "federation::thirdparty::exchange_invite::v1" =>
        ruma::api::federation::thirdparty::exchange_invite::v1::Request,
    "federation::transactions::send_transaction_message::v1" =>
        ruma::api::federation::transactions::send_transaction_message::v1::Request,
}
