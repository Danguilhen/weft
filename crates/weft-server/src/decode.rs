// SPDX-License-Identifier: AGPL-3.0-or-later

//! Invoke the isolated `weft-decode` worker.

use std::path::{Path, PathBuf};
use std::time::Duration;

use thiserror::Error;
use tokio::process::Command;
use weft_core::config::{DecodeConfig, UploadConfig};
use weft_validation::DecodedMedia;

pub fn resolve_worker(config: &DecodeConfig) -> PathBuf {
    if !config.worker_path.trim().is_empty() {
        return PathBuf::from(&config.worker_path);
    }
    if let Ok(executable) = std::env::current_exe() {
        if let Some(parent) = executable.parent() {
            let sibling = parent.join("weft-decode");
            if sibling.exists() {
                return sibling;
            }
        }
    }
    PathBuf::from("weft-decode")
}

pub async fn run_worker(
    worker: &Path,
    input: &Path,
    thumbnail: &Path,
    limits: &UploadConfig,
    decode: &DecodeConfig,
    max_bytes: u64,
    thumbnail_size: u32,
) -> Result<DecodedMedia, DecodeWorkerError> {
    let mut command = Command::new(worker);
    command
        .arg("--input")
        .arg(input)
        .arg("--thumbnail")
        .arg(thumbnail)
        .arg("--max-bytes")
        .arg(max_bytes.to_string())
        .arg("--max-pixels")
        .arg(limits.max_pixels.to_string())
        .arg("--max-frames")
        .arg(limits.max_frames.to_string())
        .arg("--max-decoded-bytes")
        .arg(
            weft_core::parse_size(&limits.max_decoded_memory)
                .unwrap_or(256 * 1024 * 1024)
                .to_string(),
        )
        .arg("--thumbnail-size")
        .arg(thumbnail_size.clamp(1, 4096).to_string())
        .kill_on_drop(true);

    let output = tokio::time::timeout(
        Duration::from_secs(decode.timeout_seconds.max(1)),
        command.output(),
    )
    .await
    .map_err(|_| DecodeWorkerError::Timeout)?
    .map_err(DecodeWorkerError::Io)?;

    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr);
        return Err(DecodeWorkerError::Rejected(detail.trim().to_string()));
    }

    serde_json::from_slice(&output.stdout).map_err(DecodeWorkerError::Json)
}

#[derive(Debug, Error)]
pub enum DecodeWorkerError {
    #[error("decode worker timed out")]
    Timeout,
    #[error("could not run decode worker: {0}")]
    Io(std::io::Error),
    #[error("decode rejected the upload: {0}")]
    Rejected(String),
    #[error("invalid decode worker output: {0}")]
    Json(serde_json::Error),
}
