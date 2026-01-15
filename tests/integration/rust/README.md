# Rust Check Integration Tests

## Testing Primitives

### `test_case` - For rules with autofix

Each edge-case should be tested with a single `test_case` function that:
1. Runs the check in **assert mode** (captures violations)
2. Runs the check in **format mode** (captures auto-fixed output)
3. Snapshots both together
4. Verifies the formatted output passes the check (no remaining violations)

```rust
use crate::utils::{opts_for, test_case};

#[test]
fn edge_case_name() {
    insta::assert_snapshot!(test_case(
        r#"
        // input code with violation
        "#,
        &opts(),
    ), @"");
}
```

Snapshot format:
```
# Assert mode
[rule-name] /main.rs:N: violation message

# Format mode
// auto-fixed code output
```

### `test_case_assert_only` - For rules without autofix

For rules that only detect violations but have no autofix capability:

```rust
use crate::utils::{opts_for, test_case_assert_only};

#[test]
fn edge_case_name() {
    insta::assert_snapshot!(test_case_assert_only(
        r#"
        // input code with violation
        "#,
        &opts(),
    ), @"");
}
```

### `assert_check_passing` - For passing cases

For cases that should pass (no violations):

```rust
#[test]
fn valid_code_passes() {
    assert_check_passing(
        r#"
        // code that should have no violations
        "#,
        &opts(),
    );
}
```

## Test Organization

Each test file tests one rule. Within each file:

- One `#[test]` per semantic edge-case
- No duplicate coverage of the same edge-case
- Passing cases use `assert_check_passing`
- Violation cases use `test_case` (rules with autofix) or `test_case_assert_only` (rules without)
