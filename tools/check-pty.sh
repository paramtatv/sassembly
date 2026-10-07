#!/bin/sh
# One erase deletes one akṣara — task `C-006`, doc 04 §3, decision D-11-B.
#
# ## What is being proved
#
# Doc 11 §3.2 moved the PTY and its line discipline out of the nucleus and into
# userspace (D-11-B), so `spec/pty.sas` asks the kernel for nothing. What it
# does ask is the one question doc 04 §3 says a Devanagari line discipline must
# answer: **erase must delete one akṣara, not one byte and not one codepoint.**
#
# An akṣara is several bytes and usually several codepoints. `नमस्ते` is
# `न म स ् त े` — 18 bytes, 6 codepoints, 3 akṣaras — and the last akṣara is
# `स्ते`, three codepoints joined by a virama. So a single backspace has three
# plausible wrong answers and exactly one right one:
#
#     17 bytes left   one BYTE went       — half of `त्`, the buffer is now
#                                           invalid UTF-8 and prints as garbage
#     15 bytes left   one CODEPOINT went  — `े` alone, leaving `नमस्त`, a
#                                           consonant dangling off the cluster
#      6 bytes left   one AKṢARA went     — `नम`                    ← required
#
# All three are named below, so a failure says WHICH rule ran rather than just
# that a number was wrong. This is the whole reason the test data is Devanagari:
# on ASCII the three rules agree, and an erase checked against ASCII has proved
# nothing at all about the property this row exists for.
#
# The program prints eight numbers and two lines of text:
#
#     0000000000000012   `नमस्ते` fed in            18 bytes
#     0000000000000003                               3 akṣaras
#     0000000000000006   after one backspace         6 bytes
#     0000000000000002                               2 akṣaras
#     नम                 the buffer itself
#     000000000000000f   `अक्षि` fed in             15 bytes
#     0000000000000002                               2 akṣaras
#     0000000000000003   after one backspace         3 bytes
#     0000000000000001                               1 akṣara
#     अ                  the buffer itself
#
# ## Why the before-numbers are checked too
#
# `W-087`'s failure mode is a check whose green light means "nothing happened".
# A line discipline that never appends anything erases correctly from an empty
# buffer every time. So the length BEFORE the backspace is printed and asserted:
# a program that fills nothing prints 0 where 18 is required and is caught at
# the first assertion, not passed at the third.
#
# ## Why two inputs
#
# One example cannot separate a computation from a constant. `अक्षि` puts the
# conjunct in a different shape — `अ | क्षि`, where the erased akṣara is
# `क ् ष ि`, four codepoints and twelve bytes — and demands a different answer
# (3, not 6). Both must hold.
#
# ## Why two link addresses
#
# Every address in the program is computed with `auipc` + `addi`: the buffer,
# the two inputs, the erase byte. A single run cannot tell a computed address
# from a lucky constant, so the same assertions run at 0x80200000 and
# 0x80400000.
set -eu

# Byte semantics for every text tool below, and NOT cosmetic. The most
# important failure this check has to describe — a byte-wise erase — leaves the
# buffer holding half of `त्`, and the program then echoes those bytes. In a
# UTF-8 locale macOS `tr` meets that, prints "Illegal byte sequence" and
# TRUNCATES the stream, so the evidence disappears at exactly the moment it is
# needed and the precise "one BYTE was erased" diagnosis degrades into "some
# lines are missing". Measured: the mutant reported 4 of 8 lines instead of 17
# bytes. Comparisons in `sh` are byte-wise regardless of locale, so nothing is
# lost by pinning it.
LC_ALL=C
export LC_ALL

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

