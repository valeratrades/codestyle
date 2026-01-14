use crate::utils::{assert_check_passing, opts_for, test_case_assert_only};

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("no_tokio_spawn")
}

// === Passing cases ===

#[test]
fn spawn_blocking_is_allowed() {
	assert_check_passing(
		r#"
		async fn main() {
			tokio::spawn_blocking(|| { println!("blocking"); });
			tokio::task::spawn_blocking(|| { println!("also blocking"); });
		}
		"#,
		&opts(),
	);
}

#[test]
fn non_tokio_spawn_passes() {
	assert_check_passing(
		r#"
		fn spawn_process(name: &str) {
			println!("Spawning process: {}", name);
		}

		async fn do_work() {
			let result = async { 42 }.await;
			println!("{result}");
		}

		fn main() {
			spawn_process("test");
		}
		"#,
		&opts(),
	);
}

// === Violation cases (no autofix) ===

#[test]
fn spawn_variants() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		async fn main() {
			tokio::spawn(async { println!("1"); });
			tokio::task::spawn(async { println!("2"); });
			tokio::spawn_local(async { println!("3"); });
			tokio::task::spawn_local(async { println!("4"); });
		}
		"#,
		&opts(),
	), @"
	[no-tokio-spawn] /main.rs:2: Usage of `tokio::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:3: Usage of `tokio::task::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:4: Usage of `tokio::spawn_local` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:5: Usage of `tokio::task::spawn_local` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	");
}

#[test]
fn nested_spawn() {
	insta::assert_snapshot!(test_case_assert_only(
		r#"
		async fn main() {
			let handle = tokio::spawn(async {
				tokio::spawn(async { println!("nested"); });
			});
		}
		"#,
		&opts(),
	), @"
	[no-tokio-spawn] /main.rs:2: Usage of `tokio::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:3: Usage of `tokio::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	");
}
