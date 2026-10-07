#!/bin/sh
# The bootloader `आरम्भकः` — task `E-004`, doc 16 §4.3, doc 11 §4.1.
#
# A/B slots, a boot counter and slot verification exist for ONE reason: an
# update must be able to fail without bricking the machine. So what is checked
# here is not that a machine boots. It is that the three ways a boot can go
# wrong each end somewhere survivable.
#
# ## The three properties, and why each is written the way it is
#
# 1. THE COUNTER IS SPENT BEFORE THE KERNEL IS TRUSTED (R-16-3). A kernel that
#    hangs must still burn its attempt — otherwise hanging is free and the
#    machine returns to the same slot forever. The only way to prove this is a
#    kernel that genuinely never comes back, so slot क really does hang, and the
#    only exit is the watchdog. It is entered three times and its counter reads
#    2, 1, 0 across three SEPARATE boots. A loader that wrote the counter once
#    the kernel had proved itself would loop on क forever, because क never gets
#    that far — which is precisely the brick this mechanism exists to prevent.
#
# 2. THE FALLBACK IS EXERCISED, NOT COUNTED. Reaching zero proves nothing; the
#    other slot has to actually run. After क's third hang, ख is entered and
#    calls मार्कम् (§4.3 step 6) — the machine boots.
#
# 3. A SLOT THAT DOES NOT VERIFY IS REFUSED, NOT BOOTED. This is the property a
#    loader most easily reports without providing: verifying and then jumping
#    anyway is WORSE than not verifying, because it claims a safety it does not
#    have. So the check does not look for a verification step; it looks for the
#    ABSENCE of the jump. In scenario 2, slot क's digest does not match and क's
#    own entry line never appears. In scenario 3 neither slot matches and NO
#    slot is entered at all — the loader stops instead.
#
#    The corruption is one octet in a word past the end of each slot's reachable
#    code, chosen so that a loader which booted the slot anyway would run it
#    normally. The failure would therefore look exactly like a healthy boot,
#    which is the point: nothing but the missing entry line distinguishes them.
#
# ## The digest is checked against the ELF, not taken on trust
#
# `अङ्कनम्` is FNV-1a 64 — a content digest, NOT a signature; doc 16 §3.2 wants
# Ed25519 over SHA-256 against a root key embedded per R-16-2, and that is task
# 16.3.2/16.6.2, not this row. What is proven here is independent of the digest
# chosen: on mismatch, control is not transferred.
#
# But a digest routine that returned a constant would satisfy every comparison
# in the program and the program could not tell. So this check recomputes
# FNV-1a-64 ITSELF, in Python, over the slot's bytes read out of the ELF at the
# address and length the loader printed, and requires the two to agree.
#
# ## What moves and what does not
#
# The slot base addresses are DERIVED from where the image was linked and must
# move by exactly the link delta. The digests must NOT move — a content digest
# names the image, not where it sits, and that is the whole property doc 16 §4.2
# rests कोशागारम् on. Everything else — the event words, the counters, scause —
# is asserted at each address on its own, so it is pinned twice over and is not
# compared again across the two; that comparison could not fail and was removed.
#
# NEITHER HALF OF THAT COMPARISON CAN FAIL TODAY — not the digests-must-not-move
# half and not the bases-must-move half — and both are marked as guards where
# they are written. The ELF cross-check is what actually carries this weight: it
# is what binds a digest to real octets, and it is what kills a constant base,
# because a base that does not move is not inside the second run's loadable
# segment. The two-link-address comparison is a backstop for a future in which a
# slot holds a linked kernel with absolute addresses in it, and only then does
# either half become able to fire.
#
# Nothing else here is unfalsifiable: every other assertion has been watched to
# fire under a mutation.
#
# ## What this row does NOT prove — read this before believing the counter
#
# The state survives a WATCHDOG RESET: the timer interrupt fires in a hung
# kernel, the handler re-enters the loader from its first instruction, and the
# loader does not reinitialise the counter on that path. That last part is a
# real and commonly-broken property, and it is what is tested here.
#
# It does NOT survive a power cycle. The state lives in RAM, because this
# machine offers no non-volatile store this image can write: that needs CFI NOR
# programming on `-drive if=pflash` or a virtio-blk driver, and either is a
# device driver, not a bootloader. Until one exists, "the counter is durable" is
# unproven and is deliberately not claimed. See the report for `E-004`.
set -eu
root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/bootloader.sas" "$tmp/b.elf" >/dev/null
    # SIGKILL, not perl's alarm — QEMU handles SIGALRM (W-060). The bound has to
    # exist: slot क hangs by design and only the guest's own watchdog gets out of
    # it, so a loader that never arms the timer spins here until this kills it.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/b.elf" > "$tmp/out" 2>&1 &
    qpid=$!; ( sleep 25; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host. Ordered after the `wait`: the sleep dying
    # lets the subshell run its kill, a no-op only because the process it
    # would kill has already been reaped.
    wait "$qpid" 2>/dev/null || true; pkill -P "$watcher" 2>/dev/null || true; kill "$watcher" 2>/dev/null || true
    # -nographic ends every line with CR; an anchored grep would match nothing.
    # `|| true` is for the EMPTY case only: a program that printed nothing is a
    # verdict this check reports, not a shell error. An assembler failure is not
    # swallowed — cargo's own status aborts the script above, with its message.
    tr -d '\r' < "$tmp/out" | grep -aE '^[0-9a-f]{16}$' || true
}

