#!/bin/sh
# The editor's one command, end to end, asserted — task `F-020`.
#
# ## Why this exists
#
# `F-020`'s acceptance is a ROUND TRIP: open a `.sas`, press one command, and
# watch it assemble, link and RUN with output in the editor. That is the row's
# acceptance, which means it is the thing most worth gating and the thing most
# likely to be left as a demonstration instead.
#
# A DEMONSTRATED THING DECAYS AND A GATED THING DOES NOT. If the round trip is
# only ever run by a person pressing the button, it can break and every gate
# still says PASS — and it breaks in the place where a demo is watched.
#
# ## What it runs, and why not QEMU
#
#     cargo run -p sadhana -- spec/namaste.sas <tmp>/namaste.elf
#     cargo run -p yantra --bin yantra-run -- <tmp>/namaste.elf
#
# Both are in-workspace. `yantra` models the UART at `0x1000_0000`
# (crates/yantra/src/lib.rs:173), so the bytes the program stores reach the
# host without an emulator. `tools/sanskriti-demo.sh` remains the QEMU arm and
# `check-sassembly-identity.sh` remains the browser arm; neither can run on a
# machine without `qemu-system-riscv64` or the wasm32 target, and BOTH of those
# already exit 77 here. This one needs nothing but `cargo`, which is why it can
# be the editor's default and why it can be in a gate that runs every cycle.
#
# ## The expectation is not a snapshot of today's output
#
# `नमस्ते संसार` is the exact `EXPECT_SUBSTRING` that
# `tools/check-sassembly-identity.sh` already records for this program, and
# `0x5555` is the finisher's exit-0 code. Recording what the pipeline happens to
# print today would assert only that it still does what it did, which passes for
# a program that has silently become wrong.
#
# It is ALSO checked against the source: `spec/namaste.sas` carries the message
# in an `अष्टकाः` directive, so if the program is edited to say something else
# this REFUSES rather than reporting a pipeline failure. One message for two
# causes sends the reader to the wrong fix (`W-069`).
#
# ## The control, and why a green run without it proves less than it looks
#
# Arm 1 passing says the pipeline printed the expected text. It does NOT say
# this check could have noticed if it had not. So arm 2 runs the SAME pipeline
# and the SAME assertion against `spec/bare-metal.sas`, which writes no
# characters at all — `check-sassembly-identity.sh` REFUSES that program for
# exactly this reason, in as many words. The assertion must FAIL there. If it
# passes, the assertion has stopped discriminating and arm 1's green is
# meaningless.
#
# Usage: tools/check-roundtrip.sh
set -u

cd "$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)" || exit 1

# `CANNOT RUN:` must be the FIRST line and spelled EXACTLY that. gate.sh's run()
# matches `^(SKIPPED|CANNOT RUN):` and prints only the first match; a decorated
# prefix loses the reason, and the whole point of a skip is that it names itself.
command -v cargo >/dev/null || { echo "CANNOT RUN: cargo absent"; exit 77; }

PROGRAM=spec/namaste.sas
CONTROL=spec/bare-metal.sas
EXPECT="नमस्ते संसार"
EXPECT_HALT=21845          # 0x5555 — the finisher's exit-0 code

