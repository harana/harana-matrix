use std::time::Duration;

use matrix::{
    attachment::{AttachmentConfig, AttachmentInfo, BaseImageInfo, BaseVideoInfo, Thumbnail},
    media::{MediaFormat, MediaRequestParameters, MediaThumbnailSettings},
    room::reply::{EnforceThread, Reply},
    test_utils::mocks::MatrixMockServer,
};
use sdk_test::{ALICE, DEFAULT_TEST_ROOM_ID, async_test, event_factory::EventFactory};
use ruma::{
    event_id,
    events::{
        Mentions,
        room::{
            MediaSource,
            message::{AddMentions, ReplyWithinThread, TextMessageEventContent},
        },
    },
    mxc_uri, owned_mxc_uri, owned_user_id, uint,
};
use serde_json::json;

#[async_test]
async fn test_room_attachment_send() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;

    let expected_event_id = event_id!("$h29iv0s8:example.com");

    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "info": {
                "mimetype": "image/jpeg",
            }
        }))
        .ok(expected_event_id)
        .mock_once()
        .mount()
        .await;

    mock.mock_upload()
        .expect_mime_type("image/jpeg")
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    let response = room
        .send_attachment(
            "image",
            &mime::IMAGE_JPEG,
            b"Hello world".to_vec(),
            AttachmentConfig::new(),
        )
        .await
        .unwrap();

    assert_eq!(expected_event_id, response.event_id);
}

/// A minimal JPEG carrying an Exif block with a GPS tag and an orientation,
/// plus a comment segment, and nothing that looks like an image.
fn jpeg_with_exif() -> Vec<u8> {
    fn segment(marker: u8, payload: &[u8]) -> Vec<u8> {
        let mut segment = vec![0xFF, marker];
        segment.extend_from_slice(&((payload.len() + 2) as u16).to_be_bytes());
        segment.extend_from_slice(payload);
        segment
    }

    // A TIFF block with an orientation and a GPS latitude reference.
    let mut tiff = b"II\x2a\x00\x08\x00\x00\x00".to_vec();
    tiff.extend_from_slice(&2u16.to_le_bytes());
    // Orientation (0x0112), SHORT, 1, value 6.
    tiff.extend_from_slice(&[0x12, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0, 0, 0]);
    // GPSLatitudeRef (0x0001), ASCII, 2, value "N\0".
    tiff.extend_from_slice(&[0x01, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00]);
    tiff.extend_from_slice(b"N\0\0\0");
    tiff.extend_from_slice(&0u32.to_le_bytes());

    let mut exif = b"Exif\0\0".to_vec();
    exif.extend_from_slice(&tiff);

    let mut jpeg = vec![0xFF, 0xD8];
    jpeg.extend_from_slice(&segment(0xE1, &exif));
    jpeg.extend_from_slice(&segment(0xFE, b"taken at home"));
    jpeg.extend_from_slice(&segment(0xDA, b"scan"));
    jpeg.extend_from_slice(b"entropy coded data");
    jpeg
}

/// With `strip_exif` set, what reaches the media repository is the image
/// without its metadata.
#[async_test]
async fn test_room_attachment_send_strips_exif() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;
    mock.mock_room_send().ok(event_id!("$h29iv0s8:example.com")).mock_once().mount().await;

    let (uploaded, upload_mock) =
        mock.mock_upload().ok_with_capture(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"));
    upload_mock.mock_once().mount().await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    room.send_attachment(
        "image",
        &mime::IMAGE_JPEG,
        jpeg_with_exif(),
        AttachmentConfig::new().strip_exif(true),
    )
    .await
    .unwrap();

    let uploaded = uploaded.await.unwrap();

    let contains = |needle: &[u8]| uploaded.windows(needle.len()).any(|w| w == needle);

    // The comment and the GPS tag are gone.
    assert!(!contains(b"taken at home"));
    assert!(!contains(b"GPS"));
    assert!(uploaded.len() < jpeg_with_exif().len());

    // The image data is untouched, and the orientation survived.
    assert!(contains(b"entropy coded data"));
    assert!(contains(b"scan"));
    assert!(contains(b"Exif\0\0"));
}

