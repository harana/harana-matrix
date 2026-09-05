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

//! Delivering log events to the bindings consumer.
//!
//! The SDK owns its log writers, so a consumer that wants to route logs to a
//! platform logger or collect them in a test harness had no way to receive
//! them. [`set_log_event_listener`] registers a callback that receives every
//! log statement that passes the filter set up by `initPlatform`, in addition
//! to (not instead of) the existing writers.

use std::{
    cell::Cell,
    fmt::{self, Write as _},
    sync::{Arc, RwLock},
    time::{SystemTime, UNIX_EPOCH},
};

use matrix_sdk_common::{SendOutsideWasm, SyncOutsideWasm};
use tracing_core::{Event, Field, Level, Subscriber, field::Visit};
use tracing_subscriber::{Layer, layer::Context, registry::LookupSpan};

use super::tracing::LogLevel;

/// A log statement emitted by the SDK.
#[derive(Clone, uniffi::Record)]
pub struct LogEvent {
    /// How severe the statement is.
    pub level: LogLevel,

    /// The target it was emitted with, usually a module path, e.g.
    /// `matrix_sdk::client`.
    pub target: String,

    /// The formatted message, with the event's other fields appended as
    /// `key=value` pairs.
    pub message: String,

    /// The source file the statement comes from, if the callsite recorded one.
    pub file: Option<String>,

    /// The line in `file` the statement comes from, if the callsite recorded
    /// one.
    pub line: Option<u32>,

    /// When the statement was emitted, in milliseconds since the Unix epoch.
    pub timestamp: u64,
}

/// A consumer of [`LogEvent`]s.
#[matrix_sdk_ffi_macros::export(callback_interface)]
pub trait LogEventListener: SyncOutsideWasm + SendOutsideWasm {
    fn call(&self, event: LogEvent);
}

static LISTENER: RwLock<Option<Arc<dyn LogEventListener>>> = RwLock::new(None);

thread_local! {
    /// Whether this thread is currently inside the listener.
    ///
    /// A listener that logs would otherwise be called from within its own call,
    /// forever.
    static IN_LISTENER: Cell<bool> = const { Cell::new(false) };
}

/// Receive every log statement the SDK emits.
///
/// The listener is called for the statements that pass the filter configured by
/// `initPlatform`, and does not replace the file or stdout writers set up
/// there. Setting a listener replaces the previously set one.
#[matrix_sdk_ffi_macros::export]
pub fn set_log_event_listener(listener: Box<dyn LogEventListener>) {
    *LISTENER.write().unwrap() = Some(Arc::from(listener));
}

/// Stop delivering log statements to the listener set by
/// [`set_log_event_listener`].
#[matrix_sdk_ffi_macros::export]
pub fn clear_log_event_listener() {
    *LISTENER.write().unwrap() = None;
}

/// The tracing layer forwarding events to the registered [`LogEventListener`].
pub(super) struct LogEventListenerLayer;

impl<S> Layer<S> for LogEventListenerLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if IN_LISTENER.get() {
            return;
        }

        // Take a copy of the listener rather than holding the lock across the call:
        // the callback runs consumer code of unknown duration.
        let Some(listener) = LISTENER.read().unwrap().clone() else {
            return;
        };

        let metadata = event.metadata();

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);

        let log_event = LogEvent {
            level: level_to_log_level(*metadata.level()),
            target: metadata.target().to_owned(),
            message: visitor.message,
            file: metadata.file().map(ToOwned::to_owned),
            line: metadata.line(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_millis() as u64)
                .unwrap_or_default(),
        };

        IN_LISTENER.set(true);
        listener.call(log_event);
        IN_LISTENER.set(false);
    }
}

fn level_to_log_level(level: Level) -> LogLevel {
    match level {
        Level::ERROR => LogLevel::Error,
        Level::WARN => LogLevel::Warn,
        Level::INFO => LogLevel::Info,
        Level::DEBUG => LogLevel::Debug,
        Level::TRACE => LogLevel::Trace,
    }
}

/// Renders an event's fields into a single message.
///
/// The `message` field comes first and unadorned, as it is what a log line
/// usually is; the remaining fields follow as `key=value` pairs.
#[derive(Default)]
struct MessageVisitor {
    message: String,
}

impl MessageVisitor {
    fn record(&mut self, field: &Field, value: fmt::Arguments<'_>) {
        if field.name() == "message" {
            // The message is written first, so that it reads like a log line even when
            // it's recorded after another field.
            let rest = std::mem::take(&mut self.message);
            let _ = write!(&mut self.message, "{value}");

            if !rest.is_empty() {
                let _ = write!(&mut self.message, " {rest}");
            }
        } else {
            if !self.message.is_empty() {
                self.message.push(' ');
            }

            let _ = write!(&mut self.message, "{}={value}", field.name());
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

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.record(field, format_args!("{value}"));
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tracing_subscriber::{Layer as _, layer::SubscriberExt as _};

    use super::{
        LogEvent, LogEventListener, LogEventListenerLayer, clear_log_event_listener,
        set_log_event_listener,
    };
    use crate::platform::tracing::LogLevel;

    #[derive(Default)]
    struct Collector(Arc<Mutex<Vec<LogEvent>>>);

    impl LogEventListener for Collector {
        fn call(&self, event: LogEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    // One test, because the listener is a process-wide global: separate tests would
    // clobber each other's listener.
    #[test]
    fn test_log_event_listener() {
        let collected = Arc::new(Mutex::new(Vec::new()));
        set_log_event_listener(Box::new(Collector(collected.clone())));

        let subscriber =
            tracing_subscriber::registry().with(LogEventListenerLayer.with_filter(
                tracing_subscriber::filter::LevelFilter::from_level(tracing::Level::INFO),
            ));

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "test_target", user = "alice", "hello");
            // Below the filter, so it never reaches the layer.
            tracing::debug!(target: "test_target", "not delivered");
        });

        {
            let collected = collected.lock().unwrap();
            assert_eq!(collected.len(), 1);

            let event = &collected[0];
            assert_eq!(event.target, "test_target");
            assert_eq!(event.message, "hello user=alice");
            assert!(matches!(event.level, LogLevel::Info));
            assert!(event.line.is_some());
            assert!(event.timestamp > 0);
        }

        // A listener that logs is not called from within its own call, forever.
        struct Logging;

        impl LogEventListener for Logging {
            fn call(&self, _event: LogEvent) {
                tracing::info!(target: "test_target", "from the listener");
            }
        }

        set_log_event_listener(Box::new(Logging));

        let subscriber = tracing_subscriber::registry().with(LogEventListenerLayer);

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "test_target", "hello");
        });

        clear_log_event_listener();

        // Once cleared, nothing else is delivered.
        let subscriber = tracing_subscriber::registry().with(LogEventListenerLayer);
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "test_target", "after clearing");
        });

        assert_eq!(collected.lock().unwrap().len(), 1);
    }
}
