//! Reversible control of other desktop audio while Yap records.

use std::{process::Stdio, time::Duration};

use serde_json::Value;
use tokio::process::Command;

use crate::store::{AudioMode, Settings};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_SINK: &str = "@DEFAULT_AUDIO_SINK@";
const PAUSE_SETTLE_DELAY: Duration = Duration::from_millis(200);

#[async_trait::async_trait]
trait AudioBackend: Sync {
    async fn run(&self, program: &str, arguments: &[&str]) -> Result<String, String>;

    async fn sleep(&self, duration: Duration) {
        tokio::time::sleep(duration).await;
    }
}

struct SystemAudioBackend;

#[async_trait::async_trait]
impl AudioBackend for SystemAudioBackend {
    async fn run(&self, program: &str, arguments: &[&str]) -> Result<String, String> {
        run_command(program, arguments).await
    }
}

const SYSTEM_AUDIO: SystemAudioBackend = SystemAudioBackend;

#[derive(Debug)]
enum AudioAction {
    None,
    Lowered {
        original_volume: f64,
    },
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
            AudioMode::Lower => lower_if_playing(&SYSTEM_AUDIO, settings.duck_level).await,
            AudioMode::Pause => pause_players(&SYSTEM_AUDIO).await,
        };
        Self { action }
    }

    /// Restores exactly the volume and players changed by [`Self::begin`].
    pub async fn restore(self) {
        match self.action {
            AudioAction::None => {}
            AudioAction::Lowered { original_volume } => {
                let from = get_volume(&SYSTEM_AUDIO).await.unwrap_or(original_volume);
                if let Err(error) = ramp_volume(&SYSTEM_AUDIO, from, original_volume, 7).await {
                    eprintln!("yapd: could not restore output volume: {error}");
                }
            }
            AudioAction::Paused {
                players,
                original_volume,
            } => {
                if original_volume.is_some() {
                    let _ = set_volume(&SYSTEM_AUDIO, 0.0).await;
                }
                for player in players {
                    if let Err(error) = call_player(&SYSTEM_AUDIO, &player, "Play").await {
                        eprintln!("yapd: could not resume a player paused by Yap: {error}");
                    }
                }
                if let Some(original_volume) = original_volume {
                    if let Err(error) = ramp_volume(&SYSTEM_AUDIO, 0.0, original_volume, 9).await {
                        eprintln!("yapd: could not restore output volume after pause: {error}");
                    }
                }
            }
        }
    }
}

async fn lower_if_playing(backend: &impl AudioBackend, target: f64) -> AudioAction {
    if !pipewire_has_playback(backend).await && playing_players(backend).await.is_empty() {
        return AudioAction::None;
    }
    let Ok(original_volume) = get_volume(backend).await else {
        return AudioAction::None;
    };
    let lowered = original_volume.min(target.clamp(0.0, 1.0));
    if ramp_volume(backend, original_volume, lowered, 6)
        .await
        .is_err()
    {
        let _ = set_volume(backend, original_volume).await;
        AudioAction::None
    } else {
        AudioAction::Lowered { original_volume }
    }
}

