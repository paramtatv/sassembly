#!/bin/sh
# tools/native-spike/fj-check.sh — FORK-JOIN threads in the oracle (FORKJOIN.md): every image given,
# translated with ANUVADA_FORKJOIN=1, run with SASSEMBLY_NATIVE_THREADS = 0 and each N in $FJ_NS, must
# give EXACTLY the stdout, stderr and exit status of the image translated as before (no ANUVADA_FORKJOIN,
# run serially) -- or both runs must be ones tools/t1-run.sh discards and reruns under yantra-run --
# and that serial run's stdout, exit status and halt line must equal yantra-run's.
#   fj-check.sh IMAGE.elf...      env: FJ_NS ("1 2 8 20 cpus"; cpus = unset, the default), FJ_MIN (the
#                                 small-region threshold K, default 0 = off, so every region threads), YANTRA (yantra-run; skipped when unset),
#                                 FJ_REPEAT (runs per N, default 1), FJ_TIMEOUT (s per run, 20), FJ_WORK (a temp dir)
# Exit 0 only if every comparison held. Run it on a Linux host (x86_64 or aarch64), never here on the Mac.
set -u
here=$(cd "$(dirname "$0")" && pwd)
ns=${FJ_NS:-"1 2 8 20 cpus"}; rep=${FJ_REPEAT:-1}; tmo=${FJ_TIMEOUT:-20}
case $(uname -m) in x86_64) tgt=x86_64-linux ;; aarch64) tgt=aarch64-linux ;; *) echo "fj-check: no Linux target here" >&2; exit 2 ;; esac
if [ -n "${FJ_WORK:-}" ]; then work=$FJ_WORK; else work=$(mktemp -d "${TMPDIR:-/tmp}/fj-check.XXXXXX"); trap 'rm -rf "$work"' EXIT; trap 'exit 143' INT TERM; fi
mkdir -p "$work"
fail=0; runs=0
# the small-region threshold is OFF here (every region threads) unless FJ_MIN says otherwise; N "cpus" = unset
export SASSEMBLY_NATIVE_FJ_MIN=${FJ_MIN:-0}
run() {   # run OUTPREFIX NATIVE [N]
    if [ "${3:-}" = cpus ]; then
        env -u SASSEMBLY_NATIVE_THREADS timeout "$tmo" "$2" > "$1.out" 2> "$1.err"; echo $? > "$1.rc"
    elif [ -n "${3:-}" ]; then
        SASSEMBLY_NATIVE_THREADS=$3 timeout "$tmo" "$2" > "$1.out" 2> "$1.err"; echo $? > "$1.rc"
    else
        env -u SASSEMBLY_NATIVE_THREADS timeout "$tmo" "$2" > "$1.out" 2> "$1.err"; echo $? > "$1.rc"
    fi
}
fb='^native: \(not in milestone 1\|jump to\|refused a store\)\|^halt: BadAccess'   # tools/t1-run.sh's rerun-under-yantra test
exp=$here/fj/expected.tsv   # name -> the status every run must report (8f's m1: 882; a "first to halt wins" runtime says 896)
want() {  # want NAME PREFIX: the halt status in PREFIX.err equals the pinned one, if NAME is pinned
    w=$(awk -v n="$1" '$1 == n {print $2}' "$exp" 2>/dev/null)
    [ -z "$w" ] && return 0
    grep -aq "status: Some($w) }" "$2.err" && return 0
    echo "fj-check: WRONG STATUS $1 ($2): $(grep -a '^halt' "$2.err" | head -1), pinned $w"; fail=1; bad=1; return 1
}
same() {  # same A B WHAT — or BOTH runs are ones tools/t1-run.sh discards and reruns under yantra-run
    if grep -aq "$fb" "$1.err" && grep -aq "$fb" "$2.err"; then return 0; fi
    for x in out err rc; do
        cmp -s "$1.$x" "$2.$x" || { echo "fj-check: DIVERGED $3 ($x): $(head -c 300 "$2.$x" | tr '\n' '|')  vs serial  $(head -c 300 "$1.$x" | tr '\n' '|')"; fail=1; bad=1; return 1; }
    done
}
for img in "$@"; do
    n=$(basename "$img" .elf); bad=0
    python3 "$here/anuvada.py" --target $tgt "$img" "$work/$n.ser" > /dev/null 2>&1 || { echo "fj-check: $n: no serial translation"; fail=1; continue; }
    ANUVADA_FORKJOIN=1 python3 "$here/anuvada.py" --target $tgt "$img" "$work/$n.fj" > /dev/null 2>&1 || { echo "fj-check: $n: no fork-join translation"; fail=1; continue; }
    run "$work/$n.S" "$work/$n.ser"
    want "$n" "$work/$n.S"
    if [ -n "${YANTRA:-}" ] && ! grep -aq "$fb" "$work/$n.S.err"; then   # (a serial run t1-run reruns IS yantra's)
        YANTRA_STEPS=1000000000000 timeout "$tmo" "$YANTRA" "$img" > "$work/$n.Y.out" 2> "$work/$n.Y.err"; yrc=$?
        [ "$yrc" -gt 1 ] && yrc=1
        cmp -s "$work/$n.Y.out" "$work/$n.S.out" && [ "$yrc" = "$(cat "$work/$n.S.rc")" ] &&
            [ "$(grep -a '^halt:' "$work/$n.Y.err")" = "$(grep -a '^halt:' "$work/$n.S.err")" ] ||
            { echo "fj-check: $n: the serial native run is not yantra-run's"; fail=1; bad=1; }
    fi
    for N in 0 $ns; do
        i=0
        while [ $i -lt "$rep" ]; do
            run "$work/$n.N$N" "$work/$n.fj" "$N"; runs=$((runs + 1))
            same "$work/$n.S" "$work/$n.N$N" "$n N=$N run $i" || break
            want "$n" "$work/$n.N$N" || break
            i=$((i + 1))
        done
    done
    [ "$bad" = 0 ] && echo "fj-check: $n ok ($(grep -a '^halt:\|^native:' "$work/$n.S.err" | head -1 | cut -c1-90))"
done
echo "fj-check: $runs threaded runs, $([ $fail = 0 ] && echo ALL EXACT || echo FAILED)"
exit $fail
