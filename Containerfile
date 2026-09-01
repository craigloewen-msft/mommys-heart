# syntax=docker/dockerfile:1

# ---- Build stage: compile the server binary + wasm client + CSS ----
FROM rust:1-bookworm AS build

# Network flakiness on a CI runner should retry, not fail the whole build.
ENV CARGO_NET_RETRY=5 \
    CARGO_TERM_COLOR=never \
    CARGO_INCREMENTAL=0

# cargo-leptos drives the whole build; the client bundle needs the wasm target.
# NOTE: consider pinning `cargo-leptos@<version>` here once you've picked one --
# I had no network access to confirm a valid version number, so this stays
# unpinned rather than guessing one that might not exist.
RUN rustup target add wasm32-unknown-unknown \
    && curl -L --proto '=https' --tlsv1.2 -sSf --retry 5 --retry-all-errors \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y --locked cargo-leptos

WORKDIR /app

# Dependency pre-build: a stub crate carrying the real manifest, so the large,
# rarely-changing dependency graph becomes its own cached layer. Editing source
# no longer invalidates it -- only Cargo.toml/Cargo.lock do.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src \
    && echo 'fn main() {}' > src/main.rs \
    && touch src/lib.rs \
    && cargo build --release --no-default-features --features ssr \
    && cargo build --release --lib --no-default-features --features hydrate \
        --target wasm32-unknown-unknown \
    && rm -rf src

# Real build. Drop only this crate's stub artifacts; dependencies above are kept.
COPY . .
RUN cargo clean --release -p mommys-heart-app \
    && cargo clean --release -p mommys-heart-app --target wasm32-unknown-unknown \
    && cargo leptos build --release

# ---- Runtime stage: minimal image with just the binary and its assets ----
FROM debian:bookworm-slim AS runtime

# ca-certificates lets reqwest reach Azure OpenAI over HTTPS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /app/target/release/mommys-heart-app /app/mommys-heart-app
COPY --from=build /app/target/site /app/site
COPY --from=build /app/docs /app/docs

# Leptos runtime configuration (mirrors [package.metadata.leptos]).
ENV LEPTOS_OUTPUT_NAME=mommys-heart-app \
    LEPTOS_SITE_ROOT=site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:3000 \
    LEPTOS_ENV=PROD

EXPOSE 3000
CMD ["/app/mommys-heart-app"]
