#!/usr/bin/env nu

# Reject drift between the hand-maintained copies of the check suite: the
# `pre-commit` and `check` recipes in `justfile` and the steps in
# `.forgejo/workflows/check.yml`. Every cargo leg and every guard script in
# `pre-commit` must appear in the workflow and the reverse, so that comparison
# runs in both directions.
#
# `check` is the third copy: the host-side lint pass, which LINKS-42 grew into the
# same three clippy configurations CI runs. It is compared one way, because it
# deliberately omits the build, test, doc-test and database legs that need the
# compose stack: every clippy configuration and the fmt leg the workflow runs must
# appear in `check`, and nothing demands the reverse. Before LINKS-64 the recipe was
# outside the guard entirely, so deleting its `--features server` clippy leg left
# both this guard and scripts/check-build-flags.nu at exit 0.
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
const CHECK_RECIPE = "check"

# Floors, set AT the real counts rather than below them: nine cargo legs and
# eight guard scripts run today. They exist so a parser that stopped matching
# fails instead of comparing [] to [], and so a leg deleted from BOTH copies at
# once fails too, which the two-way comparison cannot catch on its own because
# both sides shrink together. Slack is exactly the size of that hole: at a floor
# of 7 a both-sided drop to seven guards still passed. Removing a leg on purpose
# therefore means lowering the number here in the same commit, where a reviewer
# sees it.
#
# Raising them means extending the known-good fixtures first. sample-justfile and
# sample-workflow are what the self-test proves a matching set against, so a floor
# above what they enumerate fails the self-test rather than the real files. That
# coupling, not neglect, is why MIN_GUARD_SCRIPTS sat at 5 while eight guards ran.
const MIN_CARGO_LEGS = 9
const MIN_GUARD_SCRIPTS = 8

# `check` resolves to three clippy configurations plus fmt. Same reasoning: a
# recipe renamed, emptied, or stripped of a dependency has to fail here rather
# than compare an empty set to the workflow's lint legs (LINKS-64).
const MIN_CHECK_LEGS = 4

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

# The dependency list on a recipe header: everything after the first colon.
# `(ensure-env mode)` is one dependency carrying an argument, so keep the name and
# drop the argument.
def recipe-deps [header: string]: nothing -> list<string> {
    $header
    | split row ":"
    | skip 1
    | str join ":"
    | str replace --regex --all '\(\s*([A-Za-z0-9_-]+)[^)]*\)' '$1'
    | split row --regex '\s+'
    | each {|d| $d | str trim}
    | where {|d| $d =~ '^[A-Za-z0-9_-]+$'}
}

# A recipe's own command lines plus, breadth first, those of the recipes it depends
# on. `check: check-web check-clippy check-fmt` has an empty body and all of its legs
# in its dependencies; `just --dry-run check` prints the same expansion. A dependency
# naming a recipe that does not exist is reported, never skipped: renaming
# `check-clippy` would otherwise shrink the comparison in silence, which is the whole
# failure mode LINKS-64 exists to close.
def recipe-cmds [text: string, recipe: string]: nothing -> record {
    let lines = ($text | lines)
    mut pending = [$recipe]
    mut seen = []
    mut cmds = []
    mut problems = []

    while ($pending | is-not-empty) {
        let name = ($pending | first)
        $pending = ($pending | skip 1)
        if $name in $seen { continue }
        $seen = ($seen | append $name)

        # The name must be followed by whitespace or the colon, so `check` does not
        # match `check-web` and a parameterised header (`ensure-env mode="x":`) does.
        let starts = ($lines | enumerate | where {|r| $r.item =~ ('^' + $name + '(\s|:)')} | get index)
        if ($starts | is-empty) {
            $problems = ($problems | append $"($JUSTFILE): found no `($name):` recipe. It was renamed or removed, so nothing was compared.")
            continue
        }

        let at = ($starts | first)
        let after = ($lines | skip ($at + 1))
        let ends = (
            $after
            | enumerate
            | where {|r| (($r.item | str trim) != "") and (not ($r.item =~ '^\s'))}
            | get index
        )
        let body = if ($ends | is-empty) { $after } else { $after | first ($ends | first) }
        $cmds = ($cmds | append (
            $body
            | each {|l| $l | str trim}
            | where {|l| ($l != "") and (not ($l | str starts-with "#"))}
        ))
        $pending = ($pending | append (recipe-deps ($lines | get $at)))
    }

    {cmds: $cmds, problems: $problems}
}

