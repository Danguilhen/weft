// SPDX-License-Identifier: MIT

//! TOML configuration, defaults and environment overrides.

use std::env;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

/// The complete node configuration for the MVP.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub node: NodeConfig,
    pub server: ServerConfig,
    pub database: DatabaseConfig,
    pub storage: StorageConfig,
    pub upload: UploadConfig,
    pub decode: DecodeConfig,
    pub logging: LoggingConfig,
    pub metrics: MetricsConfig,
    pub privacy: PrivacyConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NodeConfig {
    pub public_url: String,
    pub name: String,
    pub description: String,
    pub contact: String,
    pub key_file: String,
    pub actor_url: String,
}

impl NodeConfig {
    /// The actor URL, defaulting to `<public_url>/actor`.
    #[must_use]
    pub fn actor_url(&self) -> String {
        if !self.actor_url.trim().is_empty() {
            return self.actor_url.trim_end_matches('/').to_string();
        }
        format!("{}/actor", self.public_url.trim_end_matches('/'))
    }
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            public_url: "http://127.0.0.1:8080".to_string(),
            name: "Weft node".to_string(),
            description: String::new(),
            contact: String::new(),
            key_file: "node.key".to_string(),
            actor_url: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ServerConfig {
    pub bind: String,
    pub external_url: String,
    pub max_json_bytes: usize,
    pub request_timeout_seconds: u64,
    pub trusted_proxies: Vec<String>,
    /// Bearer token for the admin API. Empty disables admin access.
    pub admin_token: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:8080".to_string(),
            external_url: "http://127.0.0.1:8080".to_string(),
            max_json_bytes: 1_048_576,
            request_timeout_seconds: 30,
            trusted_proxies: vec!["127.0.0.1/32".to_string(), "::1/128".to_string()],
            admin_token: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
    pub connect_timeout_seconds: u64,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            url: "postgres://weft:weft@127.0.0.1:5432/weft".to_string(),
            max_connections: 10,
            connect_timeout_seconds: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct StorageConfig {
    pub backend: String,
    pub path: String,
    pub max_size: String,
    pub max_file_size: String,
    pub thumbnail_size: u32,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            backend: "local".to_string(),
            path: "./data".to_string(),
            max_size: "20GB".to_string(),
            max_file_size: "15MB".to_string(),
            thumbnail_size: 512,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UploadConfig {
    pub mode: String,
    pub allowed_mimes: Vec<String>,
    pub max_pixels: u64,
    pub max_frames: u32,
    pub max_decoded_memory: String,
    pub max_concurrent_decodes: u32,
    pub decode_timeout_seconds: u64,
    pub max_title_chars: u32,
    pub max_description_chars: u32,
    pub max_tags: u32,
    pub max_tag_chars: u32,
    pub rate_limit_per_hour: u32,
    pub rate_limit_burst: u32,
    pub max_pending_per_client: u32,
    pub max_daily_bytes: String,
    pub rate_limit_ipv6_prefix: u8,
    pub proof_of_work: bool,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            mode: "review".to_string(),
            allowed_mimes: vec![
                "image/gif".to_string(),
                "image/png".to_string(),
                "image/jpeg".to_string(),
                "image/webp".to_string(),
            ],
            max_pixels: 16_000_000,
            max_frames: 300,
            max_decoded_memory: "256MB".to_string(),
            max_concurrent_decodes: 2,
            decode_timeout_seconds: 10,
            max_title_chars: 200,
            max_description_chars: 2000,
            max_tags: 20,
            max_tag_chars: 40,
            rate_limit_per_hour: 20,
            rate_limit_burst: 5,
            max_pending_per_client: 10,
            max_daily_bytes: "100MB".to_string(),
            rate_limit_ipv6_prefix: 64,
            proof_of_work: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DecodeConfig {
    /// Path to `weft-decode`; empty means the sibling binary.
    pub worker_path: String,
    pub timeout_seconds: u64,
}

impl Default for DecodeConfig {
    fn default() -> Self {
        Self {
            worker_path: String::new(),
            timeout_seconds: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            format: "json".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MetricsConfig {
    pub enabled: bool,
    pub bind: String,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            bind: "127.0.0.1:9090".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PrivacyConfig {
    pub hash_salt_file: String,
    pub client_hash_retention_days: u32,
    pub report_retention_days: u32,
}

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            hash_salt_file: "hash.salt".to_string(),
            client_hash_retention_days: 7,
            report_retention_days: 90,
        }
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("invalid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid configuration value: {0}")]
    Invalid(String),
}

impl AppConfig {
    /// Load a TOML file and apply `WEFT__SECTION__KEY` environment overrides.
    ///
    /// Relative storage and key paths are resolved against the directory that
    /// contains the configuration file, not the process working directory.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let text = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;
        let mut config = Self::from_toml(&text)?;
        let base = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."));
        config.resolve_relative_paths(base);
        config.validate()?;
        Ok(config)
    }

    /// Make relative storage and key paths absolute against `base`.
    pub fn resolve_relative_paths(&mut self, base: &Path) {
        if !self.storage.path.trim().is_empty() && Path::new(&self.storage.path).is_relative() {
            self.storage.path = base.join(&self.storage.path).to_string_lossy().into_owned();
        }
        if !self.node.key_file.trim().is_empty() && Path::new(&self.node.key_file).is_relative() {
            self.node.key_file = base
                .join(&self.node.key_file)
                .to_string_lossy()
                .into_owned();
        }
    }

    /// Parse a TOML string and apply environment overrides.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        let toml_value: toml::Value = toml::from_str(text)?;
        let mut value = serde_json::to_value(toml_value)
            .map_err(|error| ConfigError::Invalid(error.to_string()))?;
        apply_env(&mut value)?;
        let config: Self = serde_json::from_value(value)
            .map_err(|error| ConfigError::Invalid(error.to_string()))?;
        config.validate()?;
        Ok(config)
    }

    /// Serialize the effective configuration to TOML.
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        toml::to_string_pretty(self).map_err(|error| ConfigError::Invalid(error.to_string()))
    }

    /// Reject values that would make the node unsafe or unroutable.
    pub fn validate(&self) -> Result<(), ConfigError> {
        validate_origin(&self.node.public_url, "node.public_url")?;
        validate_origin(&self.node.actor_url(), "node.actor_url")?;
        validate_origin(&self.server.external_url, "server.external_url")?;

        self.server
            .bind
            .parse::<SocketAddr>()
            .map_err(|_| ConfigError::Invalid("server.bind must be a socket address".into()))?;

        if self.database.url.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "database.url must not be empty".into(),
            ));
        }
        if self.storage.backend != "local" {
            return Err(ConfigError::Invalid(
                "only storage.backend = \"local\" is implemented in Milestone 0".into(),
            ));
        }
        if self.storage.path.trim().is_empty() {
            return Err(ConfigError::Invalid(
                "storage.path must not be empty".into(),
            ));
        }
        if !matches!(self.upload.mode.as_str(), "open" | "review" | "closed") {
            return Err(ConfigError::Invalid(
                "upload.mode must be open, review or closed".into(),
            ));
        }
        if self.upload.allowed_mimes.is_empty() {
            return Err(ConfigError::Invalid(
                "upload.allowed_mimes must not be empty".into(),
            ));
        }
        if self.upload.max_pixels == 0 || self.upload.max_frames == 0 {
            return Err(ConfigError::Invalid(
                "upload limits must be greater than zero".into(),
            ));
        }
        if !matches!(self.logging.format.as_str(), "json" | "pretty") {
            return Err(ConfigError::Invalid(
                "logging.format must be json or pretty".into(),
            ));
        }
        Ok(())
    }
}

