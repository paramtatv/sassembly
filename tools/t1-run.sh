#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# t1-run.sh — A DROP-IN FOR yantra-run that runs the image natively where it can.
#
#   tools/t1-run.sh [yantra-run flags] IMAGE.elf [args...]
#
# Same environment contract as yantra-run (YANTRA_INPUT, _NAME, _ENTRY, _TRACE, YANTRA_RAM):
# the native runtime implements it and refuses as yantra-run refuses. IMAGE is translated
# for this host (tools/t1-native-lib.sh; cached by the image's sha256, the translator's
# version and ARG0 = IMAGE as given, which is what yantra-run hands the program as argument
# 0), run, and its exit status returned with yantra-run's meaning (0 = a finisher with status
# 0, 1 = any other halt or refusal; the runtime's own 2 for an unreadable input / unmappable
# RAM / missing tag is reported as 1, as yantra-run reports those).
#
# IT RUNS yantra-run INSTEAD, and says why on stderr, when:
#   - a yantra-run flag is given (--events, --record-events, --net-*, --listen, --smp,
#     --source-stamp, --version: none has a native counterpart); --files DIR runs natively
#     (YANTRA_FILES=DIR), unless DIR is not a directory;
#   - the environment asks for a yantra-only feature: YANTRA_STEPS (an exact step limit;
#     the native code counts no steps, so this does NOT use attestation counting),
#     YANTRA_VERDICT, YANTRA_SCANOUT, YANTRA_VIRTIO_DEFER, YANTRA_WATERMARK;
#   - this host has no native target, or the translator refuses the image;
#   - the native run reaches something the translator refuses AT RUN TIME (an instruction
#     it does not lower, e.g. an ecall for threads or a CSR read; an MMIO load such as
#     virtio/gpu, or a store to a device other than the UART and FINISHER — both halt
#     natively as BadAccess; a jump outside the code; a store into the code). Its output is held back
#     until it finishes, so such a run is discarded whole and yantra-run's is the only one
#     seen (stdin, if the program reads it, is not replayed).
# YANTRA_RECORD_EVENTS=<log> (the native clock's recorder) is honoured on both routes: natively it
# writes the log of the clock waits, on the fallback it becomes `yantra-run --record-events <log>`.
# env: SAS, SAS_BIN (as tools/t1-build.sh), SASSEMBLY_NATIVE_CACHE, T1_RUN_ROUTE=yantra (force yantra-run)
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

say() { echo "t1-run: $*" >&2; }
yantra() {
    # T1_RUN_REQUIRE_NATIVE=1: a caller that must prove a NATIVE run (tools/selfcheck, land) gets a
    # refusal, never a silent yantra-run whose halt line would look the same
    if switch T1_RUN_REQUIRE_NATIVE; then say "REFUSED — native required (T1_RUN_REQUIRE_NATIVE) but: $1"; exit 3; fi
    say "route yantra-run — $1"
    # YANTRA_RECORD_EVENTS=<log> is the native twin of `yantra-run --record-events <log>`; yantra-run
    # itself ignores the variable, so the fallback spells it as the flag (a drop-in, and the log is yantra's)
    if [ -n "${YANTRA_RECORD_EVENTS:-}" ]; then
        # only for an image that declares the event interface: yantra-run refuses --record-events on
        # an image with no SASEVENT tag (exit 1) where native runs it (review N1); the tag's 8 octets
        # read "SASEVENT" in the image
        case ${orig[0]} in -*) ;; *)
            if LC_ALL=C grep -qa SASEVENT "${orig[0]}" 2>/dev/null; then
                orig=(--record-events "$YANTRA_RECORD_EVENTS" "${orig[@]}")
            fi ;;
        esac
    fi
    [ -x "$BIN/yantra-run" ] || { say "REFUSED — no yantra-run at $BIN to fall back to"; exit 1; }
    exec "$BIN/yantra-run" "${orig[@]}"
}
orig=("$@")
[ $# -gt 0 ] || { say "usage: t1-run.sh [yantra-run flags] IMAGE.elf [args...]"; exit 64; }
[ "${T1_RUN_ROUTE:-}" = yantra ] && yantra "T1_RUN_ROUTE=yantra"
# --files DIR (first, as yantra-run takes it): the native runtime's file window takes its root from
# YANTRA_FILES (7a983fe4). A DIR that is not a directory stays yantra-run's, which refuses it at load
# where the native runtime would refuse each request.
if [ "$1" = --files ] && [ $# -ge 3 ]; then
    [ -d "$2" ] || yantra "--files $2 is not a directory (yantra-run refuses it at load)"
    export YANTRA_FILES=$2
    shift 2
fi
case $1 in -*) yantra "the yantra-run flag $1 has no native counterpart" ;; esac
for v in YANTRA_STEPS YANTRA_VERDICT YANTRA_SCANOUT YANTRA_VIRTIO_DEFER YANTRA_WATERMARK; do
    if [ -n "${!v+x}" ]; then
        case $v in
            YANTRA_STEPS) yantra "YANTRA_STEPS asks for an exact step limit; native code counts no steps" ;;
            *) yantra "$v is a yantra-only feature" ;;
        esac
    fi
done
img=$1; shift
[ -r "$img" ] || yantra "$img is not readable here (yantra-run reports it)"
[ -n "$(host_target)" ] || yantra "no native target for $(uname -s) $(uname -m)"
nat=$(translate "$img" "$img") || yantra "the translator refused $img"

work=$(mktemp -d "${TMPDIR:-/tmp}/t1-run.XXXXXX") || yantra "no temporary directory"
trap 'rm -rf "$work"' EXIT
say "route native ($nat)"
# a native recording goes to a temporary and is published only by a run that completes natively:
# a run that faults to yantra-run leaves no partial native log (yantra-run writes its own)
if [ -n "${YANTRA_RECORD_EVENTS:-}" ]; then
    YANTRA_RECORD_EVENTS="$work/events.log" "$nat" "$@" > "$work/out" 2> "$work/err"
else
    "$nat" "$@" > "$work/out" 2> "$work/err"
fi
rc=$?
# a native-only halt reruns under yantra-run: "native: …" and the runtime's own "halt: BadAccess (…)".
# yantra's own form "halt: BadAccess { pc, addr }" (a UART or FINISHER load) is native's answer too: kept
if grep -aq '^native: \(not in milestone 1\|jump to\|refused a store\)\|^halt: BadAccess (' "$work/err"; then
    why=$(grep -a '^native: \|halt: BadAccess (' "$work/err" | head -1)
    say "the native run reached what the translator refuses at run time: $why"
    rm -rf "$work"
    trap - EXIT
    yantra "rerun whole under yantra-run"
fi
[ -n "${YANTRA_RECORD_EVENTS:-}" ] && [ -f "$work/events.log" ] && cp "$work/events.log" "$YANTRA_RECORD_EVENTS"
cat "$work/out"
cat "$work/err" >&2
[ "$rc" -eq 2 ] && rc=1
exit "$rc"
