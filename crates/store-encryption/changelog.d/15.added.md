`StoreCipher::encrypt_value_with` and `StoreCipher::decrypt_value_with` take a
`StoreCodec`, so the serialization format of an encrypted value is a caller
choice rather than JSON by fiat. Both the value and the envelope that carries it
go through the codec; `encrypt_value` and `decrypt_value` keep writing JSON and
are now thin wrappers over the new pair.
