mod doctor;
mod setup;

use std::process::ExitCode;

use clap::{Parser, Subcommand};
use doctor::{Compatibility, Doctor, RealSystem};
use yap_daemon::model::{self, InstallOutcome};

#[derive(Debug, Parser)]
#[command(name = "yap", version, about = "Local-first voice dictation")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Check whether this Linux session can run Yap and which capabilities will degrade.
    Doctor {
        /// Emit a stable JSON report for bug reports and package verification.
        #[arg(long)]
        json: bool,
    },
    /// Manage the pinned local speech model.
    Model {
        #[command(subcommand)]
        command: ModelCommand,
    },
    /// Configure a supported Linux desktop and start the per-user daemon.
    Setup {
        #[command(subcommand)]
        command: SetupCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ModelCommand {
    /// Download and verify the 547 MiB large-v3-turbo Q5 model.
    Install,
}

#[derive(Debug, Subcommand)]
enum SetupCommand {
    /// Start Yap and show the commands for user-owned Hyprland bindings.
    Hyprland,
}

#[tokio::main]
async fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Doctor { json } => {
            let report = Doctor::new(RealSystem).run();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&report)
                        .expect("diagnostic report is serializable")
                );
            } else {
                report.print_human();
            }
            if report.compatibility == Compatibility::Blocked {
                ExitCode::from(2)
            } else {
                ExitCode::SUCCESS
            }
        }
        Command::Model {
            command: ModelCommand::Install,
        } => {
            println!(
                "Downloading Yap's pinned 547 MiB model from Hugging Face; speech stays local after setup."
            );
            match model::install().await {
                Ok(InstallOutcome::AlreadyPresent) => {
                    println!("Model is already installed and verified.");
                    ExitCode::SUCCESS
                }
                Ok(InstallOutcome::Installed) => {
                    println!("Model installed and SHA-256 verified.");
                    ExitCode::SUCCESS
                }
                Err(error) => {
                    eprintln!("yap: model installation failed: {error}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Setup {
            command: SetupCommand::Hyprland,
        } => match setup::hyprland().await {
            Ok(outcome) => {
                println!("Yap's daemon and visual indicator are enabled and running.");
                if let Some(backup) = outcome.main_backup {
                    println!(
                        "Removed the legacy Right-Super include. Backup: {}",
                        backup.display()
                    );
                }
                if let Some(config) = outcome.removed_legacy_config {
                    println!("Removed legacy generated binding: {}", config.display());
                }
                if let Some(config) = outcome.preserved_config {
                    println!("Left user-owned config unchanged: {}", config.display());
                }
                println!("Yap leaves the hotkey choice to your Hyprland configuration.");
                println!("Press edge:   yapctl press dictation");
                println!("Release edge: yapctl release dictation");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("yap: Hyprland setup failed: {error}");
                ExitCode::FAILURE
            }
        },
    }
}
