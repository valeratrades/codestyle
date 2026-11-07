use crate::utils::{assert_check_passing, opts_for, test_case, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("split_asserts")
}

// === Passing cases ===

#[test]
fn single_condition_and_disjunction_pass() {
	assert_check_passing(
		r#"
		fn f(a: bool, b: bool) {
			assert!(a);
			assert!(a || b);
			assert!(!(a && b));
			assert_eq!(a && b, false);
		}
		"#,
		&opts(),
	);
}

// === Violation cases ===

#[test]
fn chain_with_message() {
	insta::assert_snapshot!(test_case(
		r#"
		fn f(a: bool, b: bool, c: bool) {
			debug_assert!(a && (b && c), "bad state: {a}");
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[split-asserts] /main.rs:2: `debug_assert!` over `&&`: split into one `debug_assert!` per condition, so a failure points at the exact one

	# Format mode
	fn f(a: bool, b: bool, c: bool) {
		debug_assert!(a, "bad state: {a}");
		debug_assert!(b, "bad state: {a}");
		debug_assert!(c, "bad state: {a}");
	}
	"#);
}

#[test]
fn mixed_precedence_splits_only_top_level_and() {
	insta::assert_snapshot!(test_case(
		r#"
		fn f(x: Option<u32>, b: bool, c: bool) {
			if b {
				assert!(x.is_some() && (b || c));
			}
		}
		"#,
		&opts(),
	), @"
	# Assert mode
	[split-asserts] /main.rs:3: `assert!` over `&&`: split into one `assert!` per condition, so a failure points at the exact one

	# Format mode
	fn f(x: Option<u32>, b: bool, c: bool) {
		if b {
			assert!(x.is_some());
			assert!(b || c);
		}
	}
	");
}

#[test]
fn expression_position_has_no_autofix() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn f(v: &[bool]) {
			v.iter().for_each(|b| assert!(*b && v.len() > 1));
		}
		"#,
		&opts(),
	), @"[split-asserts] /main.rs:2: `assert!` over `&&`: split into one `assert!` per condition, so a failure points at the exact one");
}
