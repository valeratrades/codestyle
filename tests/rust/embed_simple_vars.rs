use crate::utils::{assert_check_passing, opts_for, simulate_check, simulate_format};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("embed_simple_vars")
}

#[test]
fn simple_var_in_println_should_embed() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			let name = "world";
			println!("Hello, {}", name);
		}
		"#,
		&opts(),
	), @"[embed-simple-vars] /main.rs:3: variable `name` should be embedded in format string: use `{name}` instead of `{}, name`");
}

#[test]
fn already_embedded_var_passes() {
	assert_check_passing(
		r#"
		fn test() {
			let name = "world";
			println!("Hello, {name}");
		}
		"#,
		&opts(),
	);
}

#[test]
fn complex_expression_method_call_is_fine() {
	assert_check_passing(
		r#"
		fn test() {
			let s = String::new();
			println!("len: {}", s.len());
		}
		"#,
		&opts(),
	);
}

#[test]
fn field_access_is_fine() {
	assert_check_passing(
		r#"
		struct Foo { x: i32 }
		fn test() {
			let f = Foo { x: 1 };
			println!("x: {}", f.x);
		}
		"#,
		&opts(),
	);
}

#[test]
fn all_simple_vars() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			let a = 1;
			let b = 2;
			println!("{} + {}", a, b);
		}
		"#,
		&opts(),
	), @r#"
	[embed-simple-vars] /main.rs:4: variable `a` should be embedded in format string: use `{a}` instead of `{}, a`
	[embed-simple-vars] /main.rs:4: variable `b` should be embedded in format string: use `{b}` instead of `{}, b`
	"#);
}

#[test]
fn mixed_simple_and_complex_all_simple_vars_reported() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			let a = 1;
			let b = 2;
			let sum = a + b;
			println!("{} + {} = {}", a, b, sum);
		}
		"#,
		&opts(),
	), @r#"
	[embed-simple-vars] /main.rs:5: variable `a` should be embedded in format string: use `{a}` instead of `{}, a`
	[embed-simple-vars] /main.rs:5: variable `b` should be embedded in format string: use `{b}` instead of `{}, b`
	[embed-simple-vars] /main.rs:5: variable `sum` should be embedded in format string: use `{sum}` instead of `{}, sum`
	"#);
}

#[test]
fn format_macro_works_too() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			let x = 42;
			let s = format!("value: {}", x);
		}
		"#,
		&opts(),
	), @"[embed-simple-vars] /main.rs:3: variable `x` should be embedded in format string: use `{x}` instead of `{}, x`");
}

#[test]
fn write_macro() {
	insta::assert_snapshot!(simulate_check(
		r#"
		use std::io::Write;
		fn test() {
			let x = 42;
			let mut buf = Vec::new();
			write!(buf, "{}", x).unwrap();
		}
		"#,
		&opts(),
	), @"[embed-simple-vars] /main.rs:5: variable `x` should be embedded in format string: use `{x}` instead of `{}, x`");
}

#[test]
fn no_placeholder_no_violation() {
	assert_check_passing(
		r#"
		fn test() {
			println!("Hello, world!");
		}
		"#,
		&opts(),
	);
}

#[test]
fn named_placeholder_is_fine() {
	assert_check_passing(
		r#"
		fn test() {
			let width = 5;
			println!("{:width$}", "hi");
		}
		"#,
		&opts(),
	);
}

#[test]
fn multi_line_format_macro_detected() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			let name = "world";
			let count = 42;
			println!(
				"Hello {}, you have {} messages",
				name,
				count
			);
		}
		"#,
		&opts(),
	), @r#"
	[embed-simple-vars] /main.rs:6: variable `name` should be embedded in format string: use `{name}` instead of `{}, name`
	[embed-simple-vars] /main.rs:7: variable `count` should be embedded in format string: use `{count}` instead of `{}, count`
	"#);
}

#[test]
fn mixed_simple_and_complex_args_check() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			let tf = "1d";
			let s = format!("{}_{}", Utc::now().format("%Y/%m/%d"), tf);
		}
		"#,
		&opts(),
	), @"[embed-simple-vars] /main.rs:3: variable `tf` should be embedded in format string: use `{tf}` instead of `{}, tf`");
}

#[test]
fn autofix_mixed_args() {
	insta::assert_snapshot!(simulate_format(
		r#"
		fn test() {
			let tf = "1d";
			let s = format!("{}_{}", Utc::now().format("%Y/%m/%d"), tf);
		}
		"#,
		&opts(),
	), @r#"
	fn test() {
		let tf = "1d";
		let s = format!("{}_{tf}", Utc::now().format("%Y/%m/%d"));
	}
	"#);
}

#[test]
fn multiple_simple_vars_mixed_with_complex_check() {
	insta::assert_snapshot!(simulate_check(
		r#"
		fn test() {
			let issue_number = 123;
			let sanitized = "foo";
			let s = format!("{}_-_{}.{}", issue_number, sanitized, extension.as_str());
		}
		"#,
		&opts(),
	), @r#"
	[embed-simple-vars] /main.rs:4: variable `issue_number` should be embedded in format string: use `{issue_number}` instead of `{}, issue_number`
	[embed-simple-vars] /main.rs:4: variable `sanitized` should be embedded in format string: use `{sanitized}` instead of `{}, sanitized`
	"#);
}

#[test]
fn autofix_embeds_both_simple_vars() {
	insta::assert_snapshot!(simulate_format(
		r#"
		fn test() {
			let issue_number = 123;
			let sanitized = "foo";
			let s = format!("{}_-_{}.{}", issue_number, sanitized, extension.as_str());
		}
		"#,
		&opts(),
	), @r#"
	fn test() {
		let issue_number = 123;
		let sanitized = "foo";
		let s = format!("{issue_number}_-_{sanitized}.{}", extension.as_str());
	}
	"#);
}

#[test]
fn field_access_should_not_be_doubled() {
	insta::assert_snapshot!(simulate_format(
		r#"
		fn test() {
			let workspace_id = "ws123";
			let s = format!("{}/user/{}/time-entries", workspace_id, user.id);
		}
		"#,
		&opts(),
	), @r#"
	fn test() {
		let workspace_id = "ws123";
		let s = format!("{workspace_id}/user/{}/time-entries", user.id);
	}
	"#);
}
