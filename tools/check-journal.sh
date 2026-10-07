#!/bin/sh
# An UNCOMMITTED journal record is discarded; a COMMITTED one is replayed.
# Row `C-005`, doc 08 §2.3 — one slice of it, named honestly below.
#
# ## What is being proved, and what is NOT
#
# `C-005` reads "VFS + journaled filesystem". This check does not prove that row.
# There is no VFS here: no paths, no directories, no names, no stat, no rename.
# What is proved is the one property without which none of that is *journaled*,
# and which cannot be established by inspection:
#
#     a record that was not committed is thrown away on restart;
#     a record that was committed is applied on restart.
#
# Everything else in the row sits on top of that guarantee and assumes it. So it
# is the piece that is worth proving first, and the piece whose absence is silent
# — a filesystem missing it looks perfectly healthy until the power goes off.
#
# ## Why the crash has to be a real one
#
# The interesting instant is between two writes: the record has reached the
# medium, the commit marker has not. Inside one process that instant can be
# faked, and a "journal" that is really a variable in RAM passes. So the crash
# here is a whole QEMU process ending — `spec/journal.sas` writes the record and
# then powers the machine off through SBI — and a SECOND QEMU process, started
# fresh against the same backing file, is what performs recovery. Nothing crosses
# between them except the bytes on the disk. That is the same shape as
# `tools/check-boot-counter.sh`, for the same reason.
#
# ## Why the value is random
#
# The value that travels record -> journal -> home is drawn from /dev/urandom on
# every invocation, and the program reads it out of a control sector this script
# writes. A program that printed a plausible constant, or that kept the value in
# RAM across a call it never actually made, cannot produce it. When run 4 prints
# the number run 3 was given, that number came off the medium.
#
# ## What the medium is asked directly
#
# The program's own words are not the only witness. After each power cycle this
# script reads the raw image with `od` and asserts the sector contents itself:
# that the record really is on the disk in the uncommitted case (otherwise
# "DISCARDED" would be true but vacuous — nothing was there to discard), that the
# commit marker really is absent, and that the home sector is untouched until a
# committed record is replayed into it. Doc 11 §2's rule again: a check whose
# failure mode is silence cannot be told apart from a check that did not run.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

prog=$root/spec/journal.sas
[ -f "$prog" ] || { echo "RED, as written: spec/journal.sas does not exist."; exit 1; }

# Sector map, mirrored from the program's header comment. Byte offsets, because
# that is what `od -j` wants and what a reader can check against `od` output.
HOME_OFF=0        # [0..4) "HOME", [8..16) the settled value
REC_OFF=512       # [0..4) "JRNL", [8..16) the proposed value
CMIT_OFF=1024     # [0..4) "CMIT"
CTL_OFF=1536      # [0] mode, [8..16) the value to journal
MAGIC_REC=4a524e4c
MAGIC_CMIT=434d4954
MAGIC_HOME=484f4d45
ZERO16=00000000000000000000000000000000

# Assembly failure is reported as itself, loudly, with the assembler's own words.
# This row was reopened once because its only evidence was a `.sas` file that did
# not assemble, and a check that lets `sadhana` fail into `set -e` says nothing
# about which line was wrong.
assemble() {
    if ! cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
            --स्थान "$1" "$prog" "$2" > "$tmp/asm.log" 2>&1; then
        echo "RED: spec/journal.sas did not assemble at $1 —"
        sed 's/^/    /' "$tmp/asm.log"
        exit 1
    fi
}

new_disk() { dd if=/dev/zero of="$1" bs=512 count=64 2>/dev/null; }

# Sixteen hex digits of real entropy. Not a counter and not a constant: the whole
# anti-forgery argument above rests on this being unguessable at write time.
fresh_value() { od -A n -N 8 -t x1 /dev/urandom | tr -d ' \n'; }

