{ pkgs ? import <nixpkgs> { } }:

pkgs.mkShell {
  packages = with pkgs; [
    rustc
    cargo
    rustfmt
    clippy
    python3
    postgresql_16
    curl
    jq
    pkg-config
  ];
}
