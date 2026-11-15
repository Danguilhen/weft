// SPDX-License-Identifier: AGPL-3.0-or-later

//! Minimal Prometheus-compatible metrics for Milestone 0.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

#[derive(Debug)]
pub struct Metrics {
    requests: AtomicU64,
    errors: AtomicU64,
    uploads: AtomicU64,
    uploads_failed: AtomicU64,
    decode_failures: AtomicU64,
    decode_busy: AtomicU64,
    quota_rejected: AtomicU64,
    quarantine_expired: AtomicU64,
    retention_purged: AtomicU64,
    bytes_stored: AtomicU64,
    media_bytes: AtomicU64,
    started: Instant,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            requests: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            uploads: AtomicU64::new(0),
            uploads_failed: AtomicU64::new(0),
            decode_failures: AtomicU64::new(0),
            decode_busy: AtomicU64::new(0),
            quota_rejected: AtomicU64::new(0),
            quarantine_expired: AtomicU64::new(0),
            retention_purged: AtomicU64::new(0),
            bytes_stored: AtomicU64::new(0),
            media_bytes: AtomicU64::new(0),
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

    pub fn upload_accepted(&self) {
        self.uploads.fetch_add(1, Ordering::Relaxed);
    }

    pub fn upload_failed(&self) {
        self.uploads_failed.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decode_failed(&self) {
        self.decode_failures.fetch_add(1, Ordering::Relaxed);
    }

    pub fn decode_busy(&self) {
        self.decode_busy.fetch_add(1, Ordering::Relaxed);
    }

    pub fn quota_rejected(&self) {
        self.quota_rejected.fetch_add(1, Ordering::Relaxed);
    }

    pub fn quarantine_expired(&self, count: u64) {
        self.quarantine_expired.fetch_add(count, Ordering::Relaxed);
    }

    pub fn retention_purged(&self, count: u64) {
        self.retention_purged.fetch_add(count, Ordering::Relaxed);
    }

    pub fn stored(&self, bytes: u64) {
        self.bytes_stored.fetch_add(bytes, Ordering::Relaxed);
    }

    pub fn media_served(&self, bytes: u64) {
        self.media_bytes.fetch_add(bytes, Ordering::Relaxed);
    }

    #[must_use]
    pub fn render(&self) -> String {
        let requests = self.requests.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        let uploads = self.uploads.load(Ordering::Relaxed);
        let uploads_failed = self.uploads_failed.load(Ordering::Relaxed);
        let decode_failures = self.decode_failures.load(Ordering::Relaxed);
        let decode_busy = self.decode_busy.load(Ordering::Relaxed);
        let quota_rejected = self.quota_rejected.load(Ordering::Relaxed);
        let quarantine_expired = self.quarantine_expired.load(Ordering::Relaxed);
        let retention_purged = self.retention_purged.load(Ordering::Relaxed);
        let bytes_stored = self.bytes_stored.load(Ordering::Relaxed);
        let media_bytes = self.media_bytes.load(Ordering::Relaxed);
        let uptime = self.started.elapsed().as_secs();
        format!(
            "# HELP weft_requests_total HTTP requests handled.\n\
             # TYPE weft_requests_total counter\n\
             weft_requests_total {requests}\n\
             # HELP weft_errors_total HTTP responses with status >= 500.\n\
             # TYPE weft_errors_total counter\n\
             weft_errors_total {errors}\n\
             # HELP weft_uploads_total Accepted uploads.\n\
             # TYPE weft_uploads_total counter\n\
             weft_uploads_total {uploads}\n\
             # HELP weft_uploads_failed_total Rejected or failed uploads.\n\
             # TYPE weft_uploads_failed_total counter\n\
             weft_uploads_failed_total {uploads_failed}\n\
             # HELP weft_decode_failures_total Decode worker failures.\n\
             # TYPE weft_decode_failures_total counter\n\
             weft_decode_failures_total {decode_failures}\n\
             # HELP weft_decode_busy_total Uploads refused because all decode slots were busy.\n\
             # TYPE weft_decode_busy_total counter\n\
             weft_decode_busy_total {decode_busy}\n\
             # HELP weft_quota_rejected_total Uploads refused by a pending or byte quota.\n\
             # TYPE weft_quota_rejected_total counter\n\
             weft_quota_rejected_total {quota_rejected}\n\
             # HELP weft_quarantine_expired_total Pending items removed by quarantine expiry.\n\
             # TYPE weft_quarantine_expired_total counter\n\
             weft_quarantine_expired_total {quarantine_expired}\n\
             # HELP weft_retention_purged_total Client hashes and reports removed by retention.\n\
             # TYPE weft_retention_purged_total counter\n\
             weft_retention_purged_total {retention_purged}\n\
             # HELP weft_bytes_stored_total Original bytes stored.\n\
             # TYPE weft_bytes_stored_total counter\n\
             weft_bytes_stored_total {bytes_stored}\n\
             # HELP weft_media_bytes_total Original bytes served.\n\
             # TYPE weft_media_bytes_total counter\n\
             weft_media_bytes_total {media_bytes}\n\
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
        metrics.upload_accepted();
        metrics.stored(10);
        let output = metrics.render();
        assert!(output.contains("weft_requests_total 2"));
        assert!(output.contains("weft_errors_total 1"));
        assert!(output.contains("weft_uploads_total 1"));
        assert!(output.contains("weft_bytes_stored_total 10"));
    }
}
