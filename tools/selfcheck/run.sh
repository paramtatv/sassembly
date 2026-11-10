#!/bin/sh
# tools/selfcheck/run.sh — the everyday Sassembly sanity check.
#
#   run.sh                 run every program's image in parallel, compare EXACT status + stdout with expected.tsv
#   run.sh --verify-pins   the NIGHTLY path: under yantra, check expected.tsv against yantra AND against oracle.tsv
#   run.sh --capture       print candidate expected.tsv lines from a yantra run (review before committing)
#   run.sh name...         restrict to those programs (an unknown or empty name FAILS)
# (every run prints 'selfcheck: digest <sha256>' over the actual results, for cross-ISA comparison)
#
# FAILS CLOSED. Exit 0 only if at least one program ran and every one matched. It exits non-zero, naming the program, on: a
# mismatch of status or stdout; a runner that exited with a code inconsistent with the halt it printed (segfault, exit 9, exit 0
# after a non-zero halt: yantra-run and t1-run.sh exit 0 iff the status is 0, else 1); not exactly one `halt:` line; a timeout
# (only when SC_TIMEOUT s per program is set > 0, default none; the group is killed, only that group); an image older than its source; a missing image;
# a program with no pin or a pin with no program; a duplicate, blank or malformed pin; zero programs; an unknown name.
#
# Env:
#   SC_DIR      where the programs, expected.tsv, oracle.tsv and root/ are (default: this script's directory). The landing runs the
#               TRUSTED copy of this script (from origin/main) with SC_DIR = the landed tree's tools/selfcheck.
#   SC_RUNNER   native | yantra | auto (default). auto = native when tools/t1-run.sh exists, else yantra.
#   SC_ELFS     images built by build.sh (default $SC_DIR/out)
#   SC_YANTRA   the yantra-run binary (default $CARGO_TARGET_DIR/release/yantra-run, else target/release)
#   SC_YANTRA_PROGS  programs that run on the pinned yantra-run even in native mode (default: none; float_ieee and file_* run natively,
#               file_* with --files, the native file window);
#               every other native program runs with an EMPTY bin dir, so t1-run.sh cannot fall back to yantra (it refuses)
#   SC_FILES_PROGS  programs that run NATIVELY with `--files <their root copy>` (default: none; t1tests passes a group's files.txt)
#   SC_EXPECTED the pin file (default $SC_DIR/expected.tsv)     SC_SKIP "glob ..." programs left out of the run and the digest
#   SC_REQUIRE  a file of program names that MUST be pinned and present (the landing passes the base's list)
#   SC_T1RUN    the native runner (default tools/t1-run.sh)     SC_TIMEOUT  seconds per program (unset/0 = none)  SC_JOBS  max programs at once (cores)
#
# RUNNER CONTRACTS. yantra: `yantra-run [--files DIR] IMAGE`; native: `t1-run.sh IMAGE` (the native runtime prints the same
# `halt: Finisher { value: V, status: Some(S) }` line on stderr and t1-run.sh relays it). stdout = the program's output.
set -u
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
dir=${SC_DIR:-$here}
mode=run; names=; nargs=0
for a in "$@"; do
  case $a in
    --verify-pins) mode=verify ;;
    --capture) mode=capture ;;
    -h|--help) sed -n '2,32p' "$0"; exit 0 ;;
    -*) echo "run.sh: unknown option $a" >&2; exit 2 ;;
    '') echo "selfcheck: FAILED, an empty program name was given" >&2; exit 1 ;;
    *) names="$names $a"; nargs=$((nargs + 1)) ;;
  esac
done

