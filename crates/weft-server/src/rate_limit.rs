// SPDX-License-Identifier: AGPL-3.0-or-later

//! Client identity, salted hashing and in-memory per-client token buckets.

use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::net::{IpAddr, Ipv6Addr};
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use axum::http::HeaderMap;
use sha2::{Digest, Sha256};

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

/// A persistent salt for client pseudonyms stored in PostgreSQL.
#[derive(Debug)]
pub struct ClientHasher {
    salt: [u8; 32],
}

impl ClientHasher {
    /// Load a salt, creating it with mode `0600` when absent.
    pub fn load_or_create(path: &str) -> std::io::Result<Self> {
        if path.trim().is_empty() {
            return Ok(Self {
                salt: random_bytes()?,
            });
        }
        let path = Path::new(path);
        match fs::read(path) {
            Ok(bytes) if bytes.len() >= 32 => {
                let mut salt = [0_u8; 32];
                salt.copy_from_slice(&bytes[..32]);
                Ok(Self { salt })
            }
            Ok(_) => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "hash salt file must contain at least 32 bytes",
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let salt = random_bytes()?;
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent)?;
                }
                create_salt(path, &salt)?;
                Ok(Self { salt })
            }
            Err(error) => Err(error),
        }
    }

    /// Hash a client identity without storing the raw address.
    #[must_use]
    pub fn digest(&self, client: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.salt);
        hasher.update(client.as_bytes());
        hasher.finalize().into()
    }
}

#[cfg(unix)]
fn create_salt(path: &Path, salt: &[u8; 32]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(salt)
}

#[cfg(not(unix))]
fn create_salt(path: &Path, salt: &[u8; 32]) -> std::io::Result<()> {
    fs::write(path, salt)
}

fn random_bytes() -> std::io::Result<[u8; 32]> {
    let mut bytes = [0_u8; 32];
    getrandom::getrandom(&mut bytes).map_err(std::io::Error::other)?;
    Ok(bytes)
}

/// A parsed trusted-proxy CIDR.
#[derive(Debug, Clone, Copy)]
struct IpNet {
    network: IpAddr,
    prefix: u8,
}

impl IpNet {
    fn parse(raw: &str) -> Result<Self, String> {
        let (address, prefix) = raw.split_once('/').unwrap_or((raw, ""));
        let network: IpAddr = address
            .parse()
            .map_err(|_| format!("invalid IP address in {raw}"))?;
        let max = match network {
            IpAddr::V4(_) => 32,
            IpAddr::V6(_) => 128,
        };
        let prefix = if prefix.is_empty() {
            max
        } else {
            prefix
                .parse::<u8>()
                .map_err(|_| format!("invalid prefix in {raw}"))?
        };
        if prefix > max {
            return Err(format!("prefix in {raw} is larger than {max}"));
        }
        Ok(Self { network, prefix })
    }

    fn contains(&self, address: IpAddr) -> bool {
        match (self.network, address) {
            (IpAddr::V4(network), IpAddr::V4(address)) => {
                let mask = if self.prefix == 0 {
                    0
                } else {
                    u32::MAX << (32 - self.prefix)
                };
                (u32::from(network) & mask) == (u32::from(address) & mask)
            }
            (IpAddr::V6(network), IpAddr::V6(address)) => {
                let mask = if self.prefix == 0 {
                    0
                } else {
                    u128::MAX << (128 - self.prefix)
                };
                (u128::from(network) & mask) == (u128::from(address) & mask)
            }
            _ => false,
        }
    }
}

/// CIDRs allowed to supply `X-Forwarded-For` and `X-Real-IP`.
#[derive(Debug, Clone, Default)]
pub struct TrustedProxies {
    networks: Vec<IpNet>,
}

impl TrustedProxies {
    pub fn parse(values: &[String]) -> Result<Self, String> {
        let networks = values
            .iter()
            .map(|value| IpNet::parse(value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { networks })
    }

    #[must_use]
    pub fn contains(&self, address: IpAddr) -> bool {
        self.networks
            .iter()
            .any(|network| network.contains(address))
    }
}

/// Build the canonical client identity used for buckets and quotas.
///
/// Forwarded headers are only trusted when the directly connected peer is in
/// `trusted_proxies`. IPv6 clients are grouped by `ipv6_prefix` (`/64` by
/// default). An unknown caller shares the `unknown` bucket.
#[must_use]
pub fn client_identity(
    headers: &HeaderMap,
    peer: Option<IpAddr>,
    trusted: &TrustedProxies,
    ipv6_prefix: u8,
) -> String {
    let hops = forwarded_hops(headers);
    let client = match peer {
        Some(peer) if !trusted.contains(peer) => Some(peer),
        Some(peer) => first_untrusted_hop(&hops, peer, trusted),
        None => hops.first().copied(),
    };
    client.map_or_else(
        || "unknown".to_string(),
        |address| normalize_ip(address, ipv6_prefix),
    )
}

fn first_untrusted_hop(hops: &[IpAddr], peer: IpAddr, trusted: &TrustedProxies) -> Option<IpAddr> {
    let mut current = peer;
    for hop in hops.iter().rev() {
        current = *hop;
        if !trusted.contains(current) {
            return Some(current);
        }
    }
    Some(current)
}

fn forwarded_hops(headers: &HeaderMap) -> Vec<IpAddr> {
    let mut hops = Vec::new();
    for value in headers.get_all("x-forwarded-for").iter() {
        if let Ok(value) = value.to_str() {
            for part in value.split(',') {
                if let Some(address) = parse_hop(part) {
                    hops.push(address);
                }
            }
        }
    }
    if hops.is_empty() {
        if let Some(value) = headers
            .get("x-real-ip")
            .and_then(|value| value.to_str().ok())
        {
            if let Some(address) = parse_hop(value) {
                hops.push(address);
            }
        }
    }
    hops
}

fn parse_hop(raw: &str) -> Option<IpAddr> {
    let raw = raw.trim();
    if let Ok(address) = raw.parse() {
        return Some(address);
    }
    if let Some(rest) = raw.strip_prefix('[') {
        return rest
            .split_once(']')
            .and_then(|(address, _)| address.parse().ok());
    }
    // IPv4 with a port, for example 203.0.113.7:41234.
    raw.rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|character| character.is_ascii_digit()))
        .and_then(|(address, _)| address.parse().ok())
}

