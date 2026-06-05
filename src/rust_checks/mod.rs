pub mod cargo_dep_ordering;
pub mod embed_simple_vars;
pub mod ignored_error;
pub mod impl_folds;
pub mod impl_follows_type;
pub mod inline_default;
pub mod insta_snapshots;
pub mod instrument;
pub mod join_split_impls;
pub mod loops;
pub mod no_chrono;
pub mod no_tokio_spawn;
pub mod path_rewrite;
pub mod prefer_ahash;
pub mod prefer_default_over_bare_new;
pub mod pub_first;
pub mod skip;
pub mod test_fn_prefix;
pub mod too_explicit;
pub mod unconventional_new;
pub mod use_bail;
pub mod workspace_dep_hoisting;

use std::{
	collections::HashSet,
	fs,
	path::{Path, PathBuf},
	process::Command,
};

use smart_default::SmartDefault;
use syn::{ItemFn, parse_file};
use walkdir::WalkDir;

pub struct Dependency {
	pub crate_name: &'static str,
	pub features: &'static [&'static str],
}

#[derive(Clone, SmartDefault)]
pub struct RustCheckOptions {
	/// Order and group dependencies in Cargo.toml (default: true)
	#[default = true]
	pub cargo_dep_ordering: bool,
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
	pub ignored_error: bool,
	/// Check that shared dependencies are hoisted to [workspace.dependencies] (default: true)
	#[default = true]
	pub workspace_dep_hoisting: bool,
	/// Inline `impl Default` bodies as field defaults (RFC 3681, Rust 1.82+) (default: true)
	#[default = true]
	pub inline_default: bool,
	/// Flag unconventional `fn new`: returning Result (rename to try_new); rewrite call-sites (default: true)
	#[default = true]
	pub unconventional_new: bool,
	/// Flag argument-less `pub fn new()` (prefer Default); rewrite call-sites to `::default()` (default: false)
	///
	/// Disabled by default because not every `new()` is semantically a Default — e.g. `OpenOptions::new()`
	/// creates an instance with all options off. `new()` is explicit about "I'm building one from scratch".
	#[default = false]
	pub prefer_default_over_bare_new: bool,
	/// Replace `HashMap` with `ahash::AHashMap` (default: false)
	#[default = false]
	pub prefer_ahash: bool,
	/// Rewrite inline fully-qualified std paths to short forms and add imports (default: true)
	#[default = true]
	pub too_explicit: bool,
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

pub fn run_assert(target_dir: &Path, opts: &RustCheckOptions, exclude: &[PathBuf]) -> i32 {
	if !target_dir.exists() {
		eprintln!("Target directory does not exist: {target_dir:?}");
		return 1;
	}

	let src_dirs = find_src_dirs(target_dir);
	if src_dirs.is_empty() {
		eprintln!("No source directories found");
		return 1;
	}

	let mut all_violations = Vec::default();

	// Cargo.toml checks
	if opts.cargo_dep_ordering {
		for toml_path in collect_cargo_tomls(target_dir) {
			if let Ok(content) = fs::read_to_string(&toml_path) {
				all_violations.extend(cargo_dep_ordering::check(&toml_path, &content));
			}
		}
	}
	if opts.workspace_dep_hoisting {
		all_violations.extend(workspace_dep_hoisting::check(target_dir));
	}

	for src_dir in src_dirs {
		let file_infos = collect_rust_files(&src_dir, exclude);
		let try_new_types = if opts.unconventional_new {
			unconventional_new::collect_try_new_types(&file_infos)
		} else {
			HashSet::default()
		};
		let nontrivial_default_types = if opts.prefer_default_over_bare_new {
			prefer_default_over_bare_new::collect_nontrivial_default_types(&file_infos)
		} else {
			HashSet::default()
		};
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
				if opts.ignored_error {
					all_violations.extend(ignored_error::check(&info.path, &info.contents, tree));
				}
				if opts.unconventional_new {
					all_violations.extend(unconventional_new::check(&info.path, &info.contents, tree, &try_new_types));
				}
				if opts.prefer_default_over_bare_new {
					all_violations.extend(prefer_default_over_bare_new::check(&info.path, &info.contents, tree, &nontrivial_default_types));
				}
				if opts.inline_default {
					all_violations.extend(inline_default::check(&info.path, &info.contents, tree));
				}
				if opts.prefer_ahash {
					all_violations.extend(prefer_ahash::check(&info.path, &info.contents, tree));
				}
				if opts.too_explicit {
					all_violations.extend(too_explicit::check(&info.path, &info.contents, tree));
				}
			}
		}
	}

	all_violations.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
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

