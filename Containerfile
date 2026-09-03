# syntax=docker/dockerfile:1

# ---- Build stage: compile the server binary + wasm client + CSS ----
FROM rust:1-bookworm AS build

# Network flakiness on a CI runner should retry, not fail the whole build.
#
# RUSTFLAGS: Leptos erases `view!` component types instead of monomorphising
# them. Deep view trees otherwise generate enormous concrete types -- measured
# on this crate, the release wasm build costs 7m04s of CPU without the flag and
# 3m56s with it, and it is also what local builds use, so CI now matches.
# It must be set before the dependency pre-build below: changing RUSTFLAGS
# afterwards would invalidate every cached dependency artifact.
ENV CARGO_NET_RETRY=5 \
    CARGO_TERM_COLOR=never \
    CARGO_INCREMENTAL=0 \
    RUSTFLAGS="--cfg erase_components"

WORKDIR /app

# Toolchain layer. rust-toolchain.toml is copied on its own so rustup resolves
# and downloads the channel exactly once, into a layer that source edits do not
# invalidate. Without it the channel is synced during the source build instead.
COPY rust-toolchain.toml ./

# cargo-leptos drives the whole build; the client bundle needs the wasm target.
RUN rustup target add wasm32-unknown-unknown \
    && curl -L --proto '=https' --tlsv1.2 -sSf --retry 5 --retry-all-errors \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y --locked cargo-leptos@0.3.7

# Dependency pre-build: a stub crate carrying the real manifest, so the large,
# rarely-changing dependency graph becomes its own cached layer. Editing source
# no longer invalidates it -- only Cargo.toml/Cargo.lock do.
#
# This runs `cargo leptos build` rather than two hand-written `cargo build`s so
# the layer matches the real build exactly. That matters: cargo-leptos compiles
# the client with `--target-dir target/front`, so wasm artifacts left in the
# default `target/wasm32-unknown-unknown` are never looked at and the whole
# client dependency graph is rebuilt from scratch. Measured with cargo-leptos
# 0.3.7, the wrong directory made the source build compile 194 crates; this
# makes it compile 2 (this crate, once per side).
#
# It also pre-seeds the tailwindcss, wasm-bindgen and wasm-opt binaries that
# cargo-leptos fetches on demand. Those downloads otherwise happen in the final,
# uncached layer, where a transient network failure discards the entire build.
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src style \
    && echo 'fn main() {}' > src/main.rs \
    && touch src/lib.rs \
    && touch style/tailwind.css \
    && cargo leptos build --release \
    && rm -rf src style target/site

# Real build. Drop only this crate's stub artifacts; dependencies above are kept.
COPY . .
RUN cargo clean --release -p mommys-heart-app \
    && cargo clean --release -p mommys-heart-app \
        --target-dir target/front --target wasm32-unknown-unknown \
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
