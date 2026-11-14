// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimal Prometheus-compatible metrics for Milestone 0.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug)]
pub struct Metrics {
    requests: AtomicU64,
    errors: AtomicU64,
    started: Instant,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            requests: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            started: Instant::now(),
        }
    }
}

impl Metrics {
    pub fn record(&self, status: u16) {
        self.requests.fetch_add(1, Ordering::Relaxed);
        if status >= 500 {
            self.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    #[must_use]
    pub fn render(&self) -> String {
        let requests = self.requests.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        let uptime = self.started.elapsed().as_secs();
        format!(
            "# HELP weft_requests_total HTTP requests handled.\n\
             # TYPE weft_requests_total counter\n\
             weft_requests_total {requests}\n\
             # HELP weft_errors_total HTTP responses with status >= 500.\n\
             # TYPE weft_errors_total counter\n\
             weft_errors_total {errors}\n\
             # HELP weft_uptime_seconds Process uptime.\n\
             # TYPE weft_uptime_seconds gauge\n\
             weft_uptime_seconds {uptime}\n"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Metrics;

    #[test]
    fn renders_counters() {
        let metrics = Metrics::default();
        metrics.record(200);
        metrics.record(503);
        let output = metrics.render();
        assert!(output.contains("weft_requests_total 2"));
        assert!(output.contains("weft_errors_total 1"));
    }
}
