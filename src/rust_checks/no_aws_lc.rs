//! Flag dependency declarations that pull `aws-lc-sys` (a bundled C library rebuilt per target dir) into the graph.
//!
//! Attribution needs feature activations, which `Cargo.lock` does not record, so the graph comes from
//! `cargo tree -e features -i aws-lc-rs`. Only chains of feature nodes are followed: a package node
//! under feature `Y/f` is a declaration that turns `f` on; going through package nodes would blame
//! every crate that merely uses rustls.

use std::{
	collections::{BTreeSet, HashMap},
	fs,
	path::{Path, PathBuf},
	process::Command,
};

use super::Violation;

const RULE: &str = "no-aws-lc";

pub fn check(workspace_root: &Path) -> Vec<Violation> {
	let lock_path = workspace_root.join("Cargo.lock");
	let Ok(lock) = fs::read_to_string(&lock_path) else {
		return vec![]; // no lockfile: nothing resolved yet
	};
	if !lock.contains("name = \"aws-lc-rs\"") {
		return vec![];
	}

	let output = Command::new("cargo")
		.args([
			"tree", "--locked", "--workspace", "--target", "all", "-e", "features", "-i", "aws-lc-rs", "--prefix", "depth", "--manifest-path",
		])
		.arg(workspace_root.join("Cargo.toml"))
		.output()
		.expect("cargo is on PATH wherever codestyle checks a Rust workspace");
	assert!(
		output.status.success(),
		"`cargo tree -i aws-lc-rs` failed in {}:\n{}",
		workspace_root.display(),
		String::from_utf8_lossy(&output.stderr)
	);
	let tree = String::from_utf8(output.stdout).expect("cargo tree prints utf-8");

	let declarations: BTreeSet<Enabler> = enablers(&tree).into_iter().collect();

	let mut seen: BTreeSet<(String, usize, String, String)> = BTreeSet::default();
	let mut violations = Vec::default();
	for e in declarations {
		let (file, line, whose) = match local_manifest(&e.package) {
			Some(manifest) => {
				let (file, line) = e.locate_declaration(&manifest, workspace_root);
				(file, line, format!("`{}`", package_name(&e.package)))
			}
			None => (lock_path.clone(), e.lock_line(&lock), format!("upstream crate `{}` (fix it there, then bump it here)", e.package)),
		};
		let file = file.display().to_string();
		if !seen.insert((file.clone(), line, e.krate.clone(), e.feature.clone())) {
			continue;
		}
		violations.push(Violation {
			rule: RULE,
			file,
			line,
			column: 1,
			message: format!("{whose} enables `{}/{}`, which builds aws-lc-sys (bundled C); {}", e.krate, e.feature, e.guidance()),
			fix: None,
		});
	}
	violations
}

/// `package`'s own declaration turns on `krate/feature`.
#[derive(Eq, Ord, PartialEq, PartialOrd)]
struct Enabler {
	package: String,
	krate: String,
	feature: String,
}
impl Enabler {
	/// Line of `krate`'s declaration; a `workspace = true` declaration resolves to the root's `[workspace.dependencies]`.
	fn locate_declaration(&self, manifest: &Path, workspace_root: &Path) -> (PathBuf, usize) {
		let content = fs::read_to_string(manifest).unwrap_or_else(|e| panic!("cargo tree just resolved {}: {e}", manifest.display()));
		let (line, text) = self
			.dep_line(&content, |section| section.contains("dependencies") && !section.starts_with("workspace."))
			.or_else(|| self.dep_line(&content, |section| section == "workspace.dependencies"))
			.unwrap_or_else(|| {
				panic!(
					"`{}` is a dependency of {} per cargo tree, but no declaration line names it (renamed with `package = `?)",
					self.krate,
					manifest.display()
				)
			});
		let inherits = text.contains("workspace = true") || text.contains(".workspace");
		let root_manifest = workspace_root.join("Cargo.toml");
		if !inherits || manifest == root_manifest {
			return (manifest.to_path_buf(), line);
		}
		let root = fs::read_to_string(&root_manifest).expect("member inherits from the root manifest");
		let (line, _) = self
			.dep_line(&root, |section| section == "workspace.dependencies")
			.unwrap_or_else(|| panic!("`{}.workspace = true` without a root declaration", self.krate));
		(root_manifest, line)
	}

	fn dep_line(&self, content: &str, in_section: impl Fn(&str) -> bool) -> Option<(usize, String)> {
		let mut section = String::default();
		for (i, line) in content.lines().enumerate() {
			let trimmed = line.trim();
			if let Some(header) = trimmed.strip_prefix('[').and_then(|h| h.strip_suffix(']')) {
				section = header.to_string();
				if header.rsplit_once('.').is_some_and(|(prefix, name)| name == self.krate && in_section(prefix)) {
					let body = content.lines().skip(i + 1).take_while(|l| !l.trim_start().starts_with('[')).collect::<Vec<_>>().join("\n");
					return Some((i + 1, body));
				}
				continue;
			}
			let Some(rest) = trimmed.strip_prefix(self.krate.as_str()) else { continue };
			if in_section(&section) && rest.starts_with([' ', '=', '.']) {
				return Some((i + 1, trimmed.to_string()));
			}
		}
		None
	}

