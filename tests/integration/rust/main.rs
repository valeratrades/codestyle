//! Rust check integration tests.
//!
//! Each module contains individual #[test] functions that can run in parallel,
//! enabling proper insta snapshot workflow (all failures at once, accept all at once).

mod embed_simple_vars;
mod impl_blocks;
mod insta_snapshots;
mod instrument;
mod let_underscore_comment;
mod loops;
mod no_chrono;
mod no_tokio_spawn;
mod pub_first;
mod test_fn_prefix;
mod unwrap_or_comment;
mod use_bail;
mod utils;
