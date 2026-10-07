#!/usr/bin/env bash
# W-302 — THE QEMU CHECK SCRIPTS PASS ON THE .t1 CHAIN'S IMAGES.
#
# Every tools/check-*.sh that boots a spec/*.sas program under
# qemu-system-riscv64 (52 of them, found below by reading the scripts, not
# listed by hand) is run AS WRITTEN — its own QEMU command and its own success
# string — with one change: a `cargo` placed first on PATH hands each
# `cargo run ... -p sadhana -- <args>` to tools/w302-sadhana-shim.py, which
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
# printed. ONE NAMED EXCEPTION, checked to still hold:
#   check-namaste.sh — its compressed (RVC) half is OUT OF SCOPE by the owner's
#   ruling of 2026-10-04 ("leave compression out of W-302"): the .t1 chain must
#   FAIL it, with the shim's refusal naming --संक्षिप्त, while Rust PASSES it.
#   If it ever stops failing that way, the exception is stale and this exits 1.
# Exits 0 when every script agrees (the exception as stated), 1 otherwise, 77
# when it cannot run.

(eval ': <(:)') 2>/dev/null || exec /usr/bin/env bash "$0" "$@"

set -uo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)

for t in cargo python3 qemu-system-riscv64 timeout; do
  if ! command -v "$t" >/dev/null 2>&1; then
    echo "CANNOT RUN: $t is not on PATH"
    exit 77
  fi
done
REAL_CARGO=$(command -v cargo)

WORK=$(mktemp -d "${TMPDIR:-/tmp}/w302-qemu.XXXXXX") || exit 77
trap 'rm -rf "$WORK"' EXIT

pick() {
  python3 -c '
import json, sys
want = sys.argv[1]
for line in sys.stdin:
    try:
        m = json.loads(line)
    except ValueError:
        continue
    if m.get("reason") == "compiler-artifact" and m.get("executable") \
       and m.get("target", {}).get("name") == want:
        print(m["executable"])
' "$1"
}
built() { cargo build --release -p "$1" --bin "$2" --message-format=json 2>/dev/null | pick "$2"; }
# W302_YANTRA_RUN=<runner> replaces the runner — how a control hands the
# scripts deliberately wrong images (a runner that drops the load address).
YANTRA_RUN=${W302_YANTRA_RUN:-$(built yantra yantra-run)}
if [ -z "$YANTRA_RUN" ] || [ ! -x "$YANTRA_RUN" ]; then
  echo "FAIL: yantra-run did not build (release)"; exit 1
fi
if [ -n "${W302_NATIVE:-}" ]; then
  NATIVE=$W302_NATIVE
  [ -s "$NATIVE" ] || { echo "FAIL: W302_NATIVE=$NATIVE is not a non-empty file"; exit 1; }
  echo "native: $NATIVE — GIVEN, NOT BUILT by this run"
else
  T1_IMAGE=$(built sadhana t1_image)
  [ -x "$T1_IMAGE" ] || { echo "FAIL: t1_image did not build (release)"; exit 1; }
  NATIVE="$WORK/assemble.elf"
  echo "native: building the wrapper image (the corpus + spec/entry/assemble.t1), tens of minutes"
  "$T1_IMAGE" --spec-root spec --compiler crates/sadhana-t1/src \
      --load spec/entry/assemble.t1 --entry पाठसेतुः मुख्यम् --no-predict \
      --accept-divergence "the wrapper assembles the .sas its input channel is handed, and the census hands each of 48 programs its own input later; this build has no input, so its predict compares nothing the census runs" \
      -o "$NATIVE" crates/sadhana-t1/src/*.t1 spec/entry/assemble.t1 > "$WORK/build.log" 2>&1 \
    || { echo "FAIL: t1_image did not build the wrapper image"; tail -20 "$WORK/build.log"; exit 1; }
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
  exec python3 "$ROOT/tools/w302-sadhana-shim.py" "\${args[@]:\$((dashdash+1))}"
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
  grep -q 'qemu-system-riscv64' "$f" || continue
  grep -q '\.sas' "$f" || continue
  scripts+=("$(basename "$f")")
done
if [ -n "${W302_ONLY:-}" ]; then
  read -r -a scripts <<< "$W302_ONLY"
fi
echo "scripts: ${#scripts[@]}"

pass=0; fail=0; notfed=0; skip=0; agree=0; disagree=0; excepted=0
for s in "${scripts[@]}"; do
  timeout 600 "./tools/$s" > "$WORK/$s.rust.out" 2>&1; rrc=$?
  case $rrc in 0) rv=PASS ;; 77) rv=SKIP ;; *) rv=FAIL ;; esac
  base=" [rust $rv$( [ "$rv" = FAIL ] && printf '(%s)' "$rrc")]"
  log="$WORK/$s.shim.log"; : > "$log"
  PATH="$WORK/shim:$PATH" W302_SHIM_LOG="$log" W302_NATIVE="$NATIVE" W302_YANTRA_RUN="$YANTRA_RUN" \
    timeout 900 "./tools/$s" > "$WORK/$s.t1.out" 2>&1
  rc=$?
  images=$(grep -c '^IMAGE ' "$log"); refused=$(grep -c '^REFUSED ' "$log")
  if [ $rc = 77 ]; then v=SKIP; skip=$((skip+1))
  elif [ $rc = 0 ] && [ "$images" -gt 0 ] && [ "$refused" = 0 ]; then v=PASS; pass=$((pass+1))
  elif [ $rc = 0 ] && [ "$images" = 0 ]; then v="NOT FED"; notfed=$((notfed+1))
  else v="FAIL($rc)"; fail=$((fail+1)); fi
  tv=${v%%(*}
  if [ "$s" = check-namaste.sh ]; then
    if [ "$tv" = FAIL ] && [ "$rv" = PASS ] && grep '^REFUSED ' "$log" | grep -q -- '--संक्षिप्त'; then
      agreement="EXCEPTED (RVC out of scope, owner 2026-10-04)"; excepted=$((excepted+1))
    else
      agreement="STALE EXCEPTION (expected .t1 FAIL on --संक्षिप्त with rust PASS)"; disagree=$((disagree+1))
    fi
  elif [ "$tv" = "$rv" ]; then agreement=AGREES; agree=$((agree+1))
  else agreement=DISAGREES; disagree=$((disagree+1)); fi
  printf 'W302-QEMU %-28s %-8s images=%s refused=%s%s %s\n' "$s" "$v" "$images" "$refused" "$base" "$agreement"
  if [ "$v" != PASS ]; then
    grep '^REFUSED ' "$log" | head -3 | sed 's/^/    shim: /'
    # The first lines that say WHY, then the tail: a script's tail is often
    # its closing explanation and not its failure.
    grep -m3 -E 'FAIL|CANNOT|[Ee]rror|not installed|invalid' "$WORK/$s.t1.out" | sed 's/^/    why:  /'
    tail -2 "$WORK/$s.t1.out" | sed 's/^/    out:  /'
  fi
done
echo "METRIC w302_qemu scripts=${#scripts[@]} pass=$pass fail=$fail not_fed=$notfed skip=$skip agree=$agree excepted=$excepted disagree=$disagree"
if [ "$disagree" = 0 ] && [ $((agree+excepted)) = "${#scripts[@]}" ]; then
  echo "PASS: the .t1 chain's images get Rust's verdict on every QEMU check script (check-namaste excepted by name)"; exit 0
fi
echo "FAIL: $disagree QEMU check script(s) do not get Rust's verdict on the .t1 chain's images"
exit 1
