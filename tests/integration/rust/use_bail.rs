use crate::utils::{assert_check_passing, opts_for, test_case};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("use_bail")
}

// === Passing cases ===

#[test]
fn bail_already_used_passes() {
	assert_check_passing(
		r#"
		use eyre::bail;

		fn test() -> eyre::Result<()> {
			bail!("something went wrong");
		}
		"#,
		&opts(),
	);
}

#[test]
fn plain_return_err_not_modified() {
	assert_check_passing(
		r#"
		use std::io;

		fn test() -> io::Result<()> {
			return Err(io::Error::new(io::ErrorKind::Other, "error"));
		}
		"#,
		&opts(),
	);
}

#[test]
fn err_without_return_not_modified() {
	assert_check_passing(
		r#"
		use eyre::eyre;

		fn test() -> eyre::Result<i32> {
			let result: Result<i32, _> = Err(eyre!("error"));
			result
		}
		"#,
		&opts(),
	);
}

// === Violation cases ===

#[test]
fn return_err_eyre_simple() {
	insta::assert_snapshot!(test_case(
		r#"
		use eyre::eyre;

		fn test() -> eyre::Result<()> {
			return Err(eyre!("something went wrong"));
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[use-bail] /main.rs:4: use `bail!(...)` instead of `return Err(eyre!(...))`

	# Format mode
	use eyre::eyre;
	use eyre::bail;

	fn test() -> eyre::Result<()> {
		bail!("something went wrong");
	}
	"#);
}

#[test]
fn return_err_eyre_with_color_eyre_import() {
	insta::assert_snapshot!(test_case(
		r#"
		use color_eyre::eyre::{Result, eyre};

		fn test() -> Result<()> {
			return Err(eyre!("something went wrong"));
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[use-bail] /main.rs:4: use `bail!(...)` instead of `return Err(eyre!(...))`

	# Format mode
	use color_eyre::eyre::{Result, eyre};
	use color_eyre::eyre::bail;

	fn test() -> Result<()> {
		bail!("something went wrong");
	}
	"#);
}

#[test]
fn multiple_return_err_eyre_in_function() {
	insta::assert_snapshot!(test_case(
		r#"
		use eyre::eyre;

		fn test(x: i32) -> eyre::Result<()> {
			if x < 0 {
				return Err(eyre!("negative value"));
			}
			if x > 100 {
				return Err(eyre!("value too large"));
			}
			Ok(())
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[use-bail] /main.rs:5: use `bail!(...)` instead of `return Err(eyre!(...))`
	[use-bail] /main.rs:8: use `bail!(...)` instead of `return Err(eyre!(...))`

	# Format mode
	use eyre::eyre;
	use eyre::bail;

	fn test(x: i32) -> eyre::Result<()> {
		if x < 0 {
			bail!("negative value");
		}
		if x > 100 {
			bail!("value too large");
		}
		Ok(())
	}
	"#);
}

#[test]
fn return_err_with_format_args() {
	insta::assert_snapshot!(test_case(
		r#"
		use eyre::eyre;

		fn test(x: i32) -> eyre::Result<()> {
			return Err(eyre!("invalid value: {}", x));
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[use-bail] /main.rs:4: use `bail!(...)` instead of `return Err(eyre!(...))`

	# Format mode
	use eyre::eyre;
	use eyre::bail;

	fn test(x: i32) -> eyre::Result<()> {
		bail!("invalid value: {}" , x);
	}
	"#);
}

#[test]
fn bail_import_not_added_when_present() {
	insta::assert_snapshot!(test_case(
		r#"
		use eyre::{eyre, bail};

		fn test() -> eyre::Result<()> {
			return Err(eyre!("error"));
		}
		"#,
		&opts(),
	), @r#"
	# Assert mode
	[use-bail] /main.rs:4: use `bail!(...)` instead of `return Err(eyre!(...))`

	# Format mode
	use eyre::{eyre, bail};

	fn test() -> eyre::Result<()> {
		bail!("error");
	}
	"#);
}
