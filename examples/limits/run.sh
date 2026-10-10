#!/bin/sh
# Compile one probe with the released stage1.elf, then run it on the released yantra-run.
# Usage: examples/limits/run.sh <probe> [yantra-run options...] [-- program arguments...]
#   <probe>  a file name in this directory, with or without .t1 (for example thr2)
#   options go before the image (--record-events LOG, --events LOG, --listen ADDR);
#   arguments after a literal -- go to the program, after the image.
# Environment (all optional):
#   YANTRA_RUN  path to the release yantra-run       (default: yantra-run on PATH)
#   STAGE1      path to sassembly-v1.0.2-stage1.elf  (default: ./sassembly-v1.0.2-stage1.elf)
# Other YANTRA_* variables (for example YANTRA_STEPS) apply to the probe's run, not to the compile.
#   MODULE      module name sent to the compiler     (default: शृङ्खला)
#   ROUTINE     entry routine (v1.0.2+): sent as YANTRA_INPUT_ENTRY "MODULE ROUTINE"; unset keeps the compiler's own entry
#   KEEP_ELF    if set, also copy the built image to this path
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
P=${1:?usage: run.sh <probe> [yantra-run args]}; shift
P=${P%.t1}
Y=${YANTRA_RUN:-yantra-run}; S=${STAGE1:-./sassembly-v1.0.2-stage1.elf}
W=$(mktemp -d); trap 'rm -rf "$W"' EXIT
{ printf '%s\0' "${MODULE:-शृङ्खला}"; cat "$HERE/$P.t1"; printf '\0'; } > "$W/blob"
if [ -n "$ROUTINE" ]; then NM="YANTRA_INPUT_ENTRY=${MODULE:-शृङ्खला} $ROUTINE"; else NM=YANTRA_INPUT_NAME=x; fi
env YANTRA_INPUT="$W/blob" "$NM" YANTRA_RAM=2684354560 YANTRA_STEPS=4000000000000 \
  "$Y" "$S" > "$W/sink" 2> "$W/err" || true
grep -q 'status: Some(1200)' "$W/err" || { echo "compile of $P failed:"; grep halt: "$W/err"; exit 2; }
n=$(wc -c < "$W/sink"); tail -c +2 "$W/sink" | dd bs=1 count=$((n-2)) of="$W/p.elf" 2>/dev/null
[ -z "$KEEP_ELF" ] || cp "$W/p.elf" "$KEEP_ELF"
# split the arguments at the first -- (words must not contain spaces)
OPTS=; PRG=; seen=0
for a in "$@"; do
  if [ "$a" = -- ] && [ "$seen" = 0 ]; then seen=1
  elif [ "$seen" = 0 ]; then OPTS="$OPTS $a"
  else PRG="$PRG $a"; fi
done
exec_status=0
# shellcheck disable=SC2086
"$Y" $OPTS "$W/p.elf" $PRG 2>&1 || exec_status=$?
echo "[yantra-run exit $exec_status]"
