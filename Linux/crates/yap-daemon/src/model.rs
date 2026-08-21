//! Installation and discovery of Yap's pinned local speech and language models.

use std::{
    fs::File,
    io::{BufReader, Read},
    path::{Path, PathBuf},
    process::Stdio,
};

use sha2::{Digest, Sha256};
use tokio::process::Command;

use crate::RuntimeError;

pub const MODEL_NAME: &str = "large-v3-turbo-q5_0";
pub const MODEL_FILE_NAME: &str = "ggml-large-v3-turbo-q5_0.bin";
pub const MODEL_SHA256: &str = "394221709cd5ad1f40c46e6031ca61bce88931e6e088c188294c6d5a55ffa7e2";
pub const MODEL_URL: &str =
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3-turbo-q5_0.bin";
pub const CLEANUP_MODEL_NAME: &str = "qwen3-4b-q4_k_m";
pub const CLEANUP_MODEL_FILE_NAME: &str = "Qwen3-4B-Q4_K_M.gguf";
pub const CLEANUP_MODEL_SHA256: &str =
    "7485fe6f11af29433bc51cab58009521f205840f5b4ae3a32fa7f92e8534fdf5";
pub const CLEANUP_MODEL_URL: &str = "https://huggingface.co/Qwen/Qwen3-4B-GGUF/resolve/bc640142c66e1fdd12af0bd68f40445458f3869b/Qwen3-4B-Q4_K_M.gguf";

#[derive(Clone, Copy, Debug)]
struct ModelSpec {
    name: &'static str,
    file_name: &'static str,
    sha256: &'static str,
    url: &'static str,
}

const SPEECH_MODEL: ModelSpec = ModelSpec {
    name: MODEL_NAME,
    file_name: MODEL_FILE_NAME,
    sha256: MODEL_SHA256,
    url: MODEL_URL,
};
const CLEANUP_MODEL: ModelSpec = ModelSpec {
    name: CLEANUP_MODEL_NAME,
    file_name: CLEANUP_MODEL_FILE_NAME,
    sha256: CLEANUP_MODEL_SHA256,
    url: CLEANUP_MODEL_URL,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallOutcome {
    AlreadyPresent,
    Installed,
    Repaired,
}

#[must_use]
pub fn default_path() -> PathBuf {
    model_path(SPEECH_MODEL)
}

#[must_use]
pub fn cleanup_default_path() -> PathBuf {
    model_path(CLEANUP_MODEL)
}

/// Downloads the pinned model with `curl`, verifies its SHA-256 digest, and atomically moves it
/// into the content-addressed model directory.
///
/// # Errors
///
/// Repairs a corrupt existing model. Returns an error if repair, download, verification, or the
/// filesystem operation fails.
pub async fn install() -> Result<InstallOutcome, RuntimeError> {
    install_model(SPEECH_MODEL).await
}

/// Downloads and verifies the pinned local language model used for cleanup and Command Mode.
///
/// # Errors
///
/// Returns an error if repair, download, or verification fails.
pub async fn install_cleanup() -> Result<InstallOutcome, RuntimeError> {
    install_model(CLEANUP_MODEL).await
}

async fn install_model(spec: ModelSpec) -> Result<InstallOutcome, RuntimeError> {
    let destination = model_path(spec);
    let mut repairing = false;
    if destination.exists() {
        if verify(destination.clone(), spec.sha256).await? {
            return Ok(InstallOutcome::AlreadyPresent);
        }
        tokio::fs::remove_file(&destination).await.map_err(|error| {
            RuntimeError(format!(
                "existing model failed verification and could not be removed for repair: {error}"
            ))
        })?;
        repairing = true;
    }

    let parent = destination
        .parent()
        .ok_or_else(|| RuntimeError("model path has no parent directory".to_owned()))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| RuntimeError(format!("could not create model directory: {error}")))?;

    let partial = destination.with_extension("part");
    match tokio::fs::remove_file(&partial).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(RuntimeError(format!(
                "could not clear an incomplete model download: {error}"
            )));
        }
    }

    let status = Command::new("curl")
        .args(["--fail", "--location", "--retry", "3", "--progress-bar"])
        .arg("--output")
        .arg(&partial)
        .arg(spec.url)
        .stdin(Stdio::null())
        .status()
        .await
        .map_err(|error| RuntimeError(format!("could not run curl: {error}")))?;
    if !status.success() {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err(RuntimeError(format!(
            "model download failed with status {status}"
        )));
    }

    if !verify(partial.clone(), spec.sha256).await? {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err(RuntimeError(
            "downloaded model failed SHA-256 verification".to_owned(),
        ));
    }

    tokio::fs::rename(&partial, &destination)
        .await
        .map_err(|error| RuntimeError(format!("could not finalize model download: {error}")))?;
    Ok(if repairing {
        InstallOutcome::Repaired
    } else {
        InstallOutcome::Installed
    })
}

async fn verify(path: PathBuf, expected_sha256: &'static str) -> Result<bool, RuntimeError> {
    tokio::task::spawn_blocking(move || verify_blocking(&path, expected_sha256))
        .await
        .map_err(|error| RuntimeError(format!("model verification task failed: {error}")))?
}

fn verify_blocking(path: &Path, expected_sha256: &str) -> Result<bool, RuntimeError> {
    let file = File::open(path)
        .map_err(|error| RuntimeError(format!("could not read model for verification: {error}")))?;
    let mut reader = BufReader::new(file);
    let mut digest = Sha256::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|error| RuntimeError(format!("could not verify model: {error}")))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(format!("{:x}", digest.finalize()) == expected_sha256)
}

fn model_path(spec: ModelSpec) -> PathBuf {
    data_home()
        .join("yap/models")
        .join(spec.name)
        .join(spec.sha256)
        .join(spec.file_name)
}

fn data_home() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME").map_or_else(
        || {
            std::env::var_os("HOME").map_or_else(
                || PathBuf::from(".local/share"),
                |home| PathBuf::from(home).join(".local/share"),
            )
        },
        PathBuf::from,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_model_path_is_content_addressed() {
        let path = default_path();
        assert!(
            path.ends_with(
                Path::new(MODEL_NAME)
                    .join(MODEL_SHA256)
                    .join(MODEL_FILE_NAME)
            )
        );
    }

    #[test]
    fn cleanup_model_path_is_content_addressed() {
        let path = cleanup_default_path();
        assert!(
            path.ends_with(
                Path::new(CLEANUP_MODEL_NAME)
                    .join(CLEANUP_MODEL_SHA256)
                    .join(CLEANUP_MODEL_FILE_NAME)
            )
        );
    }
}
