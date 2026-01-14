use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("no_chrono")
}

// === Passing cases ===

#[test]
fn jiff_and_std_time_passes() {
	assert_check_passing(
		r#"
		use jiff::Timestamp;
		use std::time::Duration;

		mod chrono_helper {
			pub fn helper() {}
		}

		fn main() {
			let _ts = Timestamp::now();
			chrono_helper::helper();
		}
		"#,
		&opts(),
	);
}

// === Violation cases (no autofix) ===

#[test]
fn chrono_use_statements_and_paths() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		use chrono::DateTime;
		use chrono::{Utc, Local};

		fn get_time() -> chrono::DateTime<chrono::Utc> {
			todo!()
		}

		fn main() {
			let _now = chrono::Local::now();
		}
		"#,
		&opts(),
	), @"
	[no-chrono] /main.rs:1: Usage of `chrono` crate is disallowed in use statement. Use `jiff` crate instead.
	[no-chrono] /main.rs:2: Usage of `chrono` crate is disallowed in use statement. Use `jiff` crate instead.
	[no-chrono] /main.rs:4: Usage of `chrono` crate is disallowed. Use `jiff` crate instead.
	[no-chrono] /main.rs:4: Usage of `chrono` crate is disallowed. Use `jiff` crate instead.
	[no-chrono] /main.rs:9: Usage of `chrono` crate is disallowed. Use `jiff` crate instead.
	");
}
