use crate::sequence;

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
    run: fn(&str),
}

const SUBCOMMANDS: &[Subcommand] =
    &[Subcommand { name: "sequence", about: "Create a sequence diagram", run: sequence::run }];

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

Sequence input format:
  sender -> receiver: description
  sender <- receiver: description
",
        env!("CARGO_PKG_NAME"),
    )
}

pub fn run(args: &[String]) {
    if let Err(error) = execute(args) {
        match error {
            Error::Usage(message) => eprintln!("error: {message}\n\n{}", usage()),
            Error::Message(message) => eprintln!("error: {message}"),
        }
    }
}

fn execute(args: &[String]) -> Result<(), Error> {
    let Some(argument) = args.first() else {
        return Err(Error::Usage("missing command".to_owned()));
    };

    for argument in args {
        match argument.as_str() {
            "-h" | "--help" => {
                print!("{}", usage());
                return Ok(());
            }
            "-V" | "--version" => {
                println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            option if option.starts_with('-') => {
                return Err(Error::Usage(format!("unrecognized option '{option}'")));
            }
            _ => {}
        }
    }

    SUBCOMMANDS.iter().find(|command| command.name == argument).map_or_else(
        || Err(Error::Usage(format!("unrecognized command '{argument}'"))),
        |command| run_subcommand(command, &args[1..]),
    )
}

fn run_subcommand(command: &Subcommand, args: &[String]) -> Result<(), Error> {
    let Some(path) = args.first() else {
        return Err(Error::Usage("missing input file".to_owned()));
    };

    if args.len() > 1 {
        return Err(Error::Message("only one input file may be specified".to_owned()));
    }

    let input = std::fs::read_to_string(path)
        .map_err(|error| Error::Message(format!("failed to read '{path}': {error}")))?;

    (command.run)(&input);
    Ok(())
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
        let path = std::env::temp_dir().join("ascii-uml-cli-test.txt");
        std::fs::write(&path, "a -> b: hi\n").unwrap();
        let args = ["sequence".to_owned(), path.display().to_string()];
        assert!(execute(&args).is_ok());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn reports_error_cases() {
        let cases = [
            (vec![], "missing command"),
            (vec!["--bogus"], "unrecognized option '--bogus'"),
            (vec!["bogus"], "unrecognized command 'bogus'"),
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
