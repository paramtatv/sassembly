#!/usr/bin/env python3
"""Pack .t1 sources into ONE file for the input channel: `name NUL text NUL`, repeated.

The input channel (`yantra::input`, `t1_image --input`) carries one file, and
the self-image rung `स्वपरीक्षास्वप्रतिबिम्बम्` needs the whole corpus. NUL never
occurs in UTF-8 source text, so it frames without escaping. The rung splits on it.

NAMES ARE DERIVED EXACTLY AS `t1_image` DERIVES THEM, or the two builds would
compile the same sources under different module names: the first line's
`मण्डलम् <name>`, else the file stem (`lib.t1` declares no module and is "lib").
ORDER is the order given, so pass the same list to both sides.

Usage: tools/pack-corpus.py OUT FILE.t1...
"""
import sys, pathlib
out, files = sys.argv[1], sys.argv[2:]
blob = bytearray()
for f in files:
    p = pathlib.Path(f)
    src = p.read_bytes()
    first = src.split(b"\n", 1)[0].decode("utf-8").split()
    name = first[1] if len(first) > 1 and first[0] == "मण्डलम्" else p.stem
    assert b"\0" not in src, f"{f} contains NUL; it cannot be framed"
    blob += name.encode("utf-8") + b"\0" + src + b"\0"
    print(f"  {name:<16} {len(src):>7} octets  {f}")
pathlib.Path(out).write_bytes(blob)
print(f"packed {len(files)} source(s), {len(blob)} octets -> {out}")
