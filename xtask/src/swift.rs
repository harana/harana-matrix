use std::{
    collections::HashMap,
    fs::{copy, create_dir_all, remove_dir_all, remove_file, rename},
};

use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, Subcommand};
use uniffi_bindgen::bindings::{GenerateOptions, TargetLanguage, generate};
use xshell::cmd;

use crate::{Result, sh, workspace};

/// Builds the SDK for Swift as a Static Library or XCFramework.
#[derive(Args)]
pub struct SwiftArgs {
    #[clap(subcommand)]
    cmd: SwiftCommand,
}

#[derive(Subcommand)]
#[allow(clippy::enum_variant_names)]
enum SwiftCommand {
    /// Builds the SDK for Swift as a static lib.
    BuildLibrary,

    /// Builds the SDK for Swift as an XCFramework.
    BuildFramework {
        /// Build with the release profile
        #[clap(long)]
        release: bool,

        /// Build with a custom profile, takes precedence over `--release`
        #[clap(long)]
        profile: Option<String>,

        /// Build the given target. This option can be specified multiple times
        /// to build more than one. Omitting this option will build all
        /// supported targets.
        #[clap(long)]
        target: Option<Vec<String>>,

        /// Includes the Tier 3 targets (such as watchOS) when building all
        /// supported targets. Requires a nightly toolchain with the `rust-src`
        /// component installed.
        #[clap(long)]
        tier3_targets: bool,

        /// Move the generated xcframework and swift sources into the given
        /// components-folder
        #[clap(long)]
        components_path: Option<Utf8PathBuf>,

        /// The macOS deployment target to use when building the framework.
        ///
        /// Defaults to not being set, which implies that the build will use the
        /// default values provided by the Rust and Xcode toolchains.
        #[clap(long)]
        macos_deployment_target: Option<String>,

        /// The iOS deployment target to use when building the framework.
        ///
        /// Defaults to not being set, which implies that the build will use the
        /// default values provided by the Rust and Xcode toolchains.
        #[clap(long)]
        ios_deployment_target: Option<String>,

        /// The wachOS deployment target to use when building the framework.
        ///
        /// Defaults to not being set, which implies that the build will use the
        /// default values provided by the Rust and Xcode toolchains.
        #[clap(long)]
        watchos_deployment_target: Option<String>,

        /// Build the targets one by one instead of passing all of them
        /// to cargo in one go, which makes it hang on lesser devices like plain
        /// Apple Silicon M1s
        #[clap(long)]
        sequentially: bool,
    },

    /// Builds the Crypto SDK for Swift as an XCFramework, zipped up next to
    /// its Swift sources and the licence, ready to be attached to a release.
    BuildCryptoFramework {
        /// Build with the release profile. This is the default; pass
        /// `--profile` to build with another one.
        #[clap(long)]
        release: bool,

        /// Build with a custom profile, takes precedence over `--release`
        #[clap(long)]
        profile: Option<String>,

        /// Build for iOS only, skipping macOS and the iOS simulator
        #[clap(long)]
        only_ios: bool,

        /// Build the targets one by one instead of passing all of them to
        /// cargo in one go, which makes it hang on lesser devices like plain
        /// Apple Silicon M1s
        #[clap(long)]
        sequentially: bool,

        /// The iOS deployment target to use when building the framework.
        ///
        /// Defaults to not being set, which implies that the build will use the
        /// default values provided by the Rust and Xcode toolchains.
        #[clap(long)]
        ios_deployment_target: Option<String>,

        /// The macOS deployment target to use when building the framework.
        ///
        /// Defaults to not being set, which implies that the build will use the
        /// default values provided by the Rust and Xcode toolchains.
        #[clap(long)]
        macos_deployment_target: Option<String>,
    },
}

impl SwiftArgs {
    pub fn run(self) -> Result<()> {
        let sh = sh();
        let _p = sh.push_dir(workspace::root_path()?);

        match self.cmd {
            SwiftCommand::BuildLibrary => build_library(),
            SwiftCommand::BuildFramework {
                release,
                profile,
                target: targets,
                tier3_targets,
                components_path,
                macos_deployment_target,
                ios_deployment_target,
                watchos_deployment_target,
                sequentially,
            } => {
                // The dev profile seems to cause crashes on some platforms so we default to
                // reldbg (https://github.com/matrix-org/matrix-rust-sdk/issues/4009)
                let profile =
                    profile.as_deref().unwrap_or(if release { "small-release" } else { "reldbg" });
                build_xcframework(
                    profile,
                    targets,
                    tier3_targets,
                    components_path,
                    sequentially,
                    macos_deployment_target.as_deref(),
                    ios_deployment_target.as_deref(),
                    watchos_deployment_target.as_deref(),
                )
            }
            SwiftCommand::BuildCryptoFramework {
                release,
                profile,
                only_ios,
                sequentially,
                ios_deployment_target,
                macos_deployment_target,
            } => {
                // The shell script this replaced always built with `--release`.
                let _ = release;
                let profile = profile.as_deref().unwrap_or("release");

                build_crypto_xcframework(
                    profile,
                    only_ios,
                    sequentially,
                    macos_deployment_target.as_deref(),
                    ios_deployment_target.as_deref(),
                )
            }
        }
    }
}

