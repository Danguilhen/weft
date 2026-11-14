// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exercise the real `weft-decode` binary, not just the library.

use std::process::Command;

#[test]
fn worker_decodes_a_png_and_writes_a_thumbnail() {
    let directory = tempfile::tempdir().unwrap();
    let input = directory.path().join("input.png");
    let thumbnail = directory.path().join("thumb.jpg");
    image::RgbImage::from_pixel(12, 9, image::Rgb([20, 40, 60]))
        .save(&input)
        .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_weft-decode"))
        .arg("--input")
        .arg(&input)
        .arg("--thumbnail")
        .arg(&thumbnail)
        .arg("--max-bytes")
        .arg("1048576")
        .arg("--max-pixels")
        .arg("16000000")
        .arg("--max-frames")
        .arg("300")
        .arg("--max-decoded-bytes")
        .arg("268435456")
        .output()
        .expect("run weft-decode");
    assert!(
        output.status.success(),
        "worker failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let decoded: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(decoded["mime"], "image/png");
    assert_eq!(decoded["width"], 12);
    assert_eq!(decoded["height"], 9);
    assert!(thumbnail.exists());
}
