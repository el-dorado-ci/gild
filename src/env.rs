use clap::builder::{PossibleValue, PossibleValuesParser};
use clap::{Arg, ArgMatches, CommandFactory, FromArgMatches, Parser, Subcommand};

use crate::commands::add::Add;
use crate::commands::ask::Ask;
use crate::commands::generate::Generate;
use crate::commands::query::Query;
use crate::commands::version::Version;
use crate::prompt::Answer;
use crate::schema::{Question, QuestionType, flag};

#[derive(Parser)]
pub struct Config {
    #[arg(long, env = "GILD_CONFIG_PATH")]
    pub config_path: Option<String>,

    #[arg(long, global = true, env = "GILD_NON_INTERACTIVE")]
    pub non_interactive: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Add(Add),
    Generate(Generate),
    Query(Query),
    Ask(Ask),
    Version(Version),
}

pub fn preparse_config_path() -> Option<String> {
    Config::command()
        .ignore_errors(true)
        .disable_help_flag(true)
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .try_get_matches()
        .ok()?
        .get_one::<String>("config_path")
        .cloned()
}

pub fn try_parse(questions: &[Question]) -> Result<(Config, ArgMatches), clap::Error> {
    let command = Config::command().mut_subcommand("add", |add| {
        questions
            .iter()
            .fold(add, |add, question| add.arg(question_arg(question)))
    });

    let matches = command.try_get_matches()?;
    let config = Config::from_arg_matches(&matches)?;

    Ok((config, matches))
}

fn leak(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn question_arg(question: &Question) -> Arg {
    let long = flag(&question.id);
    let arg = Arg::new(leak(question.id.clone()))
        .long(leak(long.clone()))
        .help(leak(question.title.clone()))
        .value_name(leak(long.to_uppercase()));

    match &question.question_type {
        QuestionType::Choice { options } => {
            let values: Vec<PossibleValue> = options
                .iter()
                .map(|option| PossibleValue::new(leak(option.clone())))
                .collect();

            arg.value_parser(PossibleValuesParser::new(values))
        }
        QuestionType::Text => arg,
        QuestionType::Bool => arg.value_parser(clap::value_parser!(bool)),
    }
}

pub fn answers(matches: &ArgMatches, questions: &[Question]) -> Vec<(String, Answer)> {
    let Some(matches) = matches.subcommand_matches("add") else {
        return Vec::new();
    };

    questions
        .iter()
        .filter_map(|question| {
            let answer = match &question.question_type {
                QuestionType::Choice { .. } => matches
                    .get_one::<String>(&question.id)
                    .cloned()
                    .map(Answer::Choice),
                QuestionType::Text => matches
                    .get_one::<String>(&question.id)
                    .cloned()
                    .map(Answer::Text),
                QuestionType::Bool => matches
                    .get_one::<bool>(&question.id)
                    .copied()
                    .map(Answer::Bool),
            }?;

            Some((question.id.clone(), answer))
        })
        .collect()
}
