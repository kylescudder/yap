//! Linux process adapters for capture, local transcription, and Wayland insertion.

use std::{
    fs::{File, OpenOptions, Permissions},
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use nix::{
    sys::signal::{Signal, kill},
    unistd::{Pid, Uid},
};
use serde::Deserialize;
use tokio::{
    io::AsyncWriteExt,
    process::{Child, Command},
    sync::Mutex,
};
use yap_core::Action;

use crate::{PipelineRuntime, RuntimeError, model};

const WHISPER_PORT: u16 = 19_401;
const SERVER_START_TIMEOUT: Duration = Duration::from_secs(150);

#[derive(Clone, Debug)]
pub struct RuntimePaths {
    pub runtime_dir: PathBuf,
    pub model: PathBuf,
}

impl RuntimePaths {
    /// Discovers private runtime storage and the content-addressed model path.
    ///
    /// # Errors
    ///
    /// Returns an error if the runtime directory cannot be created or secured for the current user.
    pub fn discover() -> Result<Self, RuntimeError> {
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(
            || std::env::temp_dir().join(format!("yap-{}", Uid::current().as_raw())),
            |base| PathBuf::from(base).join("yap"),
        );
        std::fs::create_dir_all(&runtime_dir).map_err(|error| {
            RuntimeError(format!(
                "could not create the private runtime directory: {error}"
            ))
        })?;
        std::fs::set_permissions(&runtime_dir, Permissions::from_mode(0o700)).map_err(|error| {
            RuntimeError(format!(
                "could not secure the private runtime directory: {error}"
            ))
        })?;
        Ok(Self {
            runtime_dir,
            model: model::default_path(),
        })
    }
}

#[derive(Debug)]
struct Capture {
    child: Child,
    path: PathBuf,
}

#[derive(Debug)]
struct WhisperState {
    child: Option<Child>,
}

#[derive(Debug)]
struct WhisperEngine {
    state: Mutex<WhisperState>,
    client: reqwest::Client,
    model: PathBuf,
    log_path: PathBuf,
    base_url: String,
}

impl WhisperEngine {
    fn new(paths: &RuntimePaths) -> Result<Self, RuntimeError> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .map_err(|error| RuntimeError(format!("could not create HTTP client: {error}")))?;
        Ok(Self {
            state: Mutex::new(WhisperState { child: None }),
            client,
            model: paths.model.clone(),
            log_path: paths.runtime_dir.join("whisper-server.log"),
            base_url: format!("http://127.0.0.1:{WHISPER_PORT}"),
        })
    }

    async fn warm(&self) -> Result<(), RuntimeError> {
        self.ensure_ready().await
    }

    async fn transcribe(&self, audio_path: &Path) -> Result<String, RuntimeError> {
        self.ensure_ready().await?;
        let audio = tokio::fs::read(audio_path)
            .await
            .map_err(|error| RuntimeError(format!("could not read captured audio: {error}")))?;
        let audio = reqwest::multipart::Part::bytes(audio)
            .file_name("dictation.wav")
            .mime_str("audio/wav")
            .map_err(|error| RuntimeError(format!("could not encode captured audio: {error}")))?;
        let form = reqwest::multipart::Form::new()
            .part("file", audio)
            .text("temperature", "0.0")
            .text("temperature_inc", "0.0")
            .text("response_format", "json");
        let response = self
            .client
            .post(format!("{}/inference", self.base_url))
            .multipart(form)
            .send()
            .await
            .map_err(|error| {
                RuntimeError(format!("local transcription request failed: {error}"))
            })?;
        let response = response.error_for_status().map_err(|error| {
            RuntimeError(format!(
                "local transcription server rejected audio: {error}"
            ))
        })?;
        let payload: InferenceResponse = response.json().await.map_err(|error| {
            RuntimeError(format!("local transcription response was invalid: {error}"))
        })?;
        Ok(payload.text.trim().to_owned())
    }

    async fn ensure_ready(&self) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().await;
        if let Some(child) = state.child.as_mut() {
            match child.try_wait() {
                Ok(None) if self.health_check().await => return Ok(()),
                Ok(None) => {
                    child.start_kill().map_err(|error| {
                        RuntimeError(format!("could not restart Whisper: {error}"))
                    })?;
                    let _ = child.wait().await;
                }
                Ok(Some(_)) => {}
                Err(error) => {
                    return Err(RuntimeError(format!(
                        "could not inspect the Whisper process: {error}"
                    )));
                }
            }
            state.child = None;
        }

        if !self.model.is_file() {
            return Err(RuntimeError(format!(
                "speech model is not installed; run `yap model install` (expected {})",
                self.model.display()
            )));
        }

        let log = secure_file(&self.log_path)?;
        let stderr = log
            .try_clone()
            .map_err(|error| RuntimeError(format!("could not open Whisper log: {error}")))?;
        let mut command = Command::new("whisper-server");
        command
            .arg("--model")
            .arg(&self.model)
            .args([
                "--language",
                "auto",
                "--host",
                "127.0.0.1",
                "--port",
                &WHISPER_PORT.to_string(),
                "--no-context",
                "--no-timestamps",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(stderr))
            .kill_on_drop(true);
        let child = command
            .spawn()
            .map_err(|error| RuntimeError(format!("could not start whisper-server: {error}")))?;
        state.child = Some(child);

        let started = tokio::time::Instant::now();
        while started.elapsed() < SERVER_START_TIMEOUT {
            let child = state
                .child
                .as_mut()
                .expect("Whisper child exists while it is starting");
            match child.try_wait() {
                Ok(Some(status)) => {
                    state.child = None;
                    return Err(RuntimeError(format!(
                        "whisper-server exited during model load with {status}; inspect {}",
                        self.log_path.display()
                    )));
                }
                Ok(None) => {}
                Err(error) => {
                    return Err(RuntimeError(format!(
                        "could not inspect whisper-server: {error}"
                    )));
                }
            }
            if self.health_check().await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }

        if let Some(child) = state.child.as_mut() {
            let _ = child.start_kill();
            let _ = child.wait().await;
        }
        state.child = None;
        Err(RuntimeError(format!(
            "whisper-server did not load the model within {} seconds; inspect {}",
            SERVER_START_TIMEOUT.as_secs(),
            self.log_path.display()
        )))
    }

    async fn health_check(&self) -> bool {
        self.client
            .get(format!("{}/", self.base_url))
            .timeout(Duration::from_secs(1))
            .send()
            .await
            .is_ok_and(|response| response.status().is_success())
    }
}

