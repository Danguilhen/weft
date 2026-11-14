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

    match effective_memory_bytes() {
        Some(total) if total < TWO_GIB => report.failures.push(format!(
            "detected {} MiB of usable RAM; the supported profile needs 2 GiB. Use a larger host.",
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

    match effective_cpus() {
        Some(cpus) if cpus + 0.01 < 2.0 => report.failures.push(format!(
            "detected {cpus:.1} usable CPUs; the supported profile needs 2. Use a larger host or raise the CPU quota."
        )),
        Some(_) => {}
        None => report
            .warnings
            .push("could not detect usable CPUs; CPU check skipped".to_string()),
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

/// The smaller of physical RAM and the cgroup memory limit. Containers often
/// expose the host's `/proc/meminfo`, so the cgroup value is authoritative.
fn effective_memory_bytes() -> Option<u64> {
    let physical = total_memory_bytes();
    let cgroup = cgroup_memory_limit();
    match (physical, cgroup) {
        (Some(physical), Some(cgroup)) => Some(physical.min(cgroup)),
        (physical, cgroup) => physical.or(cgroup),
    }
}

fn cgroup_memory_limit() -> Option<u64> {
    if let Some(path) = cgroup_path("0::") {
        if let Some(limit) = read_cgroup_value(&format!("/sys/fs/cgroup{path}/memory.max")) {
            return parse_cgroup_memory(&limit);
        }
    }
    let path = cgroup_path(":memory:")?;
    let limit = read_cgroup_value(&format!(
        "/sys/fs/cgroup/memory{path}/memory.limit_in_bytes"
    ))?;
    parse_cgroup_memory(&limit)
}

fn parse_cgroup_memory(raw: &str) -> Option<u64> {
    let value: u64 = raw.trim().parse().ok()?;
    (value > 0 && value < 1 << 60).then_some(value)
}

/// Usable CPUs after both affinity and the cgroup CPU quota.
fn effective_cpus() -> Option<f64> {
    let host = std::thread::available_parallelism()
        .ok()
        .map(|count| count.get() as f64)?;
    let quota = cgroup_cpu_quota();
    Some(quota.map_or(host, |quota| host.min(quota)))
}

fn cgroup_cpu_quota() -> Option<f64> {
    if let Some(path) = cgroup_path("0::") {
        if let Some(raw) = read_cgroup_value(&format!("/sys/fs/cgroup{path}/cpu.max")) {
            return parse_cpu_max(&raw);
        }
    }
    let path = cgroup_path(":cpu:")?;
    let quota = read_cgroup_value(&format!("/sys/fs/cgroup/cpu{path}/cpu.cfs_quota_us"))?;
    let period = read_cgroup_value(&format!("/sys/fs/cgroup/cpu{path}/cpu.cfs_period_us"))?;
    parse_cpu_quota(&quota, &period)
}

fn parse_cpu_max(raw: &str) -> Option<f64> {
    let mut parts = raw.split_whitespace();
    let quota = parts.next()?;
    let period = parts.next().unwrap_or("100000");
    if quota == "max" {
        return None;
    }
    parse_cpu_quota(quota, period)
}

fn parse_cpu_quota(quota: &str, period: &str) -> Option<f64> {
    let quota: f64 = quota.trim().parse().ok()?;
    let period: f64 = period.trim().parse().ok()?;
    (quota > 0.0 && period > 0.0).then_some(quota / period)
}

fn cgroup_path(prefix: &str) -> Option<String> {
    let text = fs::read_to_string("/proc/self/cgroup").ok()?;
    text.lines().find_map(|line| {
        let path = line.strip_prefix(prefix)?;
        (!path.is_empty()).then(|| path.to_string())
    })
}

fn read_cgroup_value(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cgroup_memory_max_parses_limits() {
        assert_eq!(parse_cgroup_memory("1073741824\n"), Some(1 << 30));
        assert_eq!(parse_cgroup_memory("max"), None);
        assert_eq!(parse_cgroup_memory("0"), None);
        assert_eq!(
            parse_cgroup_memory("9223372036854771712"),
            None,
            "the v1 unlimited sentinel is not a limit"
        );
    }

    #[test]
    fn cpu_max_parses_quotas() {
        assert_eq!(parse_cpu_max("200000 100000"), Some(2.0));
        assert_eq!(parse_cpu_max("50000 100000\n"), Some(0.5));
        assert_eq!(parse_cpu_max("max 100000"), None);
        assert_eq!(parse_cpu_max("garbage"), None);
        assert_eq!(parse_cpu_quota("-1", "100000"), None);
    }
}
