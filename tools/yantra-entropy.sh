#!/bin/sh
# yantra-entropy.sh — run a program under yantra-run with HOST ENTROPY (spec/entropy/yadrucchika-srotah.t1).
#
#   tools/yantra-entropy.sh [--pool N] [--record FILE | --replay FILE] ROOT YANTRA-RUN IMAGE [ARGS...]
#
# Yantra has no entropy source, so this launcher writes a FRESH pool file ".entropy" into the granted file root
# ROOT before the run (N octets from /dev/urandom, default 16000, and at most 16383: the library's buffer is
# 16384 and a pool that large does not fit) and REMOVES it after: a pool is never reused by a second run.
# The program reads it through the file window (the library keeps a cursor, so one draw never repeats another);
# a pool that runs out is the named refusal 0x3D33333, a missing one 0x3D23333, never zeros.
#
#   --record FILE   keep a copy of the pool as the run's REPLAY RECORD (a new file, mode 0600; an existing
#                   FILE is refused)
#   --replay FILE   use that record as the pool (exact: same octets, the pool is not regenerated)
#
# Entropy output is not deterministic, so such a run is excluded from byte-identity checks unless replayed.
# YANTRA-RUN may be tools/t1-run.sh: the pool is an ordinary file in ROOT, so a native run reads it through
# the native file window as yantra-run does (no entropy special case anywhere in the runtime). That native ==
# yantra here is checked by tools/check-native-entropy.sh's native leg (same replayed pool, short pool and missing
# pool, byte for byte).
set -eu
pool=16000; rec=""; rep=""
while :; do
  case "${1:-}" in
    --pool) pool=$2; shift 2;;
    --record) rec=$2; shift 2;;
    --replay) rep=$2; shift 2;;
    *) break;;
  esac
done
[ $# -ge 3 ] || { echo "usage: $0 [--pool N] [--record F | --replay F] ROOT YANTRA-RUN IMAGE [ARGS...]" >&2; exit 2; }
root=$1; run=$2; img=$3; shift 3
[ -d "$root" ] || { echo "yantra-entropy: $root is not a directory" >&2; exit 2; }
[ "$pool" -ge 0 ] && [ "$pool" -le 16383 ] || { echo "yantra-entropy: --pool must be 0..16383" >&2; exit 2; }
[ -z "$rec" ] || [ -z "$rep" ] || { echo "yantra-entropy: --record and --replay are exclusive" >&2; exit 2; }
f=$root/.entropy
# a pool or record is key material: owner-only (0600), and never written THROUGH an existing name or a
# dangling symlink (set -C: the redirection refuses an existing name, including a symlink, dangling or not)
umask 077
[ ! -e "$f" ] && [ ! -L "$f" ] || { echo "yantra-entropy: $f already exists (a pool is never reused); remove it" >&2; exit 2; }
set -C
# create the pool FIRST, then own it: a create that loses a race to another process must not delete that file
: > "$f" || { echo "yantra-entropy: $f appeared before it could be created; not touched" >&2; exit 2; }
trap 'rm -f "$f"' EXIT INT TERM
set +C
if [ -n "$rep" ]; then
  cat "$rep" >> "$f"
else
  head -c "$pool" /dev/urandom >> "$f"
  [ -z "$rec" ] || { set -C; cat "$f" > "$rec"; set +C; }
fi
"$run" --files "$root" "$img" "$@"