#[derive(Debug, Deserialize)]
struct InferenceResponse {
    text: String,
}

/// Production adapter for the first Linux dictation slice.
#[derive(Debug)]
pub struct LocalRuntime {
    capture: Mutex<Option<Capture>>,
    whisper: Arc<WhisperEngine>,
    paths: RuntimePaths,
}

impl LocalRuntime {
    /// Creates the process adapters without touching the microphone or loading a model.
    ///
    /// # Errors
    ///
    /// Returns an error when private runtime storage or the loopback HTTP client cannot be created.
    pub fn discover() -> Result<Arc<Self>, RuntimeError> {
        let paths = RuntimePaths::discover()?;
        let whisper = Arc::new(WhisperEngine::new(&paths)?);
        Ok(Arc::new(Self {
            capture: Mutex::new(None),
            whisper,
            paths,
        }))
    }

    /// Loads the local model before the first dictation.
    ///
    /// # Errors
    ///
    /// Returns an error if the model is absent or `whisper-server` cannot become ready.
    pub async fn warm(&self) -> Result<(), RuntimeError> {
        self.whisper.warm().await
    }

    async fn stop_capture(&self) -> Result<PathBuf, RuntimeError> {
        let capture = self
            .capture
            .lock()
            .await
            .take()
            .ok_or_else(|| RuntimeError("no microphone capture is active".to_owned()))?;
        let Capture { mut child, path } = capture;

        if let Some(process_id) = child.id() {
            let process_id = i32::try_from(process_id)
                .map_err(|_| RuntimeError("pw-record process identifier overflowed".to_owned()))?;
            if let Err(error) = kill(Pid::from_raw(process_id), Signal::SIGINT) {
                let _ = child.start_kill();
                return Err(RuntimeError(format!(
                    "could not stop microphone capture cleanly: {error}"
                )));
            }
        }
        if tokio::time::timeout(Duration::from_secs(3), child.wait())
            .await
            .is_err()
        {
            let _ = child.start_kill();
            let _ = child.wait().await;
            return Err(RuntimeError(
                "microphone capture did not stop within three seconds".to_owned(),
            ));
        }

        let metadata = tokio::fs::metadata(&path)
            .await
            .map_err(|error| RuntimeError(format!("captured audio is unavailable: {error}")))?;
        if metadata.len() <= 44 {
            return Err(RuntimeError(
                "microphone capture contained no usable audio".to_owned(),
            ));
        }
        Ok(path)
    }
}

