#!/usr/bin/env nu

# Build script for Rust applications using Docker BuildKit.
# Reads config from config.yml (or a provided config file) and runs docker buildx build.

export-env {
    $env.NU_LOG_LEVEL = "DEBUG"
}

# Load the build configuration from a YAML file.
def load-config []: [nothing -> any, string -> any] {
    try {
        let config = ($in | default "config.yml" | open)
        $config
    } catch {|err|
        use std log
        log error $"[load-config] Failed to load config: ($err.msg)"
        exit 1
    }
}

# Main entry point
# Usage: ./build.nu [config-file]
# Default config: config.yml
# For saas build: ./build.nu config.saas.yml
def main [config_file?: string] {
    use std log
    log info "Starting Docker BuildKit build..."

    let config = ($config_file | default "config.yml" | load-config)

    let features = ($config.builder.features? | default "server")
    let no_default = ($config.builder.no_default_features? | default false) | into string
    let image_name = $config.published.name
    let image_version = $config.published.version
    let tag = $"($image_name):($image_version)"

    # Collect labels as --label arguments
    let label_args = ($config.runtime.cfg.labels? | default []
        | each {|label| ["--label" $label]}
        | flatten)

    # Build the docker buildx command
    let project_root = ($env.FILE_PWD | path dirname)

    let args = [
        "buildx" "build"
        "--build-arg" $"CARGO_FEATURES=($features)"
        "--build-arg" $"NO_DEFAULT_FEATURES=($no_default)"
        "--tag" $tag
        ...$label_args
        "--load"
        $project_root
    ]

    log info $"Running: docker ($args | str join ' ')"
    ^docker ...$args

    log info $"Build complete: ($tag)"

    # Output for CI/CD
    mut output = "output.log"
    if ("GITHUB_OUTPUT" in $env) {
        $output = $env.GITHUB_OUTPUT
    }
    $"image=($image_name)\n" | save --append $output
    $"tags=($image_version)\n" | save --append $output

    log info $"Image: ($image_name), Tags: ($image_version)"
}
