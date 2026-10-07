#!/bin/sh
# tools/check-float-oracle.sh — row `V-012`, the SEMANTIC half of the float oracle.
#
# ## What this proves that the other two float checks cannot
#
# `crates/yantra/tests/fp_oracle.rs` has two halves and says plainly what neither
# reaches: *"no independent authority has checked what these instructions COMPUTE
# — only what they decode to and that they run"*. Its decode half asks GNU
# binutils about encodings; its coverage half asks the machine not to halt
# `Unimplemented`. **NEITHER WOULD HAVE CAUGHT `V-001`'s ONE REAL DEFECT**, which
# was `fnmsub`/`fnmadd` implemented with their signs swapped: the encodings were
# right, objdump agreed with the table and with the machine, and the arithmetic
# was still wrong.
#
# `crates/sadhana-t1/tests/t1_exec_encode.rs`'s
# `every_float_case_the_oracle_carries_encodes_to_the_word_the_oracle_gives`
# (`V-003`) closes the encode side for the `.t1` port — 432 cases, 62 families, 0
# disagreements against `riscv64-elf-as`. That is also about encoding.
#
# THIS RUNS THE ARITHMETIC ON TWO MACHINES. `oracle/float-oracle.sas` is its own
# oracle: every check compares a result's BIT PATTERN through `प्लवसंचारः` into an
# integer register and branches on an INTEGER compare, so the verdict never rests
# on a float comparison and a wrong `fadd.d` cannot be hidden by an equally wrong
# `feq.d`. It halts `0x5555` only if all TWENTY-SEVEN checks pass. We run it on
# `yantra` and on `qemu-system-riscv64` and require both.
#
# ## What the twenty-seven are, and why five groups and not one
#
#    1-6   D — fadd.d, fmul.d, fsub.d, fsqrt.d, feq.d both ways
#    7-12  F — the same five in single precision, through `flw` and `fmv.x.w`
#   13-14  fcvt.s.d and fcvt.d.s — the CONVERSIONS
#   15-19  ROUNDING and SATURATION — fdiv in both widths, the 1.5/2.5 tie, 2^31
#   20-27  SPECIAL VALUES — infinity, the canonical NaN, signed zero
#
# The F half is here because `V-012`'s margin named exactly this gap: *"NOT
# COVERED: six operations, D only. F (32-bit) is exercised by `V-003`'s encode
# corpus but not by this semantic one"*. Encoding `fadd.s` to the right word and
# COMPUTING a single-precision sum are different claims.
#
# The conversions are here because groups 1-6 and 7-12 are each self-contained
# within one width: a machine could pass both and still have `fcvt` wired to the
# wrong format, because nothing read a value one width wrote with the other
# width's instruction. 13 and 14 do.
#
# THE SPECIAL-VALUE GROUP EXISTS BECAUSE 1-19 ARE ALL FINITE AND NORMAL, so
# every one of them would pass on a float unit that has no idea what an infinity
# is. Two of the eight cannot be answered by asking the host: 21 requires
# `0.0/0.0` to be RISC-V's ONE canonical NaN, `0x7FF8000000000000`, where an x86
# divide's default NaN is `0xFFF8000000000000`; and 25 requires `fcvt.w.d` of a
# NaN to be `0x7FFFFFFF`, the maximum signed value, where x86's `cvtsd2si`
# answers the indefinite `0x80000000` and ARM's `fcvtzs` answers 0. A
# passthrough implementation fails exactly those two and passes every other
# check in this file. 24 is the signed-zero one: `-1.0 * 0.0` is
# `0x8000000000000000`, and a unit that normalised negative zero to positive
# answers 0 and fails it alone. 26 and 27 are F's own patterns, not D's
# narrowed.
#
# THE ROUNDING GROUP EXISTS BECAUSE 1-14 CANNOT ROUND AT ALL. Every constant in
# them is exact in both formats, so the margin's own last line — *"no check here
# reads `fflags`, and no rounding mode other than RNE is exercised"* — was about
# a program in which nothing rounded. 15 and 16 divide 1 by 3, whose quotient is
# representable in neither format, so the last bit of the answer IS the rounding
# decision. 17 and 18 convert 1.5 and 2.5 and are a PAIR: ties-to-even answers 2
# and 2, truncation answers 1 and 2, ties-away answers 2 and 3, so neither value
# alone pins the mode. 19 converts 2^31, which does not fit a signed 32-bit
# integer, and the answer is saturation to 0x7FFFFFFF rather than a wrap.
#
# THE `fflags` HALF IS STILL OPEN AND IS NOT FAKED. `yantra`'s `csr_read`
# (`crates/yantra/src/lib.rs:1738`) implements 0x100, 0x104, 0x105, 0x140-0x144,
# 0x180, 0xc01 and four vector numbers and refuses the rest with `None` — an
# illegal instruction. 0x001 (`fflags`), 0x002 (`frm`) and 0x003 (`fcsr`) are not
# among them, so a `csrrs` of `fflags` traps before it answers and `csrrwi` of
# `frm` cannot select a mode. A check written that way would be red for a
# REGISTER FILE reason while claiming to be about arithmetic.
#
# AND THE PROGRAM SAYS WHICH GROUP FAILED. Every check used to branch to one
# `विफलः` with a hardcoded status 1, so a red said only *a float check failed*
# and left the reader to bisect. There are FIVE finishers now — status 1 the D
# half, 2 the F half, 3 the conversions, 4 the rounding group, 5 the special
# values — and the failure path below reads the status out of the halt and names
# the group. EACH CHECK OF GROUP 5 IS KNOWN NON-VACUOUS BY FALSIFIER: all eight
# were perturbed in the arithmetic alone and each halted 0x53333, so a green
# here is eight assertions and not eight no-ops, and the group reports itself
# and not another.
#
# `qemu-riscv64` and `spike` are not installed here; `qemu-system-riscv64` is, and
# a bare-metal image is what it takes.
#
# ## Why it lives in `oracle/` and not `spec/`
#
# Owner ruling Q6, 2026-09-29: `spec/` is bound to `spec/programs.tsv` and to
# `crates/yantra/tests/application.rs`, which re-derives every column of that
# table from each program's source under ADR-0015 — 47 OS proofs and one
# application. A differential oracle is neither, and a row for it would either be
# false or would dilute the measurement that table exists to make. Measured
# earlier the same day with `bench/arena-benchmark.sas`: dropping it into `spec/`
# failed `the_table_lists_exactly_the_programs_in_spec` immediately.
#
#   spec/    formal OS proofs, listed in programs.tsv
#   bench/   performance work        (arena-benchmark.sas)
#   oracle/  differential ISA work   (float-oracle.sas)
#
# Exit: 0 both machines agree on success · 1 either disagrees · 77 CANNOT RUN
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
# CLEAN UP ON EVERY EXIT, not only the success path: `tools/paradigm-census.sh`
# leaked 204 directories into $TMPDIR over seven days doing it the other way.
trap 'rm -rf "$tmp"' EXIT INT TERM

