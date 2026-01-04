use codestyle::rust_checks::{self, Violation, loops};

fn check_code(code: &str) -> Vec<Violation> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_loops");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<Violation> = file_infos.iter().flat_map(|info| loops::check_loops(info)).collect();

	std::fs::remove_file(&test_file).ok();
	std::fs::remove_dir(&temp_dir).ok();
	violations
}

fn snapshot_violations(violations: &[Violation]) -> String {
	if violations.is_empty() {
		"(no violations)".to_string()
	} else {
		violations.iter().map(|v| &v.message).cloned().collect::<Vec<_>>().join("\n")
	}
}

fn main() {
	// Test: loop without comment triggers violation
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn bad() {
    loop {
        break;
    }
}
"#,
	)), @"Endless loop without `//LOOP` comment");

	// Test: loop with inline comment passes
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn good() {
    loop { //LOOP: justified reason
        break;
    }
}
"#,
	)), @"(no violations)");

	// Test: loop with comment on line above passes
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn good() {
    //LOOP: justified reason
    loop {
        break;
    }
}
"#,
	)), @"(no violations)");

	// Test: nested loop without comment
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn nested() {
    if true {
        loop {
            break;
        }
    }
}
"#,
	)), @"Endless loop without `//LOOP` comment");

	// Test: while and for loops don't trigger (only endless loop)
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn other_loops() {
    while true { break; }
    for i in 0..10 { break; }
}
"#,
	)), @"(no violations)");

	// Test: loop inside closure
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn with_closure() {
    let f = || {
        loop {
            break;
        }
    };
}
"#,
	)), @"Endless loop without `//LOOP` comment");

	// Test: loop inside async block
	insta::assert_snapshot!(snapshot_violations(&check_code(
		r#"
fn with_async() {
    let f = async {
        loop {
            break;
        }
    };
}
"#,
	)), @"Endless loop without `//LOOP` comment");

	println!("All loop tests passed!");
}
