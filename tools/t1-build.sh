#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# t1-build.sh — THE EVERYDAY BUILD (owner order): t1_image's arguments, Stage 1's compiler,
# natively where a native Stage 1 exists.
#
#   tools/t1-build.sh [--spec-root D] [--compiler D] [--load F.t1]... --entry MODULE ROUTINE \
#                     -o OUT.elf [F.t1 ...]
#
# THE SOURCES ARE EXACTLY WHAT t1_image COMPILES: its positionals (`--load` only feeds
# t1_image's interpreter). With no positional, the `--load` files are the sources. A module
# such as अष्टक that a source imports but no source declares is NOT packed: the Stage 1
# rung resolves it as t1_image does, and packing it changes the image (measured on the
# protein-kernel program at 3e245f49: 22,976 octets alone, equal to t1_image's; 26,270 with अष्टक).
# `--spec-root` and `--compiler` are accepted and IGNORED (a note says so): the compiler is
# Stage 1, built from the checkout's crates/sadhana-t1/src.
#
# THE ROUTE, first that can run (printed on stderr):
#   native    the host's translation of Stage 1 (x86-64 / aarch64 Linux, x86-64 / arm64
#             macOS) by tools/native-spike/t1/anuvada.t1;
#   yantra    Stage 1 under yantra-run;
#   t1_image  t1_image itself (no Stage 1 could be had).
# Stage 1 and its translations are CACHED as tools/t1-native-lib.sh describes (keys name the
# sources, never a host binary: a cache seeded from another host is still never stale).
# A missing Stage 1 (a new compiler key) is made NATIVELY from a trusted earlier image: P(corpus) = A,
# A(corpus) = B, B(corpus) = C, B == C, in seconds; t1_image's cold 25-70 minutes (2,423 s measured on
# a Linux x86_64 host) only with T1_BUILD_COLD=1. A missing translation, seconds.
#
# Stage 1 runs with YANTRA_INPUT = the packed sources and YANTRA_INPUT_ENTRY = "MODULE ROUTINE";
# its sink's ELF is cut out exactly as tools/fixpoint.sh does; any status but 1200 is refused.
#
# env: SAS              the Sassembly checkout whose compiler is used (default: this one)
#      SAS_BIN          where yantra-run and t1_image are (default: $SAS/target/release)
#      T1_BUILD_ROUTE   native | yantra | t1_image: force a route
#      T1_BUILD_CACHED_ONLY=1  never build Stage 1 or a translation; exit 77 if absent
#      T1_BUILD_COLD=1  allow the COLD builds (t1_image for Stage 1, or for the program). Without it a missing Stage 1 is made
#                       natively from a trusted earlier image (t1-native-lib.sh stage1_native: seconds), or exit 77
#      T1_STAGE1_SEED   that trusted earlier image, by hand (default: the recorded image, else a pinned ancestor's)
set -uo pipefail
# the REAL directory of this script (follow symlinks; readlink -f is absent on old macOS): a symlinked
# or copied script must find its lib or REFUSE (exit 2), never carry on without the switches
self=$0
while [ -L "$self" ]; do
    d=$(cd -P "$(dirname "$self")" && pwd -P) || exit 2
    l=$(readlink "$self") || exit 2
    case $l in /*) self=$l ;; *) self=$d/$l ;; esac
done
HERE=$(cd -P "$(dirname "$self")" && pwd -P) || { echo "$(basename "$0"): cannot resolve its own directory" >&2; exit 2; }
ROOT=$(dirname "$HERE")
SAS=${SAS:-$ROOT}
BIN=${SAS_BIN:-$SAS/target/release}
. "$HERE/t1-native-lib.sh" || { echo "$(basename "$0"): REFUSED — cannot load $HERE/t1-native-lib.sh" >&2; exit 2; }
type validate_switches >/dev/null 2>&1 || { echo "$(basename "$0"): REFUSED — $HERE/t1-native-lib.sh defines no validate_switches" >&2; exit 2; }
validate_switches   # every switch, up front, in THIS shell: a bad value is exit 64 whatever branch or subshell would read it

say() { echo "t1-build: $*" >&2; }
die() { echo "t1-build: REFUSED — $*" >&2; exit 1; }
cannot() { echo "CANNOT RUN: $*" >&2; exit 77; }

loads=(); srcs=(); out=; mod=; rtn=; ignored=
while [ $# -gt 0 ]; do
    case $1 in
        --spec-root|--compiler) ignored="$ignored $1"; shift 2 ;;
        --load) loads+=("$2"); shift 2 ;;
        --entry) mod=$2; rtn=$3; shift 3 ;;
        -o) out=$2; shift 2 ;;
        --no-predict|--accept-divergence) die "$1 is a t1_image flag this wrapper does not take (it predicts nothing)" ;;
        -*) die "unknown flag $1" ;;
        *) srcs+=("$1"); shift ;;
    esac
done
[ -n "$ignored" ] && say "note: ignored${ignored} (the compiler is Stage 1 from $SAS/crates/sadhana-t1/src)"
[ -n "$out" ] || die "no -o OUT.elf"
[ -n "$mod" ] && [ -n "$rtn" ] || die "no --entry MODULE ROUTINE"
[ ${#srcs[@]} -gt 0 ] || srcs=("${loads[@]}")
[ ${#srcs[@]} -gt 0 ] || die "no sources (positional or --load)"
for f in "${srcs[@]}"; do [ -r "$f" ] || die "cannot read $f"; done

# ---------------------------------------------------------------- the t1_image route
via_t1_image() {
    say "route t1_image ($BIN/t1_image)"
    [ -x "$BIN/t1_image" ] || die "no t1_image at $BIN"
    local args=(--spec-root "$SAS/spec" --compiler "$SAS/crates/sadhana-t1/src")
    for f in "${loads[@]}"; do args+=(--load "$f"); done
    rm -f "$out"
    "$BIN/t1_image" "${args[@]}" --entry "$mod" "$rtn" -o "$out" "${srcs[@]}" > "$out.build.log" 2>&1
    local rc=$?
    [ "$rc" -eq 0 ] && [ -s "$out" ] || { tail -20 "$out.build.log" >&2; die "t1_image exited $rc (log $out.build.log)"; }
    say "wrote $out ($(wc -c < "$out" | tr -d ' ') octets) by t1_image"
    exit 0
}
[ "${T1_BUILD_ROUTE:-}" = t1_image ] && via_t1_image

# ---------------------------------------------------------------- Stage 1, keyed by its sources
stage1() {
    S1DIR=$CACHE/stage1/$(compiler_key)
    S1=$S1DIR/stage1.elf
    stage1_ok "$S1" && return 0       # sealed AND, when the record names this key's image, that image (review F1)
    local n; n=$(stage1_native_path) || n=
    if [ -n "$n" ] && stage1_ok "$n"; then S1=$n; return 0; fi
    switch T1_BUILD_CACHED_ONLY && cannot "no cached Stage 1 for this compiler ($S1)"
    if [ -n "$n" ] && stage1_native "$n"; then   # from a trusted earlier image, natively, in seconds (t1-native-lib.sh)
        stage1_ok "$n" || die "the natively made Stage 1 $n is not the image tools/compiler-image.sha256 records for this key"
        S1=$n; return 0
    fi
    switch T1_BUILD_COLD || { say "no cached Stage 1 and no trusted earlier image to make it from natively (T1_STAGE1_SEED, the image tools/compiler-image.sha256 records, a pinned ancestor); T1_BUILD_COLD=1 builds it with t1_image (25-70 min)"; return 1; }
    [ -x "$BIN/t1_image" ] || { say "no cached Stage 1 and no t1_image to build one (seed the cache: see t1-native-lib.sh)"; return 1; }
    say "building Stage 1 once for this compiler (t1_image, 25-70 min) -> $S1"
    mkdir -p "$S1DIR" || return 1
    rm -f "$S1" "$S1.sha256"          # an entry stage1_ok refused is not to be kept
    local tmp; tmp=$(tmpname "$S1")   # one temp per process: concurrent cold builds never share a file
    ( cd "$SAS" && "$BIN/t1_image" --compiler crates/sadhana-t1/src \
        --entry शृङ्खला स्वपरीक्षास्वप्रतिबिम्बम् -o "$tmp" crates/sadhana-t1/src/*.t1 ) > "$S1DIR/stage1.log.$$" 2>&1
    if [ -s "$tmp" ]; then rm -f "$S1DIR/stage1.log.$$"; publish "$tmp" "$S1" && { stage1_ok "$S1" || die "the t1_image Stage 1 is not the image tools/compiler-image.sha256 records for this key"; }
    else mv -f "$S1DIR/stage1.log.$$" "$S1DIR/stage1.log"; rm -f "$tmp"; say "Stage 1 build failed; see $S1DIR/stage1.log"; return 1; fi
}

# ---------------------------------------------------------------- the host's native Stage 1
native() {
    read -r NATIVE_TGT _ <<< "$(host_target)"
    [ -n "$NATIVE_TGT" ] || return 1
    NATIVE=$(translate "$S1" stage1.elf) && return 0
    switch T1_BUILD_CACHED_ONLY && cannot "no cached $NATIVE_TGT translation of Stage 1"
    return 1
}

route=${T1_BUILD_ROUTE:-auto}
case $route in auto|native|yantra) ;; *) die "T1_BUILD_ROUTE=$route (native | yantra | t1_image)" ;; esac
if ! stage1; then
    [ "$route" = auto ] || die "route $route needs Stage 1, which could not be had"
    switch T1_BUILD_COLD || cannot "no Stage 1 could be had, and the t1_image route is a cold build: T1_BUILD_COLD=1 allows it (or T1_BUILD_ROUTE=t1_image)"
    say "no Stage 1 could be had; falling back"
    via_t1_image
fi
if [ "$route" != yantra ]; then
    if native; then route=native
    elif [ "$route" = native ]; then die "route native: no native Stage 1 for this host"
    else route=yantra; fi
fi
if [ "$route" = yantra ] && ! [ -x "$BIN/yantra-run" ]; then
    [ "${T1_BUILD_ROUTE:-auto}" = auto ] || die "route yantra: no yantra-run at $BIN"
    via_t1_image
fi

# ---------------------------------------------------------------- pack, run, cut out the ELF
work=$(mktemp -d "${TMPDIR:-/tmp}/t1-build.XXXXXX") || die "no temporary directory"
trap 'rm -rf "$work"' EXIT
python3 "$ROOT/tools/pack-corpus.py" "$work/input.blob" "${srcs[@]}" > "$work/pack.log" || die "packing failed"
if [ "$route" = native ]; then
    say "route native ($NATIVE_TGT: $NATIVE; Stage 1 $S1)"
    env -u YANTRA_INPUT_NAME YANTRA_INPUT="$work/input.blob" YANTRA_INPUT_ENTRY="$mod $rtn" \
        YANTRA_RAM=2684354560 "$NATIVE" > "$work/sink" 2> "$work/log"
else
    say "route yantra ($BIN/yantra-run $S1)"
    env -u YANTRA_INPUT_NAME YANTRA_INPUT="$work/input.blob" YANTRA_INPUT_ENTRY="$mod $rtn" \
        YANTRA_RAM=2684354560 YANTRA_STEPS=200000000000 "$BIN/yantra-run" "$S1" > "$work/sink" 2> "$work/log"
fi
status=$(sed -n 's/.*status: Some(\([0-9][0-9]*\)).*/\1/p' "$work/log" | tail -1)
if [ "${status:-}" != 1200 ]; then
    tail -20 "$work/log" >&2
    head -c 2000 "$work/sink" >&2
    die "Stage 1 halted with status ${status:-NONE}, not 1200 (BUILT) — no image written"
fi
python3 - "$work/sink" "$out" <<'PY' || die "the sink holds no ELF"
import sys
sink = open(sys.argv[1], "rb").read()
off = sink.find(b"\x7fELF")
if off < 0:
    sys.exit(1)
open(sys.argv[2], "wb").write(sink[off:len(sink) - 1])
PY
chmod +x "$out"
say "wrote $out ($(wc -c < "$out" | tr -d ' ') octets) by Stage 1, route $route"
