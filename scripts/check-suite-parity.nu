#!/usr/bin/env nu

# Reject drift between the two hand-maintained copies of the check suite: the
# `pre-commit` recipe in `justfile` and the steps in
# `.forgejo/workflows/check.yml`. Every cargo leg and every guard script in one
# must appear in the other, so the comparison runs in both directions.
#
# Why this exists: four issues in a row were a leg that ran and proved nothing
# (LINKS-36, LINKS-44, LINKS-48, LINKS-39). LINKS-39's residual risk is one
# level up: a later edit that drops `--target wasm32-unknown-unknown`,
# `--features server` or `-- --deny warnings` from one of the two copies. The
# command still runs and still exits 0. Dropping `--target` is the invisible
# one, because `.cargo/config.toml` pins `[build] target =
# "x86_64-unknown-linux-gnu"`, so the leg quietly re-lints the host build that
# two other legs already cover (LINKS-49).
#
# Static like `scripts/check-build-flags.nu` and
# `scripts/check-migration-docs.nu`: no cargo, no database, no compilation, so
# it runs in the cheap phase and fails in milliseconds rather than after the
# Rust build.
#
# Usage:
#   nu scripts/check-suite-parity.nu --self-test
#   nu scripts/check-suite-parity.nu
#
# Exit codes:
# - 0: both files run the same canonical legs and the same guard scripts.
# - 1: they diverge, or either file could not be parsed. A parse that stops
#      matching is a failure, never a silent pass: zero legs extracted on both
#      sides would otherwise compare equal and report "no drift".

const JUSTFILE = "justfile"
const WORKFLOW = ".forgejo/workflows/check.yml"
const RECIPE = "pre-commit"

# Floors, not targets: nine cargo legs and seven guard scripts run today. They
# exist so a parser that stopped matching fails instead of comparing [] to [].
# Raise them as the suite grows rather than lowering after a near miss.
const MIN_CARGO_LEGS = 7
const MIN_GUARD_SCRIPTS = 5

# The three compilation configurations the crate has. `default = []` gates every
# server module behind `#[cfg(feature = "server")]`, and the wasm leg is the only
# one that compiles `#[cfg(target_arch = "wasm32")]` code (LINKS-36, LINKS-39).
const REQUIRED_CLIPPY = [
    {features: [], target: "", all_targets: true}
    {features: ["server"], target: "", all_targets: true}
    {features: ["web"], target: "wasm32-unknown-unknown", all_targets: true}
]

# Flags that swallow the next token as their value, so `--test db_schema` is one
# unit rather than a flag plus a stray positional.
const VALUE_FLAGS = [
    "--features" "--target" "--test" "--bin" "--example" "--bench" "--package"
    "--manifest-path" "--profile" "--target-dir" "--jobs"
    "--deny" "--allow" "--warn" "--forbid"
]

# check.yml writes `-D warnings`, the justfile `--deny warnings`, each matching
# its own file's local style (LINKS-39). Same token, so normalise rather than
# forcing one spelling on both.
def canon-flag [flag: string]: nothing -> string {
    match $flag {
        "-D" => "--deny"
        "-A" => "--allow"
        "-W" => "--warn"
        "-F" => "--forbid"
        "-p" => "--package"
        "-j" => "--jobs"
        _ => $flag
    }
}

# Split a command line into tokens, keeping a double-quoted run as one token so
# a `print "... cargo clippy ..."` line is not read as a cargo leg. Strips
# nushell's `^` external-command sigil.
def tokenize [line: string]: nothing -> list<string> {
    $line
    | parse --regex '"(?<quoted>[^"]*)"|(?<word>\S+)'
    | each {|r| if ($r.word | is-empty) { $r.quoted } else { $r.word }}
    | each {|t| $t | str replace --regex '^\^' ''}
    | where {|t| $t != ""}
}

# Split an argv at the first bare `--`, separating cargo's own flags from the
# rustc/harness flags behind it.
def split-args [args: list<string>]: nothing -> record {
    let idx = ($args | enumerate | where {|r| $r.item == "--"} | get index)
    if ($idx | is-empty) { return {head: $args, tail: []} }
    let i = ($idx | first)
    {head: ($args | first $i), tail: ($args | skip ($i + 1))}
}