/// A specific build target supported by the SDK.
struct Target {
    triple: &'static str,
    platform: Platform,
    status: TargetStatus,
    description: &'static str,
}

#[derive(Hash, PartialEq, Eq, Clone)]
enum TargetStatus {
    /// A tier 1 or 2 target that can be built with stable Rust.
    TopTier,
    /// A tier 3 target that requires nightly Rust and `-Zbuild-std`.
    Tier3,
}

/// The platform for which a particular target can run on.
#[derive(Hash, PartialEq, Eq, Clone)]
enum Platform {
    Macos,
    Ios,
    IosSimulator,
    Watchos,
    WatchosSimulator,
}

impl Platform {
    /// The human-readable name of the platform.
    fn as_str(&self) -> &str {
        match self {
            Platform::Macos => "macOS",
            Platform::Ios => "iOS",
            Platform::IosSimulator => "iOS Simulator",
            Platform::Watchos => "watchOS",
            Platform::WatchosSimulator => "watchOS Simulator",
        }
    }

    /// The name of the subfolder in which to place the library for the platform
    /// once all architectures are lipo'd together.
    fn lib_folder_name(&self) -> &str {
        match self {
            Platform::Macos => "macos",
            Platform::Ios => "ios",
            Platform::IosSimulator => "ios-simulator",
            Platform::Watchos => "watchos",
            Platform::WatchosSimulator => "watchos-simulator",
        }
    }
}
/// A crate built into an XCFramework for Apple platforms.
struct FfiPackage {
    /// The name of the cargo package to build.
    crate_name: &'static str,

    /// The base name of the static library it produces.
    library_name: &'static str,

    /// The features to build it with.
    features: &'static str,

    /// The name of the generated framework, which is also the name of the
    /// Swift module its sources import.
    framework_name: &'static str,
}

/// The full SDK, `matrix-sdk-ffi`.
const FULL_SDK: FfiPackage = FfiPackage {
    crate_name: "matrix-sdk-ffi",
    library_name: "libmatrix_sdk_ffi.a",
    features: "sentry",
    framework_name: "MatrixSDKFFI",
};

/// The crypto module on its own, `matrix-sdk-crypto-ffi`, for clients that
/// already depend on another SDK.
const CRYPTO_SDK: FfiPackage = FfiPackage {
    crate_name: "matrix-sdk-crypto-ffi",
    library_name: "libmatrix_sdk_crypto_ffi.a",
    features: "",
    framework_name: "MatrixSDKCryptoFFI",
};

/// The list of targets supported by the SDK.
const TARGETS: &[Target] = &[
    Target {
        triple: "aarch64-apple-ios",
        platform: Platform::Ios,
        status: TargetStatus::TopTier,
        description: "iOS",
    },
    Target {
        triple: "aarch64-apple-darwin",
        platform: Platform::Macos,
        status: TargetStatus::TopTier,
        description: "macOS (Apple Silicon)",
    },
    Target {
        triple: "x86_64-apple-darwin",
        platform: Platform::Macos,
        status: TargetStatus::TopTier,
        description: "macOS (Intel)",
    },
    Target {
        triple: "aarch64-apple-ios-sim",
        platform: Platform::IosSimulator,
        status: TargetStatus::TopTier,
        description: "iOS Simulator (Apple Silicon)",
    },
    Target {
        triple: "x86_64-apple-ios",
        platform: Platform::IosSimulator,
        status: TargetStatus::TopTier,
        description: "iOS Simulator (Intel) ",
    },
    Target {
        triple: "aarch64-apple-watchos",
        platform: Platform::Watchos,
        status: TargetStatus::Tier3,
        description: "watchOS (ARM64)",
    },
    Target {
        triple: "arm64_32-apple-watchos",
        platform: Platform::Watchos,
        status: TargetStatus::Tier3,
        description: "watchOS (ARM64_32)",
    },
    Target {
        triple: "aarch64-apple-watchos-sim",
        platform: Platform::WatchosSimulator,
        status: TargetStatus::Tier3,
        description: "watchOS Simulator (ARM64)",
    },
    Target {
        triple: "x86_64-apple-watchos-sim",
        platform: Platform::WatchosSimulator,
        status: TargetStatus::Tier3,
        description: "watchOS Simulator (Intel)",
    },
];

