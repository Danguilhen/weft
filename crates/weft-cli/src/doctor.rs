// SPDX-License-Identifier: AGPL-3.0-or-later

//! Host checks for the supported Weft node profile.

use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use weft_core::{parse_size, AppConfig};

const TWO_GIB: u64 = 2 * 1024 * 1024 * 1024;
const ONE_GIB: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct Report {
    pub warnings: Vec<String>,
    pub failures: Vec<String>,
}

impl Report {
    #[must_use]
    pub fn ok(&self) -> bool {
        self.failures.is_empty()
    }

    pub fn print(&self) {
        for warning in &self.warnings {
            println!("warning: {warning}");
        }
        for failure in &self.failures {
            println!("failure: {failure}");
        }
        if self.ok() {
            println!("weft doctor: host meets the supported profile");
        } else {
            println!("weft doctor: host does not meet the supported profile");
        }
    }
}

/// Run the checks and print a human-readable report. Returns an error when a
/// hard requirement fails.
pub fn run(config_path: &Path) -> Result<()> {
    let config = AppConfig::load(config_path)
        .with_context(|| format!("loading {}", config_path.display()))?;
    let report = check(&config);
    report.print();
    if report.ok() {
        Ok(())
    } else {
        bail!("host does not meet the supported node profile")
    }
}

#[must_use]
pub fn check(config: &AppConfig) -> Report {
    let mut report = Report::default();

    match total_memory_bytes() {
        Some(total) if total < TWO_GIB => report.failures.push(format!(
            "detected {} MiB of RAM; the supported profile needs 2 GiB. Use a larger host.",
            total / 1024 / 1024
        )),
        Some(total) => {
            if let Some(decode) = parse_size(&config.upload.max_decoded_memory) {
                let budget =
                    decode.saturating_mul(u64::from(config.upload.max_concurrent_decodes.max(1)));
                if budget > total / 3 {
                    report.warnings.push(format!(
                        "configured decode budget ({} MiB) is more than a third of RAM",
                        budget / 1024 / 1024
                    ));
                }
            } else {
                report
                    .warnings
                    .push("could not parse upload.max_decoded_memory".to_string());
            }
        }
        None => report
            .warnings
            .push("could not read /proc/meminfo; memory check skipped".to_string()),
    }

    if !Path::new(&config.storage.path).exists() {
        report.failures.push(format!(
            "storage path {} does not exist; run weft init or create it",
            config.storage.path
        ));
    } else {
        match available_disk_bytes(&config.storage.path) {
            Ok(available) if available < ONE_GIB => report.failures.push(format!(
                "only {} MiB free at {}; at least 1 GiB is required",
                available / 1024 / 1024,
                config.storage.path
            )),
            Ok(available) => {
                if let Some(max_size) = parse_size(&config.storage.max_size) {
                    if available < max_size {
                        report.warnings.push(format!(
                            "only {} MiB free at {}; storage.max_size is {} MiB",
                            available / 1024 / 1024,
                            config.storage.path,
                            max_size / 1024 / 1024
                        ));
                    }
                }
            }
            Err(error) => report
                .warnings
                .push(format!("could not read free space: {error}")),
        }
    }

    report
}

fn total_memory_bytes() -> Option<u64> {
    let text = fs::read_to_string("/proc/meminfo").ok()?;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("MemTotal:") {
            let kilobytes: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kilobytes.saturating_mul(1024));
        }
    }
    None
}

fn available_disk_bytes(path: &str) -> Result<u64> {
    let output = Command::new("df")
        .args(["-Pk", path])
        .output()
        .context("running df")?;
    if !output.status.success() {
        bail!("df failed for {path}");
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let line = text
        .lines()
        .last()
        .context("df returned no filesystem line")?;
    let available_kb: u64 = line
        .split_whitespace()
        .nth(3)
        .context("df output has no available-space field")?
        .parse()
        .context("parsing df available space")?;
    Ok(available_kb.saturating_mul(1024))
}
