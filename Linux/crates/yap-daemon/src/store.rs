//! Durable, private local state for portable settings, snippets, and dictation history.

use std::{
    fs::{OpenOptions, Permissions},
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::sync::{RwLock, watch};

const STORE_VERSION: u32 = 1;
const MAX_HISTORY_ENTRIES: usize = 200;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupIntensity {
    Off,
    Light,
    #[default]
    Medium,
    High,
    Max,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioMode {
    Off,
    Lower,
    #[default]
    Pause,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Settings {
    pub cleanup_intensity: CleanupIntensity,
    pub audio_mode: AudioMode,
    pub duck_level: f64,
    pub trim_courtesy: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            cleanup_intensity: CleanupIntensity::Medium,
            audio_mode: AudioMode::Pause,
            duck_level: 0.15,
            trim_courtesy: true,
        }
    }
}

impl Settings {
    fn validate(&self) -> Result<(), StoreError> {
        if self.duck_level.is_finite() && (0.0..=1.0).contains(&self.duck_level) {
            Ok(())
        } else {
            Err(StoreError::Invalid(
                "duck_level must be a finite number between 0 and 1".to_owned(),
            ))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Snippet {
    pub id: u64,
    pub trigger: String,
    pub expansion: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct HistoryEntry {
    pub id: u64,
    pub text: String,
    pub app_name: Option<String>,
    pub created_unix_ms: u64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StoreSnapshot {
    pub version: u32,
    pub settings: Settings,
    pub snippets: Vec<Snippet>,
    pub history: Vec<HistoryEntry>,
    next_id: u64,
}

impl Default for StoreSnapshot {
    fn default() -> Self {
        Self {
            version: STORE_VERSION,
            settings: Settings::default(),
            snippets: Vec::new(),
            history: Vec::new(),
            next_id: 1,
        }
    }
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("local state is invalid: {0}")]
    Invalid(String),
    #[error("local state could not be read: {0}")]
    Read(String),
    #[error("local state could not be written: {0}")]
    Write(String),
}

/// Owns all durable user state behind one typed interface and one atomic private file.
#[derive(Debug)]
pub struct StateStore {
    path: PathBuf,
    state: RwLock<StoreSnapshot>,
    updates: watch::Sender<StoreSnapshot>,
}

impl StateStore {
    /// Opens the standard XDG data file, creating private storage on first use.
    ///
    /// # Errors
    ///
    /// Returns an error when the data directory cannot be secured or existing JSON is invalid.
    pub fn discover() -> Result<Self, StoreError> {
        let base = std::env::var_os("XDG_DATA_HOME").map_or_else(
            || {
                std::env::var_os("HOME").map_or_else(
                    || PathBuf::from(".local/share"),
                    |home| PathBuf::from(home).join(".local/share"),
                )
            },
            PathBuf::from,
        );
        Self::open(base.join("yap/state.json"))
    }

    /// Opens a specific state file. This is also the test seam for persistence behavior.
    ///
    /// # Errors
    ///
    /// Returns an error when private storage cannot be prepared or existing JSON is invalid.
    pub fn open(path: PathBuf) -> Result<Self, StoreError> {
        secure_parent(&path)?;
        let state = if path.is_file() {
            let bytes = std::fs::read(&path).map_err(|error| StoreError::Read(error.to_string()))?;
            let state: StoreSnapshot = serde_json::from_slice(&bytes)
                .map_err(|error| StoreError::Read(error.to_string()))?;
            validate_snapshot(&state)?;
            state
        } else {
            let state = StoreSnapshot::default();
            write_private_json(&path, &state)?;
            state
        };
        let (updates, _) = watch::channel(state.clone());
        Ok(Self {
            path,
            state: RwLock::new(state),
            updates,
        })
    }

    pub async fn snapshot(&self) -> StoreSnapshot {
        self.state.read().await.clone()
    }

    /// Subscribes to committed state. Receivers immediately contain the current snapshot.
    #[must_use]
    pub fn subscribe(&self) -> watch::Receiver<StoreSnapshot> {
        self.updates.subscribe()
    }

    /// Serializes the user-visible state while keeping persistence bookkeeping private.
    ///
    /// # Errors
    ///
    /// Returns an error only if the typed state cannot be represented as JSON.
    pub fn public_json(state: &StoreSnapshot) -> Result<String, StoreError> {
        #[derive(Serialize)]
        struct PublicState<'a> {
            version: u32,
            settings: &'a Settings,
            snippets: &'a [Snippet],
            history: &'a [HistoryEntry],
        }

        serde_json::to_string(&PublicState {
            version: state.version,
            settings: &state.settings,
            snippets: &state.snippets,
            history: &state.history,
        })
        .map_err(|error| StoreError::Write(error.to_string()))
    }

    /// Replaces portable settings and commits them atomically.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid values or if the private state file cannot be replaced.
    pub async fn update_settings(&self, settings: Settings) -> Result<StoreSnapshot, StoreError> {
        settings.validate()?;
        let mut state = self.state.write().await;
        let mut candidate = state.clone();
        candidate.settings = settings;
        write_private_json(&self.path, &candidate)?;
        *state = candidate.clone();
        self.updates.send_replace(candidate.clone());
        Ok(candidate)
    }

    /// Adds a snippet or updates an existing one, returning the complete committed state.
    ///
    /// # Errors
    ///
    /// Returns an error for empty values, an unknown identifier, or a failed atomic write.
    pub async fn save_snippet(
        &self,
        id: Option<u64>,
        trigger: &str,
        expansion: &str,
    ) -> Result<StoreSnapshot, StoreError> {
        let trigger = trigger.trim();
        let expansion = expansion.trim();
        if trigger.is_empty() || expansion.is_empty() {
            return Err(StoreError::Invalid(
                "snippet trigger and expansion must not be empty".to_owned(),
            ));
        }

        let mut state = self.state.write().await;
        let mut candidate = state.clone();
        if let Some(id) = id {
            let snippet = candidate
                .snippets
                .iter_mut()
                .find(|snippet| snippet.id == id)
                .ok_or_else(|| StoreError::Invalid(format!("unknown snippet id {id}")))?;
            snippet.trigger = trigger.to_owned();
            snippet.expansion = expansion.to_owned();
        } else {
            let id = take_id(&mut candidate);
            candidate.snippets.push(Snippet {
                id,
                trigger: trigger.to_owned(),
                expansion: expansion.to_owned(),
            });
        }
        write_private_json(&self.path, &candidate)?;
        *state = candidate.clone();
        self.updates.send_replace(candidate.clone());
        Ok(candidate)
    }

    /// Removes a snippet and returns the complete committed state.
    ///
    /// # Errors
    ///
    /// Returns an error for an unknown identifier or a failed atomic write.
    pub async fn remove_snippet(&self, id: u64) -> Result<StoreSnapshot, StoreError> {
        let mut state = self.state.write().await;
        let mut candidate = state.clone();
        let before = candidate.snippets.len();
        candidate.snippets.retain(|snippet| snippet.id != id);
        if candidate.snippets.len() == before {
            return Err(StoreError::Invalid(format!("unknown snippet id {id}")));
        }
        write_private_json(&self.path, &candidate)?;
        *state = candidate.clone();
        self.updates.send_replace(candidate.clone());
        Ok(candidate)
    }

    /// Records non-empty inserted text, newest first, and caps history at 200 entries.
    ///
    /// # Errors
    ///
    /// Returns an error if the private state file cannot be replaced.
    pub async fn record_history(
        &self,
        text: &str,
        app_name: Option<&str>,
    ) -> Result<StoreSnapshot, StoreError> {
        let text = text.trim();
        if text.is_empty() {
            return Ok(self.snapshot().await);
        }
        let mut state = self.state.write().await;
        let mut candidate = state.clone();
        let id = take_id(&mut candidate);
        candidate.history.insert(
            0,
            HistoryEntry {
                id,
                text: text.to_owned(),
                app_name: app_name.map(str::trim).filter(|name| !name.is_empty()).map(str::to_owned),
                created_unix_ms: unix_time_ms(),
            },
        );
        candidate.history.truncate(MAX_HISTORY_ENTRIES);
        write_private_json(&self.path, &candidate)?;
        *state = candidate.clone();
        self.updates.send_replace(candidate.clone());
        Ok(candidate)
    }

    /// Clears dictation history without changing settings or snippets.
    ///
    /// # Errors
    ///
    /// Returns an error if the private state file cannot be replaced.
    pub async fn clear_history(&self) -> Result<StoreSnapshot, StoreError> {
        let mut state = self.state.write().await;
        let mut candidate = state.clone();
        candidate.history.clear();
        write_private_json(&self.path, &candidate)?;
        *state = candidate.clone();
        self.updates.send_replace(candidate.clone());
        Ok(candidate)
    }

    /// Expands every configured whole-word snippet in insertion order.
    pub async fn apply_snippets(&self, text: &str) -> String {
        self.state
            .read()
            .await
            .snippets
            .iter()
            .fold(text.to_owned(), |result, snippet| {
                replace_whole_phrase(&result, &snippet.trigger, &snippet.expansion)
            })
    }
}

fn validate_snapshot(state: &StoreSnapshot) -> Result<(), StoreError> {
    if state.version != STORE_VERSION {
        return Err(StoreError::Invalid(format!(
            "unsupported state version {}; expected {STORE_VERSION}",
            state.version
        )));
    }
    state.settings.validate()?;
    if state.history.len() > MAX_HISTORY_ENTRIES {
        return Err(StoreError::Invalid(format!(
            "history contains more than {MAX_HISTORY_ENTRIES} entries"
        )));
    }
    Ok(())
}

fn take_id(state: &mut StoreSnapshot) -> u64 {
    let id = state.next_id;
    state.next_id = state.next_id.saturating_add(1);
    id
}

fn secure_parent(path: &Path) -> Result<(), StoreError> {
    let parent = path
        .parent()
        .ok_or_else(|| StoreError::Write("state path has no parent directory".to_owned()))?;
    std::fs::create_dir_all(parent).map_err(|error| StoreError::Write(error.to_string()))?;
    std::fs::set_permissions(parent, Permissions::from_mode(0o700))
        .map_err(|error| StoreError::Write(error.to_string()))
}

fn write_private_json(path: &Path, state: &StoreSnapshot) -> Result<(), StoreError> {
    secure_parent(path)?;
    let temporary = path.with_extension(format!("tmp-{}-{}", std::process::id(), unix_time_ms()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&temporary)
            .map_err(|error| StoreError::Write(error.to_string()))?;
        serde_json::to_writer_pretty(&mut file, state)
            .map_err(|error| StoreError::Write(error.to_string()))?;
        file.write_all(b"\n")
            .map_err(|error| StoreError::Write(error.to_string()))?;
        file.sync_all()
            .map_err(|error| StoreError::Write(error.to_string()))?;
        std::fs::rename(&temporary, path).map_err(|error| StoreError::Write(error.to_string()))?;
        std::fs::set_permissions(path, Permissions::from_mode(0o600))
            .map_err(|error| StoreError::Write(error.to_string()))
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

fn unix_time_ms() -> u64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis());
    u64::try_from(millis).unwrap_or(u64::MAX)
}

fn replace_whole_phrase(text: &str, trigger: &str, expansion: &str) -> String {
    if trigger.is_empty() {
        return text.to_owned();
    }
    let mut matches = Vec::new();
    for (start, _) in text.char_indices() {
        let end = start.saturating_add(trigger.len());
        let Some(candidate) = text.get(start..end) else {
            continue;
        };
        if !candidate.eq_ignore_ascii_case(trigger) {
            continue;
        }
        let before = text[..start].chars().next_back();
        let after = text[end..].chars().next();
        if before.is_none_or(|character| !character.is_alphanumeric())
            && after.is_none_or(|character| !character.is_alphanumeric())
        {
            matches.push((start, end));
        }
    }
    let mut result = text.to_owned();
    for (start, end) in matches.into_iter().rev() {
        result.replace_range(start..end, expansion);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "yap-store-{name}-{}-{}/state.json",
            std::process::id(),
            unix_time_ms()
        ))
    }

    #[tokio::test]
    async fn settings_snippets_and_history_survive_reopen() {
        let path = test_path("round-trip");
        let store = StateStore::open(path.clone()).expect("store opens");
        store
            .update_settings(Settings {
                cleanup_intensity: CleanupIntensity::High,
                audio_mode: AudioMode::Lower,
                duck_level: 0.25,
                trim_courtesy: false,
            })
            .await
            .expect("settings persist");
        store
            .save_snippet(None, "my address", "42 Yap Street")
            .await
            .expect("snippet persists");
        store
            .record_history("Hello from Linux", Some("Zed"))
            .await
            .expect("history persists");
        drop(store);

        let reopened = StateStore::open(path).expect("store reopens");
        let snapshot = reopened.snapshot().await;
        assert_eq!(snapshot.settings.cleanup_intensity, CleanupIntensity::High);
        assert_eq!(snapshot.settings.audio_mode, AudioMode::Lower);
        assert_eq!(snapshot.snippets[0].trigger, "my address");
        assert_eq!(snapshot.history[0].text, "Hello from Linux");
        assert_eq!(snapshot.history[0].app_name.as_deref(), Some("Zed"));
    }

    #[tokio::test]
    async fn history_is_capped_and_clear_does_not_touch_snippets() {
        let store = StateStore::open(test_path("history-cap")).expect("store opens");
        store
            .save_snippet(None, "signature", "Kind regards")
            .await
            .expect("snippet persists");
        for index in 0..=MAX_HISTORY_ENTRIES {
            store
                .record_history(&format!("entry {index}"), None)
                .await
                .expect("history persists");
        }
        let snapshot = store.snapshot().await;
        assert_eq!(snapshot.history.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(snapshot.history[0].text, format!("entry {MAX_HISTORY_ENTRIES}"));
        assert_eq!(snapshot.history.last().expect("oldest retained").text, "entry 1");

        let snapshot = store.clear_history().await.expect("history clears");
        assert!(snapshot.history.is_empty());
        assert_eq!(snapshot.snippets.len(), 1);
    }

    #[test]
    fn store_file_and_directory_are_private() {
        let path = test_path("permissions");
        StateStore::open(path.clone()).expect("store opens");
        let directory_mode = std::fs::metadata(path.parent().expect("parent"))
            .expect("directory metadata")
            .permissions()
            .mode()
            & 0o777;
        let file_mode = std::fs::metadata(path)
            .expect("file metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(directory_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }

    #[tokio::test]
    async fn snippets_expand_case_insensitive_whole_phrases() {
        let store = StateStore::open(test_path("snippet-expansion")).expect("store opens");
        store
            .save_snippet(None, "my address", "42 Yap Street")
            .await
            .expect("snippet persists");

        assert_eq!(
            store.apply_snippets("Send it to MY ADDRESS, please.").await,
            "Send it to 42 Yap Street, please."
        );
        assert_eq!(
            store.apply_snippets("notmy addressbook").await,
            "notmy addressbook"
        );
    }

    #[tokio::test]
    async fn subscribers_receive_committed_state_only() {
        let store = StateStore::open(test_path("subscription")).expect("store opens");
        let mut updates = store.subscribe();
        let settings = Settings {
            cleanup_intensity: CleanupIntensity::Light,
            ..Settings::default()
        };

        store
            .update_settings(settings.clone())
            .await
            .expect("settings persist");
        updates.changed().await.expect("update is published");
        assert_eq!(updates.borrow().settings, settings);
    }
}
