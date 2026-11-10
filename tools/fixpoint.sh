#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# THE SELF-HOSTING FIXPOINT, IN ONE COMMAND — because assembling it by hand
# costs a wrong answer.
#
# Stage 1 is the image the INTERPRETED `.t1` compiler builds from the working
# tree. Stage 2 is that image, running natively, compiling the same sources fed
# through the input channel. The fixpoint is Stage 2 == Stage 1, byte for byte.
#
# WHY THIS SCRIPT EXISTS. Two artefacts feed that comparison and only ONE of
# them follows the working tree:
#
#   t1_image   reads the SOURCES  (crates/sadhana-t1/src/*.t1)
#   yantra-run reads the BLOB     (YANTRA_INPUT, packed by pack-corpus.sh)
#
# On 2026-09-23 a blob packed at 19:34 was fed to a Stage 2 testing an edit made
# at 23:45. Stage 1 held the change and Stage 2 compiled sources without it, so
# the two images differed — and that was reported as a fixpoint FAILURE, with a
# frame-slot analysis on top of it naming "4 of 873 routines" and a supposed
# interpreter-vs-native divergence. All of it was the stale blob. Two rounds,
# about 3.5 hours, and a published claim that had to be retracted. The tell was
# available and missed: a second round with a DIFFERENT Stage 1 produced a Stage
# 2 byte-identical to the first, and two different compilers cannot agree on
# inputs that differ.
#
# So: this script packs the blob FROM THE SAME TREE, immediately before the
# build, every time. There is no flag to reuse an existing one.
#
# Usage: tools/fixpoint.sh [outdir]
# Exit:  0 the fixpoint holds · 1 it does not · 2 a stage failed to produce one
set -u

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-${TMPDIR:-/tmp}/sassembly-fixpoint}"
SRC="$ROOT/crates/sadhana-t1/src"
mkdir -p "$OUT" || exit 2

# The release binaries, wherever this checkout put them. `--release` matters:
# a debug Stage 1 build is hours, not minutes.
BIN="${SASSEMBLY_BIN:-$ROOT/target/release}"
# RUST-FREE: a fully native run (FIXPOINT_STAGE1=native + FIXPOINT_STAGE2=native) needs NEITHER binary:
# t1_image is needed only for the interpreter Stage 1 (also the fallback below), yantra-run only for
# a yantra Stage 2.
need=""
[ "${FIXPOINT_STAGE1:-native}" = native ] || need="$need t1_image"
for b in $need; do
    if [ ! -x "$BIN/$b" ]; then
        echo "fixpoint: no $b under $BIN" >&2
        echo "  build with: tools/t1-build.sh (crates retired)" >&2
        echo "  or point SASSEMBLY_BIN at a directory holding both" >&2
        exit 2
    fi
done

# THE HOST TARGET for every native translation below (8f, 2026-10-10: anuvada.py defaults to x86_64-linux, so a native Stage 1 or
# Stage 2 failed on Apple Silicon): host_target() of tools/t1-native-lib.sh, the same answer t1-run.sh and the landing use.
NTGT=
if [ "${FIXPOINT_STAGE1:-native}" = native ] || [ "${FIXPOINT_STAGE2:-native}" = native ]; then
    NTGT=$(ROOT="$ROOT" SAS="$ROOT" BIN=/nonexistent bash -c '. "$ROOT/tools/t1-native-lib.sh" && host_target' | cut -d' ' -f1)
    [ -n "$NTGT" ] || { echo "fixpoint: no native target for $(uname -s) $(uname -m) (tools/t1-native-lib.sh host_target)" >&2; exit 2; }
    echo "fixpoint: native translations target $NTGT (this host)"
fi

