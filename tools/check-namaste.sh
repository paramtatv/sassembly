#!/bin/sh
# The first Devanagari program that SPEAKS — task `B-066`.
#
# spec/namaste.sas holds its message in `ॱदत्त` as a string literal, takes the
# address of it pc-relatively, and writes it a byte at a time to the QEMU virt
# machine's UART at 0x10000000 until it reaches a zero.
#
# The check is the OUTPUT, not the exit code. A program can exit 0 having done
# nothing; this one has to put the right bytes on the wire.
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
  "$root/spec/namaste.sas" "$tmp/namaste.elf"

qemu-system-riscv64 -machine virt -nographic -bios none -kernel "$tmp/namaste.elf" \
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
wait "$qpid" 2>/dev/null || true

got=$(head -1 "$tmp/out")
if [ "$got" = "$expected" ]; then
  echo "ok  a Devanagari program printed: $got"
  echo "    source: spec/namaste.sas   string: उक्तम् … इति   ELF: kosha, no GNU ld"
else
  echo "FAIL: expected '$expected'" >&2
  echo "         got '$got'" >&2
  exit 1
fi

# And the same program at the compressed target — `B-058b2b6`. A mixed-width
# image is where the layout can go wrong invisibly: every branch and every
# label moves when an instruction shrinks, and a wrong address produces a
# program that runs and does the wrong thing rather than one that fails to
# build. Running it is the only check that covers that.
cargo run --quiet -p sadhana -- --संक्षिप्त \
  "$root/spec/namaste.sas" "$tmp/small.elf" >/dev/null

qemu-system-riscv64 -machine virt -nographic -bios none -kernel "$tmp/small.elf" \
  > "$tmp/out2" 2>&1 &
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

got2=$(tr -d '\0' < "$tmp/out2" | head -c 200 | tr -d '\r\n')
if [ "$got2" != "$expected" ]; then
  echo "FAIL: the compressed image printed '$got2', not '$expected'" >&2
  exit 1
fi

wide=$(wc -c < "$tmp/namaste.elf" | tr -d ' ')
small=$(wc -c < "$tmp/small.elf" | tr -d ' ')
echo "compressed image runs too ($small bytes against $wide)"