/// Without `strip_exif`, the attachment is uploaded byte for byte.
#[async_test]
async fn test_room_attachment_send_keeps_exif_by_default() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;
    mock.mock_room_send().ok(event_id!("$h29iv0s8:example.com")).mock_once().mount().await;

    let (uploaded, upload_mock) =
        mock.mock_upload().ok_with_capture(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"));
    upload_mock.mock_once().mount().await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    room.send_attachment("image", &mime::IMAGE_JPEG, jpeg_with_exif(), AttachmentConfig::new())
        .await
        .unwrap();

    assert_eq!(uploaded.await.unwrap(), jpeg_with_exif());
}

/// With `generate_blurhash` set, the media event carries a BlurHash the
/// receiving client can render while the image downloads.
#[cfg(feature = "image-proc")]
#[async_test]
async fn test_room_attachment_send_generates_blurhash() {
    use image::{ImageFormat, Rgba, RgbaImage};

    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;
    mock.mock_upload()
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let mut png = std::io::Cursor::new(Vec::new());
    RgbaImage::from_pixel(32, 24, Rgba([255, 0, 0, 255]))
        .write_to(&mut png, ImageFormat::Png)
        .unwrap();

    let client = mock.client_builder().build().await;
    let user_id = client.user_id().unwrap().to_owned();

    let (sent, send_mock) =
        mock.mock_room_send().ok_with_capture(event_id!("$h29iv0s8:example.com"), user_id);
    send_mock.mock_once().mount().await;

    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    room.send_attachment(
        "image.png",
        &mime::IMAGE_PNG,
        png.into_inner(),
        AttachmentConfig::new()
            .info(AttachmentInfo::Image(BaseImageInfo {
                height: Some(uint!(24)),
                width: Some(uint!(32)),
                ..Default::default()
            }))
            .generate_blurhash(true),
    )
    .await
    .unwrap();

    let sent: serde_json::Value = serde_json::to_value(sent.await.unwrap()).unwrap();
    let info = &sent["content"]["info"];

    // A 4x3 hash is a fixed length.
    let blurhash =
        info["xyz.amorgan.blurhash"].as_str().expect("the media event should carry a blurhash");
    assert_eq!(blurhash.len(), 28);

    // The dimensions the caller gave are untouched.
    assert_eq!(info["w"], 32);
    assert_eq!(info["h"], 24);
}

/// Without `generate_blurhash`, no hash is computed.
#[async_test]
async fn test_room_attachment_send_has_no_blurhash_by_default() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;
    mock.mock_upload()
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let user_id = client.user_id().unwrap().to_owned();

    let (sent, send_mock) =
        mock.mock_room_send().ok_with_capture(event_id!("$h29iv0s8:example.com"), user_id);
    send_mock.mock_once().mount().await;

    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    room.send_attachment("image.png", &mime::IMAGE_PNG, b"not really a png".to_vec(), {
        AttachmentConfig::new().info(AttachmentInfo::Image(BaseImageInfo::default()))
    })
    .await
    .unwrap();

    let sent: serde_json::Value = serde_json::to_value(sent.await.unwrap()).unwrap();
    assert!(sent["content"]["info"]["xyz.amorgan.blurhash"].is_null());
}

