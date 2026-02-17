pub mod embed_simple_vars;
pub mod ignored_error_comment;
pub mod impl_folds;
pub mod impl_follows_type;
pub mod insta_snapshots;
pub mod instrument;
pub mod join_split_impls;
pub mod loops;
pub mod no_chrono;
pub mod no_tokio_spawn;
pub mod pub_first;
pub mod skip;
pub mod test_fn_prefix;
pub mod use_bail;

use std::{
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
	/// Wrap impl blocks with vim 1-fold markers (default: false)
	#[default = false]
	pub impl_folds: bool,
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
	/// Replace `return Err(eyre!(...))` with `bail!(...)` (default: true)
	#[default = true]
	pub use_bail: bool,
	/// Check that test functions don't have redundant `test_` prefix (default: false)
	#[default = false]
	pub test_fn_prefix: bool,
	/// Check that public items come before private items (default: true)
	#[default = true]
	pub pub_first: bool,
	/// Check for //IGNORED_ERROR comments on unwrap_or/unwrap_or_default/unwrap_or_else and `let _ = ...` (default: true)
	#[default = false] // useful, but too many false positives. Sadly, the time commitment might not be worth it, unless I somehow make this smarter
	pub ignored_error_comment: bool,
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
		eprintln!("Target directory does not exist: {target_dir:?}");
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
				// Order matters: join_split_impls -> impl_follows_type -> impl_folds
				if opts.join_split_impls {
					all_violations.extend(join_split_impls::check(&info.path, &info.contents, tree));
				}
				if opts.impl_follows_type {
					all_violations.extend(impl_follows_type::check(&info.path, &info.contents, tree));
				}
				if opts.impl_folds {
					all_violations.extend(impl_folds::check(&info.path, &info.contents, tree));
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
				if opts.use_bail {
					all_violations.extend(use_bail::check(&info.path, &info.contents, tree));
				}
				if opts.test_fn_prefix {
					all_violations.extend(test_fn_prefix::check(&info.path, &info.contents, tree));
				}
				if opts.pub_first {
					all_violations.extend(pub_first::check(&info.path, &info.contents, tree));
				}
				if opts.ignored_error_comment {
					all_violations.extend(ignored_error_comment::check(&info.path, &info.contents, tree));
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
		eprintln!("Target directory does not exist: {target_dir:?}");
		return 1;
	}

	let src_dirs = find_src_dirs(target_dir);
	if src_dirs.is_empty() {
		eprintln!("No source directories found");
		return 1;
	}

	// Delete any .snap and .pending-snap files in the target directory (only if insta check is enabled)
	if opts.insta_inline_snapshot {
		delete_snap_files(target_dir);
	}

	let mut fixed_count = 0;
	let mut unfixable_violations = Vec::new();

	// Process files iteratively - when a fix is applied, re-check that file
	for src_dir in src_dirs {
		let file_paths: Vec<PathBuf> = collect_rust_files(&src_dir).into_iter().map(|f| f.path).collect();

		for file_path in file_paths {
			let (file_fixed, file_unfixable) = format_file_iteratively(&file_path, opts);
			fixed_count += file_fixed;
			unfixable_violations.extend(file_unfixable);
		}
	}

	if fixed_count == 0 && unfixable_violations.is_empty() {
		println!("codestyle: all checks passed, nothing to format");
		0
	} else {
		if fixed_count > 0 {
			println!("codestyle: fixed {fixed_count} violation(s)");
		}

		if !unfixable_violations.is_empty() {
			eprintln!("codestyle: {} violation(s) need manual fixing:\n", unfixable_violations.len());
			for v in &unfixable_violations {
				eprintln!("  [{}] {}:{}:{}: {}", v.rule, v.file, v.line, v.column, v.message);
			}
			1
		} else {
			0
		}
	}
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
/// Format a single file iteratively - apply one fix at a time, re-parse, repeat.
/// Unfixable violations are only collected on the final pass (when no more fixes are found),
/// ensuring line numbers are stable and no duplicates are reported.
fn format_file_iteratively(file_path: &Path, opts: &RustCheckOptions) -> (usize, Vec<Violation>) {
	let mut fixed_count = 0;

	loop {
		let Some(info) = parse_rust_file(file_path.to_path_buf()) else {
			break;
		};

		// Find the first fixable violation
		let mut first_fix: Option<(Violation, Fix)> = None;

		if opts.instrument {
			for v in instrument::check_instrument(&info) {
				if let Some(fix) = v.fix.clone() {
					first_fix = Some((v, fix));
					break;
				}
			}
		}

		if first_fix.is_none() && opts.loops {
			for v in loops::check_loops(&info) {
				if let Some(fix) = v.fix.clone() {
					first_fix = Some((v, fix));
					break;
				}
			}
		}

		if let Some(ref tree) = info.syntax_tree {
			// Order matters: join_split_impls -> impl_follows_type -> impl_folds
			if first_fix.is_none() && opts.join_split_impls {
				for v in join_split_impls::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.impl_follows_type {
				for v in impl_follows_type::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.impl_folds {
				for v in impl_folds::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.embed_simple_vars {
				for v in embed_simple_vars::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.insta_inline_snapshot {
				for v in insta_snapshots::check(&info.path, &info.contents, tree, true) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.no_chrono {
				for v in no_chrono::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.no_tokio_spawn {
				for v in no_tokio_spawn::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.use_bail {
				for v in use_bail::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.test_fn_prefix {
				for v in test_fn_prefix::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.pub_first {
				for v in pub_first::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.ignored_error_comment {
				for v in ignored_error_comment::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}
		}

		// Apply the fix if found
		let Some((_violation, fix)) = first_fix else {
			// No more fixes - collect unfixable violations now (final pass)
			return (fixed_count, collect_unfixable(&info, opts));
		};

		if fix.start_byte <= info.contents.len() && fix.end_byte <= info.contents.len() {
			let mut new_content = info.contents.clone();
			new_content.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
			if fs::write(file_path, new_content).is_ok() {
				fixed_count += 1;
				// Loop again to find more violations in the modified file
				continue;
			}
		}

		break;
	}

	(fixed_count, Vec::new())
}

/// Collect all unfixable violations from a file (called only on final pass)
fn collect_unfixable(info: &FileInfo, opts: &RustCheckOptions) -> Vec<Violation> {
	let mut unfixable = Vec::new();

	if opts.instrument {
		unfixable.extend(instrument::check_instrument(info).into_iter().filter(|v| v.fix.is_none()));
	}
	if opts.loops {
		unfixable.extend(loops::check_loops(info).into_iter().filter(|v| v.fix.is_none()));
	}
	if let Some(ref tree) = info.syntax_tree {
		if opts.join_split_impls {
			unfixable.extend(join_split_impls::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.impl_follows_type {
			unfixable.extend(impl_follows_type::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.impl_folds {
			unfixable.extend(impl_folds::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.embed_simple_vars {
			unfixable.extend(embed_simple_vars::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.insta_inline_snapshot {
			unfixable.extend(insta_snapshots::check(&info.path, &info.contents, tree, true).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.no_chrono {
			unfixable.extend(no_chrono::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.no_tokio_spawn {
			unfixable.extend(no_tokio_spawn::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.use_bail {
			unfixable.extend(use_bail::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.test_fn_prefix {
			unfixable.extend(test_fn_prefix::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.pub_first {
			unfixable.extend(pub_first::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.ignored_error_comment {
			unfixable.extend(ignored_error_comment::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
	}

	unfixable
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

fn parse_rust_file(path: PathBuf) -> Option<FileInfo> {
	let contents = fs::read_to_string(&path).ok()?;
	let syntax_tree = match parse_file(&contents) {
		Ok(tree) => tree,
		Err(e) => {
			eprintln!("Failed to parse file {path:?}: {e}");
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

fn delete_snap_files(target_dir: &Path) {
	let walker = WalkDir::new(target_dir).into_iter().filter_entry(|e| {
		let name = e.file_name().to_string_lossy();
		!name.starts_with('.') && name != "target"
	});

	let mut snapshot_dirs_to_delete = Vec::new();

	for entry in walker.filter_map(Result::ok) {
		let path = entry.path();

		// If we find a snapshots/ directory, mark it for deletion
		if path.is_dir() && path.file_name().is_some_and(|n| n == "snapshots") {
			// Check if it contains any .snap or .pending-snap files
			let has_snap_files = WalkDir::new(path)
				.into_iter()
				.filter_map(Result::ok)
				.any(|e| e.path().extension().is_some_and(|ext| ext == "snap" || ext == "pending-snap"));
			if has_snap_files {
				snapshot_dirs_to_delete.push(path.to_path_buf());
			}
		}
	}

	// Delete snapshots/ directories (this also removes all files inside)
	for dir in snapshot_dirs_to_delete {
		if let Err(e) = fs::remove_dir_all(&dir) {
			eprintln!("Warning: Failed to delete snapshots dir {dir:?}: {e}");
		} else {
			println!("codestyle: deleted snapshots dir {dir:?}");
		}
	}
}
