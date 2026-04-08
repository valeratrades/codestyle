use crate::utils::{assert_check_passing, opts_for, test_case, test_case_assert_only};

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
	// fn new with args is fine — not no-arg, not returning Result
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

// === Violation cases: pub fn new() no-args (unfixable definition) ===

#[test]
fn pub_new_no_args() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		struct Foo;
		impl Foo {
			pub fn new() -> Self {
				Self
			}
		}
		"#,
		&opts(),
	), @"[unconventional-new] /main.rs:3: argument-less `pub fn new()` found; implement `Default` instead — callers should use `Type::default()`");
}

#[test]
fn multiple_types_with_argless_new() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		struct Foo;
		struct Bar;
		impl Foo {
			pub fn new() -> Self { Self }
		}
		impl Bar {
			pub fn new() -> Self { Self }
		}
		"#,
		&opts(),
	), @"
	[unconventional-new] /main.rs:4: argument-less `pub fn new()` found; implement `Default` instead — callers should use `Type::default()`
	[unconventional-new] /main.rs:7: argument-less `pub fn new()` found; implement `Default` instead — callers should use `Type::default()`
	");
}

// === Violation cases: Type::new() -> Type::try_new() callsite rename (cross-file) ===

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
	[unconventional-new] /main.rs:2: `Foo::new` was renamed to `try_new`
	[unconventional-new] /lib.rs:3: `fn new` returns `Result` — rename to `try_new` to signal fallibility

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

// === Passing cases: Default impl with function calls — must not be rewritten ===

#[test]
fn default_impl_with_fn_call_not_rewritten() {
	assert_check_passing(
		r#"
		pub struct RandomPlayer {
			params: Random,
			rng: SmallRng,
		}
		impl Default for RandomPlayer {
			fn default() -> Self {
				Self {
					params: Random {},
					rng: rand::make_rng(),
				}
			}
		}
		fn use_it() {
			let _p = RandomPlayer::new();
		}
		"#,
		&opts(),
	);
}

// === Violation cases: Type::new() call-sites (fixable: rename to default) ===

#[test]
fn vec_new_call() {
	insta::assert_snapshot!(test_case(
		r#"
		fn foo() {
			let v = Vec::<i32>::new();
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /main.rs:2: `Type::new()` — use `Type::default()` instead

	# Format mode
	fn foo() {
		let v = Vec::<i32>::default();
	}
	");
}

#[test]
fn string_new_call() {
	insta::assert_snapshot!(test_case(
		r#"
		fn foo() {
			let s = String::new();
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /main.rs:2: `Type::new()` — use `Type::default()` instead

	# Format mode
	fn foo() {
		let s = String::default();
	}
	");
}

#[test]
fn multiple_callsites() {
	insta::assert_snapshot!(test_case(
		r#"
		fn foo() {
			let a = String::new();
			let b = Vec::<u8>::new();
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /main.rs:2: `Type::new()` — use `Type::default()` instead
	[unconventional-new] /main.rs:3: `Type::new()` — use `Type::default()` instead

	# Format mode
	fn foo() {
		let a = String::default();
		let b = Vec::<u8>::default();
	}
	");
}