#[cfg(feature = "e2e-encryption")]
#[async_test]
async fn test_room_attachment_send_in_encrypted_room_has_binary_mime_type() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;

    let expected_event_id = event_id!("$h29iv0s8:example.com");

    mock.mock_room_send().ok(expected_event_id).mock_once().mount().await;

    mock.mock_upload()
        .expect_mime_type("application/octet-stream")
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .up_to_n_times(2)
        .mount()
        .await;

    // Needed for the message to be sent in an encrypted room
    mock.mock_get_members().ok(Vec::new()).mock_once().mount().await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().encrypted().mount().await;

    let response = room
        .send_attachment(
            "image",
            &mime::IMAGE_JPEG,
            b"Hello world".to_vec(),
            AttachmentConfig::new(),
        )
        .await
        .expect("Failed to send attachment");

    assert_eq!(expected_event_id, response.event_id);
}

#[async_test]
async fn test_room_attachment_send_info() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;

    let expected_event_id = event_id!("$h29iv0s8:example.com");
    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "info": {
                "mimetype": "image/jpeg",
                "h": 600,
                "w": 800,
            }
        }))
        .ok(expected_event_id)
        .mock_once()
        .mount()
        .await;

    mock.mock_upload()
        .expect_mime_type("image/jpeg")
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    let config = AttachmentConfig::new()
        .info(AttachmentInfo::Image(BaseImageInfo {
            height: Some(uint!(600)),
            width: Some(uint!(800)),
            ..Default::default()
        }))
        .caption(Some(TextMessageEventContent::plain("image caption")));

    let response = room
        .send_attachment("image.jpg", &mime::IMAGE_JPEG, b"Hello world".to_vec(), config)
        .await
        .unwrap();

    assert_eq!(expected_event_id, response.event_id)
}

#[async_test]
async fn test_room_attachment_send_wrong_info() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;

    // Note: this mock is NOT called because the height and width are lost, because
    // we're trying to send the attachment as an image, while we provide a
    // `VideoInfo`.
    //
    // So long for static typing.

    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "info": {
                "mimetype": "image/jpeg",
                "h": 600,
                "w": 800,
            }
        }))
        .ok(event_id!("$unused"))
        .mount()
        .await;

    mock.mock_upload()
        .expect_mime_type("image/jpeg")
        .ok(mxc_uri!("mxc://example.com/yo"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    // Here, using `AttachmentInfo::Video`…
    let config = AttachmentConfig::new()
        .info(AttachmentInfo::Video(BaseVideoInfo {
            height: Some(uint!(600)),
            width: Some(uint!(800)),
            duration: Some(Duration::from_millis(3600)),
            ..Default::default()
        }))
        .caption(Some(TextMessageEventContent::plain("image caption")));

    // But here, using `image/jpeg`.
    let response =
        room.send_attachment("image.jpg", &mime::IMAGE_JPEG, b"Hello world".to_vec(), config).await;

    // In the real-world, this would lead to the size information getting lost,
    // instead of an error during upload. …Is this test any useful?
    response.unwrap_err();
}

#[async_test]
async fn test_room_attachment_send_info_thumbnail() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;

    let media_mxc = owned_mxc_uri!("mxc://example.com/media");
    let thumbnail_mxc = owned_mxc_uri!("mxc://example.com/thumbnail");

    let expected_event_id = event_id!("$h29iv0s8:example.com");

    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "info": {
                "mimetype": "image/jpeg",
                "h": 600,
                "w": 800,
                "thumbnail_info": {
                    "h": 360,
                    "w": 480,
                    "mimetype":"image/jpeg",
                    "size": 3600,
                },
                "thumbnail_url": thumbnail_mxc,
            }
        }))
        .ok(expected_event_id)
        .mock_once()
        .mount()
        .await;

    // First request to /upload: return the thumbnail MXC.
    mock.mock_upload().expect_mime_type("image/jpeg").ok(&thumbnail_mxc).mock_once().mount().await;

    // Second request: return the media MXC.
    mock.mock_upload().expect_mime_type("image/jpeg").ok(&media_mxc).mock_once().mount().await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    // Preconditions: nothing is found in the cache.
    let media_request =
        MediaRequestParameters { source: MediaSource::Plain(media_mxc), format: MediaFormat::File };
    let thumbnail_request = MediaRequestParameters {
        source: MediaSource::Plain(thumbnail_mxc.clone()),
        format: MediaFormat::Thumbnail(MediaThumbnailSettings::new(uint!(480), uint!(360))),
    };

    let _ = client.media().get_media_content(&media_request, true).await.unwrap_err();
    let _ = client.media().get_media_content(&thumbnail_request, true).await.unwrap_err();

    // Send the attachment with a thumbnail.
    let config = AttachmentConfig::new()
        .thumbnail(Some(Thumbnail {
            data: b"Thumbnail".to_vec(),
            content_type: mime::IMAGE_JPEG,
            height: uint!(360),
            width: uint!(480),
            size: uint!(3600),
        }))
        .info(AttachmentInfo::Image(BaseImageInfo {
            height: Some(uint!(600)),
            width: Some(uint!(800)),
            ..Default::default()
        }));

    let response = room
        .send_attachment("image", &mime::IMAGE_JPEG, b"Hello world".to_vec(), config)
        .store_in_cache()
        .await
        .unwrap();

    // The event was sent.
    assert_eq!(response.event_id, expected_event_id);

    // The media is immediately cached in the cache store, so we don't need to set
    // up another mock endpoint for getting the media.
    let reloaded = client.media().get_media_content(&media_request, true).await.unwrap();
    assert_eq!(reloaded, b"Hello world");

    // The thumbnail is cached with sensible defaults.
    let reloaded = client.media().get_media_content(&thumbnail_request, true).await.unwrap();
    assert_eq!(reloaded, b"Thumbnail");

    // The thumbnail can't be retrieved as a file.
    let _ = client
        .media()
        .get_media_content(
            &MediaRequestParameters {
                source: MediaSource::Plain(thumbnail_mxc.clone()),
                format: MediaFormat::File,
            },
            true,
        )
        .await
        .unwrap_err();

    // But it is not found when requesting it as a thumbnail with a different size.
    let thumbnail_request = MediaRequestParameters {
        source: MediaSource::Plain(thumbnail_mxc),
        format: MediaFormat::Thumbnail(MediaThumbnailSettings::new(uint!(42), uint!(1337))),
    };
    let _ = client.media().get_media_content(&thumbnail_request, true).await.unwrap_err();
}