elfs=${SC_ELFS:-$dir/out}
tgt=${CARGO_TARGET_DIR:-$root/target}
yantra=${SC_YANTRA:-$tgt/release/yantra-run}
t1run=${SC_T1RUN:-$root/tools/t1-run.sh}
yprogs=${SC_YANTRA_PROGS:-}
fprogs=${SC_FILES_PROGS:-}
runner=${SC_RUNNER:-auto}
tmo=${SC_TIMEOUT:-0}   # 0 = no ceiling (with-timeout.pl arms no alarm); host load gave 34 false TIMEOUTs at 60 (L-NIGHTLY-SCTIMEOUT)
case $tmo in *[!0-9]*) echo "run.sh: SC_TIMEOUT must be a non-negative integer (got '$tmo')" >&2; exit 2 ;; esac
case ${SC_JOBS-} in '') ;; 0*|*[!0-9]*) echo "run.sh: SC_JOBS must be a positive integer (got '$SC_JOBS')" >&2; exit 2 ;; esac
wt="perl $here/../with-timeout.pl"
jobs=${SC_JOBS:-$(c=$(getconf _NPROCESSORS_ONLN 2>/dev/null || nproc 2>/dev/null || echo 4); l=$(uptime | sed "s/.*load averages*: *//;s/[ ,].*//;s/\..*//"); l=${l:-0}; j=$((c - l)); [ $j -ge $((c / 2)) ] || j=$((c / 2)); [ $j -ge 1 ] || j=1; echo $j)}   # default = idle cores, floor cores/2, never all at once on a loaded host (L-SCWARM, L-SCLOAD)
[ "$runner" = auto ] && { if [ -x "$t1run" ]; then runner=native; else runner=yantra; fi; }
[ "$mode" != run ] && runner=yantra
case $runner in
  yantra) YANTRA_STEPS=${YANTRA_STEPS:-1000000000000}; export YANTRA_STEPS
          [ -x "$yantra" ] || { echo "run.sh: no yantra-run at $yantra (set SC_YANTRA)" >&2; exit 2; } ;;
  native) [ -x "$t1run" ] || { echo "run.sh: no native runner at $t1run" >&2; exit 2; } ;;
  *) echo "run.sh: SC_RUNNER must be native, yantra or auto" >&2; exit 2 ;;
esac
[ -f "$here/../with-timeout.pl" ] || { echo "run.sh: no with-timeout.pl beside tools/" >&2; exit 2; }

if command -v sha256sum >/dev/null 2>&1; then sha() { sha256sum | cut -d' ' -f1; }; else sha() { shasum -a 256 | cut -d' ' -f1; }; fi
now() { perl -MTime::HiRes=time -e 'printf "%.3f\n", time'; }
bad=0
mismatch() { echo "MISMATCH $1: $2" >&2; bad=$((bad + 1)); }

tmp=$(mktemp -d "${TMPDIR:-/tmp}/selfcheck.XXXXXX")
trap 'rm -rf "$tmp"' EXIT INT TERM

exp=${SC_EXPECTED:-$dir/expected.tsv}
[ -f "$exp" ] || { echo "run.sh: no $exp" >&2; exit 2; }
# ---- the pin file: no blank, malformed or duplicate row ----
awk -F'\t' '
  /^#/ { next }
  NF < 4 || $1 == "" || $2 == "" || $3 == "" || length($4) != 64 || $4 ~ /[^0-9a-f]/ { printf "MISMATCH expected.tsv: line %d is blank or malformed\n", NR > "/dev/stderr"; bad = 1; next }
  seen[$1]++ { printf "MISMATCH %s: has a DUPLICATE row in expected.tsv (line %d)\n", $1, NR > "/dev/stderr"; bad = 1 }
  END { exit bad }' "$exp" || bad=$((bad + 1))
