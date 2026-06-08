# Build stage. The base image bundles Rust 1.93 + WASM target + cargo-binstall
# + dioxus-cli (pinned). See niceguyit/oci-images/rust-builder for details.
FROM ghcr.io/niceguyit/rust-builder-glibc:v1.0.2-rust1.94-trixie AS builder

# Build metadata — populated by CI (see .forgejo/workflows/build-oci-image.yml).
# Read by build.rs and embedded into the binary so /api/health and the UI footer
# can report the exact build. Defaults keep local docker builds working.
ARG BUILD_VERSION=unknown
ARG BUILD_GIT_HASH=unknown
ARG BUILD_DATE=unknown
ENV BUILD_VERSION=${BUILD_VERSION}
ENV BUILD_GIT_HASH=${BUILD_GIT_HASH}
ENV BUILD_DATE=${BUILD_DATE}

WORKDIR /build

# A single binary serves both deployment modes; the mode is resolved at runtime
# from OIDC_ISSUER (set = hosted, unset = standalone). There is no longer a
# parallel sibling build, so cargo uses the full CPU count by default.

# --- Dependency caching (server-side only) ---
COPY Cargo.toml Cargo.lock ./

# Guard against a dx/dioxus toolchain mismatch (LINKS-19). The base image's dx
# CLI must match the dioxus crate version resolved in Cargo.lock; dx otherwise
# only prints `dx and dioxus versions are incompatible!` and continues, which can
# silently ship a broken client bundle. Compare the exact patch version: dx 0.7.7
# vs dioxus 0.7.9 share the 0.7 minor yet dx still reports them incompatible, so a
# minor-only check would not catch it. Runs before the dep build so a mismatch
# fails fast instead of after a multi-minute compile.
RUN dx_ver="$(dx --version | grep -oE '[0-9]+\.[0-9]+\.[0-9]+' | head -n1)"; \
    dioxus_ver="$(awk '/^name = "dioxus"$/{getline; gsub(/[",]/, ""); print $3; exit}' Cargo.lock)"; \
    echo "dx=${dx_ver} dioxus=${dioxus_ver}"; \
    if [ "${dx_ver}" != "${dioxus_ver}" ]; then \
        echo "ERROR: dx CLI ${dx_ver} does not match dioxus crate ${dioxus_ver}; bump rust-builder-glibc (see LINKS-19)" >&2; \
        exit 1; \
    fi

RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    cargo build --release --features server \
    && rm -rf src target/release/deps/rusty_links*

# --- Full build with dioxus ---
# Copy source code (respects .dockerignore)
COPY . .

# Build fullstack app (server binary + WASM client bundle). dioxus enables the
# `server` and `web` features automatically for the fullstack platform.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    dx build --release

# Runtime stage (must match builder glibc version)
FROM debian:trixie-slim

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
COPY --from=builder /build/target/dx/rusty-links/release/web/ /app/

# Copy WASM/JS assets to /app/assets/ for the ServeDir("/assets") route in main.rs
RUN cp /app/public/assets/* /app/assets/ 2>/dev/null || true

# Copy favicon (not processed by dx asset pipeline since it's not referenced via asset!())
COPY --from=builder /build/assets/favicon.ico /app/assets/

# Set ownership of all standard directories
RUN chown -R appuser:appuser /app /data /config

USER appuser

LABEL org.opencontainers.image.source=https://dev.a8n.run/a8n-tools/rusty-links
LABEL org.opencontainers.image.description="rusty-links"

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

ENTRYPOINT ["/app/rusty-links"]
