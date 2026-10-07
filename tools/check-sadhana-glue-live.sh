#!/bin/sh
# The browser assembler emits the page's OWN ELF, octet for octet — `F-004`.
#
# The twin of `tools/check-yantra-glue-live.sh`, and the arm that would have
# caught today's divergence by itself. `check-sadhana-glue.sh` drives a STAND-IN,
# because what it pins (which exports the glue calls, with what lengths, what it
# leaves behind between two runs) is invisible from a real module's return value;
# its arm 11 then compares `Sadhana.BARE_METAL` against `kosha::LOAD_ADDRESS` by
# READING BOTH NUMBERS. That is a comparison of two constants and it is not the
# claim. THE CLAIM IS THAT THE BYTES AGREE, and only the real module can answer.
#
# ## What the constant arm cannot see
#
# Measured 2026-10-05: with `BARE_METAL` at `0x80200000` the glue assembled
# `spec/namaste.sas` to sha256 `a28bccab…` while the page ships `40324925…` —
# both 904 octets, so even a LENGTH check would have passed. Arm 11 catches that
# one defect because it has a number to compare against. It would NOT catch a
# divergence with no constant behind it: a glue that truncated the source, dropped
# the high half of the base, or handed back a stale buffer all keep every constant
# correct and change the bytes. This check compares the bytes.
#
# ## It will NOT build the module, and that is the point
#
# `tools/build-sassembly-web.sh` already builds `sadhana-wasm` for wasm32, and a
# second builder would add that step to every gate for a cross-check. So this uses
# a module that is ALREADY THERE, which is why it lives in the deep suite:
# `deep-gate.sh` globs `tools/check-*.sh` and skips what `gate.sh` already runs.
#
# FOUR STATES AND IT SAYS WHICH, and 78 vs 77 is the distinction that matters —
# `deep-gate.sh:214` exports `GATE_STRICT=1`, under which 77 (THIS MACHINE cannot
# run the check) is a RED. An absent wasm module is not that: node and cargo are
# both here and the check ran; it had NO SUBJECT, which is `VACUOUS:` and 78.
# Getting it backwards would red the deep gate on every tree that has not built
# the page. The reasoning is `check-yantra-glue-live.sh`'s and is not re-derived.
#
# Usage: tools/check-sadhana-glue-live.sh
set -u

cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)" || exit 1

# `CANNOT RUN:` must be the FIRST line and spelled EXACTLY that — gate.sh's run()
# and deep-gate.sh both match `^(SKIPPED|CANNOT RUN):`.
command -v node >/dev/null || {
    echo "CANNOT RUN: node absent — the glue is a JS module and cannot be run"
    echo "            Exiting 77 rather than 0: a check that cannot run has not passed."
    exit 77
}
command -v cargo >/dev/null || {
    echo "CANNOT RUN: cargo absent — the reference ELF has to be assembled natively"
    echo "            Exiting 77 rather than 0: a check that cannot run has not passed."
    exit 77
}

wasm=crates/sadhana-wasm/target/wasm32-unknown-unknown/release/sadhana_wasm.wasm
[ -f "$wasm" ] || {
    echo "VACUOUS: no sadhana wasm module on this tree, so there is no subject to \
cross-check against; build one with tools/build-sassembly-web.sh. 78 and not 77: the \
toolchain is present and nothing about this machine is broken, so it must not red a \
GATE_STRICT run."
    echo "         Exiting 78 rather than 0 — this is NO MEASUREMENT, not a green."
    exit 78
}

# ———— THE REFERENCE ELFS, ASSEMBLED THE WAY THE PAGE'S ARE ————————————————
#
# The two kinds, because the base is what diverged and the two kinds get it from
# different places. A PROOF is assembled with no `--स्थान` at all, so it lands on
# `kosha::LOAD_ADDRESS`; an APPLICATION is given the address out of
# `spec/application-load.tsv`. BOTH SPELLINGS ARE THE BUILD'S OWN, copied from
# `build-sassembly-web.sh:99` and its program loop rather than shared with it —
# sharing would make this check pass whatever the build does rather than whatever
# the build is SUPPOSED to do, which is the rule both wrappers already follow for
# the splice `sed`.
APP_LOAD=$(awk '$1 == "0x2000_0000" {print $2}' spec/application-load.tsv)
[ -n "$APP_LOAD" ] || {
    echo "  FAIL  spec/application-load.tsv names no 0x2000_0000 row, so an application"
    echo "        has no reference base and this check has no subject for its second kind."
    exit 1; }

out=$(mktemp -d) || exit 1
trap 'rm -rf "$out"' EXIT

# `spec/atithi.sas` is the application, as it is on the page; the two proofs are
# the page's two. Named rather than globbed for `build-sassembly-web.sh`'s own
# reason: most of `spec/programs.tsv` needs SYSTEM, FENCE or AMO.
for spec in 'namaste:proof' 'bare-metal:proof' 'atithi:app'; do
    p=${spec%%:*}; kind=${spec##*:}
    [ -f "spec/$p.sas" ] || { echo "  FAIL  spec/$p.sas does not exist"; exit 1; }
    case $kind in app) at="--स्थान $APP_LOAD" ;; *) at= ;; esac
    # A failure HERE is the tree's, not the machine's — cargo was found above.
    # shellcheck disable=SC2086
    if ! cargo run -q -p sadhana -- $at "spec/$p.sas" "$out/$p.elf" >"$out/$p.log" 2>&1; then
        echo "  FAIL  sadhana could not assemble spec/$p.sas natively."
        sed 's/^/        /' "$out/$p.log"
        exit 1
    fi
    [ -s "$out/$p.elf" ] || {
        echo "  FAIL  sadhana reported success on spec/$p.sas and wrote no ELF."; exit 1; }
done

echo "the glue against the module in $wasm:"
node tools/check-sadhana-glue-live.mjs "$wasm" "$out" || exit 1