#[async_test]
async fn test_room_attachment_send_mentions() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;

    let expected_event_id = event_id!("$h29iv0s8:example.com");

    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "m.mentions": {
                "user_ids": ["@user:localhost"],
            }
        }))
        .ok(expected_event_id)
        .mock_once()
        .mount()
        .await;

    mock.mock_upload()
        .expect_mime_type("image/jpeg")
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    let response = room
        .send_attachment(
            "image",
            &mime::IMAGE_JPEG,
            b"Hello world".to_vec(),
            AttachmentConfig::new()
                .mentions(Some(Mentions::with_user_ids([owned_user_id!("@user:localhost")]))),
        )
        .await
        .unwrap();

    assert_eq!(expected_event_id, response.event_id);
}

#[async_test]
async fn test_room_attachment_reply_outside_thread() {
    let mock = MatrixMockServer::new().await;

    let expected_event_id = event_id!("$h29iv0s8:example.com");
    let replied_to_event_id = event_id!("$foo:bar.com");

    mock.mock_authenticated_media_config().ok_default().mount().await;

    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "m.relates_to": {
                "m.in_reply_to": {
                    "event_id": replied_to_event_id
                },
            }
        }))
        .ok(expected_event_id)
        .mock_once()
        .mount()
        .await;

    let f = EventFactory::new();
    mock.mock_room_event()
        .match_event_id()
        .ok(f
            .text_msg("Send me your attachments")
            .sender(*ALICE)
            .event_id(replied_to_event_id)
            .into())
        .mock_once()
        .mount()
        .await;

    mock.mock_upload()
        .expect_mime_type("image/jpeg")
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    let response = room
        .send_attachment(
            "image",
            &mime::IMAGE_JPEG,
            b"Hello world".to_vec(),
            AttachmentConfig::new()
                .mentions(Some(Mentions::with_user_ids([owned_user_id!("@user:localhost")])))
                .reply(Some(Reply {
                    event_id: replied_to_event_id.into(),
                    enforce_thread: EnforceThread::Unthreaded,
                    add_mentions: AddMentions::Yes,
                })),
        )
        .await
        .unwrap();

    assert_eq!(expected_event_id, response.event_id);
}

