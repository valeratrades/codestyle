use std::{
	collections::{HashMap, HashSet},
	fs,
	path::{Path, PathBuf},
};

use super::Violation;

const RULE: &str = "workspace-dep-hoisting";

/// Check that dependencies used by more than one workspace member are declared
/// at the `[workspace.dependencies]` level.
///
/// Only fires when run against a workspace root. No-op for standalone crates.
pub fn check(workspace_root: &Path) -> Vec<Violation> {
	let workspace_toml_path = workspace_root.join("Cargo.toml");
	let workspace_content = match fs::read_to_string(&workspace_toml_path) {
		Ok(c) => c,
		Err(_) => return vec![],
	};

	let members = resolve_workspace_members(workspace_root, &workspace_content);
	if members.is_empty() {
		return vec![];
	}

	let workspace_deps = parse_workspace_deps(&workspace_content);

	// dep_name -> set of member dirs that use it
	let mut dep_users: HashMap<String, HashSet<String>> = HashMap::default();

	for member_dir in &members {
		let member_toml = member_dir.join("Cargo.toml");
		let content = match fs::read_to_string(&member_toml) {
			Ok(c) => c,
			Err(_) => continue,
		};
		let member_name = member_dir
			.file_name()
			.map(|n| n.to_string_lossy().into_owned())
			.unwrap_or_else(|| member_dir.display().to_string());

		for dep in parse_regular_deps(&content) {
			dep_users.entry(dep).or_default().insert(member_name.clone());
		}
	}

	let path_str = workspace_toml_path.display().to_string();
	let mut violations: Vec<Violation> = dep_users
		.into_iter()
		.filter(|(dep, users)| users.len() >= 2 && !workspace_deps.contains(dep.as_str()))
		.map(|(dep, users)| {
			let mut sorted_users: Vec<String> = users.into_iter().collect();
			sorted_users.sort();
			Violation {
				rule: RULE,
				file: path_str.clone(),
				line: 1,
				column: 1,
				message: format!(
					"dependency `{dep}` is used by {} members ({}) but not declared in [workspace.dependencies]",
					sorted_users.len(),
					sorted_users.join(", ")
				),
				fix: None,
			}
		})
		.collect();

	violations.sort_by(|a, b| a.message.cmp(&b.message));
	violations
}

/// Parse `[workspace.dependencies]` keys from workspace Cargo.toml.
fn parse_workspace_deps(content: &str) -> HashSet<String> {
	let mut deps = HashSet::default();
	let mut in_section = false;

	for line in content.lines() {
		let trimmed = line.trim();

		if trimmed == "[workspace.dependencies]" {
			in_section = true;
			continue;
		}
		if trimmed.starts_with('[') {
			in_section = false;
			continue;
		}
		if in_section && let Some(name) = extract_dep_name(trimmed) {
			deps.insert(name);
		}
	}

	deps
}

/// Parse regular (non-workspace, non-path) dependency names from a member Cargo.toml.
/// Looks through `[dependencies]`, `[dev-dependencies]`, `[build-dependencies]`.
fn parse_regular_deps(content: &str) -> HashSet<String> {
	const DEP_SECTIONS: &[&str] = &["[dependencies]", "[dev-dependencies]", "[build-dependencies]"];
	let mut deps = HashSet::default();
	let mut in_dep_section = false;

	for line in content.lines() {
		let trimmed = line.trim();

		if DEP_SECTIONS.contains(&trimmed) {
			in_dep_section = true;
			continue;
		}
		if trimmed.starts_with('[') {
			in_dep_section = false;
			continue;
		}
		if in_dep_section {
			if is_workspace_or_path_dep(trimmed) {
				continue;
			}
			if let Some(name) = extract_dep_name(trimmed) {
				deps.insert(name);
			}
		}
	}

	deps
}

fn is_workspace_or_path_dep(line: &str) -> bool {
	// workspace = true / name.workspace = true / name = { workspace = true, ... }
	if line.contains(".workspace") || line.contains("workspace = true") {
		return true;
	}
	// path dep: path = "../..."
	if line.contains("path") && line.contains("\"../") {
		return true;
	}
	false
}

fn extract_dep_name(line: &str) -> Option<String> {
	let trimmed = line.trim();
	if trimmed.is_empty() || trimmed.starts_with('#') {
		return None;
	}
	// Must contain `=` to be a dep line
	let eq = trimmed.find('=')?;
	let raw_name = trimmed[..eq].trim();
	// dotted key like `name.workspace` — take the part before the dot
	let name = raw_name.split('.').next()?.trim();
	if name.is_empty() {
		return None;
	}
	Some(name.to_string())
}

fn resolve_workspace_members(root: &Path, content: &str) -> Vec<PathBuf> {
	let mut in_workspace = false;
	let mut patterns = Vec::default();

	for line in content.lines() {
		let trimmed = line.trim();
		if trimmed == "[workspace]" {
			in_workspace = true;
		} else if trimmed.starts_with('[') && trimmed != "[workspace]" {
			in_workspace = false;
		} else if in_workspace
			&& trimmed.starts_with("members")
			&& let Some(start) = line.find('[')
			&& let Some(end) = line.find(']')
		{
			let list = &line[start + 1..end];
			for member in list.split(',') {
				let member = member.trim().trim_matches('"').trim_matches('\'');
				if !member.is_empty() {
					patterns.push(member.to_string());
				}
			}
		}
	}

	let mut members = Vec::default();
	for pattern in patterns {
		if pattern.contains('*') {
			let prefix = pattern.trim_end_matches('*');
			let (parent, name_prefix) = if let Some(slash) = prefix.rfind('/') {
				(root.join(&prefix[..slash]), &prefix[slash + 1..])
			} else {
				(root.to_path_buf(), prefix)
			};
			if let Ok(entries) = fs::read_dir(&parent) {
				for entry in entries.filter_map(Result::ok) {
					let name = entry.file_name();
					let name = name.to_string_lossy();
					if name.starts_with(name_prefix) && entry.path().is_dir() {
						members.push(entry.path());
					}
				}
			}
		} else {
			members.push(root.join(&pattern));
		}
	}

	members
}
