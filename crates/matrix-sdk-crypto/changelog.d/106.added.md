Room key exports and backups now carry the `SenderData` we had established for
the session, under the vendor-prefixed key
`org.matrix.matrix_rust_sdk.sender_data`. It is restored when importing from a
backup we could authenticate, so a session restored from backup can still attest
to its sender instead of showing its messages as coming from nobody in
particular. File imports and unauthenticated backups still ignore the field.
