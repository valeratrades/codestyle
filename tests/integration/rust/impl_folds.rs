use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("impl_folds")
}

// === Passing cases ===

#[test]
fn impl_with_fold_markers_passes() {
	assert_check_passing(
		r#"
		struct Foo;
		impl Foo /*{{{1*/ {
			fn new() -> Self { Self }
		}
		//,}}}1
		"#,
		&opts(),
	);
}

#[test]
fn impl_with_fold_markers_no_space_passes() {
	assert_check_passing(
		r#"
		struct Bar;
		impl Bar/*{{{1*/{
			fn get(&self) -> i32 { 0 }
		}
		//,}}}1
		"#,
		&opts(),
	);
}

#[test]
fn trait_impl_ignored() {
	// Trait impls should not require fold markers (only direct `impl Type`)
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
fn generic_impl_with_fold_markers_passes() {
	assert_check_passing(
		r#"
		struct Container<T>(T);
		impl<T> Container<T> /*{{{1*/ {
			fn inner(&self) -> &T { &self.0 }
		}
		//,}}}1
		"#,
		&opts(),
	);
}

#[test]
fn impl_with_where_clause_and_fold_markers_passes() {
	assert_check_passing(
		r#"
		struct Wrapper<T>(T);
		impl<T> Wrapper<T>
		where
			T: Clone,
		/*{{{1*/ {
			fn cloned(&self) -> T { self.0.clone() }
		}
		//,}}}1
		"#,
		&opts(),
	);
}

#[test]
fn multiple_impls_each_with_fold_markers_passes() {
	assert_check_passing(
		r#"
		struct Foo;
		struct Bar;

		impl Foo /*{{{1*/ {
			fn foo() {}
		}
		//,}}}1

		impl Bar /*{{{1*/ {
			fn bar() {}
		}
		//,}}}1
		"#,
		&opts(),
	);
}

// === Violation cases ===

#[test]
fn simple_impl_without_fold_markers() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			fn new() -> Self { Self }
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[impl-folds] /main.rs:2: impl block missing vim fold markers

	# Format mode
	struct Foo;
	impl Foo /*{{{1*/ {
		fn new() -> Self { Self }
	}
	//,}}}1
	");
}

#[test]
fn impl_missing_closing_fold_marker() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo /*{{{1*/ {
			fn new() -> Self { Self }
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[impl-folds] /main.rs:2: impl block missing closing vim fold marker //,}}}1

	# Format mode
	struct Foo;
	impl Foo /*{{{1*/ {
		fn new() -> Self { Self }
	}
	//,}}}1
	");
}

#[test]
fn impl_missing_opening_fold_marker() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Foo;
		impl Foo {
			fn new() -> Self { Self }
		}
		//,}}}1
		"#,
		&opts(),
	), @"
	# Assert mode
	[impl-folds] /main.rs:2: impl block missing opening vim fold marker /*{{{1*/

	# Format mode
	struct Foo;
	impl Foo /*{{{1*/ {
		fn new() -> Self { Self }
	}
	//,}}}1
	");
}

#[test]
fn generic_impl_without_fold_markers() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Container<T>(T);
		impl<T> Container<T> {
			fn inner(&self) -> &T { &self.0 }
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[impl-folds] /main.rs:2: impl block missing vim fold markers

	# Format mode
	struct Container<T>(T);
	impl<T> Container<T> /*{{{1*/ {
		fn inner(&self) -> &T { &self.0 }
	}
	//,}}}1
	");
}

#[test]
fn impl_with_where_clause_without_fold_markers() {
	insta::assert_snapshot!(test_case(
		r#"
		struct Wrapper<T>(T);
		impl<T> Wrapper<T>
		where
			T: Clone,
		{
			fn cloned(&self) -> T { self.0.clone() }
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[impl-folds] /main.rs:2: impl block missing vim fold markers

	# Format mode
	struct Wrapper<T>(T);
	impl<T> Wrapper<T>
	where
		T: Clone,
	/*{{{1*/ {
		fn cloned(&self) -> T { self.0.clone() }
	}
	//,}}}1
	");
}

#[test]
fn multiple_impls_without_fold_markers() {
	insta::assert_snapshot!(test_case(
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
	), @"
	# Assert mode
	[impl-folds] /main.rs:4: impl block missing vim fold markers
	[impl-folds] /main.rs:8: impl block missing vim fold markers

	# Format mode
	struct Foo;
	struct Bar;

	impl Foo /*{{{1*/ {
		fn foo() {}
	}
	//,}}}1


	impl Bar /*{{{1*/ {
		fn bar() {}
	}
	//,}}}1
	");
}

#[test]
fn impl_from_example_in_spec() {
	// Example from the user's specification
	insta::assert_snapshot!(test_case(
		r#"
		#[derive(Clone, Debug)]
		pub struct FetchedIssue {
			pub link: IssueLink,
			pub title: String,
		}

		impl FetchedIssue {
			pub fn new(link: IssueLink, title: impl Into<String>) -> Self {
				Self { link, title: title.into() }
			}

			/// Create from owner, repo, number, and title (constructs the URL internally).
			pub fn from_parts(owner: &str, repo: &str, number: u64, title: impl Into<String>) -> Option<Self> {
				let url_str = format!("https://github.com/{owner}/{repo}/issues/{number}");
				let link = IssueLink::parse(&url_str)?;
				Some(Self { link, title: title.into() })
			}

			/// Convenience: get the issue number
			pub fn number(&self) -> u64 {
				self.link.number()
			}

			/// Convenience: get owner
			pub fn owner(&self) -> &str {
				self.link.owner()
			}

			/// Convenience: get repo
			pub fn repo(&self) -> &str {
				self.link.repo()
			}
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[impl-folds] /main.rs:7: impl block missing vim fold markers

	# Format mode
	#[derive(Clone, Debug)]
	pub struct FetchedIssue {
		pub link: IssueLink,
		pub title: String,
	}

	impl FetchedIssue /*{{{1*/ {
		pub fn new(link: IssueLink, title: impl Into<String>) -> Self {
			Self { link, title: title.into() }
		}

		/// Create from owner, repo, number, and title (constructs the URL internally).
		pub fn from_parts(owner: &str, repo: &str, number: u64, title: impl Into<String>) -> Option<Self> {
			let url_str = format!("https://github.com/{owner}/{repo}/issues/{number}");
			let link = IssueLink::parse(&url_str)?;
			Some(Self { link, title: title.into() })
		}

		/// Convenience: get the issue number
		pub fn number(&self) -> u64 {
			self.link.number()
		}

		/// Convenience: get owner
		pub fn owner(&self) -> &str {
			self.link.owner()
		}

		/// Convenience: get repo
		pub fn repo(&self) -> &str {
			self.link.repo()
		}
	}
	//,}}}1
	"#);
}
