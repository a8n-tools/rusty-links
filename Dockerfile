# Build mode: "standalone" (default) or "saas"
ARG BUILD_MODE=standalone

# Build stage (Debian-based for pre-built dioxus-cli binary)
FROM rust:1.93-bookworm AS builder

ARG BUILD_MODE

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install --yes --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Install dioxus-cli from pre-built binary (seconds instead of minutes)
RUN cargo binstall dioxus-cli --no-confirm

# Install WASM target for client build
RUN rustup target add wasm32-unknown-unknown

# Resolve build flags from BUILD_MODE
RUN if [ "$BUILD_MODE" = "saas" ]; then \
      echo "--no-default-features --features saas,server" > /tmp/cargo-features; \
      echo "--no-default-features --features saas --bin rusty-links-saas" > /tmp/dx-flags; \
    else \
      echo "--features standalone,server" > /tmp/cargo-features; \
      echo "--features standalone" > /tmp/dx-flags; \
    fi

# --- Dependency caching (server-side only) ---
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release $(cat /tmp/cargo-features) \
    && rm -rf src target/release/deps/rusty_links*

# --- Full build with dioxus ---
# Copy source code (respects .dockerignore)
COPY . .

# Build fullstack app (server binary + WASM client bundle)
RUN dx build --release $(cat /tmp/dx-flags)

# Create consistent output path regardless of build mode
RUN if [ "$BUILD_MODE" = "saas" ]; then \
      ln -s /build/target/dx/rusty-links-saas/release/web /build/dx-output; \
    else \
      ln -s /build/target/dx/rusty-links/release/web /build/dx-output; \
    fi

# Runtime stage (slim Debian since the binary links against glibc)
FROM debian:bookworm-slim

ARG BUILD_MODE

# Install runtime dependencies
RUN apt-get update && apt-get install --yes --no-install-recommends \
    ca-certificates tzdata curl \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd --create-home --uid 1001 appuser

# Create standard directory structure:
#   /app    — application binary, static assets, and WASM client (read-only)
#   /data   — persistent application data (Docker volume)
#   /config — application configuration (Docker volume)
RUN mkdir -p /app/assets /data /config

WORKDIR /app

# Copy dx build output (server binary + public/ with index.html and WASM/JS)
# Note: migrations/ are embedded at compile time by sqlx::migrate!() and not needed at runtime
COPY --from=builder /build/dx-output/ /app/

# Copy WASM/JS assets to /app/assets/ for the ServeDir("/assets") route in main.rs
RUN cp /app/public/assets/* /app/assets/ 2>/dev/null || true

# Copy favicon (not processed by dx asset pipeline since it's not referenced via asset!())
COPY --from=builder /build/assets/favicon.ico /app/assets/

# Normalize binary name for consistent ENTRYPOINT
RUN if [ "$BUILD_MODE" = "saas" ]; then mv /app/rusty-links-saas /app/rusty-links; fi

# Set ownership of all standard directories
RUN chown -R appuser:appuser /app /data /config

USER appuser

LABEL org.opencontainers.image.source=https://dev.a8n.run/a8n-tools/rusty-links
LABEL org.opencontainers.image.description="rusty-links (${BUILD_MODE})"

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

ENTRYPOINT ["/app/rusty-links"]
