#!/usr/bin/env nu

# Run every tests/*.rs target and prove the database-backed ones really ran.
#
# Why this exists: `just pre-commit` and .forgejo/workflows/check.yml only ever
# ran `cargo test --lib`. The tests/ targets were compiled by `--all-targets`
# and never executed, and CI had no database for them to reach, so a green run
# proved nothing about any SQL. LINKS-33 could not cover its write's round trip
# and LINKS-35 shipped the whole approval-gate schema verified only by hand
# (LINKS-44).
#
# Running the leg is not enough on its own: `cargo test` exits 0 when a target
# has no tests, when every case is filtered out, and when every case is
# #[ignore]d. So the harness summary is parsed per target and a floor is
# enforced on the database-backed pass count. A suite that skips looks green,
# which is the exact failure being removed here.
#
# Targets named tests/db_*.rs are the database-backed ones. That list is read
# from disk, so a new db_*.rs suite is required to run the moment it is added,
# rather than being silently left out of a hand-maintained allowlist.
#
# Usage:
#   nu scripts/check-db-tests-ran.nu --self-test
#   nu scripts/check-db-tests-ran.nu
#   nu scripts/check-db-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm app"
#
# Exit codes:
# - 0: every target ran, nothing was ignored or filtered out, and the db_* floor was met.
# - 1: a target failed or skipped, or too few database-backed tests passed.

# Targets with this prefix must reach Postgres.
const DB_PREFIX = "db_"

# Floor, not a target: enough that an empty or filtered run cannot clear it,
# low enough that deleting one obsolete case does not fail the build. Raise it
# as the suites grow rather than lowering it after a near miss. Last raised for
# the LINKS-59 refresh-token hashing, which took the suites from 51 passes to 59
# across a sixth target.
const MIN_DB_PASSED = 52

# Fold a target's harness output into counts. A target that printed no summary
# line at all reports zero summaries, which is itself a violation.
def summarize [output: string]: nothing -> record {
    let rows = (
        $output
        | parse --regex 'test result: (?<status>\w+)\. (?<passed>\d+) passed; (?<failed>\d+) failed; (?<ignored>\d+) ignored; (?<measured>\d+) measured; (?<filtered>\d+) filtered out'
    )
    if ($rows | is-empty) {
        return {summaries: 0, passed: 0, failed: 0, ignored: 0, filtered: 0}
    }
    {
        summaries: ($rows | length)
        passed: ($rows | get passed | into int | math sum)
        failed: ($rows | get failed | into int | math sum)
        ignored: ($rows | get ignored | into int | math sum)
        filtered: ($rows | get filtered | into int | math sum)
    }
}

def run-target [target: string, prefix: list<string>]: nothing -> record {
    let db = ($target | str starts-with $DB_PREFIX)
    # The db_* cases share one database and the LINKS-35 sweep deletes every
    # expired row, so two at once would delete each other's expired fixture.
    let threads = if $db { ["--" "--test-threads=1"] } else { [] }
    let argv = (
        $prefix
        | append ["cargo" "test" "--features" "server" "--test" $target]
        | append $threads
    )

    print $"[check-db-tests] ($argv | str join ' ')"
    let exe = ($argv | first)
    let args = ($argv | skip 1)
    let result = (do { run-external $exe ...$args } | complete)
    print $result.stdout
    if ($result.stderr | str trim | is-not-empty) {
        print --stderr $result.stderr
    }

    {target: $target, db: $db, exit: $result.exit_code} | merge (summarize $result.stdout)
}

