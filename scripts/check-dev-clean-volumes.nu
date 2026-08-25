#!/usr/bin/env nu

# Reject drift between the named volumes the compose files declare and the list
# `just dev-clean` removes. They are two hand-maintained copies of one list, so
# the comparison runs in both directions: a volume `dev-clean` names that no
# compose file declares, and a declared volume `dev-clean` never removes.
#
# Why this exists: the drift has already landed twice, and a human reading the
# file caught both. LINKS-60 removed `rusty-links-target-server-${USER}` and
# `rusty-links-target-wasm-${USER}` from the compose file and the recipe
# together; before that the former `clean` recipe listed `app-data` and
# `db-data`, which neither compose file declared. A stale entry is a silent
# no-op, and a missing one leaves a volume behind that `dev-clean` promises to
# remove, so the next `just dev-up` reuses a cache the developer believes they
# deleted.
#
# Static like `scripts/check-build-flags.nu` and `scripts/check-suite-parity.nu`:
# no docker, no compose, no compilation, so it runs in the cheap phase of
# `pre-commit` and fails in milliseconds.
#
# Usage:
#   nu scripts/check-dev-clean-volumes.nu --self-test
#   nu scripts/check-dev-clean-volumes.nu
#
# Exit codes:
# - 0: every declared volume is removed by `dev-clean` and every volume it names is declared.
# - 1: they diverge, or either file could not be parsed. A parse that stops
#      matching is a failure, never a silent pass: zero names on both sides would
#      otherwise compare equal and report "no drift".

const JUSTFILE = "justfile"
const RECIPE = "dev-clean"
const COMPOSE_FILES = ["compose.dev.yml" "compose.yml"]

# Floor, not a target: compose.yml declares 3 named volumes today, compose.dev.yml
# 4, and `dev-clean` names 4. It exists so a parser that stopped matching fails
# instead of comparing [] to []. Raise it as the files grow rather than lowering
# it after a near miss.
const MIN_VOLUMES = 3

# The per-user suffix is written `${USER}` in compose and `($suffix)` in the
# recipe, each in its own file's syntax. Normalise both to one placeholder so the
# two lists are comparable rather than trivially different.
const USER_PLACEHOLDER = "<user>"

def normalise [name: string]: nothing -> string {
    $name
    | str replace --all '${USER}' $USER_PLACEHOLDER
    | str replace --all '($suffix)' $USER_PLACEHOLDER
}

# Collect the `name:` of every entry in a compose file's top-level `volumes:`
# block. Docker uses the key as the name when `name:` is absent, but every entry
# in this repository sets it explicitly, so an entry without one is drift rather
# than a shorthand and is reported as such.
def compose-volumes [text: string, label: string]: nothing -> record {
    let doc = (try { $text | from yaml } catch { null })
    if $doc == null {
        return {label: $label, names: [], problems: [$"($label) is not readable as YAML, so its volumes could not be compared"]}
    }
    if ($doc | describe | str starts-with "record") == false {
        return {label: $label, names: [], problems: [$"($label) does not parse to a YAML mapping, so its shape changed"]}
    }
    let vols = ($doc | get --optional volumes)
    if $vols == null {
        return {label: $label, names: [], problems: [$"($label) has no top-level `volumes:` block, so its shape changed and the comparison would have been vacuous"]}
    }
    if ($vols | describe | str starts-with "record") == false {
        return {label: $label, names: [], problems: [$"($label) has a top-level `volumes:` that is not a mapping, so its shape changed"]}
    }
    let rows = ($vols | items {|key, body|
        let name = if ($body | describe | str starts-with "record") { $body | get --optional name } else { null }
        {key: $key, name: $name}
    })
    let unnamed = ($rows | where name == null | get key)
    if ($unnamed | is-not-empty) {
        return {label: $label, names: [], problems: [$"($label) declares ($unnamed | str join ', ') with no `name:` field, so the volume this repository actually creates cannot be read from the file"]}
    }
    {label: $label, names: ($rows | get name | each {|n| normalise $n} | uniq), problems: []}
}

