// Copyright 2026 The Matrix.org Foundation C.I.C.
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
// See the License for the specific language governing permissions and
// limitations under the License.

//! A pluggable abstraction over where the bindings' logs and traces go.
//!
//! The bindings install their own `tracing` subscriber in
//! [`init_platform`](super::init_platform), wiring up a text log (to stdout,
//! logcat, or rotated files) and, when built with the `sentry` feature, a
//! Sentry sink. Neither of those can be replaced from the outside, and until
//! this module existed nothing else could be added either.
//!
//! A [`TelemetryProvider`] contributes an extra `tracing` layer to that
//! subscriber, so an application embedding these bindings from Rust can send
//! the SDK's spans and events to an OpenTelemetry pipeline, its own crash
//! reporter, or a structured log shipper, without the SDK knowing about any of
//! them. Providers are installed process-wide with [`set_telemetry_providers`]
//! and must be in place *before* `init_platform` runs, since that is when the
//! subscriber is built.
//!
//! # Example
//!
//! ```ignore
//! use std::sync::Arc;
//!
//! use matrix_sdk_ffi::platform::telemetry::{
//!     TelemetryLayer, TelemetryProvider, set_telemetry_providers,
//! };
//!
//! #[derive(Debug)]
//! struct OpenTelemetry;
//!
//! impl TelemetryProvider for OpenTelemetry {
//!     fn layer(&self) -> Option<TelemetryLayer> {
//!         Some(Box::new(tracing_opentelemetry::layer().with_tracer(my_tracer())))
//!     }
//! }
//!
//! set_telemetry_providers(vec![Arc::new(OpenTelemetry)]).unwrap();
//! matrix_sdk_ffi::platform::init_platform(config, false)?;
//! ```

use std::{fmt, sync::Arc};

use tracing_subscriber::{Layer, Registry};

use crate::error::ClientError;

/// One `tracing` layer contributed by a [`TelemetryProvider`].
///
/// It is attached directly to the subscriber's registry, underneath the log
/// level filter the bindings build from the
/// [`TracingConfiguration`](super::TracingConfiguration), so it sees the same
/// events the text log does.
pub type TelemetryLayer = Box<dyn Layer<Registry> + Send + Sync + 'static>;

/// A sink for the SDK's logs and traces, beyond the text log and Sentry the
/// bindings set up themselves.
pub trait TelemetryProvider: fmt::Debug + Send + Sync + 'static {
    /// The `tracing` layer this provider wants attached to the subscriber.
    ///
    /// Called once, while the subscriber is being built. Returning `None`
    /// contributes nothing, which is how a provider disables itself based on
    /// its own configuration.
    fn layer(&self) -> Option<TelemetryLayer>;
}

#[cfg(not(target_family = "wasm"))]
mod imp {
    use std::sync::{Arc, OnceLock};

    use super::TelemetryProvider;

    pub(super) static PROVIDERS: OnceLock<Vec<Arc<dyn TelemetryProvider>>> = OnceLock::new();
}

#[cfg(target_family = "wasm")]
mod imp {
    use std::{cell::RefCell, sync::Arc};

    use super::TelemetryProvider;

    thread_local! {
        pub(super) static PROVIDERS: RefCell<Option<Vec<Arc<dyn TelemetryProvider>>>> =
            const { RefCell::new(None) };
    }
}

/// Install the telemetry providers the bindings should feed.
///
/// This has to be called before
/// [`init_platform`](super::init_platform), which is when the subscriber is
/// built, and can only be called once. A second call, or a call after the
/// subscriber exists, returns an error and changes nothing.
///
/// Not calling it at all is fine: the bindings then set up only the text log,
/// plus Sentry when it is configured.
pub fn set_telemetry_providers(
    providers: Vec<Arc<dyn TelemetryProvider>>,
) -> Result<(), ClientError> {
    #[cfg(not(target_family = "wasm"))]
    {
        imp::PROVIDERS.set(providers).map_err(|_| ClientError::Generic {
            msg: "telemetry providers have already been installed".to_owned(),
            details: None,
        })
    }

    #[cfg(target_family = "wasm")]
    {
        imp::PROVIDERS.with(|cell| {
            let mut cell = cell.borrow_mut();

            if cell.is_some() {
                return Err(ClientError::Generic {
                    msg: "telemetry providers have already been installed".to_owned(),
                    details: None,
                });
            }

            *cell = Some(providers);
            Ok(())
        })
    }
}

/// The layers the installed providers contribute, in the order they were
/// installed.
pub(super) fn layers() -> Vec<TelemetryLayer> {
    #[cfg(not(target_family = "wasm"))]
    let providers: Vec<Arc<dyn TelemetryProvider>> =
        imp::PROVIDERS.get().cloned().unwrap_or_default();

    #[cfg(target_family = "wasm")]
    let providers: Vec<Arc<dyn TelemetryProvider>> =
        imp::PROVIDERS.with(|cell| cell.borrow().clone().unwrap_or_default());

    providers.iter().filter_map(|provider| provider.layer()).collect()
}
