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

//! Delivery of log events to the consumer of the bindings.
//!
//! The SDK owns its log writers, which is enough to get logs into a file or
//! into logcat, but not to route them into a platform logger or a test
//! harness. A [`LogEventListener`] receives every log event that passes the
//! filter configured in `init_platform`, in addition to (not instead of) the
//! writers set up there.

use std::{
    cell::Cell,
    fmt::{self, Write as _},
    sync::{Arc, OnceLock, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use matrix_sdk_common::{SendOutsideWasm, SyncOutsideWasm};
use tracing_core::{Event, Field, Subscriber, field::Visit};
use tracing_subscriber::{Layer, layer::Context};

use super::tracing::LogLevel;

/// A single log event, as delivered to a [`LogEventListener`].
#[derive(Clone, uniffi::Record)]
pub struct LogEvent {
    /// Severity of the event.
    pub level: LogLevel,

    /// Target the event was logged with, usually a module path.
    pub target: String,

    /// The event's message, with any additional structured fields appended as
    /// `key=value` pairs.
    pub message: String,

    /// Source file the event was logged from, if known.
    pub file: Option<String>,

    /// Line in `file` the event was logged from, if known.
    pub line: Option<u32>,

    /// When the event was observed, in milliseconds since the Unix epoch.
    pub timestamp: crate::utils::Timestamp,
}

/// A listener receiving the SDK's log events.
///
/// The callback runs on the thread that logged the event, so it must return
/// quickly. Log statements made by the callback itself are not delivered back
/// to it.
#[matrix_sdk_ffi_macros::export(callback_interface)]
pub trait LogEventListener: SyncOutsideWasm + SendOutsideWasm {
    /// Called for every log event that passes the configured filter.
    fn on_log_event(&self, event: LogEvent);
}

type SharedListener = Arc<RwLock<Option<Arc<dyn LogEventListener>>>>;

fn listener() -> &'static SharedListener {
    static LISTENER: OnceLock<SharedListener> = OnceLock::new();
    LISTENER.get_or_init(|| Arc::new(RwLock::new(None)))
}

/// Registers the listener that will receive the SDK's log events, replacing
/// any previously registered one.
///
/// Passing `None` removes the current listener.
///
/// Events are only delivered once `init_platform` has been called, and only
/// for the targets and levels its configuration enables.
#[matrix_sdk_ffi_macros::export]
pub fn set_log_event_listener(listener_to_set: Option<Box<dyn LogEventListener>>) {
    *listener().write().unwrap() = listener_to_set.map(Arc::from);
}

thread_local! {
    /// Whether this thread is currently inside the listener callback, in which
    /// case events it logs are dropped rather than fed back into it.
    static IN_CALLBACK: Cell<bool> = const { Cell::new(false) };
}

/// The tracing layer forwarding events to the registered [`LogEventListener`].
pub(crate) struct LogEventLayer;

impl<S: Subscriber> Layer<S> for LogEventLayer {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if IN_CALLBACK.get() {
            return;
        }

        // Cheap check first: no listener, nothing to format.
        let listener = listener().read().unwrap().clone();
        let Some(listener) = listener else { return };

        let metadata = event.metadata();

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let log_event = LogEvent {
            level: LogLevel::from(*metadata.level()),
            target: metadata.target().to_owned(),
            message: visitor.into_message(),
            file: metadata.file().map(ToOwned::to_owned),
            line: metadata.line(),
            timestamp: now_millis(),
        };

        IN_CALLBACK.set(true);
        listener.on_log_event(log_event);
        IN_CALLBACK.set(false);
    }
}

fn now_millis() -> crate::utils::Timestamp {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);

    crate::utils::Timestamp::from(u64::try_from(millis).unwrap_or(u64::MAX))
}

/// Collects an event's `message` field, with its remaining fields appended as
/// `key=value` pairs.
#[derive(Default)]
struct MessageVisitor {
    message: String,
    fields: String,
}

impl MessageVisitor {
    fn into_message(mut self) -> String {
        if !self.fields.is_empty() {
            if !self.message.is_empty() {
                self.message.push(' ');
            }
            self.message.push_str(&self.fields);
        }

        self.message
    }

    fn record(&mut self, field: &Field, value: fmt::Arguments<'_>) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value}");
        } else {
            if !self.fields.is_empty() {
                self.fields.push(' ');
            }
            let _ = write!(self.fields, "{}={value}", field.name());
        }
    }
}

impl Visit for MessageVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record(field, format_args!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record(field, format_args!("{value}"));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record(field, format_args!("{value}"));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record(field, format_args!("{value}"));
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record(field, format_args!("{value}"));
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.record(field, format_args!("{value}"));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

    use tracing::{Level, subscriber};
    use tracing_subscriber::{Layer as _, layer::SubscriberExt as _, registry};

    use super::{LogEvent, LogEventLayer, LogEventListener, listener};
    use crate::platform::tracing::LogLevel;

    /// The registered listener is global, so the tests that swap it out must
    /// not run at the same time.
    fn serialise() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(())).lock().unwrap_or_else(|error| error.into_inner())
    }

    #[derive(Default)]
    struct Recorder {
        events: Mutex<Vec<LogEvent>>,
    }

    impl LogEventListener for Recorder {
        fn on_log_event(&self, event: LogEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    #[test]
    fn test_events_are_forwarded_to_the_listener() {
        let _guard = serialise();

        let recorder = Arc::new(Recorder::default());
        *listener().write().unwrap() = Some(recorder.clone());

        let subscriber = registry().with(
            LogEventLayer
                .with_filter(tracing_subscriber::filter::LevelFilter::from_level(Level::INFO)),
        );

        subscriber::with_default(subscriber, || {
            tracing::info!(target: "test_target", answer = 42, "hello");
            // Filtered out, so never delivered.
            tracing::debug!(target: "test_target", "not delivered");
        });

        *listener().write().unwrap() = None;

        let events = recorder.events.lock().unwrap();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.target, "test_target");
        assert_eq!(event.message, "hello answer=42");
        assert!(matches!(event.level, LogLevel::Info));
        assert!(event.file.as_deref().expect("the callsite is known").ends_with("log_listener.rs"));
    }

    #[test]
    fn test_the_listener_does_not_receive_its_own_logs() {
        let _guard = serialise();

        struct Reentrant;

        impl LogEventListener for Reentrant {
            fn on_log_event(&self, _: LogEvent) {
                // Would recurse forever without the re-entrancy guard.
                tracing::info!(target: "test_reentrant", "from the callback");
            }
        }

        *listener().write().unwrap() = Some(Arc::new(Reentrant));

        let subscriber = registry().with(LogEventLayer);
        subscriber::with_default(subscriber, || {
            tracing::info!(target: "test_reentrant", "trigger");
        });

        *listener().write().unwrap() = None;
    }
}