fail=0; prev_inv=""; prev_base=""
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    if ! python3 - "$tmp/l" "$tmp/b.elf" "$tmp/m" > "$tmp/v" 2>&1 <<'PY'
import struct, sys

words_path, elf_path, machine_path = sys.argv[1], sys.argv[2], sys.argv[3]
words = [l.strip() for l in open(words_path) if l.strip()]

# ---- the event stream ---------------------------------------------------
# [63:56] kind  [55:48] scenario  [47:40] slot (0=क, 1=ख, 0xff=none)
# [39:32] the slot's remaining attempts  [31:0] zero.
# A kind says how many payload words follow it, so the parse is positional and
# a payload word can never be mistaken for an event.
MUDRA, STHANA = 0x00, 0x0b          # digest recorded / slot located
ARAMBHA = 0x0a                      # power-on for a scenario
VICHARA, KSHAYA, APARIKSHA = 1, 2, 3    # considering / out of attempts / refused
PRATYAVARTANA, AVATARANA = 4, 5     # fall back to the other slot / counter written, jumping
PRAVESHA, MARKA = 6, 7              # the SLOT'S OWN first instruction / it marked itself good
ABHUTYA, PUNARARAMBHA = 8, 9        # nothing bootable / the watchdog reset us
PAYLOAD = {STHANA: 2, MUDRA: 1, APARIKSHA: 1, PUNARARAMBHA: 1}
NAME = {MUDRA: "मुद्रा", STHANA: "स्थान", ARAMBHA: "प्रारम्भ", VICHARA: "विचार",
        KSHAYA: "क्षय", APARIKSHA: "अपरीक्षा", PRATYAVARTANA: "प्रत्यावर्तन",
        AVATARANA: "अवतरण", PRAVESHA: "प्रवेश", MARKA: "मार्क",
        ABHUTYA: "अबूट्य", PUNARARAMBHA: "पुनरारम्भ"}
SLOT = {0: "क", 1: "ख", 0xff: "-"}

bad = []
def die(msg):
    print(msg)
    sys.exit(1)

if not words:
    die("nothing printed at all. The image did not reach its first ecall, or it\n"
        "shut down before printing. Bisect: print at the very first instruction;\n"
        "if that line appears, the program started and the fault is later.")

ev, i = [], 0
while i < len(words):
    w = int(words[i], 16); i += 1
    kind, sc, slot, cnt = w >> 56, (w >> 48) & 0xff, (w >> 40) & 0xff, (w >> 32) & 0xff
    if kind not in PAYLOAD and kind not in NAME:
        die(f"word {i-1} is {w:016x}: kind {kind:#04x} is not an event this check knows")
    if w & 0xffffffff:
        die(f"word {i-1} is {w:016x}: an event word's low 32 bits must be zero, so\n"
            f"the stream is out of step — a payload word was read as an event")
    n = PAYLOAD.get(kind, 0)
    if i + n > len(words):
        die(f"the stream ends inside a {NAME[kind]} event, which owes {n} more word(s).\n"
            "The machine was killed mid-print, or a slot hung with the watchdog unarmed.")
    ev.append((kind, sc, slot, cnt, [int(x, 16) for x in words[i:i+n]]))
    i += n