# Fold an argv into canonical `--flag value` units: `--features=web` and
# `--features web` collapse to the same string, `-D warnings` to `--deny warnings`.
# A trailing value flag with no value is reported rather than dropped.
def pair-flags [args: list<string>]: nothing -> record {
    let expanded = (
        $args
        | each {|t|
            let m = ($t | parse --regex '^(?<flag>--?[A-Za-z0-9][A-Za-z0-9._-]*)=(?<value>.*)$')
            if ($m | is-empty) { [$t] } else { [($m | get flag.0) ($m | get value.0)] }
        }
        | flatten
    )
    $expanded | reduce --fold {out: [], pending: null} {|tok, acc|
        if $acc.pending != null {
            {out: ($acc.out | append $"($acc.pending) ($tok)"), pending: null}
        } else {
            let canon = (canon-flag $tok)
            if $canon in $VALUE_FLAGS {
                {out: $acc.out, pending: $canon}
            } else {
                {out: ($acc.out | append $canon), pending: null}
            }
        }
    }
}

# The values a repeated `--flag value` unit carries, e.g. every `--features`.
def values-of [pairs: list<string>, flag: string]: nothing -> list<string> {
    let prefix = $"($flag) "
    $pairs
    | where {|p| $p | str starts-with $prefix}
    | each {|p| $p | str substring ($prefix | str length)..}
}

# Reduce a cargo argv (starting at the `cargo` token) to a canonical leg: the
# subcommand, a sorted feature set, the target, the remaining flags sorted, and
# the lint flags behind `--`. Sorting means flag ORDER is not drift; a different
# flag is.
def normalise-cargo [tokens: list<string>]: nothing -> record {
    let rest = ($tokens | skip 1)
    if ($rest | is-empty) {
        return {ok: false, reason: "`cargo` with no subcommand"}
    }
    let sub = ($rest | first)
    if ($sub | str starts-with "-") {
        return {ok: false, reason: $"`cargo ($sub)` starts with a flag, so there is no subcommand to compare"}
    }
    let split = (split-args ($rest | skip 1))
    let head = (pair-flags $split.head)
    let tail = (pair-flags $split.tail)
    if ($head.pending != null) or ($tail.pending != null) {
        let dangling = (if $head.pending != null { $head.pending } else { $tail.pending })
        return {ok: false, reason: $"`($dangling)` is the last token, so it has no value"}
    }

    let features = (
        values-of $head.out "--features"
        | each {|v| $v | split row ","}
        | flatten
        | each {|f| $f | str trim}
        | where {|f| $f != ""}
        | uniq
        | sort
    )
    let targets = (values-of $head.out "--target" | uniq | sort)
    let others = (
        $head.out
        | where {|p| not (($p | str starts-with "--features ") or ($p | str starts-with "--target "))}
        | uniq
        | sort
    )
    let lints = ($tail.out | uniq | sort)

    let parts = (
        [$"cargo ($sub)"]
        | append $others
        | append (if ($features | is-empty) { [] } else { [$"--features ($features | str join ',')"] })
        | append (if ($targets | is-empty) { [] } else { [$"--target ($targets | str join ',')"] })
    )
    let canonical = if ($lints | is-empty) {
        $parts | str join " "
    } else {
        $"($parts | str join ' ') -- ($lints | str join ' ')"
    }

    {
        ok: true
        subcommand: $sub
        features: $features
        target: (if ($targets | is-empty) { "" } else { $targets | first })
        all_targets: ("--all-targets" in $others)
        deny_warnings: ("--deny warnings" in $lints)
        canonical: $canonical
    }
}

# Describe a required clippy configuration the way it is written on the command line.
def describe-config [want: record]: nothing -> string {
    let parts = (
        (if $want.all_targets { ["--all-targets"] } else { [] })
        | append (if ($want.features | is-empty) { [] } else { [$"--features ($want.features | str join ',')"] })
        | append (if ($want.target | is-empty) { [] } else { [$"--target ($want.target)"] })
    )
    if ($parts | is-empty) { "default features" } else { $parts | str join " " }
}