	fn lock_line(&self, lock: &str) -> usize {
		let mut parts = self.package.split(' ');
		let name = parts.next().expect("split yields at least one item");
		let version = parts.next().and_then(|v| v.strip_prefix('v')).expect("cargo tree prints `name vX`");
		let lines: Vec<&str> = lock.lines().collect();
		lines
			.windows(2)
			.position(|w| w[0] == format!("name = \"{name}\"") && w[1] == format!("version = \"{version}\""))
			.map(|i| i + 1)
			.unwrap_or_else(|| panic!("cargo tree --locked printed `{}`, so Cargo.lock lists it", self.package))
	}

	/// Fixes verified against the crates' feature tables (reqwest 0.13, sentry 0.48, tonic 0.14, sqlx 0.9, rustls 0.23).
	fn guidance(&self) -> String {
		let (krate, feature) = (self.krate.as_str(), self.feature.as_str());
		const INSTALL: &str = "add `rustls = { version = \"0.23\", default-features = false, features = [\"ring\", \"std\", \"tls12\"] }` and call `rustls::crypto::ring::default_provider().install_default()` at the start of every process (bins, examples, test harnesses) that builds a client: it has no ring feature, and without an installed provider it panics on client build";
		match (krate, feature) {
			("tonic", "tls-aws-lc") => "use `tls-ring` instead".into(),
			("sqlx", "tls-rustls-aws-lc-rs") => "use `tls-rustls-ring` instead".into(),
			("reqwest", "rustls" | "default-tls" | "default") => format!("set `default-features = false` and use `rustls-no-provider` instead of `rustls`; {INSTALL}"),
			("sentry", "rustls" | "default") => format!("set `default-features = false` and use `rustls-no-provider` instead of `rustls`; {INSTALL}"),
			("rustls" | "tokio-rustls" | "hyper-rustls" | "rustls-webpki", _) =>
				"set `default-features = false`, keep the defaults you need, and add `ring` (`rustls-webpki`: `ring`); keep exactly one provider enabled, or `ClientConfig::builder()` panics on ambiguity".into(),
			_ => format!("switch `{krate}` to its ring variant (see `cargo tree -e features -i aws-lc-rs`)"),
		}
	}
}

fn enablers(tree: &str) -> Vec<Enabler> {
	let mut children: HashMap<String, Vec<String>> = HashMap::default();
	let mut stack: Vec<String> = Vec::default();
	let mut roots: Vec<String> = Vec::default();
	for raw in tree.lines() {
		let digits = raw.bytes().take_while(u8::is_ascii_digit).count();
		if digits == 0 {
			continue; // blank separator lines between roots
		}
		let depth: usize = raw[..digits].parse().expect("prefix is digits");
		let node = raw[digits..].trim_end_matches(" (*)").trim_end_matches(" (command-line)").to_string();
		stack.truncate(depth);
		if let Some(parent) = stack.last() {
			children.entry(parent.clone()).or_default().push(node.clone());
		}
		if depth == 1 && feature_of(&node).is_some() {
			roots.push(node.clone());
		}
		stack.push(node);
	}

	let mut out = Vec::default();
	let mut visited: BTreeSet<String> = BTreeSet::default();
	let mut queue = roots;
	//LOOP: worklist over a finite graph, each node visited once
	while let Some(node) = queue.pop() {
		if !visited.insert(node.clone()) {
			continue;
		}
		let (krate, feature) = feature_of(&node).expect("only feature nodes are queued");
		for child in children.get(&node).into_iter().flatten() {
			if feature_of(child).is_some() {
				queue.push(child.clone());
			} else {
				out.push(Enabler {
					package: child.clone(),
					krate: krate.to_string(),
					feature: feature.to_string(),
				});
			}
		}
	}
	out
}

fn feature_of(node: &str) -> Option<(&str, &str)> {
	let (krate, rest) = node.split_once(" feature \"")?;
	Some((krate, rest.trim_end_matches('"')))
}

fn package_name(package: &str) -> &str {
	package.split(' ').next().expect("split yields at least one item")
}

/// `name vX (/abs/path)` -> the package's manifest, for packages we can edit here.
fn local_manifest(package: &str) -> Option<PathBuf> {
	let source = package.split_once(" (")?.1.trim_end_matches(')');
	source.starts_with('/').then(|| Path::new(source).join("Cargo.toml"))
}
