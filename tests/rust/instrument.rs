use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("instrument")
}

// === Passing cases ===

#[test]
fn sync_function_passes() {
	assert_check_passing(
		r#"
		fn sync_no_instrument() {
			println!("hello");
		}
		"#,
		&opts(),
	);
}

#[test]
fn async_function_with_instrument_passes() {
	assert_check_passing(
		r#"
		#[instrument]
		async fn with_instrument() {
			println!("hello");
		}
		"#,
		&opts(),
	);
}

#[test]
fn main_function_is_exempt() {
	assert_check_passing(
		r#"
		async fn main() {
			println!("hello");
		}
		"#,
		&opts(),
	);
}

#[test]
fn async_functions_in_utils_rs_are_exempt() {
	assert_check_passing(
		r#"
		//- /utils.rs
		async fn helper() {
			println!("hello");
		}
		"#,
		&opts(),
	);
}

// === Violation cases (no autofix) ===

#[test]
fn async_function_without_instrument() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		async fn async_no_instrument() {
			println!("hello");
		}
		"#,
		&opts(),
	), @"[instrument] /main.rs:1: No #[instrument] on async fn `async_no_instrument`");
}

#[test]
fn multiple_async_functions_without_instrument() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn sync_one() {}
		async fn async_one() {}
		async fn async_two() {}
		#[instrument]
		async fn async_three() {}
		"#,
		&opts(),
	), @"
	[instrument] /main.rs:2: No #[instrument] on async fn `async_one`
	[instrument] /main.rs:3: No #[instrument] on async fn `async_two`
	");
}
