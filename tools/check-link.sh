#!/bin/sh
# Multi-file link — task `B-014`, BUILD.md gate B10: "Multi-file link, no GNU ld".
#
# spec/namaste-main.sas calls मुद्रकः, which is defined in spec/lib-mudraka.sas
# and exported with `॥ वैश्विकम् ॥`. Neither file assembles to a working program
# on its own; the build lays them out together and resolves the call.
#
# The check is the OUTPUT, so the linked call has to actually reach the callee
# and the callee has to actually return.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

expected='नमस्ते संसार'

# AN ABSENT TOOL IS `CANNOT RUN` (77), NOT A FAILURE. `gate.sh:307` renders 77 as
# SKIPPED with this reason; exit 1 counts it against the product and stops every
# landing. Two of the three blockers that held the rail through 2026-09-15/16 were
# this shape — a stale binary and an absent wasm32 target, both reported FAILED.
if ! command -v qemu-system-riscv64 >/dev/null; then
  echo "CANNOT RUN: qemu-system-riscv64 is not installed — cannot run on the metal"
  exit 77
fi

cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  "$root/spec/namaste-main.sas" "$root/spec/lib-mudraka.sas" "$tmp/two.elf"

qemu-system-riscv64 -machine virt -nographic -bios none -kernel "$tmp/two.elf" \
  > "$tmp/out" 2>&1 &
qpid=$!
waited=0
while kill -0 "$qpid" 2>/dev/null; do
  if [ "$waited" -ge 15 ]; then kill -9 "$qpid" 2>/dev/null || true; break; fi
  sleep 1
  waited=$((waited + 1))
done
wait "$qpid" 2>/dev/null || true

got=$(head -1 "$tmp/out")
if [ "$got" = "$expected" ]; then
  echo "ok  two files linked and printed: $got"
  echo "    मुद्रकः exported from lib-mudraka.sas, called from namaste-main.sas"
else
  echo "FAIL: expected '$expected'" >&2
  echo "         got '$got'" >&2
  exit 1
fi