# AN ABSENT TOOL IS `CANNOT RUN` (77), NOT A FAILURE. `gate.sh:307` renders 77 as
# SKIPPED with this reason; exit 1 counts it against the product and stops every
# landing. Two of the three blockers that held the rail through 2026-09-15/16
# were this shape — a stale binary and an absent wasm32 target, both reported
# FAILED.
if ! command -v qemu-system-riscv64 >/dev/null; then
  echo "CANNOT RUN: qemu-system-riscv64 is not installed — the INDEPENDENT half"
  echo "  of this differential check is the part that needs it, so running only"
  echo "  yantra would be this repository agreeing with itself"
  exit 77
fi

elf=$tmp/float-oracle.elf
cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  "$root/oracle/float-oracle.sas" "$elf" >/dev/null

# ── yantra ──────────────────────────────────────────────────────────────────
#
# `yantra-run` EXITS NON-ZERO ON A BAD HALT since `W-344`. The margin this
# replaces read "ALWAYS EXITS ZERO", and the incident is why it is kept: an
# earlier check that trusted the exit code passed over a `StepLimit` halt that
# executed almost nothing. The `|| true` below stays deliberate — this script
# wants to READ the log on a failure rather than abort at it under `set -e`. `0x5555` (21845) is the only success sentinel (`yantra/src/lib.rs:310`);
# `0x3333 | (n<<16)` is failure with status n.
cargo run --quiet --manifest-path "$root/Cargo.toml" -p yantra --bin yantra-run -- \
  "$elf" >/dev/null 2>"$tmp/yantra.log" || true
