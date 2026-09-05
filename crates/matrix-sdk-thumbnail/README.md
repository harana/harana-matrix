# matrix-sdk-thumbnail

Thumbnail generation for Matrix media.

A client uploading an image is expected to upload a thumbnail with it, and a
homeserver serving `/thumbnail` generates one on demand. Both do the same three
things, and both are decoding a picture someone else supplied:

- [`Dim`] carries a requested width, height and method, and answers the
  questions the specification asks about them: what the scaled size is,
  which [normalized] size a request rounds up to, and whether generating would
  merely reproduce or upscale the source ([`Dim::is_passthrough`]).
- [`decode`] decodes within a pixel budget. The budget is checked against the
  header's declared dimensions *before* any decoder allocates, because the
  decoder's own byte limit is advisory and a decoder is free to ignore it: a
  small file declaring an enormous canvas is the shape of a decompression bomb.
- [`generate`] scales or crops, never upscaling, and [`thumbnail`] does all
  three in one call and encodes the result as PNG.

Ported from [tuwunel]'s `src/service/media/thumbnail.rs`, without its storage,
federation fetching or database metadata.

[normalized]: https://spec.matrix.org/latest/client-server-api/#thumbnails
[tuwunel]: https://github.com/matrix-construct/tuwunel