# `od` is the only reader of the image. Lowercase hex, no separators, so a
# comparison is a plain string equality and no shell hexadecimal is needed.
bytes() {  # $1 = image, $2 = byte offset, $3 = count
    od -A n -j "$2" -N "$3" -t x1 "$1" | tr -d ' \n'
}

# "00000000deadbe57" -> "57beadde00000000". The program stores a 64-bit word; the
# machine is little-endian, so the disk holds the pairs reversed. Written out
# rather than assumed, because getting it backwards would make every value
# assertion fail in a way that looks like a broken driver.
le64() { awk -v h="$1" 'BEGIN{ s=""; for (k=0;k<8;k++) s = s substr(h, 16-2*k-1, 2); print s }'; }

# The control sector: the experiment's input. mode in byte 0, value at [8..16).
# `printf` is handed a computed format string on purpose — it is how a NUL byte
# is produced portably, and the string is built here and contains nothing but
# `\NNN` escapes.
set_control() {  # $1 = image, $2 = mode, $3 = 16 hex digits
    oct=$(awk -v mode="$2" -v hv="$3" 'BEGIN{
        hex = "0123456789abcdef"
        printf "\\%03o", mode + 0
        for (i = 1; i < 8; i++) printf "\\000"
        for (k = 0; k < 8; k++) {
            p  = 16 - 2*k - 1
            d1 = index(hex, substr(hv, p,   1)) - 1
            d2 = index(hex, substr(hv, p+1, 1)) - 1
            printf "\\%03o", d1*16 + d2
        }
    }')
    # shellcheck disable=SC2059
    printf "$oct" | dd of="$1" bs=1 seek="$CTL_OFF" conv=notrunc 2>/dev/null
}

