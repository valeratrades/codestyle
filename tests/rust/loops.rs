use codestyle::rust_checks::{self, loops};

fn check_code(code: &str) -> Vec<String> {
	let temp_dir = std::env::temp_dir().join("codestyle_test_loops");
	std::fs::create_dir_all(&temp_dir).unwrap();
	let test_file = temp_dir.join("test.rs");
	std::fs::write(&test_file, code).unwrap();

	let file_infos = rust_checks::collect_rust_files(&temp_dir);
	let violations: Vec<String> = file_infos.iter().flat_map(|info| loops::check_loops(info)).map(|v| v.message).collect();

	std::fs::remove_file(&test_file).ok();
	std::fs::remove_dir(&temp_dir).ok();
	violations
}

fn main() {
	// Test: loop without comment triggers violation
	let violations = check_code(
		r#"
fn bad() {
    loop {
        break;
    }
}
"#,
	);
	assert_eq!(violations.len(), 1);
	assert!(violations[0].contains("//LOOP"));

	// Test: loop with inline comment passes
	let violations = check_code(
		r#"
fn good() {
    loop { //LOOP: justified reason
        break;
    }
}
"#,
	);
	assert!(violations.is_empty(), "inline comment should pass: {violations:?}");

	// Test: loop with comment on line above passes
	let violations = check_code(
		r#"
fn good() {
    //LOOP: justified reason
    loop {
        break;
    }
}
"#,
	);
	assert!(violations.is_empty(), "comment above should pass: {violations:?}");

	// Test: nested loop without comment
	let violations = check_code(
		r#"
fn nested() {
    if true {
        loop {
            break;
        }
    }
}
"#,
	);
	assert_eq!(violations.len(), 1, "nested loop should be caught: {violations:?}");

	// Test: while and for loops don't trigger (only endless loop)
	let violations = check_code(
		r#"
fn other_loops() {
    while true { break; }
    for i in 0..10 { break; }
}
"#,
	);
	assert!(violations.is_empty(), "while/for should not trigger: {violations:?}");

	// Test: loop inside closure
	let violations = check_code(
		r#"
fn with_closure() {
    let f = || {
        loop {
            break;
        }
    };
}
"#,
	);
	assert_eq!(violations.len(), 1, "loop in closure should be caught: {violations:?}");

	// Test: loop inside async block
	let violations = check_code(
		r#"
fn with_async() {
    let f = async {
        loop {
            break;
        }
    };
}
"#,
	);
	assert_eq!(violations.len(), 1, "loop in async should be caught: {violations:?}");

	println!("All loop tests passed!");
}
