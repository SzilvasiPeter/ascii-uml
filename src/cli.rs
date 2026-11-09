use crate::sequence;

struct Subcommand {
    name: &'static str,
    about: &'static str,
    run: fn(&[String]),
}

const SUBCOMMANDS: &[Subcommand] =
    &[Subcommand { name: "sequence", about: "Create a sequence diagram", run: sequence::run }];

const OPTIONS: &str = "\
Options:
  -h, --help     Show this help message
  -V, --version  Show version information";

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

{OPTIONS}

Sequence input format:
  sender -> receiver: description
  sender <- receiver: description
",
        env!("CARGO_PKG_NAME"),
    )
}

pub fn run(args: &[String]) {
    let Some(argument) = args.first() else {
        return usage_error("missing command");
    };

    for argument in args {
        match argument.as_str() {
            "-h" | "--help" => {
                print!("{}", usage());
                return;
            }
            "-V" | "--version" => {
                println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
                return;
            }
            option if option.starts_with('-') => {
                usage_error(&format!("unrecognized option '{option}'"));
                return;
            }
            _ => {}
        }
    }

    match SUBCOMMANDS.iter().find(|command| command.name == argument) {
        Some(command) => (command.run)(&args[1..]),
        None => usage_error(&format!("unrecognized command '{argument}'")),
    }
}

pub fn usage_error(message: &str) {
    eprintln!("error: {message}\n\n{}", usage());
}
