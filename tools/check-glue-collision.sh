#!/bin/sh
# THE TWO BROWSER GLUES DO NOT COLLIDE WITH EACH OTHER — arm 3 of `F-004`.
#
# Invoked by BOTH `tools/check-yantra-glue.sh` and `tools/check-sadhana-glue.sh`,
# so either one refuses on it. ONE implementation and not two, which is the
# opposite of the rule those files follow for the build's `sed` — and deliberately:
# that `sed` is mirrored in each because the subject is each glue's own inlined
# form, whereas the subject HERE is the PAIR, and a pair has one reader.
#
# ## What was missing, measured 2026-10-05
#
# Each glue check compared its glue against `web/sassembly.template.html` and
# NEVER against the other glue. `web/yantra.mjs` and `web/sadhana.mjs` both
# declared a top-level `load`. The page does not `import` either module — it must
# open from `file://` with no server, so `build-sassembly-web.sh:246` strips
# `^export ` and pastes the body into the template's own `<script type="module">`.
# So the day the build splices BOTH, which is exactly what F-004's in-browser
# assembler needs, two top-level `load`s land in one module scope and the page
# dies with a `SyntaxError` before a line of it runs — with both checks green.
# `5dbac251` renamed the assembler's loader `loadSadhana`; this file is the reader
# that keeps it renamed.
#
# ## IT ASKS THE PARSER, IT DOES NOT MATCH NAMES
#
# The obvious arm is a name-set intersection, and both existing checks do that
# against the template with `sed -E -n 's/^[[:space:]]*(const|let|…)/'`. THAT
# PATTERN IS WRONG AND THIS FILE DOES NOT COPY IT: `^[[:space:]]*` matches an
# INDENTED declaration, so it reads the template's inner-scope names as page-level
# bindings — `app b elf have r refused slab t1`, eight of them, all local to
# `b.onclick` or `reserve`. A glue that declared a top-level `elf` would be refused
# for a collision that is not one; shadowing an inner scope is legal JavaScript and
# the page would load. So this arm BUILDS the module the browser would actually get
# and runs `node --check` over it. The parser's answer needs no heuristic, and the
# allowed case is allowed for free rather than by a second rule.
#
# ## The four states, and the last two are the point
#
#   SPLICED        both glues + the template body parse as one module      -> ok
#   PAIR           a name the OTHER glue declares                          -> must be RED
#   PAGE           a name the template declares at module level            -> must be RED
#   INNER          a name the template declares only INSIDE a function     -> must STAY GREEN
#
# PAIR and PAGE are injected here and must go red; a reader that passes them is
# blind and says so. INNER is the case that must still be ALLOWED, and it is the
# one the name-matching arm gets wrong.
#
# All three are read THROUGH the real pair, so if SPLICED is red they are not
# interpretable — they would each report the same first SyntaxError, which is one
# defect counted four times. That is a FOURTH state and it is reported as itself.
# Proved by content on 2026-10-05: `sed -i 's/loadSadhana/load/'` on the glue and
# this file exits 1 naming `Identifier 'load' has already been declared`.
#
# Usage: tools/check-glue-collision.sh
set -u

cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)" || exit 1

# `CANNOT RUN:` must be the FIRST line and spelled EXACTLY that — gate.sh's run()
# matches `^(SKIPPED|CANNOT RUN):` and surfaces only the first line.
command -v node >/dev/null || {
    echo "CANNOT RUN: node absent — the glues are JS modules and only a parser can answer this"
    echo "            Exiting 77 rather than 0: a check that cannot run has not passed."
    exit 77
}

for f in web/yantra.mjs web/sadhana.mjs web/sassembly.template.html; do
    [ -f "$f" ] || { echo "  FAIL  $f is missing — the pair cannot be assembled."; exit 1; }
done

tmp=$(mktemp -d) || exit 1
trap 'rm -rf "$tmp"' EXIT

# The template's module body, verbatim. There is exactly one `<script type="module">`
# in the file; `1d;$d` drops the two tags and keeps what the browser parses.
sed -n '/<script type="module">/,/<\/script>/p' web/sassembly.template.html |
    sed '1d;$d' > "$tmp/body"
body=$(grep -a -c . "$tmp/body")
# A FLOOR, for the same reason `check-yantra-glue.sh` has one: a reader that got an
# empty body would splice two glues into nothing and report PARSES. That is the
# failure mode this whole file exists to refuse, so it cannot be its own blind spot.
[ "$body" -ge 40 ] || {
    echo "  FAIL  read only $body line(s) of the template's module body; the file has at least 40."
    echo "        An empty body would parse whatever was spliced into it, so this is its own red."
    exit 1; }
# BOTH PLACEHOLDERS, and the second one is why this file had to change today. Until
# `build-sassembly-web.sh` learned to splice the assembler, the template declared ONE
# glue placeholder and this reader pasted BOTH glues into it — which was the right
# stand-in for a page that did not exist yet. Now the page has a placeholder each, so
# the stand-in pastes one glue into each, and a template that names only one of them is
# a template this reader cannot stand in for.
#
# MEASURED, and it is why this is a floor and not an `if`: with the template carrying
# `@@SADHANA_GLUE@@` and this awk substituting only `@@GLUE@@`, the line survived into
# the module verbatim and node answered `SyntaxError: Invalid or unexpected token`. The
# arm went red over a placeholder rather than over a collision — a red for the wrong
# reason, which this file refuses by name three arms below.
for ph in @@GLUE@@ @@SADHANA_GLUE@@; do
    grep -a -q "$ph" "$tmp/body" || {
        echo "  FAIL  the template's module body names no $ph placeholder — the page this"
        echo "        reader stands in for splices a glue there, so there is nothing to splice."
        exit 1; }
