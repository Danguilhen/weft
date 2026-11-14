// SPDX-License-Identifier: MIT

//! Small size parser for configuration values such as `256MB`.

/// Parse a byte size with optional B, KB/KiB, MB/MiB or GB/GiB suffix.
#[must_use]
pub fn parse_size(value: &str) -> Option<u64> {
    let value = value.trim();
    let split = value
        .find(|character: char| !character.is_ascii_digit() && character != '.')
        .unwrap_or(value.len());
    let number: f64 = value[..split].parse().ok()?;
    let unit = value[split..].trim().to_ascii_uppercase();
    let multiplier = match unit.as_str() {
        "" | "B" => 1.0,
        "KB" | "K" => 1_000.0,
        "KIB" => 1024.0,
        "MB" | "M" => 1_000_000.0,
        "MIB" => 1024.0 * 1024.0,
        "GB" | "G" => 1_000_000_000.0,
        "GIB" => 1024.0 * 1024.0 * 1024.0,
        _ => return None,
    };
    Some((number * multiplier) as u64)
}

#[cfg(test)]
mod tests {
    use super::parse_size;

    #[test]
    fn parses_size_units() {
        assert_eq!(parse_size("256MB"), Some(256_000_000));
        assert_eq!(parse_size("256MiB"), Some(268_435_456));
        assert_eq!(parse_size("20GB"), Some(20_000_000_000));
        assert_eq!(parse_size("1024"), Some(1024));
        assert_eq!(parse_size("nope"), None);
    }
}
