Room key exports and backups now carry the [MSC4268] `ForwarderData` of a
forwarded session, under the vendor-prefixed key
`org.matrix.matrix_rust_sdk.forwarder_data`, so the provenance of a forwarded
key is not lost by a trip through the backup. It is restored on the same terms
as the sender data.

[MSC4268]: https://github.com/matrix-org/matrix-spec-proposals/pull/4268