# One power cycle: a whole QEMU process, started and gone. Nothing is carried
# between calls except the file named in $2 — which is the entire point here,
# since the record and the commit marker are written by DIFFERENT processes from
# the one that recovers them.
#
# Bounded by an explicit SIGKILL from a background `sleep`, not by perl's alarm:
# QEMU runs its own timer subsystem and survives SIGALRM (`W-060`). A driver that
# mis-programs the queue spins forever on the used ring, so an unbounded wait
# here would hang the check rather than fail it.
cycle() {  # $1 = elf, $2 = disk, $3 = where to leave the cleaned output
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$1" \
        -drive file="$2",format=raw,if=none,id=d0 \
        -device virtio-blk-device,drive=d0 > "$tmp/raw" 2>&1 &
    qpid=$!
    # The watcher's own stderr is closed off, not just the kills' — the SUBSHELL
    # is what announces "Terminated: 15  sleep 25" when its `sleep` is killed, and
    # it inherited this script's stderr when it was forked. Silencing the `pkill`
    # instead does nothing, and the word Terminated lands in the middle of this
    # check's report, where the next reader will take it for a failure.
    ( sleep 25; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!
    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone orphans its `sleep` to init
    # (`W-096`). By PARENT pid, never by pattern — `W-044`/`W-047`/`W-049`;
    # another agent's QEMU has been seen live on this host.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    wait "$watcher" 2>/dev/null || true
    # QEMU's serial line ends every line with CR. Without stripping it,
    # `grep -E '^[0-9a-f]{16}$'` matches NOTHING and every value reads empty,
    # which looks exactly like a program that printed nothing at all.
    # LC_ALL=C because macOS `tr` aborts on invalid UTF-8 and TRUNCATES the
    # stream, and OpenSBI's banner is not guaranteed clean.
    LC_ALL=C tr -d '\r' < "$tmp/raw" > "$3"
}

verdict() { grep -E '^JOURNAL-[A-Z-]+$' "$1" | tail -1; }
# Sixteen hex digits alone on a line — the shape the छापनम् helper prints. OpenSBI's
# own hexadecimal is written `0x…` with a label, so it cannot collide.
value()   { grep -E '^[0-9a-f]{16}$'   "$1" | tail -1; }

fail=0
note() { echo "  FAIL  $*"; fail=1; }

# $1 = what to call it, $2 = got, $3 = wanted
same() {
    [ "$2" = "$3" ] || note "$1
        got    $2
        wanted $3"
}

echo "Assembling at two link addresses..."
assemble ०षोड्८०२००००० "$tmp/j-a.elf"
assemble ०षोड्८०४००००० "$tmp/j-b.elf"

# The whole experiment, once per link address. Running it twice is not padding:
# the queue area, the message strings and the branch targets are all built with
# `auipc`, and an address that was a constant which happened to be right at
# 0x80200000 stops being right at 0x80400000.
scenario() {  # $1 = elf, $2 = label for the report
    elf=$1; where=$2
    vu=$(fresh_value); vc=$(fresh_value)
    [ "$vu" != "$vc" ] || { note "$where: /dev/urandom handed out the same value twice"; return; }

    u=$tmp/u-$where.img; c=$tmp/c-$where.img
    new_disk "$u"; new_disk "$c"

    echo "  [$where] value to be journalled without a commit: $vu"
    echo "  [$where] value to be journalled WITH a commit:     $vc"

    # ---- uncommitted -------------------------------------------------------
    # Power cycle 1: write the record, stop before the commit marker.
    set_control "$u" 1 "$vu"
    cycle "$elf" "$u" "$tmp/u1.out"
    same "$where u1 verdict"  "$(verdict "$tmp/u1.out")" "JOURNAL-ARMED"
    same "$where u1 echoed the control value" "$(value "$tmp/u1.out")" "$vu"
    # The medium, asked directly. Without this, "DISCARDED" on the next run could
    # be true because nothing was ever written — the vacuous pass.
    same "$where u1 left a record on the medium" \
         "$(bytes "$u" $REC_OFF 4)" "$MAGIC_REC"
    same "$where u1 left the right value in the record" \
         "$(bytes "$u" $((REC_OFF + 8)) 8)" "$(le64 "$vu")"
    same "$where u1 left NO commit marker" \
         "$(bytes "$u" $CMIT_OFF 4)" "00000000"
    same "$where u1 left home untouched" \
         "$(bytes "$u" $HOME_OFF 16)" "$ZERO16"

    # Power cycle 2: a different process recovers. The row.
    set_control "$u" 0 0000000000000000
    cycle "$elf" "$u" "$tmp/u2.out"
    same "$where u2 verdict"  "$(verdict "$tmp/u2.out")" "JOURNAL-DISCARDED"
    same "$where u2 home value" "$(value "$tmp/u2.out")" "0000000000000000"
    same "$where u2 left home still untouched" \
         "$(bytes "$u" $HOME_OFF 16)" "$ZERO16"
    # And the discarded value is gone from the whole image, not merely unread.
    if od -A n -t x1 "$u" | tr -d ' \n' | grep -q "$(le64 "$vu")"; then
        note "$where u2: the uncommitted value $vu is still somewhere in the image;
        it was not discarded, only ignored"
    fi

    # ---- committed ---------------------------------------------------------
    # Power cycle 3: record AND commit marker, then stop before the checkpoint.
    # One extra write is the entire difference from cycle 1.
    set_control "$c" 2 "$vc"
    cycle "$elf" "$c" "$tmp/c1.out"
    same "$where c1 verdict"  "$(verdict "$tmp/c1.out")" "JOURNAL-COMMITTED"
    same "$where c1 echoed the control value" "$(value "$tmp/c1.out")" "$vc"
    same "$where c1 left a record on the medium" \
         "$(bytes "$c" $REC_OFF 4)" "$MAGIC_REC"
    same "$where c1 left the right value in the record" \
         "$(bytes "$c" $((REC_OFF + 8)) 8)" "$(le64 "$vc")"
    same "$where c1 left a commit marker" \
         "$(bytes "$c" $CMIT_OFF 4)" "$MAGIC_CMIT"
    # Home is still empty here. This is what makes run 4 mean something: the value
    # can only reach home by being replayed out of the journal, because at the
    # moment the machine died it was not in home at all.
    same "$where c1 left home NOT yet written" \
         "$(bytes "$c" $HOME_OFF 16)" "$ZERO16"

    # Power cycle 4: a different process recovers. The row, the other way.
    set_control "$c" 0 0000000000000000
    cycle "$elf" "$c" "$tmp/c2.out"
    same "$where c2 verdict"  "$(verdict "$tmp/c2.out")" "JOURNAL-REPLAYED"
    same "$where c2 home value" "$(value "$tmp/c2.out")" "$vc"
    same "$where c2 wrote the home magic" \
         "$(bytes "$c" $HOME_OFF 4)" "$MAGIC_HOME"
    same "$where c2 settled the journalled value into home" \
         "$(bytes "$c" $((HOME_OFF + 8)) 8)" "$(le64 "$vc")"

    # Power cycle 5: recovery again. A journal that is merely READ rather than
    # CONSUMED replays forever, and on a real filesystem that is how a stale
    # record silently overwrites newer data. CLEAN is the journal saying it is
    # spent; the value staying put is the checkpoint holding.
    cycle "$elf" "$c" "$tmp/c3.out"
    same "$where c3 verdict"  "$(verdict "$tmp/c3.out")" "JOURNAL-CLEAN"
    same "$where c3 home value" "$(value "$tmp/c3.out")" "$vc"

    # The two scenarios must not agree. If they do, the program is ignoring the
    # commit marker and this whole check has been reading one behaviour twice.
    if [ "$(verdict "$tmp/u2.out")" = "$(verdict "$tmp/c2.out")" ]; then
        note "$where: committed and uncommitted recovery reached the same verdict"
    fi
}

echo
echo "Ten power cycles: five per link address, each a separate QEMU process."
echo
scenario "$tmp/j-a.elf" 0x80200000
scenario "$tmp/j-b.elf" 0x80400000
echo

# A named failure is reported as itself rather than as a missing verdict.
for f in "$tmp"/u1.out "$tmp"/u2.out "$tmp"/c1.out "$tmp"/c2.out "$tmp"/c3.out; do
    [ -f "$f" ] || continue
    for tag in NO-DEVICE NO-QUEUE IO-FAILED; do
        if grep -q "^JOURNAL-$tag\$" "$f"; then
            echo "  note  the driver reported $tag in $(basename "$f")"
        fi
    done
done

[ "$fail" -eq 0 ] || {
    echo
    echo "the journal does not survive a crash."
    exit 1; }

echo "ok  the journal discards what was not committed and replays what was."
echo
echo "    At each of two link addresses, on a freshly zeroed medium:"
echo
echo "      cycle 1  a record is written to sector 1; the machine powers off"
echo "               before the commit marker.  od confirms JRNL on the medium"
echo "               and sector 2 empty."
echo "      cycle 2  a NEW QEMU process reads that medium and says DISCARDED."
echo "               Sector 0 is untouched and the value is nowhere in the image."
echo
echo "      cycle 3  the same thing plus ONE more write — the commit marker."
echo "               od confirms JRNL and CMIT, and sector 0 still empty."
echo "      cycle 4  a NEW QEMU process reads that medium and says REPLAYED,"
echo "               and prints the value it was never told, having read it"
echo "               out of the journal and settled it into sector 0."
echo "      cycle 5  once more: CLEAN, value unchanged. The journal was spent,"
echo "               not merely read."
echo
echo "    The value is drawn from /dev/urandom on every run and reaches the"
echo "    program only through a control sector this script writes, so it cannot"
echo "    have been carried in the image."
echo
echo "    This is one property of row C-005, not the row. There is no VFS here:"
echo "    no paths, no directory tree, no names, no stat, no rename, no ext4 or"
echo "    FAT32 reader. What is settled is that the commit marker — and nothing"
echo "    else — decides whether a record survives a crash."
