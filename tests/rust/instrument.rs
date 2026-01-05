use codestyle::{
	rust_checks::{self, Violation, instrument},
	test_fixture::Fixture,
};

fn check_violations(code: &str, expected: &[&str]) {
	let fixture = Fixture::parse(code);
	let temp = fixture.write_to_tempdir();

	let file_infos = rust_checks::collect_rust_files(&temp.root);
	let violations: Vec<Violation> = file_infos.iter().flat_map(|info| instrument::check_instrument(info)).collect();
	let messages: Vec<&str> = violations.iter().map(|v| v.message.as_str()).collect();

	assert_eq!(messages, expected, "Violations mismatch for fixture:\n{code}");
}

fn check_ok(code: &str) {
	check_violations(code, &[]);
}

fn main() {
	// sync function without #[instrument] passes (only async is checked)
	check_ok(
		r#"
		fn sync_no_instrument() {
			println!("hello");
		}
		"#,
	);

	// async function without #[instrument] triggers violation
	check_violations(
		r#"
		async fn async_no_instrument() {
			println!("hello");
		}
		"#,
		&["No #[instrument] on async fn `async_no_instrument`"],
	);

	// async function with #[instrument] passes
	check_ok(
		r#"
		#[instrument]
		async fn with_instrument() {
			println!("hello");
		}
		"#,
	);

	// main function is exempt (even if async)
	check_ok(
		r#"
		async fn main() {
			println!("hello");
		}
		"#,
	);

	// async functions in utils.rs are exempt
	check_violations(
		r#"
		//- /utils.rs
		async fn helper() {
			println!("hello");
		}
		"#,
		&[], // exempt in utils.rs
	);

	// multiple functions - only async without instrument are caught
	check_violations(
		r#"
		fn sync_one() {}
		async fn async_one() {}
		async fn async_two() {}
		#[instrument]
		async fn async_three() {}
		"#,
		&["No #[instrument] on async fn `async_one`", "No #[instrument] on async fn `async_two`"],
	);

	println!("All instrument tests passed!");
}
