`StateStoreDataValue::UtdHookManagerData` now holds a
`client_common::bloom_filter::GrowableBloom` instead of a
`growable_bloom_filter::GrowableBloom`. The type is the same implementation
vendored into `client-common`, and its serialized form is unchanged, so only the
import path needs updating.
