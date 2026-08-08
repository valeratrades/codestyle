#!/home/v/nix/home/scripts/nix-run-cached
---cargo

[package]
edition = "2024"

[dependencies]
crossterm = "0.28"
---

//! Picks what `cl review` should run, biased towards whatever is least recently run in the given
//! project. Emits the ready-to-use prompt on stdout and a human label on stderr. With `--fuzzy` the
//! choice is made by hand through a selector instead, but still counts as a run.
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
	Prompt { name: String, text: String },
}

impl Entry {
	/// Stable across reordering the registry; editing a prompt deliberately resets its history.
	fn key(&self) -> String {
		match self {
			Self::Skill(name) => name.clone(),
			Self::Prompt { text, .. } => format!("p:{:016x}", text.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3))),
		}
	}

	fn name(&self) -> &str {
		match self {
			Self::Skill(name) => name,
			Self::Prompt { name, .. } => name,
		}
	}

	/// What the entry actually asks the agent to do, as prose to search over.
	fn body(&self) -> String {
		match self {
			Self::Skill(name) => fs::read_to_string(format!("{SKILLS_DIR}/{name}/SKILL.md")).expect("skill existence is asserted on load"),
			Self::Prompt { text, .. } => text.clone(),
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
			Self::Prompt { text, .. } => write!(f, "{text}"),
		}
	}
}

