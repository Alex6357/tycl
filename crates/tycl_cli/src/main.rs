mod cmd;
mod codegen;

use clap::Parser;
use std::process;

#[derive(Parser)]
#[command(name = "tycl", about = "TyCL configuration toolkit")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand)]
enum Command {
    Convert(cmd::convert::Args),
    Generate(cmd::generate::Args),
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Convert(args) => cmd::convert::run(args),
        Command::Generate(args) => cmd::generate::run(args),
    };
    if let Err(e) = result {
        eprintln!("error: {:#}", e);
        process::exit(1);
    }
}
