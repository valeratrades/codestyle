use crate::utils::{assert_check_passing, opts_for, simulate_check};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("instrument")
}

#[test]
fn sync_function_without_instrument_passes() {
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
fn async_function_without_instrument_triggers_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		async fn async_no_instrument() {
			println!("hello");
		}
		"#,
		&opts(),
	), @"[instrument] /main.rs:1: No #[instrument] on async fn `async_no_instrument`");
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

#[test]
fn multiple_functions_only_async_without_instrument_caught() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn sync_one() {}
		async fn async_one() {}
		async fn async_two() {}
		#[instrument]
		async fn async_three() {}
		"#,
		&opts(),
	), @r#"
	[instrument] /main.rs:2: No #[instrument] on async fn `async_one`
	[instrument] /main.rs:3: No #[instrument] on async fn `async_two`
	"#);
}
