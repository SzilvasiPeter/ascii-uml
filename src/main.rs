#![forbid(unsafe_code)]
mod class;
mod cli;
mod sequence;

fn main() {
    cli::run(&std::env::args().skip(1).collect::<Vec<_>>());
}