def of(sc, *kinds):
    return [e for e in ev if e[1] == sc and (not kinds or e[0] in kinds)]

# A scenario that never started has not failed its properties — it has not been
# tested. Saying "the loader did not refuse क" about a scenario the machine never
# reached is a false sentence, and a false sentence in a diagnostic sends the
# reader to the wrong place. So the missing scenarios are named once, and their
# assertions are skipped rather than reported as failures.
ran = sorted({e[1] for e in ev if e[1]})
missing = [s for s in (1, 2, 3) if s not in ran]
if missing:
    bad.append(f"scenario(s) {missing} never ran; the machine was still in scenario "
               f"{max(ran) if ran else 0} when this check killed it, so nothing below "
               "that point was exercised. Whatever went wrong there comes first")

# ---- the recorded digests, and the ELF they claim to be over -------------
b = open(elf_path, 'rb').read()
phoff, = struct.unpack_from('<Q', b, 0x20)
phentsize, phnum = struct.unpack_from('<HH', b, 0x36)
segs = []
for k in range(phnum):
    typ, _fl, off, va, _pa, fsz, _msz, _al = struct.unpack_from('<IIQQQQQQ', b, phoff + k*phentsize)
    if typ == 1:
        segs.append((va, off, fsz))

def image(va, n):
    for v, o, f in segs:
        if v <= va and va + n <= v + f:
            return b[o + (va - v): o + (va - v) + n]
    return None

def fnv1a64(bs):
    h = 0xcbf29ce484222325
    for x in bs:
        h ^= x
        h = (h * 0x100000001b3) & 0xffffffffffffffff
    return h

located = {e[2]: e[4] for e in ev if e[0] == STHANA}
recorded = {e[2]: e[4][0] for e in ev if e[0] == MUDRA}
if sorted(located) != [0, 1] or sorted(recorded) != [0, 1]:
    die(f"the loader located slots {sorted(located)} and recorded digests for "
        f"{sorted(recorded)}; both must cover exactly क and ख")

for s in (0, 1):
    base, ln = located[s]
    if ln == 0:
        bad.append(f"slot {SLOT[s]} has length 0, so its digest is over nothing")
        continue
    img = image(base, ln)
    if img is None:
        bad.append(f"slot {SLOT[s]} claims {base:#x}+{ln}, which is not inside any "
                   "loadable segment of the ELF — the loader is digesting memory that "
                   "was never part of the image")
        continue
    want = fnv1a64(img)
    if want != recorded[s]:
        bad.append(f"slot {SLOT[s]}: the loader recorded {recorded[s]:016x}, but FNV-1a-64 "
                   f"over the {ln} octets at {base:#x} IN THE ELF is {want:016x}. The digest "
                   "is not a function of the image it names, so every comparison the "
                   "loader makes is comparing something else")

if recorded.get(0) == recorded.get(1):
    bad.append(f"both slots recorded the digest {recorded.get(0):016x}, and their bytes "
               "differ — so the digest does not depend on its input and any slot would "
               "verify against any recording")

