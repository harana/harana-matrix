use std::{
    collections::HashMap,
    fmt::{self, Write as _},
    sync::Arc,
};

use tracing::{
    Event, Level, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{EnvFilter, layer::Context, prelude::*, registry::LookupSpan};

/// The severity of a log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum LogLevel {
    /// Something went wrong and needs to be looked at.
    Error,
    /// Something unexpected happened, but we carried on.
    Warn,
    /// A noteworthy event in the normal course of operation.
    Info,
    /// Detail that is useful when diagnosing a problem.
    Debug,
    /// Very fine-grained detail, usually far too much to keep on.
    Trace,
}

impl From<&Level> for LogLevel {
    fn from(value: &Level) -> Self {
        match *value {
            Level::ERROR => Self::Error,
            Level::WARN => Self::Warn,
            Level::INFO => Self::Info,
            Level::DEBUG => Self::Debug,
            Level::TRACE => Self::Trace,
        }
    }
}

/// Trait that can be used to forward Rust logs over FFI to a language specific
/// logger.
#[matrix_sdk_ffi_macros::export(callback_interface)]
pub trait Logger: Send + Sync {
    /// Called every time the Rust side wants to post a log line.
    ///
    /// The level, the message and the structured fields are passed separately,
    /// so a downstream logger can route by severity and read the fields
    /// instead of having to parse one flattened string.
    ///
    /// # Arguments
    ///
    /// * `level` - How severe this log line is.
    /// * `target` - The module path the log line came from, e.g.
    ///   `matrix_sdk_crypto::machine`.
    /// * `message` - The log message itself.
    /// * `fields` - The structured fields attached to the log line and to the
    ///   spans it was emitted in, formatted as strings.
    fn log(
        &self,
        level: LogLevel,
        target: String,
        message: String,
        fields: HashMap<String, String>,
    );
}

/// A [`tracing_subscriber`] layer that hands each event to a [`Logger`].
struct LoggerLayer {
    inner: Arc<dyn Logger>,
}

impl<S> tracing_subscriber::Layer<S> for LoggerLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        // Fields recorded on the spans the event was emitted in are just as useful as
        // the ones on the event itself, and they are what carries the context (the
        // session ID, the user ID, …) that a flattened string used to hide.
        if let Some(scope) = ctx.event_scope(event) {
            for span in scope.from_root() {
                if let Some(fields) = span.extensions().get::<SpanFields>() {
                    for (name, value) in &fields.0 {
                        visitor.fields.entry(name.clone()).or_insert_with(|| value.clone());
                    }
                }
            }
        }

        let FieldVisitor { message, fields } = visitor;

        self.inner.log(
            event.metadata().level().into(),
            event.metadata().target().to_owned(),
            message,
            fields,
        );
    }

    fn on_new_span(
        &self,
        attrs: &tracing::span::Attributes<'_>,
        id: &tracing::span::Id,
        ctx: Context<'_, S>,
    ) {
        let mut visitor = FieldVisitor::default();
        attrs.record(&mut visitor);

        if let Some(span) = ctx.span(id) {
            span.extensions_mut().insert(SpanFields(visitor.fields));
        }
    }

    fn on_record(
        &self,
        id: &tracing::span::Id,
        values: &tracing::span::Record<'_>,
        ctx: Context<'_, S>,
    ) {
        let mut visitor = FieldVisitor::default();
        values.record(&mut visitor);

        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();

            if let Some(fields) = extensions.get_mut::<SpanFields>() {
                fields.0.extend(visitor.fields);
            } else {
                extensions.insert(SpanFields(visitor.fields));
            }
        }
    }
}

/// The fields recorded on a span, kept in the span's extensions so that events
/// emitted inside it can pick them up.
struct SpanFields(HashMap<String, String>);

/// Collects the `message` field of an event separately from the rest, which is
/// what lets us hand the message and the structured data over on their own.
#[derive(Default)]
struct FieldVisitor {
    message: String,
    fields: HashMap<String, String>,
}

impl FieldVisitor {
    fn record(&mut self, field: &Field, value: String) {
        if field.name() == "message" {
            if !self.message.is_empty() {
                self.message.push(' ');
            }

            let _ = self.message.write_str(&value);
        } else {
            self.fields.insert(field.name().to_owned(), value);
        }
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.record(field, format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.record(field, value.to_owned());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.record(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.record(field, value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.record(field, value.to_string());
    }

    fn record_error(&mut self, field: &Field, value: &(dyn std::error::Error + 'static)) {
        self.record(field, value.to_string());
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

    let layer = LoggerLayer { inner: Arc::from(logger) };

    let _ = tracing_subscriber::registry().with(filter).with(layer).try_init();
}
