use codestyle::test_fixture::{assert_check_passing, simulate_check};

use crate::utils::opts_for;

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("no_chrono")
}

#[test]
fn chrono_use_statement_is_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		use chrono::DateTime;

		fn main() {}
		"#,
		&opts(),
	), @"[no-chrono] /main.rs:1: Usage of `chrono` crate is disallowed in use statement. Use `jiff` crate instead.");
}

#[test]
fn chrono_full_path_in_type_is_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn get_time() -> chrono::DateTime<chrono::Utc> {
			todo!()
		}
		"#,
		&opts(),
	), @r"
	[no-chrono] /main.rs:1: Usage of `chrono` crate is disallowed. Use `jiff` crate instead.
	[no-chrono] /main.rs:1: Usage of `chrono` crate is disallowed. Use `jiff` crate instead.
	");
}

#[test]
fn chrono_in_expression_is_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn main() {
			let now = chrono::Utc::now();
		}
		"#,
		&opts(),
	), @"[no-chrono] /main.rs:2: Usage of `chrono` crate is disallowed. Use `jiff` crate instead.");
}

#[test]
fn chrono_nested_use_is_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		use chrono::{DateTime, Utc};

		fn main() {}
		"#,
		&opts(),
	), @"[no-chrono] /main.rs:1: Usage of `chrono` crate is disallowed in use statement. Use `jiff` crate instead.");
}

#[test]
fn non_chrono_crate_passes() {
	assert_check_passing(
		r#"
		use jiff::Timestamp;
		use std::time::Duration;

		fn main() {
			let _ts = Timestamp::now();
		}
		"#,
		&opts(),
	);
}

#[test]
fn chrono_like_name_but_not_crate_passes() {
	assert_check_passing(
		r#"
		mod chrono_helper {
			pub fn helper() {}
		}

		fn main() {
			chrono_helper::helper();
		}
		"#,
		&opts(),
	);
}

#[test]
fn multiple_chrono_violations() {
	insta::assert_snapshot!(simulate_check(
		r#"
		use chrono::DateTime;
		use chrono::Utc;

		fn main() {
			let _now = chrono::Local::now();
		}
		"#,
		&opts(),
	), @r"
	[no-chrono] /main.rs:1: Usage of `chrono` crate is disallowed in use statement. Use `jiff` crate instead.
	[no-chrono] /main.rs:2: Usage of `chrono` crate is disallowed in use statement. Use `jiff` crate instead.
	[no-chrono] /main.rs:5: Usage of `chrono` crate is disallowed. Use `jiff` crate instead.
	");
}
