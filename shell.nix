{ pkgs ? import <nixpkgs> {} }:

pkgs.mkShell {
  nativeBuildInputs = with pkgs; [
    gnumake
    cargo
    rustc
    rust-analyzer
    clippy
    rustfmt
    sccache
    docker
    docker-compose
    python3
    python3Packages.pip
    maturin
  ];
}
