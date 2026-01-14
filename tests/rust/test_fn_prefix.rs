use crate::utils::{assert_check_passing, opts_for, simulate_check, simulate_format};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("test_fn_prefix")
}

#[test]
fn fn_with_test_prefix_triggers_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		#[test]
		fn test_something() {}
		"#,
		&opts(),
	), @"[test-fn-prefix] /main.rs:2: test function `test_something` has redundant `test_` prefix");
}

#[test]
fn rstest_fn_with_test_prefix_triggers_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		#[rstest]
		fn test_something() {}
		"#,
		&opts(),
	), @"[test-fn-prefix] /main.rs:2: test function `test_something` has redundant `test_` prefix");
}

#[test]
fn fn_without_prefix_passes() {
	assert_check_passing(
		r#"
		#[test]
		fn something() {}
		"#,
		&opts(),
	);
}

#[test]
fn rstest_fn_without_prefix_passes() {
	assert_check_passing(
		r#"
		#[rstest]
		fn something() {}
		"#,
		&opts(),
	);
}

#[test]
fn autofix_strips_test_prefix() {
	insta::assert_snapshot!(simulate_format(
		r#"
		#[test]
		fn test_something() {}
		"#,
		&opts(),
	), @"
	#[test]
	fn something() {}
	");
}

#[test]
fn autofix_strips_test_prefix_rstest() {
	insta::assert_snapshot!(simulate_format(
		r#"
		#[rstest]
		fn test_foo_bar() {}
		"#,
		&opts(),
	), @"
	#[rstest]
	fn foo_bar() {}
	");
}

#[test]
fn multiple_test_fns_with_prefix() {
	insta::assert_snapshot!(simulate_check(
		r#"
		#[test]
		fn test_first() {}

		#[test]
		fn test_second() {}

		#[test]
		fn third_is_fine() {}
		"#,
		&opts(),
	), @"
	[test-fn-prefix] /main.rs:2: test function `test_first` has redundant `test_` prefix
	[test-fn-prefix] /main.rs:5: test function `test_second` has redundant `test_` prefix
	");
}

#[test]
fn tokio_test_with_prefix_triggers() {
	insta::assert_snapshot!(simulate_check(
		r#"
		#[tokio::test]
		fn test_async_thing() {}
		"#,
		&opts(),
	), @"[test-fn-prefix] /main.rs:2: test function `test_async_thing` has redundant `test_` prefix");
}