# The APPLICATION arm. `ADR-0015` splits this tree's programs in two and the split
# is not cosmetic: a boot-proof is a machine, entered bare at the reset vector in
# M-mode with SBI beneath it; an application is a GUEST, entered by a loader in
# U-mode, calling `spec/application-abi.tsv` and writing to a surface it was
# handed. They need different runners AND different link addresses.
#
# THIS ARM EXISTS BECAUSE THE FIRST VERSION OF THIS CHECK COULD NOT SEE THE BUG.
# It ran namaste.sas and bare-metal.sas — both boot-proofs — so subject and
# control were the SAME KIND, and a control drawn from the same category cannot
# expose a category error. The editor therefore shipped one code path for both
# kinds, and the owner pressed the button on spec/atithi.sas and got
# `BadAccess { pc: 2147483648, addr: 0 }`: 0x8000_0000, a fault on the entry
# instruction. Correct output, wrong invocation, and it reads like a broken
# program.
APPLICATION=spec/atithi.sas
EXPECT_APP="अतिथिः"
# Outside the supervisor's gigabyte at 0x8000_0000, which `host` refuses to load
# into. Same value as `LINKED_AT[0]` in crates/yantra/tests/application.rs, so the
# demo and the test place the application identically. Written in the language,
# as `--स्थान` requires: `०षोड्` is the hexadecimal prefix.
# `spec/` spelled literally, as every other path in this file is (PROGRAM,
# CONTROL, APPLICATION above). It was written `"$SPECS/application-load.tsv"`,
# and SPECS is assigned NOWHERE in this script or its environment — under
# `set -u` that aborts the step, and the gate's `round trip` was red for every
# worker until this line was read rather than the failure message believed.
#
# The failure was diagnosable only because SADHANA REFUSES a missing address
# instead of assembling something wrong: `--स्थान wants an address, e.g.
# ०षोड्८०२०००००`. An assembler that defaulted here would have produced an ELF
# linked at 0 and the round trip would have failed somewhere much further away.
APP_LOAD=$(awk '$1 == "0x2000_0000" {print $2}' spec/application-load.tsv)

