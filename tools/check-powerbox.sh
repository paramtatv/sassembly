#!/bin/sh
# A sandboxed app holds NO directory authority, is handed ONE file by the
# powerbox, and reads that file and NOTHING ELSE.
# Row `E-010`, doc 08 §4 — one slice of it, named honestly below.
#
# ## What is being proved, and what is NOT
#
# `E-010` reads "File browser कोशदर्शकः + powerbox". This check does not prove
# that row. There is no file browser here: no display, no listing, no names, no
# selection UI, no drag and drop, no mouse. What is proved is the one property
# without which none of that is a *powerbox*:
#
#     the app begins with no authority at all; the user's choice is turned into
#     exactly one capability; and after that the app can reach that one file and
#     nothing else -- not the other files, and not the directory that names them.
#
# The first half is unremarkable: every system can open a file. The whole of the
# powerbox idea lives in the second half, and the second half is the one that
# fails silently -- an app that quietly holds more authority than it was granted
# looks exactly like one that does not.
#
# ## Why the denial is proved by an ATTEMPT, not by silence
#
# A denial that is proved by the absence of a read has proved nothing: a program
# that never asks is indistinguishable from one that asked and was refused, and
# the second is the only one that demonstrates a gate. So `powerbox.sas` makes
# EIGHT open attempts -- four sectors, twice, once before the grant and once
# after -- and every one of them prints `POWERBOX-ATTEMPT-<n>` BEFORE the check
# runs, followed by `POWERBOX-DENIED-<n>` or `POWERBOX-OPENED-<n>`. This script
# asserts both members of every pair. A program that stopped asking would fail
# here on the missing ATTEMPT lines, not pass by being quiet.
#
# ## Why the program's own words are not the only witness
#
# A program can print whatever it likes. Three witnesses sit outside its
# narration:
#
#  1. THE DEVICE REQUEST COUNT. Every virtio-blk request bumps the available
#     ring index, and the program prints that index at the end. Eight attempts;
#     the count must be exactly THREE -- the control sector, the directory, and
#     the one granted file. A denied attempt that quietly reached the medium
#     makes it four. That number is a measurement of the wire the device talks
#     on, not an opinion.
#
#  2. THE OTHER FILES' CONTENTS. Each of the three files gets a fresh
#     /dev/urandom value on every invocation, written by this script and
#     existing nowhere but on the disk. If the app had read a file it was not
#     granted, that value would appear in the output. This script asserts the
#     granted value IS there and the other two are NOT -- and asserts, with
#     `od`, that the other two really are on the medium, so the absence is a
#     refusal and not an empty disk.
#
#  3. THE CHOICE ITSELF. The user's pick is read from a control sector this
#     script writes, and all three picks are exercised at both link addresses.
#     A program hard-coding "always file 2" is caught on the runs that ask for
#     1 and 3.
#
# ## The named failure modes are exercised, not merely declared
#
# Two extra cycles hand the powerbox a pick that is not in the directory -- 0,
# which is the directory sector itself, and 7, which is past the end. Both must
# come back `POWERBOX-BAD-PICK` with no file opened at all. Without these, the
# refusal tags would be decoration that no run ever reaches, and a regression
# that made them unreachable would look like a pass.
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

# 77 is SKIPPED. It is never a pass: a check that could not run has measured
# nothing, and reporting 0 here would launder "not tested" into "tested".
command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

root=$(CDPATH= cd -- "$here/.." && pwd)
prog=$root/spec/powerbox.sas
[ -f "$prog" ] || { echo "RED, as written: $prog does not exist."; exit 1; }

# `cargo run`, like every other check in this tree.
#
# This was drafted against a prebuilt assembler so it could run while a gate
# held the cargo lock. Dropped deliberately: that path resolves to the
# repository root, and a stale binary left there would assemble with an
# assembler that is not the one in the tree — and still print `ok`. A check
# that can silently measure the wrong build is worse than one that waits.

# ---- sector map, mirrored from the program's header comment ----------------
# Byte offsets, because that is what `od -j` wants and what a reader can check
# against `od` output.
DIR_OFF=0        # [0..4) "PBOX", [8..16) how many files the directory lists
F1_OFF=512       # [0..4) "FILE", [8..16) a fresh random value
F2_OFF=1024
F3_OFF=1536
CTL_OFF=2048     # [0] which file the user picked
MAGIC_DIR=50424f58     # "PBOX"
MAGIC_FILE=46494c45    # "FILE"

fail=0
note() { echo "  FAIL  $*"; fail=1; }

