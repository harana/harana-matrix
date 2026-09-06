//! The macro used by the [`routes`](crate::routes) submodules to declare their
//! endpoints.

/// Declare the endpoints of an API.
///
/// The first field is the [`Api`](crate::endpoint::Api) the endpoints belong
/// to, the rest is a list of `"name" => RequestType` pairs, each of which may
/// carry `#[cfg]` attributes.
macro_rules! endpoints {
    (
        api: $api:expr,
        $( $( #[$attr:meta] )* $name:literal => $ty:ty ),* $(,)?
    ) => {
        /// The Matrix API the endpoints of this module belong to.
        pub const API: $crate::endpoint::Api = $api;

        /// Register a stub route for each endpoint of this module.
        pub(crate) fn register_stubs<S>(router: &mut $crate::MatrixRouter<S>)
        where
            S: Clone + Send + Sync + 'static,
        {
            $( $( #[$attr] )* router.register_stub::<$ty>($name); )*
        }

        /// Append the metadata of each endpoint of this module to `out`.
        pub(crate) fn describe(out: &mut Vec<$crate::EndpointMeta>) {
            $( $( #[$attr] )* out.push($crate::EndpointMeta::of::<$ty>($name, API)); )*
        }
    };
}