#[async_trait]
impl PipelineRuntime for LocalRuntime {
    async fn start_capture(&self, action: Action) -> Result<(), RuntimeError> {
        if action != Action::Dictation {
            return Err(RuntimeError(
                "Command Mode is not included in the first Linux slice".to_owned(),
            ));
        }

        let mut capture = self.capture.lock().await;
        if capture.is_some() {
            return Err(RuntimeError(
                "microphone capture is already active".to_owned(),
            ));
        }
        let path = self.paths.runtime_dir.join("dictation.wav");
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(RuntimeError(format!(
                    "could not clear stale captured audio: {error}"
                )));
            }
        }
        OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&path)
            .map_err(|error| RuntimeError(format!("could not create captured audio: {error}")))?;

        let log = secure_file(&self.paths.runtime_dir.join("pw-record.log"))?;
        let stderr = log
            .try_clone()
            .map_err(|error| RuntimeError(format!("could not open capture log: {error}")))?;
        let mut command = Command::new("pw-record");
        command
            .args([
                "--rate=16000",
                "--channels=1",
                "--channel-map=mono",
                "--format=s16",
            ])
            .arg(&path)
            .stdin(Stdio::null())
            .stdout(Stdio::from(log))
            .stderr(Stdio::from(stderr))
            .kill_on_drop(true);
        let mut child = command
            .spawn()
            .map_err(|error| RuntimeError(format!("could not start pw-record: {error}")))?;
        tokio::time::sleep(Duration::from_millis(75)).await;
        if let Some(status) = child
            .try_wait()
            .map_err(|error| RuntimeError(format!("could not inspect pw-record: {error}")))?
        {
            return Err(RuntimeError(format!(
                "pw-record exited before capture began with {status}; inspect {}",
                self.paths.runtime_dir.join("pw-record.log").display()
            )));
        }
        *capture = Some(Capture { child, path });
        Ok(())
    }

    async fn discard_capture(&self) -> Result<(), RuntimeError> {
        let path = self.stop_capture().await?;
        tokio::fs::remove_file(path)
            .await
            .map_err(|error| RuntimeError(format!("could not discard captured audio: {error}")))
    }

    async fn stop_and_process(&self, action: Action) -> Result<(), RuntimeError> {
        if action != Action::Dictation {
            return Err(RuntimeError(
                "Command Mode is not included in the first Linux slice".to_owned(),
            ));
        }

        let path = self.stop_capture().await?;
        let transcription = self.whisper.transcribe(&path).await;
        let cleanup = tokio::fs::remove_file(&path).await;
        let text = transcription?;
        cleanup.map_err(|error| {
            RuntimeError(format!("could not remove private captured audio: {error}"))
        })?;
        if text.is_empty() {
            return Ok(());
        }
        insert_text(&text).await
    }
}

async fn insert_text(text: &str) -> Result<(), RuntimeError> {
    let mut child = Command::new("wtype")
        .args(["-d", "1", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| RuntimeError(format!("could not start wtype: {error}")))?;
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| RuntimeError("wtype stdin was not available".to_owned()))?;
    stdin
        .write_all(text.as_bytes())
        .await
        .map_err(|error| RuntimeError(format!("could not send text to wtype: {error}")))?;
    stdin
        .shutdown()
        .await
        .map_err(|error| RuntimeError(format!("could not finish wtype input: {error}")))?;
    let status = child
        .wait()
        .await
        .map_err(|error| RuntimeError(format!("could not wait for wtype: {error}")))?;
    if status.success() {
        Ok(())
    } else {
        Err(RuntimeError(format!("wtype failed with {status}")))
    }
}

fn secure_file(path: &Path) -> Result<File, RuntimeError> {
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(0o600)
        .open(path)
        .map_err(|error| RuntimeError(format!("could not create {}: {error}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inference_response_parses_without_exposing_more_server_shape() {
        let response: InferenceResponse =
            serde_json::from_str(r#"{"text":" hello from Yap "}"#).expect("valid response");
        assert_eq!(response.text.trim(), "hello from Yap");
    }
}
