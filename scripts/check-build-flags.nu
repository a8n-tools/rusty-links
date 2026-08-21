#!/usr/bin/env nu

# Reject build flags in the justfile that nothing downstream defines: a cargo
# `--features` name that is not in Cargo.toml `[features]`, or a
# `--build-arg NAME=` that no Dockerfile declares with `ARG NAME`.
#
# Why this exists: the justfile carried `--features standalone,web` and
# `--build-arg BUILD_MODE=...` long after those concepts were removed. The cargo
# ones broke `just check` / `just pre-commit` for every contributor; the
# build-arg ones were silently ignored by the Dockerfile, so they read as a
# working build-time mode switch that does not exist (LINKS-25). Deployment mode
# is resolved at runtime from OIDC_ISSUER.
#
# Exit codes:
# - 0: every feature and build-arg the justfile names exists downstream.
# - 1: at least one does not; the offending flags are listed.

# Collect the `name` capture of every match, tolerating zero matches.
def captures [regex: string]: string -> list<string> {
    let rows = ($in | parse --regex $regex)
    if ($rows | is-empty) { [] } else { $rows | get name | uniq }
}

export def main [] {
    let justfile = (open --raw justfile)

    # `dioxus/server`-style paths address another crate's features, not ours.
    let defined_features = (open Cargo.toml | get features | columns)
    let used_features = (
        $justfile
        | captures '--features[= ]+(?<name>[A-Za-z0-9_,./-]+)'
        | each { split row "," }
        | flatten
        | where { |f| not ($f | str contains "/") }
        | uniq
    )
    let bad_features = ($used_features | where { |f| $f not-in $defined_features })

    let declared_args = (
        [Dockerfile oci-build/Dockerfile]
        | each { |f| open --raw $f | captures '(?m)^ARG\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)' }
        | flatten
        | uniq
    )
    let used_args = ($justfile | captures '--build-arg\s+(?<name>[A-Za-z_][A-Za-z0-9_]*)')
    let bad_args = ($used_args | where { |a| $a not-in $declared_args })

    if ($bad_features | is-empty) and ($bad_args | is-empty) {
        print "[check-build-flags] OK: justfile features and build-args all exist downstream."
        exit 0
    }

    print --stderr "[check-build-flags] FAILED:"
    for f in $bad_features {
        print --stderr $"  - justfile passes `--features ($f)`, but Cargo.toml [features] defines only: ($defined_features | str join ', ')"
    }
    for a in $bad_args {
        print --stderr $"  - justfile passes `--build-arg ($a)=...`, but no Dockerfile declares `ARG ($a)`"
    }
    exit 1
}
