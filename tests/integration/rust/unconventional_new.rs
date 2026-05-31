use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("unconventional_new")
}

// === Passing cases ===

#[test]
fn try_new_already_named_correctly() {
	assert_check_passing(
		r#"
		struct Foo;
		impl Foo {
			pub fn try_new() -> Result<Self, String> {
				Ok(Self)
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn new_with_args_passes() {
	// fn new with args is fine — not returning Result
	assert_check_passing(
		r#"
		struct Foo { x: i32 }
		impl Foo {
			pub fn new(x: i32) -> Self {
				Self { x }
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn new_with_option_return_passes() {
	assert_check_passing(
		r#"
		struct Foo;
		impl Foo {
			pub fn new(x: i32) -> Option<Self> {
				Some(Self)
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn trait_impl_new_result_passes() {
	assert_check_passing(
		r#"
		trait Builder {
			fn new() -> Result<Self, String> where Self: Sized;
		}
		struct Foo;
		impl Builder for Foo {
			fn new() -> Result<Self, String> { Ok(Self) }
		}
		"#,
		&opts(),
	);
}

#[test]
fn trait_impl_no_args_new_passes() {
	assert_check_passing(
		r#"
		trait Builder {
			fn new() -> Self;
		}
		struct Foo;
		impl Builder for Foo {
			fn new() -> Self { Self }
		}
		"#,
		&opts(),
	);
}

// === Violation cases: fn new() -> Result (fixable: rename to try_new) ===

#[test]
fn pub_new_returns_result() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			pub fn new() -> Result<Self, String> {
				Ok(Self)
			}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /main.rs:3: `fn new` returns `Result` — rename to `try_new` to signal fallibility

	# Format mode
	struct Foo;
	impl Foo {
		pub fn try_new() -> Result<Self, String> {
			Ok(Self)
		}
	}
	");
}

#[test]
fn private_new_returns_result() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			fn new() -> Result<Self, String> {
				Ok(Self)
			}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /main.rs:3: `fn new` returns `Result` — rename to `try_new` to signal fallibility

	# Format mode
	struct Foo;
	impl Foo {
		fn try_new() -> Result<Self, String> {
			Ok(Self)
		}
	}
	");
}

#[test]
fn new_with_args_returns_result() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo { x: i32 }
		impl Foo {
			pub fn new(x: i32) -> Result<Self, String> {
				Ok(Self { x })
			}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /main.rs:3: `fn new` returns `Result` — rename to `try_new` to signal fallibility

	# Format mode
	struct Foo { x: i32 }
	impl Foo {
		pub fn try_new(x: i32) -> Result<Self, String> {
			Ok(Self { x })
		}
	}
	");
}

// === Violation cases: Type::new(...) -> Type::try_new(...) callsite rename (cross-file) ===

#[test]
fn try_new_callsite_renamed_cross_file() {
	insta::assert_snapshot!(test_case(
		r#"
		//- /lib.rs
		struct Foo;
		impl Foo {
			pub fn new() -> Result<Self, String> {
				Ok(Self)
			}
		}
		//- /main.rs
		fn main() {
			let _f = Foo::new();
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /lib.rs:3: `fn new` returns `Result` — rename to `try_new` to signal fallibility
	[unconventional-new] /main.rs:2: `Foo::new` was renamed to `try_new`

	# Format mode
	//- /lib.rs
	struct Foo;
	impl Foo {
		pub fn try_new() -> Result<Self, String> {
			Ok(Self)
		}
	}
	//- /main.rs
	fn main() {
		let _f = Foo::try_new();
	}
	");
}
