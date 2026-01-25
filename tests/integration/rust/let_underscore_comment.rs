use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("let_underscore_comment")
}

// === Passing cases ===

#[test]
fn let_underscore_with_inline_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			let _ = some_result(); //IGNORED_ERROR: intentionally ignoring return value
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&opts(),
	);
}

#[test]
fn let_underscore_with_comment_on_line_above_passes() {
	assert_check_passing(
		r#"
		fn good() {
			//IGNORED_ERROR: we don't care about this result
			let _ = some_result();
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&opts(),
	);
}

#[test]
fn named_underscore_binding_no_comment_needed() {
	assert_check_passing(
		r#"
		fn good() {
			let _unused = 42;
		}
		"#,
		&opts(),
	);
}

#[test]
fn destructuring_with_underscore_no_comment_needed() {
	assert_check_passing(
		r#"
		fn good() {
			let (a, _) = (1, 2);
			let Foo { x, .. } = Foo { x: 1, y: 2 };
		}
		struct Foo { x: i32, y: i32 }
		"#,
		&opts(),
	);
}

// === Violation cases (no autofix) ===

#[test]
fn let_underscore_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn bad() {
			let _ = some_result();
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&opts(),
	), @"
	[let-underscore-comment] /main.rs:2: `let _ = ...` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}

#[test]
fn nested_let_underscore_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn nested() {
			if true {
				let _ = some_result();
			}
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&opts(),
	), @"
	[let-underscore-comment] /main.rs:3: `let _ = ...` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}

#[test]
fn let_underscore_in_closure() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn with_closure() {
			let f = || {
				let _ = some_result();
			};
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
		&opts(),
	), @"
	[let-underscore-comment] /main.rs:3: `let _ = ...` without `//IGNORED_ERROR` comment
	HINT: could the pattern be allowing to continue with corrupted state? Error out properly or explain why it's part of the intended logic.
	");
}
