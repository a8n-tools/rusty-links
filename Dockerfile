# Development Dockerfile - mirrors oci-build/Dockerfile approach with debug builds
FROM rust:1-slim-trixie

RUN apt-get update && apt-get install --yes --no-install-recommends \
    pkg-config libssl-dev curl \
    && rm -rf /var/lib/apt/lists/*

# Install WASM target
RUN rustup target add wasm32-unknown-unknown

# Install dioxus-cli via pre-built binary (seconds, not minutes)
RUN curl --location --silent --show-error \
    https://github.com/cargo-bins/cargo-binstall/releases/latest/download/cargo-binstall-x86_64-unknown-linux-gnu.tgz \
    | tar --extract --gzip --directory /usr/local/cargo/bin
RUN cargo binstall dioxus-cli --no-confirm

RUN mkdir -p /app /data /config

WORKDIR /app

# Copy dependency files and migrations (needed by sqlx::migrate! at compile time)
COPY Cargo.toml Cargo.lock ./
COPY migrations/ ./migrations/

# Pre-build dependencies (no source code, no nightly, no DB needed)
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --features standalone,server && \
    rm -rf src

# Source code is mounted via volumes in compose.yml

EXPOSE 8080

# Build full app (WASM + server) then run the server binary produced by dx build
CMD ["sh", "-c", "dx build --features standalone && exec target/dx/rusty-links/debug/web/rusty-links"]
