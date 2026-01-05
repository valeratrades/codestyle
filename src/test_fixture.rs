//! Test fixture utilities for defining multi-file test cases inline.
//!
//! This module provides a way to define file trees inline in test code using
//! the `//- /path.rs` syntax inspired by rust-analyzer.
//!
//! # Single file fixture
//!
//! ```ignore
//! r#"
//! fn main() {
//!     println!("Hello World")
//! }
//! "#
//! ```
//!
//! # Multi-file fixture
//!
//! ```ignore
//! r#"
//! //- /main.rs
//! mod foo;
//! fn main() { foo::bar(); }
//!
//! //- /foo.rs
//! pub fn bar() {}
//! "#
//! ```
//!
//! # Testing with insta snapshots
//!
//! Use `simulate_check` which returns a string for snapshot testing:
//!
//! ```ignore
//! // Test violation detection - returns violations as readable string
//! insta::assert_snapshot!(simulate_check(r#"
//!     fn test() {
//!         insta::assert_snapshot!(output);
//!     }
//! "#, &opts), @"...");
//!
//! // Test auto-fix - returns the modified fixture
//! insta::assert_snapshot!(simulate_format(r#"
//!     struct Foo;
//!     impl Foo { fn one() {} }
//!     impl Foo { fn two() {} }
//! "#, &opts), @"...");
//! ```

use std::{collections::HashMap, fs, path::PathBuf};

use crate::rust_checks::{self, Fix, RustCheckOptions, Violation};

/// A single file in a fixture
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FixtureFile {
	/// Path relative to fixture root (e.g., "/main.rs" or "/tests/test.rs")
	pub path: String,
	/// File contents with meta lines stripped
	pub text: String,
}

/// Parsed fixture containing multiple files
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Fixture {
	pub files: Vec<FixtureFile>,
}

impl Fixture {
	/// Parse a fixture string into files.
	///
	/// Supports the `//- /path.rs` syntax for multi-file fixtures.
	/// If no file markers are present, treats the whole string as a single `/main.rs` file.
	pub fn parse(fixture: &str) -> Self {
		let fixture = trim_indent(fixture);
		let fixture = fixture.as_str();

		let mut files = Vec::new();

		if !fixture.contains("//-") {
			// Single file fixture - treat as /main.rs
			return Self {
				files: vec![FixtureFile {
					path: "/main.rs".to_owned(),
					text: fixture.to_owned(),
				}],
			};
		}

		let mut current_path: Option<String> = None;
		let mut current_text = String::new();

		for line in fixture.split_inclusive('\n') {
			if let Some(rest) = line.strip_prefix("//-") {
				// Save previous file if any
				if let Some(path) = current_path.take() {
					files.push(FixtureFile {
						path,
						text: std::mem::take(&mut current_text),
					});
				}

				// Parse new file path
				let meta = rest.trim();
				let path = meta.split_whitespace().next().expect("fixture meta must have a path");
				assert!(path.starts_with('/'), "fixture path must start with `/`: {path:?}");
				current_path = Some(path.to_owned());
			} else if current_path.is_some() {
				current_text.push_str(line);
			}
		}

		// Save last file
		if let Some(path) = current_path {
			files.push(FixtureFile { path, text: current_text });
		}

		Self { files }
	}

	/// Write fixture files to a temporary directory and return the path
	pub fn write_to_tempdir(&self) -> TempFixture {
		use std::sync::atomic::{AtomicU64, Ordering};
		static COUNTER: AtomicU64 = AtomicU64::new(0);
		let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
		let temp_dir = std::env::temp_dir().join(format!("codestyle_fixture_{}_{}", std::process::id(), unique_id));
		fs::create_dir_all(&temp_dir).expect("failed to create temp dir");

		for file in &self.files {
			let path = temp_dir.join(file.path.trim_start_matches('/'));
			if let Some(parent) = path.parent() {
				fs::create_dir_all(parent).expect("failed to create parent dirs");
			}
			fs::write(&path, &file.text).expect("failed to write fixture file");
		}

		TempFixture {
			root: temp_dir,
			files: self.files.clone(),
		}
	}

	/// Get a file by path
	pub fn file(&self, path: &str) -> Option<&FixtureFile> {
		self.files.iter().find(|f| f.path == path)
	}

	/// Get the single file (panics if multiple files)
	pub fn single_file(&self) -> &FixtureFile {
		assert_eq!(self.files.len(), 1, "expected single file fixture");
		&self.files[0]
	}
}

/// A fixture written to a temporary directory
pub struct TempFixture {
	pub root: PathBuf,
	pub files: Vec<FixtureFile>,
}

impl TempFixture {
	/// Get the full path to a file
	pub fn path(&self, relative: &str) -> PathBuf {
		self.root.join(relative.trim_start_matches('/'))
	}

