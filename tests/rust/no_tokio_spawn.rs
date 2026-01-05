use codestyle::test_fixture::{assert_check_passing, simulate_check};

use crate::utils::opts_for;

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("no_tokio_spawn")
}

#[test]
fn all_spawn_variants_are_violations() {
	insta::assert_snapshot!(simulate_check(
		r#"
		async fn main() {
			tokio::spawn(async { println!("1"); });
			tokio::spawn_blocking(|| { println!("2"); });
			tokio::task::spawn(async { println!("3"); });
			tokio::task::spawn_blocking(|| { println!("4"); });
			tokio::spawn_local(async { println!("5"); });
		}
		"#,
		&opts(),
	), @r#"
	[no-tokio-spawn] /main.rs:2: Usage of `tokio::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:3: Usage of `tokio::spawn_blocking` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:4: Usage of `tokio::task::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:5: Usage of `tokio::task::spawn_blocking` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:6: Usage of `tokio::spawn_local` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	"#);
}

#[test]
fn nested_spawn_detected() {
	insta::assert_snapshot!(simulate_check(
		r#"
		async fn main() {
			let handle = tokio::spawn(async {
				tokio::spawn(async { println!("nested"); });
			});
		}
		"#,
		&opts(),
	), @r#"
	[no-tokio-spawn] /main.rs:2: Usage of `tokio::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:3: Usage of `tokio::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	"#);
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
