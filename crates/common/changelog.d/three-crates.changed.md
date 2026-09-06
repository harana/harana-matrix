The workspace is now three published crates rather than thirty:
`harana-matrix-client`, `harana-matrix-common` and `harana-matrix-server`, plus
`harana-matrix-macros` for the proc macros, which cannot be merged into a normal
library. Each of the old crates is a module of the crate for its tier, behind a
feature of the same name where it was optional, so `client_crypto::OlmMachine`
is now `harana_matrix_client::crypto::OlmMachine` and `common_ruma::events` is
`harana_matrix_common::events`.
