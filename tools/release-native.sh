#!/usr/bin/env bash
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# release-native.sh — package a release's NATIVE build for one target (owner ruling: releases after
# v1.0.2 ship the native Stage 1 + tools/t1-build.sh/t1-run.sh in place of the Rust yantra-run).
#
#   tools/release-native.sh <tag> <target> [OUTDIR]
#
# <target>: x86_64-linux | aarch64-linux | aarch64-macos | x86_64-macos (windows: refused until the
# translator lands it). RUNS ON THE TARGET'S OWN HOST (the translation is made by tools/t1-native-lib.sh,
# which translates for the host it runs on). Run it in a checkout of <tag> (or set SAS).
#   env: T1_STAGE1=path   the release's stage1.elf (default: the cached Stage 1 for this checkout's
#                         compiler key, as tools/t1-build.sh keeps it)
#        RELEASE_SUMS=...      REQUIRED: the release's published SHA256SUMS (file or URL); stage1.elf must match
#        RELEASE_STAGE1_NAME   the Stage 1's file name in RELEASE_SUMS (default stage1.elf); for a published
#                              release it is sassembly-<tag>-stage1.elf, e.g. RELEASE_STAGE1_NAME=sassembly-v1.1.0-stage1.elf
#        SASSEMBLY_NATIVE_CACHE, SAS_BIN (as t1-build.sh: only needed if a translator must be built)
# Output: OUTDIR/sassembly-native-<tag>-<target>.tar.gz (+ .sha256, and SHA256SUMS lines inside the
# package and in OUTDIR/SHA256SUMS; the package carries LICENSE.md, NOTICE and COMMERCIAL-LICENSE.md), after a SMOKE from the unpacked package under a scrubbed
# environment with no repo on the path: namaste.t1 built with the default entry and with a
# non-default one (YANTRA_INPUT_ENTRY), run, exit status line. T0 .sas stays a Rust tool: the
# package is .t1-only and its README says so. SMOKE LOG: OUTDIR/smoke-<target>.log.
set -uo pipefail
HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(dirname "$HERE")
tag=${1:-}; target=${2:-}; outdir=${3:-$ROOT/dist}
[ -n "$tag" ] && [ -n "$target" ] || { sed -n '3,6p' "$0" >&2; exit 64; }
die() { echo "release-native: REFUSED - $*" >&2; exit 1; }
SAS=${SAS:-$ROOT}; BIN=${SAS_BIN:-$SAS/target/release}
. "$HERE/t1-native-lib.sh"
case $target in windows*|*-windows*) die "no Windows native target yet (the translator does not emit one)" ;; esac
read -r host _ <<< "$(host_target)"
[ "$host" = "$target" ] || die "this host is '${host:-none}', not $target: run it on the target's own host"
[ -r "$ROOT/LICENSE.md" ] || die "no LICENSE.md"
for f in NOTICE.in COMMERCIAL-LICENSE.md README-BIN.txt.in; do [ -r "$HERE/release-native/$f" ] || die "no tools/release-native/$f"; done

S1=${T1_STAGE1:-$CACHE/stage1/$(compiler_key)/stage1.elf}
[ -s "$S1" ] || die "no stage1.elf at $S1 (set T1_STAGE1 to the release's stage1.elf)"
# the Stage 1 must equal the release's PUBLISHED sum (RELEASE_SUMS = a SHA256SUMS file or http(s) URL;
# line for RELEASE_STAGE1_NAME, default stage1.elf): mismatch or no sum is a refusal, a cache is not proof
[ -n "${RELEASE_SUMS:-}" ] || die "RELEASE_SUMS (the release's published SHA256SUMS, file or URL) is required"
sumsf=$(mktemp "${TMPDIR:-/tmp}/relsums.XXXXXX")
case $RELEASE_SUMS in
    http://*|https://*) curl -fsSL "$RELEASE_SUMS" -o "$sumsf" || die "cannot fetch $RELEASE_SUMS" ;;
    *) cp "$RELEASE_SUMS" "$sumsf" 2>/dev/null || die "cannot read $RELEASE_SUMS" ;;
esac
want=$(awk -v n="${RELEASE_STAGE1_NAME:-stage1.elf}" '{f=$2; sub(/^\*/,"",f); sub(/^.*\//,"",f); if (f==n) {print $1; exit}}' "$sumsf"); rm -f "$sumsf"
[ -n "$want" ] || die "no ${RELEASE_STAGE1_NAME:-stage1.elf} line in $RELEASE_SUMS: no published sum, no package"
[ "$(sha "$S1")" = "$want" ] || die "stage1.elf sha256 $(sha "$S1") != published $want"
echo "release-native: stage1.elf $want matches the published sum"
stamp=$(mktemp "${TMPDIR:-/tmp}/relstamp.XXXXXX")
tlog() { [ -z "${T1_BUILD_CACHED_ONLY:-}" ] || echo "release-native: T1_BUILD_CACHED_ONLY is set (even '=0' counts as set): nothing is translated, only cached translations are used - unset it" >&2
         echo "release-native: translator refusal; translate.log tails (kept):" >&2
         find "$CACHE" -name translate.log -newer "$stamp" -print 2>/dev/null | while read -r l; do echo "--- $l" >&2; tail -8 "$l" >&2; done; }