# The command lines a recipe runs, its dependency recipes included. A missing recipe
# is a parse failure, not an empty suite.
def justfile-legs [text: string, recipe: string]: nothing -> record {
    let resolved = (recipe-cmds $text $recipe)
    let legs = (extract-legs $"($JUSTFILE) `($recipe)`" $resolved.cmds)
    {
        cargo: $legs.cargo
        guards: $legs.guards
        problems: ($resolved.problems | append $legs.problems)
    }
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

# `check` is the host-side lint subset of the workflow, so the comparison runs one
# way: every clippy configuration and the fmt leg the workflow runs must appear in
# `check`, and nothing demands the reverse. A two-way comparison would demand that
# `check` grow the build, test, doc-test and database legs it exists to leave out,
# which need the compose stack. `clippy-violations` is reused unchanged, so `check`
# is held to the same "every configuration is linted, every clippy leg denies
# warnings" invariant as the other two copies (LINKS-64).
def check-violations [check: record, ci: record]: nothing -> list<string> {
    let label = $"($JUSTFILE) `($CHECK_RECIPE)`"
    mut found = $check.problems

    let legs = (canon-set $check.cargo)
    if ($legs | length) < $MIN_CHECK_LEGS {
        $found = ($found | append $"($label): parsed only ($legs | length) cargo legs and the floor is ($MIN_CHECK_LEGS) \(three clippy configurations plus fmt), so the recipe or one of its dependency recipes changed shape and the comparison would have been vacuous")
    }

    for leg in (canon-set ($ci.cargo | where {|l| $l.subcommand in ["clippy" "fmt"]})) {
        if $leg not-in $legs {
            $found = ($found | append $"`($leg)` runs in ($WORKFLOW) but not in ($label), so `just ($CHECK_RECIPE)` no longer lints what CI lints")
        }
    }

    $found | append (clippy-violations $label $check.cargo)
}

# Compare both directions, plus the per-file invariants that survive a drift
# applied to both copies at once, plus the one-way `check` comparison.
def violations [just: record, ci: record, check: record]: nothing -> list<string> {
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
        $found = ($found | append $"($just_label): parsed only ($jg_scripts | length) guard scripts and the floor is ($MIN_GUARD_SCRIPTS), so a guard was dropped from both copies at once or the recipe shape changed")
    }
    if ($cg_scripts | length) < $MIN_GUARD_SCRIPTS {
        $found = ($found | append $"($WORKFLOW): parsed only ($cg_scripts | length) guard scripts and the floor is ($MIN_GUARD_SCRIPTS), so a guard was dropped from both copies at once or the step shape changed")
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
    | append (check-violations $check $ci)
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
        '    ^nu scripts/check-suite-parity.nu --self-test'
        '    ^nu scripts/check-suite-parity.nu'
        '    ^nu scripts/check-migration-immutability.nu'
        '    ^nu scripts/check-migration-docs.nu --self-test'
        '    ^nu scripts/check-migration-docs.nu'
        '    ^nu scripts/check-build-flags.nu'
        '    ^nu scripts/check-dev-clean-volumes.nu --self-test'
        '    ^nu scripts/check-dev-clean-volumes.nu'
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
        '[private]'
        'ensure-env mode="standalone":'
        '    @test -f .env || cp .env.{{ mode }}.example .env'
        ''
        '[private]'
        'ensure-css:'
        '    @test -f assets/tailwind.css || touch assets/tailwind.css'
        ''
        '# The host-side lint pass, compared one way against the workflow (LINKS-64).'
        'check: check-web check-clippy check-fmt'
        ''
        'check-web: ensure-css'
        '    cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings'
        ''
        'check-clippy: ensure-css'
        '    cargo clippy --all-targets -- --deny warnings'
        '    cargo clippy --all-targets --features server -- --deny warnings'
        ''
        'check-fmt:'
        '    cargo fmt --check'
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
        '      - name: Guard the dev-clean volume list'
        '        run: |'
        '          nu scripts/check-dev-clean-volumes.nu --self-test'
        '          nu scripts/check-dev-clean-volumes.nu'
        '      - name: Guard check-suite parity'
        '        run: |'
        '          nu scripts/check-suite-parity.nu --self-test'
        '          nu scripts/check-suite-parity.nu'
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

def expect-rejected [label: string, found: list<string>] {
    if ($found | is-empty) {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: ($label) was accepted."
        exit 1
    }
}

# Reject, and reject by the rule the fixture is aimed at. A fixture that trips some
# other comparison first leaves its own invariant unreached while the self-test still
# passes, which is how a guard grows a rule nothing ever exercises.
def expect-rejected-by [label: string, needle: string, found: list<string>] {
    expect-rejected $label $found
    if ($found | where {|f| $f | str contains $needle} | is-empty) {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: ($label) was rejected, but by no rule whose message contains `($needle)`, so the invariant the fixture aims at was never reached. Got: ($found | str join '; ')"
        exit 1
    }
}

# Prove the parser still reads both real files and that each drift is still
# rejected. Without it a drifted parser passes every job silently, which is the
# same blindness one level up that this guard exists to remove.
def run-self-test [] {
    let just = (justfile-legs (sample-justfile) $RECIPE)
    let ci = (workflow-legs (sample-workflow))
    let check = (justfile-legs (sample-justfile) $CHECK_RECIPE)

    if ($just.problems | is-not-empty) or ($ci.problems | is-not-empty) or ($check.problems | is-not-empty) {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: the parser rejected a well-formed set: (($just.problems | append $ci.problems | append $check.problems) | str join '; ')"
        exit 1
    }
    if ($just.cargo | length) != 9 or ($ci.cargo | length) != 9 {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: the parser no longer reads the real shape; it found ($just.cargo | length) justfile legs and ($ci.cargo | length) workflow legs, expected 9 each. A `print` line or the `docker compose` prefix is probably being counted."
        exit 1
    }
    # `check` has an empty body: every one of its legs comes from a dependency recipe,
    # so a resolver that stopped following dependencies reads it as running nothing.
    if ($check.cargo | length) != 4 {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: `($CHECK_RECIPE)` resolved to ($check.cargo | length) cargo legs, expected 4 \(three clippy configurations plus fmt). Its dependency recipes are not being followed."
        exit 1
    }
    let wasm = ($just.cargo | where {|l| $l.target == "wasm32-unknown-unknown"})
    if ($wasm | length) != 1 or ($wasm | first | get canonical) != "cargo clippy --all-targets --features web --target wasm32-unknown-unknown -- --deny warnings" {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: the canonical form drifted: ($wasm | to json --raw)"
        exit 1
    }

    # `-D warnings` vs `--deny warnings` and `--features=server` vs
    # `--features server` are the same leg, so the matching pair must pass.
    let accepted = (violations $just $ci $check)
    if ($accepted | is-not-empty) {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: a matching set was rejected: ($accepted | str join '; ')"
        exit 1
    }

    # The LINKS-39 scenario: --target dropped from one copy re-lints the host build.
    let no_target = (justfile-legs (sample-justfile | str replace " --target wasm32-unknown-unknown" "") $RECIPE)
    expect-rejected "a justfile clippy leg with --target dropped" (violations $no_target $ci $check)
    let ci_no_target = (workflow-legs (sample-workflow | str replace " --target=wasm32-unknown-unknown" ""))
    expect-rejected "a workflow clippy leg with --target dropped" (violations $just $ci_no_target $check)

    # --deny warnings dropped from one copy: the leg runs and exits 0 anyway. The needle
    # keeps this on the rule it is aimed at, now that a mutated workflow also moves the
    # one-way `check` comparison.
    let no_deny = (workflow-legs (sample-workflow | str replace " -- -D warnings" "" --all))
    expect-rejected-by "workflow clippy legs with no --deny warnings" "passes no `--deny warnings`" (violations $just $no_deny $check)

    # A leg present in one file only, in either direction.
    let extra_ci = (workflow-legs (sample-workflow | str replace "      - name: Check formatting" "      - name: Extra\n        run: cargo test --features server --test route_surface\n      - name: Check formatting"))
    expect-rejected "a leg in the workflow but not the justfile" (violations $just $extra_ci $check)
    let fewer_just = (justfile-legs (sample-justfile | str replace --regex '(?m)^.*cargo test --features server --lib.*\n' '') $RECIPE)
    expect-rejected "a leg in the workflow but missing from the justfile" (violations $fewer_just $ci $check)

    # A guard script invoked in one file only.
    let no_guard = (justfile-legs (sample-justfile | str replace --regex '(?m)^.*check-build-flags\.nu.*\n' '') $RECIPE)
    expect-rejected "a guard script the justfile stopped running" (violations $no_guard $ci $check)

    # A flag change that is a real difference rather than a synonym.
    let all_features = (justfile-legs (sample-justfile | str replace "cargo build --all-targets --features server" "cargo build --all-targets --all-features") $RECIPE)
    expect-rejected "a justfile leg whose flags genuinely differ" (violations $all_features $ci $check)

    # Nothing parsed must fail rather than compare [] to [].
    let renamed = (justfile-legs (sample-justfile | str replace "pre-commit: ensure-env ensure-css" "precommit: ensure-env ensure-css") $RECIPE)
    if ($renamed.problems | is-empty) {
        print --stderr "[check-suite-parity] SELF-TEST FAILED: a renamed recipe was accepted instead of failing loudly."
        exit 1
    }
    expect-rejected "a justfile with no pre-commit recipe" (violations $renamed $ci $check)
    let no_jobs = (workflow-legs "name: Check\non:\n  push:\n    branches: [main]\n")
    if ($no_jobs.problems | is-empty) {
        print --stderr "[check-suite-parity] SELF-TEST FAILED: a workflow with no jobs was accepted instead of failing loudly."
        exit 1
    }
    expect-rejected "a workflow with no jobs" (violations $just $no_jobs $check)
    expect-rejected "two empty sides" (violations $no_jobs $no_jobs $check)

    # LINKS-64. Deleting the server clippy leg from `check-clippy` left this guard and
    # scripts/check-build-flags.nu both at exit 0 before this change. Each case asserts
    # the rule it aims at actually fired, so a fixture caught by some other comparison
    # first cannot leave the new invariant unreached.
    let check_no_server = (justfile-legs (sample-justfile | str replace "\n    cargo clippy --all-targets --features server -- --deny warnings" "") $CHECK_RECIPE)
    expect-rejected-by "a `check` that lost its --features server clippy leg" "no clippy leg covers `--all-targets --features server`" (violations $just $ci $check_no_server)

    let check_no_deny = (justfile-legs (sample-justfile | str replace "\n    cargo clippy --all-targets --features server -- --deny warnings" "\n    cargo clippy --all-targets --features server") $CHECK_RECIPE)
    expect-rejected-by "a `check` clippy leg with no --deny warnings" "passes no `--deny warnings`" (violations $just $ci $check_no_deny)

    let check_no_fmt = (justfile-legs (sample-justfile | str replace "check: check-web check-clippy check-fmt" "check: check-web check-clippy") $CHECK_RECIPE)
    expect-rejected-by "a `check` that stopped running cargo fmt" "`cargo fmt --check` runs in" (violations $just $ci $check_no_fmt)

    # A dependency recipe renamed away: `check` still resolves, just to fewer legs.
    let check_dep_gone = (justfile-legs (sample-justfile | str replace "check-clippy: ensure-css" "check-lints: ensure-css") $CHECK_RECIPE)
    if ($check_dep_gone.problems | is-empty) {
        print --stderr "[check-suite-parity] SELF-TEST FAILED: a `check` dependency recipe that no longer exists was accepted instead of failing loudly."
        exit 1
    }
    expect-rejected-by "a `check` dependency recipe renamed away" "found no `check-clippy:` recipe" (violations $just $ci $check_dep_gone)

    # `check` itself renamed away must fail loudly AND trip the floor, rather than
    # comparing an empty set to the workflow's lint legs.
    let check_gone = (justfile-legs (sample-justfile | str replace "check: check-web check-clippy check-fmt" "lint: check-web check-clippy check-fmt") $CHECK_RECIPE)
    if ($check_gone.problems | is-empty) {
        print --stderr "[check-suite-parity] SELF-TEST FAILED: a justfile with no `check` recipe was accepted instead of failing loudly."
        exit 1
    }
    let check_gone_found = (violations $just $ci $check_gone)
    expect-rejected-by "a justfile with no `check` recipe" "found no `check:` recipe" $check_gone_found
    expect-rejected-by "a `check` recipe that parsed nothing" "cargo legs and the floor is" $check_gone_found

    # `check` deliberately omits the workflow's build, test, doc-test and database legs,
    # which need the compose stack. Prove the sample workflow actually runs some, so
    # `check` not being asked for them is a property rather than an empty statement.
    let excluded = (canon-set ($ci.cargo | where {|l| $l.subcommand not-in ["clippy" "fmt"]}))
    if ($excluded | length) < 3 {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: the sample workflow runs ($excluded | length) non-lint cargo legs, so `($CHECK_RECIPE)` not being asked for them proves nothing."
        exit 1
    }
    let asked = (check-violations $check $ci)
    if ($asked | is-not-empty) {
        print --stderr $"[check-suite-parity] SELF-TEST FAILED: `($CHECK_RECIPE)` matches every lint leg of the workflow but was rejected: ($asked | str join '; ')"
        exit 1
    }

    print $"[check-suite-parity] SELF-TEST OK: a dropped --target, a dropped --deny warnings, a one-sided leg, a one-sided guard, a genuinely different flag and an empty parse are all rejected; in `($CHECK_RECIPE)` a lost --features server clippy leg, a lost --deny warnings, a lost fmt leg, a renamed dependency recipe and a renamed recipe are all rejected, each by the rule it is aimed at; a matching set written in each file's own flag spellings, and a `($CHECK_RECIPE)` that omits the workflow's ($excluded | length) non-lint legs, are not."
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

    let just = (justfile-legs (open --raw $JUSTFILE) $RECIPE)
    let ci = (workflow-legs (open --raw $WORKFLOW))
    let check = (justfile-legs (open --raw $JUSTFILE) $CHECK_RECIPE)
    let found = (violations $just $ci $check)

    if ($found | is-not-empty) {
        print --stderr "[check-suite-parity] FAILED:"
        for problem in $found {
            print --stderr $"  - ($problem)"
        }
        print --stderr ""
        print --stderr $"The `($RECIPE)` recipe in ($JUSTFILE) and the steps in ($WORKFLOW) are two copies of one check suite, and the `($CHECK_RECIPE)` recipe is a third copy of its lint legs. Every cargo leg and every guard script in `($RECIPE)` must appear in ($WORKFLOW) and the reverse, so `just ($RECIPE)` runs what CI runs; every clippy configuration and the fmt leg ($WORKFLOW) runs must appear in `($CHECK_RECIPE)`, so `just ($CHECK_RECIPE)` lints what CI lints. Fix whichever copy is wrong, or change them together."
        exit 1
    }

    let legs = (canon-set $just.cargo)
    let scripts = ($just.guards | get script | uniq)
    let check_legs = (canon-set $check.cargo)
    print ($legs | wrap leg)
    print $"[check-suite-parity] OK: ($legs | length) cargo legs and ($scripts | length) guard scripts run in both ($JUSTFILE) `($RECIPE)` and ($WORKFLOW), and ($JUSTFILE) `($CHECK_RECIPE)` runs the ($check_legs | length) lint legs of ($WORKFLOW)."
}
