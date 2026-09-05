use std::{
    collections::HashMap,
    fmt::{self, Write as _},
    sync::{Arc, Mutex},
};

use tracing_core::{Event, Field, Level, Subscriber, field::Visit};
use tracing_subscriber::{
    EnvFilter, Layer,
    layer::{Context, SubscriberExt as _},
    util::SubscriberInitExt as _,
};

/// Trait that can be used to forward Rust logs over FFI to a language specific
/// logger.
///
/// One method per severity, so a downstream logger can route by level, and the
/// message is kept separate from the event's structured fields rather than
/// being flattened into a single line.
#[matrix_sdk_ffi_macros::export(callback_interface)]
pub trait Logger: Send {
    /// Called for every log statement recorded at the `ERROR` level.
    ///
    /// `target` is the module path the statement was recorded under, `message`
    /// is its message and `fields` holds the remaining structured fields of
    /// the statement, keyed by field name.
    fn error(&self, target: String, message: String, fields: HashMap<String, String>);

    /// Called for every log statement recorded at the `WARN` level. See
    /// [`Logger::error`] for the arguments.
    fn warn(&self, target: String, message: String, fields: HashMap<String, String>);

    /// Called for every log statement recorded at the `INFO` level. See
    /// [`Logger::error`] for the arguments.
    fn info(&self, target: String, message: String, fields: HashMap<String, String>);

    /// Called for every log statement recorded at the `DEBUG` level. See
    /// [`Logger::error`] for the arguments.
    fn debug(&self, target: String, message: String, fields: HashMap<String, String>);

    /// Called for every log statement recorded at the `TRACE` level. See
    /// [`Logger::error`] for the arguments.
    fn trace(&self, target: String, message: String, fields: HashMap<String, String>);
}

/// A tracing layer dispatching events to a foreign [`Logger`].
#[derive(Clone)]
pub struct LoggerWrapper {
    inner: Arc<Mutex<Box<dyn Logger>>>,
}

impl<S: Subscriber> Layer<S> for LoggerWrapper {
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let target = metadata.target().to_owned();

        let mut visitor = EventVisitor::default();
        event.record(&mut visitor);

        let EventVisitor { message, fields } = visitor;

        let logger = self.inner.lock().unwrap();

        match *metadata.level() {
            Level::ERROR => logger.error(target, message, fields),
            Level::WARN => logger.warn(target, message, fields),
            Level::INFO => logger.info(target, message, fields),
            Level::DEBUG => logger.debug(target, message, fields),
            Level::TRACE => logger.trace(target, message, fields),
        }
    }
}

/// Splits an event into its `message` field and the rest of its fields.
#[derive(Default)]
struct EventVisitor {
    message: String,
    fields: HashMap<String, String>,
}

impl EventVisitor {
    fn record(&mut self, field: &Field, value: fmt::Arguments<'_>) {
        if field.name() == "message" {
            let _ = write!(self.message, "{value}");
        } else {
            self.fields.insert(field.name().to_owned(), value.to_string());
        }
    }
}

