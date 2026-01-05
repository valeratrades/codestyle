use crate::utils::{assert_check_passing, opts_for, simulate_check};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("loops")
}

#[test]
fn loop_without_comment_triggers_violation() {
	insta::assert_snapshot!(simulate_check(
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
fn nested_loop_without_comment() {
	insta::assert_snapshot!(simulate_check(
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

#[test]
fn loop_inside_closure() {
	insta::assert_snapshot!(simulate_check(
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
	insta::assert_snapshot!(simulate_check(
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
