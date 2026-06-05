# codestyle
![Minimum Supported Rust Version](https://img.shields.io/badge/nightly-1.93+-ab6000.svg)
[<img alt="crates.io" src="https://img.shields.io/crates/v/codestyle.svg?color=fc8d62&logo=rust" height="20" style=flat-square>](https://crates.io/crates/codestyle)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs&style=flat-square" height="20">](https://docs.rs/codestyle)
![Lines Of Code](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/codestyle-loc.json)
<br>
[<img alt="ci errors" src="https://img.shields.io/github/actions/workflow/status/valeratrades/codestyle/errors.yml?branch=master&style=for-the-badge&style=flat-square&label=errors&labelColor=420d09" height="20">](https://github.com/valeratrades/codestyle/actions?query=branch%3Amaster) <!--NB: Won't find it if repo is private-->
[<img alt="ci warnings" src="https://img.shields.io/github/actions/workflow/status/valeratrades/codestyle/warnings.yml?branch=master&style=for-the-badge&style=flat-square&label=warnings&labelColor=d16002" height="20">](https://github.com/valeratrades/codestyle/actions?query=branch%3Amaster) <!--NB: Won't find it if repo is private-->

An opinionated style checker and auto-formatter for Rust, layered on top of `rustfmt` and `clippy`.

It enforces conventions those tools don't cover (loop comments, impl block placement, format-string variable embedding, banned crates, manual `IGNORED_ERROR` annotations instead of silently masking with `unwrap_or`, etc.) and can either assert violations (CI) or auto-fix them in place (`format`).
<!-- markdownlint-disable -->
<details>
<summary>
<h2>Installation</h2>
</summary>

#### Prebuilt binary (linux-x86_64)

Grab the latest release from [GitHub releases](https://github.com/valeratrades/codestyle/releases), or via [`cargo-binstall`](https://github.com/cargo-bins/cargo-binstall):

```sh
cargo binstall codestyle
```

#### Cargo

```sh
cargo install codestyle
```

#### From source

```sh
git clone https://github.com/valeratrades/codestyle
cd codestyle
cargo install --path .
```

</details>
<!-- markdownlint-restore -->

## Usage
#### Basic usage

```sh
# Check for violations (exit 1 on failure)
codestyle rust assert .

# Auto-fix violations in place
codestyle rust format .

# Collect occurrences into a per-rule markdown worktable for manual review
codestyle rust --only ignored-error audit .
```

#### Toggling checks

Each check has a default and can be flipped with `--<check>=true|false`. Pass flags before the subcommand:

```sh
# Enable instrument check (off by default)
codestyle rust --instrument=true assert .

# Disable specific checks
codestyle rust --loops=false --embed-simple-vars=false assert .
```

#### Excluding paths

Use `--exclude` (repeatable, before the subcommand) to skip vendored or third-party trees:

```sh
codestyle --exclude libs/nautilus_trader --exclude vendor rust assert .
```

#### Available flags

| Flag | Default | Description |
|------|---------|-------------|
| `--cargo-dep-ordering` | true | Order and group dependencies in `Cargo.toml` |
| `--instrument` | false | Require `#[instrument]` on async functions |
| `--loops` | true | Endless loops must carry a `//LOOP` comment |
| `--join-split-impls` | true | Join split `impl` blocks for the same type |
| `--impl-folds` | false | Wrap `impl` blocks in vim 1-fold markers |
| `--impl-follows-type` | true | `impl` blocks follow their type definition |
| `--embed-simple-vars` | true | Embed simple vars directly in format strings |
| `--insta-inline-snapshot` | true | `insta` macros use inline `@""` syntax |
| `--no-chrono` | true | Forbid `chrono` (use `jiff` instead) |
| `--no-tokio-spawn` | true | Forbid `tokio::spawn` (use structured concurrency) |
| `--use-bail` | true | Replace `return Err(eyre!(...))` with `bail!(...)` |
| `--test-fn-prefix` | false | Test fns must not start with `test_` |
| `--pub-first` | true | `pub` items come before private items |
| `--ignored-error` | true | `unwrap_or*` and `let _ =` need `//IGNORED_ERROR` |
| `--workspace-dep-hoisting` | true | Hoist shared deps to `[workspace.dependencies]` |
| `--unconventional-new` | true | `fn new` returning `Result` -> rename to `try_new` |
| `--prefer-default-over-bare-new` | false | Argument-less `pub fn new()` -> `Default` |
| `--inline-default` | true | Inline `impl Default` bodies as field defaults (RFC 3681) |
| `--prefer-ahash` | false | Replace `HashMap` with `ahash::AHashMap` |
| `--too-explicit` | true | Rewrite inline fully-qualified `std::` paths and add imports |

#### Format mode

`format` will:
1. Auto-fix violations where possible.
2. Delete `.snap` / `.pending-snap` files (when the `insta` check is enabled).
3. Report violations that still need manual fixing.

```sh
codestyle rust format .
# codestyle: fixed 3 violation(s)
# codestyle: 1 violation(s) need manual fixing:
#   [loops] src/main.rs:42:5: Endless loop without //LOOP comment
```

#### Audit mode

Some rules flag patterns that are genuinely hard to auto-fix and need human judgement case-by-case
(e.g. `ignored-error` flags every `unwrap_or*` / `let _ = …` — each must be individually decided
"keep & justify vs switch to Error/Panic"). `audit` scaffolds that review: it collects every
occurrence of each audit-capable rule into `<target_dir>/tmp/audit/<rule>.md` (override with
`--audit-dir`) as a `- [ ]` checklist with a `TODO: reason` line per item, under a header that
spells out the default decision. It's a collection step, not a gate — it always exits 0 on success.

Only a subset of rules know how to audit (currently just `ignored-error`), and audit is normally
run with `--only`:

```sh
codestyle rust --only ignored-error audit .
# codestyle: wrote 12 occurrence(s) to ./docs/.readme_assets/tmp/audit/ignored-error.md
```



<br>

<sup>
	This repository follows <a href="https://github.com/valeratrades/.github/tree/master/best_practices">my best practices</a> and <a href="https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/TIGER_STYLE.md">Tiger Style</a> (except "proper capitalization for acronyms": (VsrState, not VSRState) and formatting). For project's architecture, see <a href="./docs/ARCHITECTURE.md">ARCHITECTURE.md</a>.
</sup>

#### License

<sup>
	Licensed under <a href="LICENSE">Blue Oak 1.0.0</a>
</sup>

<br>

<sub>
	Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be licensed as above, without any additional terms or conditions.
</sub>

