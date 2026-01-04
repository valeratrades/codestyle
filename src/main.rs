use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod checks;

#[derive(Parser)]
#[command(author, version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_HASH"), ")"), about, long_about = None)]
struct Cli {
	#[command(subcommand)]
	command: Commands,
}

#[derive(Subcommand)]
enum Commands {
	/// Run all checks and assert they pass (exit 1 on failure)
	Assert {
		/// Target directory to check
		target_dir: PathBuf,

		/// Check for missing #[instrument] attributes
		#[arg(long)]
		instrument: bool,

		/// Check for endless loops without //LOOP comments
		#[arg(long)]
		loops: bool,
	},
}

fn main() {
	v_utils::clientside!();
	let cli = Cli::parse();

	match cli.command {
		Commands::Assert { target_dir, instrument, loops } => {
			run_assert(&target_dir, instrument, loops);
		}
	}
}

fn run_assert(target_dir: &PathBuf, instrument: bool, loops: bool) {
	if !target_dir.exists() {
		eprintln!("Target directory does not exist: {:?}", target_dir);
		std::process::exit(1);
	}

	let check_all = !instrument && !loops;

	let file_infos = checks::collect_rust_files(target_dir);
	let mut all_issues = Vec::new();

	if instrument || check_all {
		for info in &file_infos {
			all_issues.extend(checks::check_instrument(info));
		}
	}

	if loops || check_all {
		for info in &file_infos {
			all_issues.extend(checks::check_loops(info));
		}
	}

	if all_issues.is_empty() {
		std::process::exit(0);
	} else {
		for issue in &all_issues {
			eprintln!("{issue}");
		}
		std::process::exit(1);
	}
}
