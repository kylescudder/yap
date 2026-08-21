//! Reversible control of other desktop audio while Yap records.

use std::{process::Stdio, time::Duration};

use serde_json::Value;
use tokio::process::Command;

use crate::store::{AudioMode, Settings};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_SINK: &str = "@DEFAULT_AUDIO_SINK@";

#[derive(Debug)]
enum AudioAction {
    None,
    Lowered { original_volume: f64 },
    Paused {
        players: Vec<String>,
        original_volume: Option<f64>,
    },
}

/// One reversible audio action owned by one capture.
#[derive(Debug)]
pub struct AudioSession {
    action: AudioAction,
}

impl AudioSession {
    /// Samples playback and applies the configured policy before the microphone starts.
    pub async fn begin(settings: &Settings) -> Self {
        let action = match settings.audio_mode {
            AudioMode::Off => AudioAction::None,
            AudioMode::Lower => lower_if_playing(settings.duck_level).await,
            AudioMode::Pause => pause_players().await,
        };
        Self { action }
    }

    /// Restores exactly the volume and players changed by [`Self::begin`].
    pub async fn restore(self) {
        match self.action {
            AudioAction::None => {}
            AudioAction::Lowered { original_volume } => {
                let from = get_volume().await.unwrap_or(original_volume);
                if let Err(error) = ramp_volume(from, original_volume, 7).await {
                    eprintln!("yapd: could not restore output volume: {error}");
                }
            }
            AudioAction::Paused {
                players,
                original_volume,
            } => {
                if original_volume.is_some() {
                    let _ = set_volume(0.0).await;
                }
                for player in players {
                    if let Err(error) = call_player(&player, "Play").await {
                        eprintln!("yapd: could not resume a player paused by Yap: {error}");
                    }
                }
                if let Some(original_volume) = original_volume {
                    if let Err(error) = ramp_volume(0.0, original_volume, 9).await {
                        eprintln!("yapd: could not restore output volume after pause: {error}");
                    }
                }
            }
        }
    }
}

async fn lower_if_playing(target: f64) -> AudioAction {
    if !pipewire_has_playback().await && playing_players().await.is_empty() {
        return AudioAction::None;
    }
    let Ok(original_volume) = get_volume().await else {
        return AudioAction::None;
    };
    let lowered = original_volume.min(target.clamp(0.0, 1.0));
    if ramp_volume(original_volume, lowered, 6).await.is_err() {
        let _ = set_volume(original_volume).await;
        AudioAction::None
    } else {
        AudioAction::Lowered { original_volume }
    }
}

async fn pause_players() -> AudioAction {
    let players = playing_players().await;
    if players.is_empty() {
        return AudioAction::None;
    }
    let original_volume = get_volume().await.ok();
    if let Some(volume) = original_volume {
        let _ = ramp_volume(volume, 0.0, 7).await;
    }

    let mut paused = Vec::new();
    for player in players {
        if call_player(&player, "Pause").await.is_ok() {
            paused.push(player);
        }
    }
    if let Some(volume) = original_volume {
        let _ = set_volume(volume).await;
    }
    if paused.is_empty() {
        AudioAction::None
    } else {
        AudioAction::Paused {
            players: paused,
            original_volume,
        }
    }
}

async fn playing_players() -> Vec<String> {
    let Ok(output) = run("busctl", &["--user", "--list", "--no-pager", "--no-legend"]).await
    else {
        return Vec::new();
    };
    let mut playing = Vec::new();
    for player in mpris_names(&output) {
        let Ok(status) = run(
            "busctl",
            &[
                "--user",
                "get-property",
                &player,
                "/org/mpris/MediaPlayer2",
                "org.mpris.MediaPlayer2.Player",
                "PlaybackStatus",
            ],
        )
        .await
        else {
            continue;
        };
        if playback_status(&status) == Some("Playing") {
            playing.push(player);
        }
    }
    playing
}

async fn call_player(player: &str, method: &str) -> Result<(), String> {
    run(
        "busctl",
        &[
            "--user",
            "call",
            player,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
            method,
        ],
    )
    .await
    .map(|_| ())
}

async fn pipewire_has_playback() -> bool {
    let Ok(output) = run("pw-dump", &[]).await else {
        return false;
    };
    serde_json::from_str::<Value>(&output)
        .ok()
        .and_then(|value| value.as_array().cloned())
        .is_some_and(|objects| objects.iter().any(is_running_audio_output))
}

fn is_running_audio_output(object: &Value) -> bool {
    object.pointer("/info/state").and_then(Value::as_str) == Some("running")
        && object
            .pointer("/info/props/media.class")
            .and_then(Value::as_str)
            .is_some_and(|class| class == "Stream/Output/Audio")
}

async fn get_volume() -> Result<f64, String> {
    let output = run("wpctl", &["get-volume", DEFAULT_SINK]).await?;
    parse_volume(&output).ok_or_else(|| "wpctl returned an unrecognized volume".to_owned())
}

async fn set_volume(volume: f64) -> Result<(), String> {
    let volume = format!("{:.4}", volume.max(0.0));
    run("wpctl", &["set-volume", DEFAULT_SINK, &volume])
        .await
        .map(|_| ())
}

async fn ramp_volume(from: f64, to: f64, steps: u32) -> Result<(), String> {
    for step in 1..=steps.max(1) {
        let fraction = f64::from(step) / f64::from(steps.max(1));
        set_volume(from + (to - from) * fraction).await?;
        tokio::time::sleep(Duration::from_millis(18)).await;
    }
    Ok(())
}

async fn run(program: &str, arguments: &[&str]) -> Result<String, String> {
    let output = tokio::time::timeout(
        COMMAND_TIMEOUT,
        Command::new(program)
            .args(arguments)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output(),
    )
    .await
    .map_err(|_| format!("{program} timed out"))?
    .map_err(|error| format!("could not start {program}: {error}"))?;
    if !output.status.success() {
        return Err(format!("{program} failed with {}", output.status));
    }
    String::from_utf8(output.stdout).map_err(|_| format!("{program} returned invalid UTF-8"))
}

fn parse_volume(output: &str) -> Option<f64> {
    let (_, value) = output.split_once(':')?;
    value.split_whitespace().next()?.parse().ok()
}

fn mpris_names(output: &str) -> Vec<String> {
    output
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|name| name.starts_with("org.mpris.MediaPlayer2."))
        .map(str::to_owned)
        .collect()
}

fn playback_status(output: &str) -> Option<&str> {
    output.split('"').nth(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wpctl_volume_parser_preserves_exact_scalar() {
        assert_eq!(parse_volume("Volume: 0.420000 [MUTED]\n"), Some(0.42));
        assert_eq!(parse_volume("unexpected"), None);
    }

    #[test]
    fn mpris_discovery_uses_only_well_known_player_names() {
        let names = mpris_names(
            "org.mpris.MediaPlayer2.spotify 123 user\n:1.42 999 user\norg.example.Other 4 user\n",
        );
        assert_eq!(names, vec!["org.mpris.MediaPlayer2.spotify".to_owned()]);
        assert_eq!(playback_status("s \"Playing\"\n"), Some("Playing"));
    }

    #[test]
    fn pipewire_stream_detection_requires_running_audio_output() {
        let running: Value = serde_json::json!({
            "info": {"state": "running", "props": {"media.class": "Stream/Output/Audio"}}
        });
        let idle: Value = serde_json::json!({
            "info": {"state": "idle", "props": {"media.class": "Stream/Output/Audio"}}
        });
        assert!(is_running_audio_output(&running));
        assert!(!is_running_audio_output(&idle));
    }
}
