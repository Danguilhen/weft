// SPDX-License-Identifier: MIT

//! Node key loading and Ed25519 signatures over canonical JSON.

use std::fs;
use std::path::{Path, PathBuf};

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use serde::Serialize;
use serde_json::Value;
use thiserror::Error;

/// A JSON signature object used in manifests and federation documents.
#[derive(Debug, Clone, Serialize)]
pub struct Signature {
    pub alg: &'static str,
    #[serde(rename = "keyId")]
    pub key_id: String,
    pub value: String,
}

/// Loads or creates the node's Ed25519 key.
#[derive(Debug)]
pub struct NodeSigner {
    key: SigningKey,
    key_id: String,
}

impl NodeSigner {
    /// Load a base64-encoded 32-byte secret key, creating it when absent.
    ///
    /// A newly created key file is written with mode `0600` on Unix.
    pub fn load_or_create(
        path: impl AsRef<Path>,
        key_id: impl Into<String>,
    ) -> Result<Self, SigningError> {
        let path = path.as_ref();
        let secret = if path.exists() {
            let encoded = fs::read_to_string(path)?;
            base64::engine::general_purpose::STANDARD
                .decode(encoded.trim())
                .map_err(|_| SigningError::InvalidKeyFile)?
        } else {
            if let Some(parent) = path.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent)?;
                }
            }
            let mut bytes = [0_u8; 32];
            getrandom::getrandom(&mut bytes)
                .map_err(|error| SigningError::Random(error.to_string()))?;
            fs::write(
                path,
                base64::engine::general_purpose::STANDARD.encode(bytes),
            )?;
            restrict_permissions(path)?;
            bytes.to_vec()
        };

        let bytes: [u8; 32] = secret
            .try_into()
            .map_err(|_| SigningError::InvalidKeyFile)?;
        Ok(Self {
            key: SigningKey::from_bytes(&bytes),
            key_id: key_id.into(),
        })
    }

    /// The public key encoded as base64, for node descriptors.
    #[must_use]
    pub fn public_key(&self) -> String {
        base64::engine::general_purpose::STANDARD.encode(self.key.verifying_key().to_bytes())
    }

    /// Sign a JSON value with canonical JSON.
    pub fn sign_value(&self, value: &Value) -> Result<Signature, SigningError> {
        let canonical = canonical_json(value);
        let signature = self.key.sign(canonical.as_bytes());
        Ok(Signature {
            alg: "Ed25519",
            key_id: self.key_id.clone(),
            value: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes()),
        })
    }

    /// Sign a byte slice, used for tests and future receipts.
    #[must_use]
    pub fn sign_bytes(&self, bytes: &[u8]) -> String {
        let signature = self.key.sign(bytes);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(signature.to_bytes())
    }
}

/// Encode a JSON value as RFC 8785-style canonical JSON.
///
/// Weft's objects use strings, numbers, booleans, nulls and arrays; this
/// implementation sorts object keys and emits no insignificant whitespace.
#[must_use]
pub fn canonical_json(value: &Value) -> String {
    let mut output = String::new();
    write_value(value, &mut output);
    output
}

fn write_value(value: &Value, output: &mut String) {
    match value {
        Value::Null => output.push_str("null"),
        Value::Bool(boolean) => output.push_str(if *boolean { "true" } else { "false" }),
        Value::Number(number) => output.push_str(&number.to_string()),
        Value::String(string) => {
            output.push_str(&serde_json::to_string(string).expect("string serialization"));
        }
        Value::Array(values) => {
            output.push('[');
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_value(value, output);
            }
            output.push(']');
        }
        Value::Object(object) => {
            output.push('{');
            let mut keys: Vec<_> = object.keys().collect();
            keys.sort();
            for (index, key) in keys.into_iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                output.push_str(&serde_json::to_string(key).expect("key serialization"));
                output.push(':');
                if let Some(value) = object.get(key) {
                    write_value(value, output);
                }
            }
            output.push('}');
        }
    }
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), SigningError> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), SigningError> {
    Ok(())
}

#[derive(Debug, Error)]
pub enum SigningError {
    #[error("signing I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("node key file is not a base64-encoded 32-byte Ed25519 secret")]
    InvalidKeyFile,
    #[error("could not generate key material: {0}")]
    Random(String),
}

impl NodeSigner {
    /// The configured key file path, for diagnostics.
    #[must_use]
    pub fn path_hint(path: &Path) -> PathBuf {
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::{canonical_json, NodeSigner};
    use serde_json::json;

    #[test]
    fn canonical_json_sorts_keys() {
        let value = json!({"b": 2, "a": [true, null, "x"]});
        assert_eq!(canonical_json(&value), r#"{"a":[true,null,"x"],"b":2}"#);
    }

    #[test]
    fn creates_and_signs_with_a_key_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("node.key");
        let signer = NodeSigner::load_or_create(&path, "https://example/actor#key-1").unwrap();
        let signature = signer.sign_value(&json!({"b": 2, "a": 1})).unwrap();
        assert_eq!(signature.alg, "Ed25519");
        assert!(!signature.value.is_empty());
        assert_eq!(signature.key_id, "https://example/actor#key-1");
        assert!(path.exists());
    }
}