# Outside the tree. gate.sh fingerprints every modified and untracked file, so a
# harness writing an ELF into the working tree changes what it is measured by —
# and a generated artefact inside the repository inherits every check the tree
# gets, including the secret scanner that reads untracked files.
work=$(mktemp -d) || { echo "  FAIL  could not create a work directory"; exit 1; }
cleanup() { rm -f "$work"/*.elf 2>/dev/null; rmdir "$work" 2>/dev/null; }
trap cleanup EXIT

for f in "$PROGRAM" "$CONTROL"; do
    [ -f "$f" ] || { echo "  FAIL  $f is missing; this check has nothing to run"; exit 1; }
done

# Drift guard, before anything is built. If the program no longer CONTAINS the
# message, the expectation below is about a program that no longer exists.
if ! grep -q "$EXPECT" "$PROGRAM"; then
    echo "  REFUSED  $PROGRAM no longer contains \"$EXPECT\"."
    echo "           The expectation here is recorded for the program as written."
    echo "           If the message changed on purpose, change EXPECT and"
    echo "           check-sassembly-identity.sh together — they must not drift"
    echo "           apart, because they are the same claim on two paths."
    exit 1
fi

# One `run` for both arms: the control is only a control if it goes through the
# identical path. A second, simpler code path for the negative case would be
# testing something other than what arm 1 tests.
roundtrip() {
    src=$1; elf=$work/$(basename "$src" .sas).elf
    cargo run --quiet -p sadhana -- "$src" "$elf" >/dev/null 2>&1 || return 2
    cargo run --quiet -p yantra --bin yantra-run -- "$elf" 2>&1
}

echo "  assembling and running $PROGRAM"
out=$(roundtrip "$PROGRAM")
rc=$?
if [ "$rc" -eq 2 ]; then
    echo "  FAIL  sadhana could not assemble $PROGRAM"
    cargo run --quiet -p sadhana -- "$PROGRAM" "$work/x.elf" 2>&1 | tail -10 | sed 's/^/        /'
    exit 1
fi

if [ -z "$out" ]; then
    echo "  FAIL  yantra-run produced no output at all."
    echo "        An empty result must never read as a pass: 'the expected text is"
    echo "        absent' and 'nothing ran' are different faults with different fixes."
    exit 1
fi

ok=1
case "$out" in
    *"$EXPECT"*) printf '  ok    %-28s %s\n' "UART carries the message" "$EXPECT" ;;
    *) ok=0
       echo "  FAIL  the UART did not carry \"$EXPECT\". What came back:"
       printf '%s\n' "$out" | sed 's/^/        /' ;;
esac

# The halt code is asserted SEPARATELY from the text. A program can print the
# right bytes and then fault, and a check that only greps the output would call
# that a working round trip — which is precisely the state a demo fails in.
case "$out" in
    *"value: $EXPECT_HALT"*) printf '  ok    %-28s %s\n' "finisher" "$EXPECT_HALT (0x5555, exit 0)" ;;
    *) ok=0
       echo "  FAIL  the finisher was not $EXPECT_HALT (0x5555). The program printed its"
       echo "        text but did not halt cleanly, or did not reach the finisher:"
       printf '%s\n' "$out" | grep halt | sed 's/^/        /' ;;
esac
[ "$ok" -eq 1 ] || exit 1

# ---- the control ----------------------------------------------------------
echo "  running the control ($CONTROL writes no characters)"
control_out=$(roundtrip "$CONTROL")
if [ $? -eq 2 ]; then
    echo "  FAIL  sadhana could not assemble the control, $CONTROL."
    echo "        The control is not optional scenery: without it, this check has"
    echo "        shown that the pipeline agrees with the expectation but NOT that"
    echo "        it would have disagreed with a wrong one."
    exit 1
fi
case "$control_out" in
    *"$EXPECT"*)
        echo "  FAIL  the control PASSED the assertion. $CONTROL writes no characters,"
        echo "        so \"$EXPECT\" cannot legitimately appear in its output. The"
        echo "        assertion has stopped discriminating and the green above means"
        echo "        nothing. Do not trust this check until that is explained."
        exit 1 ;;
    *) printf '  ok    %-28s %s\n' "control refuted" "the assertion can fail" ;;
esac

# ---- `.सस`, the Devanagari file extension ---------------------------------
#
# The row asks for `.sas` AND `.सस` to be first-class. Zero `.सस` files are
# tracked, so the extension's file association would otherwise be an ASSERTION
# in a manifest that nothing exercises — and a declaration nobody runs is the
# kind of thing that is wrong for months.
#
# Exercised by COPYING the program to a Devanagari name and requiring BYTE-
# IDENTICAL output. Byte-identity, not "it also works": a pipeline that silently
# took a different path for a non-ASCII filename would still print the right
# message, and the point here is that the two names are the same program.
#
# The copy is made at RUN TIME in the temp directory rather than committed. A
# tracked second copy of `namaste.sas` could drift from the original and this
# check would then be comparing two programs instead of two names for one.
sas_elf=$work/namaste.elf
dev_src=$work/नमस्ते.सस
cp "$PROGRAM" "$dev_src" || { echo "  FAIL  could not write a .सस copy"; exit 1; }
if ! cargo run --quiet -p sadhana -- "$dev_src" "$work/devanagari.elf" >/dev/null 2>&1; then
    echo "  FAIL  sadhana could not assemble a source file named नमस्ते.सस."
    echo "        The row makes .सस first-class; if the toolchain cannot read one,"
    echo "        the editor's file association points at something that does not work."
    exit 1
fi
if cmp -s "$sas_elf" "$work/devanagari.elf"; then
    # Padded by BYTES, not columns — every Devanagari akṣara here is 3 bytes, so
    # a label wide enough to align in a terminal overruns %-28s and the column
    # silently collapses. Kept short deliberately.
    printf '  ok    %-22s %s\n' ".सस" "assembles to the same bytes as .sas"
else
    echo "  FAIL  नमस्ते.सस and $PROGRAM produced DIFFERENT bytes from identical source."
    echo "        The filename changed the output, which means the .सस path is not the"
    echo "        same path — and only one of the two is the one that is tested."
    exit 1
fi

# ---- the application arm, and the dispatch itself --------------------------
#
# `spec/programs.tsv` already records which kind each program is, and
# crates/yantra/tests/application.rs re-derives its `checks` column rather than
# trusting it, so the registry is maintained. The editor reads the `class` column
# to choose a path; this reads the same column, so the check and the extension
# cannot disagree about what a program IS.
class_of() {
    awk -F'\t' -v p="$(basename "$1")" '$1 == p { print $2; found = 1 }
        END { exit !found }' spec/programs.tsv
}

for pair in "$PROGRAM:boot-proof" "$APPLICATION:app"; do
    prog=${pair%:*}; want=${pair#*:}
    got=$(class_of "$prog") || {
        echo "  FAIL  $prog is not listed in spec/programs.tsv."
        echo "        The editor dispatches on that column, so an unlisted program has"
        echo "        no path it can be run by — and guessing one produces a fault that"
        echo "        reads like a broken program rather than a wrong invocation."
        exit 1
    }
    [ "$got" = "$want" ] || {
        echo "  FAIL  spec/programs.tsv calls $prog '$got'; this check assumes '$want'."
        echo "        The arms below are written for the two kinds ADR-0015 defines. If"
        echo "        the registry changed, this check is testing the wrong dispatch."
        exit 1
    }
done
printf '  ok    %-28s %s\n' "registry" "namaste=boot-proof, atithi=app"

echo "  assembling and hosting $APPLICATION"
app_elf=$work/atithi.elf
if ! cargo run --quiet -p sadhana -- --स्थान "$APP_LOAD" "$APPLICATION" "$app_elf" >/dev/null 2>&1; then
    echo "  FAIL  sadhana could not assemble $APPLICATION at $APP_LOAD"
    cargo run --quiet -p sadhana -- --स्थान "$APP_LOAD" "$APPLICATION" "$app_elf" 2>&1 |
        tail -10 | sed 's/^/        /'
    exit 1
fi
app_out=$(cargo run --quiet -p yantra --bin yantra-host -- "$app_elf" 2>&1)
case "$app_out" in
    *"$EXPECT_APP"*) printf '  ok    %-28s %s\n' "surface carries the string" "$EXPECT_APP" ;;
    *) echo "  FAIL  the application's surface did not carry \"$EXPECT_APP\":"
       printf '%s\n' "$app_out" | sed 's/^/        /'
       exit 1 ;;
esac
# An application EXITS; it does not halt the machine. `Ended::Stopped` is the
# machine stopping, which nothing reachable through this ABI produces — so
# asserting the exit separately from the text catches a program that printed and
# then died, the same way the finisher assertion does for a boot-proof.
case "$app_out" in
    *"Exited { status: 0 }"*) printf '  ok    %-28s %s\n' "application exited" "status 0, machine still running" ;;
    *) echo "  FAIL  the application did not exit cleanly (ADR-0015 A5):"
       printf '%s\n' "$app_out" | grep -E 'ended|scause' | sed 's/^/        /'
       exit 1 ;;
esac

# THE DISPATCH CONTROL, BOTH DIRECTIONS. The arms above show each kind works on
# its own path. They do NOT show the two paths are different — and that is the
# entire defect this arm was added for. So run each kind through the OTHER kind's
# path and require both to fail. If either succeeds, one runner handles both and
# the dispatch this check exists to assert is not doing anything.
bare_app=$work/atithi-bare.elf
cargo run --quiet -p sadhana -- "$APPLICATION" "$bare_app" >/dev/null 2>&1
wrong=$(cargo run --quiet -p yantra --bin yantra-run -- "$bare_app" 2>&1)
case "$wrong" in
    *"$EXPECT_APP"*)
        echo "  FAIL  the application produced its output when run as a BOOT-PROOF."
        echo "        The two paths are supposed to be different — if the bare runner can"
        echo "        run an application, the dispatch asserted above is decoration."
        exit 1 ;;
    *) printf '  ok    %-28s %s\n' "app via bare path" "refused (this is the owner's bug)" ;;
esac

hosted_proof=$(cargo run --quiet -p yantra --bin yantra-host -- "$sas_elf" 2>&1)
case "$hosted_proof" in
    *"$EXPECT"*)
        echo "  FAIL  the boot-proof produced its output when run as an APPLICATION."
        echo "        A boot-proof is linked into the supervisor's own gigabyte and writes"
        echo "        to a device; if hosting one works, the loader is not enforcing what"
        echo "        ADR-0015 says it enforces."
        exit 1 ;;
    *) printf '  ok    %-28s %s\n' "boot-proof via host path" "refused" ;;
esac

echo
echo "the round trip: source -> sadhana -> ELF -> yantra -> the message, asserted,"
echo "for BOTH kinds ADR-0015 defines, and neither kind runs on the other's path."
