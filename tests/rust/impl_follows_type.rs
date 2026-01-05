use codestyle::test_fixture::{assert_check_passing, simulate_check, simulate_format};

use crate::utils::opts_for;

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("impl_follows_type")
}

#[test]
fn impl_immediately_after_struct_passes() {
	assert_check_passing(
		r#"
		struct Foo {
			x: i32,
		}
		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}
		"#,
		&opts(),
	);
}

#[test]
fn impl_with_gap_triggers_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		struct Foo {
			x: i32,
		}


		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}
		"#,
		&opts(),
	), @"[impl-follows-type] /main.rs:6: `impl Foo` should follow type definition (line 3), but has 2 blank line(s)");
}

#[test]
fn trait_impl_is_exempt() {
	assert_check_passing(
		r#"
		struct Foo;


		impl Default for Foo {
			fn default() -> Self { Foo }
		}
		"#,
		&opts(),
	);
}

#[test]
fn enum_works_same_as_struct() {
	insta::assert_snapshot!(simulate_check(
		r#"
		enum Bar {
			A,
			B,
		}


		impl Bar {
			fn is_a(&self) -> bool { matches!(self, Self::A) }
		}
		"#,
		&opts(),
	), @"[impl-follows-type] /main.rs:7: `impl Bar` should follow type definition (line 4), but has 2 blank line(s)");
}

#[test]
fn chained_impls_pass() {
	assert_check_passing(
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}
		impl Foo {
			fn two() {}
		}
		"#,
		&opts(),
	);
}

#[test]
fn impl_for_type_not_defined_in_file_is_ignored() {
	assert_check_passing(
		r#"


		impl String {
			fn custom() {}
		}
		"#,
		&opts(),
	);
}

#[test]
fn autofix_removes_blank_lines() {
	insta::assert_snapshot!(simulate_format(
		r#"
		struct Foo {
			x: i32,
		}


		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}
		"#,
		&opts(),
	), @r#"
	struct Foo {
		x: i32,
	}
	impl Foo {
		fn new() -> Self { Self { x: 0 } }
	}
	"#);
}

#[test]
fn autofix_relocates_impl_when_other_code_in_between() {
	insta::assert_snapshot!(simulate_format(
		r#"
		struct Foo {
			x: i32,
		}

		fn unrelated() {}

		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}
		"#,
		&opts(),
	), @r#"
	struct Foo {
		x: i32,
	}
	impl Foo {
		fn new() -> Self { Self { x: 0 } }
	}

	fn unrelated() {}
	"#);
}

#[test]
fn autofix_with_multiple_impl_blocks_for_same_struct() {
	insta::assert_snapshot!(simulate_format(
		r#"
		struct Foo;

		fn other() {}

		impl Foo {
			fn one() {}
		}

		impl Foo {
			fn two() {}
		}
		"#,
		&opts(),
	), @r#"
	struct Foo;
	impl Foo {
		fn one() {}
	}

	fn other() {}

	impl Foo {
		fn two() {}
	}
	"#);
}
