use codestyle::{rust_checks::RustCheckOptions, test_fixture::simulate_check};

fn opts() -> RustCheckOptions {
	RustCheckOptions {
		loops: true,
		join_split_impls: false,
		impl_follows_type: false,
		embed_simple_vars: false,
		insta_inline_snapshot: false,
		instrument: false,
	}
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
	insta::assert_snapshot!(simulate_check(
		r#"
		fn good() {
			loop { //LOOP: justified reason
				break;
			}
		}
		"#,
		&opts(),
	), @"(no violations)");
}

#[test]
fn loop_with_comment_on_line_above_passes() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn good() {
			//LOOP: justified reason
			loop {
				break;
			}
		}
		"#,
		&opts(),
	), @"(no violations)");
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
	insta::assert_snapshot!(simulate_check(
		r#"
		fn other_loops() {
			while true { break; }
			for i in 0..10 { break; }
		}
		"#,
		&opts(),
	), @"(no violations)");
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
