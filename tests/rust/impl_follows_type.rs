use crate::utils::{assert_check_passing, opts_for, simulate_check, simulate_format};

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

/// When there's code between type def and impl, we don't auto-fix to avoid
/// creating overlapping fixes that could corrupt the file.
#[test]
fn no_autofix_when_other_code_in_between() {
	// File should remain unchanged - violation requires manual fix
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

	fn unrelated() {}

	impl Foo {
		fn new() -> Self { Self { x: 0 } }
	}
	"#);
}

/// When there's code between type def and impl, we don't auto-fix to avoid
/// creating overlapping fixes that could corrupt the file. The second impl Foo
/// is OK because it follows immediately after the first impl Foo.
#[test]
fn no_autofix_with_code_between_type_and_impl() {
	// File should remain unchanged - first violation requires manual fix
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

	fn other() {}

	impl Foo {
		fn one() {}
	}

	impl Foo {
		fn two() {}
	}
	"#);
}

/// Regression test: when struct B is defined between struct A and impl A,
/// and impl B comes after impl A, auto-fixing could corrupt the file by
/// creating overlapping replacement ranges. Now we don't auto-fix when
/// there's code between type def and impl.
#[test]
fn no_autofix_with_interleaved_types_and_impls() {
	// File should remain unchanged - both violations require manual fix
	insta::assert_snapshot!(simulate_format(
		r#"
		struct Foo {
			x: i32,
		}

		/// Bar is defined here, between Foo struct and Foo impl
		struct Bar {
			y: i32,
		}

		impl Foo {
			fn foo_method(&self) -> i32 { self.x }
		}

		fn unrelated_function() {}

		impl Bar {
			fn bar_method(&self) -> i32 { self.y }
		}
		"#,
		&opts(),
	), @r#"
	struct Foo {
		x: i32,
	}

	/// Bar is defined here, between Foo struct and Foo impl
	struct Bar {
		y: i32,
	}

	impl Foo {
		fn foo_method(&self) -> i32 { self.x }
	}

	fn unrelated_function() {}

	impl Bar {
		fn bar_method(&self) -> i32 { self.y }
	}
	"#);
}
