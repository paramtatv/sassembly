#!/bin/sh
# The browser glue agrees with `sadhana-wasm` about the ASSEMBLER — task `F-004`.
#
# The twin of `tools/check-yantra-glue.sh`, and it exists because that file's own
# reason applied to the OTHER half of the pair and nobody had followed it there.
#
# ## What was uncovered, measured 2026-10-05 and not estimated
#
# `web/sadhana.mjs` — 80 lines, the only caller anywhere of `sadhana-wasm`'s six
# exports — was read by NOTHING. `grep -rn 'sadhana\.mjs'` over the tree hits
# `BACKLOG.tsv` prose and NOT ONE line of code, tool or template.
# `build-sassembly-web.sh:246` splices exactly one module into the page
# (`*@@GLUE@@*) ... web/yantra.mjs`), `web/sassembly.template.html` declares three
# placeholders and no fourth, and `cargo`/`clippy` do not read JS. So the assembler
# glue was in the same state `yantra_input_alloc` was in from `1119c792`: present,
# plausible, and verified by no instrument.
#
# AND IT CARRIES A CROSS-ARTEFACT DIVERGENCE NOTHING COULD SEE. `web/sadhana.mjs:52`
# HARD-CODES the application load address; `build-sassembly-web.sh:99` READS the same
# number out of `spec/application-load.tsv` with awk. One reader follows the spec and
# one does not, so a spec edit splits the page's two halves apart silently. Arm 8 is
# that assertion, and it is the only arm here that reads outside `web/`.
#
# ## AND THE PROOF BASE WAS THE SAME DIVERGENCE, UNGUARDED — measured 2026-10-05
#
# Arm 8 read `APP_LOAD` and NOTHING read `Sadhana.BARE_METAL`, which is the base
# TWO of the page's three buttons are assembled at. It was `0x80200000` — the
# OpenSBI payload address — while `build-sassembly-web.sh` passes no `--स्थान` for
# a proof and so gets `kosha::LOAD_ADDRESS`, `0x8000_0000`. Measured over the real
# `sadhana-wasm`: `namaste.sas` at the glue's base is sha256 `a28bccab…` and the
# page's own ELF is `40324925…`, both 904 octets; native `--स्थान ०षोड्८०२०००००`
# reproduces `a28bccab…`, which is what proves the base was the WHOLE divergence.
# So the day the build splices this glue the page would have assembled a different
# image from the one it ships. Arm 11 is that assertion, against `kosha.rs`.
#
# ## Usage: tools/check-sadhana-glue.sh
set -u

cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)" || exit 1

# `CANNOT RUN:` must be the FIRST line and spelled EXACTLY that — gate.sh's run()
# matches `^(SKIPPED|CANNOT RUN):` and surfaces only the first line. Exit 77, which
# `run()` renders as SKIPPED and `GATE_STRICT=1` turns into a failure.
command -v node >/dev/null || {
    echo "CANNOT RUN: node absent — the glue is a JS module and cannot be run"
    echo "            Exiting 77 rather than 0: a check that cannot run has not passed."
    exit 77
}

for f in web/sadhana.mjs spec/application-load.tsv tools/check-sadhana-glue.mjs \
         tools/check-glue-collision.sh; do
    [ -f "$f" ] || { echo "  FAIL  $f is missing — the check has nothing to read."; exit 1; }
done

node tools/check-sadhana-glue.mjs || exit 1

# ———— arm 2: THE THIRD STATE, which the twin does not have ————————————————
#
# `check-yantra-glue.sh` has a second arm over the INLINED form, because the page
# does not `import` the glue: the build strips `^export ` and pastes the body into
# the template's own `<script type="module">`, where a top-level name can collide
# with one the template already binds.
#
# THAT ARM CANNOT SIMPLY BE COPIED HERE, because `web/sadhana.mjs` IS NOT SPLICED
# INTO THE PAGE TODAY — and a check that runs it anyway would assert a property of
# a program that does not exist, which reads as coverage and is not. A check that
# runs it NEVER is just as wrong the day someone adds the placeholder.
#
# So this arm has THREE states and reports which one it is in, rather than two that
# hide the transition: NOT SPLICED (stated, not passed over), SPLICED AND SOUND, or
# SPLICED AND COLLIDING. The day `build-sassembly-web.sh` learns to inline the
# assembler, this arm starts asserting the collision property by itself.
if grep -q 'web/sadhana\.mjs' tools/build-sassembly-web.sh; then
    tmp=$(mktemp -d) || exit 1
    trap 'rm -rf "$tmp"' EXIT
    # The same `sed` the build runs. Spelled here rather than shared, because
    # sharing it would make this check pass whatever the build does rather than
    # whatever the build is SUPPOSED to do.
    sed 's/^export //' web/sadhana.mjs > "$tmp/inlined.mjs"
    if ! node --check "$tmp/inlined.mjs" 2>"$tmp/err"; then
        echo "  FAIL  the export-stripped assembler glue does not parse — the page would not load."
        sed 's/^/        /' "$tmp/err"
        exit 1
    fi
    # `sed -E`, and NOT `\(const\|let\)`: BRE alternation is a GNU extension and
    # BSD sed does not have it — it would match nothing here and find no collision.
    names=$(sed -E -n 's/^(export )?(const|let|var|class|async function|function) ([A-Za-z_$][A-Za-z0-9_$]*).*/\3/p' web/sadhana.mjs | sort -u)
    page=$(sed -E -n 's/^[[:space:]]*(const|let|var|class|async function|function) ([A-Za-z_$][A-Za-z0-9_$]*).*/\2/p' web/sassembly.template.html | sort -u)
    clash=$(printf '%s\n' "$names" | while read -r n; do
        [ -n "$n" ] || continue
        printf '%s\n' "$page" | grep -qx "$n" && echo "$n"
    done)
    if [ -n "$clash" ]; then
        echo "  FAIL  the inlined assembler glue rebinds a name the page already declares:"
        printf '%s\n' "$clash" | sed 's/^/        /'
        exit 1
    fi
    echo "  ok    the inlined assembler glue parses; $(printf '%s\n' "$names" | grep -c .) top-level binding(s), none colliding with the page"
else
    echo "  ok    NOT SPLICED — build-sassembly-web.sh names no placeholder for web/sadhana.mjs,"
    echo "        so the inlined-form arm has no subject. It is NOT passed over silently: the"
    echo "        page ships pre-assembled programs via @@PROGRAMS@@ and does not assemble in"
    echo "        the browser yet. Add the placeholder and this arm starts asserting by itself."
fi

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
