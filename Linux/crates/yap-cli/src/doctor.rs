use std::{env, path::Path, process::Command};

#[cfg(test)]
use std::collections::HashMap;

use serde::Serialize;
use yap_daemon::model;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Pass,
    Warning,
    SetupRequired,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Compatibility {
    Ready,
    Degraded,
    SetupRequired,
    Blocked,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Check {
    pub id: &'static str,
    pub status: CheckStatus,
    pub summary: &'static str,
    pub detail: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Report {
    pub compatibility: Compatibility,
    pub checks: Vec<Check>,
}

impl Report {
    pub fn print_human(&self) {
        println!("Yap Linux compatibility: {:?}\n", self.compatibility);
        for check in &self.checks {
            println!("[{:?}] {}: {}", check.status, check.summary, check.detail);
        }
        println!("\nThis check records no audio, text, or network traffic.");
    }
}

pub(crate) trait System {
    fn architecture(&self) -> String;
    fn environment(&self, name: &str) -> Option<String>;
    fn command_exists(&self, name: &str) -> bool;
    fn run(&self, name: &str, args: &[&str]) -> Result<String, String>;
    fn path_exists(&self, path: &Path) -> bool;
}

pub struct RealSystem;

impl System for RealSystem {
    fn architecture(&self) -> String {
        env::consts::ARCH.to_owned()
    }

    fn environment(&self, name: &str) -> Option<String> {
        env::var(name).ok().filter(|value| !value.is_empty())
    }

    fn command_exists(&self, name: &str) -> bool {
        let Some(path) = env::var_os("PATH") else {
            return false;
        };
        env::split_paths(&path).any(|directory| directory.join(name).is_file())
    }

    fn run(&self, name: &str, args: &[&str]) -> Result<String, String> {
        let output = Command::new(name)
            .args(args)
            .output()
            .map_err(|error| error.to_string())?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        if output.status.success() {
            Ok(if stdout.is_empty() { stderr } else { stdout })
        } else {
            Err(if stderr.is_empty() { stdout } else { stderr })
        }
    }

    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }
}

pub struct Doctor<S> {
    system: S,
}

impl<S: System> Doctor<S> {
    pub fn new(system: S) -> Self {
        Self { system }
    }

    pub fn run(&self) -> Report {
        let checks = vec![
            self.architecture(),
            self.session(),
            self.hyprland(),
            self.global_shortcuts(),
            self.pipewire(),
            self.playback_control(),
            self.transcriber(),
            self.language_runtime(),
            self.insertion(),
            self.acceleration(),
            self.speech_model(),
            self.cleanup_model(),
        ];
        let compatibility = compatibility(&checks);
        Report {
            compatibility,
            checks,
        }
    }

    fn architecture(&self) -> Check {
        let architecture = self.system.architecture();
        if architecture == "x86_64" {
            check(
                "architecture",
                CheckStatus::Pass,
                "CPU architecture",
                architecture,
            )
        } else {
            check(
                "architecture",
                CheckStatus::Blocked,
                "CPU architecture",
                format!("{architecture} is not supported by the current Linux build"),
            )
        }
    }

    fn session(&self) -> Check {
        let session = self
            .system
            .environment("XDG_SESSION_TYPE")
            .unwrap_or_else(|| "unknown".to_owned());
        if session == "wayland" {
            check(
                "session",
                CheckStatus::Pass,
                "Desktop session",
                "Wayland is active".to_owned(),
            )
        } else {
            check(
                "session",
                CheckStatus::Blocked,
                "Desktop session",
                format!("expected Wayland for the first release, found {session}"),
            )
        }
    }

    fn hyprland(&self) -> Check {
        if self
            .system
            .environment("HYPRLAND_INSTANCE_SIGNATURE")
            .is_none()
        {
            return check(
                "hyprland",
                CheckStatus::Blocked,
                "Hyprland adapter",
                "no active Hyprland instance was detected".to_owned(),
            );
        }
        match self.system.run("hyprctl", &["version"]) {
            Ok(version) => check(
                "hyprland",
                CheckStatus::Pass,
                "Hyprland adapter",
                version.lines().next().unwrap_or("detected").to_owned(),
            ),
            Err(error) => check(
                "hyprland",
                CheckStatus::Blocked,
                "Hyprland adapter",
                format!("hyprctl could not reach the compositor: {error}"),
            ),
        }
    }

    fn global_shortcuts(&self) -> Check {
        let result = self.system.run(
            "busctl",
            &[
                "--user",
                "introspect",
                "org.freedesktop.portal.Desktop",
                "/org/freedesktop/portal/desktop",
                "org.freedesktop.portal.GlobalShortcuts",
            ],
        );
        if result.is_ok() {
            check(
                "global_shortcuts",
                CheckStatus::Pass,
                "Global shortcuts",
                "portal interface is available".to_owned(),
            )
        } else {
            check(
                "global_shortcuts",
                CheckStatus::Warning,
                "Global shortcuts",
                "portal unavailable; user-configured Hyprland bindings remain usable".to_owned(),
            )
        }
    }

    fn pipewire(&self) -> Check {
        match self.system.run("pw-cli", &["info", "0"]) {
            Ok(_) => check(
                "pipewire",
                CheckStatus::Pass,
                "Microphone runtime",
                "PipeWire is reachable".to_owned(),
            ),
            Err(error) => check(
                "pipewire",
                CheckStatus::Blocked,
                "Microphone runtime",
                format!("PipeWire is not reachable: {error}"),
            ),
        }
    }

    fn playback_control(&self) -> Check {
        if self.system.command_exists("wpctl") && self.system.command_exists("pw-dump") {
            check(
                "playback_control",
                CheckStatus::Pass,
                "Playback control",
                "WirePlumber volume control and playback detection are installed".to_owned(),
            )
        } else {
            check(
                "playback_control",
                CheckStatus::Blocked,
                "Playback control",
                "wpctl or pw-dump is missing; reinstall the Yap package dependencies".to_owned(),
            )
        }
    }

    fn transcriber(&self) -> Check {
        if self.system.command_exists("whisper-server") {
            check(
                "transcriber",
                CheckStatus::Pass,
                "Transcription runtime",
                "whisper.cpp server is available on PATH".to_owned(),
            )
        } else {
            check(
                "transcriber",
                CheckStatus::Blocked,
                "Transcription runtime",
                "whisper-server is missing; reinstall the Yap package dependencies".to_owned(),
            )
        }
    }

    fn insertion(&self) -> Check {
        let wtype = self.system.command_exists("wtype");
        let clipboard =
            self.system.command_exists("wl-copy") && self.system.command_exists("wl-paste");
        if wtype && clipboard {
            check(
                "insertion",
                CheckStatus::Pass,
                "Text insertion",
                "virtual keyboard and clipboard fallback are installed".to_owned(),
            )
        } else if wtype {
            check(
                "insertion",
                CheckStatus::Warning,
                "Text insertion",
                "virtual keyboard is installed; clipboard fallback is unavailable".to_owned(),
            )
        } else {
            check(
                "insertion",
                CheckStatus::Blocked,
                "Text insertion",
                "required insertion adapters are missing; reinstall the Yap package dependencies"
                    .to_owned(),
            )
        }
    }

    fn language_runtime(&self) -> Check {
        if self.system.command_exists("llama-server") {
            check(
                "language_runtime",
                CheckStatus::Pass,
                "Language runtime",
                "llama.cpp server is installed for local cleanup and Command Mode".to_owned(),
            )
        } else {
            check(
                "language_runtime",
                CheckStatus::Blocked,
                "Language runtime",
                "llama-server is missing; reinstall the Yap package dependencies".to_owned(),
            )
        }
    }

    fn acceleration(&self) -> Check {
        match self.system.run(
            "nvidia-smi",
            &[
                "--query-gpu=name,driver_version,memory.total",
                "--format=csv,noheader",
            ],
        ) {
            Ok(gpu) => check(
                "acceleration",
                CheckStatus::Warning,
                "Inference acceleration",
                format!(
                    "{}; GPU acceleration depends on the packaged whisper.cpp backend",
                    gpu.lines().next().unwrap_or("NVIDIA GPU detected")
                ),
            ),
            Err(_) => check(
                "acceleration",
                CheckStatus::Warning,
                "Inference acceleration",
                "no NVIDIA runtime detected; Yap will use its CPU fallback".to_owned(),
            ),
        }
    }

    fn speech_model(&self) -> Check {
        let data_home = self.system.environment("XDG_DATA_HOME").unwrap_or_else(|| {
            self.system.environment("HOME").map_or_else(
                || ".local/share".to_owned(),
                |home| format!("{home}/.local/share"),
            )
        });
        let model = Path::new(&data_home)
            .join("yap/models/large-v3-turbo-q5_0")
            .join("394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2")
            .join("ggml-large-v3-turbo-q5_0.bin");
        if self.system.path_exists(&model) {
            check(
                "model",
                CheckStatus::Pass,
                "Transcription model",
                format!("found {}", model.display()),
            )
        } else {
            check(
                "model",
                CheckStatus::SetupRequired,
                "Transcription model",
                "run `yap model install` to download and verify the pinned model".to_owned(),
            )
        }
    }

    fn cleanup_model(&self) -> Check {
        let data_home = self.system.environment("XDG_DATA_HOME").unwrap_or_else(|| {
            self.system.environment("HOME").map_or_else(
                || ".local/share".to_owned(),
                |home| format!("{home}/.local/share"),
            )
        });
        let model = Path::new(&data_home)
            .join("yap/models")
            .join(model::CLEANUP_MODEL_NAME)
            .join(model::CLEANUP_MODEL_SHA256)
            .join(model::CLEANUP_MODEL_FILE_NAME);
        if self.system.path_exists(&model) {
            check(
                "cleanup_model",
                CheckStatus::Pass,
                "Language model",
                format!("found {}", model.display()),
            )
        } else {
            check(
                "cleanup_model",
                CheckStatus::SetupRequired,
                "Language model",
                "run `yap model install` to download and verify local cleanup and Command Mode"
                    .to_owned(),
            )
        }
    }
}

fn check(id: &'static str, status: CheckStatus, summary: &'static str, detail: String) -> Check {
    Check {
        id,
        status,
        summary,
        detail,
    }
}

fn compatibility(checks: &[Check]) -> Compatibility {
    if checks
        .iter()
        .any(|check| check.status == CheckStatus::Blocked)
    {
        Compatibility::Blocked
    } else if checks
        .iter()
        .any(|check| check.status == CheckStatus::SetupRequired)
    {
        Compatibility::SetupRequired
    } else if checks
        .iter()
        .any(|check| check.status == CheckStatus::Warning)
    {
        Compatibility::Degraded
    } else {
        Compatibility::Ready
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeSystem {
        architecture: String,
        environment: HashMap<String, String>,
        commands: HashMap<String, Result<String, String>>,
        paths: Vec<String>,
    }

    impl FakeSystem {
        fn compatible() -> Self {
            Self {
                architecture: "x86_64".to_owned(),
                environment: HashMap::from([
                    ("XDG_SESSION_TYPE".to_owned(), "wayland".to_owned()),
                    ("HYPRLAND_INSTANCE_SIGNATURE".to_owned(), "test".to_owned()),
                    ("HOME".to_owned(), "/home/test".to_owned()),
                ]),
                commands: HashMap::from([
                    ("hyprctl".to_owned(), Ok("Hyprland 0.56.2".to_owned())),
                    ("busctl".to_owned(), Ok("GlobalShortcuts".to_owned())),
                    ("pw-cli".to_owned(), Ok("PipeWire core".to_owned())),
                    ("pw-dump".to_owned(), Ok(String::new())),
                    ("wpctl".to_owned(), Ok(String::new())),
                    ("whisper-server".to_owned(), Ok(String::new())),
                    ("llama-server".to_owned(), Ok(String::new())),
                    ("wtype".to_owned(), Ok(String::new())),
                    ("wl-copy".to_owned(), Ok(String::new())),
                    ("wl-paste".to_owned(), Ok(String::new())),
                    ("nvidia-smi".to_owned(), Ok("RTX 3080".to_owned())),
                ]),
                paths: Vec::new(),
            }
        }
    }

    impl System for FakeSystem {
        fn architecture(&self) -> String {
            self.architecture.clone()
        }

        fn environment(&self, name: &str) -> Option<String> {
            self.environment.get(name).cloned()
        }

        fn command_exists(&self, name: &str) -> bool {
            self.commands.contains_key(name)
        }

        fn run(&self, name: &str, args: &[&str]) -> Result<String, String> {
            let invocation = format!("{name} {}", args.join(" "));
            self.commands
                .get(&invocation)
                .or_else(|| self.commands.get(name))
                .cloned()
                .unwrap_or_else(|| Err(format!("{name} missing")))
        }

        fn path_exists(&self, path: &Path) -> bool {
            self.paths
                .iter()
                .any(|candidate| candidate == &path.to_string_lossy())
        }
    }

    #[test]
    fn compatible_machine_needing_only_a_model_is_setup_required() {
        let report = Doctor::new(FakeSystem::compatible()).run();
        assert_eq!(report.compatibility, Compatibility::SetupRequired);
        assert!(report.checks.iter().all(|check| {
            matches!(
                check.status,
                CheckStatus::Pass | CheckStatus::Warning | CheckStatus::SetupRequired
            )
        }));
    }

    #[test]
    fn missing_pipewire_blocks_yap() {
        let mut system = FakeSystem::compatible();
        system
            .commands
            .insert("pw-cli".to_owned(), Err("connection refused".to_owned()));
        let report = Doctor::new(system).run();
        assert_eq!(report.compatibility, Compatibility::Blocked);
    }

    #[test]
    fn missing_whisper_server_blocks_transcription() {
        let mut system = FakeSystem::compatible();
        system.commands.remove("whisper-server");

        let report = Doctor::new(system).run();
        let transcriber = report
            .checks
            .iter()
            .find(|check| check.id == "transcriber")
            .unwrap();

        assert_eq!(report.compatibility, Compatibility::Blocked);
        assert_eq!(transcriber.status, CheckStatus::Blocked);
    }

    #[test]
    fn health_checks_do_not_require_an_arch_package_manager() {
        let report = Doctor::new(FakeSystem::compatible()).run();
        let transcriber = report
            .checks
            .iter()
            .find(|check| check.id == "transcriber")
            .unwrap();

        assert_eq!(transcriber.status, CheckStatus::Pass);
        assert!(!transcriber.detail.contains("pacman"));
        assert!(!transcriber.detail.contains("ggml-cpu"));
    }

    #[test]
    fn cpu_fallback_is_degraded_but_not_blocked() {
        let mut system = FakeSystem::compatible();
        system.commands.remove("nvidia-smi");
        system.paths.push(
            "/home/test/.local/share/yap/models/large-v3-turbo-q5_0/394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2/ggml-large-v3-turbo-q5_0.bin"
                .to_owned(),
        );
        system.paths.push(
            format!(
                "/home/test/.local/share/yap/models/{}/{}/{}",
                model::CLEANUP_MODEL_NAME,
                model::CLEANUP_MODEL_SHA256,
                model::CLEANUP_MODEL_FILE_NAME
            ),
        );
        let report = Doctor::new(system).run();
        assert_eq!(report.compatibility, Compatibility::Degraded);
    }

    #[test]
    fn report_json_contains_no_captured_user_content() {
        let report = Doctor::new(FakeSystem::compatible()).run();
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("setup_required"));
        assert!(!json.contains("transcript"));
        assert!(!json.contains("audio"));
    }
}
