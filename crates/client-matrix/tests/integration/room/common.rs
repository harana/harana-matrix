use std::{collections::BTreeMap, iter, ops::Not, time::Duration};

use assert_matches2::{assert_let, assert_matches};
use futures_util::{FutureExt, StreamExt, pin_mut};
use js_int::uint;
use client_matrix::{
    RoomDisplayName, RoomMemberships,
    config::{SyncSettings, SyncToken},
    room::{RoomMember, RoomMemberSortOrder},
    test_utils::mocks::{AnyRoomBuilder, MatrixMockServer},
};
use client_base::{DmRoomDefinition, RoomMembersUpdate};
use common_test::{
    BOB, DEFAULT_TEST_ROOM_ID, JoinedRoomBuilder, LeftRoomBuilder, SyncResponseBuilder, async_test,
    bulk_room_members, event_factory::EventFactory, sync_state_event, test_json,
};
use harana_matrix_common::{
    RoomVersionId, event_id,
    events::{
        AnyGlobalAccountDataEvent, AnySyncStateEvent, AnySyncTimelineEvent, StateEventType,
        direct::DirectUserIdentifier,
        room::{
            avatar,
            member::{MembershipState, RoomMemberEvent},
            message::RoomMessageEventContent,
        },
    },
    int, mxc_uri, owned_room_alias_id, room_id, room_version_id,
    serde::Raw,
    user_id,
};
use serde_json::json;
use stream_assert::assert_pending;
use tokio::{task::spawn, time::sleep};
use wiremock::{
    Mock, ResponseTemplate,
    matchers::{body_json, header, method, path, path_regex},
};

use crate::{logged_in_client_with_server, mock_sync};

#[async_test]
async fn test_user_presence() {
    let (client, server) = logged_in_client_with_server().await;

    mock_sync(&server, &*test_json::SYNC, None).await;

    Mock::given(method("GET"))
        .and(path_regex(r"^/_matrix/client/r0/rooms/.*/members"))
        .and(header("authorization", "Bearer 1234"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&*test_json::MEMBERS))
        .mount(&server)
        .await;

    let sync_settings = SyncSettings::new().timeout(Duration::from_millis(3000));

    let _response = client.sync_once(sync_settings).await.unwrap();

    let room = client.get_room(&DEFAULT_TEST_ROOM_ID).unwrap();
    let members: Vec<RoomMember> = room.members(RoomMemberships::ACTIVE).await.unwrap();

    assert_eq!(2, members.len());
    // assert!(room.power_levels.is_some())
}

#[async_test]
async fn test_banned_member_has_no_profile() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    let room_id = room_id!("!a:b.c");
    let admin = user_id!("@admin:localhost");
    let banned = user_id!("@banned:localhost");
    let f = || EventFactory::new().room(room_id);

    let room = server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(
                    f().sender(banned)
                        .member(banned)
                        .display_name("Spammer")
                        .avatar_url(mxc_uri!("mxc://localhost/spammer")),
                )
                .add_state_event(f().sender(admin).member(admin).display_name("Admin")),
        )
        .await;

    // While they are joined, the member has the profile they set.
    let member = room.get_member_no_sync(banned).await.unwrap().expect("the member is known");
    assert_eq!(member.display_name(), Some("Spammer"));
    assert_eq!(member.avatar_url(), Some(mxc_uri!("mxc://localhost/spammer")));

    // The ban event carries the profile the member had. A banned member must not
    // be shown with it.
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_state_event(
                f().sender(admin)
                    .member(banned)
                    .banned(banned)
                    .display_name("Spammer")
                    .avatar_url(mxc_uri!("mxc://localhost/spammer"))
                    .reason("spam"),
            ),
        )
        .await;

    let member = room.get_member_no_sync(banned).await.unwrap().expect("the member is known");
    assert_eq!(member.membership(), &MembershipState::Ban);
    assert!(member.is_banned());
    assert_eq!(member.display_name(), None);
    assert_eq!(member.avatar_url(), None);
    assert_eq!(member.name(), "banned");
    // The raw event is still there for a caller that needs it.
    assert_eq!(member.event().displayname_value(), Some("Spammer"));

    // The other member is unaffected.
    let member = room.get_member_no_sync(admin).await.unwrap().expect("the member is known");
    assert_eq!(member.display_name(), Some("Admin"));
}

