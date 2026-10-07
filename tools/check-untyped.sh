#!/bin/sh
# Untyped retype — task `C-002b`, doc 11 §3.1, phase 11.2.2.
#
# The nucleus never allocates. A process donates its own `अव्यक्तम्` and objects
# are carved from it, so kernel memory cannot be exhausted and quotas are
# structural rather than policed: whoever asks, pays.
#
# ## Three properties, and the third is the quiet one
#
#   * Objects do not overlap. The watermark only moves forward, so overlap is
#     impossible by construction rather than by checking.
#
#   * A request past the end is REFUSED, not trimmed. Trimming hands back an
#     object smaller than asked for without saying so, and the caller writes it
#     at full size — over its neighbour.
#
#   * A REFUSAL COSTS NOTHING. If a failed request still advanced the watermark,
#     the region would shrink on every denial with no count ever looking wrong.
#     That is a leak, and nothing shows it at the moment it happens.
#
# The third is checked directly: after the refused 2048, the next 1024 must land
# exactly where it would have landed had the refusal never occurred.
#
# The answer is a pair — verdict, then address — for the reason `C-002a` found:
# 0 alone reads both as a valid answer and as a denial.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/untyped.sas" "$tmp/u.elf" >/dev/null
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/u.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' | tail -13
}

fail=0; prev_base=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    n=$(wc -l < "$tmp/l" | tr -d ' ')
    if [ "$n" -ne 13 ]; then
        echo "  FAIL  $addr printed $n hex lines, expected 13 (the region base, then"
        echo "        a verdict and an address for each of six requests)"
        fail=1; continue
    fi
    if ! python3 - "$tmp/l" > "$tmp/v" 2>&1 <<'PY'
import sys
v = [int(l, 16) for l in open(sys.argv[1]) if l.strip()]
base, rest = v[0], v[1:]
pairs = [(rest[i], rest[i+1]) for i in range(0, 12, 2)]
K, REGION = 1024, 4096
asked = [K, K, K, 2 * K, K, 8]
want_ok = [True, True, True, False, True, False]
bad = []

if base % REGION:
    bad.append(f"the region base {base:016x} is not {REGION}-aligned")

for i, ((verdict, got), ok, a) in enumerate(zip(pairs, want_ok, asked)):
    if ok and verdict != 1:
        bad.append(f"request {i} for {a} octets fits in the region and was REFUSED — "
                   "nothing could ever be created")
    if not ok and verdict != 0:
        bad.append(f"request {i} for {a} octets does not fit and was GRANTED at "
                   f"{got:016x}. If it was trimmed, the caller will write {a} octets "
                   "over whatever follows and nothing will have said so")
    if verdict == 0 and got != 0:
        bad.append(f"request {i} was refused but returned {got:016x} — a refusal must "
                   "carry no address")

granted = [(i, g) for i, ((vd, g), ok) in enumerate(zip(pairs, want_ok)) if vd == 1]
for n, (i, g) in enumerate(granted):
    if not (base <= g < base + REGION):
        bad.append(f"object {i} at {g:016x} is outside the donated region "
                   f"{base:016x}..{base+REGION:016x} — it was never given that memory")
    if g != base + n * K:
        bad.append(f"object {i} is at {g:016x}, expected {base + n*K:016x} — the "
                   "objects are not laid end to end, so two of them share octets")

# The quiet property: the refusal at index 3 must have cost nothing.
if len(granted) >= 4:
    after = granted[3][1]
    if after != base + 3 * K:
        bad.append(f"after the refused {2*K}-octet request the next object landed at "
                   f"{after:016x}, not {base + 3*K:016x} — the DENIAL MOVED THE "
                   "WATERMARK, so the region shrinks on every refusal and no count "
                   "ever looks wrong")
if bad:
    print("\n".join(bad)); sys.exit(1)
print(f"region {base:016x}: four objects laid end to end, two requests refused with "
      f"no address, and the refusal cost nothing — the next object resumed at "
      f"{base + 3*K:016x}")
PY
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi
    printf "  %s -> %s\n" "$addr" "$(cat "$tmp/v")"
    b=$(sed -n 1p "$tmp/l")
    [ -n "$prev_base" ] && [ "$b" = "$prev_base" ] && { echo "  FAIL  region base $b at both link addresses — a constant"; fail=1; }
    prev_base=$b
done
[ "$fail" -eq 0 ] || { echo; echo "untyped retype is wrong."; exit 1; }
echo
echo "ok  objects are carved from donated memory end to end and cannot overlap;"
echo "    a request past the end is refused whole rather than trimmed; and a"
echo "    refusal costs nothing, so denials cannot leak the region away"
