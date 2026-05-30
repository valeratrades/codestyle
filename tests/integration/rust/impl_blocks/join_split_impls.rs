use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("join_split_impls")
}

// === Passing cases ===

#[test]
fn single_impl_block_passes() {
	assert_check_passing(
		r#"
		struct Foo {
			x: i32,
		}
		impl Foo {
			fn new() -> Self { Self { x: 0 } }
			fn get(&self) -> i32 { self.x }
		}
		"#,
		&opts(),
	);
}

#[test]
fn trait_impl_not_joined_with_inherent_impl() {
	assert_check_passing(
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}
		impl Default for Foo {
			fn default() -> Self { Foo }
		}
		"#,
		&opts(),
	);
}

#[test]
fn different_trait_impls_not_joined() {
	assert_check_passing(
		r#"
		struct Foo;
		impl Default for Foo {
			fn default() -> Self { Foo }
		}
		impl Clone for Foo {
			fn clone(&self) -> Self { Foo }
		}
		"#,
		&opts(),
	);
}

#[test]
fn impl_blocks_for_different_types_not_joined() {
	assert_check_passing(
		r#"
		struct Foo;
		struct Bar;
		impl Foo {
			fn foo() {}
		}
		impl Bar {
			fn bar() {}
		}
		"#,
		&opts(),
	);
}

#[test]
fn impl_blocks_with_different_generic_params_not_joined() {
	// impl<R: LocalReader> LocalIssueSource<R> and impl LocalIssueSource<FsReader>
	// are different impl blocks and should NOT be joined
	assert_check_passing(
		r#"
		struct FsReader;
		struct GitReader;
		trait LocalReader {}
		struct LocalIssueSource<R> {
			reader: R,
		}
		impl<R: LocalReader> LocalIssueSource<R> {
			fn new() -> Self { todo!() }
		}
		impl LocalIssueSource<FsReader> {
			fn submitted() -> Self { todo!() }
		}
		impl LocalIssueSource<GitReader> {
			fn consensus() -> Self { todo!() }
		}
		"#,
		&opts(),
	);
}

#[test]
fn cross_file_impl_blocks_not_detected() {
	// Currently NOT detected (single-file scope)
	assert_check_passing(
		r#"
		//- /src/first.rs
		pub struct Foo;
		impl Foo {
			fn bar() {}
		}

		//- /src/second.rs
		use crate::first::Foo;
		impl Foo {
			fn yuck() {
				println!("Cross-file impl - not detected");
			}
		}
		"#,
		&opts(),
	);
}

// === Violation cases ===

#[test]
fn two_consecutive_impl_blocks() {
	insta::assert_snapshot!(test_case(
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
	), @"
	# Assert mode
	[join-split-impls] /main.rs:5: split `impl Foo` blocks should be joined into one

	# Format mode
	struct Foo;
	impl Foo {
		fn one() {}
		fn two() {}
	}
	");
}

#[test]
fn impl_blocks_with_code_in_between() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}

		fn unrelated() {}

		impl Foo {
			fn two() {}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[join-split-impls] /main.rs:8: split `impl Foo` blocks should be joined into one

	# Format mode
	struct Foo;
	impl Foo {
		fn one() {}
		fn two() {}
	}

	fn unrelated() {}
	");
}

#[test]
fn three_impl_blocks() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}
		impl Foo {
			fn two() {}
		}
		impl Foo {
			fn three() {}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[join-split-impls] /main.rs:5: split `impl Foo` blocks should be joined into one

	# Format mode
	struct Foo;
	impl Foo {
		fn one() {}
		fn two() {}
		fn three() {}
	}
	");
}

#[test]
fn join_preserves_existing_fold_markers() {
	// First impl has fold markers, second doesn't
	// The join should preserve the fold markers from the first impl
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo /*{{{1*/ {
			fn one() {}
		}
		//,}}}1
		impl Foo {
			fn two() {}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[join-split-impls] /main.rs:6: split `impl Foo` blocks should be joined into one

	# Format mode
	struct Foo;
	impl Foo /*{{{1*/ {
		fn one() {}
		fn two() {}
	}

	//,}}}1
	");
}

#[test]
fn join_with_multibyte_char_in_second_doc_comment() {
	// Regression: a multibyte char (em-dash) in the second impl block's doc comment,
	// positioned before the impl's opening brace, must not corrupt the join.
	// `find_impl_brace` returned a char index that was treated as a byte offset, so
	// every multibyte char before the brace shifted the items slice and produced
	// broken Rust (an orphaned ` {` and a dropped closing brace / between-section).
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			fn one() {}
		}

		my_macro!(Foo, c);

		/// doc comment with an em-dash — before the brace
		impl Foo {
			fn two() {}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[join-split-impls] /main.rs:8: split `impl Foo` blocks should be joined into one

	# Format mode
	struct Foo;
	impl Foo {
		fn one() {}
		fn two() {}
	}

	my_macro!(Foo, c);
	");
}

#[test]
fn join_preserves_nested_indentation() {
	// Functions with nested blocks should preserve their internal indentation
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			fn one() {
				if true {
					println!("nested");
				}
			}
		}
		impl Foo {
			fn two() {
				for i in 0..10 {
					println!("{i}");
				}
			}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[join-split-impls] /main.rs:9: split `impl Foo` blocks should be joined into one

	# Format mode
	struct Foo;
	impl Foo {
		fn one() {
			if true {
				println!(\"nested\");
			}
		}
		fn two() {
			for i in 0..10 {
				println!(\"{i}\");
			}
		}
	}
	");
}
