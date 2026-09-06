`MediaFormat` and `MediaThumbnailSettings` now derive `PartialEq`, `Eq`,
`PartialOrd`, `Ord` and `Hash`, so they can be used as map or set keys.
`MediaRequestParameters` still cannot, because `ruma`'s `MediaSource` does not
implement those traits.