# Classify each command line as a cargo leg, a guard-script invocation, or
# neither. Lines that are neither (print, echo, the stylesheet placeholder) are
# ignored; a cargo line that cannot be read is reported, never dropped.
def extract-legs [source: string, cmds: list<string>]: nothing -> record {
    mut cargo = []
    mut guards = []
    mut problems = []

    for line in $cmds {
        let tokens = (tokenize $line)
        let script = ($tokens | where {|t| $t =~ '^scripts/.+\.nu$'})
        if ($script | is-not-empty) {
            let name = ($script | first)
            let self_test = ("--self-test" in $tokens)
            # The justfile passes --runner so the guard shells into the compose
            # container; CI runs cargo on the runner. That is plumbing, not a leg.
            $guards = ($guards | append {
                script: $name
                self_test: $self_test
                canonical: (if $self_test { $"nu ($name) --self-test" } else { $"nu ($name)" })
            })
            continue
        }

        let at = ($tokens | enumerate | where {|r| $r.item == "cargo"} | get index)
        if ($at | is-empty) { continue }
        let leg = (normalise-cargo ($tokens | skip ($at | first)))
        if not $leg.ok {
            $problems = ($problems | append $"($source): could not read `($line)`: ($leg.reason)")
            continue
        }
        $cargo = ($cargo | append $leg)
    }

    {cargo: $cargo, guards: $guards, problems: $problems}
}

# The command lines of the `pre-commit` recipe body: every indented line until
# the recipe ends. A missing recipe is a parse failure, not an empty suite.
def justfile-legs [text: string]: nothing -> record {
    let lines = ($text | lines)
    let starts = ($lines | enumerate | where {|r| $r.item =~ $'^($RECIPE)\s*:'} | get index)
    if ($starts | is-empty) {
        return {
            cargo: []
            guards: []
            problems: [$"($JUSTFILE): found no `($RECIPE):` recipe. It was renamed or removed, so nothing was compared."]
        }
    }
    let after = ($lines | skip (($starts | first) + 1))
    let ends = (
        $after
        | enumerate
        | where {|r| (($r.item | str trim) != "") and (not ($r.item =~ '^\s'))}
        | get index
    )
    let body = if ($ends | is-empty) { $after } else { $after | first ($ends | first) }
    let cmds = (
        $body
        | each {|l| $l | str trim}
        | where {|l| ($l != "") and (not ($l | str starts-with "#"))}
    )
    extract-legs $"($JUSTFILE) `($RECIPE)`" $cmds
}

# The command lines of every `run:` step in every job. A `run:` block can hold
# several lines, so each is classified on its own.
def workflow-legs [text: string]: nothing -> record {
    let doc = (try { $text | from yaml } catch { null })
    if $doc == null {
        return {cargo: [], guards: [], problems: [$"($WORKFLOW): is not valid YAML, so nothing was compared."]}
    }
    let jobs = ($doc | get --optional jobs)
    if ($jobs | is-empty) {
        return {cargo: [], guards: [], problems: [$"($WORKFLOW): has no `jobs:` block, so nothing was compared."]}
    }
    let steps = ($jobs | values | each {|j| $j | get --optional steps | default []} | flatten)
    if ($steps | is-empty) {
        return {cargo: [], guards: [], problems: [$"($WORKFLOW): has no job with a `steps:` list, so nothing was compared."]}
    }
    let cmds = (
        $steps
        | each {|s| $s | get --optional run | default ""}
        | where {|r| $r != ""}
        | each {|r| $r | lines}
        | flatten
        | each {|l| $l | str trim}
        | where {|l| ($l != "") and (not ($l | str starts-with "#"))}
    )
    extract-legs $WORKFLOW $cmds
}

def canon-set [legs: list<any>]: nothing -> list<string> {
    if ($legs | is-empty) { [] } else { $legs | get canonical | uniq | sort }
}

# A clippy leg with no `--deny warnings` prints its findings and exits 0, and a
# configuration no clippy leg covers is code nothing lints at all.
def clippy-violations [source: string, legs: list<any>]: nothing -> list<string> {
    let clippy = ($legs | where {|l| $l.subcommand == "clippy"})
    mut found = []
    if ($clippy | is-empty) {
        return [$"($source): runs no `cargo clippy` leg at all"]
    }
    for leg in $clippy {
        if not $leg.deny_warnings {
            $found = ($found | append $"($source): `($leg.canonical)` passes no `--deny warnings` \(or `-D warnings`), so its findings stay warnings and the leg exits 0 anyway")
        }
    }
    for want in $REQUIRED_CLIPPY {
        let hit = ($clippy | where {|l|
            ($l.features == $want.features) and ($l.target == $want.target) and ($l.all_targets == $want.all_targets)
        })
        if ($hit | is-empty) {
            let hint = if ($want.target | is-empty) {
                ""
            } else {
                " `.cargo/config.toml` pins `[build] target = \"x86_64-unknown-linux-gnu\"`, so a leg that lost `--target` silently re-lints the host build two other legs already cover (LINKS-39)."
            }
            $found = ($found | append $"($source): no clippy leg covers `(describe-config $want)`, so that compilation configuration is linted by nothing.($hint)")
        }
    }
    $found
}

