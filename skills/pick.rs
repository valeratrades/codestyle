#!/usr/bin/env rust-script
//! Picks a skill for `cl review`, biased towards the ones least recently run in the given project.
//!
//! State lives at `$XDG_CACHE_HOME/codestyle/llm_review/<project>`: first line is the next run index,
//! the rest are `<skill> <index of its last run>`. A skill's weight is `(age + 1)^utility` where
//! `age = next - last`, so an unseen skill wins by default and a just-run one stays reachable.
//! The counter is renormalised on every write, keeping it bounded by the skill count.

use std::{collections::HashMap, fs, io::Read as _, path::PathBuf};

const SKILLS_DIR: &str = "/home/v/s/codestyle/skills";

/// How steeply a skill's odds grow while it goes unpicked. Raising it spaces that skill's own runs
/// more evenly, and wins it a larger share of runs against lower-set skills.
struct Utility(f64);

impl Utility {
	const MAX: f64 = 3.0;
	const MIN: f64 = 1.5;

	fn new(v: f64) -> Self {
		assert!((Self::MIN..=Self::MAX).contains(&v), "utility must be within {}..={}, got {v}", Self::MIN, Self::MAX);
		Self(v)
	}

	/// Quantised so sampling stays integer arithmetic, and so no skill's weight can round to zero.
	fn weight(&self, age: u32) -> u64 {
		(f64::from(age + 1).powf(self.0) * 1024.) as u64
	}
}

impl Default for Utility {
	fn default() -> Self {
		Self(Self::MIN)
	}
}

fn main() {
	let project = match std::env::args().nth(1) {
		Some(p) => PathBuf::from(p),
		None => std::env::current_dir().expect("cwd exists"),
	};
	let key: String = project.display().to_string().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();

	let mut skills: Vec<String> = fs::read_dir(SKILLS_DIR)
		.expect("skills dir is hardcoded and in-repo")
		.map(|e| e.expect("readable dir entry").path())
		.filter(|p| p.join("SKILL.md").is_file())
		.map(|p| p.file_name().expect("dir entry has a name").to_string_lossy().into_owned())
		.collect();
	assert!(!skills.is_empty(), "no <dir>/SKILL.md found under {SKILLS_DIR}");
	skills.sort();

	let utilities: HashMap<String, Utility> = fs::read_to_string(format!("{SKILLS_DIR}/utilities.txt"))
		.expect("utility table ships next to the skills")
		.lines()
		.map(str::trim)
		.filter(|l| !l.is_empty() && !l.starts_with('#'))
		.map(|l| {
			let (name, v) = l.split_once(char::is_whitespace).expect("`<skill> <utility>`");
			assert!(skills.contains(&name.to_owned()), "utility table names unknown skill `{name}`");
			(name.to_owned(), Utility::new(v.trim().parse().expect("utility is a float")))
		})
		.collect();

	let dir = cache_home().join("codestyle/llm_review");
	fs::create_dir_all(&dir).expect("cache dir is ours to create");
	let state_file = dir.join(key);

	let raw = fs::read_to_string(&state_file).unwrap_or_default();
	let mut lines = raw.lines();
	let mut next: u32 = lines.next().map_or(0, |l| l.parse().expect("first line is the run counter"));
	let mut last: HashMap<String, u32> = lines
		.map(|l| {
			let (name, idx) = l.rsplit_once(' ').expect("`<skill> <run index>`");
			(name.to_owned(), idx.parse().expect("run index"))
		})
		.collect();

	let weights: Vec<u64> = skills
		.iter()
		.map(|s| utilities.get(s).unwrap_or(&Utility::default()).weight(next - last.get(s).copied().unwrap_or(0)))
		.collect();
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

	last.insert(skills[picked].clone(), next);
	next += 1;
	let shift = skills.iter().map(|s| last.get(s).copied().unwrap_or(0)).min().expect("skills is non-empty");
	let mut out = format!("{}\n", next - shift);
	for s in &skills {
		out += &format!("{s} {}\n", last.get(s).copied().unwrap_or(0) - shift);
	}
	fs::write(&state_file, out).expect("cache file is ours to write");

	println!("{SKILLS_DIR}/{}/SKILL.md", skills[picked]);
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