# Pull the `vols` list literal out of the `dev-clean` recipe body. Recipe bodies
# are indented, so the body runs to the next line that starts at column 0.
def recipe-volumes [text: string]: nothing -> record {
    let label = $"the `($RECIPE)` recipe in ($JUSTFILE)"
    # Recipe bodies are indented, so the body runs to the next column-0 line or to
    # the end of the file. Built by concatenation: a raw string keeps the regex
    # escapes out of nu's string interpolation.
    let pattern = ('(?ms)^' + $RECIPE + ':.*?\n(?<body>.*?)(?=\n[^\s\n]|\z)')
    let bodies = ($text | parse --regex $pattern)
    if ($bodies | is-empty) {
        return {names: [], problems: [$"($JUSTFILE) has no `($RECIPE):` recipe, so there is no volume list to compare"]}
    }
    let body = ($bodies | get body.0)
    let blocks = ($body | parse --regex '(?ms)let vols = \[(?<inner>.*?)\]')
    if ($blocks | is-empty) {
        return {names: [], problems: [$"($label) has no `let vols = [...]` list, so the recipe changed shape and the comparison would have been vacuous"]}
    }
    let names = ($blocks | get inner.0 | parse --regex '\$"(?<name>[^"]+)"' | get name)
    if ($names | is-empty) {
        return {names: [], problems: [$"($label) has a `let vols = [...]` list holding no `$\"...\"` volume names, so the comparison would have been vacuous"]}
    }
    {names: ($names | each {|n| normalise $n} | uniq), problems: []}
}

# Every way the two lists are a failure: an unreadable file, a list too short to
# have been parsed, a recipe entry nothing declares, a declared volume the recipe
# never removes.
def violations [files: list<any>, recipe: record]: nothing -> list<string> {
    let parse_problems = (($files | each {|f| $f.problems} | flatten) | append $recipe.problems)
    if ($parse_problems | is-not-empty) { return $parse_problems }

    mut found = []
    for f in $files {
        if ($f.names | length) < $MIN_VOLUMES {
            $found = ($found | append $"($f.label) parsed only ($f.names | length) volume names and the floor is ($MIN_VOLUMES), so the parser stopped matching and the comparison would have been vacuous")
        }
    }
    if ($recipe.names | length) < $MIN_VOLUMES {
        $found = ($found | append $"the `($RECIPE)` recipe in ($JUSTFILE) parsed only ($recipe.names | length) volume names and the floor is ($MIN_VOLUMES), so the parser stopped matching and the comparison would have been vacuous")
    }

    let declared = ($files | each {|f| $f.names} | flatten | uniq)
    let where_declared = {|name|
        $files | where {|f| $name in $f.names} | get label | str join " and "
    }

    for name in $recipe.names {
        if $name not-in $declared {
            $found = ($found | append $"`($RECIPE)` removes `($name)`, which no compose file declares: drop it from the `vols` list in ($JUSTFILE), or declare the volume")
        }
    }
    for name in $declared {
        if $name not-in $recipe.names {
            $found = ($found | append $"(do $where_declared $name) declares `($name)`, which `($RECIPE)` never removes: add it to the `vols` list in ($JUSTFILE), or drop it from the compose file")
        }
    }

    $found
}

def sample-compose-dev []: nothing -> string {
    [
        '---'
        'name: rusty-links-${USER}'
        'services:'
        '  app:'
        '    volumes:'
        '      - ./src:/app/src'
        '      - cargo_home:/usr/local/cargo'
        'volumes:'
        '  cargo_home:'
        '    # a comment between the key and its name'
        '    name: rusty-links-cargo-${USER}'
        '  cargo_target:'
        '    name: rusty-links-target-${USER}'
        '  dx_out:'
        '    name: rusty-links-dx-${USER}'
        '  postgres_data:'
        '    name: rusty-links-postgres-${USER}'
        'networks:'
        '  network-traefik-public:'
        '    external: true'
    ] | str join "\n"
}

def sample-compose []: nothing -> string {
    [
        '---'
        'services:'
        '  app:'
        '    volumes:'
        '      - cargo_home:/usr/local/cargo'
        'volumes:'
        '  cargo_home:'
        '    name: rusty-links-cargo-${USER}'
        '  cargo_target:'
        '    name: rusty-links-target-${USER}'
        '  postgres_data:'
        '    name: rusty-links-postgres-${USER}'
    ] | str join "\n"
}

