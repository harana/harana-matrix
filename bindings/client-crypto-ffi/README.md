# Uniffi based bindings for the Rust SDK crypto crate

This crate contains Uniffi based bindings for the `crypto` crate. The
README mainly describes how to build and integrate the bindings into a Kotlin
based Android project, but the Android specific bits can be skipped if you are
targeting an x86 Linux project.

To build and distribute bindings for iOS projects, see a
[dedicated page](../apple/README.md)

## Prerequisites

### Rust

To build the bindings [Rust] will be needed it can be either installed using an
OS specific package manager or directly with the provided
[installer](https://rustup.rs/).

### Android NDK

The Android NDK will be required as well, it can be installed either through
Android Studio or directly using an
[installer](https://developer.android.com/ndk/downloads).

Point one of `ANDROID_NDK_HOME`, `ANDROID_NDK_ROOT` or `ANDROID_NDK` at the
installation:

```text
$ export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/<some-installed-version>
```

The whole cross-compilation toolchain is derived from that directory. This
matters beyond the linker: the bindings pull in a bundled SQLite through
`libsqlite3-sys`, which compiles C code and therefore needs the NDK's C
compiler and archiver as well.

### Rust target

Install the Rust target for the Android architecture you are building for, for
example:

```text
$ rustup target add aarch64-linux-android
```

Rust supports many different [targets], you'll have to make sure to pick the
right one for your device or emulator.

## Building

### With the xtask (recommended)

`cargo ndk` derives the linker, the C compiler and the archiver from the NDK, so
the whole build works with one command. Install it once:

```text
$ cargo install cargo-ndk
```

Then build the bindings, together with their Kotlin sources, from the repository
root:

```text
$ cargo xtask kotlin build-android-library --package crypto-sdk --src-dir <output-dir>
```

Pass `--only-target aarch64-linux-android` to build a single architecture, and
`--release` for a release build. The task fails early, naming what to set, if no
NDK is configured.

### With cargo directly

A plain `cargo build` does not know about the NDK, so the toolchain variables
have to be set by hand. For `aarch64`, with a Linux host:

```text
$ TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
$ export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$TOOLCHAIN/aarch64-linux-android30-clang"
$ export CC_aarch64_linux_android="$TOOLCHAIN/aarch64-linux-android30-clang"
$ export AR_aarch64_linux_android="$TOOLCHAIN/llvm-ar"
$ cargo build --target aarch64-linux-android
```

Replace `linux-x86_64` with `darwin-x86_64` on macOS, and `30` with the minimum
API level you target. Setting only the linker is not enough: without `CC` the
bundled SQLite build fails with "no C compiler found". Alternatively, the linker
may be set in the `.cargo/config.toml` file in the current directory, any parent
directory, or your home directory:

```text
[target.aarch64-linux-android]
linker = "<path-to-ndk-installation>/toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android30-clang"
```

`CC` and `AR` are read from the environment by the `cc` crate and have no
equivalent Cargo configuration key, so they always have to be exported.

Building without the bundled SQLite avoids the C compilation entirely, if the
target already provides SQLite:

```text
$ cargo build --target aarch64-linux-android --no-default-features
```

### Using the result

After the build, a dynamic library can be found in the
`target/aarch64-linux-android/debug` directory, under the repository root
directory. The library will be called `libcrypto_ffi.so` and needs to
be renamed and copied into the `jniLibs` directory of your Android project, for
Element Android:

```text
$ cp ../../target/aarch64-linux-android/debug/libcrypto_ffi.so \
     /home/example/matrix-android/src/main/jniLibs/aarch64/libuniffi_olm.so
```

The xtask does this copying for you, into the `jniLibs` subdirectory of the
`--src-dir` you pass it.

## License

[Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0)

[Rust]: https://www.rust-lang.org/
[installer]: https://rustup.rs/
[targets]: https://doc.rust-lang.org/nightly/rustc/platform-support.html
[Cargo]: https://doc.rust-lang.org/cargo/
