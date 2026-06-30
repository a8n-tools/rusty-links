#!/usr/bin/env nu

# Reject any change that modifies, renames, or deletes a migration file that is
# already committed on the baseline branch. Adding a NEW migration is always
# allowed.
#
# Why this exists: sqlx records a SHA-384 checksum of each migration in
# `_sqlx_migrations` when it applies it and re-verifies that checksum on every
# startup. Editing an already-applied migration makes every database that ran it
# refuse to boot ("migration N was previously applied but has been modified").
# Migrations are immutable once applied; the only safe change is a NEW migration.
#
# This is the break that took down mokosh-server v0.4.0 on nc-01 (DEV-395): an
# already-applied seed migration was edited during a routine cleanup, so the
# released image could not start until the prod DB's recorded checksum was
# reconciled by hand. This guard stops that edit at PR time.
#
# Exit codes:
# - 0: no committed migration was modified, renamed, or deleted (new files pass).
# - 1: one or more committed migrations were modified/renamed/deleted; the
#      offending file(s) are listed.
# - 2: the diff itself could not run (e.g. the baseline ref is unavailable).
#      Fail loud so a broken base never reads as "nothing changed".
export def main [
    --base: string = "origin/main"   # immutable baseline ref to diff HEAD against
] {
    # Three-dot range: diff from the merge-base of <base> and HEAD up to HEAD, so
    # only the changes this branch introduced are considered. --diff-filter=MRD
    # selects Modified / Renamed / Deleted and excludes Added, so a brand-new
    # migration file passes.
    let diff = (^git diff --diff-filter=MRD --name-only $"($base)...HEAD" -- migrations/ | complete)

    if $diff.exit_code != 0 {
        print --stderr $"[check-migration-immutability] FATAL: `git diff ($base)...HEAD` failed \(exit ($diff.exit_code)). Cannot verify migration immutability; refusing to pass. Ensure the checkout has full history \(fetch-depth: 0) and that '($base)' is fetched."
        let err = ($diff.stderr | str trim)
        if ($err | is-not-empty) { print --stderr $err }
        exit 2
    }

    let offending = ($diff.stdout | lines | each { str trim } | where ($it | is-not-empty))

    if ($offending | is-empty) {
        print "[check-migration-immutability] OK: no committed migration was modified, renamed, or deleted."
        exit 0
    }

    print --stderr "[check-migration-immutability] FAILED: the following already-committed migration file(s) were modified, renamed, or deleted:"
    for f in $offending {
        print --stderr $"  - ($f)"
    }
    print --stderr ""
    print --stderr "Committed migrations are immutable. sqlx verifies each migration's checksum on every startup, so editing one breaks every database that already applied it. Add a NEW migration instead of editing an existing one."
    exit 1
}