/// Collect occurrences of audit-capable rules into per-rule markdown worktables for manual review.
///
/// Unlike [`run_assert`] (a pass/fail gate) and [`run_format`] (an auto-fixer), `audit` is a
/// collection step: it reuses each audit-capable rule's existing `check()` and renders the found
/// occurrences to a `- [ ]` checklist scaffolded with `TODO: reason`, one markdown file per rule.
///
/// Audit-capable rules are an explicit allowlist below. v1: only `ignored_error`. A rule with its
/// option disabled (e.g. `ignored_error` default-false) contributes nothing — run with
/// `--only ignored-error` to enable it. Returns 0 on success, non-zero only on IO/dir errors.
pub fn run_audit(target_dir: &Path, opts: &RustCheckOptions, exclude: &[PathBuf], audit_dir: Option<&Path>) -> i32 {
	if !target_dir.exists() {
		eprintln!("Target directory does not exist: {target_dir:?}");
		return 1;
	}

	let src_dirs = find_src_dirs(target_dir);
	if src_dirs.is_empty() {
		eprintln!("No source directories found");
		return 1;
	}

	// Group violations by rule, retaining each violation's source contents so the renderer can
	// quote the offending line back (Violation carries file/line/column but not source text).
	let mut by_rule: std::collections::BTreeMap<&'static str, Vec<(Violation, String)>> = std::collections::BTreeMap::default();

	for src_dir in src_dirs {
		let file_infos = collect_rust_files(&src_dir, exclude);
		for info in &file_infos {
			if let Some(ref tree) = info.syntax_tree {
				// Audit-capable rule allowlist. v1: ignored_error only.
				if opts.ignored_error {
					for v in ignored_error::check(&info.path, &info.contents, tree) {
						by_rule.entry(v.rule).or_default().push((v, info.contents.clone()));
					}
				}
			}
		}
	}

	let default_dir = target_dir.join("tmp/audit");
	let audit_dir = audit_dir.unwrap_or(&default_dir);
	if let Err(e) = fs::create_dir_all(audit_dir) {
		eprintln!("codestyle: failed to create audit dir {audit_dir:?}: {e}");
		return 1;
	}

	for (rule, mut group) in by_rule {
		group.sort_by(|(a, _), (b, _)| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
		let markdown = render_audit_markdown(rule, &group);
		let out_path = audit_dir.join(format!("{rule}.md"));
		if let Err(e) = fs::write(&out_path, markdown) {
			eprintln!("codestyle: failed to write {out_path:?}: {e}");
			return 1;
		}
		println!("codestyle: wrote {} occurrence(s) to {}", group.len(), out_path.display());
	}

	0
}

/// Render one rule's occurrences into a markdown worktable: a rule-specific header followed by a
/// `- [ ]` checklist item per occurrence, each with a `TODO: reason` child line. The source line is
/// quoted back from the file contents captured during collection.
fn render_audit_markdown(rule: &str, occurrences: &[(Violation, String)]) -> String {
	let mut out = String::default();
	out.push_str(audit_header(rule));
	for (v, contents) in occurrences {
		let src_line = contents.lines().nth(v.line - 1).unwrap_or("").trim();
		out.push_str(&format!("- [ ] `{}:{}:{}` — `{src_line}`\n  TODO: reason\n", v.file, v.line, v.column));
	}
	out
}

/// Rule-specific audit-file header. v1 has a single audit-capable rule; match on it explicitly so
/// adding a rule to the allowlist forces a header decision rather than silently reusing another's.
fn audit_header(rule: &str) -> &'static str {
	match rule {
		"ignored-error" => {
			"# `ignored-error` audit\n\n\
			Goal: every flagged `unwrap_or(_else/_default)` and `let _ = …` is either **KEEP** (one-line why)\n\
			or switched to **PANIC** / **ERROR** instead. No silent defaulting / discarding of state.\n\n\
			Verdict legend: `TODO` | `KEEP: <why>` | `PANIC` | `ERROR: <how>` | `REMOVE: <why dead>` | `DONE`\n\n\
			**Default decision is Error/Panic.** KEEP is a special case that must be very well justified —\n\
			if unsure, error/panic. Dead code is `REMOVE`, not kept.\n\n\
			---\n\n"
		}
		other => panic!("no audit header defined for rule {other:?} — every audit-capable rule needs one"),
	}
}

