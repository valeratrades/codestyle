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
	[embed-simple-vars] /main.rs:3: variable `name` should be embedded in format string: use `{name}` instead of `{}, name`

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
	[embed-simple-vars] /main.rs:4: variable `a` should be embedded in format string: use `{a}` instead of `{}, a`
	[embed-simple-vars] /main.rs:4: variable `b` should be embedded in format string: use `{b}` instead of `{}, b`

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
	[embed-simple-vars] /main.rs:3: variable `x` should be embedded in format string: use `{x}` instead of `{}, x`

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
	[embed-simple-vars] /main.rs:5: variable `x` should be embedded in format string: use `{x}` instead of `{}, x`

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
	[embed-simple-vars] /main.rs:6: variable `name` should be embedded in format string: use `{name}` instead of `{}, name`
	[embed-simple-vars] /main.rs:7: variable `count` should be embedded in format string: use `{count}` instead of `{}, count`

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
	[embed-simple-vars] /main.rs:3: variable `tf` should be embedded in format string: use `{tf}` instead of `{}, tf`

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
	[embed-simple-vars] /main.rs:4: variable `issue_number` should be embedded in format string: use `{issue_number}` instead of `{}, issue_number`
	[embed-simple-vars] /main.rs:4: variable `sanitized` should be embedded in format string: use `{sanitized}` instead of `{}, sanitized`

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
	[embed-simple-vars] /main.rs:3: variable `workspace_id` should be embedded in format string: use `{workspace_id}` instead of `{}, workspace_id`

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
	[embed-simple-vars] /main.rs:3: variable `expected` should be embedded in format string: use `{expected}` instead of `{}, expected`
	[embed-simple-vars] /main.rs:4: variable `expected` should be embedded in format string: use `{expected}` instead of `{}, expected`
	[embed-simple-vars] /main.rs:5: variable `expected` should be embedded in format string: use `{expected}` instead of `{}, expected`

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
	[embed-simple-vars] /main.rs:3: variable `feature` should be embedded in format string: use `{feature}` instead of `{}, feature`
	[embed-simple-vars] /main.rs:4: variable `feature` should be embedded in format string: use `{feature}` instead of `{}, feature`
	[embed-simple-vars] /main.rs:5: variable `feature` should be embedded in format string: use `{feature}` instead of `{}, feature`

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
	[embed-simple-vars] /main.rs:3: variable `reason` should be embedded in format string: use `{reason}` instead of `{}, reason`
	[embed-simple-vars] /main.rs:4: variable `reason` should be embedded in format string: use `{reason}` instead of `{}, reason`

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
	[embed-simple-vars] /main.rs:3: variable `value` should be embedded in format string: use `{value}` instead of `{}, value`
	[embed-simple-vars] /main.rs:4: variable `value` should be embedded in format string: use `{value}` instead of `{}, value`

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
	[embed-simple-vars] /main.rs:3: variable `value` should be embedded in format string: use `{value:?}` instead of `{:?}, value`

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
	[embed-simple-vars] /main.rs:3: variable `value` should be embedded in format string: use `{value:?}` instead of `{:?}, value`
	[embed-simple-vars] /main.rs:4: variable `pandoc_to_typst` should be embedded in format string: use `{pandoc_to_typst:?}` instead of `{:?}, pandoc_to_typst`
	[embed-simple-vars] /main.rs:5: variable `precision` should be embedded in format string: use `{precision:.0}` instead of `{:.0}, precision`

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
	[embed-simple-vars] /main.rs:4: variable `name` should be embedded in format string: use `{name}` instead of `{}, name`
	[embed-simple-vars] /main.rs:4: variable `data` should be embedded in format string: use `{data:?}` instead of `{:?}, data`

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
	[embed-simple-vars] /main.rs:3: variable `value` should be embedded in format string: use `{value:#?}` instead of `{:#?}, value`

	# Format mode
	fn test() {
		let value = vec![1, 2, 3];
		println!("value: {value:#?}");
	}
	"#);
}