# Compare both directions, plus the per-file invariants that survive a drift
# applied to both copies at once.
def violations [just: record, ci: record]: nothing -> list<string> {
    mut found = ($just.problems | append $ci.problems)

    let jc = (canon-set $just.cargo)
    let cc = (canon-set $ci.cargo)
    let jg = (canon-set $just.guards)
    let cg = (canon-set $ci.guards)
    let jg_scripts = (if ($just.guards | is-empty) { [] } else { $just.guards | get script | uniq })
    let cg_scripts = (if ($ci.guards | is-empty) { [] } else { $ci.guards | get script | uniq })

    let just_label = $"($JUSTFILE) `($RECIPE)`"
    if ($jc | length) < $MIN_CARGO_LEGS {
        $found = ($found | append $"($just_label): parsed only ($jc | length) cargo legs and the floor is ($MIN_CARGO_LEGS), so the recipe shape changed and the comparison would have been vacuous")
    }
    if ($cc | length) < $MIN_CARGO_LEGS {
        $found = ($found | append $"($WORKFLOW): parsed only ($cc | length) cargo legs and the floor is ($MIN_CARGO_LEGS), so the step shape changed and the comparison would have been vacuous")
    }
    if ($jg_scripts | length) < $MIN_GUARD_SCRIPTS {
        $found = ($found | append $"($just_label): parsed only ($jg_scripts | length) guard scripts and the floor is ($MIN_GUARD_SCRIPTS)")
    }
    if ($cg_scripts | length) < $MIN_GUARD_SCRIPTS {
        $found = ($found | append $"($WORKFLOW): parsed only ($cg_scripts | length) guard scripts and the floor is ($MIN_GUARD_SCRIPTS)")
    }

    for leg in $jc {
        if $leg not-in $cc {
            $found = ($found | append $"`($leg)` runs in ($just_label) but not in ($WORKFLOW)")
        }
    }
    for leg in $cc {
        if $leg not-in $jc {
            $found = ($found | append $"`($leg)` runs in ($WORKFLOW) but not in ($just_label)")
        }
    }
    for g in $jg {
        if $g not-in $cg {
            $found = ($found | append $"`($g)` runs in ($just_label) but not in ($WORKFLOW)")
        }
    }
    for g in $cg {
        if $g not-in $jg {
            $found = ($found | append $"`($g)` runs in ($WORKFLOW) but not in ($just_label)")
        }
    }

    $found
    | append (clippy-violations $just_label $just.cargo)
    | append (clippy-violations $WORKFLOW $ci.cargo)
}

# A miniature of the real suite: the justfile spellings on one side, the
# workflow spellings on the other, so a matching pair proves the normalisation
# rather than proving the two texts are identical.
def sample-justfile []: nothing -> string {
    [
        '# A comment above the recipe.'
        'pre-commit: ensure-env ensure-css'
        '    #!/usr/bin/env nu'
        '    print "\n[pre-commit] cargo clippy --all-targets --features bogus -- --deny warnings"'
        '    ^nu scripts/check-migration-immutability.nu'
        '    ^nu scripts/check-migration-docs.nu --self-test'
        '    ^nu scripts/check-migration-docs.nu'
        '    ^nu scripts/check-build-flags.nu'
        '    ^nu scripts/check-dependency-audit.nu --self-test'
        '    ^nu scripts/check-dependency-audit.nu --runner "docker compose --file compose.dev.yml run --rm --no-deps app"'
        '    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo fmt --check'
        '    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo clippy --all-targets -- --deny warnings'
        '    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo clippy --all-targets --features server -- --deny warnings'
        '    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings'
        '    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo build --all-targets'
        '    ^docker compose --file compose.dev.yml run --rm --no-deps app cargo build --all-targets --features server'
        '    ^docker compose --file compose.dev.yml run --rm app cargo test --lib'
        '    ^docker compose --file compose.dev.yml run --rm app cargo test --features server --lib'
        '    ^docker compose --file compose.dev.yml run --rm app cargo test --features server --test db_schema'
        '    ^nu scripts/check-doc-tests-ran.nu --self-test'
        '    ^nu scripts/check-doc-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm --no-deps app"'
        '    ^nu scripts/check-db-tests-ran.nu --self-test'
        '    ^nu scripts/check-db-tests-ran.nu --runner "docker compose --file compose.dev.yml run --rm app"'
        ''
        'unrelated-recipe:'
        '    cargo doc --open'
    ] | str join "\n"
}

