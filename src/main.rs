// The schema is parsed before anything consumes it; drop this once the
// changelog writer reads these fields.
#![allow(dead_code)]

mod env;
mod parser;
mod schema;

use std::path::Path;
use std::process::ExitCode;

use clap::Parser;

fn main() -> ExitCode {
    let config = env::Config::parse();

    match parser::load_from(config.config_path.as_deref(), Path::new(".")) {
        Ok(gild_file) => {
            println!("{gild_file:#?}");

            // Already validated while parsing, so this cannot fail here.
            if let Ok(questions) = gild_file.resolved_questions() {
                println!("\nquestions:");
                for question in questions {
                    let answers = match &question.question_type {
                        schema::QuestionType::Choice { options } => {
                            format!(" [{}]", options.join(", "))
                        }
                        _ => String::new(),
                    };

                    println!("  {:<12} {}{answers}", question.id, question.title);
                }
            }

            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("gild: {error}");
            ExitCode::FAILURE
        }
    }
}
