Blocking SQLite work no longer goes through `deadpool`'s Tokio integration. It
runs on the blocking pool of whichever runtime the SDK was configured with
(see `sdk_common::runtime`), so the crate works on runtimes other than
Tokio. `connection::RUNTIME` is gone as a result.