#[async_test]
async fn test_member_list_filters_sorts_and_paginates() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    let room_id = room_id!("!a:b.c");
    let admin = user_id!("@admin:localhost");
    let alice = user_id!("@alice:localhost");
    let bob = user_id!("@bob:localhost");
    let carol = user_id!("@carol:localhost");
    let dave = user_id!("@dave:localhost");
    let f = || EventFactory::new().room(room_id);

    let room = server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f().sender(admin).create(admin, RoomVersionId::V11))
                .add_state_event(f().sender(admin).member(admin).display_name("Zoe the admin"))
                .add_state_event(f().sender(alice).member(alice).display_name("alice"))
                .add_state_event(f().sender(bob).member(bob).display_name("Bob"))
                .add_state_event(
                    f().sender(admin).member(carol).invited(carol).display_name("Carol"),
                )
                .add_state_event(f().sender(admin).member(dave).banned(dave))
                .add_state_event(
                    f().sender(admin)
                        .power_levels(&mut BTreeMap::from([(admin.to_owned(), int!(100))])),
                ),
        )
        .await;

    // Sorted by name, case-insensitively, across every membership.
    let all = room.member_list().no_sync().all().await.unwrap();
    assert_eq!(
        all.iter().map(|member| member.name()).collect::<Vec<_>>(),
        vec!["alice", "Bob", "Carol", "dave", "Zoe the admin"]
    );

    // Filtered by membership.
    let joined =
        room.member_list().no_sync().memberships(RoomMemberships::JOIN).all().await.unwrap();
    assert_eq!(
        joined.iter().map(|member| member.name()).collect::<Vec<_>>(),
        vec!["alice", "Bob", "Zoe the admin"]
    );

    // Sorted by power level first: the admin comes before everyone else despite
    // their name sorting last.
    let by_power_level = room
        .member_list()
        .no_sync()
        .memberships(RoomMemberships::JOIN)
        .sort_by(RoomMemberSortOrder::PowerLevelThenName)
        .all()
        .await
        .unwrap();
    assert_eq!(
        by_power_level.iter().map(|member| member.name()).collect::<Vec<_>>(),
        vec!["Zoe the admin", "alice", "Bob"]
    );

    // Searching matches the display name or the user ID, case-insensitively.
    let searched = room.member_list().no_sync().search("BO").all().await.unwrap();
    assert_eq!(searched.iter().map(|member| member.name()).collect::<Vec<_>>(), vec!["Bob"]);
    let searched = room.member_list().no_sync().search("@carol").all().await.unwrap();
    assert_eq!(searched.iter().map(|member| member.name()).collect::<Vec<_>>(), vec!["Carol"]);
    // An empty search term is not a filter.
    assert_eq!(room.member_list().no_sync().search("   ").count().await.unwrap(), 5);

    // Filtering on power level is what showing the admins separately needs.
    let admins = room.member_list().no_sync().min_power_level(100).all().await.unwrap();
    assert_eq!(
        admins.iter().map(|member| member.name()).collect::<Vec<_>>(),
        vec!["Zoe the admin"]
    );

    // Pagination reports the total, so a view knows how far it can scroll.
    let page = room.member_list().no_sync().page(1, 2).await.unwrap();
    assert_eq!(page.total, 5);
    assert_eq!(
        page.members.iter().map(|member| member.name()).collect::<Vec<_>>(),
        vec!["Bob", "Carol"]
    );

    // Scrolling past the end is an empty page, not an error.
    let page = room.member_list().no_sync().page(50, 2).await.unwrap();
    assert_eq!(page.total, 5);
    assert!(page.members.is_empty());
}

#[async_test]
async fn test_subscribe_to_member_updates() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    let room_id = room_id!("!a:b.c");
    let alice = user_id!("@alice:localhost");
    let f = || EventFactory::new().room(room_id);

    let room = server.sync_room(&client, JoinedRoomBuilder::new(room_id)).await;

    let updates = room.subscribe_to_member_updates();
    pin_mut!(updates);
    assert_pending!(updates);

    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f().sender(alice).member(alice).display_name("Alice")),
        )
        .await;

    assert_let!(
        Some(RoomMembersUpdate::Partial(user_ids)) = updates.next().now_or_never().flatten()
    );
    assert!(user_ids.contains(alice));
}

#[async_test]
async fn test_sync_members_fills_in_the_member_counts() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    let room_id = room_id!("!a:b.c");
    let alice = user_id!("@alice:localhost");
    let bob = user_id!("@bob:localhost");
    let carol = user_id!("@carol:localhost");
    let dave = user_id!("@dave:localhost");
    let f = || EventFactory::new().room(room_id);

    // A lazy-loading server may never send a room summary, leaving the counts at
    // zero even though the member events are known.
    let room = server.sync_room(&client, JoinedRoomBuilder::new(room_id)).await;
    assert_eq!(room.active_members_count(), 0);

    let members: Vec<Raw<RoomMemberEvent>> = vec![
        f().sender(alice).member(alice).into(),
        f().sender(bob).member(bob).into(),
        f().sender(alice).member(carol).invited(carol).into(),
        f().sender(alice).member(dave).banned(dave).into(),
    ];
    server.mock_get_members().ok(members).mock_once().mount().await;

    room.sync_members().await.unwrap();

    // The full member list is authoritative, so the counts are now exact.
    assert_eq!(room.joined_members_count(), 2);
    assert_eq!(room.invited_members_count(), 1);
    assert_eq!(room.active_members_count(), 3);
    assert_eq!(room.members(RoomMemberships::ACTIVE).await.unwrap().len(), 3);
}

#[async_test]
async fn test_get_members_does_not_overwrite_newer_sync_state() {
    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    let room_id = room_id!("!a:b.c");
    let alice = user_id!("@alice:localhost");
    let bob = user_id!("@bob:localhost");
    let f = || EventFactory::new().room(room_id);

    let room = server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id)
                .add_state_event(f().sender(alice).member(alice).display_name("Alice"))
                .add_state_event(f().sender(bob).member(bob).display_name("Bob")),
        )
        .await;

    // The `/members` response describes the room as it was when the request was
    // sent: Alice is still there. It is slow enough for a sync to land first.
    let alice_event: Raw<RoomMemberEvent> =
        f().sender(alice).member(alice).display_name("Alice").into();
    let bob_event: Raw<RoomMemberEvent> = f().sender(bob).member(bob).display_name("Bob").into();
    server
        .mock_get_members()
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({ "chunk": [alice_event, bob_event] }))
                .set_delay(Duration::from_millis(700)),
        )
        .mock_once()
        .mount()
        .await;

    let members = spawn({
        let room = room.clone();
        async move { room.sync_members().await }
    });

    // Let the request go out before the sync lands.
    sleep(Duration::from_millis(200)).await;

    // Alice leaves; this is fresher than what the `/members` request will answer.
    server
        .sync_room(
            &client,
            JoinedRoomBuilder::new(room_id).add_state_event(
                f().sender(alice).member(alice).membership(MembershipState::Leave),
            ),
        )
        .await;

    members.await.unwrap().unwrap();

    // The outdated `/members` response must not have brought Alice back.
    let alice_member =
        room.get_member_no_sync(alice).await.unwrap().expect("Alice is a known member");
    assert_eq!(alice_member.membership(), &MembershipState::Leave);

    // Everyone else is still updated from the response.
    let bob_member = room.get_member_no_sync(bob).await.unwrap().expect("Bob is a known member");
    assert_eq!(bob_member.membership(), &MembershipState::Join);
    assert_eq!(bob_member.display_name(), Some("Bob"));
}

