#!/bin/sh
# Pack .t1 sources into ONE file for the input channel: `name NUL text NUL`, repeated.
# POSIX sh port of pack-corpus.py (PY3 car C1): same stdout, rc and OUT octets.
#
# NAMES ARE DERIVED EXACTLY AS `t1_image` DERIVES THEM: the first line's
# `मण्डलम् <name>`, else the file stem (`lib.t1` declares no module and is "lib").
# ORDER is the order given. A source containing NUL is refused (rc 1, OUT not written).
#
# Usage: tools/pack-corpus.sh OUT FILE.t1...
# Known limit (same as no real input): the first line is split on ASCII white space only.
export LC_ALL=C
[ $# -ge 1 ] || { echo "usage: pack-corpus.sh OUT FILE.t1..." >&2; exit 1; }
out=$1; shift
n=$#
tmp=$out.tmp.$$
: > "$tmp" || exit 1
trap 'rm -f "$tmp"' EXIT HUP INT TERM
ws=$(printf ' \t\r\f\v')
for f in "$@"; do
  [ -f "$f" ] && [ -r "$f" ] || { echo "pack-corpus.sh: cannot read $f" >&2; exit 1; }
  size=$(wc -c < "$f" | tr -d ' ')
  nonul=$(tr -d '\000' < "$f" | wc -c | tr -d ' ')
  [ "$size" = "$nonul" ] || { echo "pack-corpus.sh: $f contains NUL; it cannot be framed" >&2; exit 1; }
  first=$(head -n 1 "$f")
  set -f
  old=$IFS; IFS=$ws
  set -- $first
  IFS=$old
  set +f
  name=
  if [ $# -gt 1 ] && [ "$1" = "मण्डलम्" ]; then
    name=$2
  else
    b=${f##*/}
    case $b in
      ?*.*) name=${b%.*} ;;
      *) name=$b ;;
    esac
  fi
  # codepoint count of the name, for the %-16s pad (python pads by characters)
  cp=$(printf '%s' "$name" | od -An -v -tu1 | awk '{for(i=1;i<=NF;i++) if($i<128||$i>=192) c++} END{print c+0}')
  pad=$((16 - cp)); sp=
  while [ "$pad" -gt 0 ]; do sp="$sp "; pad=$((pad - 1)); done
  { printf '%s\000' "$name"; cat "$f"; printf '\000'; } >> "$tmp"
  printf '  %s%s %7d octets  %s\n' "$name" "$sp" "$size" "$f"
done
total=$(wc -c < "$tmp" | tr -d ' ')
mv "$tmp" "$out" || exit 1
trap - EXIT HUP INT TERM
echo "packed $n source(s), $total octets -> $out"
