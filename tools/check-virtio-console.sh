#!/bin/sh
# Virtio-console driver in userspace (C-004a)
# Checks that the driver can initialize the MMIO device and write to the console.

set -e
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "CANNOT RUN: qemu-system-riscv64 is not installed."
  exit 77;
}

prog=$root/spec/virtio-console.sas
[ -f "$prog" ] || {
  echo "RED, as written: spec/virtio-console.sas does not exist."
  exit 1
}

echo "Assembling..."
cargo run -q -p sadhana -- --स्थान ०षोड्८०२००००० "$prog" "$tmp/vc.elf" || {
  echo "Assembly failed."
  exit 1
}

echo "Running in QEMU..."
qemu-system-riscv64 -machine virt -nographic -bios default \
    -kernel "$tmp/vc.elf" -device virtio-serial-device -device virtconsole,chardev=dummy -chardev null,id=dummy > "$tmp/out" 2>&1 &
qpid=$!
( sleep 5; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
wait "$qpid" 2>/dev/null || true

# Look for the driver's success message
if grep -qa "VIRTIO-CONSOLE-OK" "$tmp/out"; then
  echo "PASS: virtio-console driver successfully initialized and printed to output."
  exit 0
else
  echo "FAIL: Expected 'VIRTIO-CONSOLE-OK', got:"
  cat "$tmp/out"
  exit 1
fi
