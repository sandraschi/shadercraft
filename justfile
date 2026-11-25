set shell := ["powershell.exe", "-NoProfile", "-Command"]

default:
    @just --list

test:
    cargo test --workspace

clippy:
    cargo clippy --workspace --all-targets -- -D warnings

smoke:
    ./start.ps1 -Smoke

floor:
    ./scripts/craft-floor.ps1
