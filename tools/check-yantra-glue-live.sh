#!/bin/sh
# The stand-in in `check-yantra-glue.mjs` is answerable to the real module — `F-004`.
#
# `check-yantra-glue.sh` is the every-cycle check and drives a STAND-IN, because
# what it pins (which exports the glue calls, with what lengths, and what it
# leaves behind between two runs) is invisible from a real module's return value.
# A stand-in can be wrong about the real one. This is the arm that asks.
#
# ## It will NOT build the module, and that is the point
#
# `tools/build-sassembly-web.sh` and `check-sassembly-identity.sh` already build
# `yantra-wasm` for wasm32, and a second builder would add a two-minute step to
# every gate for a cross-check. So this uses a module that is ALREADY THERE —
# which is why it lives in the deep suite rather than in `gate.sh`:
# `deep-gate.sh` globs `tools/check-*.sh` and skips what `gate.sh` already runs.
#
# FOUR STATES AND IT SAYS WHICH, and the distinction that matters is 78 vs 77.
# `deep-gate.sh:214` exports `GATE_STRICT=1`, under which 77 — THIS MACHINE
# cannot run the check — is a RED. An absent wasm module is not that: node and
# cargo are both here and the check ran; it had NO SUBJECT, which is `VACUOUS:`
# and 78 (`deep-gate.sh:280-288`, `gate.sh:307-313`), counted as skipped and
# never as a failure. Getting this backwards would red the deep gate on every
# tree that has not built the page — a red that fires on correct behaviour, and
# the margin at `deep-gate.sh:203` records that this exact confusion is why
# `GATE_STRICT` could not be armed for four months. 77 is kept for a genuinely
# missing node or cargo.
#
# Usage: tools/check-yantra-glue-live.sh
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
    echo "CANNOT RUN: cargo absent — the program under test has to be assembled"
    echo "            Exiting 77 rather than 0: a check that cannot run has not passed."
    exit 77
}

wasm=crates/yantra-wasm/target/wasm32-unknown-unknown/release/yantra_wasm.wasm
# ॥ AND WHO IS SUPPOSED TO HAVE BUILT IT ॥ The subject comes from a DIFFERENT
# script: `build-sassembly-web.sh` puts the module at the path above, and both
# `check-sassembly-page.sh` and `check-sassembly-identity.sh` call it. Either
# exits 77 on a host with no `wasm32-unknown-unknown`. So on such a host this
# check is VACUOUS through no fault of its own, and 78 is counted as skipped and
# never as a failure: the coverage goes away SILENTLY. Naming the producer here
# is what makes a vacuous line say WHOSE 77 caused it rather than read as an
# instruction to the person watching.
#
# WHICH of the two produces it CHANGED, and the old margin here named one line
# number for it (`gate.sh:614`) that no longer holds. Under `gate.sh` the page
# check now runs FIRST and hands its page to the identity check, so the module is
# the PAGE check's output and the identity check usually builds nothing at all —
# see `tools/lib/page-handoff.sh`. The ordering that reaches this check is still
# not luck: `s` sorts before `y`, so `deep-gate.sh`'s `tools/check-*.sh` glob
# reaches both sassembly checks first, and in a full cycle gate.sh has already
# run them.
[ -f "$wasm" ] || {
    echo "VACUOUS: no wasm module on this tree, so there is no subject to cross-check \
against. The producer is tools/check-sassembly-page.sh or -identity.sh via build-sassembly-web.sh; \
if it exited 77 this tree has no wasm32-unknown-unknown target and THAT is the cause. \
78 and not 77: the toolchain THIS check needs is present and nothing about this machine \
is broken, so it must not red a GATE_STRICT run."
    echo "         Exiting 78 rather than 0 — this is NO MEASUREMENT, not a green."
    exit 78
}

# The program is `spec/namaste.sas`: a boot proof that PRINTS, so "the page still
# works" is a readable string and not just an exit code. It declares no input
# globals, which is what makes the second refusal arm possible.
[ -f spec/namaste.sas ] || { echo "  FAIL  spec/namaste.sas is missing."; exit 1; }

# ॥ AND A SECOND PROGRAM, FOR THE HIGH WATER ALONE ॥ `namaste` writes to the UART
# and the finisher and nothing else, both MMIO, so its high water is ZERO — which is
# the reading `web/yantra.mjs`'s `#water()` keeps apart from `null`, and also the
# reading an export stuck at a constant 0 would produce. One subject cannot tell
# those apart, so the check needs a program whose water is NOT zero.
#
# `spec/freelist.sas` is that program and is used UNCHANGED: it hands out eight
# 64-octet blocks threaded through the memory it allocates, so it genuinely stores
# to RAM, it needs no device and no FDT, and it halts by shutdown rather than running
# out of budget. Measured on this module: water 4,552 against 1,748 retired — two
# figures that are not each other, which is also what makes the "it is not the step
# count" arm mean something. No NEW .sas is added: the water arms do not need a
# program written for them, and spec/ is a fixpoint input.
storer=spec/freelist.sas
[ -f "$storer" ] || { echo "  FAIL  $storer is missing."; exit 1; }

out=$(mktemp -d) || exit 1
trap 'rm -rf "$out"' EXIT

# A failure HERE is the tree's, not the machine's — cargo was found above.
if ! cargo run -q -p sadhana -- spec/namaste.sas "$out/namaste.elf" >"$out/sadhana.log" 2>&1; then
    echo "  FAIL  sadhana could not assemble spec/namaste.sas."
    sed 's/^/        /' "$out/sadhana.log"
    exit 1
fi
[ -s "$out/namaste.elf" ] || {
    echo "  FAIL  sadhana reported success and wrote no ELF."; exit 1; }

if ! cargo run -q -p sadhana -- "$storer" "$out/storer.elf" >"$out/storer.log" 2>&1; then
    echo "  FAIL  sadhana could not assemble $storer."
    sed 's/^/        /' "$out/storer.log"
    exit 1
fi
[ -s "$out/storer.elf" ] || {
    echo "  FAIL  sadhana reported success and wrote no ELF for $storer."; exit 1; }

echo "the glue against the module in $wasm:"
node tools/check-yantra-glue-live.mjs "$wasm" "$out/namaste.elf" "$out/storer.elf" || exit 1