fn build_library() -> Result<()> {
    println!("Running debug library build.");

    let root_directory = workspace::root_path()?;
    let target_directory = workspace::target_path()?;
    let ffi_directory = root_directory.join("bindings/apple/generated/matrix_sdk_ffi");
    let lib_output_dir = target_directory.join("debug");

    create_dir_all(ffi_directory.as_path())?;

    let sh = sh();
    let package = &FULL_SDK;
    let (crate_name, features) = (package.crate_name, package.features);
    cmd!(sh, "rustup run stable cargo build -p {crate_name} --features {features}").run()?;

    rename(lib_output_dir.join(package.library_name), ffi_directory.join(package.library_name))?;
    let swift_directory = root_directory.join("bindings/apple/generated/swift");
    create_dir_all(swift_directory.as_path())?;

    generate_uniffi(&ffi_directory.join(package.library_name), &ffi_directory)?;

    let module_map_file = ffi_directory.join("module.modulemap");
    if module_map_file.exists() {
        remove_file(module_map_file.as_path())?;
    }

    consolidate_modulemap_files(&ffi_directory, &ffi_directory)?;
    move_files("swift", &ffi_directory, &swift_directory)?;
    update_swift_module_imports(&swift_directory, package.framework_name)?;
    Ok(())
}

fn generate_uniffi(library_path: &Utf8Path, ffi_directory: &Utf8Path) -> Result<()> {
    generate(GenerateOptions {
        languages: vec![TargetLanguage::Swift],
        source: library_path.to_path_buf(),
        out_dir: ffi_directory.to_path_buf(),
        ..GenerateOptions::default()
    })?;
    Ok(())
}

fn build_xcframework(
    profile: &str,
    targets: Option<Vec<String>>,
    tier3_targets: bool,
    components_path: Option<Utf8PathBuf>,
    sequentially: bool,
    macos_deployment_target: Option<&str>,
    ios_deployment_target: Option<&str>,
    watchos_deployment_target: Option<&str>,
) -> Result<()> {
    let root_dir = workspace::root_path()?;
    let apple_dir = root_dir.join("bindings/apple");
    let generated_dir = apple_dir.join("generated");

    // Cleanup destination folder
    let _ = remove_dir_all(&generated_dir);
    create_dir_all(&generated_dir)?;

    let headers_dir = generated_dir.join("headers");
    // Use a subdirectory to fix conflicts with other UniFFI libraries.
    let headers_module_dir = headers_dir.join("MatrixSDKFFI");
    let swift_dir = generated_dir.join("swift");
    create_dir_all(headers_module_dir.clone())?;
    create_dir_all(swift_dir.clone())?;

    let targets = if let Some(triples) = targets {
        triples
            .iter()
            .map(|t| {
                TARGETS.iter().find(|target| target.triple == *t).expect("Invalid target specified")
            })
            .collect()
    } else if tier3_targets {
        TARGETS.iter().collect()
    } else {
        TARGETS.iter().filter(|target| target.status == TargetStatus::TopTier).collect()
    };

    let package = &FULL_SDK;

    let platform_build_paths = build_targets(
        targets,
        profile,
        sequentially,
        macos_deployment_target,
        ios_deployment_target,
        watchos_deployment_target,
        package,
    )?;
    let libs = lipo_platform_libraries(&platform_build_paths, &generated_dir, package)?;

    println!("-- Generating uniffi files");
    let uniffi_lib_path = platform_build_paths.values().next().unwrap().first().unwrap().clone();
    generate_uniffi(&uniffi_lib_path, &generated_dir)?;

    move_files("h", &generated_dir, &headers_module_dir)?;
    consolidate_modulemap_files(&generated_dir, &headers_module_dir)?;

    move_files("swift", &generated_dir, &swift_dir)?;
    update_swift_module_imports(&swift_dir, package.framework_name)?;

    println!("-- Generating MatrixSDKFFI.xcframework framework");
    let xcframework_path = generated_dir.join("MatrixSDKFFI.xcframework");
    if xcframework_path.exists() {
        remove_dir_all(&xcframework_path)?;
    }
    let sh = sh();
    let mut cmd = cmd!(sh, "xcodebuild -create-xcframework");
    for p in libs {
        cmd = cmd.arg("-library").arg(p).arg("-headers").arg(&headers_dir)
    }
    cmd.arg("-output").arg(&xcframework_path).run()?;

    // Copy the Swift package manifest to the SDK root for local development.
    copy(apple_dir.join("Debug-Package.swift"), root_dir.join("Package.swift"))?;

    // Copy an empty package to folders we want ignored
    let ignored_package_folders = ["target"];
    for path in ignored_package_folders {
        copy(
            apple_dir.join("Debug-Empty-Package.swift"),
            root_dir.join(path).join("Package.swift"),
        )?;
    }

    // cleaning up the intermediate data
    remove_dir_all(headers_dir.as_path())?;

    if let Some(path) = components_path {
        println!("-- Copying MatrixSDKFFI.xcframework to {path}");
        let framework_target = path.join("MatrixSDKFFI.xcframework");
        let swift_target = path.join("Sources/MatrixRustSDK");
        if framework_target.exists() {
            remove_dir_all(&framework_target)?;
        }
        if swift_target.exists() {
            remove_dir_all(&swift_target)?;
        }
        create_dir_all(&framework_target)?;
        create_dir_all(&swift_target)?;

        let copy_options = fs_extra::dir::CopyOptions { content_only: true, ..Default::default() };

        fs_extra::dir::copy(&xcframework_path, &framework_target, &copy_options)?;
        fs_extra::dir::copy(&swift_dir, &swift_target, &copy_options)?;
    }

    println!("-- All done and hunky dory. Enjoy!");

    Ok(())
}

