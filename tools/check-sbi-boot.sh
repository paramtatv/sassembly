#!/bin/sh
# riscv64 boot under OpenSBI — task `C-001a2`, doc 11 §7.1.1.
#
# Every image this toolchain has run so far ran with `-bios none`: QEMU's reset
# vector jumped straight into our text at 0x80000000 and there was no firmware
# in the machine at all. That is a program on bare metal, not an operating
# system booting. Stage C boots UNDER firmware, which changes three things at
# once:
#
#   * the load address. OpenSBI occupies 0x80000000 and hands its payload
#     0x80200000, so the image has to be built somewhere else (`C-001a1`).
#   * the privilege level. The payload starts in S-mode, not M-mode.
#   * how a character reaches the wire. The UART belongs to the firmware now;
#     a kernel asks for it with an SBI call.
#
# The check is the OUTPUT — `namaste saMsAra`, in SLP1 because a boot payload
# runs before any font or shaper exists (doc 01 §6, use 2 of 3). The exit code
# alone would not do: OpenSBI prints its own banner and shuts the machine down
# cleanly whether or not our payload ever said anything.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# AN ABSENT TOOL IS `CANNOT RUN` (77), NOT A FAILURE. `gate.sh:307` renders 77 as
# SKIPPED with this reason; exit 1 counts it against the product and stops every
# landing. Two of the three blockers that held the rail through 2026-09-15/16 were
# this shape — a stale binary and an absent wasm32 target, both reported FAILED.
if ! command -v qemu-system-riscv64 >/dev/null; then
  echo "CANNOT RUN: qemu-system-riscv64 is not installed — cannot boot"
  exit 77
fi

# ०षोड्८०२००००० is 0x80200000 — where OpenSBI's fw_dynamic leaves its payload.
cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  --स्थान ०षोड्८०२००००० "$root/spec/boot-sbi.sas" "$tmp/boot.elf"

# No -bios: QEMU loads its bundled OpenSBI, which is the point of the task.
qemu-system-riscv64 -machine virt -nographic -kernel "$tmp/boot.elf" \
  > "$tmp/out" 2>&1 &
qpid=$!
waited=0
while kill -0 "$qpid" 2>/dev/null; do
  if [ "$waited" -ge 20 ]; then
    kill -9 "$qpid" 2>/dev/null || true
    echo "FAIL: the machine did not shut down — SBI shutdown never took" >&2
    exit 1
  fi
  sleep 1
  waited=$((waited + 1))
done
wait "$qpid" 2>/dev/null || true

expected='namaste saMsAra'
got=$(tr -d '\r' < "$tmp/out" | tail -1)

if [ "$got" != "$expected" ]; then
  echo "FAIL: expected the last line to be '$expected'" >&2
  echo "         got '$got'" >&2
  echo "     full output follows:" >&2
  tr -d '\r' < "$tmp/out" >&2
  exit 1
fi

# The banner is what says the firmware was really there. Without this a future
# regression to `-bios none` would still print the message and still pass.
if ! grep -aq 'OpenSBI' "$tmp/out"; then
  echo "FAIL: no OpenSBI banner — this booted without firmware" >&2
  exit 1
fi
if ! grep -aq '0x0000000080200000' "$tmp/out"; then
  echo "FAIL: firmware did not hand off at 0x80200000" >&2
  exit 1
fi

echo "ok  a Devanagari program booted under OpenSBI at 0x80200000 and said: $got"
echo "    source: spec/boot-sbi.sas   console: SBI console_putchar   exit: SBI shutdown"
