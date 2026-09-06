// Copyright 2025 The Matrix.org Foundation C.I.C.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for that specific language governing permissions and
// limitations under the License.

//! Forwarding of Rust panics to the host application.
//!
//! A panic inside the SDK unwinds out of the FFI call that triggered it, and
//! the host only sees a generic failure. Registering a [`PanicListener`] lets
//! the host observe the panic itself, so it can offer a crash report or shut
//! down cleanly.

use std::{
    backtrace::{Backtrace, BacktraceStatus},
    panic,
    sync::{Arc, OnceLock, RwLock},
};

use matrix_sdk_common::{SendOutsideWasm, SyncOutsideWasm};

/// Details about a panic that happened inside the SDK.
#[derive(Clone, uniffi::Record)]
pub struct PanicDetails {
    /// The panic message, as produced by the panicking code.
    ///
    /// Empty if the payload wasn't a string, which cannot happen for panics
    /// raised by the standard `panic!` machinery.
    pub message: String,

    /// Source file the panic originated from, if known.
    pub file: Option<String>,

    /// Line in `file` the panic originated from, if known.
    pub line: Option<u32>,

    /// Column in `file` the panic originated from, if known.
    pub column: Option<u32>,

    /// Name of the thread that panicked, if it had one.
    pub thread: Option<String>,

    /// A backtrace captured at the panic site.
    ///
    /// Only present when backtraces are available and enabled for the build;
    /// `init_platform` enables them.
    pub backtrace: Option<String>,
}

/// A listener notified whenever the SDK panics.
///
/// The callback runs on the panicking thread, while the panic is being
/// handled, so it must not panic itself and should return quickly. The panic
/// still unwinds afterwards.
#[matrix_sdk_ffi_macros::export(callback_interface)]
pub trait PanicListener: SyncOutsideWasm + SendOutsideWasm {
    /// Called with the details of a panic that just happened.
    fn on_panic(&self, details: PanicDetails);
}

type SharedListener = Arc<RwLock<Option<Arc<dyn PanicListener>>>>;

fn listener() -> &'static SharedListener {
    static LISTENER: OnceLock<SharedListener> = OnceLock::new();
    LISTENER.get_or_init(|| Arc::new(RwLock::new(None)))
}

/// Registers the listener that will be notified about SDK panics, replacing
/// any previously registered one.
///
/// Passing `None` removes the current listener.
///
/// This can be called before or after `init_platform`; the hook that feeds it
/// is installed on first use and chains to whatever panic hook was in place,
/// so the existing panic logging keeps working.
#[matrix_sdk_ffi_macros::export]
pub fn set_panic_listener(listener_to_set: Option<Box<dyn PanicListener>>) {
    install_hook();
    *listener().write().unwrap() = listener_to_set.map(Arc::from);
}

/// Installs the panic hook that forwards panics to the registered listener.
///
/// Chains to the hook that was previously installed, so panic logging set up
/// by `init_platform` is preserved. Idempotent: only the first call installs a
/// hook.
pub(crate) fn install_hook() {
    static INSTALLED: OnceLock<()> = OnceLock::new();

    INSTALLED.get_or_init(|| {
        panic::set_hook(chaining_hook(panic::take_hook()));
    });
}

/// The hook [`install_hook`] installs: notifies the listener, and runs
/// `previous` first so nothing that was already being reported is lost.
fn chaining_hook(previous: PanicHook) -> PanicHook {
    Box::new(move |info| {
        // Run the pre-existing hook first, so its output is emitted even if the
        // listener misbehaves.
        previous(info);

        let listener = listener().read().unwrap().clone();
        let Some(listener) = listener else { return };

        listener.on_panic(details_from(info));
    })
}