def sample-workflow []: nothing -> string {
    [
        'name: Check'
        'jobs:'
        '  check:'
        '    steps:'
        '      - name: Clone the repository'
        '        uses: https://code.forgejo.org/actions/checkout@v5'
        '      - name: Guard migration immutability'
        '        run: nu scripts/check-migration-immutability.nu'
        '      - name: Guard the migration history doc'
        '        run: |'
        '          nu scripts/check-migration-docs.nu --self-test'
        '          nu scripts/check-migration-docs.nu'
        '      - name: Guard justfile build flags'
        '        run: nu scripts/check-build-flags.nu'
        '      - name: Dependency audit'
        '        run: |'
        '          nu scripts/check-dependency-audit.nu --self-test'
        '          nu scripts/check-dependency-audit.nu'
        '      - name: Cap build parallelism'
        '        run: echo "CARGO_BUILD_JOBS=$(($(nproc) / 2))" >> "$GITHUB_ENV"'
        '      - name: Check formatting'
        '        run: cargo fmt --check'
        '      - name: Clippy (default features)'
        '        run: cargo clippy --all-targets -- -D warnings'
        '      - name: Clippy (server)'
        '        run: cargo clippy --all-targets --features=server -- -D warnings'
        '      - name: Clippy (web/wasm)'
        '        run: cargo clippy --all-targets --features web --target=wasm32-unknown-unknown -- -D warnings'
        '      - name: Build (default features)'
        '        run: cargo build --all-targets'
        '      - name: Build (server)'
        '        run: cargo build --all-targets --features server'
        '      - name: Unit tests (default features)'
        '        run: cargo test --lib'
        '      - name: Unit tests (server)'
        '        run: cargo test --features server --lib'
        '      - name: Doc tests (server)'
        '        run: |'
        '          nu scripts/check-doc-tests-ran.nu --self-test'
        '          nu scripts/check-doc-tests-ran.nu'
        '      - name: Apply migrations to the test database'
        '        run: cargo test --features server --test db_schema'
        '      - name: Integration tests (Postgres-backed)'
        '        run: |'
        '          nu scripts/check-db-tests-ran.nu --self-test'
        '          nu scripts/check-db-tests-ran.nu'
    ] | str join "\n"
}

def expect-rejected [label: string, just: record, ci: record] {
    if (violations $just $ci | is-empty) {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: ($label) was accepted."
        exit 1
    }
}

