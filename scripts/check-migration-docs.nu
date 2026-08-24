#!/usr/bin/env nu

# Reject drift between `migrations/` and the Migration History table in
# docs/DATABASE.md. Every migration file needs a row and every row needs a file,
# so the comparison runs in both directions.
#
# Why this exists: the table listed six of fourteen migrations and read as if
# the schema stopped changing in January 2025 (LINKS-46). Eight rows went missing
# because only a human ever compared the two lists, and nobody did.
#
# Static like `scripts/check-build-flags.nu` and
# `scripts/check-migration-immutability.nu`: no cargo, no database, no
# compilation, so it runs in the cheap phase of the check job and fails in
# seconds rather than after the Rust build.
#
# Usage:
#   nu scripts/check-migration-docs.nu --self-test
#   nu scripts/check-migration-docs.nu
#
# Exit codes:
# - 0: `migrations/` and the table name exactly the same versions.
# - 1: they disagree, or the table could not be parsed. A parse that stops
#      matching is a failure, never a silent pass: an unrecognised table shape
#      would otherwise report "no drift" for a table it never read.

const DOC = "docs/DATABASE.md"
const HEADING = "### Migration History"
const COLUMNS = ["Version" "Description" "Date"]

# A migration file name: <14-digit version>_<name>.sql.
const VERSION_RE = '^(?<version>\d{14})_(?<name>.+)$'

# The table body: every `|` line between the heading and the next heading.
def table-lines [doc: string]: nothing -> list<string> {
    let lines = ($doc | lines)
    let starts = ($lines | enumerate | where {|r| ($r.item | str trim) == $HEADING} | get index)
    if ($starts | is-empty) { return [] }
    let after = ($lines | skip (($starts | first) + 1))
    let ends = ($after | enumerate | where {|r| ($r.item | str trim | str starts-with "#")} | get index)
    let body = if ($ends | is-empty) { $after } else { $after | first ($ends | first) }
    $body | each {|l| $l | str trim} | where {|l| $l | str starts-with "|"}
}

def cells [line: string]: nothing -> list<string> {
    $line | str trim --char "|" | split row "|" | each {|c| $c | str trim}
}

# The date the version prefix encodes, e.g. 20260821000014 -> 2026-08-21.
def version-date [version: string]: nothing -> string {
    let y = ($version | str substring 0..<4)
    let m = ($version | str substring 4..<6)
    let d = ($version | str substring 6..<8)
    $"($y)-($m)-($d)"
}

# Parse the table into rows, reporting every row it could not read rather than
# dropping it. Returns {rows, problems}.
def doc-rows [doc: string]: nothing -> record {
    let lines = (table-lines $doc)
    if ($lines | is-empty) {
        return {
            rows: []
            problems: [$"($DOC): found no table under `($HEADING)`. The section was renamed, moved or lost its table, so nothing was compared."]
        }
    }

    let parsed = ($lines | each {|l| cells $l})
    let header = ($parsed | first)
    if $header != $COLUMNS {
        return {
            rows: []
            problems: [$"($DOC): the Migration History header is ($header | str join ' | '), expected ($COLUMNS | str join ' | '). The table shape changed, so nothing was compared."]
        }
    }

    let body = ($parsed | skip 1 | where {|row| not ($row | all {|c| $c =~ '^:?-{3,}:?$'})})
    if ($body | is-empty) {
        return {rows: [], problems: [$"($DOC): the Migration History table has a header but no rows."]}
    }

    mut rows = []
    mut problems = []
    for row in $body {
        if ($row | length) != ($COLUMNS | length) {
            $problems = ($problems | append $"($DOC): unreadable row `($row | str join ' | ')`: expected ($COLUMNS | length) cells, got ($row | length)")
            continue
        }
        let version = ($row | get 0)
        if not ($version =~ '^\d{14}$') {
            $problems = ($problems | append $"($DOC): row `($row | str join ' | ')` has version `($version)`, which is not a 14-digit migration version")
            continue
        }
        if ($row | get 1 | is-empty) {
            $problems = ($problems | append $"($DOC): row for ($version) has an empty Description")
        }
        let expected = (version-date $version)
        if ($row | get 2) != $expected {
            $problems = ($problems | append $"($DOC): row for ($version) says Date ($row | get 2), but the version prefix encodes ($expected)")
        }
        $rows = ($rows | append {version: $version, description: ($row | get 1)})
    }

    {rows: $rows, problems: $problems}
}

