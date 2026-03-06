use std::path::Path;

use codestyle::rust_checks::cargo_dep_ordering;

fn check(content: &str) -> Vec<codestyle::rust_checks::Violation> {
	cargo_dep_ordering::check(Path::new("Cargo.toml"), content)
}

fn format(content: &str) -> String {
	let violations = check(content);
	let mut result = content.to_string();
	// Apply fixes in reverse order to preserve byte offsets
	let mut fixes: Vec<_> = violations.into_iter().filter_map(|v| v.fix).collect();
	fixes.sort_by(|a, b| b.start_byte.cmp(&a.start_byte));
	for fix in fixes {
		result.replace_range(fix.start_byte..fix.end_byte, &fix.replacement);
	}
	result
}

// === Passing cases ===

#[test]
fn already_ordered_passes() {
	let content = r#"[package]
name = "foo"

[dependencies]
my-lib = { path = "../my-lib" }

derive-new = "0.7"
tokio = { version = "^1", features = ["full"] }

serde.workspace = true
tracing.workspace = true
"#;
	assert!(check(content).is_empty());
}

#[test]
fn only_regular_deps_passes() {
	let content = r#"[dependencies]
derive-new = "0.7"
tokio = { version = "^1", features = ["full"] }
"#;
	assert!(check(content).is_empty());
}

#[test]
fn only_workspace_deps_passes() {
	let content = r#"[dependencies]
serde.workspace = true
tracing.workspace = true
"#;
	assert!(check(content).is_empty());
}

#[test]
fn only_path_deps_passes() {
	let content = r#"[dependencies]
my-lib = { path = "../my-lib" }
other-lib = { path = "../other-lib" }
"#;
	assert!(check(content).is_empty());
}

#[test]
fn empty_dependencies_passes() {
	let content = r#"[package]
name = "foo"

[dependencies]
"#;
	assert!(check(content).is_empty());
}

#[test]
fn no_dependencies_section_passes() {
	let content = r#"[package]
name = "foo"
version = "0.1.0"
"#;
	assert!(check(content).is_empty());
}

#[test]
fn patch_crates_io_not_checked() {
	// path deps in [patch.crates-io] should be ignored
	let content = r#"[dependencies]
tokio = "1"

[patch.crates-io]
some-crate = { path = "../some-crate" }
"#;
	assert!(check(content).is_empty());
}

// === Violation + fix cases ===

#[test]
fn workspace_before_regular_needs_reorder() {
	let input = r#"[dependencies]
serde.workspace = true
tokio = "1"
"#;
	let expected = r#"[dependencies]
tokio = "1"

serde.workspace = true
"#;
	let violations = check(input);
	assert_eq!(violations.len(), 1);
	assert_eq!(violations[0].rule, "cargo-dep-ordering");
	assert_eq!(format(input), expected);
}

#[test]
fn regular_before_path_needs_reorder() {
	let input = r#"[dependencies]
tokio = "1"
my-lib = { path = "../my-lib" }
"#;
	let expected = r#"[dependencies]
my-lib = { path = "../my-lib" }

tokio = "1"
"#;
	let violations = check(input);
	assert_eq!(violations.len(), 1);
	assert_eq!(format(input), expected);
}