type PanicHook = Box<dyn Fn(&panic::PanicHookInfo<'_>) + Sync + Send + 'static>;

fn details_from(info: &panic::PanicHookInfo<'_>) -> PanicDetails {
    let payload = info.payload();
    let message = payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_default();

    let location = info.location();

    let backtrace = Backtrace::capture();
    let backtrace =
        (backtrace.status() == BacktraceStatus::Captured).then(|| backtrace.to_string());

    PanicDetails {
        message,
        file: location.map(|location| location.file().to_owned()),
        line: location.map(panic::Location::line),
        column: location.map(panic::Location::column),
        thread: std::thread::current().name().map(ToOwned::to_owned),
        backtrace,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        panic,
        sync::{
            Arc, Mutex, MutexGuard, OnceLock,
            atomic::{AtomicUsize, Ordering},
        },
    };

    use super::{
        PanicDetails, PanicListener, chaining_hook, install_hook, listener, set_panic_listener,
    };

    /// The panic hook and the registered listener are both process-wide, so
    /// the tests that swap them out must not run at the same time.
    fn serialise() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|error| error.into_inner())
    }

    #[derive(Default)]
    struct Recorder {
        calls: AtomicUsize,
        last: Mutex<Option<PanicDetails>>,
    }

    impl PanicListener for Recorder {
        fn on_panic(&self, details: PanicDetails) {
            self.calls.fetch_add(1, Ordering::SeqCst);
            *self.last.lock().unwrap() = Some(details);
        }
    }

    /// Runs `f`, which is expected to panic, with the panic output silenced.
    fn catch_panic(f: impl FnOnce() + std::panic::UnwindSafe) {
        let result = std::panic::catch_unwind(f);
        assert!(result.is_err(), "the closure was expected to panic");
    }

    #[test]
    fn test_listener_receives_panic_details() {
        let _guard = serialise();
        let recorder = Arc::new(Recorder::default());

        install_hook();
        *listener().write().unwrap() = Some(recorder.clone());

        let line_of_the_panic = line!() + 1;
        catch_panic(|| panic!("boom"));

        assert_eq!(recorder.calls.load(Ordering::SeqCst), 1);

        let details = recorder.last.lock().unwrap().clone().expect("a panic was recorded");
        assert_eq!(details.message, "boom");
        assert!(details.file.expect("the panic location is known").ends_with("panic.rs"));
        assert_eq!(details.line, Some(line_of_the_panic));
        assert!(details.column.is_some_and(|column| column > 0));
        assert!(details.thread.is_some(), "the panicking thread is named in a test binary");

        // Clearing the listener stops the notifications.
        set_panic_listener(None);
        catch_panic(|| panic!("second"));
        assert_eq!(recorder.calls.load(Ordering::SeqCst), 1);
    }

    /// A `String` payload, which is what `panic!("{}", ...)` produces, reads
    /// the same way as the `&str` one a literal produces.
    #[test]
    fn test_a_formatted_panic_message_is_reported() {
        let _guard = serialise();
        let recorder = Arc::new(Recorder::default());

        install_hook();
        *listener().write().unwrap() = Some(recorder.clone());

        catch_panic(|| panic!("{} went wrong", 1));

        *listener().write().unwrap() = None;

        let details = recorder.last.lock().unwrap().clone().expect("a panic was recorded");
        assert_eq!(details.message, "1 went wrong");
    }

    /// `init_platform` installs a hook that logs panics. Ours has to chain to
    /// it, or registering a listener would silently stop the panic logging.
    #[test]
    fn test_the_hook_that_was_already_installed_still_runs() {
        let _guard = serialise();
        let recorder = Arc::new(Recorder::default());
        *listener().write().unwrap() = Some(recorder.clone());

        let previous_calls = Arc::new(AtomicUsize::new(0));
        let hook = chaining_hook({
            let previous_calls = previous_calls.clone();
            Box::new(move |_| {
                previous_calls.fetch_add(1, Ordering::SeqCst);
            })
        });

        let installed = panic::take_hook();
        panic::set_hook(hook);

        catch_panic(|| panic!("chained"));

        panic::set_hook(installed);
        *listener().write().unwrap() = None;

        assert_eq!(previous_calls.load(Ordering::SeqCst), 1, "the previous hook was not run");
        assert_eq!(recorder.calls.load(Ordering::SeqCst), 1, "the listener was not notified");
    }
}
