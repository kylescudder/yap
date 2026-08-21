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
    /// Open Yap's Linux desktop dashboard.
    Gui,
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
    /// Download and verify the pinned speech and language models.
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
        Command::Gui => match tokio::process::Command::new("yap-ui").spawn() {
            Ok(_) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("yap: could not open the desktop dashboard: {error}");
                ExitCode::FAILURE
            }
        },
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
                "Installing Yap's pinned local models: 547 MiB for speech and 2.33 GiB for cleanup and Command Mode."
            );
            let speech = model::install().await;
            match &speech {
                Ok(InstallOutcome::AlreadyPresent) => println!("Speech model is already verified."),
                Ok(InstallOutcome::Installed) => println!("Speech model installed and verified."),
                Err(error) => eprintln!("yap: speech model installation failed: {error}"),
            }
            if speech.is_err() {
                ExitCode::FAILURE
            } else {
                match model::install_cleanup().await {
                    Ok(InstallOutcome::AlreadyPresent) => {
                        println!("Language model is already verified.");
                        ExitCode::SUCCESS
                    }
                    Ok(InstallOutcome::Installed) => {
                        println!("Language model installed and verified.");
                        ExitCode::SUCCESS
                    }
                    Err(error) => {
                        eprintln!("yap: language model installation failed: {error}");
                        ExitCode::FAILURE
                    }
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
                println!("Dashboard:    yap gui");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("yap: Hyprland setup failed: {error}");
                ExitCode::FAILURE
            }
        },
    }
}
