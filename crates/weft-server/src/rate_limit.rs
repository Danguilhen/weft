// SPDX-License-Identifier: AGPL-3.0-or-later

//! In-memory per-client token buckets for anonymous uploads.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Mutex;
use std::time::Instant;

use axum::http::HeaderMap;

#[derive(Debug)]
pub struct RateLimiter {
    per_hour: f64,
    capacity: f64,
    salt: u64,
    buckets: Mutex<HashMap<u64, Bucket>>,
}

#[derive(Debug)]
struct Bucket {
    tokens: f64,
    updated: Instant,
}

impl RateLimiter {
    #[must_use]
    pub fn new(per_hour: u32, burst: u32) -> Self {
        let per_hour = f64::from(per_hour.max(1));
        let capacity = f64::from(burst.max(1));
        let salt = random_salt();
        Self {
            per_hour,
            capacity,
            salt,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Consume one token. Returns the seconds to wait when the bucket is empty.
    pub fn check(&self, client: &str) -> Result<(), u64> {
        let key = self.hash(client);
        let mut buckets = self.buckets.lock().expect("rate limiter lock");
        if buckets.len() > 10_000 {
            buckets.retain(|_, bucket| bucket.tokens < self.capacity);
        }
        let now = Instant::now();
        let bucket = buckets.entry(key).or_insert(Bucket {
            tokens: self.capacity,
            updated: now,
        });
        let elapsed = now.duration_since(bucket.updated).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.per_hour / 3600.0).min(self.capacity);
        bucket.updated = now;
        if bucket.tokens < 1.0 {
            let seconds = ((1.0 - bucket.tokens) / (self.per_hour / 3600.0)).ceil();
            return Err(seconds.max(1.0) as u64);
        }
        bucket.tokens -= 1.0;
        Ok(())
    }

    fn hash(&self, client: &str) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.salt.hash(&mut hasher);
        client.hash(&mut hasher);
        hasher.finish()
    }
}

fn random_salt() -> u64 {
    let mut bytes = [0_u8; 8];
    if getrandom::getrandom(&mut bytes).is_ok() {
        u64::from_le_bytes(bytes)
    } else {
        0x5765_6674_0000_0001
    }
}

/// Extract a client identifier from the proxy headers.
///
/// The server trusts the reverse proxy to set these; direct deployments can
/// omit them and share a single bucket.
#[must_use]
pub fn client_key(headers: &HeaderMap) -> String {
    for name in ["x-forwarded-for", "x-real-ip"] {
        if let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) {
            let first = value.split(',').next().unwrap_or(value).trim();
            if !first.is_empty() {
                return first.to_string();
            }
        }
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::RateLimiter;

    #[test]
    fn consumes_burst_then_limits() {
        let limiter = RateLimiter::new(100, 2);
        assert!(limiter.check("a").is_ok());
        assert!(limiter.check("a").is_ok());
        assert!(limiter.check("a").is_err());
        assert!(limiter.check("b").is_ok());
    }
}
