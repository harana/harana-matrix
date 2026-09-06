# FFI examples

Reference walk-throughs of [`client-matrix-ffi`] from the languages it is bound
to. Each one covers the same ground, in the order a client would:

1. setting the platform up (logging, the async runtime, the log and panic
   listeners),
2. building a client and pointing it at a session store,
3. authenticating with a password, and restoring a stored session instead,
4. bootstrapping cross-signing,
5. starting the sync service,
6. sending a message,
7. basic room operations: listing rooms, reading and writing state events, room
   and global account data, inviting a user.

- [`swift/Example.swift`](./swift/Example.swift)
- [`kotlin/Example.kt`](./kotlin/Example.kt)

They are written to be read and copied from rather than run as they are: every
step is a function of its own, and `runExample` chains them together. Fill in a
user ID, a password and a room ID of your own before running one against a
homeserver.

## Building the bindings the examples use

### Swift

Build the framework and the Swift sources, then point a package at them:

```text
cargo xtask swift build-framework
```

The generated `MatrixSDKFFI.xcframework` and Swift sources land in
`bindings/apple/generated`, and `Package.swift` is copied to the repository
root so a local package can depend on them. Add `Example.swift` to a target
that depends on `MatrixRustSDK`. `bindings/apple/README.md` covers the build in
detail, including building the crypto module on its own.

### Kotlin

Build the Android library and generate the Kotlin sources:

```text
cargo xtask kotlin build-android-library --package full-sdk --src-dir <output-dir>
```

This needs the Android NDK and `cargo ndk`; see
`bindings/client-crypto-ffi/README.md` for the toolchain setup. The task
writes the native libraries to `<output-dir>/jniLibs` and the Kotlin sources to
`<output-dir>/kotlin`, both of which go into an Android project.

The generated Kotlin targets Android (it uses `android.os.Build` for the
object cleaner), so `Example.kt` belongs in an Android module or an
instrumented test rather than a plain JVM program. Its calls suspend, so run
them from a coroutine.

## Keeping them honest

The examples are compiled by hand, not by CI, so they are kept in sync with the
FFI surface as it changes. If one drifts from the API, that is a bug worth
reporting.

[`client-matrix-ffi`]: ../client-matrix-ffi