#[async_test]
async fn test_calculate_room_names_from_summary() {
    let (client, server) = logged_in_client_with_server().await;

    mock_sync(&server, &*test_json::DEFAULT_SYNC_SUMMARY, None).await;

    let sync_settings = SyncSettings::new().timeout(Duration::from_millis(3000));
    let _response = client.sync_once(sync_settings).await.unwrap();
    let room = client.get_room(&DEFAULT_TEST_ROOM_ID).unwrap();

    assert_eq!(
        RoomDisplayName::Calculated("example2".to_owned()),
        room.display_name().await.unwrap()
    );
}

#[async_test]
async fn test_room_names() {
    let (client, server) = logged_in_client_with_server().await;
    let own_user_id = client.user_id().unwrap();

    // Room with a canonical alias.
    mock_sync(&server, &*test_json::SYNC, None).await;

    client.sync_once(SyncSettings::default().token(SyncToken::NoToken)).await.unwrap();
    server.reset().await;

    assert_eq!(client.rooms().len(), 1);
    let room = client.get_room(&DEFAULT_TEST_ROOM_ID).unwrap();

    assert_eq!(RoomDisplayName::Aliased("tutorial".to_owned()), room.display_name().await.unwrap());

    // Room with a name.
    mock_sync(&server, &*test_json::INVITE_SYNC, None).await;

    client.sync_once(SyncSettings::default().token(SyncToken::NoToken)).await.unwrap();
    server.reset().await;

    assert_eq!(client.rooms().len(), 2);
    let invited_room = client.get_room(room_id!("!696r7674:example.com")).unwrap();

    assert_eq!(
        RoomDisplayName::Named("My Room Name".to_owned()),
        invited_room.display_name().await.unwrap()
    );

    let mut sync_builder = SyncResponseBuilder::new();

    let own_left_member_event = sync_state_event!({
        "content": {
            "membership": "leave",
        },
        "event_id": "$747273582443PhrS9:localhost",
        "origin_server_ts": 1472735820,
        "sender": own_user_id,
        "state_key": own_user_id,
        "type": "m.room.member",
        "unsigned": {
            "age": 1234
        }
    });

    // Left room with a lot of members.
    let room_id = room_id!("!plenty_of_members:localhost");
    sync_builder.add_left_room(
        LeftRoomBuilder::new(room_id).add_state_bulk(
            bulk_room_members(0, 0..15, "localhost", &MembershipState::Join)
                .chain(iter::once(own_left_member_event.clone())),
        ),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;

    client.sync_once(SyncSettings::default().token(SyncToken::NoToken)).await.unwrap();
    server.reset().await;

    let room = client.get_room(room_id).unwrap();

    assert_eq!(
        RoomDisplayName::Calculated(
            "user_0, user_1, user_10, user_11, user_12, and 10 others".to_owned()
        ),
        room.display_name().await.unwrap()
    );

    // Room with joined and invited members.
    let room_id = room_id!("!joined_invited_members:localhost");
    sync_builder.add_left_room(LeftRoomBuilder::new(room_id).add_state_bulk([
        sync_state_event!({
            "content": {
                "membership": "join",
            },
            "event_id": "$example1_join",
            "origin_server_ts": 151800140,
            "sender": "@example1:localhost",
            "state_key": "@example1:localhost",
            "type": "m.room.member",
        }),
        sync_state_event!({
            "content": {
                "displayname": "Bob",
                "membership": "invite",
            },
            "event_id": "$bob_invite",
            "origin_server_ts": 151800140,
            "sender": "@example1:localhost",
            "state_key": "@bob:localhost",
            "type": "m.room.member",
        }),
        sync_state_event!({
            "content": {
                "membership": "leave",
            },
            "event_id": "$example3_leave",
            "origin_server_ts": 151800140,
            "sender": "@example3:localhost",
            "state_key": "@example3:localhost",
            "type": "m.room.member",
        }),
        own_left_member_event.clone(),
    ]));
    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;

    client.sync_once(SyncSettings::default().token(SyncToken::NoToken)).await.unwrap();
    server.reset().await;

    let room = client.get_room(room_id).unwrap();

    assert_eq!(
        RoomDisplayName::Calculated("Bob, example1".to_owned()),
        room.display_name().await.unwrap()
    );

    // Room with only left members.
    let room_id = room_id!("!left_members:localhost");
    sync_builder.add_left_room(
        LeftRoomBuilder::new(room_id).add_state_bulk(
            bulk_room_members(0, 0..3, "localhost", &MembershipState::Leave)
                .chain(iter::once(own_left_member_event)),
        ),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;

    client.sync_once(SyncSettings::default().token(SyncToken::NoToken)).await.unwrap();
    server.reset().await;

    let room = client.get_room(room_id).unwrap();

    assert_eq!(
        RoomDisplayName::EmptyWas("user_0, user_1, user_2".to_owned()),
        room.display_name().await.unwrap()
    );
}

#[async_test]
async fn test_state_event_getting() {
    let (client, server) = logged_in_client_with_server().await;

    let sync = json!({
        "next_batch": "1234",
        "rooms": {
            "join": {
                *DEFAULT_TEST_ROOM_ID: {
                    "state": {
                      "events": [
                        {
                          "type": "m.custom.note",
                          "sender": "@example:localhost",
                          "content": {
                            "body": "Note 1",
                          },
                          "state_key": "note.1",
                          "origin_server_ts": 1611853078727u64,
                          "unsigned": {
                            "replaces_state": "$2s9GcbVxbbFS3EZY9vN1zhavaDJnF32cAIGAxi99NuQ",
                            "age": 15458166523u64
                          },
                          "event_id": "$NVCTvrlxodf3ZGjJ6foxepEq8ysSkTq8wG0wKeQBVZg"
                        },
                        {
                          "type": "m.custom.note",
                          "sender": "@example2:localhost",
                          "content": {
                            "body": "Note 2",
                          },
                          "state_key": "note.2",
                          "origin_server_ts": 1611853078727u64,
                          "unsigned": {
                            "replaces_state": "$2s9GcbVxbbFS3EZY9vN1zhavaDJnF32cAIGAxi99NuQ",
                            "age": 15458166523u64
                          },
                          "event_id": "$NVCTvrlxodf3ZGjJ6foxepEq8ysSkTq8wG0wKeQBVZg"
                        },
                        {
                          "type": "m.room.encryption",
                          "sender": "@example:localhost",
                          "content": {
                            "algorithm": "m.megolm.v1.aes-sha2"
                          },
                          "state_key": "",
                          "origin_server_ts": 1586437448151u64,
                          "unsigned": {
                            "age": 40873797099u64
                          },
                          "event_id": "$vyG3wu1QdJSh5gc-09SwjXBXlXo8gS7s4QV_Yxha0Xw"
                        },
                      ]
                    }
                }
            }
        }
    });

    mock_sync(&server, sync, None).await;

    let room = client.get_room(&DEFAULT_TEST_ROOM_ID);
    assert!(room.is_none());

    client.sync_once(SyncSettings::default()).await.unwrap();

    let room = client.get_room(&DEFAULT_TEST_ROOM_ID).unwrap();

    let state_events = room.get_state_events(StateEventType::RoomEncryption).await.unwrap();
    assert_eq!(state_events.len(), 1);

    let state_events = room.get_state_events("m.custom.note".into()).await.unwrap();
    assert_eq!(state_events.len(), 2);

    let encryption_event = room
        .get_state_event(StateEventType::RoomEncryption, "")
        .await
        .unwrap()
        .unwrap()
        .deserialize()
        .unwrap();

    assert_matches::assert_matches!(
        encryption_event.as_sync(),
        Some(AnySyncStateEvent::RoomEncryption(_))
    );
}

#[async_test]
async fn test_room_route() {
    let (client, server) = logged_in_client_with_server().await;
    let mut sync_builder = SyncResponseBuilder::new();
    let room_id = &*DEFAULT_TEST_ROOM_ID;
    let f = EventFactory::new();

    // Without eligible server
    sync_builder.add_joined_room(
        JoinedRoomBuilder::new(room_id).add_timeline_bulk([
            f.create(user_id!("@creator:127.0.0.1"), room_version_id!("6"))
                .event_id(event_id!("$151957878228ekrDs"))
                .server_ts(15195787)
                .sender(user_id!("@creator:127.0.0.1"))
                .state_key("")
                .into_raw_sync(),
            f.member(user_id!("@creator:127.0.0.1"))
                .membership(MembershipState::Join)
                .event_id(event_id!("$151800140517rfvjc"))
                .server_ts(151800140)
                .sender(user_id!("@creator:127.0.0.1"))
                .state_key("@creator:127.0.0.1")
                .into_raw_sync(),
        ]),
    );

    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;
    let sync_token = client.sync_once(SyncSettings::new()).await.unwrap().next_batch;
    let room = client.get_room(room_id).unwrap();

    let route = room.route().await.unwrap();
    assert_eq!(route.len(), 0);

    // With a single eligible server
    let mut batch = 0;
    sync_builder.add_joined_room(JoinedRoomBuilder::new(room_id).add_timeline_state_bulk(
        bulk_room_members(batch, 0..1, "localhost", &MembershipState::Join),
    ));
    mock_sync(&server, sync_builder.build_json_sync_response(), Some(sync_token.clone())).await;
    let sync_token =
        client.sync_once(SyncSettings::new().token(sync_token)).await.unwrap().next_batch;

    let route = room.route().await.unwrap();
    assert_eq!(route.len(), 1);
    assert_eq!(route[0], "localhost");

    // With two eligible servers
    batch += 1;
    sync_builder.add_joined_room(JoinedRoomBuilder::new(room_id).add_timeline_state_bulk(
        bulk_room_members(batch, 0..15, "notarealhs", &MembershipState::Join),
    ));
    mock_sync(&server, sync_builder.build_json_sync_response(), Some(sync_token.clone())).await;
    let sync_token =
        client.sync_once(SyncSettings::new().token(sync_token)).await.unwrap().next_batch;

    let route = room.route().await.unwrap();
    assert_eq!(route.len(), 2);
    assert_eq!(route[0], "notarealhs");
    assert_eq!(route[1], "localhost");

    // With three eligible servers
    batch += 1;
    sync_builder.add_joined_room(JoinedRoomBuilder::new(room_id).add_timeline_state_bulk(
        bulk_room_members(batch, 0..5, "mymatrix", &MembershipState::Join),
    ));
    mock_sync(&server, sync_builder.build_json_sync_response(), Some(sync_token.clone())).await;
    let sync_token =
        client.sync_once(SyncSettings::new().token(sync_token)).await.unwrap().next_batch;

    let route = room.route().await.unwrap();
    assert_eq!(route.len(), 3);
    assert_eq!(route[0], "notarealhs");
    assert_eq!(route[1], "mymatrix");
    assert_eq!(route[2], "localhost");

    // With four eligible servers
    batch += 1;
    sync_builder.add_joined_room(JoinedRoomBuilder::new(room_id).add_timeline_state_bulk(
        bulk_room_members(batch, 0..10, "yourmatrix", &MembershipState::Join),
    ));
    mock_sync(&server, sync_builder.build_json_sync_response(), Some(sync_token.clone())).await;
    let sync_token =
        client.sync_once(SyncSettings::new().token(sync_token)).await.unwrap().next_batch;

    let route = room.route().await.unwrap();
    assert_eq!(route.len(), 3);
    assert_eq!(route[0], "notarealhs");
    assert_eq!(route[1], "yourmatrix");
    assert_eq!(route[2], "mymatrix");

    // With power levels
    let mut user_map = BTreeMap::from([(user_id!("@user_0:localhost").into(), 50.into())]);
    sync_builder.add_joined_room(
        JoinedRoomBuilder::new(room_id).add_timeline_event(
            f.power_levels(&mut user_map)
                .event_id(event_id!("$15139375512JaHAW"))
                .server_ts(151393755)
                .sender(user_id!("@creator:127.0.0.1"))
                .state_key("")
                .into_raw_sync(),
        ),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), Some(sync_token.clone())).await;
    let sync_token =
        client.sync_once(SyncSettings::new().token(sync_token)).await.unwrap().next_batch;

    let route = room.route().await.unwrap();
    assert_eq!(route.len(), 3);
    assert_eq!(route[0], "localhost");
    assert_eq!(route[1], "notarealhs");
    assert_eq!(route[2], "yourmatrix");

    // With higher power levels
    let mut user_map = BTreeMap::from([
        (user_id!("@user_0:localhost").into(), 50.into()),
        (user_id!("@user_2:mymatrix").into(), 70.into()),
    ]);
    sync_builder.add_joined_room(
        JoinedRoomBuilder::new(room_id).add_timeline_event(
            f.power_levels(&mut user_map)
                .event_id(event_id!("$15139375512JaHAW"))
                .server_ts(151393755)
                .sender(user_id!("@creator:127.0.0.1"))
                .state_key("")
                .into_raw_sync(),
        ),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), Some(sync_token.clone())).await;
    let sync_token =
        client.sync_once(SyncSettings::new().token(sync_token)).await.unwrap().next_batch;

    let route = room.route().await.unwrap();
    assert_eq!(route.len(), 3);
    assert_eq!(route[0], "mymatrix");
    assert_eq!(route[1], "notarealhs");
    assert_eq!(route[2], "yourmatrix");

    // With server ACLs
    sync_builder.add_joined_room(
        JoinedRoomBuilder::new(room_id).add_timeline_event(
            f.server_acl(true, vec!["*".to_owned()], vec!["notarealhs".to_owned()])
                .event_id(event_id!("$143273582443PhrSn"))
                .server_ts(1432735824)
                .sender(user_id!("@creator:127.0.0.1"))
                .state_key("")
                .into_raw_sync(),
        ),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), Some(sync_token.clone())).await;
    client.sync_once(SyncSettings::new().token(sync_token)).await.unwrap();

    let route = room.route().await.unwrap();
    assert_eq!(route.len(), 3);
    assert_eq!(route[0], "mymatrix");
    assert_eq!(route[1], "yourmatrix");
    assert_eq!(route[2], "localhost");
}

