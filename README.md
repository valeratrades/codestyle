# codestyle
![Minimum Supported Rust Version](https://img.shields.io/badge/nightly-1.93+-ab6000.svg)
[<img alt="crates.io" src="https://img.shields.io/crates/v/codestyle.svg?color=fc8d62&logo=rust" height="20" style=flat-square>](https://crates.io/crates/codestyle)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs&style=flat-square" height="20">](https://docs.rs/codestyle)
![Lines Of Code](https://img.shields.io/endpoint?url=https://gist.githubusercontent.com/valeratrades/b48e6f02c61942200e7d1e3eeabf9bcb/raw/codestyle-loc.json)
<br>
[<img alt="ci errors" src="https://img.shields.io/github/actions/workflow/status/valeratrades/codestyle/errors.yml?branch=master&style=for-the-badge&style=flat-square&label=errors&labelColor=420d09" height="20">](https://github.com/valeratrades/codestyle/actions?query=branch%3Amaster) <!--NB: Won't find it if repo is private-->
[<img alt="ci warnings" src="https://img.shields.io/github/actions/workflow/status/valeratrades/codestyle/warnings.yml?branch=master&style=for-the-badge&style=flat-square&label=warnings&labelColor=d16002" height="20">](https://github.com/valeratrades/codestyle/actions?query=branch%3Amaster) <!--NB: Won't find it if repo is private-->

A code style checker and formatter for Rust that enforces opinionated conventions beyond what rustfmt and clippy provide.
<!-- markdownlint-disable -->
<details>
<summary>
<h3>Installation</h3>
</summary>

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
# Check for violations
codestyle rust assert ./my-project

# Auto-fix violations
codestyle rust format ./my-project
```

#### Check options

Each check can be enabled or disabled with `--<check>=true|false`:

```sh
# Enable instrument check (off by default)
codestyle rust --instrument=true assert ./my-project

# Disable specific checks
codestyle rust --loops=false --embed-simple-vars=false assert ./my-project
```

#### Available flags

| Flag | Default | Description |
|------|---------|-------------|
| `--instrument` | false | Check async functions for `#[instrument]` |
| `--loops` | true | Check endless loops for `//LOOP` comments |
| `--impl-follows-type` | true | Check impl blocks follow type definitions |
| `--embed-simple-vars` | true | Check format strings embed simple variables |
| `--insta-inline-snapshot` | true | Check insta macros use inline snapshots |

#### Format mode

Format mode will:
1. Automatically fix violations where possible
2. Delete any `.pending-snap` files (when insta check enabled)
3. Report violations that require manual fixing

```sh
codestyle rust format ./my-project
# codestyle: fixed 3 violation(s)
# codestyle: 1 violation(s) need manual fixing:
#   [loops] src/main.rs:42:5: Endless loop without //LOOP comment
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

