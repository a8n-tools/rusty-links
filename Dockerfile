# syntax=docker/dockerfile:1

# ===== Stage 1: Nushell binary =====
FROM ghcr.io/nushell/nushell:latest-bookworm AS nushell

# ===== Stage 2: Builder =====
ARG RUST_VERSION=1.91
ARG DEBIAN_VERSION=bookworm
FROM docker.io/rust:${RUST_VERSION}-slim AS builder

ARG CARGO_FEATURES=server
ARG NO_DEFAULT_FEATURES=false

ENV CARGO_FEATURES=${CARGO_FEATURES}
ENV NO_DEFAULT_FEATURES=${NO_DEFAULT_FEATURES}

# Copy nushell and setup script
COPY --from=nushell /usr/bin/nu /usr/local/bin/nu
COPY oci-build/setup.nu /usr/local/bin/setup.nu

# Install build dependencies
RUN nu /usr/local/bin/setup.nu install-build-deps

WORKDIR /build

# Copy dependency files and source
COPY Cargo.toml Cargo.lock ./
COPY .cargo/ .cargo/
COPY src/ src/
COPY migrations/ migrations/
COPY tailwind.css tailwind.css

# Build with BuildKit cache mounts for cargo registry and target directory.
# The cache mount on /build/target persists across builds but its contents
# are NOT part of the image layer -- setup.nu copies the binary to /build/app.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/build/target \
    nu /usr/local/bin/setup.nu build

# ===== Stage 3: Runtime =====
FROM debian:${DEBIAN_VERSION}-slim

COPY --from=nushell /usr/bin/nu /usr/local/bin/nu
COPY oci-build/setup.nu /usr/local/bin/setup.nu

# Install runtime dependencies and create appuser
RUN nu /usr/local/bin/setup.nu install-runtime-deps

WORKDIR /app

# Copy binary and migrations from builder
COPY --from=builder /build/app ./app
COPY --from=builder /build/migrations ./migrations

# Create assets directory (populated at runtime by Dioxus)
RUN mkdir -p assets

# Set ownership
RUN nu /usr/local/bin/setup.nu finalize /app

# Remove nushell from the final image (~40MB savings)
RUN rm /usr/local/bin/nu /usr/local/bin/setup.nu

USER appuser

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/bin/sh", "-c", "command -v curl > /dev/null && curl -f http://localhost:8080/api/health || exit 0"]

CMD ["/app/app"]