# Every way a run can be green without having tested anything.
def violations [rows: table, min_db_passed: int]: nothing -> list<string> {
    mut found = []
    for row in $rows {
        if $row.exit != 0 {
            $found = ($found | append $"($row.target): cargo test exited ($row.exit)")
        }
        if $row.summaries == 0 {
            $found = ($found | append $"($row.target): printed no test result line, so the harness never ran")
        }
        if $row.ignored > 0 {
            $found = ($found | append $"($row.target): ($row.ignored) ignored, and an ignored test looks green while proving nothing")
        }
        if $row.filtered > 0 {
            $found = ($found | append $"($row.target): ($row.filtered) filtered out by a name filter")
        }
        if $row.db and $row.passed < 1 {
            $found = ($found | append $"($row.target): a database-backed target that passed nothing")
        }
    }

    let db_rows = ($rows | where db)
    if ($db_rows | is-empty) {
        $found = ($found | append $"no tests/($DB_PREFIX)*.rs target ran at all")
    }
    let db_passed = if ($db_rows | is-empty) { 0 } else { $db_rows | get passed | math sum }
    if $db_passed < $min_db_passed {
        $found = ($found | append $"only ($db_passed) database-backed tests passed, and the floor is ($min_db_passed)")
    }
    $found
}

# Prove the guard still detects a vacuous run, and that the parser still reads
# what the harness actually prints. Without this, a broken parser would pass
# every job silently, which is the same blindness one level up.
def run-self-test [min_db_passed: int] {
    let sample = "
running 3 tests
test schema_migrations_apply ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s
"
    let parsed = (summarize $sample)
    if ($parsed.summaries != 1) or ($parsed.passed != 3) or ($parsed.failed != 0) or ($parsed.ignored != 0) or ($parsed.filtered != 0) {
        print --stderr $"[check-db-tests] SELF-TEST FAILED: the parser no longer reads a real summary line: ($parsed)"
        exit 1
    }

    let vacuous = [
        {target: "db_example", db: true, exit: 0, summaries: 1, passed: 0, failed: 0, ignored: 0, filtered: 7}
        {target: "route_surface", db: false, exit: 0, summaries: 1, passed: 6, failed: 0, ignored: 0, filtered: 0}
    ]
    let ignored = [
        {target: "db_example", db: true, exit: 0, summaries: 1, passed: $min_db_passed, failed: 0, ignored: 1, filtered: 0}
    ]
    let healthy = [
        {target: "db_example", db: true, exit: 0, summaries: 1, passed: $min_db_passed, failed: 0, ignored: 0, filtered: 0}
    ]

    if (violations $vacuous $min_db_passed | is-empty) {
        print --stderr "[check-db-tests] SELF-TEST FAILED: a run with no database-backed passes was accepted."
        exit 1
    }
    if (violations $ignored $min_db_passed | is-empty) {
        print --stderr "[check-db-tests] SELF-TEST FAILED: an #[ignore]d test was accepted."
        exit 1
    }
    let rejected = (violations $healthy $min_db_passed)
    if ($rejected | is-not-empty) {
        print --stderr $"[check-db-tests] SELF-TEST FAILED: a healthy run was rejected: ($rejected | str join '; ')"
        exit 1
    }
    print "[check-db-tests] SELF-TEST OK: vacuous and ignored runs are rejected, a healthy one is not."
}

export def main [
    --runner: string = "" # command that fronts `cargo`, e.g. a `docker compose run` wrapper
    --min-db-passed: int = -1 # override the built-in floor
    --self-test # check the guard still detects a vacuous run, then exit
] {
    let floor = if $min_db_passed < 0 { $MIN_DB_PASSED } else { $min_db_passed }
    if $self_test {
        run-self-test $floor
        return
    }

    if not ("Cargo.toml" | path exists) {
        print --stderr "[check-db-tests] FAILED: run this from the repository root."
        exit 1
    }

    let prefix = ($runner | split row --regex '\s+' | where {|part| $part != ""})
    let targets = (ls tests/*.rs | get name | path parse | get stem | sort)
    if ($targets | is-empty) {
        print --stderr "[check-db-tests] FAILED: no tests/*.rs targets found."
        exit 1
    }

    let rows = ($targets | each {|target| run-target $target $prefix})
    print ""
    print ($rows | select target db passed ignored filtered exit)

    let found = (violations $rows $floor)
    if ($found | is-not-empty) {
        print --stderr "[check-db-tests] FAILED:"
        for problem in $found {
            print --stderr $"  - ($problem)"
        }
        exit 1
    }

    let db_rows = ($rows | where db)
    print $"[check-db-tests] OK: ($db_rows | get passed | math sum) database-backed tests passed across ($db_rows | length) targets, ($rows | get passed | math sum) in total."
}
