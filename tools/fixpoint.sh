#!/usr/bin/env bash
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
#   yantra-run reads the BLOB     (YANTRA_INPUT, packed by pack-corpus.py)
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
for b in t1_image yantra-run; do
    if [ ! -x "$BIN/$b" ]; then
        echo "fixpoint: no $b under $BIN" >&2
        echo "  build with: cargo build --release -p sadhana -p yantra" >&2
        echo "  or point SASSEMBLY_BIN at a directory holding both" >&2
        exit 2
    fi
done

# ── the blob, packed NOW, from the tree about to be compiled ────────────────
echo "fixpoint: packing the corpus from $SRC"
python3 "$ROOT/tools/pack-corpus.py" "$OUT/corpus.blob" "$SRC"/*.t1 | tail -1 || exit 2

# A blob older than any source it claims to contain is the failure this script
# exists to prevent. `pack-corpus.py` just wrote it, so this can only fire if
# something raced us — but the check costs nothing and the alternative cost
# 3.5 hours once.
for f in "$SRC"/*.t1; do
    if [ "$f" -nt "$OUT/corpus.blob" ]; then
        echo "fixpoint: $f is NEWER than the blob just packed — refusing" >&2
        exit 2
    fi
done

# ── Stage 1: the interpreted compiler builds the image ──────────────────────
echo "fixpoint: Stage 1 building (25-70 min, longer under memory pressure)"
S1="$OUT/stage1.elf"
"$BIN/t1_image" --compiler "$SRC" \
    --entry शृङ्खला स्वपरीक्षास्वप्रतिबिम्बम् \
    -o "$S1" "$SRC"/*.t1 > "$OUT/stage1.log" 2>&1
if [ ! -s "$S1" ]; then
    echo "fixpoint: Stage 1 produced no image; see $OUT/stage1.log" >&2
    exit 2
fi
echo "fixpoint: Stage 1 $(wc -c < "$S1" | tr -d ' ') octets"

# ── Stage 2: that image compiles the same sources, natively ─────────────────
#
# `yantra-run` ALWAYS EXITS ZERO — the status is on stderr. An earlier attempt
# halted `StepLimit` after two seconds and exited 0; trusting the exit code
# would have compared against a run that executed almost nothing. RAM defaults
# to 20 MiB against a ~1.6 GB high water, so both ceilings are set here.
echo "fixpoint: Stage 2 running (40-110 min)"
YANTRA_INPUT="$OUT/corpus.blob" \
YANTRA_INPUT_NAME=शृङ्खला \
YANTRA_RAM="${YANTRA_RAM:-2684354560}" \
YANTRA_STEPS="${YANTRA_STEPS:-4000000000000}" \
YANTRA_WATERMARK=1 \
    "$BIN/yantra-run" "$S1" > "$OUT/stage2.sink" 2> "$OUT/stage2.log"

if ! grep -q "halt: Finisher" "$OUT/stage2.log"; then
    echo "fixpoint: Stage 2 did not reach a finisher — $(tail -1 "$OUT/stage2.log")" >&2
    echo "  (yantra-run exits 0 whatever happens; read $OUT/stage2.log)" >&2
    exit 2
fi
grep -E "halt:|ram:" "$OUT/stage2.log" | sed 's/^/  /'

# ── Stage 2's halt status, read against the rung's OWN table ────────────────
#
# `shrinkhala.t1:3528` states it: **`१२०० built · १२०१ built an EMPTY image ·
# १२०२ no input arrived`**. So `1200` is the outcome this round wants, and the
# status is a named result, not a Unix exit code — `0x3333 | (n<<16)` encodes it,
# which is the FAILURE word of the halt protocol whatever `n` says.
#
# AND STAGE 1'S OWN `predict:` LINE MUST NOT BE READ AS THE EXPECTED VALUE. It
# prints `interpreted … -> 1202; the image's exit status must equal it`, and that
# sentence cannot hold on this path: `t1_image` runs the prediction with NO
# corpus on the input channel, `शृङ्खला:3537-3540` returns `१२०२` the moment
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
    1200) echo "  status:  $actual — BUILT (shrinkhala.t1:3528)" ;;
    1201) echo "  status:  $actual — built an EMPTY image; the octet count below is of nothing" >&2 ;;
    1202) echo "  status:  $actual — NO INPUT ARRIVED; Stage 2 compiled nothing" >&2 ;;
    "")   echo "  status:  NOT READ — no 'status: Some(n)' in stage2.log" >&2 ;;
    *)    echo "  status:  $actual — not one of 1200/1201/1202; read shrinkhala.t1:3528" >&2 ;;
esac

# The rung prints its image between markers, so the sink carries one octet of
# marker on each side of the ELF.
python3 - "$S1" "$OUT/stage2.sink" "$OUT/stage2.elf" <<'PY'
import sys
s1 = open(sys.argv[1], "rb").read()
sink = open(sys.argv[2], "rb").read()
off = sink.find(b"\x7fELF")
if off < 0:
    print("fixpoint: the sink holds no ELF — Stage 2 emitted no image", file=sys.stderr)
    sys.exit(2)
s2 = sink[off:len(sink) - 1]
open(sys.argv[3], "wb").write(s2)
print(f"fixpoint: Stage 2 {len(s2)} octets")
if s1 == s2:
    print(f"FIXPOINT HOLDS: {len(s1)} octets, byte-identical")
    sys.exit(0)
n = min(len(s1), len(s2))
first = next((i for i in range(n) if s1[i] != s2[i]), n)
print(f"FIXPOINT BROKEN: Stage 1 {len(s1)}, Stage 2 {len(s2)}, "
      f"delta {len(s1) - len(s2)}, first difference at offset {first}",
      file=sys.stderr)
# Offset 96 is `p_filesz` in the first program header — a DERIVED length. A
# divergence there is a consequence, not a cause; the cause is in the
# instruction stream. Diff the `addi sp,sp,-N` prologues across both images
# (opcode 0x13, rd=2, funct3=0, rs1=2, imm<0) before reading section layout.
if first < 120:
    print("  the first difference is inside the ELF/program header, which holds "
          "DERIVED lengths — look at the instruction stream, not the layout",
          file=sys.stderr)
sys.exit(1)
PY
