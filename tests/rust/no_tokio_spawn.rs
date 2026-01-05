use codestyle::test_fixture::{assert_check_passing, simulate_check};

use crate::utils::opts_for;

fn opts() -> codestyle::rust_checks::RustCheckOptions {
	opts_for("no_tokio_spawn")
}

#[test]
fn tokio_spawn_is_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		async fn main() {
			tokio::spawn(async { println!("hello"); });
		}
		"#,
		&opts(),
	), @r#"[no-tokio-spawn] /main.rs:2: Usage of `tokio::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/"#);
}

#[test]
fn tokio_spawn_blocking_is_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		async fn main() {
			tokio::spawn_blocking(|| { std::thread::sleep(std::time::Duration::from_secs(1)); });
		}
		"#,
		&opts(),
	), @r#"[no-tokio-spawn] /main.rs:2: Usage of `tokio::spawn_blocking` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/"#);
}

#[test]
fn tokio_task_spawn_is_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		async fn main() {
			tokio::task::spawn(async { println!("hello"); });
		}
		"#,
		&opts(),
	), @r#"[no-tokio-spawn] /main.rs:2: Usage of `tokio::task::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/"#);
}

#[test]
fn tokio_task_spawn_blocking_is_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		async fn main() {
			tokio::task::spawn_blocking(|| { println!("blocking"); });
		}
		"#,
		&opts(),
	), @r#"[no-tokio-spawn] /main.rs:2: Usage of `tokio::task::spawn_blocking` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/"#);
}

#[test]
fn tokio_spawn_local_is_violation() {
	insta::assert_snapshot!(simulate_check(
		r#"
		async fn main() {
			tokio::spawn_local(async { println!("local"); });
		}
		"#,
		&opts(),
	), @r#"[no-tokio-spawn] /main.rs:2: Usage of `tokio::spawn_local` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/"#);
}

#[test]
fn non_tokio_spawn_passes() {
	assert_check_passing(
		r#"
		fn spawn_process(name: &str) {
			println!("Spawning process: {}", name);
		}

		fn main() {
			spawn_process("test");
		}
		"#,
		&opts(),
	);
}

#[test]
fn other_async_code_passes() {
	assert_check_passing(
		r#"
		async fn do_work() {
			let result = async { 42 }.await;
			println!("{result}");
		}

		fn main() {}
		"#,
		&opts(),
	);
}

#[test]
fn multiple_tokio_spawn_violations() {
	insta::assert_snapshot!(simulate_check(
		r#"
		async fn main() {
			tokio::spawn(async { println!("1"); });
			tokio::spawn(async { println!("2"); });
			tokio::spawn_blocking(|| { println!("3"); });
		}
		"#,
		&opts(),
	), @r#"
	[no-tokio-spawn] /main.rs:2: Usage of `tokio::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:3: Usage of `tokio::spawn` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	[no-tokio-spawn] /main.rs:4: Usage of `tokio::spawn_blocking` is disallowed. Unstructured concurrency makes code harder to reason about. See: https://vorpus.org/blog/notes-on-structured-concurrency-or-go-statement-considered-harmful/
	"#);
}

#[test]
fn nested_tokio_spawn_is_violation() {
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
