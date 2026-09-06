New `client_common::types` module, re-exported as `client_matrix::types`,
collecting the `ruma` types that appear throughout the SDK's API (identifiers,
the `Any*Event` enums, `Raw`). Consumers had to add a direct `ruma` dependency
and match its version to name the types the SDK hands them, and the SDK's docs
could not link to those types.
