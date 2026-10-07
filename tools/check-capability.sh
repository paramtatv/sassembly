#!/bin/sh
# Capability derivation attenuates, never amplifies — task `C-002a`, doc 11 §3.1.
#
# `अधिकारः` is an unforgeable reference to an object plus rights, and doc 11
# §3.1 says it is THE ONLY WAY TO NAME ANYTHING. The whole model rests on one
# one-way property: a derived capability may drop rights and may never gain one
# its parent lacked. If a capability could exceed its parent, the system is a
# suggestion.
#
# ## Refusal must be distinguishable from an empty grant
#
# The first version of this program returned rights alone, so "asked for nothing
# and got it" and "asked and was refused" were both 0 — the same silence this
# file argues against. A difference the caller cannot read is not a difference.
# The answer is now two words: a verdict, then the rights.
#
#   parent 7             read|write|exec, NO grant
#   (1, 3) (1, 1) (1, 0) three attenuations, the last granting nothing
#   (0, 0) (0, 0)        two amplifications, both REFUSED
#
# The third attenuation is the one that earns the pair: it is granted and empty,
# and reads (1, 0) where a refusal reads (0, 0).
#
# Partial grants are refused whole. Asking for read|write|grant when the parent
# has read|write is not "here is read|write" — that is the silent masking the
# model forbids, and the caller would go on believing it holds grant.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/capability.sas" "$tmp/c.elf" >/dev/null
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/c.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -11
}

fail=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 11 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 11 (the parent's rights,"
        echo "        then a verdict and a rights word for each of five attempts)"
        fail=1; continue
    fi
    if ! python3 - "$tmp/l" > "$tmp/v" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
parent, rest = v[0], v[1:]
pairs = [(rest[i], rest[i+1]) for i in range(0, 10, 2)]
READ, WRITE, EXEC, GRANT = 1, 2, 4, 8
want = [(1, 3), (1, 1), (1, 0), (0, 0), (0, 0)]
asked = [3, 1, 0, GRANT, READ | WRITE | GRANT]
bad = []
if parent != READ | WRITE | EXEC:
    bad.append(f"the parent holds {parent:b}, expected 111 — read, write, exec and NO grant")
for i, ((gv, gr), (wv, wr), a) in enumerate(zip(pairs, want, asked)):
    if (gv, gr) != (wv, wr):
        if wv == 0 and gv == 1:
            bad.append(f"attempt {i} asked for {a:b}, which the parent ({parent:b}) does not "
                       f"hold, and it was GRANTED as {gr:b}. A capability exceeded its parent, "
                       "so the model is a suggestion")
        elif wv == 0 and gr != 0:
            bad.append(f"attempt {i} was refused but returned rights {gr:b} — a refusal must "
                       "carry nothing")
        elif wv == 1 and gv == 0:
            bad.append(f"attempt {i} asked for {a:b}, entirely within the parent, and was "
                       "REFUSED — attenuation must be allowed or nothing can be delegated")
        else:
            bad.append(f"attempt {i} asked {a:b}: got verdict {gv} rights {gr:b}, "
                       f"expected verdict {wv} rights {wr:b}")
# The property the pair exists for.
granted_empty = pairs[2]
refused = pairs[3]
if granted_empty == refused:
    bad.append(f"a granted-but-empty derivation reads {granted_empty} and a REFUSAL reads "
               f"{refused} — identical, so the caller cannot tell being given nothing from "
               "being turned down, which is the ambiguity the two-word answer exists to remove")
if bad: print("\n".join(bad)); sys.exit(1)
print(f"parent {parent:b}; attenuations {[p[1] for p in pairs[:3]]} all granted, "
      f"amplifications both refused, and granted-empty {granted_empty} differs from refused {refused}")
PY
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
done
[ "$fail" -eq 0 ] || { echo; echo "capability derivation is wrong."; exit 1; }
echo
echo "ok  a derived capability may drop rights and never gains one its parent"
echo "    lacked; an amplification is REFUSED whole rather than silently masked,"
echo "    and a refusal is distinguishable from a grant of nothing"
