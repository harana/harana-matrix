use std::{collections::HashMap, fmt};

use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{EnvFilter, Layer, layer::Context, prelude::*, registry::LookupSpan};

/// Trait that can be used to forward Rust logs over FFI to a language specific
/// logger.
///
/// One method per log level, so that the receiving logger can route the log
/// line by severity, and the structured fields of the event are handed over
/// separately from the message rather than flattened into it.
#[matrix_sdk_ffi_macros::export(callback_interface)]
pub trait Logger: Send + Sync {
    /// Called for a log event at the `error` level.
    fn error(&self, event: LogEvent);
    /// Called for a log event at the `warn` level.
    fn warn(&self, event: LogEvent);
    /// Called for a log event at the `info` level.
    fn info(&self, event: LogEvent);
    /// Called for a log event at the `debug` level.
    fn debug(&self, event: LogEvent);
    /// Called for a log event at the `trace` level.
    fn trace(&self, event: LogEvent);
}

/// A single log event.
#[derive(Clone, Debug, uniffi::Record)]
pub struct LogEvent {
    /// The module path the event was logged from, e.g.
    /// `matrix_sdk_crypto::machine`.
    pub target: String,
    /// The message of the event, i.e. everything that was not a named field.
    ///
    /// May be empty for an event which only carries fields.
    pub message: String,
    /// The structured fields of the event, keyed by field name.
    ///
    /// The values are rendered with their `Debug` representation, apart from
    /// strings, which are passed through as they are.
    pub fields: HashMap<String, String>,
}

/// Collects the message and the fields of a `tracing` event.
#[derive(Default)]
struct EventVisitor {
    message: String,
    fields: HashMap<String, String>,
}

impl Visit for EventVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_owned();
        } else {
            self.fields.insert(field.name().to_owned(), value.to_owned());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        let value = format!("{value:?}");

        if field.name() == "message" {
            self.message = value;
        } else {
            self.fields.insert(field.name().to_owned(), value);
        }
    }
}

struct LoggerLayer {
    inner: Box<dyn Logger>,
}

impl<S> Layer<S> for LoggerLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let metadata = event.metadata();
        let event = LogEvent {
            target: metadata.target().to_owned(),
            message: visitor.message,
            fields: visitor.fields,
        };

        match *metadata.level() {
            Level::ERROR => self.inner.error(event),
            Level::WARN => self.inner.warn(event),
            Level::INFO => self.inner.info(event),
            Level::DEBUG => self.inner.debug(event),
            Level::TRACE => self.inner.trace(event),
        }
    }
}

/// Set the logger that should be used to forward Rust logs over FFI.
#[matrix_sdk_ffi_macros::export]
pub fn set_logger(logger: Box<dyn Logger>) {
    let filter = EnvFilter::from_default_env()
        .add_directive(
            "matrix_sdk_crypto=trace".parse().expect("Can't parse logging filter directive"),
        )
        .add_directive(
            "matrix_sdk_sqlite=debug".parse().expect("Can't parse logging filter directive"),
        );

    let _ = tracing_subscriber::registry()
        .with(LoggerLayer { inner: logger }.with_filter(filter))
        .try_init();
}