pub fn run_format(target_dir: &Path, opts: &RustCheckOptions, exclude: &[PathBuf]) -> i32 {
	if !target_dir.exists() {
		eprintln!("Target directory does not exist: {target_dir:?}");
		return 1;
	}

	let src_dirs = find_src_dirs(target_dir);
	if src_dirs.is_empty() {
		eprintln!("No source directories found");
		return 1;
	}

	let mut fixed_count = 0;
	let mut unfixable_violations = Vec::default();
	let mut modified_files: HashSet<PathBuf> = HashSet::default();

	// Cargo.toml checks
	if opts.cargo_dep_ordering {
		for toml_path in collect_cargo_tomls(target_dir) {
			if let Ok(content) = fs::read_to_string(&toml_path) {
				let violations = cargo_dep_ordering::check(&toml_path, &content);
				for v in violations {
					if let Some(fix) = v.fix {
						if fix.start_byte <= content.len() && fix.end_byte <= content.len() {
							let mut new_content = content.clone();
							new_content.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
							if fs::write(&toml_path, new_content).is_ok() {
								fixed_count += 1;
							}
						}
					} else {
						unfixable_violations.push(v);
					}
				}
			}
		}
	}

	// Process files iteratively. For most rules, files are independent. For
	// unconventional_new the try_new callsite rename is cross-file: after renaming
	// a `fn new` definition we must re-collect the type set and fix callers in
	// other files. We therefore loop project-wide until no more fixes land.
	let all_file_paths: Vec<PathBuf> = src_dirs.iter().flat_map(|d| collect_rust_files(d, exclude)).map(|f| f.path).collect();

	loop {
		// Re-collect cross-file type sets from current on-disk state each round.
		// unconventional_new needs try_new_types; prefer_default_over_bare_new needs nontrivial_default_types.
		let needs_reparse = opts.unconventional_new || opts.prefer_default_over_bare_new;
		let (try_new_types, nontrivial_default_types) = if needs_reparse {
			let file_infos: Vec<FileInfo> = all_file_paths.iter().filter_map(|p| parse_rust_file(p.clone())).collect();
			let try_new = if opts.unconventional_new {
				unconventional_new::collect_try_new_types(&file_infos)
			} else {
				HashSet::default()
			};
			let nontrivial = if opts.prefer_default_over_bare_new {
				prefer_default_over_bare_new::collect_nontrivial_default_types(&file_infos)
			} else {
				HashSet::default()
			};
			(try_new, nontrivial)
		} else {
			(HashSet::default(), HashSet::default())
		};

		let mut round_fixed = 0;
		for file_path in &all_file_paths {
			let (file_fixed, file_unfixable) = format_file_iteratively(file_path, opts, &try_new_types, &nontrivial_default_types);
			if file_fixed > 0 {
				modified_files.insert(file_path.clone());
				if opts.insta_inline_snapshot
					&& let Some(parent) = file_path.parent()
				{
					let snapshots_dir = parent.join("snapshots");
					if snapshots_dir.is_dir() {
						let _ = fs::remove_dir_all(&snapshots_dir);
					}
				}
			}
			round_fixed += file_fixed;
			unfixable_violations.extend(file_unfixable);
		}
		fixed_count += round_fixed;
		if round_fixed == 0 {
			break;
		}
	}

	if opts.prefer_ahash && !modified_files.is_empty() {
		run_cargo_add(target_dir, &modified_files, prefer_ahash::DEPENDENCIES);
	}

	if opts.inline_default && !modified_files.is_empty() {
		let mut visited_roots: HashSet<PathBuf> = HashSet::default();
		for file_path in &modified_files {
			if let Some(root_file) = find_crate_lib_or_main(file_path)
				&& visited_roots.insert(root_file.clone())
				&& let Ok(content) = fs::read_to_string(&root_file)
				&& !content.contains("default_field_values")
			{
				let injected = format!("#![feature(default_field_values)]\n{content}");
				if fs::write(&root_file, injected).is_ok() {
					fixed_count += 1;
				}
			}
		}
	}

	if fixed_count == 0 && unfixable_violations.is_empty() {
		println!("codestyle: all checks passed, nothing to format");
		0
	} else {
		if fixed_count > 0 {
			println!("codestyle: fixed {fixed_count} violation(s)");
		}

		unfixable_violations.sort_by(|a, b| a.file.cmp(&b.file).then(a.line.cmp(&b.line)));
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

pub fn collect_rust_files(target_dir: &Path, exclude: &[PathBuf]) -> Vec<FileInfo> {
	let mut file_infos = Vec::default();

	let cwd = std::env::current_dir().expect("failed to get cwd");
	let exclude_abs: Vec<PathBuf> = exclude.iter().map(|p| if p.is_absolute() { p.clone() } else { cwd.join(p) }).collect();

	let walker = WalkDir::new(target_dir).sort_by_file_name().into_iter().filter_entry(|e| {
		let name = e.file_name().to_string_lossy();
		if name.starts_with('.') || name == "target" || name == "libs" {
			return false;
		}
		if !exclude_abs.is_empty() {
			let entry_abs = if e.path().is_absolute() { e.path().to_path_buf() } else { cwd.join(e.path()) };
			if exclude_abs.iter().any(|ex| entry_abs.starts_with(ex)) {
				return false;
			}
		}
		// skip git submodule roots (directory containing a .git entry that isn't the repo root)
		e.depth() == 0 || !e.path().join(".git").exists()
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
fn format_file_iteratively(file_path: &Path, opts: &RustCheckOptions, try_new_types: &HashSet<String>, nontrivial_default_types: &HashSet<String>) -> (usize, Vec<Violation>) {
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

			if first_fix.is_none() && opts.ignored_error {
				for v in ignored_error::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.unconventional_new {
				for v in unconventional_new::check(&info.path, &info.contents, tree, try_new_types) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.prefer_default_over_bare_new {
				for v in prefer_default_over_bare_new::check(&info.path, &info.contents, tree, nontrivial_default_types) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.inline_default {
				for v in inline_default::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.prefer_ahash {
				for v in prefer_ahash::check(&info.path, &info.contents, tree) {
					if let Some(fix) = v.fix.clone() {
						first_fix = Some((v, fix));
						break;
					}
				}
			}

			if first_fix.is_none() && opts.too_explicit {
				for v in too_explicit::check(&info.path, &info.contents, tree) {
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
			return (fixed_count, collect_unfixable(&info, opts, try_new_types, nontrivial_default_types));
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

	(fixed_count, Vec::default())
}

/// Collect all unfixable violations from a file (called only on final pass)
fn collect_unfixable(info: &FileInfo, opts: &RustCheckOptions, try_new_types: &HashSet<String>, nontrivial_default_types: &HashSet<String>) -> Vec<Violation> {
	let mut unfixable = Vec::default();

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
		if opts.ignored_error {
			unfixable.extend(ignored_error::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.unconventional_new {
			unfixable.extend(unconventional_new::check(&info.path, &info.contents, tree, try_new_types).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.prefer_default_over_bare_new {
			unfixable.extend(
				prefer_default_over_bare_new::check(&info.path, &info.contents, tree, nontrivial_default_types)
					.into_iter()
					.filter(|v| v.fix.is_none()),
			);
		}
		if opts.inline_default {
			unfixable.extend(inline_default::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.prefer_ahash {
			unfixable.extend(prefer_ahash::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
		}
		if opts.too_explicit {
			unfixable.extend(too_explicit::check(&info.path, &info.contents, tree).into_iter().filter(|v| v.fix.is_none()));
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

	let members = resolve_workspace_members(root);
	if members.is_empty() {
		return collect_standard_dirs(root);
	}

	let mut dirs = Vec::default();
	for member_root in members {
		dirs.extend(collect_standard_dirs(&member_root));
	}
	dirs
}

/// Parse workspace members from Cargo.toml, expanding glob patterns.
/// Returns resolved directory paths for each member.
/// Returns empty vec if no [workspace] section or no members found.
fn resolve_workspace_members(root: &Path) -> Vec<PathBuf> {
	let cargo_toml = root.join("Cargo.toml");
	let content = match fs::read_to_string(&cargo_toml) {
		Ok(c) => c,
		Err(_) => return vec![],
	};

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
			// Simple glob: only support trailing `*` after a prefix, e.g. `foo_*`
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

/// Collect standard Rust directories: src/, tests/, examples/, benches/
fn collect_standard_dirs(root: &Path) -> Vec<PathBuf> {
	let standard_dirs = ["src", "tests", "examples", "benches"];
	standard_dirs.iter().map(|d| root.join(d)).filter(|p| p.exists()).collect()
}

/// Collect all Cargo.toml files in the workspace that may have [dependencies].
/// For a workspace root, returns member Cargo.tomls. For a standalone crate, returns its Cargo.toml.
fn collect_cargo_tomls(root: &Path) -> Vec<PathBuf> {
	let cargo_toml = root.join("Cargo.toml");
	if !cargo_toml.exists() {
		return vec![];
	}

	let members = resolve_workspace_members(root);
	if members.is_empty() {
		// Standalone crate
		return vec![cargo_toml];
	}

	members.into_iter().map(|m| m.join("Cargo.toml")).filter(|p| p.exists()).collect()
}

fn parse_rust_file(path: PathBuf) -> Option<FileInfo> {
	let contents = fs::read_to_string(&path).ok()?;
	let syntax_tree = match parse_file(&contents) {
		Ok(tree) => tree,
		Err(_) => return None,
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

/// Run `cargo add` for each dependency on every Cargo.toml that was affected by fixes.
fn run_cargo_add(target_dir: &Path, modified_files: &HashSet<PathBuf>, deps: &[Dependency]) {
	if deps.is_empty() {
		return;
	}

	// Collect unique Cargo.toml paths for modified files
	let mut seen: HashSet<(PathBuf, &str)> = HashSet::default();

	for file_path in modified_files {
		let Some(cargo_toml) = find_package_cargo_toml(file_path) else {
			continue;
		};

		for dep in deps {
			if !seen.insert((cargo_toml.clone(), dep.crate_name)) {
				continue;
			}

			// Check if this package is a workspace member
			let package_name = read_package_name(&cargo_toml);
			let workspace_root = find_workspace_root(target_dir);
			let is_workspace_member = workspace_root.is_some() && package_name.is_some();

			let mut cmd = Command::new("cargo");
			cmd.arg("add");

			if is_workspace_member {
				cmd.arg("-p").arg(package_name.as_deref().unwrap());
			}

			cmd.arg(dep.crate_name);

			if !dep.features.is_empty() {
				cmd.arg("--features").arg(dep.features.join(","));
			}

			// Run from workspace root or package dir
			let run_dir = workspace_root.as_deref().unwrap_or_else(|| cargo_toml.parent().unwrap_or(target_dir));
			cmd.current_dir(run_dir);

			match cmd.status() {
				Ok(status) if status.success() => {
					println!("codestyle: ran `cargo add {}` for {}", dep.crate_name, cargo_toml.display());
				}
				Ok(status) => {
					eprintln!("codestyle: `cargo add {}` exited with {status}", dep.crate_name);
				}
				Err(e) => {
					eprintln!("codestyle: failed to run `cargo add {}`: {e}", dep.crate_name);
				}
			}
		}
	}
}

/// Walk up from `file_path` to find the nearest `Cargo.toml` containing `[package]`.
fn find_package_cargo_toml(file_path: &Path) -> Option<PathBuf> {
	let mut dir = file_path.parent()?;
	loop {
		let candidate = dir.join("Cargo.toml");
		if candidate.exists()
			&& let Ok(content) = fs::read_to_string(&candidate)
			&& content.contains("[package]")
		{
			return Some(candidate);
		}
		dir = dir.parent()?;
	}
}

/// Read the `name` field from `[package]` in a Cargo.toml.
fn read_package_name(cargo_toml: &Path) -> Option<String> {
	let content = fs::read_to_string(cargo_toml).ok()?;
	let mut in_package = false;
	for line in content.lines() {
		let trimmed = line.trim();
		if trimmed == "[package]" {
			in_package = true;
		} else if trimmed.starts_with('[') {
			in_package = false;
		} else if in_package && let Some(rest) = trimmed.strip_prefix("name") {
			let rest = rest.trim();
			if let Some(rest) = rest.strip_prefix('=') {
				let name = rest.trim().trim_matches('"').trim_matches('\'');
				if !name.is_empty() {
					return Some(name.to_string());
				}
			}
		}
	}
	None
}

/// Walk up from `file_path` to find the crate's `src/lib.rs`, falling back to `src/main.rs`.
///
/// Works for both standalone crates and sub-crates inside a workspace: it looks for the
/// nearest ancestor directory that contains a `Cargo.toml` (i.e. the crate root), then
/// checks `<crate_root>/src/lib.rs` and `<crate_root>/src/main.rs` in that order.
pub fn find_crate_lib_or_main(file_path: &Path) -> Option<PathBuf> {
	let mut dir = file_path.parent()?;
	loop {
		if dir.join("Cargo.toml").exists() {
			let lib = dir.join("src").join("lib.rs");
			if lib.exists() {
				return Some(lib);
			}
			let main = dir.join("src").join("main.rs");
			if main.exists() {
				return Some(main);
			}
			return None;
		}
		dir = dir.parent()?;
	}
}

/// Walk up from `target_dir` to find a `Cargo.toml` with `[workspace]`.
fn find_workspace_root(target_dir: &Path) -> Option<PathBuf> {
	let mut dir = target_dir;
	loop {
		let candidate = dir.join("Cargo.toml");
		if candidate.exists()
			&& let Ok(content) = fs::read_to_string(&candidate)
			&& content.contains("[workspace]")
		{
			return Some(dir.to_path_buf());
		}
		dir = dir.parent()?;
	}
}
