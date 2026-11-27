use std::io::Read;

use crate::diagrams::{architecture, class, sequence, usecase};

/// Limit inputs to 4 KiB to bound memory use.
const MAX_INPUT_BYTES: u64 = 4 * 1024;

const SUBCOMMANDS: &[Subcommand] = &[
    Subcommand { name: "sequence", about: "Create a sequence diagram", run: sequence::run },
    Subcommand { name: "class", about: "Create a class diagram", run: class::run },
    Subcommand { name: "usecase", about: "Create a use case diagram", run: usecase::run },
    Subcommand {
        name: "architecture",
        about: "Create an architecture diagram",
        run: architecture::run,
    },
];

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
        .map(|cmd| format!("  {:<13} {}", cmd.name, cmd.about))
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        "\
Create ASCII UML diagrams from text descriptions.

Usage: {} <COMMAND> FILE

Commands:
{commands}

Options:
  -h, --help     Show help
  -V, --version  Show version

Input must be a regular file, max {} KiB.

Sequence:
  sender --> receiver: request
  sender ..> receiver: reply

Class:
  layout {{ A, B }}
  A --|> B
  A *-- B
  A --> B

Use case:
  system S {{ Use case }}
  actor User
  User --> Use case

Architecture:
  group G {{ A; B }}
  A --> B
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
        return Err(Error::Usage("missing command".into()));
    };
    let (pkg_name, pkg_version) = (env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    for arg in args {
        match arg.as_str() {
            "-h" | "--help" => return Ok(usage()),
            "-V" | "--version" => return Ok(format!("{pkg_name} {pkg_version}")),
            opt if opt.starts_with('-') => return Err(Error::Usage("unknown option".into())),
            _ => {}
        }
    }
    SUBCOMMANDS.iter().find(|cmd| cmd.name == argument).map_or_else(
        || Err(Error::Usage("unknown command".into())),
        |cmd| run_subcommand(cmd, &args[1..]),
    )
}

fn run_subcommand(command: &Subcommand, args: &[String]) -> Result<String, Error> {
    let [path] = args else {
        return Err(match args.len() {
            0 => Error::Usage("missing input file".into()),
            _ => Error::Message("too many input files".into()),
        });
    };
    (command.run)(&read_input(path)?).map_err(Error::Message)
}

fn read_input(path: &str) -> Result<String, Error> {
    let err = |_| Error::Message("failed to read input file".into());
    let mut file = std::fs::File::open(path).map_err(err)?;
    let meta = file.metadata().map_err(err)?;
    if !meta.is_file() {
        return Err(Error::Message("input file is not a regular file".into()));
    } else if meta.len() > MAX_INPUT_BYTES {
        return Err(Error::Message("input file is too large".into()));
    }

    let mut input = String::new();
    file.read_to_string(&mut input).map_err(err)?;
    Ok(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_documents_commands_and_options() {
        let usage = usage();
        assert!(usage.contains("sequence"));
        assert!(usage.contains("class"));
        assert!(usage.contains("usecase"));
        assert!(usage.contains("architecture"));
        assert!(usage.contains("-h, --help"));
        assert!(usage.contains("-V, --version"));
        assert!(usage.contains("sender --> receiver"));
    }

    #[test]
    fn subcommands_are_registered() {
        assert_eq!(SUBCOMMANDS.len(), 4);
        assert_eq!(SUBCOMMANDS[0].name, "sequence");
        assert_eq!(SUBCOMMANDS[0].about, "Create a sequence diagram");
        assert_eq!(SUBCOMMANDS[1].name, "class");
        assert_eq!(SUBCOMMANDS[1].about, "Create a class diagram");
        assert_eq!(SUBCOMMANDS[2].name, "usecase");
        assert_eq!(SUBCOMMANDS[2].about, "Create a use case diagram");
        assert_eq!(SUBCOMMANDS[3].name, "architecture");
        assert_eq!(SUBCOMMANDS[3].about, "Create an architecture diagram");
    }

    #[test]
    fn accepts_help_and_version() {
        assert!(execute(&["--help".to_owned()]).is_ok());
        assert!(execute(&["-V".to_owned()]).is_ok());
    }

    #[test]
    fn runs_subcommands() {
        use std::io::Write;

        for (command, input) in
            [("sequence", "a --> b: hi\n"), ("architecture", "group G {\nA\n}\n")]
        {
            let mut file = tempfile::NamedTempFile::new().unwrap();
            file.write_all(input.as_bytes()).unwrap();

            let args = [command.to_owned(), file.path().display().to_string()];
            assert!(execute(&args).is_ok(), "command: {command}");
        }
    }

    #[test]
    fn rejects_input_larger_than_limit() {
        let file = tempfile::NamedTempFile::new().unwrap();
        file.as_file().set_len(MAX_INPUT_BYTES + 1).unwrap();

        let args = ["sequence".to_owned(), file.path().display().to_string()];
        let error = execute(&args).unwrap_err().to_string();
        assert_eq!(error, "input file is too large");
    }

    #[cfg(unix)]
    #[test]
    fn rejects_non_regular_file() {
        let directory = tempfile::TempDir::new().unwrap();
        let args = ["sequence".to_owned(), directory.path().display().to_string()];
        let error = execute(&args).unwrap_err().to_string();
        assert_eq!(error, "input file is not a regular file");
    }

    #[test]
    fn escapes_control_characters_in_arguments() {
        assert_eq!(
            execute(&["bad\u{1b}[2J".to_owned()]).unwrap_err().to_string(),
            "unknown command"
        );
        assert_eq!(
            execute(&["--bad\u{1b}[31m".to_owned()]).unwrap_err().to_string(),
            "unknown option"
        );
    }

    #[test]
    fn reports_error_cases() {
        let cases = [
            (vec![], "missing command"),
            (vec!["--bogus"], "unknown option"),
            (vec!["bogus"], "unknown command"),
            (vec!["sequence"], "missing input file"),
            (vec!["sequence", "a", "b"], "too many input files"),
        ];

        for (args, expected) in cases {
            let args = args.iter().map(ToString::to_string).collect::<Vec<_>>();
            assert_eq!(execute(&args).unwrap_err().to_string(), expected);
        }

        let error = execute(&["sequence".to_owned(), "missing-file".to_owned()]).unwrap_err();
        assert_eq!(error.to_string(), "failed to read input file");
    }
}