def sample-justfile []: nothing -> string {
    [
        '# Remove the dev stack and every volume it owns'
        "[group: 'cleanup']"
        'dev-clean:'
        '    #!/usr/bin/env nu'
        '    docker compose -f compose.dev.yml down --remove-orphans'
        '    let suffix = $env.USER'
        '    let vols = ['
        '        $"rusty-links-cargo-($suffix)"'
        '        $"rusty-links-dx-($suffix)"'
        '        $"rusty-links-target-($suffix)"'
        '        $"rusty-links-postgres-($suffix)"'
        '    ]'
        '    for vol in $vols { docker volume rm $vol }'
        ''
        '# The next recipe, so the body has a column-0 line to stop at'
        "[group: 'cleanup']"
        'dev-clean-all: dev-clean'
        '    #!/usr/bin/env nu'
        '    docker buildx prune --force'
    ] | str join "\n"
}

def expect-rejected [label: string, found: list<string>] {
    if ($found | is-empty) {
        print --stderr $"[check-dev-clean-volumes] SELF-TEST FAILED: ($label) was accepted."
        exit 1
    }
}

# Reject, and reject by the rule the fixture is aimed at. A fixture that trips some
# other comparison first leaves its own invariant unreached while the self-test still
# passes, which is how a guard grows a rule nothing ever exercises.
def expect-rejected-by [label: string, needle: string, found: list<string>] {
    expect-rejected $label $found
    if ($found | where {|f| $f | str contains $needle} | is-empty) {
        print --stderr $"[check-dev-clean-volumes] SELF-TEST FAILED: ($label) was rejected, but by no rule whose message contains `($needle)`, so the invariant the fixture aims at was never reached. Got: ($found | str join '; ')"
        exit 1
    }
}

# Prove the comparison still reads both shapes and still rejects each drift.
# Without it a drifted parser reports "no drift" for every run, which is the same
# blindness one level up that this guard exists to remove.
def run-self-test [] {
    let dev = (compose-volumes (sample-compose-dev) "compose.dev.yml")
    let prod = (compose-volumes (sample-compose) "compose.yml")
    let recipe = (recipe-volumes (sample-justfile))
    let files = [$dev $prod]

    if ($dev.problems | is-not-empty) or ($prod.problems | is-not-empty) or ($recipe.problems | is-not-empty) {
        print --stderr $"[check-dev-clean-volumes] SELF-TEST FAILED: the parser rejected a well-formed set: (($dev.problems | append $prod.problems | append $recipe.problems) | str join '; ')"
        exit 1
    }
    if ($dev.names | length) != 4 or ($prod.names | length) != 3 or ($recipe.names | length) != 4 {
        print --stderr $"[check-dev-clean-volumes] SELF-TEST FAILED: the parser no longer reads the real shape; it found ($dev.names | length), ($prod.names | length) and ($recipe.names | length) names, expected 4, 3 and 4. A service-level `volumes:` list is probably being counted."
        exit 1
    }
    if "rusty-links-cargo-<user>" not-in $dev.names or "rusty-links-cargo-<user>" not-in $recipe.names {
        print --stderr $"[check-dev-clean-volumes] SELF-TEST FAILED: `${USER}` and `\($suffix)` no longer normalise to one placeholder, so the two lists can never compare equal. Compose: ($dev.names | str join ', '). Recipe: ($recipe.names | str join ', ')."
        exit 1
    }

    let clean = (violations $files $recipe)
    if ($clean | is-not-empty) {
        print --stderr $"[check-dev-clean-volumes] SELF-TEST FAILED: a matching pair was rejected: ($clean | str join '; ')"
        exit 1
    }

    # A recipe naming a volume no compose file declares: the `app-data` / `db-data`
    # shape the former `clean` recipe carried.
    let phantom = ($recipe | update names ($recipe.names | append "rusty-links-ghost-<user>"))
    expect-rejected-by "a recipe naming an undeclared volume" "which no compose file declares" (violations $files $phantom)

    # A compose file declaring a volume the recipe never removes: the shape a new
    # volume added to compose and not to the recipe produces.
    let added = (compose-volumes (sample-compose-dev | str replace 'networks:' "  redis_data:\n    name: rusty-links-redis-${USER}\nnetworks:") "compose.dev.yml")
    if ($added.names | length) != 5 {
        print --stderr $"[check-dev-clean-volumes] SELF-TEST FAILED: the added-volume fixture parsed ($added.names | length) names, expected 5, so it does not exercise the rule it aims at."
        exit 1
    }
    expect-rejected-by "a compose volume the recipe never removes" $"which `($RECIPE)` never removes" (violations [$added $prod] $recipe)
    expect-rejected-by "a compose volume the recipe never removes, named with its file" "compose.dev.yml declares" (violations [$added $prod] $recipe)

    # An unparseable recipe, three ways: no recipe, no list, an empty list.
    expect-rejected-by "a justfile with no dev-clean recipe" $"has no `($RECIPE):` recipe" (violations $files (recipe-volumes "build:\n    cargo build\n"))
    expect-rejected-by "a dev-clean recipe with no vols list" "has no `let vols = [...]` list" (violations $files (recipe-volumes (sample-justfile | str replace 'let vols = [' 'let names = [')))
    expect-rejected-by "a dev-clean recipe with an empty vols list" 'holding no `$"..."` volume names' (violations $files (recipe-volumes (sample-justfile | str replace --regex '(?ms)let vols = \[.*?\]' 'let vols = []')))

    # An unreadable or reshaped compose file, three ways.
    expect-rejected-by "a compose file that is not YAML" "is not readable as YAML" (violations [(compose-volumes "\tnot: [valid: yaml" "compose.dev.yml") $prod] $recipe)
    expect-rejected-by "a compose file with no volumes block" "has no top-level `volumes:` block" (violations [(compose-volumes "services:\n  app:\n    image: x\n" "compose.dev.yml") $prod] $recipe)
    expect-rejected-by "a compose volume entry with no name" "with no `name:` field" (violations [(compose-volumes "volumes:\n  cargo_home:\n  cargo_target:\n    name: rusty-links-target-${USER}\n" "compose.dev.yml") $prod] $recipe)

    # A list too short to have been parsed fails on the floor rather than reading
    # as "these two short lists happen to agree".
    let short = (compose-volumes "volumes:\n  cargo_home:\n    name: rusty-links-cargo-${USER}\n" "compose.dev.yml")
    expect-rejected-by "a compose file parsed down to one volume" "and the floor is" (violations [$short $prod] $recipe)
    let short_recipe = {names: ["rusty-links-cargo-<user>"], problems: []}
    expect-rejected-by "a recipe parsed down to one volume" "and the floor is" (violations $files $short_recipe)

    # The vacuous case the floor exists for: both sides empty compare equal.
    expect-rejected-by "two empty lists, which compare equal" "and the floor is" (violations [{label: "compose.dev.yml", names: [], problems: []}] {names: [], problems: []})

    print $"[check-dev-clean-volumes] SELF-TEST OK: a recipe naming an undeclared volume, a compose volume the recipe never removes, a missing recipe, a missing or empty `vols` list, an unreadable compose file, a missing `volumes:` block, a volume with no `name:`, a short parse and two empty lists are all rejected, each by the rule it is aimed at; a matching pair written in each file's own `${USER}` / `\($suffix)` spelling is not."
}

