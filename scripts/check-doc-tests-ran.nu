#!/usr/bin/env nu

# Run the doc tests and prove the harness collected something.
#
# Why this exists: nothing in this repo ever compiled a doc example. `just
# pre-commit` and .forgejo/workflows/check.yml ran `cargo test --lib` and, since
# LINKS-44, the tests/*.rs targets. Neither builds doc tests, and neither does
# `cargo build --all-targets`, so all ten examples had rotted into compile errors
# on missing `use` lines while every check stayed green (LINKS-48).
#
# Running the leg is not enough on its own: `cargo test --doc` exits 0 when it
# collects nothing, and collecting nothing is the easy failure here, because
# `default = []` cfgs out every module that carries an example. So the harness
# summary is parsed and a floor is enforced on the pass count.
#
# ```ignore``` is the other way to look green: rustdoc reports such a block as
# ignored rather than compiling it, so any ignored count is a violation.
# ```no_run``` is not: rustdoc compiles and type-checks it, reports it as
# `- compile ... ok`, and counts it as passed.
#
# The server leg is the one that runs. Every documented item lives behind
# `#[cfg(feature = "server")]`, so `cargo test --doc` without `--features server`
# collects zero doc tests and passes vacuously.
#
# Usage:
#   nu scripts/check-doc-tests-ran.nu --self-test
#   nu scripts/check-doc-tests-ran.nu
#   nu scripts/check-doc-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm --no-deps app"
#
# Exit codes:
# - 0: the harness ran, nothing failed, was ignored or was filtered out, and the floor was met.
# - 1: a doc example failed, or too few ran to prove anything.

# Floor, not a target: ten examples compile today, and the gap leaves room to
# delete an obsolete one without failing the build. Raise it as the examples
# grow rather than lowering it after a near miss.
const MIN_DOC_PASSED = 8

# Fold the harness output into counts. Output that printed no summary line at
# all reports zero summaries, which is itself a violation.
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

# Every way a doc-test run can be green without having compiled an example.
def violations [row: record, min_passed: int]: nothing -> list<string> {
    mut found = []
    if $row.exit != 0 {
        $found = ($found | append $"cargo test --doc exited ($row.exit)")
    }
    if $row.summaries == 0 {
        $found = ($found | append "printed no test result line, so the harness never ran")
    }
    if $row.failed > 0 {
        $found = ($found | append $"($row.failed) doc examples failed to compile or run")
    }
    if $row.ignored > 0 {
        $found = ($found | append $"($row.ignored) ignored, and an ```ignore``` block is never compiled, so it proves nothing")
    }
    if $row.filtered > 0 {
        $found = ($found | append $"($row.filtered) filtered out by a name filter")
    }
    if $row.passed < $min_passed {
        $found = ($found | append $"only ($row.passed) doc tests passed, and the floor is ($min_passed)")
    }
    $found
}

# Prove the guard still detects a vacuous run, and that the parser still reads
# what the harness actually prints. Without this, a broken regex or a drifted
# summary format would pass every job silently, which is the same blindness one
# level up.
def run-self-test [min_passed: int] {
    let sample = "
running 2 tests
test src/scheduler/mod.rs - scheduler::Scheduler (line 26) - compile ... ok
test src/error.rs - error::AppError::duplicate (line 220) ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.42s
"
    let parsed = (summarize $sample)
    if ($parsed.summaries != 1) or ($parsed.passed != 10) or ($parsed.failed != 0) or ($parsed.ignored != 0) or ($parsed.filtered != 0) {
        print --stderr $"[check-doc-tests] SELF-TEST FAILED: the parser no longer reads a real summary line: ($parsed)"
        exit 1
    }

    let empty = {exit: 0, summaries: 1, passed: 0, failed: 0, ignored: 0, filtered: 0}
    let silent = {exit: 0, summaries: 0, passed: 0, failed: 0, ignored: 0, filtered: 0}
    let ignored = {exit: 0, summaries: 1, passed: $min_passed, failed: 0, ignored: 1, filtered: 0}
    let healthy = {exit: 0, summaries: 1, passed: $min_passed, failed: 0, ignored: 0, filtered: 0}

    if (violations $empty $min_passed | is-empty) {
        print --stderr "[check-doc-tests] SELF-TEST FAILED: a run that collected no doc test was accepted."
        exit 1
    }
    if (violations $silent $min_passed | is-empty) {
        print --stderr "[check-doc-tests] SELF-TEST FAILED: a run with no summary line was accepted."
        exit 1
    }
    if (violations $ignored $min_passed | is-empty) {
        print --stderr "[check-doc-tests] SELF-TEST FAILED: an ```ignore```d example was accepted."
        exit 1
    }
    let rejected = (violations $healthy $min_passed)
    if ($rejected | is-not-empty) {
        print --stderr $"[check-doc-tests] SELF-TEST FAILED: a healthy run was rejected: ($rejected | str join '; ')"
        exit 1
    }
    print "[check-doc-tests] SELF-TEST OK: empty, silent and ignored runs are rejected, a healthy one is not."
}

export def main [
    --runner: string = "" # command that fronts `cargo`, e.g. a `docker compose run` wrapper
    --min-passed: int = -1 # override the built-in floor
    --self-test # check the guard still detects a vacuous run, then exit
] {
    let floor = if $min_passed < 0 { $MIN_DOC_PASSED } else { $min_passed }
    if $self_test {
        run-self-test $floor
        return
    }

    if not ("Cargo.toml" | path exists) {
        print --stderr "[check-doc-tests] FAILED: run this from the repository root."
        exit 1
    }

    let prefix = ($runner | split row --regex '\s+' | where {|part| $part != ""})
    let argv = ($prefix | append ["cargo" "test" "--features" "server" "--doc"])

    print $"[check-doc-tests] ($argv | str join ' ')"
    let exe = ($argv | first)
    let args = ($argv | skip 1)
    let result = (do { run-external $exe ...$args } | complete)
    print $result.stdout
    if ($result.stderr | str trim | is-not-empty) {
        print --stderr $result.stderr
    }

    # The summary can land on either stream depending on how the runner wraps
    # cargo, so parse both rather than failing closed on a plumbing detail.
    let row = ({exit: $result.exit_code} | merge (summarize ([$result.stdout $result.stderr] | str join "\n")))
    print ""
    print ($row | select passed failed ignored filtered exit)

    let found = (violations $row $floor)
    if ($found | is-not-empty) {
        print --stderr "[check-doc-tests] FAILED:"
        for problem in $found {
            print --stderr $"  - ($problem)"
        }
        exit 1
    }

    print $"[check-doc-tests] OK: ($row.passed) doc tests passed, none failed, ignored or filtered out."
}