impl Visit for EventVisitor {
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

/// Set the logger that should be used to forward Rust logs over FFI.
#[matrix_sdk_ffi_macros::export]
pub fn set_logger(logger: Box<dyn Logger>) {
    let logger = LoggerWrapper { inner: Arc::new(Mutex::new(logger)) };

    let filter = EnvFilter::from_default_env()
        .add_directive(
            "matrix_sdk_crypto=trace".parse().expect("Can't parse logging filter directive"),
        )
        .add_directive(
            "matrix_sdk_sqlite=debug".parse().expect("Can't parse logging filter directive"),
        );

    let _ = tracing_subscriber::registry().with(filter).with(logger).try_init();
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use tracing::subscriber;
    use tracing_subscriber::{layer::SubscriberExt as _, registry};

    use super::{Logger, LoggerWrapper};

    #[derive(Debug, PartialEq)]
    struct Recorded {
        level: &'static str,
        target: String,
        message: String,
        fields: HashMap<String, String>,
    }

    #[derive(Clone, Default)]
    struct Recorder(Arc<Mutex<Vec<Recorded>>>);

    impl Recorder {
        fn push(
            &self,
            level: &'static str,
            target: String,
            message: String,
            fields: HashMap<String, String>,
        ) {
            self.0.lock().unwrap().push(Recorded { level, target, message, fields });
        }
    }

    impl Logger for Recorder {
        fn error(&self, target: String, message: String, fields: HashMap<String, String>) {
            self.push("error", target, message, fields);
        }

        fn warn(&self, target: String, message: String, fields: HashMap<String, String>) {
            self.push("warn", target, message, fields);
        }

        fn info(&self, target: String, message: String, fields: HashMap<String, String>) {
            self.push("info", target, message, fields);
        }

        fn debug(&self, target: String, message: String, fields: HashMap<String, String>) {
            self.push("debug", target, message, fields);
        }

        fn trace(&self, target: String, message: String, fields: HashMap<String, String>) {
            self.push("trace", target, message, fields);
        }
    }

    #[test]
    fn test_events_are_routed_by_level_with_their_fields() {
        let recorder = Recorder::default();
        let boxed: Box<dyn Logger> = Box::new(recorder.clone());
        let wrapper = LoggerWrapper { inner: Arc::new(Mutex::new(boxed)) };

        subscriber::with_default(registry().with(wrapper), || {
            tracing::warn!(target: "test_target", user_id = "@alice:localhost", "careful");
            tracing::debug!(target: "test_target", "just a message");
        });

        let recorded = recorder.0.lock().unwrap();
        assert_eq!(recorded.len(), 2);

        assert_eq!(recorded[0].level, "warn");
        assert_eq!(recorded[0].target, "test_target");
        assert_eq!(recorded[0].message, "careful");
        assert_eq!(recorded[0].fields.get("user_id").map(String::as_str), Some("@alice:localhost"));

        assert_eq!(recorded[1].level, "debug");
        assert_eq!(recorded[1].message, "just a message");
        assert!(recorded[1].fields.is_empty());
    }

    /// Each severity has a method of its own, so a host can map them onto its
    /// own logger. All five have to be reachable.
    #[test]
    fn test_every_level_reaches_its_own_method() {
        let recorder = Recorder::default();
        let boxed: Box<dyn Logger> = Box::new(recorder.clone());
        let wrapper = LoggerWrapper { inner: Arc::new(Mutex::new(boxed)) };

        subscriber::with_default(registry().with(wrapper), || {
            tracing::error!(target: "test_levels", "an error");
            tracing::warn!(target: "test_levels", "a warning");
            tracing::info!(target: "test_levels", "some info");
            tracing::debug!(target: "test_levels", "a debug line");
            tracing::trace!(target: "test_levels", "a trace line");
        });

        let recorded = recorder.0.lock().unwrap();
        let levels: Vec<_> = recorded.iter().map(|entry| entry.level).collect();
        assert_eq!(levels, ["error", "warn", "info", "debug", "trace"]);
    }

    /// The fields arrive separately from the message, keyed by name, rather
    /// than flattened into the line as the single `log` method did.
    #[test]
    fn test_fields_are_delivered_separately_from_the_message() {
        let recorder = Recorder::default();
        let boxed: Box<dyn Logger> = Box::new(recorder.clone());
        let wrapper = LoggerWrapper { inner: Arc::new(Mutex::new(boxed)) };

        subscriber::with_default(registry().with(wrapper), || {
            tracing::info!(
                target: "test_fields",
                room_id = "!room:localhost",
                attempts = 3,
                retried = true,
                "sending"
            );
        });

        let recorded = recorder.0.lock().unwrap();
        assert_eq!(recorded.len(), 1);

        let entry = &recorded[0];
        assert_eq!(entry.message, "sending", "the fields must not be folded into the message");
        assert_eq!(entry.fields.get("room_id").map(String::as_str), Some("!room:localhost"));
        assert_eq!(entry.fields.get("attempts").map(String::as_str), Some("3"));
        assert_eq!(entry.fields.get("retried").map(String::as_str), Some("true"));
    }
}