# Prove the parser still reads both real files and that each drift is still
# rejected. Without it a drifted parser passes every job silently, which is the
# same blindness one level up that this guard exists to remove.
def run-self-test [] {
    let just = (justfile-legs (sample-justfile))
    let ci = (workflow-legs (sample-workflow))

    if ($just.problems | is-not-empty) or ($ci.problems | is-not-empty) {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: the parser rejected a well-formed pair: (($just.problems | append $ci.problems) | str join '; ')"
        exit 1
    }
    if ($just.cargo | length) != 9 or ($ci.cargo | length) != 9 {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: the parser no longer reads the real shape; it found ($just.cargo | length) justfile legs and ($ci.cargo | length) workflow legs, expected 9 each. A `print` line or the `docker compose` prefix is probably being counted."
        exit 1
    }
    let wasm = ($just.cargo | where {|l| $l.target == "wasm32-unknown-unknown"})
    if ($wasm | length) != 1 or ($wasm | first | get canonical) != "cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings" {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: the canonical form drifted: ($wasm | to json --raw)"
        exit 1
    }

    # `-D warnings` vs `--deny warnings` and `--features=server` vs
    # `--features server` are the same leg, so the matching pair must pass.
    let accepted = (violations $just $ci)
    if ($accepted | is-not-empty) {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: a matching pair was rejected: ($accepted | str join '; ')"
        exit 1
    }

    # The LINKS-39 scenario: --target dropped from one copy re-lints the host build.
    let no_target = (justfile-legs (sample-justfile | str replace " --target wasm32-unknown-unknown" ""))
    expect-rejected "a justfile clippy leg with --target dropped" $no_target $ci
    let ci_no_target = (workflow-legs (sample-workflow | str replace " --target=wasm32-unknown-unknown" ""))
    expect-rejected "a workflow clippy leg with --target dropped" $just $ci_no_target

    # --deny warnings dropped from one copy: the leg runs and exits 0 anyway.
    let no_deny = (workflow-legs (sample-workflow | str replace " -- -D warnings" "" --all))
    expect-rejected "workflow clippy legs with no --deny warnings" $just $no_deny

    # A leg present in one file only, in either direction.
    let extra_ci = (workflow-legs (sample-workflow | str replace "      - name: Check formatting" "      - name: Extra\n        run: cargo test --features server --test route_surface\n      - name: Check formatting"))
    expect-rejected "a leg in the workflow but not the justfile" $just $extra_ci
    let fewer_just = (justfile-legs (sample-justfile | str replace --regex '(?m)^.*cargo test --features server --lib.*\n' ''))
    expect-rejected "a leg in the workflow but missing from the justfile" $fewer_just $ci

    # A guard script invoked in one file only.
    let no_guard = (justfile-legs (sample-justfile | str replace --regex '(?m)^.*check-build-flags\.nu.*\n' ''))
    expect-rejected "a guard script the justfile stopped running" $no_guard $ci

    # A flag change that is a real difference rather than a synonym.
    let all_features = (justfile-legs (sample-justfile | str replace "cargo build --all-targets --features server" "cargo build --all-targets --all-features"))
    expect-rejected "a justfile leg whose flags genuinely differ" $all_features $ci

    # Nothing parsed must fail rather than compare [] to [].
    let renamed = (justfile-legs (sample-justfile | str replace "pre-commit: ensure-env ensure-css" "precommit: ensure-env ensure-css"))
    if ($renamed.problems | is-empty) {
        print --stderr "[check-suite-parity] SELF-TEST FAILED: a renamed recipe was accepted instead of failing loudly."
        exit 1
    }
    expect-rejected "a justfile with no pre-commit recipe" $renamed $ci
    let no_jobs = (workflow-legs "name: Check\non:\n  push:\n    branches: [main]\n")
    if ($no_jobs.problems | is-empty) {
        print --stderr "[check-suite-parity] SELF-TEST FAILED: a workflow with no jobs was accepted instead of failing loudly."
        exit 1
    }
    expect-rejected "a workflow with no jobs" $just $no_jobs
    expect-rejected "two empty sides" $no_jobs $no_jobs

    print "[check-suite-parity] SELF-TEST OK: a dropped --target, a dropped --deny warnings, a one-sided leg, a one-sided guard, a genuinely different flag and an empty parse are all rejected; a matching pair written in each file's own flag spellings is not."
}

export def main [
    --self-test # check the guard still detects drift, then exit
] {
    if $self_test {
        run-self-test
        return
    }

    if not ("Cargo.toml" | path exists) {
        print --stderr "[check-suite-parity] FAILED: run this from the repository root."
        exit 1
    }
    for f in [$JUSTFILE $WORKFLOW] {
        if not ($f | path exists) {
            print --stderr $"[check-suite-parity] FAILED: ($f) does not exist."
            exit 1
        }
    }

    let just = (justfile-legs (open --raw $JUSTFILE))
    let ci = (workflow-legs (open --raw $WORKFLOW))
    let found = (violations $just $ci)

    if ($found | is-not-empty) {
        print --stderr "[check-suite-parity] FAILED:"
        for problem in $found {
            print --stderr $"  - ($problem)"
        }
        print --stderr ""
        print --stderr $"The `($RECIPE)` recipe in ($JUSTFILE) and the steps in ($WORKFLOW) are two copies of one check suite. Every cargo leg and every guard script in one must appear in the other, so `just ($RECIPE)` runs what CI runs. Fix whichever copy is wrong, or change both together."
        exit 1
    }

    let legs = (canon-set $just.cargo)
    let scripts = ($just.guards | get script | uniq)
    print ($legs | wrap leg)
    print $"[check-suite-parity] OK: ($legs | length) cargo legs and ($scripts | length) guard scripts run in both ($JUSTFILE) `($RECIPE)` and ($WORKFLOW)."
}
