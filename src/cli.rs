use std::io::Read;

use crate::sequence;

/// Upper bound for input files is 4 KiB to protect against OOM.
const MAX_INPUT_BYTES: u64 = 4 * 1024;

const SUBCOMMANDS: &[Subcommand] =
    &[Subcommand { name: "sequence", about: "Create a sequence diagram", run: sequence::run }];

enum Error {
    Usage(String),
    Message(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Usage(message) | Self::Message(message) => formatter.write_str(message),
        }
    }
}

struct Subcommand {
    name: &'static str,
    about: &'static str,
    run: fn(&str) -> Result<String, String>,
}

pub fn usage() -> String {
    let commands = SUBCOMMANDS
        .iter()
        .map(|command| format!("  {:<10} {}", command.name, command.about))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "\
Create ASCII UML diagrams from simple text descriptions.

Usage: {} <COMMAND> [OPTIONS] FILE

Commands:
{commands}

Options:
  -h, --help     Show this help message
  -V, --version  Show version information

Input must be a regular file no larger than {} KiB.

Sequence input format:
  sender -> receiver: description
  sender <- receiver: description
",
        env!("CARGO_PKG_NAME"),
        MAX_INPUT_BYTES / 1024,
    )
}

pub fn run(args: &[String]) {
    match execute(args) {
        Ok(message) => println!("{message}"),
        Err(error) => match error {
            Error::Usage(message) => eprintln!("error: {message}\n\n{}", usage()),
            Error::Message(message) => eprintln!("error: {message}"),
        },
    }
}

fn execute(args: &[String]) -> Result<String, Error> {
    let Some(argument) = args.first() else {
        return Err(Error::Usage("missing command".to_owned()));
    };

    for argument in args {
        match argument.as_str() {
            "-h" | "--help" => {
                return Ok(usage());
            }
            "-V" | "--version" => {
                return Ok(format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION")));
            }
            option if option.starts_with('-') => {
                return Err(Error::Usage(format!("unknown option '{}'", option.escape_debug())));
            }
            _ => {}
        }
    }

    SUBCOMMANDS.iter().find(|command| command.name == argument).map_or_else(
        || Err(Error::Usage(format!("unknown command '{}'", argument.escape_debug()))),
        |command| run_subcommand(command, &args[1..]),
    )
}

fn run_subcommand(command: &Subcommand, args: &[String]) -> Result<String, Error> {
    let Some(path) = args.first() else {
        return Err(Error::Usage("missing input file".to_owned()));
    };

    if args.len() > 1 {
        return Err(Error::Message("only one input file may be specified".to_owned()));
    }

    let input = read_input(path)?;
    let output = (command.run)(&input).map_err(Error::Message)?;

    Ok(output)
}

fn read_input(path: &str) -> Result<String, Error> {
    let escaped_path = path.escape_debug();
    let mut file = std::fs::File::open(path)
        .map_err(|error| Error::Message(format!("failed to read '{escaped_path}': {error}")))?;
    let metadata = file
        .metadata()
        .map_err(|error| Error::Message(format!("failed to read '{escaped_path}': {error}")))?;

    if !metadata.is_file() {
        return Err(Error::Message(format!("input file '{escaped_path}' is not a regular file")));
    }

    if metadata.len() > MAX_INPUT_BYTES {
        return Err(Error::Message(format!(
            "file '{escaped_path}' exceeds {MAX_INPUT_BYTES} bytes"
        )));
    }

    let mut input = String::new();
    file.read_to_string(&mut input)
        .map_err(|error| Error::Message(format!("failed to read '{escaped_path}': {error}")))?;

    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_documents_commands_and_options() {
        let usage = usage();
        assert!(usage.contains("sequence"));
        assert!(usage.contains("-h, --help"));
        assert!(usage.contains("-V, --version"));
        assert!(usage.contains("sender -> receiver"));
    }

    #[test]
    fn sequence_is_registered() {
        assert_eq!(SUBCOMMANDS.len(), 1);
        assert_eq!(SUBCOMMANDS[0].name, "sequence");
        assert_eq!(SUBCOMMANDS[0].about, "Create a sequence diagram");
    }

    #[test]
    fn accepts_help_and_version() {
        assert!(execute(&["--help".to_owned()]).is_ok());
        assert!(execute(&["-V".to_owned()]).is_ok());
    }

    #[test]
    fn runs_sequence_subcommand() {
        use std::io::Write;

        let mut file = tempfile::NamedTempFile::new().unwrap();
        file.write_all(b"a -> b: hi\n").unwrap();

        let args = ["sequence".to_owned(), file.path().display().to_string()];
        assert!(execute(&args).is_ok());
    }

    #[test]
    fn rejects_input_larger_than_limit() {
        let file = tempfile::NamedTempFile::new().unwrap();
        file.as_file().set_len(MAX_INPUT_BYTES + 1).unwrap();

        let args = ["sequence".to_owned(), file.path().display().to_string()];
        let error = execute(&args).unwrap_err().to_string();
        assert!(error.contains(&format!("exceeds {MAX_INPUT_BYTES} bytes")));
    }

    #[cfg(unix)]
    #[test]
    fn rejects_non_regular_file() {
        let directory = tempfile::TempDir::new().unwrap();
        let args = ["sequence".to_owned(), directory.path().display().to_string()];
        let error = execute(&args).unwrap_err().to_string();
        assert!(error.contains("is not a regular file"));
    }

    #[test]
    fn escapes_control_characters_in_arguments() {
        let command = "bad\u{1b}[2J".to_owned();
        let error = execute(&[command]).unwrap_err().to_string();
        assert_eq!(error, r"unknown command 'bad\u{1b}[2J'");

        let option = "--bad\u{1b}[31m".to_owned();
        let error = execute(&[option]).unwrap_err().to_string();
        assert!(error.contains(r"--bad\u{1b}[31m"));

        let path = "missing\u{1b}]0;pwned\u{7}".to_owned();
        let error = execute(&["sequence".to_owned(), path]).unwrap_err().to_string();
        assert!(error.contains(r"missing\u{1b}]0;pwned\u{7}"));
        assert!(!error.contains('\u{1b}'));
        assert!(!error.contains('\u{7}'));
    }

    #[test]
    fn reports_error_cases() {
        let cases = [
            (vec![], "missing command"),
            (vec!["--bogus"], "unknown option '--bogus'"),
            (vec!["bogus"], "unknown command 'bogus'"),
            (vec!["sequence"], "missing input file"),
            (vec!["sequence", "a", "b"], "only one input file may be specified"),
        ];

        for (args, expected) in cases {
            let args = args.iter().map(ToString::to_string).collect::<Vec<_>>();
            assert_eq!(execute(&args).unwrap_err().to_string(), expected);
        }

        let error = execute(&["sequence".to_owned(), "missing-file".to_owned()]).unwrap_err();
        assert!(error.to_string().starts_with("failed to read 'missing-file':"));
    }
}