pinned=$(awk -F'\t' '!/^#/ && NF >= 4 {print $1}' "$exp" | sort -u)
allnames=$(cd "$dir" && ls *.t1 2>/dev/null | sed 's/\.t1$//')
for n in $allnames; do echo "$pinned" | grep -qx "$n" || mismatch "$n" "has no line in expected.tsv"; done
for n in $pinned; do [ -f "$dir/$n.t1" ] || mismatch "$n" "is pinned in expected.tsv but has no $n.t1"; done
if [ -n "${SC_REQUIRE:-}" ]; then
  [ -s "$SC_REQUIRE" ] || mismatch "SC_REQUIRE" "the required program list $SC_REQUIRE is empty or missing"
  for n in $(cat "$SC_REQUIRE" 2>/dev/null); do
    echo "$pinned" | grep -qx "$n" && [ -f "$dir/$n.t1" ] || mismatch "$n" "is in the committed program list but is not pinned or has no source"
  done
fi

if [ -z "$names" ]; then names=$allnames; else
  for n in $names; do echo "$pinned" | grep -qx "$n" && [ -f "$dir/$n.t1" ] || mismatch "$n" "is not a program (unknown name)"; done
fi
if [ -n "${SC_SKIP:-}" ]; then
  keep=; set -f
  for n in $names; do sk=; for g in $SC_SKIP; do case $n in $g) sk=1 ;; esac; done; [ -n "$sk" ] || keep="$keep $n"; done
  set +f; names=$keep
fi
count=$(echo $names | wc -w | tr -d ' ')
[ "$count" -gt 0 ] || { echo "selfcheck: FAILED, zero programs to run" >&2; exit 1; }

# L-SCWARM: warm the native translator cache serially before the fan-out: N cold oracle translations of the translator at once
# outrun SC_TIMEOUT (93/93 TIMEOUT, promdiff2). One run of the first program fills it; the result is ignored.
if [ "$runner" = native ]; then
  for first in $names; do case " $yprogs " in *" $first "*) ;; *) [ -s "$elfs/$first.elf" ] && { mkdir -p "$tmp/nobin"; SAS_BIN="$tmp/nobin" T1_RUN_REQUIRE_NATIVE=1 $wt "$tmo" "$t1run" "$elfs/$first.elf" >/dev/null 2>&1 </dev/null || :; break; } ;; esac; done
