use std::path::Path;

use clap::Args;
use colored::Colorize;
use serde_json::Value;

use crate::commands::{Command, Error};
use crate::schema::GildFileSparse;
use crate::utils;

#[derive(Args)]
pub struct Query {
    jmespath: String,
}

pub fn run_query(jmespath: &str, dir: &Path) -> Result<(), Error> {
    let expression = jmespath::compile(jmespath)
        .map_err(|source| Error::InvalidQuery(jmespath.to_owned(), Box::new(source)))?;

    let documents: Vec<Value> = utils::read_changefiles(dir)?
        .into_iter()
        .map(|changefile| serde_json::to_value(changefile.fields))
        .collect::<Result<_, _>>()?;

    let result = expression
        .search(Value::Array(documents))
        .map_err(|source| Error::QueryFailed(Box::new(source)))?;

    println!("{}", colorize_json(&serde_json::to_string_pretty(&result)?));

    Ok(())
}

impl Command for Query {
    fn run(&self, gild_file: &GildFileSparse, _interactive: bool) -> Result<(), Error> {
        run_query(&self.jmespath, Path::new(&gild_file.changefile_directory))
    }
}

fn colorize_json(json: &str) -> String {
    json.lines()
        .map(|line| match line.trim() {
            trimmed if trimmed.starts_with('"') => match trimmed.split_once(':') {
                Some((key, value)) => format!("{}:{}", key.blue(), value),
                None => line.green().to_string(),
            },
            "true" | "false" => line.yellow().to_string(),
            "null" => line.dimmed().to_string(),
            _ => line.to_string(),
        })
        .collect::<Vec<_>>()
        .join("\n")
}