fn normalize_ip(address: IpAddr, ipv6_prefix: u8) -> String {
    match address {
        IpAddr::V4(address) => address.to_string(),
        IpAddr::V6(address) => {
            let prefix = ipv6_prefix.min(128);
            let mask = if prefix == 0 {
                0
            } else {
                u128::MAX << (128 - prefix)
            };
            let network = Ipv6Addr::from(u128::from(address) & mask);
            format!("{network}/{prefix}")
        }
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

#[cfg(test)]
mod tests {
    use axum::http::{HeaderMap, HeaderValue};

    use super::{client_identity, ClientHasher, RateLimiter, TrustedProxies};

    fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for (name, value) in pairs {
            headers.append(*name, HeaderValue::from_str(value).unwrap());
        }
        headers
    }

    fn trusted() -> TrustedProxies {
        TrustedProxies::parse(&["127.0.0.1/32".to_string(), "::1/128".to_string()]).unwrap()
    }

    #[test]
    fn consumes_burst_then_limits() {
        let limiter = RateLimiter::new(100, 2);
        assert!(limiter.check("a").is_ok());
        assert!(limiter.check("a").is_ok());
        assert!(limiter.check("a").is_err());
        assert!(limiter.check("b").is_ok());
    }

    #[test]
    fn a_trusted_proxy_supplies_the_client_address() {
        let headers = headers(&[("x-forwarded-for", "203.0.113.9")]);
        let client = client_identity(&headers, Some("127.0.0.1".parse().unwrap()), &trusted(), 64);
        assert_eq!(client, "203.0.113.9");
    }

    #[test]
    fn an_untrusted_peer_cannot_spoof_forwarded_headers() {
        let headers = headers(&[("x-forwarded-for", "203.0.113.9")]);
        let client = client_identity(
            &headers,
            Some("198.51.100.4".parse().unwrap()),
            &trusted(),
            64,
        );
        assert_eq!(client, "198.51.100.4");
    }

    #[test]
    fn spoofed_leftmost_hops_are_ignored_behind_a_proxy() {
        let headers = headers(&[("x-forwarded-for", "192.0.2.99, 203.0.113.9")]);
        let client = client_identity(&headers, Some("127.0.0.1".parse().unwrap()), &trusted(), 64);
        assert_eq!(client, "203.0.113.9");
    }

    #[test]
    fn ipv6_clients_are_grouped_by_prefix() {
        let headers = headers(&[("x-forwarded-for", "2001:db8:1:2::1, 2001:db8:1:2::99")]);
        let client = client_identity(&headers, Some("127.0.0.1".parse().unwrap()), &trusted(), 64);
        assert_eq!(client, "2001:db8:1:2::/64");
    }

    #[test]
    fn bracket_and_port_forms_are_parsed() {
        let headers = headers(&[("x-forwarded-for", "[2001:db8::5]:443")]);
        let client = client_identity(
            &headers,
            Some("127.0.0.1".parse().unwrap()),
            &trusted(),
            128,
        );
        assert_eq!(client, "2001:db8::5/128");
    }

    #[test]
    fn a_hasher_is_stable_and_salted() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("hash.salt");
        let path = path.to_str().unwrap();
        let first = ClientHasher::load_or_create(path).unwrap();
        let second = ClientHasher::load_or_create(path).unwrap();
        assert_eq!(first.digest("203.0.113.9"), second.digest("203.0.113.9"));
        assert_ne!(first.digest("203.0.113.9"), first.digest("203.0.113.10"));
    }

    #[test]
    fn ipv4_networks_do_not_match_ipv6_addresses() {
        let network = super::IpNet::parse("127.0.0.1/32").unwrap();
        assert!(network.contains("127.0.0.1".parse().unwrap()));
        assert!(!network.contains("::1".parse().unwrap()));
    }
}