run_at() {
    cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
        --स्थान "$1" "$root/spec/pty.sas" "$tmp/pty.elf" >/dev/null
    # Bounded by an explicit SIGKILL, not perl's alarm: QEMU handles SIGALRM
    # (its own timer subsystem) and survives it — see W-060. The erase walks
    # backwards over the buffer, and the way to get that wrong is a loop that
    # never reaches zero, so an unbounded read hangs this check.
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$tmp/pty.elf" > "$tmp/raw" 2>&1 &
    qpid=$!
    ( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone leaves its `sleep` orphaned
    # to init, still holding whatever it inherited (`W-096`). By PARENT pid,
    # never by pattern — `W-044`/`W-047`/`W-049`, and another agent's QEMU has
    # been seen live on this host.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    # QEMU's serial console ends every line with CR as well as LF, so
    # `grep -E '^[0-9a-f]{16}$'` matches NOTHING until the CR is gone. Same
    # trap as `tools/check-trap.sh`, same fix, and it must come before any
    # comparison — including the `नम` one, where a trailing CR is invisible in
    # the failure message.
    tr -d '\r' < "$tmp/raw" | tail -n 10 > "$tmp/out"
}

line() { sed -n "$1p" "$tmp/out"; }

# `$1` line number, `$2` required text, `$3` what that line means.
want() {
    got=$(line "$1")
    [ "$got" = "$2" ] && return 0
    echo "  FAIL  $addr -> $3: expected '$2', got '$got'"
    return 1
}

fail=0
for addr in ०षोड्८०२००००० ०षोड्८०४०००००; do
    run_at "$addr"
    # Where this run started, so the summary line at the bottom is printed only
    # when this address actually passed. A per-address "here is what happened"
    # line under a list of FAILs reads as a pass and is how a red check gets
    # skimmed as green.
    before=$fail

    # Absence is the failure mode that looks like success, so it is named first
    # and the serial output is shown rather than described.
    # `-a`: a guest that faults mid-print can leave a NUL in the serial capture,
    # and GNU grep then treats the whole file as binary and DECLINES it — the
    # count comes back 0 and the failure is reported as "printed nothing" rather
    # than as the fault it was. `-a` makes it read the bytes it was given.
    hex=$(grep -a -c -E '^[0-9a-f]{16}$' "$tmp/out" || true)
    if [ "$hex" -ne 8 ]; then
        echo "  FAIL  $addr -> $hex of the 8 required 16-digit lines printed."
        echo "        The last 10 lines of serial output were:"
        sed 's/^/          /' "$tmp/out"
        fail=1; continue
    fi

    # ---- नमस्ते: 18 bytes, 6 codepoints, 3 akṣaras -----------------------
    want 1 0000000000000012 "18 bytes should have reached the buffer" || fail=1
    want 2 0000000000000003 "नमस्ते is 3 akṣaras (न म स्ते)"          || fail=1

    got=$(line 3)
    case "$got" in
        0000000000000006) ;;
        0000000000000011)
            echo "  FAIL  $addr -> 17 bytes left: one BYTE was erased, cutting"
            echo "        त् in half and leaving the buffer invalid UTF-8"
            fail=1 ;;
        000000000000000f)
            echo "  FAIL  $addr -> 15 bytes left: one CODEPOINT was erased (े"
            echo "        alone), leaving नमस्त with the consonant dangling"
            fail=1 ;;
        *)
            echo "  FAIL  $addr -> erasing स्ते from नमस्ते must leave 6 bytes,"
            echo "        got '$got'"
            fail=1 ;;
    esac

    want 4 0000000000000002 "नम is 2 akṣaras"          || fail=1
    want 5 "नम"             "the buffer after erasing" || fail=1

    # ---- अक्षि: a different cluster shape, a different answer ------------
    want 6 000000000000000f "15 bytes should have reached the buffer" || fail=1
    want 7 0000000000000002 "अक्षि is 2 akṣaras (अ क्षि)"             || fail=1

    got=$(line 8)
    case "$got" in
        0000000000000003) ;;
        000000000000000e)
            echo "  FAIL  $addr -> 14 bytes left: one BYTE was erased from अक्षि"
            fail=1 ;;
        000000000000000c)
            echo "  FAIL  $addr -> 12 bytes left: one CODEPOINT was erased (ि"
            echo "        alone); the conjunct क्ष was not treated as one akṣara"
            fail=1 ;;
        *)
            echo "  FAIL  $addr -> erasing क्षि from अक्षि must leave 3 bytes,"
            echo "        got '$got'"
            fail=1 ;;
    esac

    want  9 0000000000000001 "अ is 1 akṣara"            || fail=1
    want 10 "अ"              "the buffer after erasing" || fail=1

    [ "$fail" -eq "$before" ] || continue
    printf "  %s -> नमस्ते 18B/3akṣ -> ^H -> 6B/2akṣ 'नम'; अक्षि 15B/2akṣ -> ^H -> 3B/1akṣ 'अ'\n" "$addr"
done

[ "$fail" -eq 0 ] || { echo; echo "backspace is not deleting a whole akṣara."; exit 1; }
echo
echo "ok  the userspace line discipline erases one AKṢARA per backspace: 12 bytes"
echo "    and 4 codepoints go for स्ते, 12 bytes and 4 codepoints for क्षि, and"
echo "    the residue is valid Devanagari both times. Neither the byte rule nor"
echo "    the codepoint rule produces these numbers, and both link addresses"
echo "    produce them, so the buffer and the inputs are computed rather than"
echo "    constants."