/// Builds the Crypto SDK into an XCFramework, zipped up with its Swift sources
/// and the licence, ready to be attached to a GitHub release and consumed by
/// `MatrixSDKCrypto.podspec`.
fn build_crypto_xcframework(
    profile: &str,
    only_ios: bool,
    sequentially: bool,
    macos_deployment_target: Option<&str>,
    ios_deployment_target: Option<&str>,
) -> Result<()> {
    let package = &CRYPTO_SDK;
    let framework_name = package.framework_name;

    let root_dir = workspace::root_path()?;
    let generated_dir = root_dir.join("generated");

    // Cleanup destination folder
    let _ = remove_dir_all(&generated_dir);
    create_dir_all(&generated_dir)?;

    let headers_dir = generated_dir.join("headers");
    let swift_dir = generated_dir.join("Sources");
    create_dir_all(&headers_dir)?;
    create_dir_all(&swift_dir)?;

    let targets = TARGETS
        .iter()
        .filter(|target| {
            if only_ios {
                target.platform == Platform::Ios
            } else {
                matches!(target.platform, Platform::Ios | Platform::Macos | Platform::IosSimulator)
            }
        })
        .collect::<Vec<_>>();

    let platform_build_paths = build_targets(
        targets,
        profile,
        sequentially,
        macos_deployment_target,
        ios_deployment_target,
        None,
        package,
    )?;
    let libs = lipo_platform_libraries(&platform_build_paths, &generated_dir, package)?;

    println!("-- Generating uniffi files");
    let uniffi_lib_path = platform_build_paths
        .get(&Platform::Ios)
        .and_then(|paths| paths.first())
        .expect("iOS is always built")
        .clone();
    generate_uniffi(&uniffi_lib_path, &generated_dir)?;

    move_files("h", &generated_dir, &headers_dir)?;
    consolidate_modulemap_files(&generated_dir, &headers_dir)?;

    move_files("swift", &generated_dir, &swift_dir)?;
    update_swift_module_imports(&swift_dir, framework_name)?;

    println!("-- Generating {framework_name}.xcframework framework");
    let xcframework_path = generated_dir.join(format!("{framework_name}.xcframework"));
    if xcframework_path.exists() {
        remove_dir_all(&xcframework_path)?;
    }

    let sh = sh();
    let mut cmd = cmd!(sh, "xcodebuild -create-xcframework");
    for path in libs {
        cmd = cmd.arg("-library").arg(path).arg("-headers").arg(&headers_dir);
    }
    cmd.arg("-output").arg(&xcframework_path).run()?;

    // Cleaning up the intermediate data.
    remove_dir_all(&headers_dir)?;
    let lipo_dir = generated_dir.join("lipo");
    if lipo_dir.exists() {
        remove_dir_all(&lipo_dir)?;
    }

    // Zip the framework, the sources and the licence up together, which is what
    // the podspec downloads.
    println!("-- Zipping {framework_name}.zip");
    let licence_name = "LICENSE";
    let licence_copy = generated_dir.join(licence_name);
    copy(root_dir.join(licence_name), &licence_copy)?;

    let zip_name = format!("{framework_name}.zip");
    let framework_dir_name = format!("{framework_name}.xcframework");
    {
        let _p = sh.push_dir(&generated_dir);
        cmd!(sh, "zip -r {zip_name} {framework_dir_name} Sources {licence_name}").run()?;
    }
    remove_file(&licence_copy)?;

    println!("-- All done and hunky dory. Enjoy!");

    Ok(())
}

