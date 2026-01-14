use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("loops")
}

// === Passing cases ===

#[test]
fn loop_with_inline_comment_passes() {
	assert_check_passing(
		r#"
		fn good() {
			loop { //LOOP: justified reason
				break;
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn loop_with_comment_on_line_above_passes() {
	assert_check_passing(
		r#"
		fn good() {
			//LOOP: justified reason
			loop {
				break;
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn while_and_for_loops_dont_trigger() {
	assert_check_passing(
		r#"
		fn other_loops() {
			while true { break; }
			for i in 0..10 { break; }
		}
		"#,
		&opts(),
	);
}

// === Violation cases (no autofix) ===

#[test]
fn loop_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn bad() {
			loop {
				break;
			}
		}
		"#,
		&opts(),
	), @"[loop-comment] /main.rs:2: Endless loop without `//LOOP` comment");
}

#[test]
fn nested_loop_without_comment() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn nested() {
			if true {
				loop {
					break;
				}
			}
		}
		"#,
		&opts(),
	), @"[loop-comment] /main.rs:3: Endless loop without `//LOOP` comment");
}

#[test]
fn loop_inside_closure() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn with_closure() {
			let f = || {
				loop {
					break;
				}
			};
		}
		"#,
		&opts(),
	), @"[loop-comment] /main.rs:3: Endless loop without `//LOOP` comment");
}

#[test]
fn loop_inside_async_block() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn with_async() {
			let f = async {
				loop {
					break;
				}
			};
		}
		"#,
		&opts(),
	), @"[loop-comment] /main.rs:3: Endless loop without `//LOOP` comment");
}
