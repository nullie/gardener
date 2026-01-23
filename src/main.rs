use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    CheckUntracked,
    CheckTracked,
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::CheckUntracked => gardener::untracked::check_untracked(),
        Command::CheckTracked => gardener::tracked::check_tracked(),
    }
}
