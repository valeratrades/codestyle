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
	[unconventional-new] /main.rs:3
	HINT: `fn new` returns `Result` — rename to `try_new` to signal fallibility

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
	[unconventional-new] /main.rs:3
	HINT: `fn new` returns `Result` — rename to `try_new` to signal fallibility

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
	[unconventional-new] /main.rs:3
	HINT: `fn new` returns `Result` — rename to `try_new` to signal fallibility

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
	[unconventional-new] /lib.rs:3
	HINT: `fn new` returns `Result` — rename to `try_new` to signal fallibility
	[unconventional-new] /main.rs:2: `Foo::new`
	HINT: `new` was renamed to `try_new`

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

#[test]
fn call_sites_inside_macro_invocations_are_renamed() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			pub fn new(x: i32) -> Result<Self, String> {
				Ok(Self)
			}
		}
		fn f() {
			assert!(Foo::new(1).is_err());
			let v = vec![Foo::new(2), Foo::new(3)];
			println!("{}", Foo::new(4).is_ok());
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[unconventional-new] /main.rs:3
	HINT: `fn new` returns `Result` — rename to `try_new` to signal fallibility
	[unconventional-new] /main.rs:8: `Foo::new`
	HINT: `new` was renamed to `try_new`
	[unconventional-new] /main.rs:9: `Foo::new`
	HINT: `new` was renamed to `try_new`
	[unconventional-new] /main.rs:9: `Foo::new`
	HINT: `new` was renamed to `try_new`
	[unconventional-new] /main.rs:10: `Foo::new`
	HINT: `new` was renamed to `try_new`

	# Format mode
	struct Foo;
	impl Foo {
		pub fn try_new(x: i32) -> Result<Self, String> {
			Ok(Self)
		}
	}
	fn f() {
		assert!(Foo::try_new(1).is_err());
		let v = vec![Foo::try_new(2), Foo::try_new(3)];
		println!("{}", Foo::try_new(4).is_ok());
	}
	"#);
}

#[test]
fn self_new_call_sites_are_renamed() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo(i32);
		impl Foo {
			pub fn new(x: i32) -> Result<Self, String> {
				Ok(Self(x))
			}
			pub fn with(&self, x: i32) -> Result<Self, String> {
				assert!(Self::new(x).is_ok());
				Self::new(x)
			}
		}
		struct Bar;
		impl Bar {
			pub fn new() -> Self {
				Self
			}
			pub fn again() -> Self {
				Self::new()
			}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /main.rs:3
	HINT: `fn new` returns `Result` — rename to `try_new` to signal fallibility
	[unconventional-new] /main.rs:7: `Foo::new`
	HINT: `new` was renamed to `try_new`
	[unconventional-new] /main.rs:8: `Foo::new`
	HINT: `new` was renamed to `try_new`

	# Format mode
	struct Foo(i32);
	impl Foo {
		pub fn try_new(x: i32) -> Result<Self, String> {
			Ok(Self(x))
		}
		pub fn with(&self, x: i32) -> Result<Self, String> {
			assert!(Self::try_new(x).is_ok());
			Self::try_new(x)
		}
	}
	struct Bar;
	impl Bar {
		pub fn new() -> Self {
			Self
		}
		pub fn again() -> Self {
			Self::new()
		}
	}
	");
}

#[test]
fn call_sites_inside_trait_impls_are_renamed() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			pub fn new() -> Result<Self, String> {
				Ok(Self)
			}
		}
		impl Default for Foo {
			fn default() -> Self {
				Self::new().unwrap()
			}
		}
		impl From<()> for String {
			fn from(_: ()) -> Self {
				Foo::new().err().unwrap()
			}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /main.rs:3
	HINT: `fn new` returns `Result` — rename to `try_new` to signal fallibility
	[unconventional-new] /main.rs:9: `Foo::new`
	HINT: `new` was renamed to `try_new`
	[unconventional-new] /main.rs:14: `Foo::new`
	HINT: `new` was renamed to `try_new`

	# Format mode
	struct Foo;
	impl Foo {
		pub fn try_new() -> Result<Self, String> {
			Ok(Self)
		}
	}
	impl Default for Foo {
		fn default() -> Self {
			Self::try_new().unwrap()
		}
	}
	impl From<()> for String {
		fn from(_: ()) -> Self {
			Foo::try_new().err().unwrap()
		}
	}
	");
}

#[test]
fn new_passed_as_a_path_is_renamed() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			pub fn new(x: i32) -> Result<Self, String> {
				Ok(Self)
			}
		}
		fn f() -> Vec<Result<Foo, String>> {
			[1, 2].into_iter().map(Foo::new).collect()
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /main.rs:3
	HINT: `fn new` returns `Result` — rename to `try_new` to signal fallibility
	[unconventional-new] /main.rs:8: `Foo::new`
	HINT: `new` was renamed to `try_new`

	# Format mode
	struct Foo;
	impl Foo {
		pub fn try_new(x: i32) -> Result<Self, String> {
			Ok(Self)
		}
	}
	fn f() -> Vec<Result<Foo, String>> {
		[1, 2].into_iter().map(Foo::try_new).collect()
	}
	");
}

#[test]
fn new_defined_in_inline_module_renames_its_call_sites() {
	insta::assert_snapshot!(test_case(
		r#"
		mod inner {
			pub struct Foo;
			impl Foo {
				pub fn new() -> Result<Self, String> {
					Ok(Self)
				}
			}
		}
		fn f() {
			let _f = inner::Foo::new();
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[unconventional-new] /main.rs:4
	HINT: `fn new` returns `Result` — rename to `try_new` to signal fallibility
	[unconventional-new] /main.rs:10: `Foo::new`
	HINT: `new` was renamed to `try_new`

	# Format mode
	mod inner {
		pub struct Foo;
		impl Foo {
			pub fn try_new() -> Result<Self, String> {
				Ok(Self)
			}
		}
	}
	fn f() {
		let _f = inner::Foo::try_new();
	}
	");
}
