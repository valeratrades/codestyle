An opinionated style checker and auto-formatter for Rust, layered on top of `rustfmt` and `clippy`.

It enforces conventions those tools don't cover (loop comments, impl block placement, format-string variable embedding, banned crates, manual `IGNORED_ERROR` annotations instead of silently masking with `unwrap_or`, etc.) and can either assert violations (CI) or auto-fix them in place (`format`).
