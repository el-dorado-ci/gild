// The schema is parsed before anything consumes it; drop this once the
// changelog writer reads these fields.
#![allow(dead_code)]

mod commands;
mod env;
mod parser;
mod prompt;
mod schema;
mod words;

use std::path::Path;
use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let config = env::Config::parse();

    let gild_file = match parser::load_from(config.config_path.as_deref(), Path::new(".")) {
        Ok(gild_file) => gild_file,
        Err(error) => {
            eprintln!("gild: {error}");
            return ExitCode::FAILURE;
        }
    };

    match commands::dispatch(config.command, &gild_file, !config.non_interactive) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gild: {error}");
            ExitCode::FAILURE
        }
    }
}
