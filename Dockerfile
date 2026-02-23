# Build mode: "standalone" (default) or "saas"
ARG BUILD_MODE=standalone

# Build stage
FROM rust:1.93-alpine AS builder

ARG BUILD_MODE

WORKDIR /build

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

# Resolve cargo features from BUILD_MODE
#   standalone → --features standalone,server
#   saas       → --no-default-features --features saas,server
RUN if [ "$BUILD_MODE" = "saas" ]; then \
      echo "--no-default-features --features saas,server" > /tmp/cargo-features; \
    else \
      echo "--features standalone,server" > /tmp/cargo-features; \
    fi

# Copy cargo files for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy src for dependency compilation
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release $(cat /tmp/cargo-features) && rm -rf src target/release/deps/rusty_links*

# Copy actual source code (exclude .cargo/ which sets a glibc target)
COPY src/ src/
COPY migrations/ migrations/
COPY tailwind.css tailwind.css

# Prepare assets directory for the Dioxus asset!() macro
RUN mkdir -p assets && cp tailwind.css assets/tailwind.css

# Build the application
RUN cargo build --release $(cat /tmp/cargo-features)

# Resolve binary name: "rusty-links" for standalone, "rusty-links-saas" for saas
RUN if [ "$BUILD_MODE" = "saas" ]; then \
      ln -s /build/target/release/rusty-links-saas /build/app-binary; \
    else \
      ln -s /build/target/release/rusty-links /build/app-binary; \
    fi

# Runtime stage
FROM alpine:3.21

ARG BUILD_MODE

# Install runtime dependencies
RUN apk add --no-cache ca-certificates tzdata

# Create non-root user
RUN adduser -D -u 1001 appuser

# Create standard directory structure:
#   /app    — application binary and static assets (read-only)
#   /data   — persistent application data (Docker volume)
#   /config — application configuration (Docker volume)
RUN mkdir -p /app/assets /app/public /data /config

WORKDIR /app

# Copy binary from builder (symlink resolves to the correct binary for the build mode)
# Note: migrations/ are embedded at compile time by sqlx::migrate!() and not needed at runtime
COPY --from=builder /build/app-binary /app/rusty-links

# Set ownership of all standard directories
RUN chown -R appuser:appuser /app /data /config

USER appuser

LABEL org.opencontainers.image.source=https://dev.a8n.run/a8n-tools/rusty-links
LABEL org.opencontainers.image.description="rusty-links (${BUILD_MODE})"

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/bin/sh", "-c", "command -v curl > /dev/null && curl -f http://localhost:8080/api/health || exit 0"]

ENTRYPOINT ["/app/rusty-links"]
