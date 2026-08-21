use std::process::ExitCode;

use clap::{Parser, Subcommand, ValueEnum};
use yap_core::Action;
use yap_daemon::dbus::Client;

#[derive(Debug, Parser)]
#[command(
    name = "yapctl",
    version,
    about = "Send hotkey edges to the per-user Yap daemon"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Begin an action. Hyprland should call this on the key-down edge.
    Press { action: ActionArgument },
    /// End an action. Hyprland should call this on the key-up edge.
    Release { action: ActionArgument },
    /// Show the daemon's current session state and last runtime error.
    Status,
    /// Abort an active recording before it reaches transcription.
    Cancel,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ActionArgument {
    Dictation,
    Command,
}

impl From<ActionArgument> for Action {
    fn from(value: ActionArgument) -> Self {
        match value {
            ActionArgument::Dictation => Self::Dictation,
            ActionArgument::Command => Self::Command,
        }
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    let client = match Client::connect().await {
        Ok(client) => client,
        Err(error) => return fail(&format!("could not connect to the session bus: {error}")),
    };

    let result = match cli.command {
        Command::Press { action } => client
            .edge(action.into(), true)
            .await
            .map(|phase| print_phase(&phase)),
        Command::Release { action } => client
            .edge(action.into(), false)
            .await
            .map(|phase| print_phase(&phase)),
        Command::Cancel => client.cancel().await.map(|phase| print_phase(&phase)),
        Command::Status => client.status().await.map(|(phase, last_error)| {
            println!("phase: {phase}");
            if !last_error.is_empty() {
                println!("last error: {last_error}");
            }
        }),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => fail(&format!(
            "the Yap daemon is unavailable or rejected the command: {error}"
        )),
    }
}

fn print_phase(phase: &str) {
    println!("{phase}");
}

fn fail(message: &str) -> ExitCode {
    eprintln!("yapctl: {message}");
    ExitCode::FAILURE
}
