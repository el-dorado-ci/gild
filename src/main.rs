mod commands;
mod env;
mod forge;
mod git;
mod parser;
mod prompt;
mod schema;
mod utils;
mod version;
mod words;

use std::error::Error;
use std::path::Path;
use std::process::ExitCode;

use crate::env::Command;

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
    let preloaded = parser::load_from(env::preparse_config_path().as_deref(), Path::new("."));
    let questions = preloaded
        .as_ref()
        .ok()
        .and_then(|gild_file| gild_file.resolved_questions().ok())
        .unwrap_or_default();

    let (mut config, matches) = match env::try_parse(&questions) {
        Ok(parsed) => parsed,
        Err(invalid_args) => {
            if invalid_args.use_stderr() {
                preloaded?;
            }
            invalid_args.exit();
        }
    };

    let gild_file = preloaded?;

    if let Command::Add(add) = &mut config.command {
        add.answers = env::answers(&matches, &questions);
    }

    commands::dispatch(config.command, &gild_file, !config.non_interactive)?;

    Ok(())
}