fi
t0=$(now)
for n in $names; do
  (
    cprt=1; [ "$runner" = native ] && case " $yprogs $fprogs " in *" $n "*) ;; *) case $n in file_*) ;; *) cprt= ;; esac ;; esac   # native: only a program that gets a file window needs the root copy
    [ -d "$dir/root" ] && [ -n "$cprt" ] && { mkdir -p "$tmp/$n.root"; cp -R "$dir/root/." "$tmp/$n.root/"; } || mkdir -p "$tmp/$n.root"
    elf=$elfs/$n.elf
    if [ ! -s "$elf" ]; then echo "no image $elf (run build.sh)" > "$tmp/$n.err"; echo 127 > "$tmp/$n.rc"; : > "$tmp/$n.out"; exit 0; fi
    if [ "$dir/$n.t1" -nt "$elf" ]; then echo "image $elf is OLDER than $n.t1 (stale; rebuild)" > "$tmp/$n.err"; echo 126 > "$tmp/$n.rc"; : > "$tmp/$n.out"; exit 0; fi
    case $runner in
      yantra) $wt "$tmo" "$yantra" --files "$tmp/$n.root" "$elf" > "$tmp/$n.out" 2> "$tmp/$n.err" ;;
      native) case " $yprogs " in
                *" $n "*) if [ -x "$yantra" ]; then $wt "$tmo" "$yantra" --files "$tmp/$n.root" "$elf" > "$tmp/$n.out" 2> "$tmp/$n.err"
                          else echo "no pinned yantra-run for $n (SC_YANTRA=$yantra)" > "$tmp/$n.err"; : > "$tmp/$n.out"; false; fi ;;
                *) mkdir -p "$tmp/nobin"
                   # FORK-JOIN ON THREADS (tools/native-spike/FORKJOIN.md): a region program runs threaded even when
                   # its region is small (the threshold off) on SC_FJ_THREADS threads, so a default-on regression is a
                   # selfcheck failure; every other program runs with t1-run's default (the CPUs)
                   fl=; case " $fprogs " in *" $n "*) fl="--files $tmp/$n.root" ;; esac; case $n in file_*) fl="--files $tmp/$n.root" ;; esac   # no FILES_PROGS: fl empty, argv identical
                   case $n in
                     fj_*|*region*) SASSEMBLY_NATIVE_FJ_MIN=0 SASSEMBLY_NATIVE_THREADS=${SC_FJ_THREADS:-8} SAS_BIN="$tmp/nobin" \
                                    T1_RUN_REQUIRE_NATIVE=1 $wt "$tmo" "$t1run" $fl "$elf" > "$tmp/$n.out" 2> "$tmp/$n.err" ;;
                     *) SAS_BIN="$tmp/nobin" T1_RUN_REQUIRE_NATIVE=1 $wt "$tmo" "$t1run" $fl "$elf" > "$tmp/$n.out" 2> "$tmp/$n.err" ;;
                   esac ;;
              esac ;;
    esac
    echo $? > "$tmp/$n.rc"
    # STAGE 2 (P9-B host.t1): the second program a case left in its files root runs here (tools/selfcheck/stage2.sh, shared with mutants.sh)
    case $runner in
      yantra) sh "$here/stage2.sh" "$tmp/$n.root" "$tmp/$n.out" "$tmp/$n.err2" $wt "$tmo" "$yantra" ;;
      native) mkdir -p "$tmp/nobin"; SAS_BIN="$tmp/nobin" T1_RUN_REQUIRE_NATIVE=1 sh "$here/stage2.sh" "$tmp/$n.root" "$tmp/$n.out" "$tmp/$n.err2" $wt "$tmo" "$t1run" ;;
    esac
  ) &
  # SC_JOBS caps parallel programs (L-SCLOAD: 90 at once on a loaded host timed out three heavy ones)
  i=$((${i:-0} + 1)); [ $((i % jobs)) -ne 0 ] || wait
done
wait
t1=$(now)
elapsed=$(perl -e "printf '%.3f', $t1 - $t0")

# status and the runner-exit consistency check; prints "<status>" or "BAD:<why>"
status_of() {
  n=$1; rc=$(cat "$tmp/$n.rc" 2>/dev/null || echo 125)
  case $rc in 124) echo "BAD:TIMEOUT after ${tmo}s"; return ;; 126) echo "BAD:stale image"; return ;; 127) echo "BAD:no image"; return ;; esac
  nh=$(grep -ac '^halt: ' "$tmp/$n.err")
  if [ "$nh" != 1 ]; then echo "BAD:$nh halt lines (runner exit $rc)"; return; fi
  s=$(sed -n 's/^halt: Finisher { value: [0-9]*, status: Some(\([0-9]*\)) }$/\1/p' "$tmp/$n.err")
  if [ -n "$s" ]; then
    if [ "$s" = 0 ]; then want=0; else want=1; fi
    [ "$rc" = "$want" ] || { echo "BAD:runner exit $rc inconsistent with halt status $s (expected $want)"; return; }
    echo "$s"
  else
    [ "$rc" != 0 ] || { echo "BAD:runner exit 0 after a halt with no status"; return; }
    echo "halt:$(sed -n 's/^halt: //p' "$tmp/$n.err")"
  fi
}