# Portable helpers (no interpreter): sha256 of a file; the image inside a sink.
sha256_of() { if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
# The rung prints its image between markers, so a sink carries one octet of marker on each
# side of the ELF: write [first 0x7fELF, size-1) of $1 to $2; rc 1 and no write when no ELF.
elf_from_sink() {
    _o=$(LC_ALL=C grep -abo -F "$(printf '\177ELF')" "$1" 2>/dev/null | head -1 | cut -d: -f1)
    [ -n "$_o" ] || return 1
    _n=$(wc -c < "$1" | tr -d ' ')
    _l=$((_n - 1 - _o)); [ "$_l" -gt 0 ] || _l=0
    head -c $(( _o + 1 - 1 + $_l )) "$1" | tail -c +$((_o + 1)) > "$2"
}

# ── the blob, packed NOW, from the tree about to be compiled ────────────────
echo "fixpoint: packing the corpus from $SRC"
sh "$ROOT/tools/pack-corpus.sh" "$OUT/corpus.blob" "$SRC"/*.t1 | tail -1 || exit 2

# A blob older than any source it claims to contain is the failure this script
# exists to prevent. `pack-corpus.sh` just wrote it, so this can only fire if
# something raced us — but the check costs nothing and the alternative cost
# 3.5 hours once.
for f in "$SRC"/*.t1; do
    if [ "$f" -nt "$OUT/corpus.blob" ]; then
        echo "fixpoint: $f is NEWER than the blob just packed — refusing" >&2
        exit 2
    fi
done

# ── Stage 1: the interpreted compiler builds the image ──────────────────────
# FIXPOINT_STAGE1=native + FIXPOINT_PREV_IMAGE=<elf> (owner ruling relayed 2026-10-09: "native
# Stage 1 APPROVED for everyday landings (fixpoint via native Stage 1 + native Stage 2), with the
# Rust-interpreter Stage 1 + full yantra Stage 2 kept as a NIGHTLY (and pre-release) independent
# check that must stay green"): the PREVIOUS main's compiler image P, translated natively by
# anuvada.py, compiles THIS corpus -> A (this is Stage 1); A, translated, compiles it again -> B
# (Stage 2); A == B byte-identical is the everyday fixpoint. If FIXPOINT_PREV_IMAGE is missing or
# its sha256 sidecar (<elf>.sha256) disagrees, it FALLS BACK to the interpreter Stage 1, saying so
# by name. an internal tool always runs the interpreter Stage 1.
S1="$OUT/stage1.elf"
STAGE1_ENGINE=interpreter
PREV="${FIXPOINT_PREV_IMAGE:-}"
if [ "${FIXPOINT_STAGE1:-native}" = native ]; then
    # No image named: take the host's trusted pinned seed (t1-native-lib.sh trusted_seed: sha-checked, never older than the re-key).
    if [ -z "$PREV" ]; then
        PREV=$(ROOT="$ROOT" SAS="$ROOT" BIN=/nonexistent bash -c '. "$ROOT/tools/t1-native-lib.sh" && trusted_seed' 2>/dev/null) || PREV=
    fi
    if [ -z "$PREV" ] || [ ! -s "$PREV" ]; then
        echo "fixpoint: FALLBACK to the INTERPRETER Stage 1 — no cached previous image (FIXPOINT_PREV_IMAGE='${PREV}')" >&2
    elif [ -r "$PREV.sha256" ] && [ "$(sha256sum "$PREV" | cut -d' ' -f1)" != "$(cut -d' ' -f1 "$PREV.sha256")" ]; then
        echo "fixpoint: FALLBACK to the INTERPRETER Stage 1 — $PREV does not match its pinned sha256" >&2
    else
        echo "fixpoint: Stage 1 running NATIVELY (previous image $(sha256sum "$PREV" | cut -c1-16) translated; compiles this corpus)"
        python3 "$ROOT/tools/native-spike/anuvada.py" --target "$NTGT" "$PREV" "$OUT/prev.native" > "$OUT/stage1.translate.log" 2>&1 || {
            echo "fixpoint: the translator refused the previous image; see $OUT/stage1.translate.log" >&2; exit 2; }
        YANTRA_INPUT="$OUT/corpus.blob" YANTRA_INPUT_NAME=शृङ्खला YANTRA_RAM="${YANTRA_RAM:-2684354560}" YANTRA_STEPS="${YANTRA_STEPS:-10000000000000}" \
            "$OUT/prev.native" > "$OUT/stage1.sink" 2> "$OUT/stage1.log"
        elf_from_sink "$OUT/stage1.sink" "$S1" || :
        STAGE1_ENGINE=native
    fi
fi
if [ "$STAGE1_ENGINE" = interpreter ] && [ ! -x "$BIN/t1_image" ]; then
    echo "fixpoint: the interpreter Stage 1 is needed (no usable cached previous image) but there is no t1_image under $BIN: not building Rust here" >&2
    exit 2
fi
if [ "$STAGE1_ENGINE" = interpreter ]; then
echo "fixpoint: Stage 1 building (25-70 min, longer under memory pressure)"
"$BIN/t1_image" --compiler "$SRC" \
    --entry शृङ्खला स्वपरीक्षास्वप्रतिबिम्बम् \
    -o "$S1" "$SRC"/*.t1 > "$OUT/stage1.log" 2>&1
fi
echo "$STAGE1_ENGINE" > "$OUT/stage1.engine"
if [ ! -s "$S1" ]; then
    echo "fixpoint: Stage 1 produced no image; see $OUT/stage1.log" >&2
    exit 2
fi
echo "fixpoint: Stage 1 $(wc -c < "$S1" | tr -d ' ') octets"

# ── Stage 2: that image compiles the same sources, natively ─────────────────
#
# `yantra-run` EXITS NON-ZERO ON A BAD HALT since `W-344`; this check stays
# anyway. The margin it replaces read "ALWAYS EXITS ZERO — the status is on
# stderr", and the incident behind it is why: an earlier attempt halted
# `StepLimit` after two seconds and exited 0, so trusting the exit code would
# have compared against a run that executed almost nothing. The exit code is
# now trustworthy AND the stderr grep is kept, because this script checks for
# `halt: Finisher` specifically — a successful finisher — where the exit code
# only distinguishes success from everything else. RAM defaults
# to 20 MiB against a ~1.6 GB high water, so both ceilings are set here.
# FIXPOINT_STAGE2=native. Owner ruling (coordinator, 2026-10-09): "native Stage 2 fixpoint APPROVED, with scheduled full-yantra sanity runs … Stage 1 stays on the interpreter."
# Stage 2 runs as the NATIVE
# translation of Stage 1 (tools/native-spike/anuvada.py: RV64 image -> static x86-64 ELF,
# ~5 s; the run ~22 s against yantra's ~15 min for the same 55.4 G instructions). The
# comparison below is UNCHANGED: Stage 2's output ELF must be byte-identical to Stage 1.
# Stage 1 stays on the interpreter. Authority stays with yantra: a native Stage 2 is a
# fast pre-check, so an internal tool runs the full yantra Stage 2 whenever crates/yantra/src
# or tools/native-spike/ changed, on every 10th landing, and a nightly run is proposed
# (see an internal tool). The engine is RECORDED: $OUT/stage2.engine and, on a pass, a line in
# $(git dir)/fixpoint-engine (a sidecar, so the fixpoint-pass line's shape is untouched).
# Native refuses what yantra-only ratchets watch for (threads, sockets, CSR/counter reads)
# at run time, so such a run reaches no finisher and fails the check below.
ENGINE="${FIXPOINT_STAGE2:-native}"
echo "$ENGINE" > "$OUT/stage2.engine"
if [ "$ENGINE" = native ]; then
    echo "fixpoint: Stage 2 running NATIVELY (anuvada.py translation of Stage 1)"
    NAT="$OUT/stage1.native"
    python3 "$ROOT/tools/native-spike/anuvada.py" --target "$NTGT" "$S1" "$NAT" > "$OUT/stage2.translate.log" 2>&1 || {
        echo "fixpoint: the translator refused Stage 1; see $OUT/stage2.translate.log" >&2; exit 2; }
    YANTRA_INPUT="$OUT/corpus.blob" \
    YANTRA_INPUT_NAME=शृङ्खला \
    YANTRA_RAM="${YANTRA_RAM:-2684354560}" \
    YANTRA_STEPS="${YANTRA_STEPS:-4000000000000}" \
        "$NAT" > "$OUT/stage2.sink" 2> "$OUT/stage2.log"
    echo "fixpoint: Stage 2 engine=native, target $NTGT (translator sha256 $(sha256sum "$ROOT/tools/native-spike/anuvada.py" | cut -c1-16))" >> "$OUT/stage2.log"
else
    echo "fixpoint: FIXPOINT_STAGE2 must be 'native' (yantra Stage 2 retired, L28), not '$ENGINE'" >&2; exit 2
fi

# ── W-376, the ADR-0040 addendum's ratchet: a fixpoint image is never THREADED ─
# A threaded image's run is a function of its schedule as well as its input, so a
# compiler image that declared SASTHRDS would make Stage 2 a statement about a
# schedule. `yantra-run` prints `threads: N` whenever the tag is found, before it
# refuses a threaded image given no event log, so this check comes FIRST: the
# finisher check below would see that refusal only as "no finisher". Pinned by
# the Rust host source (removed in v1.2).
if grep -aq "^threads:" "$OUT/stage2.log"; then
    echo "fixpoint: Stage 2 is a THREADED image — $(grep -a -m1 '^threads:' "$OUT/stage2.log")" >&2
    echo "  (ADR-0040 addendum / W-376: no fixpoint image may declare SASTHRDS; find it in the corpus)" >&2
    exit 2
fi
# ── W-377, the sockets addendum's ratchet: a fixpoint image never touches the SOCKET ─
# Stage 2 runs with no socket, so a compiler image that reached the window at
# 0x1000_0110 halts `Device` and never reaches the finisher, which the check below
# would report only as "no finisher". `yantra-run` says `socket: …` whenever the
# device is configured or touched, so this check names the cause FIRST. Pinned by
# the Rust host source (removed in v1.2).
if grep -aq "^socket:" "$OUT/stage2.log"; then
    echo "fixpoint: Stage 2 touched the SOCKET device — $(grep -a -m1 '^socket:' "$OUT/stage2.log")" >&2
    echo "  (ADR-0040 addendum / W-377: no fixpoint image may reach the socket window; find it in the corpus)" >&2
    exit 2
fi
if ! grep -aq "halt: Finisher" "$OUT/stage2.log"; then
    echo "fixpoint: Stage 2 did not reach a finisher — $(tail -1 "$OUT/stage2.log")" >&2
    echo "  (read $OUT/stage2.log; since W-344 yantra-run also exits non-zero here)" >&2
    exit 2
fi
# ── W-372, ADR-0040's ratchet: a fixpoint image never reads the COUNTER ─────
# A store to WAIT is already refused above: it halts `Wait`, never `Finisher`. A
# counter read (W-374) does not halt, so `yantra-run` reports it on its own line
# and it is refused here. A compiler that read its own retired count could make
# Stage 2 differ from Stage 1 by self-reference. Pinned by
# the Rust host source (removed in v1.2).
if grep -aq "^counter:" "$OUT/stage2.log"; then
    echo "fixpoint: Stage 2 read the retired-instruction counter — $(grep -a '^counter:' "$OUT/stage2.log")" >&2
    echo "  (ADR-0040 / W-372: no fixpoint image may read it; find the read in the corpus)" >&2
    exit 2
fi
grep -aE "halt:|ram:" "$OUT/stage2.log" | sed 's/^/  /'

# ── Stage 2's halt status, read against the rung's OWN table ────────────────
#
# `shrinkhala.t1:3551` states it: **`१२०० built · १२०१ built an EMPTY image ·
# १२०२ no input arrived`**. So `1200` is the outcome this round wants, and the
# status is a named result, not a Unix exit code — `0x3333 | (n<<16)` encodes it,
# which is the FAILURE word of the halt protocol whatever `n` says.
#
# AND STAGE 1'S OWN `predict:` LINE MUST NOT BE READ AS THE EXPECTED VALUE. It
# prints `interpreted … -> 1202; the image's exit status must equal it`, and that
# sentence cannot hold on this path: `t1_image` runs the prediction with NO
# corpus on the input channel, `शृङ्खला:3560-3563` returns `१२०२` the moment
# `निवेशपाठः ॱ दैर्घ्य` is `०`, and Stage 2 is handed `YANTRA_INPUT` and so
# reaches `१२००`. Predicted 1202 / native 1200 was measured on BOTH the
# 2026-09-28 round (1,399,578 octets) and the 2026-09-29 ADR-0042 round
# (1,398,802) — identical halt word `78656307` — and it is not a divergence
# between the engines. It is one engine asked about an empty input and the other
# about the corpus. Equality there would mean Stage 2 compiled NOTHING.
#
# REPORTED, NOT GATED: byte-identity below is what the fixpoint is.
actual=$(sed -n 's/.*status: Some(\([0-9][0-9]*\)).*/\1/p' "$OUT/stage2.log" | tail -1)
case "${actual:-}" in
    1200) echo "  status:  $actual — BUILT (shrinkhala.t1:3551)" ;;
    1201) echo "  status:  $actual — built an EMPTY image; the octet count below is of nothing" >&2 ;;
    1202) echo "  status:  $actual — NO INPUT ARRIVED; Stage 2 compiled nothing" >&2 ;;
    "")   echo "  status:  NOT READ — no 'status: Some(n)' in stage2.log" >&2 ;;
    *)    echo "  status:  $actual — not one of 1200/1201/1202; read shrinkhala.t1:3551" >&2 ;;
