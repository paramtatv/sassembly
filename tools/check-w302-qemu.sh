#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# W-302 — THE QEMU CHECK SCRIPTS PASS ON THE .t1 CHAIN'S IMAGES.
#
# Every tools/check-*.sh that boots a spec/*.sas program under
# qemu-system-riscv64 (52 of them, found below by reading the scripts, not
# listed by hand) is run AS WRITTEN — its own QEMU command and its own success
# string — with one change: a `cargo` placed first on PATH hands each
# `cargo run ... -p sadhana -- <args>` to tools/w302-sadhana-shim.sh, which
# assembles with the SELF-HOSTED .t1 image (spec/entry/assemble.t1 built with
# `t1_image --entry`, run under yantra-run) instead of Rust sadhana. Every
# other cargo call goes to the real cargo unchanged. No script is edited.
#
# A script's verdict here:
#   PASS     it exited 0 AND the shim wrote at least one image for it AND the
#            shim refused nothing — the run was fed by the .t1 chain
#   FAIL     it exited non-zero (the shim's refusals, if any, are printed)
#   NOT FED  it exited 0 but the shim wrote no image: the script assembled
#            nothing through `cargo run -p sadhana`, so it says nothing here
#   SKIP     it exited 77 (cannot run on this host)
#
# W302_NATIVE=<image> grades a prebuilt wrapper image instead of building one
# (tens of minutes); W302_YANTRA_RUN=<runner> replaces the runner;
# W302_ONLY="check-arena.sh check-buddy.sh" narrows the set — BASENAMES OUT OF THE
# POPULATION BELOW, and REAL ones because the two a/b placeholders that stood in this line
# red `metrics`' ledger/tools reader: a margin naming a script that does not exist is read
# as a statement of how things are, and a usage example nobody can paste is exactly that
# kind of false statement. (The replacement cannot QUOTE the old names either — spelling
# them to explain the fix re-trips the same reader, which is how this was measured twice.)
# Usage: tools/check-w302-qemu.sh
# THE CLOSING CONDITION IS AGREEMENT WITH RUST, NOT 52 PASSES (coordinator
# ruling 2026-10-04, after review): each script is ALSO run unshimmed, on Rust
# sadhana, on the same host, every time (W302_BASELINE is no longer optional),
# and the .t1 chain's verdict class (PASS / SKIP / FAIL) must equal Rust's.
# NOT FED never agrees. A host fault that fails both sides agrees, and is
# printed. NO EXCEPTIONS: check-namaste.sh was once excepted (its compressed RVC
# half was out of scope, owner ruling 2026-10-04); the .t1 chain now builds and
# runs both images, so the exception went stale and the script must AGREE like
# every other. Exits 0 when every script agrees, 1 otherwise, 77 when it cannot
# run.

(eval ': <(:)') 2>/dev/null || exec /usr/bin/env bash "$0" "$@"

set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)

for t in qemu-system-riscv64; do
  if ! command -v "$t" >/dev/null 2>&1; then
    echo "CANNOT RUN: $t is not on PATH"
    exit 77
  fi
done
REAL_CARGO=$(command -v cargo || echo cargo)  # the crates left the tree (R3-YAN): cargo only reaches the frozen oracle build and the dormant shim below
root=$ROOT

WORK=$(mktemp -d "${TMPDIR:-/tmp}/w302-qemu.XXXXXX") || exit 77
trap 'rm -rf "$WORK"' EXIT

# W302_YANTRA_RUN=<runner> replaces the runner — how a control hands the
# scripts deliberately wrong images (a runner that drops the load address).
# W302_RUNNER=native selects tools/w302-native-runner.sh (t1-run.sh, native).
if [ -z "${W302_YANTRA_RUN:-}" ] && [ "${W302_RUNNER:-}" = native ]; then W302_YANTRA_RUN=$PWD/tools/w302-native-runner.sh; fi
if [ -z "${W302_YANTRA_RUN:-}" ]; then
  # the in-tree cargo build is gone with crates/yantra: the pinned frozen oracle (ensure_bins; 77 when it cannot be had)
  . "$ROOT/tools/ensure-yantra-bins.sh"; ensure_bins "$WORK/bin"
  W302_YANTRA_RUN=$WORK/bin/yantra-run