# $1 = what to call it, $2 = got, $3 = wanted
same() {
    [ "$2" = "$3" ] || note "$1
        got    $2
        wanted $3"
}

# Assembly failure is reported as itself, with the assembler's own words. This
# row was REOPENED because its only evidence was a `.sas` file that did not
# assemble, and a check that lets the assembler fall into `set -e` says nothing
# about which line was wrong.
assemble() {
    if ! cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
            --स्थान "$1" "$prog" "$2" > "$tmp/asm.log" 2>&1; then
        echo "RED: powerbox.sas did not assemble at $1 —"
        sed 's/^/    /' "$tmp/asm.log"
        exit 1
    fi
}

# ---- building the medium ---------------------------------------------------
new_disk() { dd if=/dev/zero of="$1" bs=512 count=16 2>/dev/null; }

# Sixteen hex digits of real entropy. Not a counter and not a constant: the
# whole anti-forgery argument above rests on this being unguessable at the time
# the program was written.
fresh_value() { od -A n -N 8 -t x1 /dev/urandom | tr -d ' \n'; }

# `od` is the only reader of the image. Lowercase hex, no separators, so a
# comparison is plain string equality and no shell hexadecimal is needed.
bytes() {  # $1 = image, $2 = byte offset, $3 = count
    od -A n -j "$2" -N "$3" -t x1 "$1" | tr -d ' \n'
}

# "00000000deadbe57" -> "57beadde00000000". The program stores a 64-bit word and
# the machine is little-endian, so the disk holds the pairs reversed. Written
# out rather than assumed: getting it backwards would make every value assertion
# fail in a way that looks like a broken driver.
le64() { awk -v h="$1" 'BEGIN{ s=""; for (k=0;k<8;k++) s = s substr(h, 16-2*k-1, 2); print s }'; }

# 16 hex digits -> eight `\NNN` octal escapes, little-endian. `printf` is handed
# a computed format string on purpose: it is how NUL bytes are produced
# portably, and the string is built here and contains nothing but `\NNN`.
octal_le64() {
    awk -v hv="$1" 'BEGIN{
        hex = "0123456789abcdef"
        for (k = 0; k < 8; k++) {
            p  = 16 - 2*k - 1
            d1 = index(hex, substr(hv, p,   1)) - 1
            d2 = index(hex, substr(hv, p+1, 1)) - 1
            printf "\\%03o", d1*16 + d2
        }
    }'
}

put() {  # $1 = image, $2 = byte offset, $3 = a string of \NNN escapes
    # shellcheck disable=SC2059
    printf "$3" | dd of="$1" bs=1 seek="$2" conv=notrunc 2>/dev/null
}

put_ascii() {  # $1 = image, $2 = byte offset, $3 = literal text
    printf '%s' "$3" | dd of="$1" bs=1 seek="$2" conv=notrunc 2>/dev/null
}

# The whole medium is written HERE, by this script. The program only ever reads
# it -- it contains no VIRTIO_BLK_T_OUT at all -- so every byte it reports came
# off a disk it did not author.
build_disk() {  # $1 = image, $2 = pick, $3 $4 $5 = the three file values
    new_disk "$1"
    put_ascii "$1" "$DIR_OFF" PBOX
    put       "$1" $((DIR_OFF + 8)) "$(octal_le64 0000000000000003)"
    put_ascii "$1" "$F1_OFF" FILE
    put       "$1" $((F1_OFF + 8)) "$(octal_le64 "$3")"
    put_ascii "$1" "$F2_OFF" FILE
    put       "$1" $((F2_OFF + 8)) "$(octal_le64 "$4")"
    put_ascii "$1" "$F3_OFF" FILE
    put       "$1" $((F3_OFF + 8)) "$(octal_le64 "$5")"
    put       "$1" "$CTL_OFF" "$(awk -v m="$2" 'BEGIN{ printf "\\%03o", m + 0 }')"
}

