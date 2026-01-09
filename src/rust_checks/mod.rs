pub mod embed_simple_vars;
pub mod impl_follows_type;
pub mod insta_snapshots;
pub mod instrument;
pub mod join_split_impls;
pub mod loops;
pub mod no_chrono;
pub mod no_tokio_spawn;

use std::{
	collections::HashMap,
	fs,
	path::{Path, PathBuf},
};

use smart_default::SmartDefault;
use syn::{ItemFn, parse_file};
use walkdir::WalkDir;

#[derive(Clone, SmartDefault)]
pub struct RustCheckOptions {
	/// Check for #[instrument] on async functions (default: false)
	#[default = false]
	pub instrument: bool,
	/// Check for //LOOP comments on endless loops (default: true)
	#[default = true]
	pub loops: bool,
	/// Join split impl blocks for the same type (default: true)
	#[default = true]
	pub join_split_impls: bool,
	/// Check that impl blocks follow type definitions (default: true)
	#[default = true]
	pub impl_follows_type: bool,
	/// Check for simple vars that should be embedded in format strings (default: true)
	#[default = true]
	pub embed_simple_vars: bool,
	/// Check that insta snapshots use inline @"" syntax (default: true)
	#[default = true]
	pub insta_inline_snapshot: bool,
	/// Disallow usage of chrono crate (use jiff instead) (default: true)
	#[default = true]
	pub no_chrono: bool,
	/// Disallow usage of tokio::spawn (default: true)
	#[default = true]
	pub no_tokio_spawn: bool,
}

