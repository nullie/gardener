use clap::{Parser, Subcommand};
use rootcause::hooks::Hooks;
use rootcause_backtrace::BacktraceCollector;

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
    Backup {
        #[command(subcommand)]
        command: BackupSubcommand,
    },
}

#[derive(Subcommand, Clone)]
enum CheckUntrackedSubCommand {
    SuggestConfig,
    Plain,
}

#[derive(Subcommand)]
enum BackupSubcommand {
    Ls,
    Size,
}

fn main() -> Result<(), rootcause::compat::MainReport> {
    // Capture backtraces for all errors
    Hooks::new()
        .report_creation_hook(BacktraceCollector::new_from_env())
        .install()
        .expect("failed to install hooks");

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
        Command::Backup {
            command: BackupSubcommand::Ls,
        } => gardener::backup::ls(),
        Command::Backup {
            command: BackupSubcommand::Size,
        } => gardener::backup::size(),
    }?;

    Ok(())
}
