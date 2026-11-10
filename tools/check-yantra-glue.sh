#!/bin/sh
# The browser glue agrees with `yantra-wasm` about the input channel — task `F-004`.
#
# Wrapper around `check-yantra-glue.mjs`, which is where the work is: the glue is a
# JS module and the only honest way to ask what it does is to let node run it.
#
# ## Why the glue needs a check at all
#
# `web/` is compiled by nothing. `cargo` never reads it, `clippy` never reads it,
# and `tools/build-sassembly-web.sh` INLINES it into a page it writes outside the
# repository — so a glue that calls an export the module does not have, or does not
# call one it should, produces a page that is only wrong when a human opens it.
# That is how `yantra_input_alloc` sat unreachable from `1119c792` to now.
#
# ## Three arms, and the last two are about the INLINED form
#
# The page does not `import` the glue: the module must open from `file://` with no
# server, so the build strips `^export ` and pastes the body into the template's
# own `<script type="module">`. That is a DIFFERENT program from the one node
# imports — a top-level name the glue declares can collide with one the template
# declares, and the result is a `SyntaxError` at page load with every test here
# still green. So arm 2 builds the stripped form and parses it, and refuses a
# top-level `const` the template already binds.
#
# AND ARM 2 ONLY EVER LOOKED AT THE TEMPLATE. The page will paste BOTH glues into
# that one scope — `web/sadhana.mjs` is the other half of F-004 — and both of them
# declared a top-level `load`, which no reader here could see. Arm 3 is
# `tools/check-glue-collision.sh`: it asks the parser about the pair instead of
# matching names, which is also the only way the ALLOWED case stays allowed
# (a glue name the template binds only in an inner scope merely shadows it).
# Arm 2's `^[[:space:]]*` pattern gets that case wrong; arm 3 does not, and the
# two are kept side by side so the disagreement is on the record rather than lost
# in a rewrite.
#
# Usage: tools/check-yantra-glue.sh
set -u

cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)" || exit 1

# `CANNOT RUN:` must be the FIRST line and spelled EXACTLY that — gate.sh's run()
# matches `^(SKIPPED|CANNOT RUN):` and surfaces only the first line.
command -v node >/dev/null || {
    echo "CANNOT RUN: node absent — the glue is a JS module and cannot be run"
    echo "            Exiting 77 rather than 0: a check that cannot run has not passed."
    exit 77
}

for f in web/yantra.mjs web/sassembly.template.html tools/check-yantra-glue.mjs \
         tools/check-glue-collision.sh; do
    [ -f "$f" ] || { echo "  FAIL  $f is missing — the check has nothing to read."; exit 1; }
done

node tools/check-yantra-glue.mjs || exit 1

# ———— arm 2: the form the PAGE gets ————————————————————————————————————
# The same `sed` the build runs (`build-sassembly-web.sh:246`). Spelled here rather
# than shared, because sharing it would make this check pass whatever the build
# does rather than whatever the build is SUPPOSED to do.
tmp=$(mktemp -d) || exit 1
trap 'rm -rf "$tmp"' EXIT
sed 's/^export //' web/yantra.mjs > "$tmp/inlined.mjs"
if ! node --check "$tmp/inlined.mjs" 2>"$tmp/err"; then
    echo "  FAIL  the export-stripped glue does not parse — the generated page would not load."
    sed 's/^/        /' "$tmp/err"
    exit 1
fi

# Every name the glue binds at module scope — `const`, `let`, `class` and
# `function` alike, because the template's `const y = await load(...)` would be
# just as broken by a colliding `load` as by a colliding `enc`.
# `sed -E`, and NOT `\(const\|let\)`: BRE alternation is a GNU extension and BSD sed
# matched nothing, so the first version of this reported "0 top-level bindings" on a
# glue that has three and walked an EMPTY collision loop. The floor below is what
# makes that a failure instead of a clean line.
ids=$(sed -E -n 's/^(const|let|var|class|async function|function) ([A-Za-z_$][A-Za-z0-9_$]*).*/\2/p' \
    "$tmp/inlined.mjs")
count=$(printf '%s\n' "$ids" | grep -c . )
[ "$count" -ge 4 ] || {
    echo "  FAIL  found $count top-level binding(s) in the inlined glue; the file has at least 4."
    echo "        A reader that sees none cannot see a collision either, so this is its own red."
    exit 1; }
printf '%s\n' "$ids" |
while IFS= read -r id; do
        [ -n "$id" ] || continue
    if sed -n '/<script/,$p' web/sassembly.template.html |
       grep -Eq "^[[:space:]]*(const|let|var|class|function)[[:space:]]+$id([^A-Za-z0-9_$]|$)"; then
        echo "  FAIL  the glue and the page both declare a top-level \`$id\`."
        echo "        Inlined into one module scope that is a SyntaxError at load."
        exit 1
    fi
done || exit 1

echo "  ok    the inlined glue parses; $count top-level binding(s), none colliding with the page"

# ———— arm 3: THE PAIR, and neither wrapper could see it ————————————————————
# Both glues declare top-level names and the page pastes BOTH bodies into ONE
# module scope. Everything above compares a glue against the TEMPLATE and never
# against the other glue, so two top-level `load`s read as green here and as a
# `SyntaxError` in the browser. Shared rather than spelled twice because the
# subject is the PAIR and a pair has one reader; invoked from both wrappers so
# either one refuses on it. Exit 77 (node absent) is passed through unchanged.
sh tools/check-glue-collision.sh
rc=$?
[ "$rc" -eq 0 ] || [ "$rc" -eq 77 ] || exit 1