#[derive(Clone, Default, derive_new::new)]
pub struct FileInfo {
	pub contents: String,
	pub syntax_tree: Option<syn::File>,
	pub fn_items: Vec<ItemFn>,
	pub path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct Violation {
	pub rule: &'static str,
	pub file: String,
	pub line: usize,
	pub column: usize,
	pub message: String,
	pub fix: Option<Fix>,
}

#[derive(Clone, Debug)]
pub struct Fix {
	pub start_byte: usize,
	pub end_byte: usize,
	pub replacement: String,
}

pub fn run_assert(target_dir: &Path, opts: &RustCheckOptions) -> i32 {
	if !target_dir.exists() {
		eprintln!("Target directory does not exist: {:?}", target_dir);
		return 1;
	}

	let src_dirs = find_src_dirs(target_dir);
	if src_dirs.is_empty() {
		eprintln!("No source directories found");
		return 1;
	}

	let mut all_violations = Vec::new();

	for src_dir in src_dirs {
		let file_infos = collect_rust_files(&src_dir);
		for info in &file_infos {
			if opts.instrument {
				all_violations.extend(instrument::check_instrument(info));
			}
			if opts.loops {
				all_violations.extend(loops::check_loops(info));
			}
			if let Some(ref tree) = info.syntax_tree {
				// join_split_impls should run before impl_follows_type
				if opts.join_split_impls {
					all_violations.extend(join_split_impls::check(&info.path, &info.contents, tree));
				}
				if opts.impl_follows_type {
					all_violations.extend(impl_follows_type::check(&info.path, &info.contents, tree));
				}
				if opts.embed_simple_vars {
					all_violations.extend(embed_simple_vars::check(&info.path, &info.contents, tree));
				}
				if opts.insta_inline_snapshot {
					all_violations.extend(insta_snapshots::check(&info.path, &info.contents, tree, false));
				}
				if opts.no_chrono {
					all_violations.extend(no_chrono::check(&info.path, &info.contents, tree));
				}
				if opts.no_tokio_spawn {
					all_violations.extend(no_tokio_spawn::check(&info.path, &info.contents, tree));
				}
			}
		}
	}

	if all_violations.is_empty() {
		println!("codestyle: all checks passed");
		0
	} else {
		eprintln!("codestyle: found {} violation(s):\n", all_violations.len());
		for v in &all_violations {
			eprintln!("  [{}] {}:{}:{}: {}", v.rule, v.file, v.line, v.column, v.message);
		}
		1
	}
}

pub fn run_format(target_dir: &Path, opts: &RustCheckOptions) -> i32 {
	if !target_dir.exists() {
		eprintln!("Target directory does not exist: {:?}", target_dir);
		return 1;
	}

	let src_dirs = find_src_dirs(target_dir);
	if src_dirs.is_empty() {
		eprintln!("No source directories found");
		return 1;
	}

	let mut all_violations = Vec::new();

	for src_dir in src_dirs {
		let file_infos = collect_rust_files(&src_dir);
		for info in &file_infos {
			if opts.instrument {
				all_violations.extend(instrument::check_instrument(info));
			}
			if opts.loops {
				all_violations.extend(loops::check_loops(info));
			}
			if let Some(ref tree) = info.syntax_tree {
				// join_split_impls should run before impl_follows_type
				if opts.join_split_impls {
					all_violations.extend(join_split_impls::check(&info.path, &info.contents, tree));
				}
				if opts.impl_follows_type {
					all_violations.extend(impl_follows_type::check(&info.path, &info.contents, tree));
				}
				if opts.embed_simple_vars {
					all_violations.extend(embed_simple_vars::check(&info.path, &info.contents, tree));
				}
				if opts.insta_inline_snapshot {
					all_violations.extend(insta_snapshots::check(&info.path, &info.contents, tree, true));
				}
				if opts.no_chrono {
					all_violations.extend(no_chrono::check(&info.path, &info.contents, tree));
				}
				if opts.no_tokio_spawn {
					all_violations.extend(no_tokio_spawn::check(&info.path, &info.contents, tree));
				}
			}
		}
	}

	// Delete any .snap and .pending-snap files in the target directory (only if insta check is enabled)
	if opts.insta_inline_snapshot {
		delete_snap_files(target_dir);
	}

	if all_violations.is_empty() {
		println!("codestyle: all checks passed, nothing to format");
		0
	} else {
		let (fixed, unfixable) = apply_fixes(&all_violations);

		if fixed > 0 {
			println!("codestyle: fixed {fixed} violation(s)");
		}

		if unfixable > 0 {
			eprintln!("codestyle: {unfixable} violation(s) need manual fixing:\n");
			for v in &all_violations {
				if v.fix.is_none() {
					eprintln!("  [{}] {}:{}:{}: {}", v.rule, v.file, v.line, v.column, v.message);
				}
			}
			1
		} else {
			0
		}
	}
}

fn find_src_dirs(root: &Path) -> Vec<PathBuf> {
	let cargo_toml = root.join("Cargo.toml");
	if !cargo_toml.exists() {
		if root.exists() {
			return vec![root.to_path_buf()];
		}
		return vec![];
	}

	let content = match fs::read_to_string(&cargo_toml) {
		Ok(c) => c,
		Err(_) => return collect_standard_dirs(root),
	};

	let mut in_workspace = false;
	let mut members = Vec::new();

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
				if !member.is_empty() && !member.contains('*') {
					members.push(member.to_string());
				}
			}
		}
	}

	if members.is_empty() {
		return collect_standard_dirs(root);
	}

	let mut dirs = Vec::new();
	for m in members {
		let member_root = root.join(&m);
		dirs.extend(collect_standard_dirs(&member_root));
	}
	dirs
}

/// Collect standard Rust directories: src/, tests/, examples/, benches/
fn collect_standard_dirs(root: &Path) -> Vec<PathBuf> {
	let standard_dirs = ["src", "tests", "examples", "benches"];
	standard_dirs.iter().map(|d| root.join(d)).filter(|p| p.exists()).collect()
}

pub fn collect_rust_files(target_dir: &Path) -> Vec<FileInfo> {
	let mut file_infos = Vec::new();

	let walker = WalkDir::new(target_dir).into_iter().filter_entry(|e| {
		let name = e.file_name().to_string_lossy();
		!name.starts_with('.') && name != "target" && name != "libs"
	});

	for entry in walker.filter_map(Result::ok) {
		let path = entry.path().to_path_buf();
		if path.extension().is_some_and(|ext| ext == "rs")
			&& let Some(info) = parse_rust_file(path)
		{
			file_infos.push(info);
		}
	}
	file_infos
}