	/// Read a file's current contents
	pub fn read(&self, relative: &str) -> String {
		fs::read_to_string(self.path(relative)).expect("failed to read file")
	}

	/// Read all files and return as a new Fixture
	pub fn read_all(&self) -> Fixture {
		let files = self
			.files
			.iter()
			.map(|f| {
				let text = self.read(&f.path);
				FixtureFile { path: f.path.clone(), text }
			})
			.collect();
		Fixture { files }
	}
}

impl Drop for TempFixture {
	fn drop(&mut self) {
		let _ = fs::remove_dir_all(&self.root);
	}
}

/// Parse a before/after fixture separated by `=>`
///
/// Returns (before_fixture, after_fixture)
pub fn parse_before_after(fixture: &str) -> (Fixture, Fixture) {
	let fixture = trim_indent(fixture);
	let parts: Vec<&str> = fixture.split("\n=>\n").collect();
	assert_eq!(parts.len(), 2, "expected exactly one `=>` separator in before/after fixture");

	let before = Fixture::parse(parts[0]);
	let after = Fixture::parse(parts[1]);

	(before, after)
}

/// Remove common leading indentation from all lines.
///
/// This allows writing nicely indented fixture strings in tests.
pub fn trim_indent(text: &str) -> String {
	let mut text = text;
	if text.starts_with('\n') {
		text = &text[1..];
	}
	let indent = text.lines().filter(|it| !it.trim().is_empty()).map(|it| it.len() - it.trim_start().len()).min().unwrap_or(0);
	text.split_inclusive('\n')
		.map(|line| if line.len() <= indent { line.trim_start_matches(' ') } else { &line[indent..] })
		.collect()
}

/// Compare two fixtures for equality, with nice diff output on failure
pub fn assert_fixture_eq(expected: &Fixture, actual: &Fixture) {
	if expected.files.len() != actual.files.len() {
		panic!(
			"fixture file count mismatch: expected {} files, got {}\nExpected: {:?}\nActual: {:?}",
			expected.files.len(),
			actual.files.len(),
			expected.files.iter().map(|f| &f.path).collect::<Vec<_>>(),
			actual.files.iter().map(|f| &f.path).collect::<Vec<_>>()
		);
	}

	for expected_file in &expected.files {
		let actual_file = actual.file(&expected_file.path).unwrap_or_else(|| {
			panic!(
				"missing file in actual: {}\nActual files: {:?}",
				expected_file.path,
				actual.files.iter().map(|f| &f.path).collect::<Vec<_>>()
			)
		});

		if expected_file.text != actual_file.text {
			panic!(
				"file {} content mismatch:\n\n--- Expected ---\n{}\n--- Actual ---\n{}\n",
				expected_file.path, expected_file.text, actual_file.text
			);
		}
	}
}

/// Render a fixture back to string format (for snapshots)
pub fn render_fixture(fixture: &Fixture) -> String {
	if fixture.files.len() == 1 {
		return fixture.files[0].text.clone();
	}

	let mut result = String::new();
	for file in &fixture.files {
		result.push_str("//- ");
		result.push_str(&file.path);
		result.push('\n');
		result.push_str(&file.text);
		if !file.text.ends_with('\n') {
			result.push('\n');
		}
	}
	result
}

/// Simulate running `codestyle rust assert` on a fixture.
///
/// Returns a string representation of violations suitable for snapshot testing.
/// Format: one violation per line as `[rule] /path:line: message`
///
/// If no violations, returns `(no violations)`
pub fn simulate_check(fixture_str: &str, opts: &RustCheckOptions) -> String {
	let fixture = Fixture::parse(fixture_str);
	let temp = fixture.write_to_tempdir();

	let violations = collect_violations(&temp.root, opts);

	if violations.is_empty() {
		return "(no violations)".to_string();
	}

	// Format violations relative to fixture root
	violations
		.iter()
		.map(|v| {
			let relative_path = v.file.strip_prefix(temp.root.to_str().unwrap_or("")).unwrap_or(&v.file);
			let relative_path = relative_path.trim_start_matches('/');
			format!("[{}] /{}:{}: {}", v.rule, relative_path, v.line, v.message)
		})
		.collect::<Vec<_>>()
		.join("\n")
}

