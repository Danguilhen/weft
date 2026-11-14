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
    let media = decode_file(&input, &thumbnail, limits)?;
    println!("{}", serde_json::to_string(&media)?);
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