nat1=$(translate "$S1" stage1.elf) || { tlog; die "the translator refused $S1"; }
natT=$(native_translator) || { tlog; die "no native translator could be made (see t1-native-lib.sh)"; }
rm -f "$stamp"

pkg=sassembly-native-$tag-$target
stage=$(mktemp -d "${TMPDIR:-/tmp}/relnat.XXXXXX"); trap 'rm -rf "$stage"' EXIT
mkdir -p "$outdir" && outdir=$(cd "$outdir" && pwd)   # absolute: the tar and smoke steps cd
d=$stage/$pkg; mkdir -p "$d/bin" "$d/lib" "$d/example"
cp "$HERE/release-native/sassembly-build" "$HERE/release-native/sassembly-run" "$d/bin/"
cp "$nat1" "$d/lib/stage1.native"; cp "$natT" "$d/lib/translator.native"; chmod +x "$d"/bin/* "$d"/lib/*
cp "$HERE/release-native/namaste.t1" "$d/example/"; cp "$ROOT/LICENSE.md" "$HERE/release-native/COMMERCIAL-LICENSE.md" "$d/"
sed -e "s/@TAG@/$tag/g" -e "s/@TARGET@/$target/g" "$HERE/release-native/NOTICE.in" > "$d/NOTICE"
note="Built $(date -u +%Y-%m-%dT%H:%MZ) on $(uname -sm) from stage1.elf sha256 $(sha "$S1")."
sed -e "s/@TAG@/$tag/g" -e "s/@TARGET@/$target/g" -e "s|@NOTE@|$note|" "$HERE/release-native/README-BIN.txt.in" > "$d/README-BIN.txt"
echo "$tag $target" > "$d/lib/VERSION"
( cd "$d" && for f in $(find . -type f ! -name SHA256SUMS | sort); do echo "$(sha "$f")  $f"; done > SHA256SUMS )

# ---------------------------------------------------------------- smoke: package only, scrubbed env
log=$outdir/smoke-$target.log; : > "$log"
smoke() {
    local s=$stage/smoke; rm -rf "$s"; mkdir -p "$s/home" "$s/tmp"
    tar -C "$s" -xzf "$outdir/$pkg.tar.gz" || return 1
    local p=$s/$pkg run=(env -i HOME="$s/home" TMPDIR="$s/tmp" PATH=/usr/bin:/bin:/usr/sbin:/sbin) rc ok=0
    echo "== smoke $pkg on $(uname -sm), unpacked at $p, env -i, no repo, PATH=/usr/bin:/bin"
    ( cd "$p" && shasum -a 256 -c SHA256SUMS 2>/dev/null || sha256sum -c SHA256SUMS ) | tail -3
    echo "-- build, default entry (first module, मुख्यम्)"
    "${run[@]}" "$p/bin/sassembly-build" -o "$s/n1.elf" "$p/example/namaste.t1" || ok=1
    echo "-- run n1"; out=$("${run[@]}" "$p/bin/sassembly-run" "$s/n1.elf" 2> "$s/e1"); rc=$?
    cat "$s/e1"; echo "stdout: $out"; echo "status line: exit $rc"
    [ "$out" = namaste ] && [ $rc -eq 0 ] || { echo "SMOKE FAIL: want 'namaste' / exit 0"; ok=1; }
    echo "-- build, non-default entry (YANTRA_INPUT_ENTRY = 'नमस्ते द्वितीयम्')"
    "${run[@]}" "$p/bin/sassembly-build" --entry नमस्ते द्वितीयम् -o "$s/n2.elf" "$p/example/namaste.t1" || ok=1
    echo "-- run n2"; out=$("${run[@]}" "$p/bin/sassembly-run" "$s/n2.elf" 2> "$s/e2"); rc=$?
    cat "$s/e2"; echo "stdout: $out"; echo "status line: exit $rc"
    [ "$out" = _ ] && [ $rc -eq 1 ] && grep -q 'status: Some(7)' "$s/e2" || { echo "SMOKE FAIL: want '_' / halt status 7 / exit 1"; ok=1; }
    echo "-- re-run n1 (translation cached)"; "${run[@]}" "$p/bin/sassembly-run" "$s/n1.elf" 2>&1 | head -3
    echo "-- refusal: yantra-only feature"; "${run[@]}" env YANTRA_STEPS=5 "$p/bin/sassembly-run" "$s/n1.elf"; echo "exit $?"
    [ $ok = 0 ] && echo "SMOKE PASS $target" || echo "SMOKE FAILED $target"
    return $ok
}
( cd "$stage" && tar -czf "$outdir/$pkg.tar.gz" "$pkg" ) || die "tar failed"
smoke 2>&1 | sed -e "s|$stage|<tmp>|g" -e "s|$outdir|<out>|g" -e "s|$HOME|~|g" -e "s|${TMPDIR:-/tmp}|<tmp>|g" | tee -a "$log"; [ "${PIPESTATUS[0]}" -eq 0 ] || die "smoke failed (see $log)"
( cd "$outdir" && echo "$(sha "$pkg.tar.gz")  $pkg.tar.gz" | tee "$pkg.tar.gz.sha256" >> SHA256SUMS )
echo "release-native: $outdir/$pkg.tar.gz ready; sums:"; cat "$d/SHA256SUMS" | sed 's/^/  /'; tail -1 "$outdir/SHA256SUMS"
