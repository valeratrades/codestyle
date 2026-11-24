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
fn called_by_argument_name_is_exempt() {
	assert_check_passing(
		r#"
		#[component]
		pub fn Button(label: String, href: String) -> Element {}
		#[dioxus::prelude::component]
		fn Qualified(a: bool, b: bool) -> Element {}
		#[component(transparent)]
		fn WithArgs(a: bool, b: bool) -> impl IntoView {}
		#[island]
		fn Island(a: u32, b: u32) -> impl IntoView {}
		#[autoprops]
		#[function_component]
		fn Yew(a: &str, b: &str) -> Html {}
		#[poise::command(slash_command)]
		async fn slash(ctx: Context<'_>, a: String, b: String) -> Result<(), Error> {}
		#[builder]
		fn built(a: f64, b: f64) {}
		struct S;
		#[bon]
		impl S {
			#[builder]
			fn built(&self, a: f64, b: f64) {}
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
		#[component]
		fn Button(label: String, href: String) -> Element {
			fn helper(a: u8, b: u8) {}
		}
		"#,
		&opts(),
	), @"
	[swappable-args] /main.rs:1: `free` (2 x `u64`)
	HINT: fn takes interchangeable parameters; wrap them in newtypes, or pass an args struct
	[swappable-args] /main.rs:2: `refs` (2 x `&str`, 2 x `bool`)
	HINT: fn takes interchangeable parameters; wrap them in newtypes, or pass an args struct
	[swappable-args] /main.rs:3: `mixed` (2 x `&mut u8`)
	HINT: fn takes interchangeable parameters; wrap them in newtypes, or pass an args struct
	[swappable-args] /main.rs:6: `method` (2 x `f64`)
	HINT: fn takes interchangeable parameters; wrap them in newtypes, or pass an args struct
	[swappable-args] /main.rs:9: `decl` (2 x `char`)
	HINT: fn takes interchangeable parameters; wrap them in newtypes, or pass an args struct
	[swappable-args] /main.rs:13: `helper` (2 x `u8`)
	HINT: fn takes interchangeable parameters; wrap them in newtypes, or pass an args struct
	");
}
