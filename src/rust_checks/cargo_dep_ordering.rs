use std::path::Path;

use super::{Fix, Violation};

const RULE: &str = "cargo-dep-ordering";

/// Sections we care about (but NOT [patch.crates-io] etc.)
const DEP_SECTIONS: &[&str] = &["[dependencies]", "[dev-dependencies]", "[build-dependencies]"];

pub fn check(path: &Path, content: &str) -> Vec<Violation> {
	let path_str = path.display().to_string();
	let mut violations = Vec::new();

	for &section_header in DEP_SECTIONS {
		if let Some(v) = check_section(content, section_header, &path_str) {
			violations.push(v);
		}
	}

	violations
}
struct DepEntry {
	/// The full original line
	line: String,
	group: DepGroup,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DepGroup {
	/// Path dependencies (value contains `path = "../`)
	Path = 0,
	/// Regular registry dependencies
	Regular = 1,
	/// Workspace dependencies (`name.workspace = true` or `name = { workspace = true }`)
	Workspace = 2,
}

fn check_section(content: &str, section_header: &str, path_str: &str) -> Option<Violation> {
	let (section_start, section_body_start, section_end) = find_section(content, section_header)?;

	let body = &content[section_body_start..section_end];
	if body.trim().is_empty() {
		return None;
	}

	let entries = parse_entries(body);
	if entries.is_empty() {
		return None;
	}

	let formatted = format_entries(&entries);
	let current = body.trim_end_matches('\n');

	if current == formatted {
		return None;
	}

	// Find line number of the section header
	let line = content[..section_start].lines().count() + 1;

	// If there's a next section after this body, preserve the blank line separator
	let has_next_section = section_end < content.len();
	let replacement = if has_next_section { format!("{formatted}\n\n") } else { format!("{formatted}\n") };

	Some(Violation {
		rule: RULE,
		file: path_str.to_string(),
		line,
		column: 1,
		message: format!("Dependencies in {section_header} are not properly grouped/ordered"),
		fix: Some(Fix {
			start_byte: section_body_start,
			end_byte: section_end,
			replacement,
		}),
	})
}

/// Find a TOML section by header. Returns (header_start_byte, body_start_byte, body_end_byte).
/// body_end_byte is right before the next section header (or EOF).
fn find_section(content: &str, header: &str) -> Option<(usize, usize, usize)> {
	let header_lower = header.to_lowercase();
	let mut pos = 0;

	while pos < content.len() {
		let remaining = &content[pos..];
		let line_end = remaining.find('\n').unwrap_or(remaining.len());
		let line = remaining[..line_end].trim();

		if line.to_lowercase() == header_lower {
			let header_start = pos;
			let body_start = pos + line_end + 1;
			// Find the end: next section header or EOF
			let body_end = find_next_section_start(content, body_start).unwrap_or(content.len());
			return Some((header_start, body_start, body_end));
		}

		pos += line_end + 1;
	}

	None
}

/// Find the byte position of the next `[...]` section header after `from`.
fn find_next_section_start(content: &str, from: usize) -> Option<usize> {
	let mut pos = from;

	while pos < content.len() {
		let remaining = &content[pos..];
		let line_end = remaining.find('\n').unwrap_or(remaining.len());
		let line = remaining[..line_end].trim();

		if line.starts_with('[') {
			return Some(pos);
		}

		pos += line_end + 1;
	}

	None
}

fn parse_entries(body: &str) -> Vec<DepEntry> {
	let mut entries = Vec::new();

	for line in body.lines() {
		let trimmed = line.trim();
		if trimmed.is_empty() || trimmed.starts_with('#') {
			continue;
		}

		// Must be a `name = ...` or `name.workspace = true` line
		if !trimmed.contains('=') {
			continue;
		}
		let group = classify_dep(trimmed);
		let normalized = normalize_workspace_syntax(trimmed);

		entries.push(DepEntry { line: normalized, group });
	}

	entries
}

fn classify_dep(line: &str) -> DepGroup {
	// Workspace: `name.workspace = true` or `name = { workspace = true }`
	if line.contains(".workspace") || (line.contains("workspace") && line.contains("true")) {
		return DepGroup::Workspace;
	}

	// Path with `../`: this is a relative path dep pointing outside the crate
	if line.contains("path") && line.contains("\"../") {
		return DepGroup::Path;
	}

	DepGroup::Regular
}

/// Normalize `name = { workspace = true }` to `name.workspace = true`
fn normalize_workspace_syntax(line: &str) -> String {
	// Match: `name = { workspace = true }` (possibly with trailing comment)
	let trimmed = line.trim();

	if let Some(eq_pos) = trimmed.find('=') {
		let name = trimmed[..eq_pos].trim();
		let value = trimmed[eq_pos + 1..].trim();

		// Check if value is `{ workspace = true }` possibly with trailing comment
		if let Some(rest) = value.strip_prefix('{') {
			let (brace_content, after_brace) = if let Some(close) = rest.find('}') {
				(&rest[..close], rest[close + 1..].trim())
			} else {
				return line.to_string();
			};

			let inner = brace_content.trim();
			if inner == "workspace = true" {
				let mut result = format!("{name}.workspace = true");
				if !after_brace.is_empty() {
					result.push(' ');
					result.push_str(after_brace);
				}
				return result;
			}
		}
	}

	line.to_string()
}

fn format_entries(entries: &[DepEntry]) -> String {
	let mut path_deps: Vec<&str> = Vec::new();
	let mut regular_deps: Vec<&str> = Vec::new();
	let mut workspace_deps: Vec<&str> = Vec::new();

	for entry in entries {
		match entry.group {
			DepGroup::Path => path_deps.push(&entry.line),
			DepGroup::Regular => regular_deps.push(&entry.line),
			DepGroup::Workspace => workspace_deps.push(&entry.line),
		}
	}

	// Sort each group alphabetically
	path_deps.sort();
	regular_deps.sort();
	workspace_deps.sort();

	let mut groups: Vec<String> = Vec::new();

	if !path_deps.is_empty() {
		groups.push(path_deps.join("\n"));
	}
	if !regular_deps.is_empty() {
		groups.push(regular_deps.join("\n"));
	}
	if !workspace_deps.is_empty() {
		groups.push(workspace_deps.join("\n"));
	}

	groups.join("\n\n")
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn normalize_brace_workspace() {
		assert_eq!(normalize_workspace_syntax("serde = { workspace = true }"), "serde.workspace = true");
	}

	#[test]
	fn normalize_brace_workspace_with_comment() {
		assert_eq!(normalize_workspace_syntax("serde = { workspace = true } # important"), "serde.workspace = true # important");
	}

	#[test]
	fn normalize_dotted_workspace_unchanged() {
		assert_eq!(normalize_workspace_syntax("serde.workspace = true"), "serde.workspace = true");
	}

	#[test]
	fn classify_path_dep() {
		assert_eq!(classify_dep(r#"foo = { path = "../foo" }"#), DepGroup::Path);
	}

	#[test]
	fn classify_regular_dep() {
		assert_eq!(classify_dep(r#"tokio = { version = "^1", features = ["full"] }"#), DepGroup::Regular);
	}

	#[test]
	fn classify_workspace_dep() {
		assert_eq!(classify_dep("serde.workspace = true"), DepGroup::Workspace);
	}
}