#[async_test]
async fn test_room_permalink() {
    let (client, server) = logged_in_client_with_server().await;
    let mut sync_builder = SyncResponseBuilder::new();
    let room_id = room_id!("!test_room:127.0.0.1");
    let f = EventFactory::new();

    // Without aliases
    sync_builder.add_joined_room(
        JoinedRoomBuilder::new(room_id)
            .add_timeline_state_bulk(bulk_room_members(
                0,
                0..1,
                "localhost",
                &MembershipState::Join,
            ))
            .add_timeline_state_bulk(bulk_room_members(
                1,
                0..5,
                "notarealhs",
                &MembershipState::Join,
            )),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;
    let sync_token = client.sync_once(SyncSettings::new()).await.unwrap().next_batch;
    let room = client.get_room(room_id).unwrap();

    assert_eq!(
        room.matrix_to_permalink().await.unwrap().to_string(),
        "https://matrix.to/#/!test_room:127.0.0.1?via=notarealhs&via=localhost"
    );
    assert_eq!(
        room.matrix_permalink(false).await.unwrap().to_string(),
        "matrix:roomid/test_room:127.0.0.1?via=notarealhs&via=localhost"
    );

    // With an alternative alias
    sync_builder.add_joined_room(
        JoinedRoomBuilder::new(room_id).add_timeline_event(
            f.canonical_alias(None, vec![owned_room_alias_id!("#alias:localhost")])
                .event_id(event_id!("$15139375513VdeRF"))
                .server_ts(151393755)
                .sender(user_id!("@user_0:localhost"))
                .state_key("")
                .into_raw_sync(),
        ),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), Some(sync_token.clone())).await;
    let sync_token =
        client.sync_once(SyncSettings::new().token(sync_token)).await.unwrap().next_batch;

    assert_eq!(
        room.matrix_to_permalink().await.unwrap().to_string(),
        "https://matrix.to/#/%23alias:localhost"
    );
    assert_eq!(room.matrix_permalink(false).await.unwrap().to_string(), "matrix:r/alias:localhost");

    // With a canonical alias
    sync_builder.add_joined_room(
        JoinedRoomBuilder::new(room_id).add_timeline_event(
            f.canonical_alias(
                Some(owned_room_alias_id!("#canonical:localhost")),
                vec![owned_room_alias_id!("#alias:localhost")],
            )
            .event_id(event_id!("$15139375513VdeRF"))
            .server_ts(151393755)
            .sender(user_id!("@user_0:localhost"))
            .state_key("")
            .into_raw_sync(),
        ),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), Some(sync_token.clone())).await;
    client.sync_once(SyncSettings::new().token(sync_token)).await.unwrap();

    assert_eq!(
        room.matrix_to_permalink().await.unwrap().to_string(),
        "https://matrix.to/#/%23canonical:localhost"
    );
    assert_eq!(
        room.matrix_permalink(false).await.unwrap().to_string(),
        "matrix:r/canonical:localhost"
    );
    assert_eq!(
        room.matrix_permalink(true).await.unwrap().to_string(),
        "matrix:r/canonical:localhost?action=join"
    );
}

