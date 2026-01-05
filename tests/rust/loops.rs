use codestyle::{
	rust_checks::{self, Violation, loops},
	test_fixture::Fixture,
};

fn check_violations(code: &str, expected: &[&str]) {
	let fixture = Fixture::parse(code);
	let temp = fixture.write_to_tempdir();

	let file_infos = rust_checks::collect_rust_files(&temp.root);
	let violations: Vec<Violation> = file_infos.iter().flat_map(|info| loops::check_loops(info)).collect();
	let messages: Vec<&str> = violations.iter().map(|v| v.message.as_str()).collect();

	assert_eq!(messages, expected, "Violations mismatch for fixture:\n{code}");
}

fn check_ok(code: &str) {
	check_violations(code, &[]);
}

fn main() {
	// loop without comment triggers violation
	check_violations(
		r#"
		fn bad() {
			loop {
				break;
			}
		}
		"#,
		&["Endless loop without `//LOOP` comment"],
	);

	// loop with inline comment passes
	check_ok(
		r#"
		fn good() {
			loop { //LOOP: justified reason
				break;
			}
		}
		"#,
	);

	// loop with comment on line above passes
	check_ok(
		r#"
		fn good() {
			//LOOP: justified reason
			loop {
				break;
			}
		}
		"#,
	);

	// nested loop without comment
	check_violations(
		r#"
		fn nested() {
			if true {
				loop {
					break;
				}
			}
		}
		"#,
		&["Endless loop without `//LOOP` comment"],
	);

	// while and for loops don't trigger (only endless loop)
	check_ok(
		r#"
		fn other_loops() {
			while true { break; }
			for i in 0..10 { break; }
		}
		"#,
	);

	// loop inside closure
	check_violations(
		r#"
		fn with_closure() {
			let f = || {
				loop {
					break;
				}
			};
		}
		"#,
		&["Endless loop without `//LOOP` comment"],
	);

	// loop inside async block
	check_violations(
		r#"
		fn with_async() {
			let f = async {
				loop {
					break;
				}
			};
		}
		"#,
		&["Endless loop without `//LOOP` comment"],
	);

	println!("All loop tests passed!");
}
