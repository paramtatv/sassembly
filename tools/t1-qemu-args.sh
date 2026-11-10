#!/bin/sh
# A .t1 image's ARGUMENTS ON QEMU (row C-015). POSIX sh+od+dd+awk port of t1-qemu-args.py (PY3 car C1):
# same stdout, rc, patched ELF and ARGS blob. It applies yantra's own argument rule —
# the retired Rust source inject_arguments and append_run — to a COPY of an ELF, writing the run as a
# blob QEMU loads with `-device loader`. It computes nothing about the program.
#
#   the SASARGV and SASARGC tags must each appear exactly once in the WRITABLE PT_LOAD bytes of the image, or it refuses (rc 1);
#   storage = align_up(top + 8, 16), its length word at storage - 8, where top is the machine's RAM size
#   (--ram) when one is given, else max(DEFAULT_RAM 20 MiB, extent + RAM_HEADROOM 16 MiB), the extent being
#   the loaded span from the lowest address (.bss included);
#   the word after SASARGV gets the storage address, after SASARGC the count.
#
# Usage: t1-qemu-args.sh [--ram OCTETS] IN.elf OUT.elf ARGS.bin ARG0 ARG1 ...
# Prints "addr <hex>" — where ARGS.bin must be loaded (-device loader) — and "run <hex>".
export LC_ALL=C
die() { echo "t1-qemu-args.sh: $*" >&2; exit 1; }
ram=
if [ "${1:-}" = "--ram" ]; then [ $# -ge 2 ] || die "usage"; ram=$2; shift 2; fi
[ $# -ge 3 ] || die "usage"
src=$1; dst=$2; blob=$3; shift 3
[ -r "$src" ] || die "cannot read $src"
case $ram in ''|*[!0-9]*) [ -z "$ram" ] || die "bad --ram" ;; esac
size=$(wc -c < "$src" | tr -d ' ')
u() { # u BYTES OFFSET -> unsigned decimal, little-endian; empty on short read
  [ "$(($2 + $1))" -le "$size" ] || return 1
  od -An -v -tu$1 -j "$2" -N "$1" "$src" | tr -d ' \n'
}
[ "$size" -ge 64 ] || die "short ELF"
[ "$(od -An -v -tx1 -N 6 "$src" | tr -d ' \n')" = 7f454c460201 ] || die "not a little-endian ELF64"
phoff=$(u 8 32) || die "short ELF"
phentsize=$(u 2 54) || die "short ELF"
phnum=$(u 2 56) || die "short ELF"
loads=
i=0
while [ "$i" -lt "$phnum" ]; do
  p=$((phoff + i * phentsize))
  [ "$((p + 56))" -le "$size" ] || die "short program header"
  if [ "$(u 4 "$p")" = 1 ]; then
    loads="$loads $(u 8 $((p + 8))),$(u 8 $((p + 16))),$(u 8 $((p + 32))),$(u 8 $((p + 40))),$(u 4 $((p + 4)))"   # off,vaddr,filesz,memsz,flags
  fi
  i=$((i + 1))
done
[ -n "$loads" ] || die "no PT_LOAD"
base=; end=
for l in $loads; do
  IFS=, read -r off vaddr fs ms fl <<EOT
$l
EOT
  [ -z "$base" ] || [ "$vaddr" -lt "$base" ] && base=$vaddr
  e=$((vaddr + ms)); [ -z "$end" ] || [ "$e" -gt "$end" ] && end=$e
done
span=$((end - base))

# find TAGHEX LABEL -> file offset of the single occurrence (counted over all PT_LOAD file ranges)
find() {
  res=$(for l in $loads; do
    IFS=, read -r off vaddr fs ms fl <<EOT
$l
EOT
    [ "$fs" -gt 0 ] || continue
    [ "$((fl & 2))" -ne 0 ] || continue   # GAP-TAGSCAN: only writable PT_LOAD bytes carry a tag
    od -An -v -tx1 -j "$off" -N "$fs" "$src" | awk -v tag="$1" -v off="$off" '
      BEGIN { buf = ""; skip = 0 }
      { h = $0; gsub(/ /, "", h); buf = buf h
        s = 1
        while ((i = index(substr(buf, s), tag)) > 0) {
          p = s + i - 1
          if ((p - 1) % 2 == 0) { print off + skip + (p - 1) / 2; s = p + 1 } else s = p + 1
        }
        keep = length(tag) - 2
        if (length(buf) > keep) { cut = length(buf) - keep; cut -= cut % 2; buf = substr(buf, cut + 1); skip += cut / 2 }
      }'
  done)
  n=$(printf '%s\n' "$res" | grep -c .)
  [ "$n" -eq 1 ] || die "the $2 tag appears $n times; refused"
  printf '%s' "$res"
}
argv_at=$(find 5341534152475600 SASARGV) || exit 1
argc_at=$(find 5341534152474300 SASARGC) || exit 1
[ "$((argv_at + 16))" -le "$size" ] && [ "$((argc_at + 16))" -le "$size" ] || die "tag word beyond the file"

# the run: the arguments joined by zero octets
n=$#
joined=$dst.joined.$$
trap 'rm -f "$joined"' EXIT HUP INT TERM
: > "$joined"
first=1
for a in "$@"; do
  [ "$first" = 1 ] || printf '\000' >> "$joined"
  printf '%s' "$a" >> "$joined"
  first=0
done
jlen=$(wc -c < "$joined" | tr -d ' ')
if [ -z "$ram" ]; then
  top=$((span + 16777216)); [ "$top" -lt 20971520 ] && top=20971520
else top=$ram; fi
storage=$(( (top + 8 + 15) / 16 * 16 ))
run=$((base + storage))

le8() { # le8 VALUE -> 8 octal-escaped octets, little-endian
  v=$1; o=; k=0
  while [ "$k" -lt 8 ]; do o="$o\\$(printf '%03o' $((v & 255)))"; v=$((v >> 8)); k=$((k + 1)); done
  printf '%s' "$o"
}
cp "$src" "$dst" || exit 1
printf "$(le8 "$run")" | dd of="$dst" bs=1 seek=$((argv_at + 8)) conv=notrunc 2>/dev/null || exit 1
printf "$(le8 "$n")" | dd of="$dst" bs=1 seek=$((argc_at + 8)) conv=notrunc 2>/dev/null || exit 1
{ printf "$(le8 "$jlen")"; cat "$joined"; } > "$blob" || exit 1
printf 'addr 0x%x\n' $((run - 8))
printf 'run 0x%x\n' "$run"