if ! grep -q 'value: 21845' "$tmp/yantra.log"; then
  echo "FAIL: yantra did not reach the success sentinel" >&2
  grep -E 'halt:|steps:' "$tmp/yantra.log" >&2 || cat "$tmp/yantra.log" >&2
  # NAME THE GROUP RATHER THAN LEAVE IT TO BE BISECTED. `yantra-run` prints
  # `status: Some(n)` for a finisher halt, and the program's five failure
  # labels write 0x3333 | (n << 16) — 0x13333 through 0x53333. An
  # absent status is NOT one of the four: it is a StepLimit or a trap, which is
  # a different failure and must not be reported as a wrong answer.
  st=$(sed -n 's/.*status: Some(\([0-9]*\)).*/\1/p' "$tmp/yantra.log" | tail -1)
  case "${st:-}" in
    1) echo "  GROUP: checks 1-6, the D half — fadd.d/fmul.d/fsub.d/fsqrt.d/feq.d" >&2 ;;
    2) echo "  GROUP: checks 7-12, the F half — fadd.s/fmul.s/fsub.s/fsqrt.s/feq.s" >&2 ;;
    3) echo "  GROUP: checks 13-14, the conversions — fcvt.s.d/fcvt.d.s" >&2 ;;
    4) echo "  GROUP: checks 15-19, rounding and saturation — fdiv.d/fdiv.s, the" >&2
       echo "    1.5/2.5 tie through fcvt.w.d, and 2^31 saturating to 0x7FFFFFFF" >&2 ;;
    5) echo "  GROUP: checks 20-27, the special values — +inf from a divide by" >&2
       echo "    zero, the CANONICAL NaN 0x7FF8000000000000 from 0.0/0.0 and from" >&2
       echo "    sqrt(-1.0), feq.d(NaN,NaN)=0, -0.0 from -1.0*0.0, fcvt.w.d(NaN)" >&2
       echo "    = 0x7FFFFFFF, and the F patterns 0x7F800000 and 0x7FC00000" >&2 ;;
    "") echo "  NO FINISHER STATUS: this is not a failed check — the program never" >&2
        echo "  reached one of its four failure labels, so read the halt line above" >&2 ;;
    *) echo "  UNKNOWN STATUS $st: the program has five failure labels writing 1" >&2
       echo "  through 5, so a sixth value means the finisher write itself is wrong" >&2 ;;
  esac
  exit 1
fi
steps=$(sed -n 's/^steps: \([0-9]*\).*/\1/p' "$tmp/yantra.log" | tail -1)

# ── qemu-system-riscv64, the authority this repository did not write ─────────
#
# The reset vector on `-bios none` goes to 0x80000000 and does not read
# `e_entry`, so the text must be mapped exactly there — which `kosha` does.
qemu-system-riscv64 -machine virt -nographic -bios none -kernel "$elf" \
  >/dev/null 2>&1 &
qpid=$!
# ONLY THIS PID IS EVER KILLED. Never a name-grep: one binary can have several
# owners and killing by name reaps another session's run.
waited=0
while kill -0 "$qpid" 2>/dev/null; do
  if [ "$waited" -ge 15 ]; then
    kill -9 "$qpid" 2>/dev/null || true
    echo "FAIL: the program never reached the finisher on qemu (still running after ${waited}s)" >&2
    echo "  yantra reached it in ${steps:-?} steps, so the two machines DISAGREE" >&2
    exit 1
  fi
  sleep 1
  waited=$((waited + 1))
done

if wait "$qpid"; then
  echo "ok  float arithmetic agrees on two machines — yantra and qemu-system-riscv64"
  echo "    oracle/float-oracle.sas: 27 checks, each comparing an IEEE BIT PATTERN"
  echo "    through प्लवसंचारः and branching on an integer compare"
  echo "      1-6   D            fadd.d fmul.d fsub.d fsqrt.d feq.d both ways"
  echo "      7-12  F            the same five in single precision, via flw"
  echo "      13-14 conversions  fcvt.s.d and fcvt.d.s"
  echo "      15-19 rounding     1/3 in both widths, the 1.5/2.5 tie, 2^31"
  echo "      20-27 special      +inf, THE canonical NaN, -0.0, fcvt.w.d(NaN)"
  echo "    NOT COVERED: fflags and a non-default frm — this program reads neither"
  echo "    (yantra has 0x001/0x002/0x003 since V-009 (i-d): v009d_hardware_agreement.rs)"
  echo "    yantra: halt 0x5555 in ${steps:-?} executed instructions"
  echo "    qemu:   exit 0 (the program's own finisher write decides)"
else
  status=$?
  echo "FAIL: the program reported failure on qemu (exit $status) while yantra" >&2
  echo "  reached 0x5555 in ${steps:-?} steps — so the arithmetic differs between" >&2
  echo "  this machine and an independent one, and yantra is what moves" >&2
  exit 1
fi
