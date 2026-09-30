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
# `feq.d`. It halts `0x5555` only if all six checks pass. We run it on `yantra`
# and on `qemu-system-riscv64` and require both.
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
# `yantra-run` ALWAYS EXITS ZERO — the status is on stderr, and an earlier check
# that trusted the exit code passed over a `StepLimit` halt that executed almost
# nothing. `0x5555` (21845) is the only success sentinel (`yantra/src/lib.rs:310`);
# `0x3333 | (n<<16)` is failure with status n.
cargo run --quiet --manifest-path "$root/Cargo.toml" -p yantra --bin yantra-run -- \
  "$elf" >/dev/null 2>"$tmp/yantra.log" || true
if ! grep -q 'value: 21845' "$tmp/yantra.log"; then
  echo "FAIL: yantra did not reach the success sentinel" >&2
  grep -E 'halt:|steps:' "$tmp/yantra.log" >&2 || cat "$tmp/yantra.log" >&2
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
  echo "    oracle/float-oracle.sas: 6 checks, each comparing an IEEE BIT PATTERN"
  echo "    through प्लवसंचारः and branching on an integer compare"
  echo "    yantra: halt 0x5555 in ${steps:-?} executed instructions"
  echo "    qemu:   exit 0 (the program's own finisher write decides)"
else
  status=$?
  echo "FAIL: the program reported failure on qemu (exit $status) while yantra" >&2
  echo "  reached 0x5555 in ${steps:-?} steps — so the arithmetic differs between" >&2
  echo "  this machine and an independent one, and yantra is what moves" >&2
  exit 1
fi
