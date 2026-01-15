use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("test_fn_prefix")
}

// === Passing cases ===

#[test]
fn test_fn_without_prefix_passes() {
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

// === Violation cases ===

#[test]
fn test_fn_with_test_prefix() {
	insta::assert_snapshot!(test_case(
		r#"
		#[test]
		fn test_something() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[test-fn-prefix] /main.rs:2: test function `test_something` has redundant `test_` prefix

	# Format mode
	#[test]
	fn something() {}
	");
}

#[test]
fn rstest_fn_with_test_prefix() {
	insta::assert_snapshot!(test_case(
		r#"
		#[rstest]
		fn test_foo_bar() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[test-fn-prefix] /main.rs:2: test function `test_foo_bar` has redundant `test_` prefix

	# Format mode
	#[rstest]
	fn foo_bar() {}
	");
}

#[test]
fn multiple_test_fns_with_prefix() {
	insta::assert_snapshot!(test_case(
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
	# Assert mode
	[test-fn-prefix] /main.rs:2: test function `test_first` has redundant `test_` prefix
	[test-fn-prefix] /main.rs:5: test function `test_second` has redundant `test_` prefix

	# Format mode
	#[test]
	fn first() {}

	#[test]
	fn second() {}

	#[test]
	fn third_is_fine() {}
	");
}

#[test]
fn tokio_test_with_prefix() {
	insta::assert_snapshot!(test_case(
		r#"
		#[tokio::test]
		fn test_async_thing() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[test-fn-prefix] /main.rs:2: test function `test_async_thing` has redundant `test_` prefix

	# Format mode
	#[tokio::test]
	fn async_thing() {}
	");
}
