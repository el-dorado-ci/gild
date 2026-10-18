use std::path::Path;

use clap::Args;

use crate::commands::{Command, Error};
use crate::schema::{BUMP_TYPE_ID, BUMP_TYPES, DESCRIPTION_ID, GildFileSparse};
use crate::utils::{self, Changefile};

#[derive(Args)]
pub struct Generate;

impl Command for Generate {
    fn run(&self, gild_file: &GildFileSparse, _interactive: bool) -> Result<(), Error> {
        let dir = Path::new(&gild_file.changefile_directory);
        let changefiles = utils::read_changefiles(dir)?;

        if changefiles.is_empty() {
            println!(
                "{}: no changefiles in {}",
                gild_file.artifact_name,
                dir.display()
            );
            return Ok(());
        }

        println!("# {} changelog preview\n", gild_file.artifact_name);

        let mut wrote_a_section = false;
        for bump in BUMP_TYPES {
            let matching: Vec<&Changefile> = changefiles
                .iter()
                .filter(|changefile| field(changefile, BUMP_TYPE_ID) == Some(bump))
                .collect();

            if matching.is_empty() {
                continue;
            }

            wrote_a_section = true;
            println!("## {bump}\n");
            for changefile in matching {
                let description = field(changefile, DESCRIPTION_ID).unwrap_or("(no description)");
                println!("- {description}  _{}_", changefile.name);
            }
            println!();
        }

        if !wrote_a_section {
            println!(
                "{} changefile(s) found, but none set `{BUMP_TYPE_ID}`",
                changefiles.len()
            );
        }

        Ok(())
    }
}

fn field<'a>(changefile: &'a Changefile, key: &str) -> Option<&'a str> {
    changefile
        .fields
        .get(key)
        .and_then(serde_yaml::Value::as_str)
}
