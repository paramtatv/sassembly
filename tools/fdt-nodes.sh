#!/bin/sh
# POSIX sh+od+awk fdt walker (PY3 car C2; the python original is retired, frozen output in tools/check-py3-c23.d).
# Usage: fdt-nodes.sh [--memory | --reg] <file.dtb>
# Differences (documented): reg cells wider than 2 words are refused (awk doubles);
# error text on stderr differs, rc is 1 as the retired python.
mode=count; file=; nargs=0; memory=0; reg=0
for a in "$@"; do
  case $a in
    --memory) memory=1 ;;
    --reg) reg=1 ;;
    *) nargs=$((nargs + 1)); file=$a ;;
  esac
done
if [ "$nargs" -ne 1 ] || { [ "$memory" = 1 ] && [ "$reg" = 1 ]; }; then
  echo "usage: fdt-nodes.sh [--memory | --reg] <file.dtb>" >&2; exit 1
fi
[ "$reg" = 1 ] && mode=reg
[ "$memory" = 1 ] && mode=memory
[ -r "$file" ] && [ ! -d "$file" ] || { echo "cannot read $file" >&2; exit 1; }
od -An -v -tu1 < "$file" | awk -v mode="$mode" '
function die(m) { print m > "/dev/stderr"; bad = 1; exit 1 }
function u32(o) { if (o < 0 || o + 4 > n) die("short read"); return ((b[o]*256 + b[o+1])*256 + b[o+2])*256 + b[o+3] }
function nulat(o,   i) { for (i = o; i < n; i++) if (b[i] == 0) return i; die("no NUL"); }
function name(o, e,   i, s) { s = ""; for (i = o; i < e; i++) { if (b[i] >= 128) die("non-ascii"); s = s sprintf("%c", b[i]) } return s }
function head(s,   p) { p = index(s, "@"); return p ? substr(s, 1, p - 1) : s }
function align4(x) { return x - (x % 4) }
function readnodes(   magic, pos, end, tok, stop, len) {
  magic = u32(0)
  if (magic != 3490578157) die("not a device tree")
  u32(4); pos = u32(8); end = pos + u32(36)
  while (pos < end) {
    tok = u32(pos); pos += 4
    if (tok == 9) return
    if (tok == 1) { stop = nulat(pos); nn++; nm[nn] = name(pos, stop); pos = align4(stop + 4) }
    else if (tok == 3) { len = u32(pos); pos = align4(pos + 8 + len + 3) }
    else if (tok != 2 && tok != 4) die("unknown token")
  }
  die("structure block ended without FDT_END")
}
function props(want, out,   magic, pos, offs, inside, found, tok, stop, len, no, s, i, k) {
  magic = u32(0); u32(4); pos = u32(8); offs = u32(12)
  if (magic != 3490578157) die("not a device tree")
  inside = 0; found = 0
  for (k in out) delete out[k]
  while (1) {
    tok = u32(pos); pos += 4
    if (tok == 9) break
    if (tok == 1) {
      if (inside) break
      stop = nulat(pos); inside = (head(name(pos, stop)) == want); pos = align4(stop + 4)
    } else if (tok == 3) {
      len = u32(pos); no = u32(pos + 4)
      if (inside) {
        stop = nulat(offs + no); s = name(offs + no, stop)
        out[s] = pos + 8 SUBSEP len; found = 1
      }
      pos = align4(pos + 8 + len + 3)
    } else if (tok == 2) { if (inside) break }
    else if (tok != 4) die("unknown token")
  }
  if (!found) die("no properties")
}
function cellval(o, cells,   v, i) { v = 0; for (i = 0; i < cells; i++) v = v * 4294967296 + u32(o + 4*i); return v }
{ for (i = 1; i <= NF; i++) b[n++] = $i + 0 }
END {
  if (bad) exit 1
  if (mode == "count") { readnodes(); if (bad) exit 1; print nn; exit 0 }
  if (mode == "memory") {
    readnodes(); if (bad) exit 1
    for (i = 1; i <= nn; i++) if (head(nm[i]) == "memory") { print i " " nm[i]; exit 0 }
    print "no /memory node in this tree" > "/dev/stderr"; exit 1
  }
  props("", root); if (bad) exit 1
  if (!("#address-cells" in root) || !("#size-cells" in root)) die("no cells")
  split(root["#address-cells"], p, SUBSEP); if (p[2] != 4) die("bad #address-cells"); ac = u32(p[1])
  split(root["#size-cells"], p, SUBSEP); if (p[2] != 4) die("bad #size-cells"); sc = u32(p[1])
  if (ac > 2 || sc > 2) die("cells wider than 2 words unsupported")
  props("memory", mem); if (bad) exit 1
  if (!("reg" in mem)) die("no reg")
  split(mem["reg"], p, SUBSEP)
  want = 4 * (ac + sc)
  if (p[2] != want) die("reg size mismatch")
  printf "%.0f %.0f\n", cellval(p[1], ac), cellval(p[1] + 4*ac, sc)
}'
