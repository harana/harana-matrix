# ruma-client (vendored)

A fork of [ruma-client](https://github.com/ruma/ruma-client), inlined into this
workspace. Upstream revision `873fefc`, MIT licensed (see `LICENSE`).

A minimal Matrix client on top of `ruma`. Nothing else in this workspace depends
on it; `matrix-sdk` has its own HTTP client.

## Differences from upstream

- Only the `reqwest` HTTP backend is vendored. The `hyper` backend and its five
  optional dependencies are dropped; this workspace standardises on `reqwest`.
- The `client-api` feature is gone. It gated `Client` on `ruma`'s client-server
  API types, which the vendored `ruma` always provides.
- The build script is gone. It validated `tls-*` features that had already been
  renamed out of the manifest, so it never did anything.
