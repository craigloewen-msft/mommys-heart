# Dev container for the Mommy's Heart CRM (Leptos + Axum fullstack, Rust).
#
# Base image ships a ready-to-use Rust toolchain (rustup, cargo, clippy,
# rustfmt), a non-root `vscode` user with passwordless sudo, git, and the
# common C build tooling that some crates need.
FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# The client bundle is compiled to WebAssembly, so we need the wasm target.
# Make sure the formatter/linter components are present too.
RUN rustup target add wasm32-unknown-unknown \
    && rustup component add rustfmt clippy

# Install cargo-binstall (fetches prebuilt binaries instead of compiling from
# source) and use it to install cargo-leptos, which drives the whole
# build/watch/serve workflow and auto-downloads the standalone Tailwind binary.
# CARGO_HOME is /usr/local/cargo in the base image, so these land on the PATH
# for every user.
RUN curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y cargo-leptos
