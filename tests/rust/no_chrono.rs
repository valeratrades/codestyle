use codestyle::test_fixture::{assert_check_passing, simulate_check};

use crate::utils::opts_for;

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("no_chrono")
}

#[test]
fn chrono_violations() {
	// Use statement (simple and nested)
	insta::assert_snapshot!(simulate_check(
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
	), @r"
	[no-chrono] /main.rs:1: Usage of `chrono` crate is disallowed in use statement. Use `jiff` crate instead.
	[no-chrono] /main.rs:2: Usage of `chrono` crate is disallowed in use statement. Use `jiff` crate instead.
	[no-chrono] /main.rs:4: Usage of `chrono` crate is disallowed. Use `jiff` crate instead.
	[no-chrono] /main.rs:4: Usage of `chrono` crate is disallowed. Use `jiff` crate instead.
	[no-chrono] /main.rs:9: Usage of `chrono` crate is disallowed. Use `jiff` crate instead.
	");
}

#[test]
fn non_chrono_passes() {
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
