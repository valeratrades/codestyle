#!/home/v/nix/home/scripts/nix-run-cached
---cargo

[package]
edition = "2024"
---

//! Picks what `cl review` should run, biased towards whatever is least recently run in the given
//! project. Emits the ready-to-use prompt on stdout and a human label on stderr.
//!
//! State lives at `$XDG_CACHE_HOME/codestyle/llm_review/<project>`: first line is the next run index,
//! the rest are `<entry key> <index of its last run>`. An entry's weight is `(age + 1)^importance`
//! where `age = next - last`, so an unseen entry wins by default and a just-run one stays reachable.
//! The counter is renormalised on every write, keeping it bounded by the entry count.

use std::{collections::HashMap, fmt, fs, io::Read as _, path::PathBuf};

const SKILLS_DIR: &str = "/home/v/s/codestyle/skills";

/// How steeply an entry's odds grow while it goes unpicked. Raising it spaces that entry's own runs
/// more evenly, and wins it a larger share of runs against lower-set entries.
struct Importance(f64);

impl Importance {
	const MAX: f64 = 3.0;
	const MIN: f64 = 1.0;

	fn new(v: f64) -> Self {
		assert!((Self::MIN..=Self::MAX).contains(&v), "importance must be within {}..={}, got {v}", Self::MIN, Self::MAX);
		Self(v)
	}

	/// Quantised so sampling stays integer arithmetic, and so no entry's weight can round to zero.
	fn weight(&self, age: u32) -> u64 {
		(f64::from(age + 1).powf(self.0) * 1024.) as u64
	}
}

impl Default for Importance {
	fn default() -> Self {
		Self(Self::MIN)
	}
}

enum Entry {
	Skill(String),
	Prompt(String),
}

impl Entry {
	/// Stable across reordering the registry; editing a prompt deliberately resets its history.
	fn key(&self) -> String {
		match self {
			Self::Skill(name) => name.clone(),
			Self::Prompt(text) => format!("p:{:016x}", text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3))),
		}
	}
}

impl fmt::Display for Entry {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Self::Skill(name) => write!(
				f,
				"Review this repo following {SKILLS_DIR}/{name}/SKILL.md. Read it first, then carry out exactly what it asks for."
			),
			Self::Prompt(text) => write!(f, "{text}"),
		}
	}
}

fn main() {
	let project = match std::env::args().nth(1) {
		Some(p) => PathBuf::from(p),
		None => std::env::current_dir().expect("cwd exists"),
	};
	let key: String = project.display().to_string().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();

	let entries: Vec<(Entry, Importance)> = registry()
		.lines()
		.map(|l| {
			let mut cols = l.split('\t');
			let (importance, kind, payload) = (cols.next().expect("importance column"), cols.next().expect("kind column"), cols.next().expect("payload column"));
			let entry = match kind {
				"skill" => {
					assert!(
						PathBuf::from(format!("{SKILLS_DIR}/{payload}/SKILL.md")).is_file(),
						"registry names skill `{payload}`, but {SKILLS_DIR}/{payload}/SKILL.md does not exist"
					);
					Entry::Skill(payload.to_owned())
				}
				_ => Entry::Prompt(payload.trim().to_owned()),
			};
			(entry, Importance::new(importance.parse().expect("importance is a number")))
		})
		.collect();
	assert!(!entries.is_empty(), "{SKILLS_DIR}/registry.nix lists nothing to draw from");

	// a skill left out of the registry would silently never run
	for skill in fs::read_dir(SKILLS_DIR)
		.expect("skills dir is hardcoded and in-repo")
		.map(|e| e.expect("readable dir entry").path())
		.filter(|p| p.join("SKILL.md").is_file())
	{
		let name = skill.file_name().expect("dir entry has a name").to_string_lossy().into_owned();
		assert!(
			entries.iter().any(|(e, _)| matches!(e, Entry::Skill(s) if *s == name)),
			"skill `{name}` is missing from the registry"
		);
	}

	let dir = cache_home().join("codestyle/llm_review");
	fs::create_dir_all(&dir).expect("cache dir is ours to create");
	let state_file = dir.join(key);

	let raw = fs::read_to_string(&state_file).unwrap_or_default();
	let mut lines = raw.lines();
	let mut next: u32 = lines.next().map_or(0, |l| l.parse().expect("first line is the run counter"));
	let mut last: HashMap<String, u32> = lines
		.map(|l| {
			let (name, idx) = l.rsplit_once(' ').expect("`<entry key> <run index>`");
			(name.to_owned(), idx.parse().expect("run index"))
		})
		.collect();

	let weights: Vec<u64> = entries.iter().map(|(e, i)| i.weight(next - last.get(&e.key()).copied().unwrap_or(0))).collect();
	let mut r = urandom_u64() % weights.iter().sum::<u64>();
	let picked = weights
		.iter()
		.position(|w| {
			if r < *w {
				return true;
			}
			r -= w;
			false
		})
		.expect("remainder is below the total");

	last.insert(entries[picked].0.key(), next);
	next += 1;
	let shift = entries.iter().map(|(e, _)| last.get(&e.key()).copied().unwrap_or(0)).min().expect("entries is non-empty");
	let mut out = format!("{}\n", next - shift);
	for (e, _) in &entries {
		out += &format!("{} {}\n", e.key(), last.get(&e.key()).copied().unwrap_or(0) - shift);
	}
	fs::write(&state_file, out).expect("cache file is ours to write");

	match &entries[picked].0 {
		Entry::Skill(name) => eprintln!("cl review → {name}"),
		Entry::Prompt(text) => eprintln!("cl review → {text}"),
	}
	println!("{}", entries[picked].0);
}

/// Flattens registry.nix to `<importance>\t(skill|prompt)\t<payload>` per entry, applying the
/// importance default and folding the indented-string newlines away so each entry stays one line.
fn registry() -> String {
	let apply = r#"es: builtins.concatStringsSep "\n" (map (e:
		builtins.replaceStrings ["\n"] [" "]
			"${toString (e.importance or 1.0)}\t${if e ? skill then "skill" else "prompt"}\t${e.skill or e.prompt}") es)"#;
	let out = std::process::Command::new("nix")
		.args(["eval", "--raw", "--file", &format!("{SKILLS_DIR}/registry.nix"), "--apply", apply])
		.output()
		.expect("nix is on PATH");
	assert!(out.status.success(), "evaluating registry.nix failed:\n{}", String::from_utf8_lossy(&out.stderr));
	String::from_utf8(out.stdout).expect("nix emits utf8")
}

fn cache_home() -> PathBuf {
	match std::env::var_os("XDG_CACHE_HOME") {
		Some(v) => PathBuf::from(v),
		None => PathBuf::from(std::env::var_os("HOME").expect("HOME is set")).join(".cache"),
	}
}

fn urandom_u64() -> u64 {
	let mut buf = [0u8; 8];
	fs::File::open("/dev/urandom").expect("linux").read_exact(&mut buf).expect("urandom never short-reads");
	u64::from_le_bytes(buf)
}
