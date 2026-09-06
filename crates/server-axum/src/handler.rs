//! The trait tying a handler function to the endpoint it serves.

use axum::{handler::Handler, routing::MethodRouter};
use ruma::api::IncomingRequest;

use crate::{extract::Ruma, router::method_filter};

/// An axum handler that serves a Matrix endpoint.
///
/// This is implemented for every async function that takes a [`Ruma<R>`](Ruma)
/// as its last argument, and up to 15 other axum extractors before it. The
/// endpoint the handler serves is [`Endpoint`](Self::Endpoint), the `R` of that
/// argument, which is what
/// [`MatrixRouter::handle()`](crate::MatrixRouter::handle) routes to it.
pub trait RumaHandler<S, T>: Handler<T, S> {
    /// The endpoint this handler serves.
    type Endpoint: IncomingRequest + 'static;

    /// Turn this handler into a router answering the HTTP method of the
    /// endpoint.
    fn into_method_router(self) -> MethodRouter<S>;
}

macro_rules! impl_ruma_handler {
    ( $( $ty:ident ),* ) => {
        #[allow(non_snake_case, unused_parens)]
        impl<F, S, R, $( $ty, )*> RumaHandler<S, ( $( $ty, )* Ruma<R>, )> for F
        where
            F: Handler<( $( $ty, )* Ruma<R>, ), S>,
            R: IncomingRequest + 'static,
            S: Clone + Send + Sync + 'static,
            $( $ty: 'static, )*
        {
            type Endpoint = R;

            fn into_method_router(self) -> MethodRouter<S> {
                axum::routing::on(method_filter::<R>(), self)
            }
        }
    };
}

impl_ruma_handler!();
impl_ruma_handler!(T1);
impl_ruma_handler!(T1, T2);
impl_ruma_handler!(T1, T2, T3);
impl_ruma_handler!(T1, T2, T3, T4);
impl_ruma_handler!(T1, T2, T3, T4, T5);
impl_ruma_handler!(T1, T2, T3, T4, T5, T6);
impl_ruma_handler!(T1, T2, T3, T4, T5, T6, T7);
impl_ruma_handler!(T1, T2, T3, T4, T5, T6, T7, T8);
impl_ruma_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9);
impl_ruma_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10);
impl_ruma_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11);
impl_ruma_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12);
impl_ruma_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13);
impl_ruma_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14);
impl_ruma_handler!(T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12, T13, T14, T15);
