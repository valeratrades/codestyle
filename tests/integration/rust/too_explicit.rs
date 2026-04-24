use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("too_explicit")
}

// === Passing cases ===

#[test]
fn multiline_use_with_existing_import_no_duplicate() {
	// If PathBuf is already in a multi-line use block, we must NOT add a duplicate import.
	insta::assert_snapshot!(test_case(
		r#"
		use std::{
			collections::HashMap,
			path::{Path, PathBuf},
		};

		fn foo(p: &std::path::Path) -> std::path::PathBuf {
			std::path::PathBuf::from("x")
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[too-explicit] /main.rs:6: use `Path` instead of `std::path::Path`
	[too-explicit] /main.rs:6: use `PathBuf` instead of `std::path::PathBuf`
	[too-explicit] /main.rs:7: use `PathBuf` instead of `std::path::PathBuf`

	# Format mode
	use std::{
		collections::HashMap,
		path::{Path, PathBuf},
	};

	fn foo(p: &Path) -> PathBuf {
		PathBuf::from("x")
	}
	"#);
}

#[test]
fn already_short_arc_passes() {
	assert_check_passing(
		r#"
		use std::sync::Arc;

		fn foo(x: Arc<i32>) -> Arc<i32> {
			Arc::new(42)
		}
		"#,
		&opts(),
	);
}

#[test]
fn use_statement_full_path_passes() {
	assert_check_passing(
		r#"
		use std::sync::Arc;
		use std::path::PathBuf;

		fn foo(_: Arc<i32>, _: PathBuf) {}
		"#,
		&opts(),
	);
}

#[test]
fn unrelated_path_passes() {
	assert_check_passing(
		r#"
		fn foo() -> my_crate::something::Widget {
			my_crate::something::Widget::new()
		}
		"#,
		&opts(),
	);
}

// === Violation cases (with autofix) ===

#[test]
fn inline_arc_type_and_expr() {
	insta::assert_snapshot!(test_case(
		r#"
		fn foo() -> std::sync::Arc<i32> {
			std::sync::Arc::new(42)
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[too-explicit] /main.rs:1: use `Arc` instead of `std::sync::Arc`
	[too-explicit] /main.rs:2: use `Arc` instead of `std::sync::Arc`
	[too-explicit] /main.rs:1: missing `use std::sync::Arc;` import

	# Format mode
	use std::sync::Arc;
	fn foo() -> Arc<i32> {
		Arc::new(42)
	}
	");
}

#[test]
fn inline_mutex() {
	insta::assert_snapshot!(test_case(
		r#"
		fn make() -> std::sync::Mutex<Vec<i32>> {
			std::sync::Mutex::new(vec![])
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[too-explicit] /main.rs:1: use `Mutex` instead of `std::sync::Mutex`
	[too-explicit] /main.rs:2: use `Mutex` instead of `std::sync::Mutex`
	[too-explicit] /main.rs:1: missing `use std::sync::Mutex;` import

	# Format mode
	use std::sync::Mutex;
	fn make() -> Mutex<Vec<i32>> {
		Mutex::new(vec![])
	}
	");
}

#[test]
fn inline_pathbuf() {
	insta::assert_snapshot!(test_case(
		r#"
		fn get_path() -> std::path::PathBuf {
			std::path::PathBuf::from("/tmp")
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[too-explicit] /main.rs:1: use `PathBuf` instead of `std::path::PathBuf`
	[too-explicit] /main.rs:2: use `PathBuf` instead of `std::path::PathBuf`
	[too-explicit] /main.rs:1: missing `use std::path::PathBuf;` import

	# Format mode
	use std::path::PathBuf;
	fn get_path() -> PathBuf {
		PathBuf::from("/tmp")
	}
	"#);
}

#[test]
fn inline_btreemap() {
	insta::assert_snapshot!(test_case(
		r#"
		fn make_map() -> std::collections::BTreeMap<String, u32> {
			std::collections::BTreeMap::new()
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[too-explicit] /main.rs:1: use `BTreeMap` instead of `std::collections::BTreeMap`
	[too-explicit] /main.rs:2: use `BTreeMap` instead of `std::collections::BTreeMap`
	[too-explicit] /main.rs:1: missing `use std::collections::BTreeMap;` import

	# Format mode
	use std::collections::BTreeMap;
	fn make_map() -> BTreeMap<String, u32> {
		BTreeMap::new()
	}
	");
}

#[test]
fn nested_arc_in_vec() {
	insta::assert_snapshot!(test_case(
		r#"
		fn foo() -> Vec<std::sync::Arc<i32>> {
			vec![std::sync::Arc::new(1)]
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[too-explicit] /main.rs:1: use `Arc` instead of `std::sync::Arc`
	[too-explicit] /main.rs:1: missing `use std::sync::Arc;` import

	# Format mode
	use std::sync::Arc;
	fn foo() -> Vec<Arc<i32>> {
		vec![std::sync::Arc::new(1)]
	}
	");
}

// Arc imported inside a function body must not trigger a spurious top-level import.
// Mirrors robot_master/robot_master_game/src/result.rs `format_elo` pattern.
#[test]
fn fn_body_import_no_spurious_top_level() {
	assert_check_passing(
		r#"
		fn format_stuff() -> String {
			use std::sync::Arc;
			let db: Arc<i32> = Arc::new(42);
			format!("{}", db)
		}
		"#,
		&opts(),
	);
}

// Arc imported inside an inline mod must not trigger a spurious top-level import.
#[test]
fn inline_mod_import_no_spurious_top_level() {
	assert_check_passing(
		r#"
		mod foo {
			use std::sync::Arc;
			pub fn bar() -> Arc<i32> {
				Arc::new(42)
			}
		}
		"#,
		&opts(),
	);
}

#[test]
fn multiple_types_in_one_file() {
	insta::assert_snapshot!(test_case(
		r#"
		fn foo(a: std::sync::Arc<i32>, b: std::path::PathBuf) {
			let _ = std::sync::Arc::new(0);
			let _ = std::path::PathBuf::from("x");
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[too-explicit] /main.rs:1: use `Arc` instead of `std::sync::Arc`
	[too-explicit] /main.rs:1: use `PathBuf` instead of `std::path::PathBuf`
	[too-explicit] /main.rs:2: use `Arc` instead of `std::sync::Arc`
	[too-explicit] /main.rs:3: use `PathBuf` instead of `std::path::PathBuf`
	[too-explicit] /main.rs:1: missing `use std::sync::Arc;` import
	[too-explicit] /main.rs:1: missing `use std::path::PathBuf;` import

	# Format mode
	use std::path::PathBuf;
	use std::sync::Arc;
	fn foo(a: Arc<i32>, b: PathBuf) {
		let _ = Arc::new(0);
		let _ = PathBuf::from("x");
	}
	"#);
}
