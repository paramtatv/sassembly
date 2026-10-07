#!/bin/sh
# W^X, and a refusal that says which rule it broke — `C-002c`, doc 11 §3.3.
#
# A frame is mapped into a `स्मृतिक्षेत्रम्` and its rights are fixed at that
# moment. The heaviest rule is W^X: NO PAGE IS EVER WRITABLE AND EXECUTABLE.
# That is what makes doc 09's D-09-C (no JIT) coherent, and what forces doc 16's
# Tier 2 patcher to remap transiently — a rare, audited operation.
#
# ## Two refusals, and they are not the same refusal
#
#   1  W^X          writable and executable together — OUR rule
#   2  W without R  reserved by RISC-V itself — THE MACHINE's rule
#
# Returning 0 for both would repeat a mistake made earlier the same day: two
# mutations at milestone M-K1 were caught correctly and explained wrongly, and a
# wrong explanation sends the next reader to the wrong place. So the answer
# carries a reason: 0 granted, 1 refused by W^X, 2 refused as reserved.
#
# The distinction is not academic. A W^X refusal means the CALLER asked for
# something the system forbids; a reserved-encoding refusal means the caller
# asked for something the HARDWARE will fault on — and a reserved PTE does not
# fault when written, it faults when someone eventually touches the page.
#
# Sv39 bits: V=1 R=2 W=4 X=8 U=16 G=32 A=64 D=128, PPN shifted left by 10.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/mapping.sas" "$tmp/m.elf" >/dev/null
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/m.elf" > "$tmp/out" 2>&1 &
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

fail=0; prev=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 11 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 11 (the page, then a"
        echo "        reason and an entry for each of five requests)"
        fail=1; continue
    fi
    if ! python3 - "$tmp/l" > "$tmp/v" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
page, rest = v[0], v[1:]
pairs = [(rest[i], rest[i+1]) for i in range(0, 10, 2)]
V, R, W, X, A = 1, 2, 4, 8, 64
asked = [R, R | W, R | X, R | W | X, W]
want  = [0, 0, 0, 1, 2]        # 0 granted, 1 W^X, 2 reserved
names = {0: "granted", 1: "refused by W^X", 2: "refused as reserved"}
bad = []
if page % 4096:
    bad.append(f"the frame {page:016x} is not 4096-aligned")
ppn = page >> 12

for i, ((reason, pte), wr, a) in enumerate(zip(pairs, want, asked)):
    if reason != wr:
        if wr == 1 and reason == 0:
            bad.append(f"request {i} asked for R|W|X and was GRANTED as {pte:#x}. W^X is "
                       "not enforced, so a page can be written and then executed — which "
                       "is the whole shape of attack the rule exists to remove")
        elif wr == 2 and reason == 0:
            bad.append(f"request {i} asked for W without R and was GRANTED as {pte:#x}. "
                       "That encoding is RESERVED in RISC-V: the fault does not arrive "
                       "when the entry is written, it arrives when someone touches the page")
        elif wr == 0 and reason != 0:
            bad.append(f"request {i} asked for {a:#x}, which breaks no rule, and was "
                       f"refused with reason {reason} ({names.get(reason,'?')})")
        else:
            bad.append(f"request {i}: reason {reason} ({names.get(reason,'?')}), "
                       f"expected {wr} ({names[wr]})")
        continue
    if reason != 0:
        if pte != 0:
            bad.append(f"request {i} was refused but returned entry {pte:#x} — a refusal "
                       "must not hand back a usable mapping")
        continue
    want_pte = (ppn << 10) | a | V | A
    if pte != want_pte:
        bad.append(f"request {i}: entry {pte:#x}, expected {want_pte:#x} — the frame or "
                   "the rights are not what was asked for")

# The two refusals must be tellable apart, which is the point of the reason.
if pairs[3][0] == pairs[4][0]:
    bad.append(f"the W^X refusal and the reserved-encoding refusal both report "
               f"{pairs[3][0]} — the caller cannot tell a rule it broke from a rule the "
               "machine imposes, and the two need different fixes")
if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"frame {page:016x}: R, R|W and R|X mapped with correct entries; R|W|X refused by "
      f"W^X (1) and W-without-R refused as reserved (2), and the two are distinguishable")
PY
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
    b=$(sed -n 1p "$tmp/l")
    [ -n "$prev" ] && [ "$b" = "$prev" ] && { echo "  FAIL  frame $b at both link addresses — a constant"; fail=1; }
    prev=$b
done
[ "$fail" -eq 0 ] || { echo; echo "mapping is wrong."; exit 1; }
echo
echo "ok  no page is both writable and executable; a reserved encoding is refused"
echo "    separately from a W^X violation, so the caller learns which rule it hit"
