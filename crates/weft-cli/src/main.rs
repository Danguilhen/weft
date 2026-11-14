// SPDX-License-Identifier: AGPL-3.0-or-later

//! `weft` node command line.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use base64::Engine;
use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;

use weft_core::AppConfig;

mod doctor;

#[derive(Debug, Parser)]
#[command(name = "weft", version, about = "Weft node command line")]
struct Cli {
    /// Path to the TOML configuration.
    #[arg(long, global = true, default_value = "config.toml")]
    config: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Create a configuration file and a node key.
    Init {
        #[arg(long, default_value = "Weft node")]
        name: String,
        #[arg(long = "public-url", default_value = "http://127.0.0.1:8080")]
        public_url: String,
        #[arg(long)]
        force: bool,
    },
    /// Generate a node key and print its public key.
    Keygen {
        #[arg(long, default_value = "node.key")]
        out: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Apply database migrations.
    Migrate,
    /// Check the host against the supported node profile.
    Doctor,
    /// Run the HTTP node until Ctrl-C.
    Serve,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init {
            name,
            public_url,
            force,
        } => init(&cli.config, &name, &public_url, force),
        Command::Keygen { out, force } => {
            let public = generate_key(&out, force)?;
            println!("public key: {public}");
            println!("key file:   {}", out.display());
            Ok(())
        }
        Command::Migrate => migrate(&cli.config).await,
        Command::Doctor => doctor::run(&cli.config),
        Command::Serve => {
            let config = AppConfig::load(&cli.config)
                .with_context(|| format!("loading {}", cli.config.display()))?;
            let report = doctor::check(&config);
            for warning in &report.warnings {
                tracing::warn!(warning, "weft doctor");
            }
            for failure in &report.failures {
                tracing::error!(failure, "weft doctor");
            }
            weft_server::run(config).await?;
            Ok(())
        }
    }
}

fn init(config_path: &Path, name: &str, public_url: &str, force: bool) -> Result<()> {
    if config_path.exists() && !force {
        bail!(
            "{} already exists; pass --force to replace it",
            config_path.display()
        );
    }
    if let Some(parent) = config_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }

    let config_dir = config_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let config_dir = fs::canonicalize(config_dir)
        .with_context(|| format!("resolving {}", config_dir.display()))?;
    let key_path = config_dir.join("node.key");
    if key_path.exists() && !force {
        bail!(
            "{} already exists; pass --force to replace it",
            key_path.display()
        );
    }
    let public = generate_key(&key_path, true)?;

    let mut config = AppConfig::default();
    config.node.name = name.to_string();
    config.node.public_url = public_url.to_string();
    config.node.key_file = key_path.to_string_lossy().to_string();
    config.storage.path = config_dir.join("data").to_string_lossy().to_string();
    config.server.admin_token = random_token()?;

    fs::create_dir_all(&config.storage.path)
        .with_context(|| format!("creating storage path {}", config.storage.path))?;
    fs::write(config_path, config.to_toml()?)
        .with_context(|| format!("writing {}", config_path.display()))?;
    restrict_permissions(config_path)?;

    println!("config:     {}", config_path.display());
    println!("key file:   {}", key_path.display());
    println!("public key: {public}");
    println!("admin token: {}", config.server.admin_token);
    println!(
        "next:       weft migrate --config {}",
        config_path.display()
    );
    Ok(())
}

fn generate_key(path: &Path, force: bool) -> Result<String> {
    if path.exists() && !force {
        bail!(
            "{} already exists; pass --force to replace it",
            path.display()
        );
    }
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
        }
    }

    let mut secret = [0_u8; 32];
    getrandom::getrandom(&mut secret).context("generating random key material")?;
    let signing_key = SigningKey::from_bytes(&secret);
    let public = signing_key.verifying_key().to_bytes();

    fs::write(
        path,
        base64::engine::general_purpose::STANDARD.encode(secret),
    )
    .with_context(|| format!("writing {}", path.display()))?;
    restrict_permissions(path)?;

    Ok(base64::engine::general_purpose::STANDARD.encode(public))
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .with_context(|| format!("restricting permissions on {}", path.display()))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<()> {
    Ok(())
}

fn random_token() -> Result<String> {
    let mut bytes = [0_u8; 32];
    getrandom::getrandom(&mut bytes).context("generating admin token")?;
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes))
}

async fn migrate(config_path: &Path) -> Result<()> {
    let config = AppConfig::load(config_path)
        .with_context(|| format!("loading {}", config_path.display()))?;
    let pool = weft_db::connect(&config.database).await?;
    weft_db::migrate(&pool).await?;
    println!("migrations applied");
    Ok(())
}