/// Simulate running `codestyle rust format` on a fixture.
///
/// Returns the fixture after applying all auto-fixes, in fixture format.
/// This allows snapshot testing of the transformation result.
pub fn simulate_format(fixture_str: &str, opts: &RustCheckOptions) -> String {
	let fixture = Fixture::parse(fixture_str);
	let temp = fixture.write_to_tempdir();

	let violations = collect_violations(&temp.root, opts);

	// Group fixes by file, deduplicating by (start_byte, end_byte)
	let mut fixes_by_file: HashMap<String, Vec<&Fix>> = HashMap::new();
	for v in &violations {
		if let Some(ref fix) = v.fix {
			let fixes = fixes_by_file.entry(v.file.clone()).or_default();
			// Deduplicate fixes with same byte range
			if !fixes.iter().any(|f| f.start_byte == fix.start_byte && f.end_byte == fix.end_byte) {
				fixes.push(fix);
			}
		}
	}

	// Apply fixes to each file
	for (file_path, fixes) in fixes_by_file {
		let content = match fs::read_to_string(&file_path) {
			Ok(c) => c,
			Err(_) => continue,
		};

		// Sort fixes by position (descending) to apply from end to start
		let mut fixes = fixes;
		fixes.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

		let mut new_content = content;
		for fix in fixes {
			if fix.start_byte <= new_content.len() && fix.end_byte <= new_content.len() {
				new_content.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
			}
		}

		let _ = fs::write(&file_path, new_content);
	}

	// Read back all files and render as fixture
	let result = temp.read_all();
	render_fixture(&result)
}

/// Collect all violations from a directory using the given options.
fn collect_violations(root: &PathBuf, opts: &RustCheckOptions) -> Vec<Violation> {
	use crate::rust_checks::{embed_simple_vars, impl_follows_type, insta_snapshots, instrument, join_split_impls, loops};

	let file_infos = rust_checks::collect_rust_files(root);
	let mut violations = Vec::new();

	for info in &file_infos {
		if opts.instrument {
			violations.extend(instrument::check_instrument(info));
		}
		if opts.loops {
			violations.extend(loops::check_loops(info));
		}
		if let Some(ref tree) = info.syntax_tree {
			if opts.join_split_impls {
				violations.extend(join_split_impls::check(&info.path, &info.contents, tree));
			}
			if opts.impl_follows_type {
				violations.extend(impl_follows_type::check(&info.path, &info.contents, tree));
			}
			if opts.embed_simple_vars {
				violations.extend(embed_simple_vars::check(&info.path, &info.contents, tree));
			}
			if opts.insta_inline_snapshot {
				violations.extend(insta_snapshots::check(&info.path, &info.contents, tree, false));
			}
		}
	}

	violations
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_trim_indent() {
		let input = r#"
            fn main() {
                println!("hello");
            }
        "#;
		let expected = "fn main() {\n    println!(\"hello\");\n}\n";
		assert_eq!(trim_indent(input), expected);
	}

	#[test]
	fn test_parse_single_file() {
		let input = r#"
            fn main() {
                println!("hello");
            }
        "#;
		let fixture = Fixture::parse(input);
		assert_eq!(fixture.files.len(), 1);
		assert_eq!(fixture.files[0].path, "/main.rs");
		assert!(fixture.files[0].text.contains("fn main()"));
	}

	#[test]
	fn test_parse_multi_file() {
		let input = r#"
            //- /main.rs
            mod foo;
            fn main() { foo::bar(); }

            //- /foo.rs
            pub fn bar() {}
        "#;
		let fixture = Fixture::parse(input);
		assert_eq!(fixture.files.len(), 2);
		assert_eq!(fixture.files[0].path, "/main.rs");
		assert!(fixture.files[0].text.contains("mod foo"));
		assert_eq!(fixture.files[1].path, "/foo.rs");
		assert!(fixture.files[1].text.contains("pub fn bar"));
	}

	#[test]
	fn test_parse_nested_paths() {
		let input = r#"
            //- /src/main.rs
            mod lib;

            //- /tests/test.rs
            fn test() {}
        "#;
		let fixture = Fixture::parse(input);
		assert_eq!(fixture.files.len(), 2);
		assert_eq!(fixture.files[0].path, "/src/main.rs");
		assert_eq!(fixture.files[1].path, "/tests/test.rs");
	}

	#[test]
	fn test_parse_before_after() {
		let input = r#"
            //- /test.rs
            fn main() { let x = 1; }
            =>
            //- /test.rs
            fn main() { let y = 1; }
        "#;
		let (before, after) = parse_before_after(input);
		assert!(before.files[0].text.contains("let x"));
		assert!(after.files[0].text.contains("let y"));
	}

	#[test]
	fn test_render_fixture() {
		let fixture = Fixture {
			files: vec![
				FixtureFile {
					path: "/main.rs".to_owned(),
					text: "fn main() {}\n".to_owned(),
				},
				FixtureFile {
					path: "/lib.rs".to_owned(),
					text: "pub fn lib() {}\n".to_owned(),
				},
			],
		};
		let rendered = render_fixture(&fixture);
		assert!(rendered.contains("//- /main.rs"));
		assert!(rendered.contains("//- /lib.rs"));
	}
}
