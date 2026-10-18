use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use clap::Args;
use rand::seq::IndexedRandom;

use crate::commands::{Command, Error};
use crate::prompt::{Interactive, NonInteractive, Prompter};
use crate::schema::GildFileSparse;
use crate::words;

#[derive(Args)]
pub struct Add;

impl Command for Add {
    fn run(&self, gild_file: &GildFileSparse, interactive: bool) -> Result<(), Error> {
        let questions = gild_file.resolved_questions()?;

        let stdin = io::stdin();
        let mut prompter: Box<dyn Prompter> = if interactive {
            Box::new(Interactive)
        } else {
            Box::new(NonInteractive::new(stdin.lock()))
        };

        let mut answers = serde_yaml::Mapping::new();
        for question in &questions {
            let answer = prompter.ask(question)?;
            answers.insert(
                serde_yaml::Value::String(question.id.clone()),
                answer.into_yaml(),
            );
        }

        let dir = Path::new(&gild_file.changefile_directory);
        fs::create_dir_all(dir)?;

        let path = unique_changefile_path(dir)?;
        fs::write(&path, serde_yaml::to_string(&answers)?)?;

        println!("wrote {}", path.display());

        Ok(())
    }
}

fn unique_changefile_path(dir: &Path) -> Result<PathBuf, Error> {
    let mut rng = rand::rng();

    for _ in 0..50 {
        let adjective = words::ADJECTIVES.choose(&mut rng).expect("non-empty");
        let noun = words::NOUNS.choose(&mut rng).expect("non-empty");
        let verb = words::VERBS.choose(&mut rng).expect("non-empty");

        let candidate = dir.join(format!("{adjective}-{noun}-{verb}.yaml"));
        if !candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(Error::NoAvailableName(dir.to_path_buf()))
}