#[async_test]
async fn test_room_attachment_start_thread() {
    let mock = MatrixMockServer::new().await;

    let expected_event_id = event_id!("$h29iv0s8:example.com");
    let replied_to_event_id = event_id!("$foo:bar.com");

    mock.mock_authenticated_media_config().ok_default().mount().await;

    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "m.relates_to": {
                "rel_type": "m.thread",
                "event_id": replied_to_event_id,
                "m.in_reply_to": {
                    "event_id": replied_to_event_id
                },
                "is_falling_back": true
            },
        }))
        .ok(expected_event_id)
        .mock_once()
        .mount()
        .await;

    let f = EventFactory::new();
    mock.mock_room_event()
        .match_event_id()
        .ok(f
            .text_msg("Send me your attachments")
            .sender(*ALICE)
            .event_id(replied_to_event_id)
            .into())
        .mock_once()
        .mount()
        .await;

    mock.mock_upload()
        .expect_mime_type("image/jpeg")
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    let response = room
        .send_attachment(
            "image",
            &mime::IMAGE_JPEG,
            b"Hello world".to_vec(),
            AttachmentConfig::new()
                .mentions(Some(Mentions::with_user_ids([owned_user_id!("@user:localhost")])))
                .reply(Some(Reply {
                    event_id: replied_to_event_id.into(),
                    enforce_thread: EnforceThread::Threaded(ReplyWithinThread::No),
                    add_mentions: AddMentions::Yes,
                })),
        )
        .await
        .unwrap();

    assert_eq!(expected_event_id, response.event_id);
}

#[async_test]
async fn test_room_attachment_reply_on_thread_as_reply() {
    let mock = MatrixMockServer::new().await;

    let expected_event_id = event_id!("$h29iv0s8:example.com");
    let thread_root_event_id = event_id!("$bar:foo.com");
    let replied_to_event_id = event_id!("$foo:bar.com");

    mock.mock_authenticated_media_config().ok_default().mount().await;

    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "m.relates_to": {
                "rel_type": "m.thread",
                "event_id": thread_root_event_id,
                "m.in_reply_to": {
                    "event_id": replied_to_event_id
                },
            },
        }))
        .ok(expected_event_id)
        .mock_once()
        .mount()
        .await;

    let f = EventFactory::new();
    mock.mock_room_event()
        .match_event_id()
        .ok(f
            .text_msg("Send me your attachments")
            .sender(*ALICE)
            .event_id(replied_to_event_id)
            .in_thread(thread_root_event_id, thread_root_event_id)
            .into())
        .mock_once()
        .mount()
        .await;

    mock.mock_upload()
        .expect_mime_type("image/jpeg")
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    let response = room
        .send_attachment(
            "image",
            &mime::IMAGE_JPEG,
            b"Hello world".to_vec(),
            AttachmentConfig::new()
                .mentions(Some(Mentions::with_user_ids([owned_user_id!("@user:localhost")])))
                .reply(Some(Reply {
                    event_id: replied_to_event_id.into(),
                    enforce_thread: EnforceThread::Threaded(ReplyWithinThread::Yes),
                    add_mentions: AddMentions::Yes,
                })),
        )
        .await
        .unwrap();

    assert_eq!(expected_event_id, response.event_id);
}

