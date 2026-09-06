use std::{env, fs::create_dir_all, path::Path};

use camino::{Utf8Path, Utf8PathBuf};
use clap::{Args, Subcommand, ValueEnum};
use uniffi_bindgen::bindings::{GenerateOptions, TargetLanguage, generate};
use xshell::cmd;

use crate::{Result, sh, workspace};

struct PackageValues {
    name: &'static str,
    features: &'static str,
}

#[derive(ValueEnum, Clone)]
enum Package {
    CryptoSDK,
    FullSDK,
}

impl Package {
    fn values(self) -> PackageValues {
        match self {
            Package::CryptoSDK => PackageValues { name: "matrix-sdk-crypto-ffi", features: "" },
            Package::FullSDK => PackageValues {
                name: "matrix-sdk-ffi",
                features: "sentry,experimental-x509-identity-verification",
            },
        }
    }
}

#[derive(Args)]
pub struct KotlinArgs {
    #[clap(subcommand)]
    cmd: KotlinCommand,
}

#[derive(Subcommand)]
enum KotlinCommand {
    /// Builds the SDK for Android as an AAR.
    BuildAndroidLibrary {
        #[clap(value_enum, long)]
        package: Package,

        /// Build with the release profile
        #[clap(long)]
        release: bool,

        /// Build with a custom profile, takes precedence over `--release`
        #[clap(long)]
        profile: Option<String>,

        /// Build the given target only
        #[clap(long)]
        only_target: Option<String>,

        /// Move the generated files into the given src directory
        #[clap(long)]
        src_dir: Utf8PathBuf,
    },
}

impl KotlinArgs {
    pub fn run(self) -> Result<()> {
        let sh = sh();
        let _p = sh.push_dir(workspace::root_path()?);

        match self.cmd {
            KotlinCommand::BuildAndroidLibrary {
                release,
                profile,
                src_dir,
                only_target,
                package,
            } => {
                let profile = profile.as_deref().unwrap_or(if release { "release" } else { "dev" });
                build_android_library(profile, only_target, &src_dir, package)
            }
        }
    }
}

/// The environment variables an Android NDK installation is usually pointed at
/// with, in the order the NDK's own tooling looks at them.
const NDK_ENV_VARS: &[&str] = &["ANDROID_NDK_HOME", "ANDROID_NDK_ROOT", "ANDROID_NDK"];

/// Locates the Android NDK the build should use.
///
/// `cargo ndk` derives the whole toolchain from it, `CC` and `AR` included.
/// Without those, a dependency that compiles C code (`libsqlite3-sys` building
/// the bundled SQLite, for one) fails with a confusing "no C compiler found"
/// error even when the linker is set in the Cargo configuration, so check for
/// it up front and say what to set.
fn ndk_path() -> Result<String> {
    let configured: Vec<(&str, Option<String>)> =
        NDK_ENV_VARS.iter().map(|name| (*name, env::var(name).ok())).collect();

    resolve_ndk_path(&configured, |path| Path::new(path).is_dir())
}

/// Picks the NDK out of the variables that are set, rejecting one that doesn't
/// point at a directory.
///
/// Split out of [`ndk_path`] so it can be exercised without touching the
/// environment of the running process.
fn resolve_ndk_path(
    configured: &[(&str, Option<String>)],
    is_dir: impl Fn(&str) -> bool,
) -> Result<String> {
    for (name, path) in configured {
        let Some(path) = path else { continue };

        if !is_dir(path) {
            return Err(format!(
                "{name} is set to `{path}`, which is not a directory. Point it at an \
                 Android NDK installation."
            )
            .into());
        }

        return Ok(path.clone());
    }

    Err(format!(
        "No Android NDK found. Set one of {} to your NDK installation, for instance \
         `export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/<version>`. The whole toolchain, \
         the C compiler and archiver a bundled SQLite build needs among it, is derived \
         from it.",
        NDK_ENV_VARS.join(", ")
    )
    .into())
}