fi
YANTRA_RUN=$W302_YANTRA_RUN
if [ ! -x "$YANTRA_RUN" ]; then
  echo "FAIL: yantra-run is not executable: $YANTRA_RUN"; exit 1
fi
if [ -n "${W302_NATIVE:-}" ]; then
  NATIVE=$W302_NATIVE
  [ -s "$NATIVE" ] || { echo "FAIL: W302_NATIVE=$NATIVE is not a non-empty file"; exit 1; }
  echo "native: $NATIVE — GIVEN, NOT BUILT by this run"
else
  NATIVE="$WORK/assemble.elf"
  echo "native: building the wrapper image natively (tools/t1-build.sh: the corpus + spec/entry/assemble.t1)"
  # the census hands each of the programs its own input later; t1-build predicts nothing (the t1_image --no-predict/--accept-divergence flags are gone with the Rust crate)
  tools/t1-build.sh --spec-root spec --compiler crates/sadhana-t1/src \
      --entry पाठसेतुः मुख्यम् \
      -o "$NATIVE" crates/sadhana-t1/src/*.t1 spec/entry/assemble.t1 > "$WORK/build.log" 2>&1 \
    || { rc=$?; echo "FAIL: t1-build did not build the wrapper image (rc $rc)"; tail -20 "$WORK/build.log"; [ "$rc" = 77 ] && exit 77; exit 1; }
fi
NATIVE=$(cd "$(dirname "$NATIVE")" && pwd)/$(basename "$NATIVE")

# The `cargo` in front of the real one.
mkdir -p "$WORK/shim"
cat > "$WORK/shim/cargo" <<EOF
#!/usr/bin/env bash
run=0; sadhana=0; dashdash=-1; i=0; args=("\$@")
for a in "\$@"; do
  case "\$a" in
    run) [ "\$dashdash" -lt 0 ] && run=1 ;;
    --) [ "\$dashdash" -lt 0 ] && dashdash=\$i ;;
  esac
  if [ "\$dashdash" -lt 0 ] && [ "\$a" = sadhana ] && [ "\$i" -gt 0 ] && [ "\${args[\$((i-1))]}" = -p ]; then sadhana=1; fi
  i=\$((i+1))
done
if [ "\$run" = 1 ] && [ "\$sadhana" = 1 ] && [ "\$dashdash" -ge 0 ]; then
  exec sh "$ROOT/tools/w302-sadhana-shim.sh" "\${args[@]:\$((dashdash+1))}"
fi
exec "$REAL_CARGO" "\$@"
EOF
chmod +x "$WORK/shim/cargo"

# The population: every check script that boots a spec program under QEMU.
scripts=()
# The W-302 scripts themselves are not in it: this driver names QEMU and .sas
# in its own text, and running itself would recurse.
for f in tools/check-*.sh; do
  case "$f" in tools/check-w302-*) continue ;; esac
  grep -aq 'qemu-system-riscv64' "$f" || continue
  grep -aq '\.sas' "$f" || continue
  scripts+=("$(basename "$f")")
done
if [ -n "${W302_ONLY:-}" ]; then
  read -r -a scripts <<< "$W302_ONLY"
fi
echo "scripts: ${#scripts[@]}"

# One unit per script, run W302_JOBS at a time (each unit has its own log and
# output files; the verdict lines and the counts are assembled in script order
# below, so the closing condition is unchanged).
w302_unit() {
  s=$1
  t0=$(date +%s)
  # the two legs share nothing (own mktemp dirs, own logs), so they run side by side
  ( "./tools/$s" > "$WORK/$s.rust.out" 2>&1; echo $? > "$WORK/$s.rust.rc" ) &
  rpid=$!
  log="$WORK/$s.shim.log"; : > "$log"
  PATH="$WORK/shim:$PATH" W302_SHIM_LOG="$log" W302_NATIVE="$NATIVE" W302_YANTRA_RUN="$YANTRA_RUN" \
    "./tools/$s" > "$WORK/$s.t1.out" 2>&1
  rc=$?
  t2=$(date +%s)
  wait "$rpid"
  rrc=$(cat "$WORK/$s.rust.rc" 2>/dev/null); rrc=${rrc:-1}
  case $rrc in 0) rv=PASS ;; 77) rv=SKIP ;; *) rv=FAIL ;; esac
  t1=$(date +%s)
  base=" [rust $rv$( [ "$rv" = FAIL ] && printf '(%s)' "$rrc")]"
  images=$(grep -ac '^IMAGE ' "$log"); refused=$(grep -ac '^REFUSED ' "$log")
  if [ $rc = 77 ]; then v=SKIP; c=skip
  elif [ $rc = 0 ] && [ "$images" -gt 0 ] && [ "$refused" = 0 ]; then v=PASS; c=pass
  elif [ $rc = 0 ] && [ "$images" = 0 ]; then v="NOT FED"; c=notfed
  else v="FAIL($rc)"; c=fail; fi
  tv=${v%%(*}
  if [ "$tv" = "$rv" ]; then agreement=AGREES; a=agree
  else agreement=DISAGREES; a=disagree; fi
  {
    printf 'COUNT %s %s\n' "$c" "$a"
    printf 'W302-QEMU %-28s %-8s images=%s refused=%s%s %s (t1 leg %ss, both %ss)\n' "$s" "$v" "$images" "$refused" "$base" "$agreement" "$((t2-t0))" "$((t1-t0))"
    if [ "$v" != PASS ]; then
      grep -a '^REFUSED ' "$log" | head -3 | sed 's/^/    shim: /'
      # The first lines that say WHY, then the tail: a script's tail is often
      # its closing explanation and not its failure.
      grep -a -m3 -E 'FAIL|CANNOT|[Ee]rror|not installed|invalid' "$WORK/$s.t1.out" | sed 's/^/    why:  /'
      tail -2 "$WORK/$s.t1.out" | sed 's/^/    out:  /'
    fi
  } > "$WORK/$s.verdict"
}
W302_JOBS=${W302_JOBS:-8}
running=0
for s in "${scripts[@]}"; do # bash32-ok: non-empty by construction
  w302_unit "$s" &
  running=$((running+1))
  if [ "$running" -ge "$W302_JOBS" ]; then wait -n 2>/dev/null || wait; running=$((running-1)); fi
done
wait
pass=0; fail=0; notfed=0; skip=0; agree=0; disagree=0; excepted=0
for s in "${scripts[@]}"; do # bash32-ok: non-empty by construction
  if [ ! -s "$WORK/$s.verdict" ]; then
    printf 'W302-QEMU %-28s NO VERDICT (unit did not finish)\n' "$s"; disagree=$((disagree+1)); continue
  fi
  read -r _ c a < "$WORK/$s.verdict"
  case $c in pass) pass=$((pass+1)) ;; fail) fail=$((fail+1)) ;; notfed) notfed=$((notfed+1)) ;; skip) skip=$((skip+1)) ;; esac
  case $a in agree) agree=$((agree+1)) ;; excepted) excepted=$((excepted+1)) ;; disagree) disagree=$((disagree+1)) ;; esac
  sed 1d "$WORK/$s.verdict"
done
echo "METRIC w302_qemu scripts=${#scripts[@]} pass=$pass fail=$fail not_fed=$notfed skip=$skip agree=$agree excepted=$excepted disagree=$disagree"
[ "$disagree" != 0 ] || [ "$pass" -gt 0 ] || { echo "UNRUN: no QEMU check script passed on either leg (could-not-run is never a pass)"; exit 77; }
if [ "$disagree" = 0 ] && [ $((agree+excepted)) = "${#scripts[@]}" ]; then
  echo "PASS: the .t1 chain's images get the reference leg's verdict on every QEMU check script"; exit 0
fi
echo "FAIL: $disagree QEMU check script(s) do not get the reference leg's verdict on the .t1 chain's images"
exit 1
