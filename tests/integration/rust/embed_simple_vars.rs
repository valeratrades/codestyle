use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("embed_simple_vars")
}

// === Passing cases ===

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
fn complex_expression_method_call_passes() {
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
fn field_access_passes() {
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
fn named_placeholder_width_specifier_passes() {
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
fn already_embedded_with_format_specifiers_passes() {
	assert_check_passing(
		r#"
		fn test() {
			let value = vec![1, 2, 3];
			println!("value: {value:?}");
			error!("pretty: {value:#?}");
			warn!("precision: {value:.0}");
		}
		"#,
		&opts(),
	);
}

// === Violation cases ===

#[test]
fn simple_var_in_println() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let name = "world";
			println!("Hello, {}", name);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{name}` instead of `{}, name`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let name = "world";
		println!("Hello, {name}");
	}
	"#);
}

#[test]
fn multiple_simple_vars() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let a = 1;
			let b = 2;
			println!("{} + {}", a, b);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:4: use `{a}` instead of `{}, a`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:4: use `{b}` instead of `{}, b`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let a = 1;
		let b = 2;
		println!("{a} + {b}");
	}
	"#);
}

#[test]
fn format_macro() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let x = 42;
			let s = format!("value: {}", x);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{x}` instead of `{}, x`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let x = 42;
		let s = format!("value: {x}");
	}
	"#);
}

#[test]
fn write_macro() {
	insta::assert_snapshot!(test_case(
		r#"
		use std::io::Write;
		fn test() {
			let x = 42;
			let mut buf = Vec::new();
			write!(buf, "{}", x).unwrap();
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:5: use `{x}` instead of `{}, x`
	HINT: variables should be embedded in the format string

	# Format mode
	use std::io::Write;
	fn test() {
		let x = 42;
		let mut buf = Vec::new();
		write!(buf, "{x}").unwrap();
	}
	"#);
}

#[test]
fn multi_line_format_macro() {
	insta::assert_snapshot!(test_case(
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
	# Assert mode
	[embed-simple-vars] /main.rs:6: use `{name}` instead of `{}, name`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:7: use `{count}` instead of `{}, count`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let name = "world";
		let count = 42;
		println!(
			"Hello {name}, you have {count} messages"
		);
	}
	"#);
}

#[test]
fn mixed_simple_and_complex_args() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let tf = "1d";
			let s = format!("{}_{}", Utc::now().format("%Y/%m/%d"), tf);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{tf}` instead of `{}, tf`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let tf = "1d";
		let s = format!("{}_{tf}", Utc::now().format("%Y/%m/%d"));
	}
	"#);
}

#[test]
fn multiple_simple_vars_mixed_with_complex() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let issue_number = 123;
			let sanitized = "foo";
			let s = format!("{}_-_{}.{}", issue_number, sanitized, extension.as_str());
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:4: use `{issue_number}` instead of `{}, issue_number`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:4: use `{sanitized}` instead of `{}, sanitized`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let issue_number = 123;
		let sanitized = "foo";
		let s = format!("{issue_number}_-_{sanitized}.{}", extension.as_str());
	}
	"#);
}

#[test]
fn field_access_not_doubled_in_format() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let workspace_id = "ws123";
			let s = format!("{}/user/{}/time-entries", workspace_id, user.id);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{workspace_id}` instead of `{}, workspace_id`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let workspace_id = "ws123";
		let s = format!("{workspace_id}/user/{}/time-entries", user.id);
	}
	"#);
}

#[test]
fn assert_macros() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let expected = 42;
			assert!(value == expected, "expected {}", expected);
			assert_eq!(a, b, "comparison failed: {}", expected);
			debug_assert!(check(), "check failed: {}", expected);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{expected}` instead of `{}, expected`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:4: use `{expected}` instead of `{}, expected`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:5: use `{expected}` instead of `{}, expected`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let expected = 42;
		assert!(value == expected, "expected {expected}");
		assert_eq!(a, b, "comparison failed: {expected}");
		debug_assert!(check(), "check failed: {expected}");
	}
	"#);
}

#[test]
fn todo_unimplemented_unreachable_macros() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let feature = "auth";
			todo!("implement {}", feature);
			unimplemented!("not yet: {}", feature);
			unreachable!("should not reach: {}", feature);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{feature}` instead of `{}, feature`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:4: use `{feature}` instead of `{}, feature`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:5: use `{feature}` instead of `{}, feature`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let feature = "auth";
		todo!("implement {feature}");
		unimplemented!("not yet: {feature}");
		unreachable!("should not reach: {feature}");
	}
	"#);
}

#[test]
fn bail_and_eyre_macros() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() -> Result<()> {
			let reason = "invalid input";
			bail!("failed: {}", reason);
			Err(eyre!("error: {}", reason))
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{reason}` instead of `{}, reason`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:4: use `{reason}` instead of `{}, reason`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() -> Result<()> {
		let reason = "invalid input";
		bail!("failed: {reason}");
		Err(eyre!("error: {reason}"))
	}
	"#);
}

#[test]
fn anyhow_and_ensure_macros() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() -> Result<()> {
			let value = 42;
			ensure!(value > 0, "value must be positive: {}", value);
			Err(anyhow!("unexpected value: {}", value))
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{value}` instead of `{}, value`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:4: use `{value}` instead of `{}, value`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() -> Result<()> {
		let value = 42;
		ensure!(value > 0, "value must be positive: {value}");
		Err(anyhow!("unexpected value: {value}"))
	}
	"#);
}

#[test]
fn debug_format_specifier() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let value = vec![1, 2, 3];
			println!("value: {:?}", value);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{value:?}` instead of `{:?}, value`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let value = vec![1, 2, 3];
		println!("value: {value:?}");
	}
	"#);
}

#[test]
fn debug_format_with_multiple_specifiers() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let value = vec![1, 2, 3];
			println!("value: {:?}", value);
			println!("pandoc md → typst:        {:?}", pandoc_to_typst);
			warn!("{:.0}", precision);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{value:?}` instead of `{:?}, value`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:4: use `{pandoc_to_typst:?}` instead of `{:?}, pandoc_to_typst`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:5: use `{precision:.0}` instead of `{:.0}, precision`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let value = vec![1, 2, 3];
		println!("value: {value:?}");
		println!("pandoc md → typst:        {pandoc_to_typst:?}");
		warn!("{precision:.0}");
	}
	"#);
}

#[test]
fn debug_format_mixed_with_display() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let name = "test";
			let data = vec![1, 2, 3];
			println!("{}: {:?}", name, data);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:4: use `{name}` instead of `{}, name`
	HINT: variables should be embedded in the format string
	[embed-simple-vars] /main.rs:4: use `{data:?}` instead of `{:?}, data`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let name = "test";
		let data = vec![1, 2, 3];
		println!("{name}: {data:?}");
	}
	"#);
}

#[test]
fn debug_format_pretty_print() {
	insta::assert_snapshot!(test_case(
		r#"
		fn test() {
			let value = vec![1, 2, 3];
			println!("value: {:#?}", value);
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[embed-simple-vars] /main.rs:3: use `{value:#?}` instead of `{:#?}, value`
	HINT: variables should be embedded in the format string

	# Format mode
	fn test() {
		let value = vec![1, 2, 3];
		println!("value: {value:#?}");
	}
	"#);
}