#[async_test]
async fn test_room_event_permalink() {
    let (client, server) = logged_in_client_with_server().await;
    let mut sync_builder = SyncResponseBuilder::new();
    let room_id = room_id!("!test_room:127.0.0.1");
    let event_id = event_id!("$15139375512JaHAW");

    // Without aliases
    sync_builder.add_joined_room(
        JoinedRoomBuilder::new(room_id)
            .add_timeline_state_bulk(bulk_room_members(
                0,
                0..1,
                "localhost",
                &MembershipState::Join,
            ))
            .add_timeline_state_bulk(bulk_room_members(
                1,
                0..5,
                "notarealhs",
                &MembershipState::Join,
            )),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;
    let sync_token = client.sync_once(SyncSettings::new()).await.unwrap().next_batch;
    let room = client.get_room(room_id).unwrap();

    assert_eq!(
        room.matrix_to_event_permalink(event_id).await.unwrap().to_string(),
        "https://matrix.to/#/!test_room:127.0.0.1/$15139375512JaHAW?via=notarealhs&via=localhost"
    );
    assert_eq!(
        room.matrix_event_permalink(event_id).await.unwrap().to_string(),
        "matrix:roomid/test_room:127.0.0.1/e/15139375512JaHAW?via=notarealhs&via=localhost"
    );

    // Adding an alias doesn't change anything
    sync_builder.add_joined_room(
        JoinedRoomBuilder::new(room_id).add_timeline_event(
            EventFactory::new()
                .canonical_alias(
                    Some(owned_room_alias_id!("#canonical:localhost")),
                    vec![owned_room_alias_id!("#alias:localhost")],
                )
                .event_id(event_id!("$15139375513VdeRF"))
                .server_ts(151393755)
                .sender(user_id!("@user_0:localhost"))
                .state_key("")
                .into_raw_sync(),
        ),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), Some(sync_token.clone())).await;
    client.sync_once(SyncSettings::new().token(sync_token)).await.unwrap();

    assert_eq!(
        room.matrix_to_event_permalink(event_id).await.unwrap().to_string(),
        "https://matrix.to/#/!test_room:127.0.0.1/$15139375512JaHAW?via=notarealhs&via=localhost"
    );
    assert_eq!(
        room.matrix_event_permalink(event_id).await.unwrap().to_string(),
        "matrix:roomid/test_room:127.0.0.1/e/15139375512JaHAW?via=notarealhs&via=localhost"
    );
}

