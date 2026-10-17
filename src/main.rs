mod commands;
mod env;
mod parser;
mod prompt;
mod schema;
mod utils;
mod words;

use std::error::Error;
use std::path::Path;
use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gild: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let config = env::Config::parse();
    let gild_file = parser::load_from(config.config_path.as_deref(), Path::new("."))?;

    commands::dispatch(config.command, &gild_file, !config.non_interactive)?;

    Ok(())
}
