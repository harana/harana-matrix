`clippy::future_not_send` is denied crate-wide on non-WASM targets.
`NotificationProcessor::push_notification_from_event_if` now needs a `Sync`
event type and a `Send` predicate, which is what makes its future `Send`.
