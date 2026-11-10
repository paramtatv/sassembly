#!/bin/sh
# tools/selfcheck/build.sh [OUTDIR [name...]] — compile the programs to OUTDIR/<name>.elf, in parallel. FAILS CLOSED: any program that
# does not build, or leaves no image is named and
# the exit is non-zero. Stale images are deleted first, so a failed build can never be run from an old image.
# Env: SC_DIR    the programs (default: this directory). The landing runs the TRUSTED copy of this script with SC_DIR = the tree's.
#      SC_TREE   the tree whose compiler is used (default: this checkout) -- for t1-build.sh it is SAS
#      SC_SPEC_ROOT / SC_COMPILER (or the flags --spec-root D / --compiler D before OUTDIR): passed to t1-build.sh
#      SC_BUILDER=t1-build  (the only builder) tools/t1-build.sh: the cached Stage 1, native. T1_BUILD_CACHED_ONLY honoured.
#      SC_JOBS   max builds at once (default: all), the same knob as run.sh
#      NO RUST (owner ruling 2026-10-10): the t1_image route was retired, and t1-build.sh gets SAS_BIN=/nonexistent so its own t1_image
#      fallback can never run either (no cached Stage 1 = the build fails).
# Compilation is NOT in the 10 s budget (BUDGET.md): images are built ahead.
set -u
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
# --spec-root D / --compiler D: forwarded to every tools/t1-build.sh call (also by env SC_SPEC_ROOT / SC_COMPILER); absent = t1-build's defaults
while [ $# -gt 0 ]; do
  case $1 in
    --spec-root|--compiler)
      [ $# -ge 2 ] || { echo "$(basename "$0"): REFUSED: $1 needs a directory" >&2; exit 2; }
      if [ "$1" = --compiler ]; then SC_COMPILER=$2; export SC_COMPILER; else SC_SPEC_ROOT=$2; export SC_SPEC_ROOT; fi
      shift 2 ;;
    *) break ;;
  esac
done
dir=${SC_DIR:-$here}; tree=${SC_TREE:-$root}
out=${1:-$dir/out}
[ $# -gt 0 ] && shift
only=" $* "
case ${SC_BUILDER:-t1-build} in t1-build) ;; *) echo "build.sh: REFUSED: SC_BUILDER=${SC_BUILDER} (only t1-build: the Rust interpreter route is gone)" >&2; exit 2 ;; esac
[ -z "${T1_IMAGE:-}" ] || { echo "build.sh: REFUSED: T1_IMAGE is gone (no Rust interpreter in the selfcheck)" >&2; exit 2; }
# no wall-clock bound (owner 2026-10-19): the build runs until it exits; its own status is the verdict
wt="bash $here/../hb-run.sh"
case ${SC_JOBS-} in '') ;; 0*|*[!0-9]*) echo "build.sh: SC_JOBS must be a positive integer (got '$SC_JOBS')" >&2; exit 2 ;; esac
mkdir -p "$out" || exit 2
rm -f "$out"/.failed.*
one() {
  f=$1; n=$(basename "$f" .t1)
  rm -f "$out/$n.elf"
  mod=$(sed -n '1s/^मण्डलम् \([^ ]*\) .*/\1/p' "$f")
  [ -n "$mod" ] || { echo "build.sh: $n has no module line" > "$out/$n.build.log"; return 1; }
  # a program with a sibling <name>.src is compiled WITH the compiler's own sources (it imports their modules)
  srcs=; [ -f "${f%.t1}.src" ] && srcs=$(ls "$tree"/crates/sadhana-t1/src/*.t1)
  SAS="$tree" SAS_BIN=/nonexistent T1_BUILD_CACHED_ONLY=${T1_BUILD_CACHED_ONLY:-} $wt "$root/tools/t1-build.sh" ${SC_SPEC_ROOT:+--spec-root "$SC_SPEC_ROOT"} ${SC_COMPILER:+--compiler "$SC_COMPILER"} --entry "$mod" मुख्यम् -o "$out/$n.elf" $srcs "$f" > "$out/$n.build.log" 2>&1
  rc=$?
  [ "$rc" = 0 ] && [ -s "$out/$n.elf" ] || { echo "build.sh: $n: refused (exit $rc, image $( [ -s "$out/$n.elf" ] && echo present || echo MISSING); see $out/$n.build.log)" >&2; return 1; }
}
first=1; made=0
for f in "$dir"/*.t1; do
  [ -f "$f" ] || continue
  n=$(basename "$f" .t1)
  [ "$only" = "  " ] || case "$only" in *" $n "*) ;; *) continue ;; esac
  made=$((made + 1))
  if [ "$first" = 1 ]; then
    one "$f" || : > "$out/.failed.$n"   # the first build alone: it translates Stage 1 into the cache; the rest then only read it
  else
    ( one "$f" || : > "$out/.failed.$n" ) &
    # SC_JOBS caps parallel builds (L-SCLOAD, as run.sh)
    i=$((${i:-0} + 1)); [ -z "${SC_JOBS:-}" ] || [ $((i % SC_JOBS)) -ne 0 ] || wait
  fi
  first=0
done
wait
[ "$made" -gt 0 ] || { echo "build.sh: no programs to build" >&2; exit 1; }
[ -z "$(ls "$out"/.failed.* 2>/dev/null)" ]