#[async_test]
async fn test_room_attachment_reply_forwarding_thread() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;

    let expected_event_id = event_id!("$h29iv0s8:example.com");
    let thread_root_event_id = event_id!("$bar:foo.com");
    let replied_to_event_id = event_id!("$foo:bar.com");

    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "m.relates_to": {
                "rel_type": "m.thread",
                "event_id": thread_root_event_id,
                "m.in_reply_to": {
                    "event_id": replied_to_event_id
                },
                "is_falling_back": true
            },
        }))
        .ok(expected_event_id)
        .mock_once()
        .mount()
        .await;

    let f = EventFactory::new();
    mock.mock_room_event()
        .match_event_id()
        .ok(f
            .text_msg("Send me your attachments")
            .sender(*ALICE)
            .event_id(replied_to_event_id)
            .in_thread(thread_root_event_id, thread_root_event_id)
            .into())
        .mock_once()
        .mount()
        .await;

    mock.mock_upload()
        .expect_mime_type("image/jpeg")
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    let response = room
        .send_attachment(
            "image",
            &mime::IMAGE_JPEG,
            b"Hello world".to_vec(),
            AttachmentConfig::new()
                .mentions(Some(Mentions::with_user_ids([owned_user_id!("@user:localhost")])))
                .reply(Some(Reply {
                    event_id: replied_to_event_id.into(),
                    enforce_thread: EnforceThread::MaybeThreaded,
                    add_mentions: AddMentions::Yes,
                })),
        )
        .await
        .unwrap();

    assert_eq!(expected_event_id, response.event_id);
}

#[async_test]
async fn test_room_attachment_send_is_animated() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;

    let expected_event_id = event_id!("$h29iv0s8:example.com");
    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "info": {
                "mimetype": "image/jpeg",
                "h": 600,
                "w": 800,
                "is_animated": false,
            }
        }))
        .ok(expected_event_id)
        .mock_once()
        .mount()
        .await;

    mock.mock_upload()
        .expect_mime_type("image/jpeg")
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    let config = AttachmentConfig::new()
        .info(AttachmentInfo::Image(BaseImageInfo {
            height: Some(uint!(600)),
            width: Some(uint!(800)),
            is_animated: Some(false),
            ..Default::default()
        }))
        .caption(Some(TextMessageEventContent::plain("image caption")));

    let response = room
        .send_attachment("image.jpg", &mime::IMAGE_JPEG, b"Hello world".to_vec(), config)
        .await
        .unwrap();

    assert_eq!(expected_event_id, response.event_id)
}

#[async_test]
async fn test_room_attachment_send_extra_content() {
    let mock = MatrixMockServer::new().await;

    mock.mock_authenticated_media_config().ok_default().mount().await;

    let expected_event_id = event_id!("$h29iv0s8:example.com");

    // The custom field must be present on the sent event, while the extra
    // field colliding with a real event field must have been ignored.
    mock.mock_room_send()
        .body_matches_partial_json(json!({
            "msgtype": "m.image",
            "com.example.custom": "custom value",
        }))
        .ok(expected_event_id)
        .mock_once()
        .mount()
        .await;

    mock.mock_upload()
        .expect_mime_type("image/jpeg")
        .ok(mxc_uri!("mxc://example.com/AQwafuaFswefuhsfAFAgsw"))
        .mock_once()
        .mount()
        .await;

    let client = mock.client_builder().build().await;
    let room = mock.sync_joined_room(&client, &DEFAULT_TEST_ROOM_ID).await;
    mock.mock_room_state_encryption().plain().mount().await;

    let config = AttachmentConfig::new().extra_content(Some(serde_json::Map::from_iter([
        ("com.example.custom".to_owned(), "custom value".into()),
        // The event's own fields take precedence over extra fields.
        ("msgtype".to_owned(), "com.example.overridden".into()),
    ])));

    let response = room
        .send_attachment("image", &mime::IMAGE_JPEG, b"Hello world".to_vec(), config)
        .await
        .unwrap();

    assert_eq!(expected_event_id, response.event_id);
}