fn main() {
	let mut fuzzy = false;
	let mut project = None;
	for arg in std::env::args().skip(1) {
		match arg.as_str() {
			"-f" | "--fuzzy" => fuzzy = true,
			_ => assert!(project.replace(PathBuf::from(&arg)).is_none(), "takes one project path, got a second: {arg}"),
		}
	}
	let project = project.unwrap_or_else(|| std::env::current_dir().expect("cwd exists"));
	let key: String = project.display().to_string().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '-' }).collect();

	let entries: Vec<(Entry, Importance)> = registry()
		.lines()
		.map(|l| {
			let mut cols = l.split('\t');
			let (importance, kind, name, payload) = (
				cols.next().expect("importance column"),
				cols.next().expect("kind column"),
				cols.next().expect("name column"),
				cols.next().expect("payload column"),
			);
			let entry = match kind {
				"skill" => {
					assert!(
						PathBuf::from(format!("{SKILLS_DIR}/{payload}/SKILL.md")).is_file(),
						"registry names skill `{payload}`, but {SKILLS_DIR}/{payload}/SKILL.md does not exist"
					);
					Entry::Skill(payload.to_owned())
				}
				_ => {
					assert!(!name.trim().is_empty(), "registry prompt has no `name`: {payload}");
					Entry::Prompt {
						name: name.trim().to_owned(),
						text: payload.trim().to_owned(),
					}
				}
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

	let picked = match fuzzy {
		true => {
			let candidates: Vec<select::Candidate> = entries
				.iter()
				.map(|(e, _)| select::Candidate {
					name: e.name().to_owned(),
					body: e.body(),
				})
				.collect();
			match select::run(&candidates) {
				Some(i) => i,
				None => std::process::exit(1), // aborted, so nothing ran and nothing is recorded
			}
		}
		false => {
			let weights: Vec<u64> = entries.iter().map(|(e, i)| i.weight(next - last.get(&e.key()).copied().unwrap_or(0))).collect();
			let mut r = urandom_u64() % weights.iter().sum::<u64>();
			weights
				.iter()
				.position(|w| {
					if r < *w {
						return true;
					}
					r -= w;
					false
				})
				.expect("remainder is below the total")
		}
	};

	last.insert(entries[picked].0.key(), next);
	next += 1;
	let shift = entries.iter().map(|(e, _)| last.get(&e.key()).copied().unwrap_or(0)).min().expect("entries is non-empty");
	let mut out = format!("{}\n", next - shift);
	for (e, _) in &entries {
		out += &format!("{} {}\n", e.key(), last.get(&e.key()).copied().unwrap_or(0) - shift);
	}
	fs::write(&state_file, out).expect("cache file is ours to write");

	eprintln!("cl review → {}", entries[picked].0.name());
	println!("{}", entries[picked].0);
}

/// Flattens registry.nix to `<importance>\t(skill|prompt)\t<name>\t<payload>` per entry, applying the
/// importance default and folding the indented-string newlines away so each entry stays one line.
fn registry() -> String {
	let apply = r#"es: builtins.concatStringsSep "\n" (map (e:
		builtins.replaceStrings ["\n"] [" "]
			"${toString (e.importance or 1.0)}\t${if e ? skill then "skill" else "prompt"}\t${e.name or (e.skill or "")}\t${e.skill or e.prompt}") es)"#;
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

/// Hand-picking a registry entry. Scoring follows the blog search in ~/s/site: a query term landing
/// on the name outweighs the same term in the body tenfold, yet body-only matches still reach every
/// entry. Terms are ANDed, so each one typed narrows the list.
///
/// stdout carries the prompt, so the UI goes to /dev/tty (crossterm reads its events from there too).
mod select {
	use std::{fs, io::Write as _};

	use crossterm::{
		cursor::{Hide, MoveTo, Show},
		event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
		execute, queue,
		style::{Attribute, Print, SetAttribute},
		terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
	};

	pub struct Candidate {
		pub name: String,
		pub body: String,
	}

	pub fn run(candidates: &[Candidate]) -> Option<usize> {
		let mut tty = fs::OpenOptions::new().read(true).write(true).open("/dev/tty").expect("`--fuzzy` needs a terminal");
		terminal::enable_raw_mode().expect("tty is ours for the duration");
		execute!(tty, EnterAlternateScreen, Hide).expect("tty accepts writes");
		let picked = interact(&mut tty, candidates);
		execute!(tty, Show, LeaveAlternateScreen).expect("tty accepts writes");
		terminal::disable_raw_mode().expect("raw mode was ours to leave");
		picked
	}

	fn interact(tty: &mut fs::File, candidates: &[Candidate]) -> Option<usize> {
		let mut query = String::new();
		let mut cursor = 0usize;
		loop {
			//LOOP: bounded by the user hitting Enter or Esc
			let shown = filter(candidates, &query);
			cursor = cursor.min(shown.len().saturating_sub(1));
			draw(tty, candidates, &query, &shown, cursor);

			let Event::Key(k) = event::read().expect("tty delivers events") else { continue };
			if k.kind != KeyEventKind::Press {
				continue;
			}
			match (k.code, k.modifiers) {
				(KeyCode::Esc, _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => return None,
				(KeyCode::Enter, _) => return shown.get(cursor).copied(),
				(KeyCode::Up, _) | (KeyCode::Char('p'), KeyModifiers::CONTROL) => cursor = cursor.saturating_sub(1),
				(KeyCode::Down, _) | (KeyCode::Char('n'), KeyModifiers::CONTROL) => cursor = (cursor + 1).min(shown.len().saturating_sub(1)),
				(KeyCode::Backspace, _) => {
					query.pop();
					cursor = 0;
				}
				(KeyCode::Char('u'), KeyModifiers::CONTROL) => {
					query.clear();
					cursor = 0;
				}
				(KeyCode::Char('w'), KeyModifiers::CONTROL) => {
					let cut = query.trim_end().rfind(char::is_whitespace).map_or(0, |i| i + 1);
					query.truncate(cut);
					cursor = 0;
				}
				(KeyCode::Char(c), m) if !m.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
					query.push(c);
					cursor = 0;
				}
				_ => {}
			}
		}
	}

	fn filter(candidates: &[Candidate], query: &str) -> Vec<usize> {
		if query.trim().is_empty() {
			return (0..candidates.len()).collect();
		}
		let mut scored: Vec<(usize, f64)> = candidates.iter().enumerate().map(|(i, c)| (i, score(c, query))).filter(|(_, s)| *s > 0.).collect();
		scored.sort_by(|a, b| b.1.partial_cmp(&a.1).expect("scores are finite"));
		scored.into_iter().map(|(i, _)| i).collect()
	}

	fn tokenize(s: &str) -> Vec<String> {
		s.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|t| !t.is_empty()).map(str::to_owned).collect()
	}

	fn score(candidate: &Candidate, query: &str) -> f64 {
		let query_tokens = tokenize(query);
		if query_tokens.is_empty() {
			return 0.;
		}
		let name_tokens = tokenize(&candidate.name);
		let body_tokens = tokenize(&candidate.body);
		let (name_lower, body_lower) = (candidate.name.to_lowercase(), candidate.body.to_lowercase());

		let mut total = 0.;
		for qt in &query_tokens {
			let mut term = 0.;
			for t in &name_tokens {
				term += match t {
					_ if t == qt => 100.,
					_ if t.starts_with(qt) => 50.,
					_ if t.contains(qt) => 20.,
					_ => 0.,
				};
			}
			for t in &body_tokens {
				term += match t {
					_ if t == qt => 10.,
					_ if t.starts_with(qt) => 5.,
					_ if t.contains(qt) => 2.,
					_ => 0.,
				};
			}
			term += if name_lower.contains(qt) { 10. } else { 0. };
			term += if body_lower.contains(qt) { 1. } else { 0. };
			if term == 0. {
				return 0.;
			}
			total += term;
		}
		total / query_tokens.len() as f64
	}

	fn draw(tty: &mut fs::File, candidates: &[Candidate], query: &str, shown: &[usize], cursor: usize) {
		let (cols, rows) = terminal::size().expect("tty reports its size");
		let (cols, rows) = (usize::from(cols), usize::from(rows));
		let list_rows = shown.len().min((rows.saturating_sub(2) / 2).max(1));
		let top = (cursor + 1).saturating_sub(list_rows);

		queue!(tty, Clear(ClearType::All), MoveTo(0, 0), Print(clip(&format!("› {query}"), cols))).expect("tty accepts writes");
		for (row, i) in shown[top..].iter().take(list_rows).enumerate() {
			let selected = top + row == cursor;
			let line = clip(&format!("{} {}", if selected { '❯' } else { ' ' }, candidates[*i].name), cols);
			let attr = if selected { Attribute::Reverse } else { Attribute::Dim };
			queue!(tty, MoveTo(0, (row + 1) as u16), SetAttribute(attr), Print(line), SetAttribute(Attribute::Reset)).expect("tty accepts writes");
		}

		if let Some(i) = shown.get(cursor) {
			let body_top = list_rows + 2;
			queue!(tty, SetAttribute(Attribute::Dim)).expect("tty accepts writes");
			for (row, line) in wrap(&candidates[*i].body, cols).into_iter().take(rows.saturating_sub(body_top)).enumerate() {
				queue!(tty, MoveTo(0, (body_top + row) as u16), Print(clip(&line, cols))).expect("tty accepts writes");
			}
			queue!(tty, SetAttribute(Attribute::Reset)).expect("tty accepts writes");
		}
		tty.flush().expect("tty accepts writes");
	}

	fn clip(s: &str, width: usize) -> String {
		s.chars().take(width).collect()
	}

	fn wrap(text: &str, width: usize) -> Vec<String> {
		let mut out = Vec::new();
		for paragraph in text.lines() {
			let mut line = String::new();
			for word in paragraph.split_whitespace() {
				if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
					out.push(std::mem::take(&mut line));
				}
				if !line.is_empty() {
					line.push(' ');
				}
				line.push_str(word);
			}
			out.push(line);
		}
		out
	}
}
