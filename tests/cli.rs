// Drives the built binary the way a user would: from a working directory, with
// GILD_CONFIG_PATH either set or not.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn fixture(dir: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(dir)
}

fn run(dir: &str, config_path: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_gild"));
    command.current_dir(fixture(dir));

    match config_path {
        Some(path) => command.env("GILD_CONFIG_PATH", path),
        None => command.env_remove("GILD_CONFIG_PATH"),
    };

    command.output().expect("failed to run gild")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn discovers_a_dedicated_gild_file() {
    let output = run("dedicated", None);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains(r#"artifact_name: "dedicated""#));
}

#[test]
fn discovers_a_pyproject_file() {
    let output = run("python", None);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains(r#"artifact_name: "widget-py""#));
}

#[test]
fn discovers_a_cargo_manifest() {
    let output = run("rust", None);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains(r#"artifact_name: "widget-rs""#));
}

#[test]
fn skips_a_manifest_that_holds_no_config() {
    let output = run("polyglot", None);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains(r#"artifact_name: "widget-polyglot""#));
}

#[test]
fn the_env_var_overrides_the_working_directory() {
    let config_path = fixture("python").join("pyproject.toml");
    let output = run("dedicated", Some(&config_path));

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(stdout(&output).contains(r#"artifact_name: "widget-py""#));
}

#[test]
fn exits_nonzero_when_no_config_is_found() {
    let output = run("bare", None);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("no gild config"));
}

#[test]
fn exits_nonzero_when_the_configured_path_holds_no_config() {
    let config_path = fixture("bare").join("Cargo.toml");
    let output = run("dedicated", Some(&config_path));

    assert!(!output.status.success());
    assert!(stderr(&output).contains("[package.metadata.gild]"));
}

#[test]
fn exits_nonzero_when_the_configured_path_is_missing() {
    let config_path = fixture("dedicated").join("nope.toml");
    let output = run("dedicated", Some(&config_path));

    assert!(!output.status.success());
    assert!(stderr(&output).contains("could not read"));
}

#[test]
fn exits_nonzero_on_malformed_toml() {
    let output = run("malformed", None);

    assert!(!output.status.success());
    assert!(stderr(&output).contains("could not parse"));
}
