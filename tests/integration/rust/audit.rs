//! Integration tests for `audit` mode: collecting audit-capable rule occurrences into a per-rule
//! markdown worktable. v1 audits only `ignored_error`.

use codestyle::rust_checks;
use v_fixtures::Fixture;

use crate::utils::opts_for;

/// Run `run_audit` over a fixture and return the generated `ignored-error.md`, with the absolute
/// temp-dir prefix stripped so the snapshot is stable across runs.
#[track_caller]
fn audit_ignored_error(fixture_str: &str) -> String {
	let fixture = Fixture::parse(fixture_str);
	let temp = fixture.write_to_tempdir();
	let opts = opts_for("ignored_error");

	let exit = rust_checks::run_audit(&temp.root, &opts, &[], None);
	assert_eq!(exit, 0, "run_audit should return 0 on success");

	let md = temp.read("tmp/audit/ignored-error.md");
	md.replace(temp.root.to_str().unwrap(), "")
}

#[test]
fn audit_collects_unwrap_or_and_let_underscore() {
	insta::assert_snapshot!(audit_ignored_error(
		r#"
		fn bad() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0);
			let _ = some_result();
		}
		fn some_result() -> Result<(), ()> { Ok(()) }
		"#,
	), @"
	# `ignored-error` audit

	Goal: every flagged `unwrap_or(_else/_default)` and `let _ = …` is either **KEEP** (one-line why)
	or switched to **PANIC** / **ERROR** instead. No silent defaulting / discarding of state.

	Verdict legend: `TODO` | `KEEP: <why>` | `PANIC` | `ERROR: <how>` | `REMOVE: <why dead>` | `DONE`

	**Default decision is Error/Panic.** KEEP is a special case that must be very well justified —
	if unsure, error/panic. Dead code is `REMOVE`, not kept.

	---

	- [ ] `/main.rs:3:11` — `let y = x.unwrap_or(0);`
	  TODO: decision (if decision is KEEP, - justify)
	- [ ] `/main.rs:4:5` — `let _ = some_result();`
	  TODO: decision (if decision is KEEP, - justify)
	");
}

/// An occurrence with an `//IGNORED_ERROR` justification is already resolved and must not appear in
/// the worktable — `audit` reuses `check()`, which already excludes commented occurrences.
#[test]
fn audit_excludes_already_justified_occurrences() {
	let fixture = Fixture::parse(
		r#"
		fn good() {
			let x: Option<i32> = None;
			let y = x.unwrap_or(0); //IGNORED_ERROR: default for missing config
		}
		"#,
	);
	let temp = fixture.write_to_tempdir();
	let opts = opts_for("ignored_error");

	let exit = rust_checks::run_audit(&temp.root, &opts, &[], None);
	assert_eq!(exit, 0);

	// No occurrences → no rule group → no file written.
	assert!(temp.try_read("tmp/audit/ignored-error.md").is_none(), "justified occurrence must not produce an audit file");
}