export def main [
    --self-test # check the guard still detects drift, then exit
] {
    if $self_test {
        run-self-test
        return
    }

    for f in ([$JUSTFILE] | append $COMPOSE_FILES) {
        if not ($f | path exists) {
            print --stderr $"[check-dev-clean-volumes] FAILED: ($f) does not exist. Run this from the repository root."
            exit 1
        }
    }

    let files = ($COMPOSE_FILES | each {|f| compose-volumes (open --raw $f) $f})
    let recipe = (recipe-volumes (open --raw $JUSTFILE))
    let found = (violations $files $recipe)

    if ($found | is-not-empty) {
        print --stderr "[check-dev-clean-volumes] FAILED:"
        for problem in $found {
            print --stderr $"  - ($problem)"
        }
        print --stderr ""
        print --stderr $"The `volumes:` blocks of ($COMPOSE_FILES | str join ' and ') and the `vols` list in the `($RECIPE)` recipe of ($JUSTFILE) are two copies of one list. Every declared volume must be removed by `($RECIPE)` and every volume it names must be declared, so `just ($RECIPE)` leaves nothing behind and removes nothing that does not exist. Fix whichever copy is wrong, or change them together."
        exit 1
    }

    let declared = ($files | each {|f| $f.names} | flatten | uniq)
    print ($declared | wrap volume)
    print $"[check-dev-clean-volumes] OK: ($declared | length) named volumes are declared across ($COMPOSE_FILES | str join ' and ') and all of them are removed by `($RECIPE)`."
}
