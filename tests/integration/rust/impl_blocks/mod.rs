//! Tests for impl-related rules: join_split_impls, impl_follows_type, impl_folds.
//!
//! Order matters: join_split_impls -> impl_follows_type -> impl_folds

mod impl_folds;
mod impl_follows_type;
mod join_split_impls;

use codestyle::rust_checks::RustCheckOptions;

use crate::utils::test_case;

fn all_impl_opts() -> RustCheckOptions {
	RustCheckOptions {
		join_split_impls: true,
		impl_follows_type: true,
		impl_folds: true,
		..Default::default()
	}
}

/// Test that verifies join_split_impls runs before impl_follows_type.
/// If impl_follows runs first, it would move one of the split impls to follow the struct,
/// leaving the second impl block orphaned. Then join would fail to merge them properly.
/// Correct order: join first merges them, then follows moves the single merged block.
#[test]
fn order_join_before_follows() {
	// Split impl blocks where the struct is far from both impls.
	// Wrong order (follows first) would move first impl to struct, leaving second orphaned.
	// Correct order (join first) merges both, then moves the single block.
	insta::assert_snapshot!(test_case(
		r#"
		fn unrelated_start() {}

		struct Foo;

		fn middle() {}

		impl Foo {
			fn one() {}
		}

		fn between() {}

		impl Foo {
			fn two() {}
		}
		"#,
		&all_impl_opts(),
	), @"
	# Assert mode
	[join-split-impls] /main.rs:13: split `impl Foo` blocks should be joined into one
	[impl-folds] /main.rs:7: impl block missing vim fold markers
	[impl-folds] /main.rs:13: impl block missing vim fold markers
	[impl-follows-type] /main.rs:7: `impl Foo` should follow type definition (line 3), but has 3 blank line(s)
	[impl-follows-type] /main.rs:13: `impl Foo` should follow type definition (line 9), but has 3 blank line(s)

	# Format mode
	fn unrelated_start() {}

	struct Foo;
	impl Foo /*{{{1*/ {
		fn one() {}
		fn two() {}
	}
	//,}}}1


	fn middle() {}

	fn between() {}
	");
}

/// Test that verifies impl_follows_type runs before impl_folds.
/// Fold markers should be added to the impl block AFTER it's been moved to follow its type.
/// This ensures the markers are in the correct final position.
#[test]
fn order_follows_before_folds() {
	// impl is far from struct and needs fold markers.
	// Wrong order (folds first) would add markers at wrong location, then follows moves it.
	// Correct order ensures markers are added at the final location.
	insta::assert_snapshot!(test_case(
		r#"
		struct Bar {
			x: i32,
		}

		fn intervening() {}


		impl Bar {
			fn method(&self) -> i32 { self.x }
		}
		"#,
		&all_impl_opts(),
	), @"
	# Assert mode
	[impl-folds] /main.rs:8: impl block missing vim fold markers
	[impl-follows-type] /main.rs:8: `impl Bar` should follow type definition (line 3), but has 4 blank line(s)

	# Format mode
	struct Bar {
		x: i32,
	}
	impl Bar /*{{{1*/ {
		fn method(&self) -> i32 { self.x }
	}
	//,}}}1


	fn intervening() {}
	");
}
