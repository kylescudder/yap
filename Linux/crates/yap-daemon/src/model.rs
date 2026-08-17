//! Installation and discovery of Yap's pinned local speech model.

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstallOutcome {
    AlreadyPresent,
    Installed,
}

#[must_use]
pub fn default_path() -> PathBuf {
    data_home()
        .join("yap/models")
        .join(MODEL_NAME)
        .join(MODEL_SHA256)
        .join(MODEL_FILE_NAME)
}

/// Downloads the pinned model with `curl`, verifies its SHA-256 digest, and atomically moves it
/// into the content-addressed model directory.
///
/// # Errors
///
/// Returns an error if an existing model is corrupt, the download command fails, the downloaded
/// digest differs, or the filesystem operation fails.
pub async fn install() -> Result<InstallOutcome, RuntimeError> {
    let destination = default_path();
    if destination.exists() {
        if verify(destination.clone()).await? {
            return Ok(InstallOutcome::AlreadyPresent);
        }
        return Err(RuntimeError(format!(
            "existing model failed checksum verification: {}",
            destination.display()
        )));
    }

    let parent = destination
        .parent()
        .ok_or_else(|| RuntimeError("model path has no parent directory".to_owned()))?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| RuntimeError(format!("could not create model directory: {error}")))?;

    let partial = destination.with_extension("bin.part");
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
        .arg(MODEL_URL)
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

    if !verify(partial.clone()).await? {
        let _ = tokio::fs::remove_file(&partial).await;
        return Err(RuntimeError(
            "downloaded model failed SHA-256 verification".to_owned(),
        ));
    }

    tokio::fs::rename(&partial, &destination)
        .await
        .map_err(|error| RuntimeError(format!("could not finalize model download: {error}")))?;
    Ok(InstallOutcome::Installed)
}

async fn verify(path: PathBuf) -> Result<bool, RuntimeError> {
    tokio::task::spawn_blocking(move || verify_blocking(&path))
        .await
        .map_err(|error| RuntimeError(format!("model verification task failed: {error}")))?
}

fn verify_blocking(path: &Path) -> Result<bool, RuntimeError> {
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
    Ok(format!("{:x}", digest.finalize()) == MODEL_SHA256)
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
}
