#!/usr/bin/env bash
# Separate compilation, from the command line — task `B-069d2c`, doc 03 §3.3.
#
# `tools/check-link.sh` builds the same two files in ONE invocation, which is
# what `बन्धकः` did: it held both parse trees and re-encoded everything. That
# satisfies gate B10 and is not separate compilation.
#
# This assembles each file on its own, with no knowledge of the other, and links
# the objects afterwards. The second invocation cannot see the first's source —
# only its bytes, its symbol table and its holes.
#
# The program printing is the check. A wrong address here does not fail to
# build: `B-069d2b` shipped one that linked, ran, and printed nothing, because a
# reference to a string in the same file was resolved before the other file was
# laid in front of it.
set -euo pipefail

cd "$(dirname "$0")/.."
root=$(pwd)

# AN ABSENT TOOL IS `CANNOT RUN` (77), NOT A FAILURE. `gate.sh:307` renders 77 as
# SKIPPED with this reason; exit 1 counts it against the product and stops every
# landing. Two of the three blockers that held the rail through 2026-09-15/16 were
# this shape — a stale binary and an absent wasm32 target, both reported FAILED.
if ! command -v qemu-system-riscv64 >/dev/null; then
  echo "CANNOT RUN: qemu-system-riscv64 is not installed — cannot run on the metal"
  exit 77
fi

tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

expected='नमस्ते संसार'

# One file at a time. Neither command is told the other file exists.
cargo run --quiet -p sadhana -- --वस्तु "$root/spec/namaste-main.sas" "$tmp/main.o" >/dev/null
cargo run --quiet -p sadhana -- --वस्तु "$root/spec/lib-mudraka.sas" "$tmp/lib.o" >/dev/null

# An object must actually carry holes, or the "separate" part is a fiction and
# this check would pass on a build that resolved everything too early.
if command -v riscv64-elf-readelf >/dev/null; then
  n=$(riscv64-elf-readelf -r "$tmp/main.o" 2>/dev/null | grep -c 'R_RISCV_' || true)
  if [ "$n" -lt 2 ]; then
    echo "FAIL: main.o has $n relocations; it should have the call and the address pair" >&2
    exit 1
  fi
fi

cargo run --quiet -p sadhana -- --संयोजय "$tmp/main.o" "$tmp/lib.o" "$tmp/joined.elf" >/dev/null

# The names must survive the link — task `B-103`. `--संयोजय` wrote text and data
# and dropped every symbol, so `nm` reported nothing on an image whose
# single-file build shows `मुद्रकः`. A stranger's tools are how this project
# checks its own object format, and that argument does not stop at the object.
if command -v riscv64-elf-nm >/dev/null; then
  if ! riscv64-elf-nm "$tmp/joined.elf" 2>/dev/null | grep -q 'मुद्रकः'; then
    echo "FAIL: the linked image carries no symbol table" >&2
    riscv64-elf-nm "$tmp/joined.elf" >&2 2>&1 || true
    exit 1
  fi
fi

qemu-system-riscv64 -machine virt -nographic -bios none -kernel "$tmp/joined.elf" \
  > "$tmp/out" 2>&1 &
qpid=$!
waited=0
while kill -0 "$qpid" 2>/dev/null; do
  if [ "$waited" -ge 15 ]; then
    kill -9 "$qpid" 2>/dev/null || true
    break
  fi
  sleep 1
  waited=$((waited + 1))
done

got=$(tr -d '\0' < "$tmp/out" | head -c 200 | tr -d '\r\n')
if [ "$got" != "$expected" ]; then
  echo "FAIL: printed '$got', not '$expected'" >&2
  echo "  the objects linked and the program ran, so an address is wrong" >&2
  exit 1
fi

echo "ok  two files assembled apart, linked from objects, printed: $expected"
echo "    no source at link time · no GNU ld · relocations applied through the derived bit map"
echo "    and nm reads its names back out of the linked image"
