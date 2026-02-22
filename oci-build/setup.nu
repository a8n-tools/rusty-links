#!/usr/bin/env nu

# Setup script that runs INSIDE the container image during Docker build.
# Invoked via: RUN nu /usr/local/bin/setup.nu <subcommand>

# Install build-stage dependencies (compiler toolchain requirements).
def "main install-build-deps" [] {
    print "Installing build dependencies..."
    ^apt-get update
    ^apt-get install -y --no-install-recommends pkg-config libssl-dev
    ^rm -rf /var/lib/apt/lists/*
    print "Build dependencies installed."
}

# Build the Rust application.
# Reads CARGO_FEATURES and NO_DEFAULT_FEATURES from environment variables.
def "main build" [] {
    let features = ($env.CARGO_FEATURES? | default "server")
    let no_default = ($env.NO_DEFAULT_FEATURES? | default "false")

    # Prepare assets/tailwind.css for the asset!() macro
    mkdir assets
    cp tailwind.css assets/tailwind.css

    # Construct cargo build flags
    mut flags = ["build" "--release"]
    if $no_default == "true" {
        $flags = ($flags | append "--no-default-features")
    }
    $flags = ($flags | append ["--features" $features])

    print $"Building with: cargo ($flags | str join ' ')"
    ^cargo ...$flags

    # Copy binary out of the cache mount directory into a stable location.
    # When using --mount=type=cache on the target dir, the contents aren't
    # part of the layer, so we must copy the binary within the same RUN step.
    let binary = "target/release/rusty-links"
    let dest = "/build/app"
    print $"Copying ($binary) -> ($dest)"
    cp $binary $dest

    print "Build complete."
}

# Install runtime-stage dependencies (minimal libs + create app user).
def "main install-runtime-deps" [] {
    print "Installing runtime dependencies..."
    ^apt-get update
    ^apt-get install -y --no-install-recommends ca-certificates libssl3
    ^rm -rf /var/lib/apt/lists/*

    print "Creating appuser (UID 1001)..."
    ^useradd -m -u 1001 appuser

    print "Runtime dependencies installed."
}

# Finalize the runtime image (set ownership on app directory).
def "main finalize" [app_dir: string = "/app"] {
    print $"Setting ownership of ($app_dir) to appuser..."
    ^chown -R appuser:appuser $app_dir
    print "Finalize complete."
}

def main [] {
    print "Usage: nu setup.nu <subcommand>"
    print "Subcommands: install-build-deps, build, install-runtime-deps, finalize"
    exit 1
}
