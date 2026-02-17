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

// === Bug reproduction tests - these should pass after fix ===

#[test]
fn impl_blocks_preserved_during_reorder() {
	// Impl blocks are interspersed between pub and private items.
	// The fix should move the private fn above the pub fn, but preserve the impl block.
	insta::assert_snapshot!(test_case(
		r#"
		fn private_helper() {}

		pub struct Foo;

		impl Foo {
			fn method(&self) {}
		}

		pub fn public_fn() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:3: public item should come before private items

	# Format mode
	pub struct Foo;
	pub fn public_fn() {}
	fn private_helper() {}


	impl Foo {
		fn method(&self) {}
	}
	");
}

#[test]
fn doc_comments_preserved_during_reorder() {
	// Doc comments should stay with their items during reordering.
	insta::assert_snapshot!(test_case(
		r#"
		/// Private helper function
		fn private_helper() {}

		/// Public struct
		pub struct Foo;
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:4: public item should come before private items

	# Format mode
	/// Public struct
	pub struct Foo;
	/// Private helper function
	fn private_helper() {}
	");
}

#[test]
fn attributes_preserved_during_reorder() {
	// Attributes should stay with their items during reordering.
	insta::assert_snapshot!(test_case(
		r#"
		#[cfg(test)]
		fn private_test_helper() {}

		#[derive(Debug)]
		pub struct Foo;
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:4: public item should come before private items

	# Format mode
	#[derive(Debug)]
	pub struct Foo;
	#[cfg(test)]
	fn private_test_helper() {}
	");
}

#[test]
fn blank_lines_preserved_during_reorder() {
	// Blank lines between items should be preserved appropriately.
	insta::assert_snapshot!(test_case(
		r#"
		fn helper1() {}

		fn helper2() {}

		pub fn public1() {}

		pub fn public2() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:5: public item should come before private items

	# Format mode
	pub fn public1() {}
	pub fn public2() {}
	fn helper1() {}

	fn helper2() {}
	");
}

#[test]
fn trait_impl_preserved_during_reorder() {
	// Trait impls should be preserved when reordering.
	insta::assert_snapshot!(test_case(
		r#"
		fn private() {}

		pub struct Foo;

		impl Default for Foo {
			fn default() -> Self {
				Foo
			}
		}

		pub fn public() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:3: public item should come before private items

	# Format mode
	pub struct Foo;
	pub fn public() {}
	fn private() {}


	impl Default for Foo {
		fn default() -> Self {
			Foo
		}
	}
	");
}

#[test]
fn use_statements_at_top_preserved() {
	// Use statements at the top should not be affected.
	insta::assert_snapshot!(test_case(
		r#"
		use std::io;

		fn private() {}

		pub fn public() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:5: public item should come before private items

	# Format mode
	use std::io;

	pub fn public() {}
	fn private() {}
	");
}

#[test]
fn static_preserved_during_reorder() {
	// Static items should be handled correctly.
	insta::assert_snapshot!(test_case(
		r#"
		static CACHE: &str = "test";

		fn private() {}

		pub fn public() {}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[pub-first] /main.rs:5: public item should come before private items

	# Format mode
	pub fn public() {}
	static CACHE: &str = "test";

	fn private() {}
	"#);
}

#[test]
fn mod_declarations_are_ignored() {
	// mod ordering is handled by rustfmt, so pub_first should skip them entirely
	assert_check_passing(
		r#"
		mod z_private;
		pub mod a_public;
		pub fn public() {}
		fn private() {}
		"#,
		&opts(),
	);
}

// === Const/type at top, then pub/private ordering tests ===
// New ordering: const (all), type (all), pub items (main > trait > other), private items (main > trait > other)

#[test]
fn correct_ordering_passes() {
	// Full correct ordering: const, type, pub main, pub trait, pub other, private main, private trait, private other
	assert_check_passing(
		r#"
		pub const A: u32 = 1;
		const B: u32 = 2;
		pub type PubInt = i32;
		type PrivInt = i64;
		pub fn main() {}
		pub trait PubTrait {}
		pub fn other() {}
		fn main() {}
		trait PrivTrait {}
		fn other() {}
		"#,
		&opts(),
	);
}

#[test]
fn const_after_type_needs_reordering() {
	insta::assert_snapshot!(test_case(
		r#"
		pub type MyInt = i32;
		pub const VERSION: &str = "1.0";
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: `const` should come before all other items

	# Format mode
	pub const VERSION: &str = \"1.0\";
	pub type MyInt = i32;
	");
}

#[test]
fn const_after_fn_needs_reordering() {
	insta::assert_snapshot!(test_case(
		r#"
		pub fn main() {}
		const VERSION: u32 = 1;
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: `const` should come before all other items

	# Format mode
	const VERSION: u32 = 1;
	pub fn main() {}
	");
}

#[test]
fn type_after_fn_needs_reordering() {
	insta::assert_snapshot!(test_case(
		r#"
		pub fn main() {}
		type MyInt = i32;
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: `type` should come before all other items (after const)

	# Format mode
	type MyInt = i32;
	pub fn main() {}
	");
}

#[test]
fn type_after_struct_needs_reordering() {
	insta::assert_snapshot!(test_case(
		r#"
		pub struct Foo;
		pub type MyInt = i32;
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: `type` should come before all other items (after const)

	# Format mode
	pub type MyInt = i32;
	pub struct Foo;
	");
}

#[test]
fn private_const_after_pub_fn_needs_reordering() {
	// Even private const should come before pub fn
	insta::assert_snapshot!(test_case(
		r#"
		pub fn public() {}
		const INTERNAL: u32 = 42;
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: `const` should come before all other items

	# Format mode
	const INTERNAL: u32 = 42;
	pub fn public() {}
	");
}

#[test]
fn private_type_after_pub_fn_needs_reordering() {
	// Even private type should come before pub fn
	insta::assert_snapshot!(test_case(
		r#"
		pub fn public() {}
		type InternalInt = i64;
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: `type` should come before all other items (after const)

	# Format mode
	type InternalInt = i64;
	pub fn public() {}
	");
}

#[test]
fn pub_trait_not_first_in_pub_category() {
	insta::assert_snapshot!(test_case(
		r#"
		pub fn other() {}
		pub trait MyTrait {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:2: `trait` should be at the top of its visibility category (after main)

	# Format mode
	pub trait MyTrait {}
	pub fn other() {}
	");
}

#[test]
fn private_trait_not_first_in_private_category() {
	insta::assert_snapshot!(test_case(
		r#"
		pub fn public() {}
		fn other() {}
		trait InternalTrait {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:3: `trait` should be at the top of its visibility category (after main)

	# Format mode
	pub fn public() {}
	trait InternalTrait {}
	fn other() {}
	");
}

#[test]
fn full_ordering_combined() {
	// This tests the full ordering: const (all), type (all), pub main, pub trait, pub other, private main, private trait, private other
	insta::assert_snapshot!(test_case(
		r#"
		fn helper() {}
		pub struct Config;
		trait InternalTrait {}
		type InternalInt = i64;
		const INTERNAL: u32 = 42;
		fn main() {}
		pub trait PubTrait {}
		pub fn run() {}
		pub type PubInt = i32;
		pub const VERSION: &str = "1.0";
		pub fn main() {}
		"#,
		&opts(),
	), @"
	# Assert mode
	[pub-first] /main.rs:5: `const` should come before all other items

	# Format mode
	const INTERNAL: u32 = 42;
	pub const VERSION: &str = \"1.0\";
	type InternalInt = i64;
	pub type PubInt = i32;
	pub fn main() {}
	pub trait PubTrait {}
	pub struct Config;
	pub fn run() {}
	fn main() {}
	trait InternalTrait {}
	fn helper() {}
	");
}