#[async_test]
async fn test_event() {
    let event_id = event_id!("$foun39djjod0f");

    let server = MatrixMockServer::new().await;
    let client = server.client_builder().build().await;

    let cache = client.event_cache();
    let _ = cache.subscribe();

    let f = EventFactory::new().sender(user_id!("@example:localhost"));
    let room = server
        .sync_room(
            &client,
            // We need the member event and power levels locally so the push rules processor
            // works.
            JoinedRoomBuilder::new(&DEFAULT_TEST_ROOM_ID)
                .add_state_event(f.member(user_id!("@example:localhost")).display_name("example"))
                .add_state_event(f.default_power_levels()),
        )
        .await;

    // First, loading the event results in a 404.
    room.load_or_fetch_event(event_id, None).await.unwrap_err();

    let f = EventFactory::new();
    server
        .mock_room_event()
        .ok(f
            .room_tombstone("This room has been replaced", room_id!("!newroom:localhost"))
            .sender(*BOB)
            .event_id(event_id)
            .room(*DEFAULT_TEST_ROOM_ID)
            .into())
        .named("/event")
        .mock_once()
        .mount()
        .await;

    // Then, we can find it using a network query.
    let timeline_event = room.load_or_fetch_event(event_id, None).await.unwrap();
    assert_let!(
        AnySyncTimelineEvent::State(AnySyncStateEvent::RoomTombstone(event)) =
            timeline_event.raw().deserialize().unwrap()
    );
    assert_eq!(event.event_id(), event_id);

    let push_actions = timeline_event.push_actions().unwrap();
    assert!(push_actions.iter().any(|a| a.is_highlight()));
    assert!(push_actions.iter().any(|a| a.should_notify()));
}

