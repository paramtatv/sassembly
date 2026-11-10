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
# FNV-1a-64 ITSELF, in awk, over the slot's bytes read out of the ELF at the
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
    "$root/tools/sadhana-native.sh" \
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
# The verifier is awk (POSIX; BWK and GNU alike): addresses are exact doubles, and the
# 64-bit digests are 16-digit hex strings and FNV-1a-64 runs on four 16-bit limbs.
cat > "$tmp/v.awk" <<'AWK'
# Verifier for tools/check-bootloader.sh. Input 1: the printed words, one 16-digit hex per line.
# Input 2: `od -An -v -tu1` of the ELF. Awk doubles are exact to 2^53, so addresses and
# lengths are numbers and every 64-bit digest stays a 16-digit hex string (never a number).
function hexn(s,   i, v) { v = 0; s = tolower(s)
    for (i = 1; i <= length(s); i++) v = v * 16 + index("0123456789abcdef", substr(s, i, 1)) - 1
    return v }
function tohex(n, w,   s) { s = ""
    if (n == 0) s = "0"
    while (n > 0) { s = substr("0123456789abcdef", n % 16 + 1, 1) s; n = int(n / 16) }
    while (length(s) < w) s = "0" s
    return s }
function hx(n) { return n < 0 ? "-0x" tohex(-n, 1) : "0x" tohex(n, 1) }
function xor8(a, b,   r, k) { r = 0
    for (k = 128; k >= 1; k = k / 2) {
        if ((a >= k) != (b >= k)) r += k
        if (a >= k) a -= k
        if (b >= k) b -= k }
    return r }
function le(o, n,   v, k) { v = 0
    for (k = n - 1; k >= 0; k--) v = v * 256 + B[o + k]
    return v }
# FNV-1a 64 over n octets of the ELF at offset o; four 16-bit limbs, prime = 2^40 + 0x1b3.
function fnv(o, n,   h0, h1, h2, h3, i, lo, low24, x, c, r0, r1, r2, r3) {
    h3 = 52210; h2 = 40164; h1 = 33826; h0 = 8997
    for (i = 0; i < n; i++) {
        lo = h0 % 256; h0 = h0 - lo + xor8(lo, B[o + i])
        low24 = (h0 + (h1 % 256) * 65536) * 256
        r0 = h0 * 435; c = int(r0 / 65536); r0 = r0 % 65536
        r1 = h1 * 435 + c; c = int(r1 / 65536); r1 = r1 % 65536
        r2 = h2 * 435 + c + low24 % 65536; c = int(r2 / 65536); r2 = r2 % 65536
        r3 = (h3 * 435 + c + int(low24 / 65536)) % 65536
        h0 = r0; h1 = r1; h2 = r2; h3 = r3 }
    return tohex(h3, 4) tohex(h2, 4) tohex(h1, 4) tohex(h0, 4) }
function die(m) { print m; fin = 1; exit 1 }
function addbad(m) { bad[nb++] = m }
function lst(a, n,   s, k) { s = "["
    for (k = 0; k < n; k++) s = s (k ? ", " : "") a[k]
    return s "]" }
function keys(set,   k, n, s) { n = 0; s = "["
    for (k = 0; k < 256; k++) if (k in set) { s = s (n ? ", " : "") k; n++ }
    return s "]" }
function cnt(sc, kind, slot,   i, n) { n = 0
    for (i = 0; i < ne; i++) if (S[i] == sc && K[i] == kind && (slot == "" || SL[i] == slot)) n++
    return n }
function image(va, n,   k) {
    for (k = 0; k < ns; k++) if (SV[k] <= va && va + n <= SV[k] + SF[k]) return SO[k] + (va - SV[k])
    return -1 }