# ---- scenario 1: the kernel hangs ---------------------------------------
if 1 in ran:
 enters1 = [e for e in of(1, PRAVESHA)]
 launch1 = [e for e in of(1, AVATARANA)]
 k_enters = [e for e in enters1 if e[2] == 0]
 k_launch = [e for e in launch1 if e[2] == 0]
 resets = of(1, PUNARARAMBHA)

 if len(k_enters) != 3:
    bad.append(f"scenario 1: slot क was entered {len(k_enters)} times, expected 3 — the "
               "budget is three attempts and क hangs every time, so it must consume all "
               "three and no more")
 elif [e[3] for e in k_launch] != [2, 1, 0]:
    bad.append(f"scenario 1: the counter written before each of क's three boots was "
               f"{[e[3] for e in k_launch]}, expected [2, 1, 0]. क never returns, so a "
               "counter that does not fall means the loader wrote it somewhere the hung "
               "kernel had to reach — or rewrote the state on the boot path, which counts "
               "nothing")
 for e in k_launch:
    if not any(x[0] == PRAVESHA and x[2] == 0 and x[3] == e[3] for x in enters1):
        bad.append(f"scenario 1: the loader wrote counter {e[3]} for क but क never ran — "
                   "the counter moved without a boot behind it")
 if any(e[0] == MARKA and e[2] == 0 for e in of(1)):
    bad.append("scenario 1: slot क marked itself good, but क hangs. Whatever ran, it was "
               "not the slot this check is about")
 if len(resets) != 3:
    bad.append(f"scenario 1: the watchdog fired {len(resets)} times, expected 3 — one per "
               "hung boot. A hang that is not caught is a brick")
 for e in resets:
    if e[4][0] != 0x8000000000000005:
        bad.append(f"scenario 1: a reset was taken with scause {e[4][0]:016x}, expected "
                   "8000000000000005 (interrupt, cause 5, the clock). Something other than "
                   "the watchdog ended that boot")

 seq1 = [(e[0], e[2]) for e in of(1)]
 try:
    j = seq1.index((KSHAYA, 0))
 except ValueError:
    bad.append("scenario 1: क's budget never ran out, so the fallback was never reached")
 else:
    tail = seq1[j:]
    if (PRATYAVARTANA, 1) not in tail:
        bad.append("scenario 1: क was exhausted and the loader did not fall back to ख")
    elif (PRAVESHA, 1) not in tail:
        bad.append("scenario 1: the loader fell back to ख and never entered it. Counting to "
                   "zero is not a fallback; the other slot has to run")
    elif (MARKA, 1) not in tail:
        bad.append("scenario 1: ख was entered and never marked itself good, so the machine "
                   "did not finish booting (doc 16 §4.3 step 6)")

# ---- scenario 2: क does not verify --------------------------------------
ref2 = []
if 2 in ran:
 ref2 = [e for e in of(2, APARIKSHA) if e[2] == 0]
 if not ref2:
    bad.append("scenario 2: slot क was corrupted and the loader did not refuse it")
 else:
    got = ref2[0][4][0]
    if got == recorded[0]:
        bad.append(f"scenario 2: क was refused, but the digest it computed ({got:016x}) "
                   "equals the recorded one — the refusal was not caused by the content")
 if any(e[2] == 0 for e in of(2, PRAVESHA)):
    bad.append("scenario 2: SLOT क RAN. Its digest did not match and the loader entered it "
               "anyway. A loader that verifies and boots regardless is worse than one that "
               "does not verify, because it reports a safety it does not provide")
 if any(e[2] == 0 for e in of(2, AVATARANA)):
    bad.append("scenario 2: the loader spent one of क's attempts on a slot it refused. "
               "Nothing was attempted, and the count is of attempts")
 if not any(e[2] == 1 and e[0] == MARKA for e in of(2)):
    bad.append("scenario 2: ख never marked itself good — refusing क has to leave the "
               "machine booted, not stopped")

# ---- scenario 3: neither slot verifies ----------------------------------
if 3 in ran:
 ref3 = {e[2]: e[4][0] for e in of(3, APARIKSHA)}
 if sorted(ref3) != [0, 1]:
    bad.append(f"scenario 3: both slots are corrupt; the loader refused {sorted(ref3)}")
 if of(3, PRAVESHA):
    bad.append("scenario 3: a slot RAN with neither slot verifying. This is the case that "
               "separates a loader which refuses from one which merely reports")
 if not of(3, ABHUTYA):
    bad.append("scenario 3: no slot verified and the loader did not say so. Stopping with a "
               "reason is the correct end here; silence is not")
 elif of(3)[-1][0] != ABHUTYA:
    bad.append("scenario 3: the loader carried on after declaring nothing bootable")
 if 0 in ref3 and ref2 and ref3[0] != ref2[0][4][0]:
    bad.append(f"क's digest came out {ref2[0][4][0]:016x} in scenario 2 and {ref3[0]:016x} "
               "in scenario 3 from identical bytes — the digest is not a function of "
               "content alone")

