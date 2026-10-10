#!/usr/bin/env python3
# A .t1 image's ARGUMENTS ON QEMU (row C-015). QEMU has no argv for a bare-metal
# image, so this applies yantra's own argument rule — crates/yantra/src/input.rs
# inject_arguments and append_run — to a COPY of an ELF, writing the run as a
# blob QEMU loads with `-device loader`. It computes nothing about the program.
# Brought from Uru (reference/qemu_args.py, checked there against yantra-run's
# own "run at" on two images). Its user here, tools/check-virtio-gpu-t1.sh,
# checks it against yantra-run's "run at" again before trusting it.
#
#   the SASARGV and SASARGC tags must each appear exactly once in the loaded
#   image, or it refuses (as yantra does);
#   the run (the arguments joined by zero octets) goes where yantra puts it:
#   storage = align_up(top + 8, 16), its length word at storage - 8, where top
#   is the length of yantra's RAM vector when it injects: the machine's RAM size
#   (YANTRA_RAM) when one is given, or else yantra's default sizing (lib.rs
#   ram_for_image): max(DEFAULT_RAM 20 MiB, extent + RAM_HEADROOM 16 MiB), the
#   extent being the loaded span from the lowest address (.bss included) —
#   pass --ram exactly when yantra had one;
#   the word after SASARGV gets the storage address, after SASARGC the count.
#
# Usage: t1-qemu-args.py [--ram OCTETS] IN.elf OUT.elf ARGS.bin ARG0 ARG1 ...
# Prints "addr <hex>" — where ARGS.bin must be loaded (-device loader) — and
# "run <hex>", which must equal yantra-run's "run at" for the same image.
import struct
import sys

ARGV_TAG = 0x0056_4752_4153_4153
ARGC_TAG = 0x0043_4752_4153_4153


def main():
    argv = sys.argv[1:]
    ram = None
    if argv[0] == "--ram":
        ram = int(argv[1])
        argv = argv[2:]
    src, dst, blob, *args = argv
    elf = bytearray(open(src, "rb").read())
    assert elf[:4] == b"\x7fELF" and elf[4] == 2 and elf[5] == 1, "not a little-endian ELF64"
    phoff, = struct.unpack_from("<Q", elf, 0x20)
    phentsize, phnum = struct.unpack_from("<HH", elf, 0x36)
    loads = []
    for i in range(phnum):
        p_type, _flags, off, vaddr, _paddr, filesz, memsz, _align = struct.unpack_from(
            "<IIQQQQQQ", elf, phoff + i * phentsize)
        if p_type == 1:
            loads.append((vaddr, off, filesz, memsz))
    base = min(v for v, _, _, _ in loads)
    span = max(v + m for v, _, _, m in loads) - base

    def find(tag, what):
        word = struct.pack("<Q", tag)
        hits = []
        for v, off, fs, _ in loads:
            seg = bytes(elf[off:off + fs])
            at = seg.find(word)
            while at >= 0:
                hits.append(off + at)
                at = seg.find(word, at + 1)
        if len(hits) != 1:
            sys.exit("qemu_args.py: the %s tag appears %d times; refused" % (what, len(hits)))
        return hits[0]

    argv_at, argc_at = find(ARGV_TAG, "SASARGV"), find(ARGC_TAG, "SASARGC")
    joined = b"\0".join(a.encode() for a in args)
    top = max(20 << 20, span + (16 << 20)) if ram is None else ram
    storage = (top + 8 + 15) // 16 * 16
    run = base + storage
    struct.pack_into("<Q", elf, argv_at + 8, run)
    struct.pack_into("<Q", elf, argc_at + 8, len(args))
    open(dst, "wb").write(elf)
    open(blob, "wb").write(struct.pack("<Q", len(joined)) + joined)
    print("addr %#x" % (run - 8))
    print("run %#x" % run)


if __name__ == "__main__":
    main()