fn build_android_library(
    profile: &str,
    only_target: Option<String>,
    src_dir: &Utf8Path,
    package: Package,
) -> Result<()> {
    let ndk_path = ndk_path()?;
    println!("-- Using the Android NDK at {ndk_path}");

    let package_values = package.values();
    let package_name = package_values.name;
    let package_features = package_values.features;

    let jni_libs_dir = src_dir.join("jniLibs");
    let jni_libs_dir_str = jni_libs_dir.as_str();

    let kotlin_generated_dir = src_dir.join("kotlin");
    create_dir_all(kotlin_generated_dir.clone())?;

    let uniffi_lib_path = if let Some(target) = only_target {
        println!("-- Building for {target} [1/1]");
        build_for_android_target(
            target.as_str(),
            profile,
            jni_libs_dir_str,
            package_name,
            package_features,
        )?
    } else {
        println!("-- Building for x86_64-linux-android[1/4]");
        build_for_android_target(
            "x86_64-linux-android",
            profile,
            jni_libs_dir_str,
            package_name,
            package_features,
        )?;
        println!("-- Building for aarch64-linux-android[2/4]");
        build_for_android_target(
            "aarch64-linux-android",
            profile,
            jni_libs_dir_str,
            package_name,
            package_features,
        )?;
        println!("-- Building for armv7-linux-androideabi[3/4]");
        build_for_android_target(
            "armv7-linux-androideabi",
            profile,
            jni_libs_dir_str,
            package_name,
            package_features,
        )?;
        println!("-- Building for i686-linux-android[4/4]");
        build_for_android_target(
            "i686-linux-android",
            profile,
            jni_libs_dir_str,
            package_name,
            package_features,
        )?
    };

    println!("-- Generate uniffi files");
    generate_uniffi_bindings(&uniffi_lib_path, &kotlin_generated_dir)?;

    println!("-- All done and hunky dory. Enjoy!");
    Ok(())
}

fn generate_uniffi_bindings(library_path: &Utf8Path, ffi_generated_dir: &Utf8Path) -> Result<()> {
    println!("-- library_path = {library_path}");
    generate(GenerateOptions {
        languages: vec![TargetLanguage::Kotlin],
        source: library_path.to_path_buf(),
        out_dir: ffi_generated_dir.to_path_buf(),
        ..GenerateOptions::default()
    })?;
    Ok(())
}

fn build_for_android_target(
    target: &str,
    profile: &str,
    dest_dir: &str,
    package_name: &str,
    features: &str,
) -> Result<Utf8PathBuf> {
    let sh = sh();
    cmd!(
        sh,
        "cargo ndk --target {target} -o {dest_dir} build --profile {profile} --package {package_name} --features {features}"
    )
    .run()
    .map_err(|error| {
        format!(
            "Building for {target} failed: {error}\n\nThis build needs `cargo ndk`, which \
             derives the toolchain from the NDK. Install it with `cargo install cargo-ndk` \
             and add the target with `rustup target add {target}`."
        )
    })?;

    // The builtin dev profile has its files stored under target/debug, all
    // other targets have matching directory names
    let profile_dir_name = if profile == "dev" { "debug" } else { profile };
    let package_camel = package_name.replace('-', "_");
    let lib_name = format!("lib{package_camel}.so");
    Ok(workspace::target_path()?.join(target).join(profile_dir_name).join(lib_name))
}

#[cfg(test)]
mod tests {
    use super::{NDK_ENV_VARS, Package, resolve_ndk_path};

    /// Every variable unset, which is the state that produced the "no C
    /// compiler found" failure the build now refuses to start in.
    #[test]
    fn an_unconfigured_ndk_is_reported_with_the_variables_to_set() {
        let configured: Vec<(&str, Option<String>)> =
            NDK_ENV_VARS.iter().map(|name| (*name, None)).collect();

        let error = resolve_ndk_path(&configured, |_| true).unwrap_err().to_string();

        for name in NDK_ENV_VARS {
            assert!(error.contains(name), "{error} doesn't name {name}");
        }
    }

    #[test]
    fn the_first_variable_that_is_set_wins() {
        let configured = [
            ("ANDROID_NDK_HOME", None),
            ("ANDROID_NDK_ROOT", Some("/opt/ndk/root".to_owned())),
            ("ANDROID_NDK", Some("/opt/ndk/plain".to_owned())),
        ];

        assert_eq!(resolve_ndk_path(&configured, |_| true).unwrap(), "/opt/ndk/root");
    }

    /// A variable pointing at a path that isn't there is a configuration
    /// mistake, not a reason to fall through to the next one: falling through
    /// would build against an NDK the developer didn't mean to use.
    #[test]
    fn a_variable_pointing_at_no_directory_is_an_error_naming_it() {
        let configured = [
            ("ANDROID_NDK_HOME", Some("/nowhere".to_owned())),
            ("ANDROID_NDK_ROOT", Some("/opt/ndk/root".to_owned())),
        ];

        let error =
            resolve_ndk_path(&configured, |path| path != "/nowhere").unwrap_err().to_string();

        assert!(error.contains("ANDROID_NDK_HOME"), "{error} doesn't name the variable at fault");
        assert!(error.contains("/nowhere"), "{error} doesn't name the path that is wrong");
    }

    /// The crypto bindings are built without features; the full SDK needs its
    /// own. Getting this wrong builds a library the Kotlin bindings don't
    /// match, which only shows up at runtime.
    #[test]
    fn each_package_builds_the_crate_it_names() {
        let crypto = Package::CryptoSDK.values();
        assert_eq!(crypto.name, "matrix-sdk-crypto-ffi");
        assert_eq!(crypto.features, "");

        let full = Package::FullSDK.values();
        assert_eq!(full.name, "matrix-sdk-ffi");
        assert!(full.features.contains("sentry"));
    }
}
