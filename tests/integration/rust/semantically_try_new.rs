use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("semantically_try_new")
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
fn new_returning_self_passes() {
	assert_check_passing(
		r#"
		struct Foo;
		impl Foo {
			pub fn new() -> Self {
				Self
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
			pub fn new() -> Option<Self> {
				Some(Self)
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn trait_impl_new_result_passes() {
	// `fn new` inside a trait impl should not be flagged
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

// === Violation cases ===

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
	[semantically-try-new] /main.rs:3: `fn new` returns `Result` — rename to `try_new` to signal fallibility

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
	[semantically-try-new] /main.rs:3: `fn new` returns `Result` — rename to `try_new` to signal fallibility

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
fn multiple_new_returning_result() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		struct Bar;
		impl Foo {
			pub fn new() -> Result<Self, String> { Ok(Self) }
		}
		impl Bar {
			fn new() -> Result<Self, anyhow::Error> { Ok(Self) }
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[semantically-try-new] /main.rs:4: `fn new` returns `Result` — rename to `try_new` to signal fallibility
	[semantically-try-new] /main.rs:7: `fn new` returns `Result` — rename to `try_new` to signal fallibility

	# Format mode
	struct Foo;
	struct Bar;
	impl Foo {
		pub fn try_new() -> Result<Self, String> { Ok(Self) }
	}
	impl Bar {
		fn try_new() -> Result<Self, anyhow::Error> { Ok(Self) }
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
	[semantically-try-new] /main.rs:3: `fn new` returns `Result` — rename to `try_new` to signal fallibility

	# Format mode
	struct Foo { x: i32 }
	impl Foo {
		pub fn try_new(x: i32) -> Result<Self, String> {
			Ok(Self { x })
		}
	}
	");
}