done

# Build the module the browser would get with BOTH glues spliced, and ask node.
# `$1`, if given, is one extra top-level line appended to the assembler glue: that
# is how the controls below inject a collision without touching the tree.
# Prints `PARSES`, or the parser's own one-line verdict.
page() {
    sed 's/^export //' web/yantra.mjs  > "$tmp/g1"
    sed 's/^export //' web/sadhana.mjs > "$tmp/g2"
    if [ -n "$1" ]; then printf '%s\n' "$1" >> "$tmp/g2"; fi
    # `@@WASM@@` and `@@SADHANA_WASM@@` both sit inside string literals and parse as
    # ones. `{@@PROGRAMS@@}` does not, so it is emptied to `{}`.
    #
    # ONE GLUE PER PLACEHOLDER, mirroring what the build now does — and the assembler's
    # arm is written FIRST on purpose. `@@GLUE@@` is not in fact a substring of
    # `@@SADHANA_GLUE@@` (the `GLUE@@` there is preceded by `_`, not by `@@`), so the
    # order does not decide it today; it is written this way because the build's own
    # `case` DOES depend on that order for the WASM pair, and a reader whose arms are
    # ordered by accident is one rename away from splicing the interpreter twice.
    awk -v a="$tmp/g1" -v b="$tmp/g2" '
        /@@SADHANA_GLUE@@/ {
            while ((getline l < b) > 0) print l; close(b)
            next }
        /@@GLUE@@/ {
            while ((getline l < a) > 0) print l; close(a)
            next }
        { gsub(/@@PROGRAMS@@/, ""); print }' "$tmp/body" > "$tmp/page.mjs"
    if node --check "$tmp/page.mjs" 2>"$tmp/err"; then
        echo PARSES
    else
        sed -n 's/^[[:space:]]*\(SyntaxError.*\)$/\1/p' "$tmp/err" | sed -n 1p
    fi
}

fails=0
# SPLICED — the real pair, as the build would hand it to the browser.
real=$(page '')
if [ "$real" != PARSES ]; then
    echo "  FAIL  the two glues spliced into one page do not parse — the page would not load."
    echo "        $real"
    echo "        Both are pasted into ONE module scope; a name may be declared once."
    fails=1
else
    glue1=$(sed -E -n 's/^(export )?(const|let|var|class|async function|function) ([A-Za-z_$][A-Za-z0-9_$]*).*/\3/p' web/yantra.mjs | sort -u | grep -c .)
    glue2=$(sed -E -n 's/^(export )?(const|let|var|class|async function|function) ([A-Za-z_$][A-Za-z0-9_$]*).*/\3/p' web/sadhana.mjs | sort -u | grep -c .)
    echo "  ok    arm 3: both glues spliced into the page parse as one module"
    echo "        ($glue1 + $glue2 top-level bindings over $body lines of template body)"
fi

# PAIR, PAGE and INNER are all read THROUGH the real pair, so they say nothing
# while the real pair is already broken: every injection would report the SAME
# first SyntaxError and three reds would be one defect counted four times.
# Stated rather than run, because "control blind" and "control unreadable" are
# different facts and an instrument with two states where the truth has three
# hides its own breakage.
if [ "$real" != PARSES ]; then
    echo "  ----  the three controls are NOT INTERPRETABLE: every injection is read through"
    echo "        the same module, which already does not parse, so each would report the"
    echo "        collision above and nothing about itself. Fix the real pair, then they run."
    exit 1
fi

# PAIR and PAGE — the two injections that MUST be refused.
# `Yantra` is a class the OTHER glue declares; `y` is bound by the template at
# module level (`const y = await load(...)`). Neither is reachable by this file
# unless the construction above is real.
for probe in 'PAIR:const Yantra = 0;:Yantra' 'PAGE:const y = 0;:y'; do
    which=${probe%%:*}; rest=${probe#*:}; line=${rest%:*}; name=${rest##*:}
    saw=$(page "$line")
    if [ "$saw" = PARSES ]; then
        echo "  FAIL  READER BLIND: injecting \`$line\` into the assembler glue still parses."
        echo "        A duplicate top-level \`$name\` is a SyntaxError in a browser, so this"
        echo "        arm cannot see the defect it exists for and its green means nothing."
        fails=1
    else
        case $saw in
            *"'$name'"*) echo "  control: $which — a top-level \`$name\` is refused ($saw)" ;;
            *) echo "  FAIL  READER CONFUSED: injecting \`$line\` failed, but not over \`$name\`."
               echo "        $saw"
               echo "        A red for the wrong reason is not evidence about collisions."
               fails=1 ;;
        esac
    fi
done

# INNER — THE CASE THAT MUST STILL BE ALLOWED. `elf` is declared in the template
# only inside `b.onclick` (`const elf = un(p.elf);`), so a top-level `elf` in a
# glue SHADOWS it and the page loads. The name-matching arm in both wrappers reads
# that inner `const` as a page binding and would refuse this; the parser does not.
inner=$(page 'const elf = 0;')
if [ "$inner" = PARSES ]; then
    echo "  ok    arm 3: INNER — a glue name the template binds only inside a function is"
    echo "        NOT a collision (\`elf\`, declared in \`b.onclick\`); shadowing is legal and"
    echo "        the page loads. This is the case the \`^[[:space:]]*\` name match gets wrong."
else
    echo "  FAIL  a top-level \`elf\` was refused, but the template declares \`elf\` only"
    echo "        inside \`b.onclick\`. Shadowing an inner scope is legal JavaScript."
    echo "        $inner"
    fails=1
fi

[ "$fails" -eq 0 ] || exit 1
