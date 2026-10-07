#!/bin/sh
# Virtio-blk driver in userspace (C-004b)
# Checks that the driver can initialize the MMIO device and read a sector.

set -e
root=$(cd "$(dirname "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "CANNOT RUN: qemu-system-riscv64 is not installed."
  exit 77;
}

prog=$root/spec/virtio-blk.sas
[ -f "$prog" ] || {
  echo "RED, as written: spec/virtio-blk.sas does not exist."
  exit 1
}

echo "Assembling..."
cargo run -q -p sadhana -- --स्थान ०षोड्८०२००००० "$prog" "$tmp/vb.elf" || {
  echo "Assembly failed."
  exit 1
}

# Create a dummy disk image with a recognizable pattern at sector 0
dd if=/dev/zero of="$tmp/disk.img" bs=512 count=2048 2>/dev/null
echo "VIRTIO-BLK-MAGIC-SECTOR-00" > "$tmp/disk.img"

echo "Running in QEMU..."
# Redirecting serial to dummy so we can use stdio for something else if needed, 
# or redirect serial to stdio.
# The UART is at 0x10000000. 
qemu-system-riscv64 -machine virt -nographic -bios default \
    -kernel "$tmp/vb.elf" \
    -drive file="$tmp/disk.img",if=none,format=raw,id=hd0 \
    -device virtio-blk-device,drive=hd0 > "$tmp/out" 2>&1 &
qpid=$!
( sleep 5; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
wait "$qpid" 2>/dev/null || true

# Look for the driver's success message
if grep -qa "VIRTIO-BLK-OK" "$tmp/out"; then
  echo "PASS: virtio-blk driver successfully initialized and printed to output."
  exit 0
else
  echo "FAIL: Expected 'VIRTIO-BLK-OK', got:"
  cat "$tmp/out"
  exit 1
fi
