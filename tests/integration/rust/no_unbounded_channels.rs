use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("no_unbounded_channels")
}

// === Passing cases ===

#[test]
fn bounded_channels_pass() {
	assert_check_passing(
		r#"
		fn f() {
			let _a = tokio::sync::mpsc::channel::<u8>(64);
			let _b = std::sync::mpsc::sync_channel::<u8>(64);
			let _c = crossbeam_channel::bounded::<u8>(64);
			let _d = tokio::sync::oneshot::channel::<u8>();
		}
		"#,
		&opts(),
	);
}

#[test]
fn justified_with_marker_passes() {
	assert_check_passing(
		r#"
		fn f() {
			//UNBOUNDED: shutdown signal, at most one message per worker
			let _a = flume::unbounded::<u8>();
			let _b = tokio::sync::mpsc::unbounded_channel::<u8>(); // UNBOUNDED: drained every tick
		}
		"#,
		&opts(),
	);
}

// === Violation cases (no autofix) ===

#[test]
fn unbounded_constructors() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		use crossbeam_channel::unbounded;
		use std::sync::mpsc;

		fn f() {
			let _a = tokio::sync::mpsc::unbounded_channel::<u8>();
			let _b = unbounded::<u8>();
			let _c = mpsc::channel::<u8>();
		}
		"#,
		&opts(),
	), @"
	[no-unbounded-channels] /main.rs:5: `tokio::sync::mpsc::unbounded_channel` creates an unbounded queue
	HINT: bound it (`channel(cap)` / `bounded(cap)` / `sync_channel(cap)`), or justify with a `//UNBOUNDED` comment
	[no-unbounded-channels] /main.rs:6: `unbounded` creates an unbounded queue
	HINT: bound it (`channel(cap)` / `bounded(cap)` / `sync_channel(cap)`), or justify with a `//UNBOUNDED` comment
	[no-unbounded-channels] /main.rs:7: `mpsc::channel` creates an unbounded queue
	HINT: bound it (`channel(cap)` / `bounded(cap)` / `sync_channel(cap)`), or justify with a `//UNBOUNDED` comment
	");
}
