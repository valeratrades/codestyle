//! Test utilities for codestyle integration tests.

use std::path::Path;

use codestyle::rust_checks::{self, RustCheckOptions, Violation};
use v_fixtures::{Fixture, render_fixture};

pub(crate) fn opts_for(check: &str) -> RustCheckOptions {
	RustCheckOptions {
		instrument: check == "instrument",
		join_split_impls: check == "join_split_impls",
		impl_folds: check == "impl_folds",
		impl_follows_type: check == "impl_follows_type",
		loops: check == "loops",
		embed_simple_vars: check == "embed_simple_vars",
		insta_inline_snapshot: check == "insta_inline_snapshot",
		no_chrono: check == "no_chrono",
		no_tokio_spawn: check == "no_tokio_spawn",
		use_bail: check == "use_bail",
		test_fn_prefix: check == "test_fn_prefix",
	}
}

/// Assert that a fixture passes all enabled checks (no violations).
#[track_caller]
pub(crate) fn assert_check_passing(fixture_str: &str, opts: &RustCheckOptions) {
	let fixture = Fixture::parse(fixture_str);
	let temp = fixture.write_to_tempdir();
	let violations = collect_violations(&temp.root, opts, false);

	if !violations.is_empty() {
		let violation_msgs: Vec<String> = violations
			.iter()
			.map(|v| {
				let relative_path = v.file.strip_prefix(temp.root.to_str().unwrap_or("")).unwrap_or(&v.file);
				let relative_path = relative_path.trim_start_matches('/');
				format!("[{}] /{relative_path}:{}: {}", v.rule, v.line, v.message)
			})
			.collect();
		panic!("expected no violations, but found {}:\n{}", violations.len(), violation_msgs.join("\n"));
	}
}

/// Unified test primitive for violation cases.
/// Runs both assert mode and format mode, returning a combined snapshot.
/// Also verifies that the formatted output passes the check.
///
/// Returns a string in the format:
/// ```text
/// # Assert mode
/// {violations}
///
/// # Format mode
/// {formatted_output}
/// ```
#[track_caller]
pub(crate) fn test_case(fixture_str: &str, opts: &RustCheckOptions) -> String {
	let fixture = Fixture::parse(fixture_str);

	// Assert mode: collect violations
	let temp_assert = fixture.write_to_tempdir();
	let violations = collect_violations(&temp_assert.root, opts, false);

	assert!(!violations.is_empty(), "test_case called but no violations found - use assert_check_passing instead");

	let assert_out = violations
		.iter()
		.map(|v| {
			let relative_path = v.file.strip_prefix(temp_assert.root.to_str().unwrap_or("")).unwrap_or(&v.file);
			let relative_path = relative_path.trim_start_matches('/');
			format!("[{}] /{relative_path}:{}: {}", v.rule, v.line, v.message)
		})
		.collect::<Vec<_>>()
		.join("\n");

	// Format mode: apply fixes
	let temp_format = fixture.write_to_tempdir();
	rust_checks::run_format(&temp_format.root, opts);
	let result = temp_format.read_all_from_disk();
	let format_out = render_fixture(&result);

	// Verify formatted output passes the check
	let format_violations = collect_violations(&temp_format.root, opts, false);
	if !format_violations.is_empty() {
		let violation_msgs: Vec<String> = format_violations
			.iter()
			.map(|v| {
				let relative_path = v.file.strip_prefix(temp_format.root.to_str().unwrap_or("")).unwrap_or(&v.file);
				let relative_path = relative_path.trim_start_matches('/');
				format!("[{}] /{relative_path}:{}: {}", v.rule, v.line, v.message)
			})
			.collect();
		panic!(
			"formatted output still has {} violation(s):\n{}\n\nFormatted output:\n{format_out}",
			format_violations.len(),
			violation_msgs.join("\n")
		);
	}

	format!("# Assert mode\n{assert_out}\n\n# Format mode\n{format_out}")
}

/// Test primitive for violations without autofix.
/// Runs assert mode only and returns violations as a string for snapshot testing.
/// Use this for rules that don't have autofix capability.
#[track_caller]
pub(crate) fn test_case_assert_only(fixture_str: &str, opts: &RustCheckOptions) -> String {
	let fixture = Fixture::parse(fixture_str);
	let temp = fixture.write_to_tempdir();

	let violations = collect_violations(&temp.root, opts, false);

	assert!(!violations.is_empty(), "test_case_assert_only called but no violations found - use assert_check_passing instead");

	violations
		.iter()
		.map(|v| {
			let relative_path = v.file.strip_prefix(temp.root.to_str().unwrap_or("")).unwrap_or(&v.file);
			let relative_path = relative_path.trim_start_matches('/');
			format!("[{}] /{relative_path}:{}: {}", v.rule, v.line, v.message)
		})
		.collect::<Vec<_>>()
		.join("\n")
}

fn collect_violations(root: &Path, opts: &RustCheckOptions, is_format_mode: bool) -> Vec<Violation> {
	use codestyle::rust_checks::{
		embed_simple_vars, impl_folds, impl_follows_type, insta_snapshots, instrument, join_split_impls, loops, no_chrono, no_tokio_spawn, test_fn_prefix, use_bail,
	};

	let file_infos = rust_checks::collect_rust_files(root);
	let mut violations = Vec::new();

	for info in &file_infos {
		if opts.instrument {
			violations.extend(instrument::check_instrument(info));
		}
		if opts.loops {
			violations.extend(loops::check_loops(info));
		}
		if let Some(ref tree) = info.syntax_tree {
			if opts.join_split_impls {
				violations.extend(join_split_impls::check(&info.path, &info.contents, tree));
			}
			if opts.impl_folds {
				violations.extend(impl_folds::check(&info.path, &info.contents, tree));
			}
			if opts.impl_follows_type {
				violations.extend(impl_follows_type::check(&info.path, &info.contents, tree));
			}
			if opts.embed_simple_vars {
				violations.extend(embed_simple_vars::check(&info.path, &info.contents, tree));
			}
			if opts.insta_inline_snapshot {
				violations.extend(insta_snapshots::check(&info.path, &info.contents, tree, is_format_mode));
			}
			if opts.no_chrono {
				violations.extend(no_chrono::check(&info.path, &info.contents, tree));
			}
			if opts.no_tokio_spawn {
				violations.extend(no_tokio_spawn::check(&info.path, &info.contents, tree));
			}
			if opts.use_bail {
				violations.extend(use_bail::check(&info.path, &info.contents, tree));
			}
			if opts.test_fn_prefix {
				violations.extend(test_fn_prefix::check(&info.path, &info.contents, tree));
			}
		}
	}

	violations
}
