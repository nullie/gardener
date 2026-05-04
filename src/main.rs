use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    CheckUntracked {
        #[command(subcommand)]
        opt_command: Option<CheckUntrackedSubCommand>,
    },
    CheckTracked,
}

#[derive(Subcommand, Clone)]
enum CheckUntrackedSubCommand {
    SuggestConfig,
    Plain,
}

fn main() -> eyre::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::CheckUntracked { opt_command: None } => gardener::untracked::check_untracked(),
        Command::CheckUntracked {
            opt_command: Some(CheckUntrackedSubCommand::SuggestConfig),
        } => gardener::untracked::suggest_config(),
        Command::CheckUntracked {
            opt_command: Some(CheckUntrackedSubCommand::Plain),
        } => gardener::untracked::print_untracked(),
        Command::CheckTracked => gardener::tracked::check_tracked(),
    }
}
