use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("unwrap_or_comment")
}

// === Passing cases ===

#[test]
fn unwrap_or_with_inline_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0); //IGNORED_ERROR: default for missing config
		}
		"#,
		&opts(),
	);
}

#[test]
fn unwrap_or_with_comment_on_line_above_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let x: Option<i32> = None;
			//IGNORED_ERROR: default for missing config
			let y = x.unwrap_or(0);
		}
		"#,
		&opts(),
	);
}

#[test]
fn unwrap_or_default_with_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let x: Option<String> = None;
			let y = x.unwrap_or_default(); //IGNORED_ERROR: empty string is fine
		}
		"#,
		&opts(),
	);
}

#[test]
fn unwrap_or_else_with_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let x: Option<i32> = None;
			let y = x.unwrap_or_else(|| 42); //IGNORED_ERROR: lazy default
		}
		"#,
		&opts(),
	);
}

#[test]
fn result_unwrap_or_with_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let x: Result<i32, ()> = Err(());
			let y = x.unwrap_or(0); //IGNORED_ERROR: fallback on error
		}
		"#,
		&opts(),
	);
}

// === Violation cases (no autofix) ===

#[test]
fn unwrap_or_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn bad() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
		}
		"#,
		&opts(),
	), @"
	[unwrap-or-comment] /main.rs:3: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}

#[test]
fn unwrap_or_default_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn bad() {
			let x: Option<String> = None;
			let y = x.unwrap_or_default();
		}
		"#,
		&opts(),
	), @"
	[unwrap-or-comment] /main.rs:3: `unwrap_or_default` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}

#[test]
fn unwrap_or_else_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn bad() {
			let x: Option<i32> = None;
			let y = x.unwrap_or_else(|| 42);
		}
		"#,
		&opts(),
	), @"
	[unwrap-or-comment] /main.rs:3: `unwrap_or_else` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}

#[test]
fn nested_unwrap_or_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn nested() {
			if true {
				let x: Option<i32> = None;
				let y = x.unwrap_or(0);
			}
		}
		"#,
		&opts(),
	), @"
	[unwrap-or-comment] /main.rs:4: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}

#[test]
fn unwrap_or_in_closure() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn with_closure() {
			let f = || {
				let x: Option<i32> = None;
				x.unwrap_or(0)
			};
		}
		"#,
		&opts(),
	), @"
	[unwrap-or-comment] /main.rs:4: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}

#[test]
fn chained_unwrap_or_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn chained() {
			let x: Option<Option<i32>> = None;
			let y = x.unwrap_or(None).unwrap_or(0);
		}
		"#,
		&opts(),
	), @"
	[unwrap-or-comment] /main.rs:3: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	[unwrap-or-comment] /main.rs:3: `unwrap_or` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}