# ---- running one machine ---------------------------------------------------
# Bounded by an explicit SIGKILL from a background `sleep`, not by perl's alarm:
# QEMU runs its own timer subsystem and survives SIGALRM. A driver that
# mis-programs the queue spins forever on the used ring, so an unbounded wait
# would hang this check rather than fail it.
cycle() {  # $1 = elf, $2 = disk, $3 = where to leave the cleaned output
    qemu-system-riscv64 -machine virt -nographic -bios default \
        -kernel "$1" \
        -drive file="$2",format=raw,if=none,id=d0 \
        -device virtio-blk-device,drive=d0 > "$tmp/raw" 2>&1 &
    qpid=$!
    # The watcher's own stderr is closed off, not just the kills': the SUBSHELL
    # is what announces "Terminated: 15  sleep 25" when its `sleep` is killed,
    # and it inherited this script's stderr when it was forked. Silencing the
    # `pkill` instead does nothing, and the word Terminated lands in the middle
    # of this check's report, where the next reader takes it for a failure.
    ( sleep 25; kill -9 "$qpid" 2>/dev/null ) 2>/dev/null & watcher=$!
    wait "$qpid" 2>/dev/null || true
    # `pkill -P` FIRST: killing the subshell alone orphans its `sleep` to init.
    # By PARENT pid, never by pattern -- another agent's QEMU has been seen live
    # on this host, and a pattern kill would take it out.
    pkill -P "$watcher" 2>/dev/null || true
    kill "$watcher" 2>/dev/null || true
    wait "$watcher" 2>/dev/null || true
    # QEMU's serial line ends every line with CR. Without stripping it,
    # `grep -E '^POWERBOX-DONE$'` matches NOTHING and every assertion reads
    # empty, which looks exactly like a program that printed nothing at all.
    # LC_ALL=C because macOS `tr` aborts on invalid UTF-8 and TRUNCATES the
    # stream, and OpenSBI's banner is not guaranteed clean.
    LC_ALL=C tr -d '\r' < "$tmp/raw" > "$3"
}

# The program's transcript, with OpenSBI's banner dropped. Sixteen lowercase hex
# digits alone on a line is the shape the छापनम् helper prints; OpenSBI writes
# its own hexadecimal as `0x…` with a label, so the two cannot collide.
transcript() { grep -aE '^(POWERBOX-[A-Z0-9-]+|[0-9a-f]{16})$' "$1" || true; }
count()      { grep -ac "$1" "$2" 2>/dev/null || true; }
has()        { grep -aqx "$1" "$2"; }

# Every named failure the program can print. Checked on every run: a driver
# fault that reported as a missing ATTEMPT line would be a mystery, and as
# POWERBOX-NO-DEVICE it is a sentence.
TAGS='POWERBOX-NO-DEVICE POWERBOX-NO-QUEUE POWERBOX-IO-FAILED POWERBOX-BAD-DIR
      POWERBOX-BAD-FILE POWERBOX-BAD-PICK POWERBOX-BREACH'

no_faults() {  # $1 = label, $2 = output, $3 = the one tag that is allowed
    for t in $TAGS; do
        [ "$t" = "$3" ] && continue
        ! has "$t" "$2" || note "$1: the program reported $t"
    done
}

echo "Assembling at two link addresses..."
assemble ०षोड्८०२००००० "$tmp/pb-a.elf"
assemble ०षोड्८०४००००० "$tmp/pb-b.elf"

# ---- the expected transcript, written out in full --------------------------
# Building it here rather than grepping for fragments means an EXTRA line is a
# failure too. A program that opened a second file, or that printed a stray
# OPENED, would satisfy every individual assertion below and still be wrong.
expect() {  # $1 = pick, $2 = the value that pick's file holds
    echo POWERBOX-PHASE-UNGRANTED
    for s in 0 1 2 3; do
        echo "POWERBOX-ATTEMPT-$s"
        echo "POWERBOX-DENIED-$s"
    done
    echo "POWERBOX-PICKED-$1"
    echo POWERBOX-PHASE-GRANTED
    for s in 0 1 2 3; do
        echo "POWERBOX-ATTEMPT-$s"
        if [ "$s" = "$1" ]; then echo "POWERBOX-OPENED-$s"; echo "$2"
        else echo "POWERBOX-DENIED-$s"; fi
    done
    echo POWERBOX-READS
    echo 0000000000000003
    echo POWERBOX-DONE
}

