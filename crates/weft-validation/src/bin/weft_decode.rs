// SPDX-License-Identifier: AGPL-3.0-or-later

//! `weft-decode` — isolated media decoder used by `weft-server`.

use std::path::PathBuf;
use std::process::ExitCode;

use weft_validation::{decode_file, Limits};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("weft-decode: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut input: Option<PathBuf> = None;
    let mut thumbnail: Option<PathBuf> = None;
    let mut limits = Limits::default();

    let mut args = std::env::args().skip(1);
    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => input = Some(args.next().ok_or("missing --input value")?.into()),
            "--thumbnail" => {
                thumbnail = Some(args.next().ok_or("missing --thumbnail value")?.into());
            }
            "--max-bytes" => limits.max_bytes = parse(&mut args, "--max-bytes")?,
            "--max-pixels" => limits.max_pixels = parse(&mut args, "--max-pixels")?,
            "--max-frames" => limits.max_frames = parse(&mut args, "--max-frames")?,
            "--max-decoded-bytes" => {
                limits.max_decoded_bytes = parse(&mut args, "--max-decoded-bytes")?;
            }
            other => return Err(format!("unknown argument {other}").into()),
        }
    }

    let input = input.ok_or("missing --input")?;
    let thumbnail = thumbnail.ok_or("missing --thumbnail")?;
    limit_resources(limits.max_decoded_bytes)?;
    let media = decode_file(&input, &thumbnail, limits)?;
    println!("{}", serde_json::to_string(&media)?);
    Ok(())
}

#[cfg(unix)]
fn limit_resources(max_decoded_bytes: u64) -> Result<(), Box<dyn std::error::Error>> {
    use nix::sys::resource::{setrlimit, Resource};
    let address_space = max_decoded_bytes.saturating_add(256 * 1024 * 1024);
    // Best-effort hardening: rlimits are a second layer behind the decoder and
    // the server timeout.
    let _ = setrlimit(Resource::RLIMIT_AS, address_space, address_space);
    let _ = setrlimit(Resource::RLIMIT_CPU, 30, 30);
    let _ = setrlimit(Resource::RLIMIT_FSIZE, 64 * 1024 * 1024, 64 * 1024 * 1024);
    let _ = setrlimit(Resource::RLIMIT_NOFILE, 64, 64);
    Ok(())
}

#[cfg(not(unix))]
fn limit_resources(_max_decoded_bytes: u64) -> Result<(), Box<dyn std::error::Error>> {
    Ok(())
}

fn parse<T: std::str::FromStr>(
    args: &mut impl Iterator<Item = String>,
    name: &str,
) -> Result<T, Box<dyn std::error::Error>>
where
    T::Err: std::fmt::Display,
{
    args.next()
        .ok_or_else(|| format!("missing {name} value"))?
        .parse()
        .map_err(|error| format!("invalid {name}: {error}").into())
}