esac

# The rung prints its image between markers, so the sink carries one octet of
# marker on each side of the ELF.
if ! elf_from_sink "$OUT/stage2.sink" "$OUT/stage2.elf"; then
    echo "fixpoint: the sink holds no ELF — Stage 2 emitted no image" >&2
    verdict=2
else
    n1=$(wc -c < "$S1" | tr -d ' '); n2=$(wc -c < "$OUT/stage2.elf" | tr -d ' ')
    echo "fixpoint: Stage 2 $n2 octets"
    if cmp -s "$S1" "$OUT/stage2.elf"; then
        echo "FIXPOINT HOLDS: $n1 octets, byte-identical"
        verdict=0
    else
        m=$n1; [ "$n2" -ge "$m" ] || m=$n2
        first=$(cmp -l "$S1" "$OUT/stage2.elf" 2>/dev/null | head -1 | awk '{print $1 - 1}')
        [ -n "$first" ] || first=$m
        echo "FIXPOINT BROKEN: Stage 1 $n1, Stage 2 $n2, delta $((n1 - n2)), first difference at offset $first" >&2
        # Offset 96 is `p_filesz` in the first program header — a DERIVED length. A
        # divergence there is a consequence, not a cause; the cause is in the
        # instruction stream. Diff the `addi sp,sp,-N` prologues across both images
        # (opcode 0x13, rd=2, funct3=0, rs1=2, imm<0) before reading section layout.
        if [ "$first" -lt 120 ]; then
            echo "  the first difference is inside the ELF/program header, which holds DERIVED lengths — look at the instruction stream, not the layout" >&2
        fi
        verdict=1
    fi
fi
# THE PASS IS RECORDED, so the tick's pre-push check can read it (W-354 family):
# one line under the git dir naming the seven inputs as measured, and the two
# images' and two binaries' sha256 as evidence.
if [ "$verdict" -eq 0 ]; then
    printf '%s stage1_engine='"$STAGE1_ENGINE"' stage2_engine=%s stage2_sha256=%s at=%s\n' "$(git -C "$ROOT" rev-parse HEAD)" "$ENGINE" \
        "$(sha256_of "$OUT/stage2.elf")" \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$(git -C "$ROOT" rev-parse --absolute-git-dir)/fixpoint-engine"
fi
exit "$verdict"
