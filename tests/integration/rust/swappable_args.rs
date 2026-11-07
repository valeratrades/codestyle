use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("swappable_args")
}

// === Passing cases ===

#[test]
fn distinct_types_and_non_primitives_pass() {
	assert_check_passing(
		r#"
		struct Price(u64);
		struct Qty(u64);
		fn a(price: u64, name: &str, flag: bool) {}
		fn b(price: Price, qty: Qty) {}
		fn c(a: &str, b: String, c: &mut String) {}
		fn d(v: Vec<u8>, w: Vec<u8>) {}
		"#,
		&opts(),
	);
}

#[test]
fn trait_impl_closures_and_tests_are_exempt() {
	assert_check_passing(
		r#"
		struct S;
		impl std::ops::Add<u64> for S {
			type Output = u64;
			fn add(self, rhs: u64) -> u64 { 0 }
		}
		impl S {
			fn method(&self, a: u64) {}
		}
		fn f() {
			let _g = |a: u64, b: u64| a + b;
		}
		#[test]
		fn t() {}
		#[cfg(test)]
		mod tests {
			fn helper(a: u64, b: u64) {}
		}
		"#,
		&opts(),
	);
}

#[test]
fn skip_marker_passes() {
	assert_check_passing(
		r#"
		//#[codestyle::skip(swappable-args)]
		fn a(x: f64, y: f64) {}
		struct S;
		impl S {
			//#[codestyle::skip(swappable-args)]
			fn b(&self, x: f64, y: f64) {}
		}
		"#,
		&opts(),
	);
}

// === Violation cases (no autofix) ===

#[test]
fn repeated_primitives() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		fn free(price: u64, qty: u64) {}
		fn refs(a: &str, b: &str, c: bool, d: bool) {}
		fn mixed(a: &str, b: String, c: &mut u8, d: &mut u8) {}
		struct S;
		impl S {
			fn method(&self, x: f64, y: f64) {}
		}
		trait T {
			fn decl(a: char, b: char);
		}
		"#,
		&opts(),
	), @"
	[swappable-args] /main.rs:1: `free` takes interchangeable parameters (2 x `u64`)
	HINT: wrap them in newtypes, or pass an args struct
	[swappable-args] /main.rs:2: `refs` takes interchangeable parameters (2 x `&str`, 2 x `bool`)
	HINT: wrap them in newtypes, or pass an args struct
	[swappable-args] /main.rs:3: `mixed` takes interchangeable parameters (2 x `&mut u8`)
	HINT: wrap them in newtypes, or pass an args struct
	[swappable-args] /main.rs:6: `method` takes interchangeable parameters (2 x `f64`)
	HINT: wrap them in newtypes, or pass an args struct
	[swappable-args] /main.rs:9: `decl` takes interchangeable parameters (2 x `char`)
	HINT: wrap them in newtypes, or pass an args struct
	");
}
