#!/bin/sh
# Assemble a Devanagari program with our own toolchain and run it on the metal.
#
# Task B-010, and the first end-to-end proof that SANSOS produces something a
# machine will execute: source in Devanagari, ELF written by `kosha`, no GNU
# `ld` anywhere in the path.
#
# The program sums 1..10, checks the answer against 55, and writes to the QEMU
# `virt` machine's SiFive test finisher — 0x5555 for pass, which is exit(0), and
# 0x13333 for fail, which is exit(1). So the EMULATOR decides whether the
# arithmetic was right, not a byte comparison.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# AN ABSENT TOOL IS `CANNOT RUN` (77), NOT A FAILURE. `gate.sh:307` renders 77 as
# SKIPPED with this reason; exit 1 counts it against the product and stops every
# landing. Two of the three blockers that held the rail through 2026-09-15/16 were
# this shape — a stale binary and an absent wasm32 target, both reported FAILED.
if ! command -v qemu-system-riscv64 >/dev/null; then
  echo "CANNOT RUN: qemu-system-riscv64 is not installed — cannot run on the metal"
  exit 77
fi

cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  "$root/spec/bare-metal.sas" "$tmp/sansos.elf"

# The reset vector on `-bios none` goes to 0x80000000 and does not read
# e_entry, so the text must be mapped exactly there.
qemu-system-riscv64 -machine virt -nographic -bios none -kernel "$tmp/sansos.elf" \
  >/dev/null 2>&1 &
qpid=$!
# The program spins forever if the finisher write does not land, so a timeout
# is part of the check rather than a convenience.
waited=0
while kill -0 "$qpid" 2>/dev/null; do
  if [ "$waited" -ge 15 ]; then
    kill -9 "$qpid" 2>/dev/null || true
    echo "FAIL: the program never reached the finisher (still running after ${waited}s)" >&2
    exit 1
  fi
  sleep 1
  waited=$((waited + 1))
done

if wait "$qpid"; then
  echo "ok  a Devanagari program summed 1..10 and exited 0 on qemu-system-riscv64"
  echo "    source: spec/bare-metal.sas   ELF: kosha, no GNU ld"
else
  status=$?
  echo "FAIL: the program ran and reported failure (exit $status)" >&2
  exit 1
fi