async fn pause_players(backend: &impl AudioBackend) -> AudioAction {
    let players = playing_players(backend).await;
    if players.is_empty() {
        return AudioAction::None;
    }
    let original_volume = get_volume(backend).await.ok();
    if let Some(volume) = original_volume {
        let _ = ramp_volume(backend, volume, 0.0, 7).await;
    }

    let mut paused = Vec::new();
    for player in players {
        if call_player(backend, &player, "Pause").await.is_ok() {
            paused.push(player);
        }
    }
    if let Some(volume) = original_volume {
        // MPRIS method completion does not mean the player's queued PipeWire frames have drained.
        // Keep the sink silent long enough for those frames to clear before restoring its slider,
        // then ramp the hardware-facing sink back up instead of making one large volume jump.
        if !paused.is_empty() {
            backend.sleep(PAUSE_SETTLE_DELAY).await;
        }
        if ramp_volume(backend, 0.0, volume, 7).await.is_err() {
            let _ = set_volume(backend, volume).await;
        }
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

async fn playing_players(backend: &impl AudioBackend) -> Vec<String> {
    let Ok(output) = backend
        .run("busctl", &["--user", "--list", "--no-pager", "--no-legend"])
        .await
    else {
        return Vec::new();
    };
    let mut playing = Vec::new();
    for player in mpris_names(&output) {
        let Ok(status) = backend
            .run(
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

async fn call_player(
    backend: &impl AudioBackend,
    player: &str,
    method: &str,
) -> Result<(), String> {
    backend
        .run(
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

async fn pipewire_has_playback(backend: &impl AudioBackend) -> bool {
    let Ok(output) = backend.run("pw-dump", &[]).await else {
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

async fn get_volume(backend: &impl AudioBackend) -> Result<f64, String> {
    let output = backend.run("wpctl", &["get-volume", DEFAULT_SINK]).await?;
    parse_volume(&output).ok_or_else(|| "wpctl returned an unrecognized volume".to_owned())
}

async fn set_volume(backend: &impl AudioBackend, volume: f64) -> Result<(), String> {
    let volume = format!("{:.4}", volume.max(0.0));
    backend
        .run("wpctl", &["set-volume", DEFAULT_SINK, &volume])
        .await
        .map(|_| ())
}

async fn ramp_volume(
    backend: &impl AudioBackend,
    from: f64,
    to: f64,
    steps: u32,
) -> Result<(), String> {
    for step in 1..=steps.max(1) {
        let fraction = f64::from(step) / f64::from(steps.max(1));
        set_volume(backend, from + (to - from) * fraction).await?;
        backend.sleep(Duration::from_millis(18)).await;
    }
    Ok(())
}

async fn run_command(program: &str, arguments: &[&str]) -> Result<String, String> {
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
    use std::sync::Mutex;

    use super::*;

    #[derive(Debug, Clone, Copy)]
    enum BackendEvent {
        Pause { at: Duration },
        Volume { at: Duration, value: f64 },
    }

    #[derive(Debug, Default)]
    struct FakeAudioBackend {
        elapsed: Mutex<Duration>,
        events: Mutex<Vec<BackendEvent>>,
    }

    impl FakeAudioBackend {
        fn now(&self) -> Duration {
            *self.elapsed.lock().expect("fake clock lock")
        }

        fn events(&self) -> Vec<BackendEvent> {
            self.events.lock().expect("fake event lock").clone()
        }
    }

    #[async_trait::async_trait]
    impl AudioBackend for FakeAudioBackend {
        async fn run(&self, program: &str, arguments: &[&str]) -> Result<String, String> {
            match (program, arguments.get(1).copied()) {
                ("busctl", Some("--list")) => {
                    Ok("org.mpris.MediaPlayer2.spotify 123 user\n".to_owned())
                }
                ("busctl", Some("get-property")) => Ok("s \"Playing\"\n".to_owned()),
                ("busctl", Some("call")) => {
                    if arguments.last() == Some(&"Pause") {
                        self.events
                            .lock()
                            .expect("fake event lock")
                            .push(BackendEvent::Pause { at: self.now() });
                    }
                    Ok(String::new())
                }
                ("wpctl", _) if arguments.first() == Some(&"get-volume") => {
                    Ok("Volume: 0.8000\n".to_owned())
                }
                ("wpctl", _) if arguments.first() == Some(&"set-volume") => {
                    let value = arguments
                        .last()
                        .expect("set-volume value")
                        .parse()
                        .expect("numeric set-volume value");
                    self.events
                        .lock()
                        .expect("fake event lock")
                        .push(BackendEvent::Volume {
                            at: self.now(),
                            value,
                        });
                    Ok(String::new())
                }
                ("pw-dump", _) => Ok("[]".to_owned()),
                _ => Err(format!("unexpected command: {program} {arguments:?}")),
            }
        }

        async fn sleep(&self, duration: Duration) {
            *self.elapsed.lock().expect("fake clock lock") += duration;
        }
    }

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

    #[tokio::test]
    async fn pause_mode_keeps_sink_silent_until_player_settles() {
        let backend = FakeAudioBackend::default();

        let action = pause_players(&backend).await;

        assert!(matches!(action, AudioAction::Paused { .. }));
        let events = backend.events();
        let pause_at = events
            .iter()
            .find_map(|event| match event {
                BackendEvent::Pause { at } => Some(*at),
                BackendEvent::Volume { .. } => None,
            })
            .expect("the playing MPRIS client should be paused");
        let fade: Vec<f64> = events
            .iter()
            .filter_map(|event| match event {
                BackendEvent::Volume { at, value } if *at <= pause_at => Some(*value),
                BackendEvent::Pause { .. } | BackendEvent::Volume { .. } => None,
            })
            .collect();
        let restored: Vec<(Duration, f64)> = events
            .iter()
            .filter_map(|event| match event {
                BackendEvent::Volume { at, value } if *at >= pause_at && *value > 0.0 => {
                    Some((*at, *value))
                }
                BackendEvent::Pause { .. } | BackendEvent::Volume { .. } => None,
            })
            .collect();
        let restored_at = restored
            .first()
            .map(|(at, _)| *at)
            .expect("the original sink volume should be restored");
        let restored_volume = restored.last().expect("a final restored volume").1;

        assert!(
            fade.windows(2).all(|levels| levels[0] > levels[1]),
            "the sink should fade down monotonically before MPRIS Pause"
        );
        assert!(
            fade.last()
                .is_some_and(|volume| volume.abs() < f64::EPSILON)
        );
        let mut restore_levels = vec![0.0];
        restore_levels.extend(restored.iter().map(|(_, volume)| *volume));
        assert!(
            restore_levels
                .windows(2)
                .all(|levels| levels[1] - levels[0] <= 0.2),
            "pause mode must not jump directly from silence to the original sink volume"
        );
        assert!((restored_volume - 0.8).abs() < f64::EPSILON);
        assert!(
            restored_at.saturating_sub(pause_at) >= Duration::from_millis(200),
            "restoring the sink immediately after MPRIS Pause leaks buffered audio"
        );
    }
}