if bad:
    print("\n".join("  " + x for x in bad))
    sys.exit(1)

# What must be identical between link addresses, and what must move.
#
# Only the digests are carried across. The lengths and the event stream were here
# too and were removed: every field of them is already pinned by the per-address
# assertions above, so comparing them again could not fail — coverage-shaped and
# empty. See the note above the comparison for why the digest line stays.
with open(machine_path, 'w') as f:
    f.write("%016x %016x\n" % (recorded[0], recorded[1]))
    f.write("%x %x\n" % (located[0][0], located[1][0]))

print("क hung 3 times and burnt 3 attempts, ख booted and marked good; क refused "
      "unverified and never entered; neither verifying, nothing entered at all")
PY
    then sed 's/^/  FAIL  /' "$tmp/v"; fail=1; continue; fi

    printf "  %s -> %s\n" "$addr" "$(tail -1 "$tmp/v")"
    inv=$(sed -n 1p "$tmp/m"); base=$(sed -n 2p "$tmp/m")

    if [ -n "$prev_inv" ]; then
        # BOTH ASSERTIONS BELOW ARE GUARDS: neither can fail today, and that is
        # recorded rather than hidden. Digests first.
        #
        # A slot image here is pure instructions, and this assembler's
        # `स्थानसापेक्षयोगः` pairs are PC-relative, so a code-only image assembles
        # to the same octets at every link address and its digest is position-
        # independent by construction. No mutation of the program can move it —
        # one that seeds the digest with the base is killed by the ELF
        # cross-check above, at the first address, before this line is reached.
        #
        # It stays because a real slot holds a LINKED KERNEL, whose image
        # contains absolute addresses; the day a slot carries data this is the
        # only assertion that catches the digest becoming a function of where
        # the image was placed, which would defeat A/B and कोशागारम् both. If
        # that day does not come, delete this — it is a guard, not coverage.
        if [ "$inv" != "$prev_inv" ]; then
            echo "  FAIL  a slot digest changed between link addresses. A content digest"
            echo "        names the image, not where it sits; one that moves is over"
            echo "        addresses and cannot survive a relocation — which is exactly"
            echo "        what an A/B slot is."
            printf '        %s\n' "$prev_inv" "$inv"
            fail=1
        fi
        # The bases are derived from the link address and must move with it —
        # and THIS ONE CANNOT FAIL TODAY EITHER, for the same reason and by the
        # same stronger check. A base is produced by `स्थानसापेक्षयोगः`, so
        # making it constant takes a literal, and a literal base is not inside
        # the second run's loadable segment: the ELF cross-check rejects it at
        # that address with "not inside any loadable segment" before this line
        # is reached. So the ELF cross-check is what enforces "derived, not a
        # constant", and this is the backstop that starts mattering when a slot
        # image is a linked kernel rather than the position-independent code it
        # is now. Guard, not coverage — the same status as the digest above.
        python3 - "$prev_base" "$base" <<'PYB' || fail=1
import sys
pa, pb = (int(x, 16) for x in sys.argv[1].split())
qa, qb = (int(x, 16) for x in sys.argv[2].split())
d = 0x80400000 - 0x80200000
if (qa - pa, qb - pb) != (d, d):
    print(f"  FAIL  the slot bases moved by {qa-pa:#x} and {qb-pb:#x}, expected {d:#x} each."
          "\n        A base that does not move is a constant, and a loader printing"
          "\n        constants would pass every other assertion here.")
    sys.exit(1)
PYB
    fi
    prev_inv=$inv; prev_base=$base
done

[ "$fail" -eq 0 ] || { echo; echo "the bootloader is wrong."; exit 1; }
echo
echo "ok  E-004: the boot counter is spent BEFORE the kernel is trusted — a slot"
echo "    that hangs still burns all three attempts, and only the watchdog gets"
echo "    out of it; the loader then falls back and the other slot actually BOOTS"
echo "    and marks itself good. A slot whose digest does not match is refused and"
echo "    never entered, and when neither matches nothing is entered at all. The"
echo "    digests are recomputed here from the ELF, so they are over the real"
echo "    image; they do not move between link addresses and the slot bases do."