/// Builds the SDK for the specified targets and profile.
fn build_targets(
    targets: Vec<&Target>,
    profile: &str,
    sequentially: bool,
    macos_deployment_target: Option<&str>,
    ios_deployment_target: Option<&str>,
    watchos_deployment_target: Option<&str>,
    package: &FfiPackage,
) -> Result<HashMap<Platform, Vec<Utf8PathBuf>>> {
    let sh = sh();
    let (crate_name, features) = (package.crate_name, package.features);

    // Note: `push_env` stores environment variables and returns a RAII guard that
    // will restore the environment variable to its previous value when dropped.
    let _env_guard1 =
        sh.push_env("CARGO_TARGET_AARCH64_APPLE_IOS_RUSTFLAGS", "-Clinker=/usr/bin/clang");
    let _env_guard2 = sh.push_env("AARCH64_APPLE_IOS_CC", "/usr/bin/clang");
    let _env_guard3 =
        macos_deployment_target.map(|target| sh.push_env("MACOSX_DEPLOYMENT_TARGET", target));
    let _env_guard4 =
        ios_deployment_target.map(|target| sh.push_env("IPHONEOS_DEPLOYMENT_TARGET", target));
    let _env_guard5 =
        watchos_deployment_target.map(|target| sh.push_env("WATCHOS_DEPLOYMENT_TARGET", target));

    if sequentially {
        for target in &targets {
            let triple = target.triple;

            println!("-- Building for {}", target.description);
            if target.status == TargetStatus::TopTier {
                cmd!(sh, "rustup run stable cargo build -p {crate_name} --target {triple} --profile {profile} --features {features}")
                    .run()?;
            } else {
                cmd!(sh, "rustup run nightly cargo build -p {crate_name} -Zbuild-std --target {triple} --profile {profile} --features {features}")
                    .run()?;
            }
        }
    } else {
        let (stable_targets, tier3_targets): (Vec<&Target>, Vec<&Target>) =
            targets.iter().partition(|t| t.status == TargetStatus::TopTier);

        if !stable_targets.is_empty() {
            let triples = stable_targets.iter().map(|target| target.triple).collect::<Vec<_>>();
            let mut cmd = cmd!(sh, "rustup run stable cargo build -p {crate_name}");
            for triple in &triples {
                cmd = cmd.arg("--target").arg(triple);
            }
            cmd = cmd.arg("--profile").arg(profile).arg("--features").arg(features);

            println!("-- Building for {} targets", triples.len());
            cmd.run()?;
        }

        if !tier3_targets.is_empty() {
            let triples = tier3_targets.iter().map(|target| target.triple).collect::<Vec<_>>();
            let mut cmd = cmd!(sh, "rustup run nightly cargo build -p {crate_name} -Zbuild-std");
            for triple in &triples {
                cmd = cmd.arg("--target").arg(triple);
            }
            cmd = cmd.arg("--profile").arg(profile).arg("--features").arg(features);

            println!("-- Building for {} targets with nightly -Zbuild-std", triples.len());
            cmd.run()?;
        }
    }

    // a hashmap of platform to array, where each array contains all the paths for
    // that platform.
    let mut platform_build_paths = HashMap::new();
    for target in targets {
        let path = build_path_for_target(target, profile, package)?;
        let paths = platform_build_paths.entry(target.platform.clone()).or_insert_with(Vec::new);
        paths.push(path);
    }

    Ok(platform_build_paths)
}

/// The path of the built library for a specific target and profile.
fn build_path_for_target(
    target: &Target,
    profile: &str,
    package: &FfiPackage,
) -> Result<Utf8PathBuf> {
    // The builtin dev profile has its files stored under target/debug, all
    // other targets have matching directory names
    let profile_dir_name = if profile == "dev" { "debug" } else { profile };
    Ok(workspace::target_path()?
        .join(target.triple)
        .join(profile_dir_name)
        .join(package.library_name))
}