#[async_test]
async fn test_event_with_context() {
    let event_id = event_id!("$cur1234");
    let prev_event_id = event_id!("$prev1234");
    let next_event_id = event_id!("$next_1234");

    let (client, server) = logged_in_client_with_server().await;
    client.event_cache().subscribe().unwrap();

    let sync_settings = SyncSettings::new().timeout(Duration::from_millis(3000));

    let f = EventFactory::new().sender(user_id!("@example:localhost"));
    let mut sync_builder = SyncResponseBuilder::new();
    sync_builder
        // We need the member event and power levels locally so the push rules processor works.
        .add_joined_room(
            JoinedRoomBuilder::new(&DEFAULT_TEST_ROOM_ID)
                .add_state_event(f.member(user_id!("@example:localhost")).display_name("example"))
                .add_state_event(f.default_power_levels()),
        );

    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;
    let _response = client.sync_once(sync_settings.clone()).await.unwrap();
    server.reset().await;

    let room = client.get_room(&DEFAULT_TEST_ROOM_ID).unwrap();
    let room_id = room.room_id();

    let f = EventFactory::new().room(room_id).sender(*BOB);
    let event =
        f.event(RoomMessageEventContent::text_plain("The requested message")).event_id(event_id);
    let event_before =
        f.event(RoomMessageEventContent::text_plain("A previous message")).event_id(prev_event_id);
    let event_next =
        f.event(RoomMessageEventContent::text_plain("A newer message")).event_id(next_event_id);
    Mock::given(method("GET"))
        .and(path(format!("/_matrix/client/r0/rooms/{room_id}/context/{event_id}")))
        .and(header("authorization", "Bearer 1234"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "events_before": [event_before.into_raw_timeline()],
            "event": event.into_raw_timeline(),
            "events_after": [event_next.into_raw_timeline()],
            "state": [],
        })))
        .mount(&server)
        .await;

    let context_ret = room.event_with_context(event_id, false, uint!(1), None).await.unwrap();

    assert_let!(Some(timeline_event) = context_ret.event);
    assert_eq!(timeline_event.event_id(), Some(event_id));

    assert_eq!(1, context_ret.events_before.len());
    assert_eq!(context_ret.events_before[0].event_id(), Some(prev_event_id));

    assert_eq!(1, context_ret.events_after.len());
    assert_eq!(context_ret.events_after[0].event_id(), Some(next_event_id));
}

#[async_test]
async fn test_is_direct() {
    let (client, server) = logged_in_client_with_server().await;
    let own_user_id = client.user_id().unwrap();
    let sync_settings =
        SyncSettings::new().timeout(Duration::from_millis(3000)).token(SyncToken::NoToken);

    let f = EventFactory::new().sender(user_id!("@example:localhost"));

    // Initialize the room with 2 members, including ourself.
    let mut sync_builder = SyncResponseBuilder::new();
    sync_builder.add_joined_room(
        JoinedRoomBuilder::new(&DEFAULT_TEST_ROOM_ID)
            .add_state_event(f.member(user_id!("@example:localhost")).display_name("example"))
            .add_state_event(f.member(&BOB).sender(&BOB)),
    );

    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;
    let _response = client.sync_once(sync_settings.clone()).await.unwrap();
    server.reset().await;

    let room = client.get_room(&DEFAULT_TEST_ROOM_ID).unwrap();

    // The room is not direct.
    assert!(room.direct_targets().is_empty());
    assert!(!room.is_direct().await.unwrap());

    // Set the room as direct.
    let direct_content = json!({
        *BOB: [*DEFAULT_TEST_ROOM_ID],
    });

    // Setting the room as direct will request the members of the room.
    Mock::given(method("GET"))
        .and(path_regex(format!("^/_matrix/client/r0/rooms/{}/members$", *DEFAULT_TEST_ROOM_ID)))
        .and(header("authorization", "Bearer 1234"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "chunk": [
                f.member(user_id!("@example:localhost")).display_name("example").into_raw_sync_state(),
                f.member(&BOB).sender(&BOB).into_raw_sync_state(),
            ],
        })))
        .expect(1)
        .named("members")
        .mount(&server)
        .await;

    Mock::given(method("PUT"))
        .and(path_regex(format!("^/_matrix/client/r0/user/{own_user_id}/account_data/m.direct$")))
        .and(header("authorization", "Bearer 1234"))
        .and(body_json(&direct_content))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .named("set_direct")
        .mount(&server)
        .await;

    room.set_is_direct(true).await.unwrap();

    // Mock the sync response we should get from the homeserver.
    let f = EventFactory::new();
    sync_builder.add_global_account_data(
        f.direct().add_user((*BOB).to_owned().into(), *DEFAULT_TEST_ROOM_ID),
    );
    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;
    let _response = client.sync_once(sync_settings.clone()).await.unwrap();
    server.reset().await;

    // The room is direct now.
    let direct_targets = room.direct_targets();
    assert_eq!(direct_targets.len(), 1);
    assert!(direct_targets.contains(<&DirectUserIdentifier>::from(*BOB)));
    assert!(room.is_direct().await.unwrap());

    // Unset the room as direct.
    let direct_content = json!({});

    Mock::given(method("PUT"))
        .and(path_regex(format!("^/_matrix/client/r0/user/{own_user_id}/account_data/m.direct$")))
        .and(header("authorization", "Bearer 1234"))
        .and(body_json(&direct_content))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
        .expect(1)
        .named("unset_direct")
        .mount(&server)
        .await;

    // Mock the sync response we should get from the homeserver.
    sync_builder.add_global_account_data(f.direct());
    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;
    let _response = client.sync_once(sync_settings.clone()).await.unwrap();
    server.reset().await;

    // The room is not direct anymore.
    assert!(room.direct_targets().is_empty());
    assert!(!room.is_direct().await.unwrap());
}