# Compare both directions. `files` are migration file names without the path.
def violations [files: list<string>, rows: list<any>]: nothing -> list<string> {
    mut found = []

    if ($files | is-empty) {
        $found = ($found | append "no migrations/*.sql files found, so there was nothing to compare")
    }
    if ($rows | is-empty) {
        $found = ($found | append $"($DOC): the Migration History table lists no migration, so there was nothing to compare")
    }

    mut by_version = {}
    for f in $files {
        let m = ($f | str replace --regex '\.sql$' '' | parse --regex $VERSION_RE)
        if ($m | is-empty) {
            $found = ($found | append $"migrations/($f) is not named <14-digit version>_<name>.sql, so it cannot be matched to a row")
            continue
        }
        let version = ($m | get version.0)
        if $version in $by_version {
            $found = ($found | append $"two migrations share version ($version): ($by_version | get $version) and ($f)")
            continue
        }
        $by_version = ($by_version | insert $version $f)
    }

    mut seen = []
    for row in $rows {
        if $row.version in $seen {
            $found = ($found | append $"($DOC): the Migration History table has two rows for ($row.version)")
            continue
        }
        $seen = ($seen | append $row.version)
        if not ($row.version in $by_version) {
            $found = ($found | append $"($DOC): row ($row.version) `($row.description)` names no migration; there is no migrations/($row.version)_*.sql. Delete or correct the row.")
        }
    }

    for version in ($by_version | columns | sort) {
        if not ($version in $seen) {
            $found = ($found | append $"migrations/($by_version | get $version) has no row in the ($DOC) Migration History table. Add one: | ($version) | <what it does> | (version-date $version) |")
        }
    }

    $found
}

# Prove the parser still reads the real table shape and that each drift is
# still rejected. Without it a parser that stopped matching would pass every
# job silently, which is the same blindness the guard exists to remove.
def run-self-test [] {
    let sample = "
### Migration History

| Version | Description | Date |
|---------|-------------|------|
| 20250101000001 | Initial schema | 2025-01-01 |
| 20260821000014 | Add pending_login_approvals | 2026-08-21 |

### Creating Custom Migrations
| 99999999999999 | not part of the history table | 9999-99-99 |
"
    let parsed = (doc-rows $sample)
    if ($parsed.problems | is-not-empty) {
        print --stderr $"[check-migration-docs] SELF-TEST FAILED: the parser rejected a well-formed table: ($parsed.problems | str join '; ')"
        exit 1
    }
    if ($parsed.rows | get version) != ["20250101000001" "20260821000014"] {
        print --stderr $"[check-migration-docs] SELF-TEST FAILED: the parser no longer reads the real table shape: ($parsed.rows | to json --raw)"
        exit 1
    }

    let files = ["20250101000001_initial_schema.sql" "20260821000014_create_pending_login_approvals.sql"]
    if (violations $files $parsed.rows | is-not-empty) {
        print --stderr $"[check-migration-docs] SELF-TEST FAILED: a matching pair was rejected: ($parsed.rows | to json --raw)"
        exit 1
    }
    if (violations ($files | first 1) $parsed.rows | is-empty) {
        print --stderr "[check-migration-docs] SELF-TEST FAILED: a row naming no migration was accepted."
        exit 1
    }
    if (violations ($files | append "20260822000015_later.sql") $parsed.rows | is-empty) {
        print --stderr "[check-migration-docs] SELF-TEST FAILED: a migration with no row was accepted."
        exit 1
    }
    if (violations [] $parsed.rows | is-empty) {
        print --stderr "[check-migration-docs] SELF-TEST FAILED: an empty migrations/ was accepted."
        exit 1
    }
    if (violations $files [] | is-empty) {
        print --stderr "[check-migration-docs] SELF-TEST FAILED: an empty table was accepted."
        exit 1
    }

    let renamed = ($sample | str replace $HEADING "### Migrations Applied")
    if ((doc-rows $renamed).problems | is-empty) {
        print --stderr "[check-migration-docs] SELF-TEST FAILED: a renamed heading was accepted instead of failing loudly."
        exit 1
    }
    let reshaped = ($sample | str replace "| Version | Description | Date |" "| Version | Notes |")
    if ((doc-rows $reshaped).problems | is-empty) {
        print --stderr "[check-migration-docs] SELF-TEST FAILED: a changed header was accepted instead of failing loudly."
        exit 1
    }
    let wrong_date = ($sample | str replace "| 20250101000001 | Initial schema | 2025-01-01 |" "| 20250101000001 | Initial schema | 2025-02-01 |")
    if ((doc-rows $wrong_date).problems | is-empty) {
        print --stderr "[check-migration-docs] SELF-TEST FAILED: a Date that contradicts the version prefix was accepted."
        exit 1
    }

    print "[check-migration-docs] SELF-TEST OK: a missing row, an extra row, an empty side and an unreadable table are all rejected, a matching pair is not."
}

export def main [
    --self-test # check the guard still detects drift, then exit
] {
    if $self_test {
        run-self-test
        return
    }

    if not ("Cargo.toml" | path exists) {
        print --stderr "[check-migration-docs] FAILED: run this from the repository root."
        exit 1
    }
    if not ($DOC | path exists) {
        print --stderr $"[check-migration-docs] FAILED: ($DOC) does not exist."
        exit 1
    }

    let files = (glob migrations/*.sql | path basename | sort)
    let parsed = (doc-rows (open --raw $DOC))
    let found = ($parsed.problems | append (violations $files $parsed.rows))

    if ($found | is-not-empty) {
        print --stderr "[check-migration-docs] FAILED:"
        for problem in $found {
            print --stderr $"  - ($problem)"
        }
        print --stderr ""
        print --stderr $"The Migration History table in ($DOC) must list every file in migrations/, one row each, and no row may name a migration that does not exist."
        exit 1
    }

    print $"[check-migration-docs] OK: all ($files | length) migrations have a row in the ($DOC) Migration History table, and every row names a migration."
}