for n in $names; do
  got_st=$(status_of "$n"); got_sz=$(wc -c < "$tmp/$n.out" | tr -d ' '); got_h=$(sha < "$tmp/$n.out")
  case $got_st in BAD:*) printf '%s\t%s\t%s\t%s\n' "$n" "$got_st" "$got_sz" "$got_h" >> "$tmp/results.tsv"
                         [ "$mode" = capture ] || mismatch "$n" "${got_st#BAD:}"; continue ;; esac
  printf '%s\t%s\t%s\t%s\n' "$n" "$got_st" "$got_sz" "$got_h" >> "$tmp/results.tsv"
  if [ "$mode" = capture ]; then
    gloss=$(head -c 60 "$tmp/$n.out" | tr '\n' '|' | LC_ALL=C tr -c '[:print:]' '?' | tr '\t' ' ')
    printf '%s\t%s\t%s\t%s\t%s\n' "$n" "$got_st" "$got_sz" "$got_h" "$gloss"
    continue
  fi
  line=$(awk -F'\t' -v n="$n" '$1 == n {print; exit}' "$exp")
  [ -n "$line" ] || continue
  want_st=$(printf '%s' "$line" | cut -f2); want_sz=$(printf '%s' "$line" | cut -f3); want_h=$(printf '%s' "$line" | cut -f4)
  [ "$got_st" = "$want_st" ] || mismatch "$n" "status $got_st, pinned $want_st"
  [ "$got_h" = "$want_h" ] || mismatch "$n" "stdout differs ($got_sz octets sha256 ${got_h%"${got_h#????????}"}..., pinned $want_sz octets ${want_h%"${want_h#????????}"}...)"
done
[ "$mode" = capture ] && { echo "elapsed ${elapsed}s (yantra, capture)" >&2; [ "$bad" = 0 ]; exit $?; }

if [ "$mode" = verify ]; then
  # the oracle half: every row well-formed and non-empty, no orphan row, and expected.tsv agrees with it
  orc=$dir/oracle.tsv
  if [ ! -s "$orc" ] && [ "${SC_ORACLE_OPTIONAL:-}" = 1 ] && [ ! -e "$orc" ]; then
    # t1tests: an expected-*.tsv group pinned from a yantra capture has no independent oracle; its pins are checked against yantra (this run) only
    echo "selfcheck: no oracle.tsv (SC_ORACLE_OPTIONAL=1): pins verified against yantra only" >&2
  elif [ ! -s "$orc" ]; then mismatch "oracle.tsv" "is missing or empty"; else
    awk -F'\t' '/^#/ {next} NF != 4 || $1 == "" || $2 == "" || $3 == "" || length($4) != 64 || $4 ~ /[^0-9a-f]/ { printf "MISMATCH oracle.tsv: line %d is malformed\n", NR > "/dev/stderr"; bad = 1 } END { exit bad }' "$orc" || bad=$((bad + 1))
    [ "$(grep -avc '^#' "$orc")" -gt 0 ] || mismatch "oracle.tsv" "has no rows"
    for n in $(grep -av '^#' "$orc" | cut -f1); do
      [ -f "$dir/$n.t1" ] || mismatch "$n" "is an ORPHAN row in oracle.tsv (no such program)"
    done
    for n in $names; do
      o=$(awk -F'\t' -v n="$n" '$1 == n {print; exit}' "$orc"); [ -n "$o" ] || continue
      e=$(awk -F'\t' -v n="$n" '$1 == n {print; exit}' "$exp")
      os=$(printf '%s' "$o" | cut -f2); oh=$(printf '%s' "$o" | cut -f4)
      [ "$os" = - ] || [ "$os" = "$(printf '%s' "$e" | cut -f2)" ] || mismatch "$n" "pinned status disagrees with the oracle ($os)"
      [ "$oh" = "$(printf '%s' "$e" | cut -f4)" ] || mismatch "$n" "pinned stdout disagrees with the oracle"
    done
  fi
fi

digest=$(LC_ALL=C sort "$tmp/results.tsv" 2>/dev/null | sha)
echo "selfcheck: digest $digest"
echo "selfcheck: $count programs, runner=$runner, mode=$mode, elapsed ${elapsed}s"
if [ "$bad" -gt 0 ]; then echo "selfcheck: FAILED, $bad mismatch(es)" >&2; exit 1; fi
echo "selfcheck: ok"
