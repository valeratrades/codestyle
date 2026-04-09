use crate::utils::{assert_check_passing, opts_for, test_case, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("prefer_default_over_bare_new")
}

// === Passing cases ===

#[test]
fn new_with_args_passes() {
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

#[test]
fn private_no_args_new_passes() {
	// Only pub fn new() is flagged
	assert_check_passing(
		r#"
		struct Foo;
		impl Foo {
			fn new() -> Self { Self }
		}
		"#,
		&opts(),
	);
}

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
	), @"[prefer-default-over-bare-new] /main.rs:3: argument-less `pub fn new()` found; implement `Default` instead — callers should use `Type::default()`");
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
	[prefer-default-over-bare-new] /main.rs:4: argument-less `pub fn new()` found; implement `Default` instead — callers should use `Type::default()`
	[prefer-default-over-bare-new] /main.rs:7: argument-less `pub fn new()` found; implement `Default` instead — callers should use `Type::default()`
	");
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
	[prefer-default-over-bare-new] /main.rs:2: `Type::new()` — use `Type::default()` instead

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
	[prefer-default-over-bare-new] /main.rs:2: `Type::new()` — use `Type::default()` instead

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
	[prefer-default-over-bare-new] /main.rs:2: `Type::new()` — use `Type::default()` instead
	[prefer-default-over-bare-new] /main.rs:3: `Type::new()` — use `Type::default()` instead

	# Format mode
	fn foo() {
		let a = String::default();
		let b = Vec::<u8>::default();
	}
	");
}
