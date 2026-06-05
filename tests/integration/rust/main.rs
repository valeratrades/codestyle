//! Rust check integration tests.
//!
//! Each module contains individual #[test] functions that can run in parallel,
//! enabling proper insta snapshot workflow (all failures at once, accept all at once).

mod cargo_dep_ordering;
mod embed_simple_vars;
mod ignored_error;
mod impl_blocks;
mod inline_default;
mod insta_snapshots;
mod instrument;
mod loops;
mod no_chrono;
mod no_tokio_spawn;
mod prefer_ahash;
mod prefer_default_over_bare_new;
mod pub_first;
mod skip_attribute;
mod test_fn_prefix;
mod too_explicit;
mod unconventional_new;
mod use_bail;
mod utils;
mod workspace_dep_hoisting;
