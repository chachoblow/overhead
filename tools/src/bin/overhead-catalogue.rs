//! Offline catalogue assembly; shared loader preserves atomic load/reporting.
use overhead_tools::catalogue::{USAGE, load};
use std::{env, path::Path, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    let result = match args.as_slice() {
        [path] => load(Path::new(path)),
        _ => Err(format!("expected one manifest argument\n\n{USAGE}")),
    };
    match result {
        Ok(loaded) => {
            eprint!("{}", loaded.diagnostics);
            print!("{}", loaded.report);
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