fn validate_origin(value: &str, field: &str) -> Result<(), ConfigError> {
    let url =
        Url::parse(value).map_err(|error| ConfigError::Invalid(format!("{field}: {error}")))?;
    match url.scheme() {
        "https" => Ok(()),
        "http" if is_loopback(&url) => Ok(()),
        scheme => Err(ConfigError::Invalid(format!(
            "{field} must use https outside a loopback address (got {scheme})"
        ))),
    }
}

fn is_loopback(url: &Url) -> bool {
    url.host_str()
        .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"))
}

fn apply_env(value: &mut serde_json::Value) -> Result<(), ConfigError> {
    for (key, raw) in env::vars() {
        let Some(rest) = key.strip_prefix("WEFT__") else {
            continue;
        };
        let path: Vec<String> = rest
            .split("__")
            .filter(|part| !part.is_empty())
            .map(str::to_ascii_lowercase)
            .collect();
        if path.is_empty() {
            continue;
        }
        let parsed = parse_env_value(&raw);
        set_json(value, &path, parsed)?;
    }
    Ok(())
}

fn parse_env_value(raw: &str) -> serde_json::Value {
    // TOML parses an empty document as an empty table; an empty override (for
    // example an unset admin token) must stay an empty string.
    if raw.trim().is_empty() {
        return serde_json::Value::String(String::new());
    }
    if let Ok(value) = toml::from_str::<toml::Value>(raw) {
        if let Ok(value) = serde_json::to_value(value) {
            return value;
        }
    }
    serde_json::Value::String(raw.to_string())
}

