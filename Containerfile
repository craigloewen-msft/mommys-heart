# syntax=docker/dockerfile:1

# ---- Build stage: compile the server binary + wasm client + CSS ----
FROM rust:1-bookworm AS build

# cargo-leptos drives the whole build; the client bundle needs the wasm target.
RUN rustup target add wasm32-unknown-unknown \
    && curl -L --proto '=https' --tlsv1.2 -sSf \
        https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash \
    && cargo binstall -y cargo-leptos

WORKDIR /app
COPY . .
RUN cargo leptos build --release

# ---- Runtime stage: minimal image with just the binary and its assets ----
FROM debian:bookworm-slim AS runtime

# ca-certificates lets reqwest reach Azure OpenAI over HTTPS.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=build /app/target/release/mommys-heart-crm /app/mommys-heart-crm
COPY --from=build /app/target/site /app/site
COPY --from=build /app/docs /app/docs

# Leptos runtime configuration (mirrors [package.metadata.leptos]).
ENV LEPTOS_OUTPUT_NAME=mommys-heart-crm \
    LEPTOS_SITE_ROOT=site \
    LEPTOS_SITE_PKG_DIR=pkg \
    LEPTOS_SITE_ADDR=0.0.0.0:3000 \
    LEPTOS_ENV=PROD

EXPOSE 3000
CMD ["/app/mommys-heart-crm"]
