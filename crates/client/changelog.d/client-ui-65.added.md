`Timeline`'s `AttachmentConfig` gained a `strip_exif` field, forwarded to
`client_matrix::attachment::AttachmentConfig::strip_exif`, which removes the
metadata embedded in an image before uploading it.
