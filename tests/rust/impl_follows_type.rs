use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("impl_follows_type")
}

// === Passing cases ===

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

// === Violation cases ===

#[test]
fn struct_with_blank_lines_before_impl() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo {
			x: i32,
		}


		impl Foo {
			fn new() -> Self { Self { x: 0 } }
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[impl-follows-type] /main.rs:6: `impl Foo` should follow type definition (line 3), but has 2 blank line(s)

	# Format mode
	struct Foo {
		x: i32,
	}
	impl Foo {
		fn new() -> Self { Self { x: 0 } }
	}
	");
}

#[test]
fn enum_with_blank_lines_before_impl() {
	insta::assert_snapshot!(test_case(
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
	), @"
	# Assert mode
	[impl-follows-type] /main.rs:7: `impl Bar` should follow type definition (line 4), but has 2 blank line(s)

	# Format mode
	enum Bar {
		A,
		B,
	}
	impl Bar {
		fn is_a(&self) -> bool { matches!(self, Self::A) }
	}
	");
}

#[test]
fn impl_with_code_in_between() {
	insta::assert_snapshot!(test_case(
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
	), @"
	# Assert mode
	[impl-follows-type] /main.rs:7: `impl Foo` should follow type definition (line 3), but has 3 blank line(s)

	# Format mode
	struct Foo {
		x: i32,
	}
	impl Foo {
		fn new() -> Self { Self { x: 0 } }
	}

	fn unrelated() {}
	");
}

#[test]
fn multiple_impl_blocks_with_code_in_between() {
	insta::assert_snapshot!(test_case(
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
	), @"
	# Assert mode
	[impl-follows-type] /main.rs:5: `impl Foo` should follow type definition (line 1), but has 3 blank line(s)

	# Format mode
	struct Foo;
	impl Foo {
		fn one() {}
	}
	impl Foo {
		fn two() {}
	}

	fn other() {}
	");
}

#[test]
fn interleaved_types_and_impls() {
	insta::assert_snapshot!(test_case(
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
	), @"
	# Assert mode
	[impl-follows-type] /main.rs:10: `impl Foo` should follow type definition (line 3), but has 6 blank line(s)
	[impl-follows-type] /main.rs:16: `impl Bar` should follow type definition (line 8), but has 7 blank line(s)

	# Format mode
	struct Foo {
		x: i32,
	}
	impl Foo {
		fn foo_method(&self) -> i32 { self.x }
	}

	/// Bar is defined here, between Foo struct and Foo impl
	struct Bar {
		y: i32,
	}
	impl Bar {
		fn bar_method(&self) -> i32 { self.y }
	}

	fn unrelated_function() {}
	");
}
