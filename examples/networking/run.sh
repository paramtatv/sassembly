#!/bin/sh
# Reproduces the NETWORKING.md example with the v1.0.0 release binaries. Needs: tar, nc, python3.
# Usage: ./run.sh [dir-containing-tarball-and-stage1.elf]   (default: .)
set -e
D=${1:-.}; V=v1.0.0; T=sassembly-$V-linux-x86_64
HERE=$(cd "$(dirname "$0")" && pwd)
cd "$D"
[ -f $T.tar.gz ] || gh release download $V -R paramtatv/sassembly -p "$T.tar.gz" -p SHA256SUMS-binaries
[ -f sassembly-$V-stage1.elf ] || gh release download $V -R paramtatv/sassembly -p sassembly-$V-stage1.elf -p SHA256SUMS
tar xzf $T.tar.gz; Y=$PWD/$T/yantra-run
# 1. compile echo.t1 with the self-hosted compiler (module and entry names are fixed by it)
{ printf 'शृङ्खला\0'; cat "$HERE/echo.t1"; printf '\0'; } > p.blob
YANTRA_INPUT=p.blob YANTRA_INPUT_NAME=x YANTRA_RAM=2684354560 YANTRA_STEPS=4000000000000 \
  $Y sassembly-$V-stage1.elf > sink 2> s.err || true
grep 'halt:' s.err                      # status Some(1200) = built
n=$(wc -c < sink); tail -c +2 sink | dd bs=1 count=$((n-2)) of=echo.elf 2>/dev/null
# 2. LIVE: serve one client on loopback, record the event log
rm -f live.log
$Y --record-events live.log --listen 127.0.0.1:5577 echo.elf > count.bin 2> live.err &
P=$!
sleep 1
printf 'hello\n' | nc -q1 127.0.0.1 5577 > client.out || true
wait $P || true
echo "--- client received:"; cat client.out
echo "--- stdout (little-endian octets-count word):"; od -An -tu8 count.bin
echo "--- yantra-run stderr (socket lines):"; grep -E '^socket:|^halt:|^steps:' live.err
echo "--- event log:"; cat live.log
# 3. REPLAY the log, no network
$Y --events live.log echo.elf > count2.bin 2> replay.err || true
echo "--- replay stdout:"; od -An -tu8 count2.bin
echo "--- replay stderr:"; grep -E '^socket:|^halt:|^steps:' replay.err
cmp count.bin count2.bin && echo "replay stdout identical to live"