#[async_test]
async fn test_room_avatar() {
    let (client, server) = logged_in_client_with_server().await;
    let own_user_id = client.user_id().unwrap();

    // Room without avatar.
    mock_sync(&server, &*test_json::SYNC, None).await;

    client.sync_once(SyncSettings::default().token(SyncToken::NoToken)).await.unwrap();
    server.reset().await;

    assert_eq!(client.rooms().len(), 1);
    let room_id = *DEFAULT_TEST_ROOM_ID;
    let room = client.get_room(room_id).unwrap();

    assert_eq!(room.avatar_url(), None);
    assert_matches!(room.avatar_info(), None);

    let factory = EventFactory::new().room(room_id).sender(own_user_id);

    // Set the avatar, but not the info.
    let avatar_url_1 = mxc_uri!("mxc://server.local/abcdef");

    let event = factory.room_avatar().url(avatar_url_1).into_raw_sync();

    let mut sync_builder = SyncResponseBuilder::new();
    sync_builder.add_joined_room(JoinedRoomBuilder::new(room_id).add_timeline_event(event));
    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;

    client.sync_once(SyncSettings::default().token(SyncToken::NoToken)).await.unwrap();
    server.reset().await;

    assert_eq!(room.avatar_url().as_deref(), Some(avatar_url_1));
    assert_matches!(room.avatar_info(), None);

    // Set the avatar and the info.
    let avatar_url_2 = mxc_uri!("mxc://server.local/ghijkl");
    let mut avatar_info_2 = avatar::ImageInfo::new();
    avatar_info_2.height = Some(uint!(200));
    avatar_info_2.width = Some(uint!(200));
    avatar_info_2.mimetype = Some("image/png".to_owned());
    avatar_info_2.size = Some(uint!(5243));

    let event = factory.room_avatar().url(avatar_url_2).info(avatar_info_2).into_raw_sync();

    sync_builder.add_joined_room(JoinedRoomBuilder::new(room_id).add_timeline_event(event));
    mock_sync(&server, sync_builder.build_json_sync_response(), None).await;

    client.sync_once(SyncSettings::default().token(SyncToken::NoToken)).await.unwrap();
    server.reset().await;

    assert_eq!(room.avatar_url().as_deref(), Some(avatar_url_2));
    let avatar_info = room.avatar_info().unwrap();
    assert_eq!(avatar_info.height, Some(uint!(200)));
    assert_eq!(avatar_info.width, Some(uint!(200)));
    assert_eq!(avatar_info.mimetype.as_deref(), Some("image/png"));
    assert_eq!(avatar_info.size, Some(uint!(5243)));
}

#[async_test]
async fn test_is_dm_using_matrix_spec() {
    let server = MatrixMockServer::new().await;
    let client = server
        .client_builder()
        .on_builder(|b| b.dm_room_definition(DmRoomDefinition::MatrixSpec))
        .build()
        .await;
    let alice_user_id = user_id!("@alice:localhost");

    let room_id = room_id!("!room:localhost");

    // We mock the m.direct account data with a single target for the room id
    let direct_data = EventFactory::new()
        .direct()
        .add_user(alice_user_id.to_owned().into(), room_id)
        .into_raw::<AnyGlobalAccountDataEvent>();
    server
        .mock_sync()
        .ok_and_run(&client, |response| {
            response.add_global_account_data(direct_data);
        })
        .await;

    let room = server.sync_joined_room(&client, room_id).await;
    // Room has direct targets, so it's considered direct and a DM using the spec
    // definition.
    assert!(room.compute_is_dm().await.unwrap());

    // We mock the m.direct account data with no targets
    let direct_data = EventFactory::new().direct().into_raw::<AnyGlobalAccountDataEvent>();
    server
        .mock_sync()
        .ok_and_run(&client, |response| {
            response.add_global_account_data(direct_data);
        })
        .await;

    // Room doesn't have direct targets anymore, so it's not a DM using the spec
    // definition.
    assert!(room.compute_is_dm().await.unwrap().not());
}

#[async_test]
async fn test_is_dm_using_at_most_two_members_definition() {
    let server = MatrixMockServer::new().await;
    let client = server
        .client_builder()
        .on_builder(|b| b.dm_room_definition(DmRoomDefinition::TwoMembers))
        .build()
        .await;
    let alice_user_id = user_id!("@alice:localhost");

    let room_id = room_id!("!room:localhost");

    // We mock the m.direct account data with a single target for the room id
    let direct_data = EventFactory::new()
        .direct()
        .add_user(alice_user_id.to_owned().into(), room_id)
        .into_raw::<AnyGlobalAccountDataEvent>();
    server
        .mock_sync()
        .ok_and_run(&client, |response| {
            response.add_global_account_data(direct_data);
        })
        .await;

    // The room has direct targets, and 2 active users, so it's a DM.
    let room = server
        .sync_room(
            &client,
            AnyRoomBuilder::Joined(JoinedRoomBuilder::new(room_id).set_joined_members_count(2)),
        )
        .await;
    assert!(room.compute_is_dm().await.unwrap());

    // The room has direct targets, and a single active user, so it's a DM.
    let room = server
        .sync_room(
            &client,
            AnyRoomBuilder::Joined(JoinedRoomBuilder::new(room_id).set_joined_members_count(1)),
        )
        .await;
    assert!(room.compute_is_dm().await.unwrap());

    // The room has direct targets, and > 2 active users, so it's NOT a DM.
    let room = server
        .sync_room(
            &client,
            AnyRoomBuilder::Joined(JoinedRoomBuilder::new(room_id).set_joined_members_count(3)),
        )
        .await;
    assert!(!room.compute_is_dm().await.unwrap());
}