fn set_json(
    root: &mut serde_json::Value,
    path: &[String],
    value: serde_json::Value,
) -> Result<(), ConfigError> {
    let mut current = root;
    for segment in &path[..path.len() - 1] {
        let object = current.as_object_mut().ok_or_else(|| {
            ConfigError::Invalid(format!("environment path {segment} is not an object"))
        })?;
        current = object
            .entry(segment.clone())
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    }
    let object = current
        .as_object_mut()
        .ok_or_else(|| ConfigError::Invalid("environment path is not an object".into()))?;
    object.insert(path[path.len() - 1].clone(), value);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{AppConfig, ConfigError};

    #[test]
    fn empty_env_override_is_an_empty_string() {
        assert_eq!(
            super::parse_env_value(""),
            serde_json::Value::String(String::new())
        );
        assert_eq!(
            super::parse_env_value("   "),
            serde_json::Value::String(String::new())
        );
    }

    #[test]
    fn defaults_are_loopback_and_safe() {
        let config = AppConfig::default();
        assert_eq!(config.upload.mode, "review");
        assert_eq!(config.database.max_connections, 10);
        assert_eq!(config.node.actor_url(), "http://127.0.0.1:8080/actor");
        config.validate().unwrap();
    }

    #[test]
    fn parses_partial_toml_with_defaults() {
        let config = AppConfig::from_toml(
            r#"
            [node]
            public_url = "https://gifs.example"
            name = "Test"
            [upload]
            mode = "open"
            "#,
        )
        .unwrap();
        assert_eq!(config.node.name, "Test");
        assert_eq!(config.upload.mode, "open");
        assert_eq!(config.upload.max_frames, 300);
    }

    #[test]
    fn rejects_plain_http_outside_loopback() {
        let error = AppConfig::from_toml(
            r#"
            [node]
            public_url = "http://gifs.example"
            [server]
            external_url = "http://gifs.example"
            "#,
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::Invalid(_)));
    }

    #[test]
    fn resolves_paths_against_the_config_directory() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = AppConfig::from_toml(
            r#"
            [node]
            key_file = "node.key"
            [storage]
            path = "data"
            "#,
        )
        .unwrap();
        config.resolve_relative_paths(directory.path());
        assert!(std::path::Path::new(&config.node.key_file).is_absolute());
        assert!(std::path::Path::new(&config.storage.path).is_absolute());
        assert!(config.node.key_file.ends_with("node.key"));
        assert!(config.storage.path.ends_with("data"));
    }

    #[test]
    fn rejects_unknown_upload_mode() {
        let error = AppConfig::from_toml(
            r#"
            [upload]
            mode = "anything"
            "#,
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::Invalid(_)));
    }
}
