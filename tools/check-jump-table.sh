#!/bin/sh
# A jump table, on the metal — task `B-095`, ADR-0013.
#
# `spec/jump-table.sas` writes two addresses into `ॱदत्त`, loads the SECOND one
# back at run time and jumps through it. The arm it lands on writes the QEMU
# `virt` finisher: 0x5555 is exit(0) and 0x13333 is exit(1). So the machine
# decides whether the address in the table was right — not a byte comparison
# against a number this toolchain also produced.
#
# The discrimination matters here more than usual. An address written into data
# is a value nothing checks: a wrong one does not fail to assemble, it produces
# a program that jumps somewhere plausible. `B-069d2b` shipped exactly that and
# it printed nothing. So the check runs the table BACKWARDS as well: the two
# entries are swapped and the program must then report failure.
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

# Run an image and echo its exit status, with a timeout: the program spins
# forever if the finisher write never lands, and a hang is a failure.
run() {
  qemu-system-riscv64 -machine virt -nographic -bios none -kernel "$1" \
    >/dev/null 2>&1 &
  qpid=$!
  waited=0
  while kill -0 "$qpid" 2>/dev/null; do
    if [ "$waited" -ge 15 ]; then
      kill -9 "$qpid" 2>/dev/null || true
      echo timeout
      return
    fi
    sleep 1
    waited=$((waited + 1))
  done
  if wait "$qpid"; then echo 0; else echo $?; fi
}

cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  "$root/spec/jump-table.sas" "$tmp/table.elf"

got=$(run "$tmp/table.elf")
if [ "$got" != "0" ]; then
  echo "FAIL: the program did not reach सफलः through the table (exit $got)" >&2
  exit 1
fi

# The same program with the two entries swapped must land on the other arm. A
# check that passes on a table it never read is not a check.
sed 's/॥ अष्टाष्टकाः विफलः सफलः ॥/॥ अष्टाष्टकाः सफलः विफलः ॥/' \
  "$root/spec/jump-table.sas" > "$tmp/swapped.sas"
cmp -s "$root/spec/jump-table.sas" "$tmp/swapped.sas" && {
  echo "FAIL: the swap edited nothing, so the discrimination is untested" >&2
  exit 1
}
cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  "$tmp/swapped.sas" "$tmp/swapped.elf"

got=$(run "$tmp/swapped.elf")
if [ "$got" = "0" ]; then
  echo "FAIL: swapping the table changed nothing — the jump does not read it" >&2
  exit 1
fi

# The same program through SEPARATE compilation — task `B-098`. The address is
# not resolved while assembling here: the object reserves eight zero bytes and a
# `R_RISCV_64` record in `.rela.data`, and the linker writes the value. Until
# `B-098` this path refused the program rather than emit an unrelocated zero.
#
# It has to be run, not merely linked. A wrong pointer in a table produces a
# file that links cleanly and jumps to whatever the zeros happened to become.
cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  --वस्तु "$root/spec/jump-table.sas" "$tmp/table.o"

if command -v riscv64-elf-readelf >/dev/null; then
  n=$(riscv64-elf-readelf -r "$tmp/table.o" 2>/dev/null | grep -c 'R_RISCV_64' || true)
  if [ "$n" -ne 2 ]; then
    echo "FAIL: the object has $n R_RISCV_64 records; the table has two entries" >&2
    echo "  a resolved-too-early address would link and point at nothing" >&2
    exit 1
  fi
fi

cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  --संयोजय "$tmp/table.o" "$tmp/from-object.elf"

got=$(run "$tmp/from-object.elf")
if [ "$got" != "0" ]; then
  echo "FAIL: linked from an object, the table does not reach सफलः (exit $got)" >&2
  exit 1
fi

cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  --वस्तु "$tmp/swapped.sas" "$tmp/swapped.o"
cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  --संयोजय "$tmp/swapped.o" "$tmp/swapped-from-object.elf"
got=$(run "$tmp/swapped-from-object.elf")
if [ "$got" = "0" ]; then
  echo "FAIL: swapping the table changed nothing through the object path" >&2
  exit 1
fi

echo "ok  a Devanagari program jumped through an address it wrote into ॱदत्त"
echo "    and swapping the two entries sends it to the other arm (exit $got)"
echo "    both directly and through .rela.data, with no source at link time"
