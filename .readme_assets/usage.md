### Basic usage

```sh
# Check for violations
codestyle rust assert ./my-project

# Auto-fix violations
codestyle rust format ./my-project
```

### Check options

Each check can be enabled or disabled with `--<check>=true|false`:

```sh
# Enable instrument check (off by default)
codestyle rust --instrument=true assert ./my-project

# Disable specific checks
codestyle rust --loops=false --embed-simple-vars=false assert ./my-project
```

### Available flags

| Flag | Default | Description |
|------|---------|-------------|
| `--instrument` | false | Check async functions for `#[instrument]` |
| `--loops` | true | Check endless loops for `//LOOP` comments |
| `--impl-follows-type` | true | Check impl blocks follow type definitions |
| `--embed-simple-vars` | true | Check format strings embed simple variables |
| `--insta-inline-snapshot` | true | Check insta macros use inline snapshots |

### Format mode

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