#[test]
fn all_three_groups_misordered() {
	let input = r#"[dependencies]
serde.workspace = true
tokio = "1"
my-lib = { path = "../my-lib" }
"#;
	let expected = r#"[dependencies]
my-lib = { path = "../my-lib" }

tokio = "1"

serde.workspace = true
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn normalizes_workspace_brace_syntax() {
	let input = r#"[dependencies]
serde = { workspace = true }
tracing = { workspace = true }
"#;
	let expected = r#"[dependencies]
serde.workspace = true
tracing.workspace = true
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn mixed_workspace_syntax_normalized() {
	let input = r#"[dependencies]
serde = { workspace = true }
tracing.workspace = true
"#;
	let expected = r#"[dependencies]
serde.workspace = true
tracing.workspace = true
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn missing_blank_line_between_groups() {
	let input = r#"[dependencies]
my-lib = { path = "../my-lib" }
tokio = "1"
serde.workspace = true
"#;
	let expected = r#"[dependencies]
my-lib = { path = "../my-lib" }

tokio = "1"

serde.workspace = true
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn sorts_within_groups() {
	let input = r#"[dependencies]
tokio = "1"
derive-new = "0.7"
"#;
	let expected = r#"[dependencies]
derive-new = "0.7"
tokio = "1"
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn dev_dependencies_also_checked() {
	let input = r#"[dev-dependencies]
serde.workspace = true
insta = "1"
"#;
	let expected = r#"[dev-dependencies]
insta = "1"

serde.workspace = true
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn build_dependencies_also_checked() {
	let input = r#"[build-dependencies]
serde.workspace = true
cc = "1"
"#;
	let expected = r#"[build-dependencies]
cc = "1"

serde.workspace = true
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn multiple_sections_each_fixed() {
	let input = r#"[dependencies]
serde.workspace = true
tokio = "1"

[dev-dependencies]
tracing.workspace = true
insta = "1"
"#;
	let violations = check(input);
	assert_eq!(violations.len(), 2);
}

#[test]
fn path_dep_with_features_classified_correctly() {
	let input = r#"[dependencies]
tokio = "1"
my-lib = { path = "../my-lib", default-features = false }
"#;
	let expected = r#"[dependencies]
my-lib = { path = "../my-lib", default-features = false }

tokio = "1"
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn trailing_comment_on_dep_preserved() {
	let input = r#"[dependencies]
tokio = "1"
my-lib = { path = "../my-lib" } #dbg
"#;
	let expected = r#"[dependencies]
my-lib = { path = "../my-lib" } #dbg

tokio = "1"
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn complex_realistic_example() {
	let input = r#"[package]
name = "my-app"
version = "0.1.0"

[dependencies]
tokio = { version = "^1.49.0", features = ["full", "signal"] }
serde.workspace = true
my-macros = { path = "../my-macros" }
color-eyre.workspace = true
derive-new = "0.7"
tracing.workspace = true
other-lib = { path = "../other-lib", default-features = false } #dbg

[dev-dependencies]
insta = "^1"
"#;
	let expected = r#"[package]
name = "my-app"
version = "0.1.0"

[dependencies]
my-macros = { path = "../my-macros" }
other-lib = { path = "../other-lib", default-features = false } #dbg

derive-new = "0.7"
tokio = { version = "^1.49.0", features = ["full", "signal"] }

color-eyre.workspace = true
serde.workspace = true
tracing.workspace = true

[dev-dependencies]
insta = "^1"
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn two_groups_path_and_regular() {
	let input = r#"[dependencies]
my-lib = { path = "../my-lib" }

tokio = "1"
"#;
	assert!(check(input).is_empty());
}

#[test]
fn two_groups_regular_and_workspace() {
	let input = r#"[dependencies]
tokio = "1"

serde.workspace = true
"#;
	assert!(check(input).is_empty());
}

#[test]
fn two_groups_path_and_workspace() {
	let input = r#"[dependencies]
my-lib = { path = "../my-lib" }

serde.workspace = true
"#;
	assert!(check(input).is_empty());
}

#[test]
fn double_bracket_section_not_mixed_in() {
	// [[test]] is a TOML array-of-tables, should not be treated as dep content
	let input = r#"[dev-dependencies]
insta = "^1"
trybuild = "^1"
v_fixtures = "^0.3.4"

[[test]]
name = "rust"
path = "tests/integration/rust/main.rs"
"#;
	assert!(check(input).is_empty());
}

#[test]
fn real_world_risk_crate() {
	// Exact reproduction of the discretionary_engine_risk Cargo.toml
	let input = r#"[package]
name = "de_risk"

[dependencies]
clap.workspace = true
color-eyre.workspace = true
de_core = { path = "../discretionary_engine_core" }
jiff.workspace = true
miette.workspace = true
smart-default.workspace = true
snapshot_fonts.workspace = true
strum.workspace = true
thiserror = "2"
tokio = { version = "^1.49.0", features = ["full", "signal"] }
tracing.workspace = true
tracing-subscriber = { version = "^0.3.22", features = ["fmt", "env-filter"] }
v_exchanges = { workspace = true, features = ["binance", "bybit", "kucoin", "mexc"] }
v_utils = { workspace = true, features = ["trades"] }

[dev-dependencies]
insta.workspace = true
"#;
	let expected = r#"[package]
name = "de_risk"

[dependencies]
de_core = { path = "../discretionary_engine_core" }

thiserror = "2"
tokio = { version = "^1.49.0", features = ["full", "signal"] }
tracing-subscriber = { version = "^0.3.22", features = ["fmt", "env-filter"] }

clap.workspace = true
color-eyre.workspace = true
jiff.workspace = true
miette.workspace = true
smart-default.workspace = true
snapshot_fonts.workspace = true
strum.workspace = true
tracing.workspace = true
v_exchanges = { workspace = true, features = ["binance", "bybit", "kucoin", "mexc"] }
v_utils = { workspace = true, features = ["trades"] }

[dev-dependencies]
insta.workspace = true
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn real_world_mixed_deps() {
	// Real-world case: path deps, workspace deps, and regular deps all mixed together
	// Some path deps appear after regular/workspace deps
	let input = r#"[dependencies]
de_core = { path = "../discretionary_engine_core" }
de_macros = { path = "../discretionary_engine_macros", version = "^0.1.1" }
de_routing = { path = "../discretionary_engine_routing" }

chrono = "^0.4.44"
clap.workspace = true
clap_complete = "4.5.66"
color-eyre.workspace = true
config = "0.15.19"
de_risk = { path = "../discretionary_engine_risk" }
de_strategy = { path = "../discretionary_engine_strategy" }
derive-new = "0.7.0"
derive_more.workspace = true
jiff.workspace = true
miette.workspace = true
nautilus-bybit = { path = "../libs/nautilus_trader/crates/adapters/bybit", default-features = false }
nautilus-model = { path = "../libs/nautilus_trader/crates/model" }
tokio = { version = "^1.50.0", features = ["full", "tracing"] }
tracing.workspace = true
v_exchanges.workspace = true
v_utils.workspace = true

[dev-dependencies]
lazy_static = "1.5.0"

insta.workspace = true
"#;
	let expected = r#"[dependencies]
de_core = { path = "../discretionary_engine_core" }
de_macros = { path = "../discretionary_engine_macros", version = "^0.1.1" }
de_risk = { path = "../discretionary_engine_risk" }
de_routing = { path = "../discretionary_engine_routing" }
de_strategy = { path = "../discretionary_engine_strategy" }
nautilus-bybit = { path = "../libs/nautilus_trader/crates/adapters/bybit", default-features = false }
nautilus-model = { path = "../libs/nautilus_trader/crates/model" }

chrono = "^0.4.44"
clap_complete = "4.5.66"
config = "0.15.19"
derive-new = "0.7.0"
tokio = { version = "^1.50.0", features = ["full", "tracing"] }

clap.workspace = true
color-eyre.workspace = true
derive_more.workspace = true
jiff.workspace = true
miette.workspace = true
tracing.workspace = true
v_exchanges.workspace = true
v_utils.workspace = true

[dev-dependencies]
lazy_static = "1.5.0"

insta.workspace = true
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn workspace_deps_with_features() {
	// workspace deps with extra features like `{ workspace = true, features = [...] }`
	// should be classified as workspace deps and kept in workspace group
	let input = r#"[dependencies]
de_core = { path = "../discretionary_engine_core" }
clap.workspace = true
color-eyre.workspace = true
jiff.workspace = true
miette.workspace = true
smart-default.workspace = true
snapshot_fonts.workspace = true
strum.workspace = true
thiserror = "2"
tokio = { version = "^1.49.0", features = ["full", "signal"] }
tracing.workspace = true
tracing-subscriber = { version = "^0.3.22", features = ["fmt", "env-filter"] }
v_exchanges = { workspace = true, features = ["binance", "bybit", "kucoin", "mexc"] }
v_utils = { workspace = true, features = ["trades"] }

[dev-dependencies]
insta.workspace = true
"#;
	let expected = r#"[dependencies]
de_core = { path = "../discretionary_engine_core" }

thiserror = "2"
tokio = { version = "^1.49.0", features = ["full", "signal"] }
tracing-subscriber = { version = "^0.3.22", features = ["fmt", "env-filter"] }

clap.workspace = true
color-eyre.workspace = true
jiff.workspace = true
miette.workspace = true
smart-default.workspace = true
snapshot_fonts.workspace = true
strum.workspace = true
tracing.workspace = true
v_exchanges = { workspace = true, features = ["binance", "bybit", "kucoin", "mexc"] }
v_utils = { workspace = true, features = ["trades"] }

[dev-dependencies]
insta.workspace = true
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn commented_out_patch_section_preserved() {
	// Commented-out [patch.crates-io] lines after deps must not be nuked
	let input = r#"[dev-dependencies]
insta = "1.46"
v_fixtures = { version = "^0.3.4" }
walkdir = "2"

#[patch.crates-io]
#v_utils = { path = "../v_utils/v_utils" }
#v_fixtures = { path = "../v_fixtures" }

[[test]]
name = "integration"
"#;
	assert!(check(input).is_empty());
}

#[test]
fn trailing_comments_preserved_in_reorder() {
	let input = r#"[dependencies]
serde.workspace = true
tokio = "1"

# this comment should survive
"#;
	let expected = r#"[dependencies]
tokio = "1"

serde.workspace = true

# this comment should survive
"#;
	assert_eq!(format(input), expected);
}

#[test]
fn section_followed_by_double_bracket() {
	let input = r#"[dev-dependencies]
v_fixtures = "^0.3.4"
insta = "^1"

[[test]]
name = "rust"
"#;
	let expected = r#"[dev-dependencies]
insta = "^1"
v_fixtures = "^0.3.4"

[[test]]
name = "rust"
"#;
	assert_eq!(format(input), expected);
}