fn parse_rust_file(path: PathBuf) -> Option<FileInfo> {
	let contents = fs::read_to_string(&path).ok()?;
	let syntax_tree = match parse_file(&contents) {
		Ok(tree) => tree,
		Err(e) => {
			eprintln!("Failed to parse file {:?}: {}", path, e);
			return None;
		}
	};

	let fn_items = syntax_tree
		.items
		.iter()
		.filter_map(|item| if let syn::Item::Fn(func) = item { Some(func.clone()) } else { None })
		.collect();

	Some(FileInfo {
		contents,
		syntax_tree: Some(syntax_tree),
		fn_items,
		path,
	})
}

fn apply_fixes(violations: &[Violation]) -> (usize, usize) {
	let mut fixes_by_file: HashMap<String, Vec<&Fix>> = HashMap::new();

	for v in violations {
		if let Some(ref fix) = v.fix {
			fixes_by_file.entry(v.file.clone()).or_default().push(fix);
		}
	}

	let mut fixed_count = 0;
	let mut unfixable_count = 0;

	for (file_path, fixes) in fixes_by_file {
		let content = match fs::read_to_string(&file_path) {
			Ok(c) => c,
			Err(e) => {
				eprintln!("Warning: Failed to read {file_path} for fixing: {e}");
				unfixable_count += fixes.len();
				continue;
			}
		};

		// Deduplicate fixes by (start_byte, end_byte)
		let mut seen_positions = std::collections::HashSet::new();
		let mut unique_fixes: Vec<&Fix> = Vec::new();
		for fix in fixes {
			let key = (fix.start_byte, fix.end_byte);
			if !seen_positions.contains(&key) {
				seen_positions.insert(key);
				unique_fixes.push(fix);
			}
		}

		// Sort fixes by start position (ascending) first for overlap detection
		unique_fixes.sort_by(|a, b| a.start_byte.cmp(&b.start_byte));

		// Filter out overlapping fixes - only keep non-overlapping ones
		let non_overlapping = filter_non_overlapping(&unique_fixes);
		let skipped_due_to_overlap = unique_fixes.len() - non_overlapping.len();
		unfixable_count += skipped_due_to_overlap;

		// Now sort by start position (descending) to apply from end to beginning
		let mut non_overlapping = non_overlapping;
		non_overlapping.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));

		let mut new_content = content.clone();

		for fix in non_overlapping {
			if fix.start_byte <= new_content.len() && fix.end_byte <= new_content.len() {
				new_content.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
				fixed_count += 1;
			} else {
				unfixable_count += 1;
			}
		}

		if let Err(e) = fs::write(&file_path, new_content) {
			eprintln!("Warning: Failed to write {file_path}: {e}");
		}
	}

	for v in violations {
		if v.fix.is_none() {
			unfixable_count += 1;
		}
	}

	(fixed_count, unfixable_count)
}

/// Filter out overlapping fixes, keeping only non-overlapping ones.
/// Input must be sorted by start_byte ascending.
/// When overlaps are detected, we keep the earlier (smaller start_byte) fix.
fn filter_non_overlapping<'a>(fixes: &[&'a Fix]) -> Vec<&'a Fix> {
	let mut result = Vec::new();
	let mut last_end: Option<usize> = None;

	for fix in fixes {
		if let Some(end) = last_end {
			// Check if this fix overlaps with the previous one
			if fix.start_byte < end {
				// Overlap detected - skip this fix
				continue;
			}
		}
		result.push(*fix);
		last_end = Some(fix.end_byte);
	}

	result
}

fn delete_snap_files(target_dir: &Path) {
	let walker = WalkDir::new(target_dir).into_iter().filter_entry(|e| {
		let name = e.file_name().to_string_lossy();
		!name.starts_with('.') && name != "target"
	});

	for entry in walker.filter_map(Result::ok) {
		let path = entry.path();
		let is_snap = path.extension().is_some_and(|ext| ext == "snap" || ext == "pending-snap");
		if is_snap {
			if let Err(e) = fs::remove_file(path) {
				eprintln!("Warning: Failed to delete snap file {:?}: {}", path, e);
			} else {
				println!("codestyle: deleted snap file {:?}", path);
			}
		}
	}
}