BEGIN {
    if (MODE != "bases") next_mode = 1
    else {
        split(PREV, p, " "); split(CUR, q, " ")
        d = 2097152   # 0x80400000 - 0x80200000
        da = hexn(q[1]) - hexn(p[1]); db = hexn(q[2]) - hexn(p[2])
        if (da != d || db != d) {
            print "  FAIL  the slot bases moved by " hx(da) " and " hx(db) ", expected " hx(d) " each."
            print "        A base that does not move is a constant, and a loader printing"
            print "        constants would pass every other assertion here."
            rcb = 1 }
        bmode = 1; exit rcb }
}
FILENAME == WF { if ($0 ~ /[^ \t]/) { gsub(/[ \t\r]/, ""); W[nw++] = $0 } ; next }
{ for (f = 1; f <= NF; f++) B[nb_++] = $f + 0 }
END {
    if (bmode) exit rcb
    MUDRA = 0; STHANA = 11; ARAMBHA = 10; VICHARA = 1; KSHAYA = 2; APARIKSHA = 3
    PRATYAVARTANA = 4; AVATARANA = 5; PRAVESHA = 6; MARKA = 7; ABHUTYA = 8; PUNARARAMBHA = 9
    NAME[0] = "मुद्रा"; NAME[11] = "स्थान"; NAME[10] = "प्रारम्भ"; NAME[1] = "विचार"
    NAME[2] = "क्षय"; NAME[3] = "अपरीक्षा"; NAME[4] = "प्रत्यावर्तन"
    NAME[5] = "अवतरण"; NAME[6] = "प्रवेश"; NAME[7] = "मार्क"
    NAME[8] = "अबूट्य"; NAME[9] = "पुनरारम्भ"
    PAY[11] = 2; PAY[0] = 1; PAY[3] = 1; PAY[9] = 1
    SLOTN[0] = "क"; SLOTN[1] = "ख"; SLOTN[255] = "-"
    nb = 0
    if (nw == 0) die("nothing printed at all. The image did not reach its first ecall, or it\nshut down before printing. Bisect: print at the very first instruction;\nif that line appears, the program started and the fault is later.")
    ne = 0; i = 0
    while (i < nw) {
        w = W[i]; i++
        kind = hexn(substr(w, 1, 2)); sc = hexn(substr(w, 3, 2)); slot = hexn(substr(w, 5, 2)); cn = hexn(substr(w, 7, 2))
        if (kind > 11) die("word " (i - 1) " is " w ": kind 0x" tohex(kind, 2) " is not an event this check knows")
        if (substr(w, 9, 8) != "00000000") die("word " (i - 1) " is " w ": an event word's low 32 bits must be zero, so\nthe stream is out of step — a payload word was read as an event")
        n = (kind in PAY) ? PAY[kind] : 0
        if (i + n > nw) die("the stream ends inside a " NAME[kind] " event, which owes " n " more word(s).\nThe machine was killed mid-print, or a slot hung with the watchdog unarmed.")
        K[ne] = kind; S[ne] = sc; SL[ne] = slot; C[ne] = cn; P1[ne] = W[i]; P2[ne] = W[i + 1]; ne++
        i += n }
    # a scenario that never started has not failed its properties — it has not been tested
    maxran = 0
    for (e = 0; e < ne; e++) if (S[e]) { ran[S[e]] = 1; if (S[e] > maxran) maxran = S[e] }
    nm = 0
    for (s = 1; s <= 3; s++) if (!(s in ran)) miss[nm++] = s
    if (nm) addbad("scenario(s) " lst(miss, nm) " never ran; the machine was still in scenario " maxran " when this check killed it, so nothing below that point was exercised. Whatever went wrong there comes first")
    # the recorded digests, and the ELF they claim to be over
    phoff = le(32, 8); phentsize = le(54, 2); phnum = le(56, 2); ns = 0
    for (k = 0; k < phnum; k++) {
        o = phoff + k * phentsize
        if (le(o, 4) == 1) { SO[ns] = le(o + 8, 8); SV[ns] = le(o + 16, 8); SF[ns] = le(o + 32, 8); ns++ } }
    for (e = 0; e < ne; e++) {
        if (K[e] == STHANA) { LOC[SL[e]] = 1; LB[SL[e]] = hexn(P1[e]); LN[SL[e]] = hexn(P2[e]) }
        if (K[e] == MUDRA) { REC[SL[e]] = 1; RD[SL[e]] = P1[e] } }
    if (keys(LOC) != "[0, 1]" || keys(REC) != "[0, 1]")
        die("the loader located slots " keys(LOC) " and recorded digests for " keys(REC) "; both must cover exactly क and ख")
    for (s = 0; s <= 1; s++) {
        base = LB[s]; ln = LN[s]
        if (ln == 0) { addbad("slot " SLOTN[s] " has length 0, so its digest is over nothing"); continue }
        o = image(base, ln)
        if (o < 0) { addbad("slot " SLOTN[s] " claims " hx(base) "+" ln ", which is not inside any loadable segment of the ELF — the loader is digesting memory that was never part of the image"); continue }
        want = fnv(o, ln)
        if (want != RD[s]) addbad("slot " SLOTN[s] ": the loader recorded " RD[s] ", but FNV-1a-64 over the " ln " octets at " hx(base) " IN THE ELF is " want ". The digest is not a function of the image it names, so every comparison the loader makes is comparing something else") }
    if (RD[0] == RD[1]) addbad("both slots recorded the digest " RD[0] ", and their bytes differ — so the digest does not depend on its input and any slot would verify against any recording")
    # scenario 1: the kernel hangs
    if (1 in ran) {
        ke = 0; kl = 0; nr = 0
        for (e = 0; e < ne; e++) if (S[e] == 1) {
            if (K[e] == PRAVESHA && SL[e] == 0) ke++
            if (K[e] == AVATARANA && SL[e] == 0) { KLC[kl++] = C[e] }
            if (K[e] == PUNARARAMBHA) RSC[nr++] = P1[e] }
        if (ke != 3) addbad("scenario 1: slot क was entered " ke " times, expected 3 — the budget is three attempts and क hangs every time, so it must consume all three and no more")
        else if (lst(KLC, kl) != "[2, 1, 0]") addbad("scenario 1: the counter written before each of क's three boots was " lst(KLC, kl) ", expected [2, 1, 0]. क never returns, so a counter that does not fall means the loader wrote it somewhere the hung kernel had to reach — or rewrote the state on the boot path, which counts nothing")
        for (j = 0; j < kl; j++) {
            ok = 0
            for (e = 0; e < ne; e++) if (S[e] == 1 && K[e] == PRAVESHA && SL[e] == 0 && C[e] == KLC[j]) ok = 1
            if (!ok) addbad("scenario 1: the loader wrote counter " KLC[j] " for क but क never ran — the counter moved without a boot behind it") }
        for (e = 0; e < ne; e++) if (S[e] == 1 && K[e] == MARKA && SL[e] == 0) { addbad("scenario 1: slot क marked itself good, but क hangs. Whatever ran, it was not the slot this check is about"); break }
        if (nr != 3) addbad("scenario 1: the watchdog fired " nr " times, expected 3 — one per hung boot. A hang that is not caught is a brick")
        for (j = 0; j < nr; j++) if (RSC[j] != "8000000000000005") addbad("scenario 1: a reset was taken with scause " RSC[j] ", expected 8000000000000005 (interrupt, cause 5, the clock). Something other than the watchdog ended that boot")
        jj = -1
        for (e = 0; e < ne; e++) if (S[e] == 1 && K[e] == KSHAYA && SL[e] == 0) { jj = e; break }
        if (jj < 0) addbad("scenario 1: क's budget never ran out, so the fallback was never reached")
        else {
            t4 = 0; t6 = 0; t7 = 0
            for (e = jj; e < ne; e++) if (S[e] == 1) {
                if (K[e] == PRATYAVARTANA && SL[e] == 1) t4 = 1
                if (K[e] == PRAVESHA && SL[e] == 1) t6 = 1
                if (K[e] == MARKA && SL[e] == 1) t7 = 1 }
            if (!t4) addbad("scenario 1: क was exhausted and the loader did not fall back to ख")
            else if (!t6) addbad("scenario 1: the loader fell back to ख and never entered it. Counting to zero is not a fallback; the other slot has to run")
            else if (!t7) addbad("scenario 1: ख was entered and never marked itself good, so the machine did not finish booting (doc 16 §4.3 step 6)") } }
    # scenario 2: क does not verify
    r2 = -1
    if (2 in ran) {
        for (e = 0; e < ne; e++) if (S[e] == 2 && K[e] == APARIKSHA && SL[e] == 0) { r2 = e; break }
        if (r2 < 0) addbad("scenario 2: slot क was corrupted and the loader did not refuse it")
        else if (P1[r2] == RD[0]) addbad("scenario 2: क was refused, but the digest it computed (" P1[r2] ") equals the recorded one — the refusal was not caused by the content")
        if (cnt(2, PRAVESHA, 0)) addbad("scenario 2: SLOT क RAN. Its digest did not match and the loader entered it anyway. A loader that verifies and boots regardless is worse than one that does not verify, because it reports a safety it does not provide")
        if (cnt(2, AVATARANA, 0)) addbad("scenario 2: the loader spent one of क's attempts on a slot it refused. Nothing was attempted, and the count is of attempts")
        if (!cnt(2, MARKA, 1)) addbad("scenario 2: ख never marked itself good — refusing क has to leave the machine booted, not stopped") }
    # scenario 3: neither slot verifies
    if (3 in ran) {
        last = -1
        for (e = 0; e < ne; e++) if (S[e] == 3) { last = e; if (K[e] == APARIKSHA) { R3[SL[e]] = 1; R3D[SL[e]] = P1[e] } }
        if (keys(R3) != "[0, 1]") addbad("scenario 3: both slots are corrupt; the loader refused " keys(R3))
        if (cnt(3, PRAVESHA, "")) addbad("scenario 3: a slot RAN with neither slot verifying. This is the case that separates a loader which refuses from one which merely reports")
        if (!cnt(3, ABHUTYA, "")) addbad("scenario 3: no slot verified and the loader did not say so. Stopping with a reason is the correct end here; silence is not")
        else if (K[last] != ABHUTYA) addbad("scenario 3: the loader carried on after declaring nothing bootable")
        if ((0 in R3) && r2 >= 0 && R3D[0] != P1[r2]) addbad("क's digest came out " P1[r2] " in scenario 2 and " R3D[0] " in scenario 3 from identical bytes — the digest is not a function of content alone") }
    if (nb) { for (k = 0; k < nb; k++) print "  " bad[k]; exit 1 }
    print RD[0], RD[1] > MACH
    print tohex(LB[0], 1), tohex(LB[1], 1) > MACH
    print "क hung 3 times and burnt 3 attempts, ख booted and marked good; क refused unverified and never entered; neither verifying, nothing entered at all"
}
AWK
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr" > "$tmp/l"
    od -An -v -tu1 "$tmp/b.elf" > "$tmp/e.od"
    if ! awk -v WF="$tmp/l" -v MACH="$tmp/m" -f "$tmp/v.awk" "$tmp/l" "$tmp/e.od" > "$tmp/v" 2>&1
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
        awk -v MODE=bases -v PREV="$prev_base" -v CUR="$base" -f "$tmp/v.awk" || fail=1
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