/// Lipo's together the libraries for each platform into a single library.
fn lipo_platform_libraries(
    platform_build_paths: &HashMap<Platform, Vec<Utf8PathBuf>>,
    generated_dir: &Utf8Path,
    package: &FfiPackage,
) -> Result<Vec<Utf8PathBuf>> {
    let mut libs = Vec::new();
    let sh = sh();
    for platform in platform_build_paths.keys() {
        let paths = platform_build_paths.get(platform).unwrap();

        if paths.len() == 1 {
            libs.push(paths[0].clone());
            continue;
        }

        let output_folder = generated_dir.join("lipo").join(platform.lib_folder_name());
        create_dir_all(&output_folder)?;

        let output_path = output_folder.join(package.library_name);
        let mut cmd = cmd!(sh, "lipo -create");
        for path in paths {
            cmd = cmd.arg(path);
        }
        cmd = cmd.arg("-output").arg(&output_path);

        println!("-- Running Lipo for {}", platform.as_str());
        cmd.run()?;

        libs.push(output_path);
    }
    Ok(libs)
}

/// Moves all files of the specified file extension from one directory into
/// another.
fn move_files(extension: &str, source: &Utf8Path, destination: &Utf8Path) -> Result<()> {
    for entry in source.read_dir_utf8()? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            let path = entry.path();
            if path.extension() == Some(extension) {
                let file_name = path.file_name().expect("Failed to get file name");
                rename(path, destination.join(file_name)).expect("Failed to move swift file");
            }
        }
    }
    Ok(())
}

/// Updates all the swift files in the given directory to import the same module
/// that gets defined by the `consolidate_modulemap_files` function.
fn update_swift_module_imports(directory: &Utf8Path, framework_name: &str) -> Result<()> {
    let regex = regex::Regex::new(r"#if canImport\(\w+FFI\)\nimport \w+FFI\n#endif")?;
    let replacement = format!("#if canImport({framework_name})\nimport {framework_name}\n#endif");

    for entry in directory.read_dir_utf8()? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let path = entry.path();
            if path.extension() == Some("swift") {
                let contents = std::fs::read_to_string(path)?;
                let new_contents = regex.replace_all(&contents, replacement.as_str());
                if new_contents != contents {
                    std::fs::write(path, new_contents.as_ref())?;
                }
            }
        }
    }

    Ok(())
}

/// Consolidates the modulemap files found in the source directory into a single
/// `module.modulemap` file in the destination directory.
///
/// The first modulemap file found is used as the base and all `header`
/// directives from the remaining modulemap files are spliced in after the
/// base's own header line. All other content in the base (e.g. `export *`,
/// `use` directives, and any future additions) is preserved verbatim.
fn consolidate_modulemap_files(source: &Utf8Path, destination: &Utf8Path) -> Result<()> {
    let mut base_contents: Option<String> = None;
    let mut extra_headers: Vec<String> = Vec::new();

    for entry in source.read_dir_utf8()? {
        let entry = entry?;

        if entry.file_type()?.is_file() {
            let path = entry.path();
            if path.extension() == Some("modulemap") {
                let contents = std::fs::read_to_string(path)?;
                if base_contents.is_none() {
                    base_contents = Some(contents);
                } else {
                    for line in contents.lines() {
                        if line.trim().starts_with("header ")
                            && !extra_headers.contains(&line.to_string())
                        {
                            extra_headers.push(line.to_string());
                        }
                    }
                }
                remove_file(path)?;
            }
        }
    }

    let base = base_contents.expect("No modulemap files found");

    // Rebuild the base line-by-line, renaming the module to MatrixSDKFFI and
    // inserting the extra headers immediately after the base's own header line.
    let mut lines: Vec<String> = Vec::new();
    let mut last_header_position: Option<usize> = None;

    for line in base.lines() {
        if line.starts_with("module ") {
            lines.push("module MatrixSDKFFI {".to_string());
        } else {
            if line.trim().starts_with("header ") {
                last_header_position = Some(lines.len());
            }
            lines.push(line.to_string());
        }
    }

    if let Some(last_header_position) = last_header_position {
        lines.splice(last_header_position + 1..last_header_position + 1, extra_headers);
    }

    let modulemap = lines.join("\n") + "\n";
    std::fs::write(destination.join("module.modulemap"), modulemap)?;
    Ok(())
}