# ---- one grant, end to end -------------------------------------------------
grant() {  # $1 = elf, $2 = label, $3 = which file the user picks
    elf=$1; where=$2; pick=$3
    tag="$where pick=$pick"

    v1=$(fresh_value); v2=$(fresh_value); v3=$(fresh_value)
    if [ "$v1" = "$v2" ] || [ "$v1" = "$v3" ] || [ "$v2" = "$v3" ]; then
        note "$tag: /dev/urandom handed out the same value twice"; return
    fi
    eval "vg=\$v$pick"

    img=$tmp/d-$where-$pick.img
    build_disk "$img" "$pick" "$v1" "$v2" "$v3"
    before=$(cksum < "$img")

    # The medium, asked directly, BEFORE the run. Without this the leak test
    # below is vacuous: a value that is nowhere on the disk cannot leak, and an
    # empty disk would pass it perfectly.
    same "$tag: the directory magic is on the medium" \
         "$(bytes "$img" $DIR_OFF 4)" "$MAGIC_DIR"
    k=1
    for off in $F1_OFF $F2_OFF $F3_OFF; do
        eval "vk=\$v$k"
        same "$tag: file $k magic is on the medium" \
             "$(bytes "$img" "$off" 4)" "$MAGIC_FILE"
        same "$tag: file $k value is on the medium" \
             "$(bytes "$img" $((off + 8)) 8)" "$(le64 "$vk")"
        k=$((k + 1))
    done

    out=$tmp/o-$where-$pick.out
    cycle "$elf" "$img" "$out"

    no_faults "$tag" "$out" none

    # ---- the transcript, whole -------------------------------------------
    expect "$pick" "$vg" > "$tmp/want"
    transcript "$out" > "$tmp/got"
    if ! cmp -s "$tmp/want" "$tmp/got"; then
        note "$tag: the transcript is not the expected one —"
        diff -u "$tmp/want" "$tmp/got" 2>/dev/null | sed 's/^/        /' || true
    fi

    # ---- the same facts named one at a time ------------------------------
    # The whole-transcript comparison above catches everything these catch, but
    # it reports as a diff. These report as sentences, and a sentence is what a
    # reader needs at 3am. Doc 11 §2: a failure must name itself.

    # THE POSITIVE HALF: the granted open happened and produced real bytes.
    has "POWERBOX-OPENED-$pick" "$out" \
        || note "$tag: the granted file was never opened"
    same "$tag: exactly one file was opened" \
         "$(count '^POWERBOX-OPENED-' "$out")" 1
    same "$tag: the value read out of file $pick" \
         "$(grep -aE '^[0-9a-f]{16}$' "$out" | head -1)" "$vg"
    has "POWERBOX-PICKED-$pick" "$out" \
        || note "$tag: the powerbox never delegated the user's choice"

    # THE NEGATIVE HALF: every other open was ATTEMPTED and REFUSED.
    same "$tag: eight open attempts were made" \
         "$(count '^POWERBOX-ATTEMPT-' "$out")" 8
    same "$tag: seven of them were refused" \
         "$(count '^POWERBOX-DENIED-' "$out")" 7

    # The directory: attempted twice, refused twice, opened never. This is the
    # row's own words -- "with no directory capability" -- read literally. Note
    # the SECOND refusal is after the grant: holding a file does not confer the
    # directory that names it.
    same "$tag: the directory was asked for in both phases" \
         "$(count '^POWERBOX-ATTEMPT-0$' "$out")" 2
    same "$tag: the directory was refused in both phases" \
         "$(count '^POWERBOX-DENIED-0$' "$out")" 2
    ! has POWERBOX-OPENED-0 "$out" \
        || note "$tag: THE APP OPENED THE DIRECTORY"

    for s in 1 2 3; do
        [ "$s" = "$pick" ] && continue
        has "POWERBOX-ATTEMPT-$s" "$out" \
            || note "$tag: file $s was never even asked for — a denial that was
        never attempted proves nothing"
        same "$tag: file $s was refused in both phases" \
             "$(count "^POWERBOX-DENIED-$s\$" "$out")" 2
        ! has "POWERBOX-OPENED-$s" "$out" \
            || note "$tag: THE APP OPENED UNGRANTED FILE $s"
    done

    # WITNESS 1: the device. Eight attempts, three requests -- control sector,
    # directory, and the one granted file. This is counted at the virtqueue,
    # not asserted by the program's prose.
    same "$tag: the device was asked for exactly three sectors" \
         "$(grep -aA1 '^POWERBOX-READS$' "$out" | tail -1)" 0000000000000003

    # WITNESS 2: the other files' contents never appear. They are on the disk
    # (asserted above) and they are random, so their absence here is the app
    # not having read them.
    for s in 1 2 3; do
        [ "$s" = "$pick" ] && continue
        eval "vs=\$v$s"
        ! grep -aqF "$vs" "$out" \
            || note "$tag: the contents of ungranted file $s ($vs) reached the
        output — the app read a file it was never given"
    done

    # A read capability is not a write capability. The program contains no
    # VIRTIO_BLK_T_OUT at all; this asserts that at the medium.
    same "$tag: the medium is byte-identical after the run" \
         "$(cksum < "$img")" "$before"
}

