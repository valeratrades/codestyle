use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("pub_first")
}

// === Passing cases ===

#[test]
fn all_pub_items_first_passes() {
	assert_check_passing(
		r#"
		pub struct Foo;
		pub fn bar() {}
		struct Baz;
		fn qux() {}
		"#,
		&opts(),
	);
}

#[test]
fn all_private_passes() {
	assert_check_passing(
		r#"
		struct Foo;
		fn bar() {}
		"#,
		&opts(),
	);
}

#[test]
fn all_pub_passes() {
	assert_check_passing(
		r#"
		pub struct Foo;
		pub fn bar() {}
		"#,
		&opts(),
	);
}

#[test]
fn pub_main_first_then_other_pub_passes() {
	assert_check_passing(
		r#"
		pub fn main() {}
		pub fn other() {}
		fn private() {}
		"#,
		&opts(),
	);
}

#[test]
fn private_main_first_then_other_private_passes() {
	assert_check_passing(
		r#"
		pub fn other_pub() {}
		fn main() {}
		fn other() {}
		"#,
		&opts(),
	);
}

#[test]
fn impl_blocks_are_ignored() {
	assert_check_passing(
		r#"
		pub struct Foo;
		impl Foo {
			fn private_method() {}
		}
		struct Bar;
		"#,
		&opts(),
	);
}

#[test]
fn use_statements_are_ignored() {
	assert_check_passing(
		r#"
		use std::io;
		pub struct Foo;
		struct Bar;
		"#,
		&opts(),
	);
}

// === Violation cases ===

#[test]
fn private_before_pub_struct() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Private;
		pub struct Public;
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: public item should come before private items

	# Format mode
	pub struct Public;
	struct Private;
	");
}

#[test]
fn private_fn_before_pub_fn() {
	insta::assert_snapshot!(test_case(
		r#"
		fn private() {}
		pub fn public() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: public item should come before private items

	# Format mode
	pub fn public() {}
	fn private() {}
	");
}

#[test]
fn mixed_items_need_reordering() {
	insta::assert_snapshot!(test_case(
		r#"
		fn private1() {}
		pub struct Foo;
		struct Bar;
		pub fn public1() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: public item should come before private items

	# Format mode
	pub struct Foo;
	pub fn public1() {}
	fn private1() {}
	struct Bar;
	");
}

#[test]
fn main_not_first_in_pub_category() {
	insta::assert_snapshot!(test_case(
		r#"
		pub fn other() {}
		pub fn main() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: `main` function should be at the top of its visibility category

	# Format mode
	pub fn main() {}
	pub fn other() {}
	");
}

#[test]
fn main_not_first_in_private_category() {
	insta::assert_snapshot!(test_case(
		r#"
		pub fn public() {}
		fn other() {}
		fn main() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:3: `main` function should be at the top of its visibility category

	# Format mode
	pub fn public() {}
	fn main() {}
	fn other() {}
	");
}

#[test]
fn complex_reordering() {
	insta::assert_snapshot!(test_case(
		r#"
		fn helper() {}
		pub struct Config;
		fn main() {}
		pub fn run() {}
		struct Internal;
		pub fn main() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: public item should come before private items

	# Format mode
	pub fn main() {}
	pub struct Config;
	pub fn run() {}
	fn main() {}
	fn helper() {}
	struct Internal;
	");
}
