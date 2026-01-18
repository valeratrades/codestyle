//! Rust check integration tests.
//!
//! Each module contains individual #[test] functions that can run in parallel,
//! enabling proper insta snapshot workflow (all failures at once, accept all at once).

mod embed_simple_vars;
mod impl_blocks;
mod insta_snapshots;
mod instrument;
mod loops;
mod no_chrono;
mod no_tokio_spawn;
mod test_fn_prefix;
mod use_bail;
mod utils;