# ---- a pick the directory does not contain ---------------------------------
# The powerbox cannot mint authority it was not given either. Pick 0 is the
# directory sector itself -- the sharpest case, a user "choosing" the thing the
# app must never reach -- and pick 7 is past the end of the directory. Both must
# be refused before any capability is written, so no file opens at all.
badpick() {  # $1 = elf, $2 = label, $3 = the impossible pick
    elf=$1; where=$2; pick=$3
    tag="$where bad pick=$pick"

    v1=$(fresh_value); v2=$(fresh_value); v3=$(fresh_value)
    img=$tmp/b-$pick.img
    build_disk "$img" "$pick" "$v1" "$v2" "$v3"
    out=$tmp/b-$pick.out
    cycle "$elf" "$img" "$out"

    has POWERBOX-BAD-PICK "$out" \
        || note "$tag: the powerbox did not refuse a choice outside the directory"
    no_faults "$tag" "$out" POWERBOX-BAD-PICK

    # Nothing opened, and the run stopped before the second phase.
    same "$tag: no file was opened" "$(count '^POWERBOX-OPENED-' "$out")" 0
    ! has POWERBOX-PHASE-GRANTED "$out" \
        || note "$tag: the app entered the granted phase without a grant"
    ! has POWERBOX-DONE "$out" \
        || note "$tag: the run completed normally despite an impossible pick"

    # The first phase still ran, so the four refusals are still evidence and not
    # an early exit that skipped the interesting part.
    same "$tag: four attempts were still made and refused" \
         "$(count '^POWERBOX-ATTEMPT-' "$out")" 4
    same "$tag: all four were refused" \
         "$(count '^POWERBOX-DENIED-' "$out")" 4

    for s in 1 2 3; do
        eval "vs=\$v$s"
        ! grep -aqF "$vs" "$out" \
            || note "$tag: file $s's contents reached the output"
    done
}

echo
echo "Eight machines: three grants at each of two link addresses, plus two"
echo "impossible picks. Each is a separate QEMU process."
echo

for pick in 1 2 3; do
    echo "  [0x80200000] the user picks file $pick"
    grant "$tmp/pb-a.elf" 0x80200000 "$pick"
done
for pick in 1 2 3; do
    echo "  [0x80400000] the user picks file $pick"
    grant "$tmp/pb-b.elf" 0x80400000 "$pick"
done
for pick in 0 7; do
    echo "  [0x80200000] the user picks $pick, which the directory does not list"
    badpick "$tmp/pb-a.elf" 0x80200000 "$pick"
done
echo

[ "$fail" -eq 0 ] || {
    echo
    echo "the powerbox does not confine the app to the file it was granted."
    exit 1; }

echo "ok  the app holds no directory authority, is handed one file, and reads"
echo "    that file and nothing else."
echo
echo "    At each of two link addresses, for each of the three files, on a"
echo "    medium this script wrote and the program only ever reads:"
echo
echo "      before the grant  the app asks for the directory and all three"
echo "                        files.  Four ATTEMPT lines, four DENIED lines."
echo "                        Its capability slot is empty, so there is no"
echo "                        sector it can name."
echo "      the powerbox      reads the directory -- which the app cannot --"
echo "                        checks the user's choice against it, and writes"
echo "                        ONE sector into the app's single slot."
echo "      after the grant   the app asks for the same four again.  Exactly"
echo "                        one opens, and it is the one the user picked."
echo "                        The directory is refused a second time: holding"
echo "                        a file does not confer the directory naming it."
echo
echo "    Three independent witnesses, none of them the program's own prose:"
echo
echo "      the device   the available-ring index is printed at the end and is"
echo "                   exactly 3 for eight attempts -- control sector,"
echo "                   directory, one file.  A denied attempt that reached"
echo "                   the medium would make it 4."
echo "      the disk     each file's value is drawn from /dev/urandom per run"
echo "                   and exists only on the medium.  od confirms all three"
echo "                   are there; the output contains the granted one and"
echo "                   neither of the others."
echo "      the choice   all three picks are exercised at both addresses, so a"
echo "                   hard-coded answer is caught by the runs it does not fit."
echo
echo "    Two further machines hand the powerbox a choice the directory does"
echo "    not list -- sector 0, the directory itself, and sector 7, past the"
echo "    end.  Both come back POWERBOX-BAD-PICK with nothing opened, so the"
echo "    refusal tags are reachable rather than decorative."
echo
echo "    This is one property of row E-010, not the row. There is no file"
echo "    browser here: no display, no listing, no names, no selection UI, no"
echo "    mouse, no IPC between separate address spaces, and no MMU -- the"
echo "    confinement is a checked capability slot in one program, not hardware"
echo "    isolation.  What is settled is that a single delegated capability --"
echo "    and nothing else -- decides which bytes the app can reach."
