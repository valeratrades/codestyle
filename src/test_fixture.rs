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
//! # Testing transformations (before -> after)
//!
//! Use the `check` pattern with `=>` separator:
//!
//! ```ignore
//! r#"
//! //- /test.rs
//! fn main() {
//!     let x = 1;
//!     println!("{}", x);
//! }
//! =>
//! //- /test.rs
//! fn main() {
//!     let x = 1;
//!     println!("{x}");
//! }
//! "#
//! ```

use std::{fs, path::PathBuf};

/// A single file in a fixture
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureFile {
	/// Path relative to fixture root (e.g., "/main.rs" or "/tests/test.rs")
	pub path: String,
	/// File contents with meta lines stripped
	pub text: String,
}

/// Parsed fixture containing multiple files
#[derive(Debug, Clone, PartialEq, Eq)]
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
		let temp_dir = std::env::temp_dir().join(format!("codestyle_fixture_{}", std::process::id()));
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
