#!/bin/sh
# The same ELF, on both paths — task `F-017`.
#
# ## What this asserts, and why it is not already covered
#
# `F-001`'s sentence is *"apps that run in the browser and outside it"*, and the whole
# weight of it rests on one word: **the same** app. Two runtimes that each execute *a*
# program built from *a* source prove far less than one artefact executing in both.
#
# That identity is currently true **by construction** — `tools/build-sassembly-web.sh`
# generates the page and never commits it, so the ELF inside it came out of `sadhana` in
# that same run. A by-construction argument is exactly as durable as the construction: the
# day someone caches the page, or embeds a checked-in copy, or adds a second assembler
# path for the web, the claim silently becomes false and every test still passes.
#
# So this asserts it directly:
#
#   1. the bytes `sadhana` emits and the bytes embedded in the generated page are equal
#   2. those bytes, run on `qemu-system-riscv64`, produce the expected output
#   3. those same bytes, run through the wasm module, produce the SAME output
#
# (1) alone would be satisfied by two identical files nobody executes. (2) and (3) alone
# would be satisfied by two different programs that happen to print the same thing. The
# three together say the sentence.
#
# ## Absence is loud
#
# Exits **77** when `qemu-system-riscv64`, `node`, `cargo` or the `wasm32-unknown-unknown`
# target is missing — `W-087`'s convention, and `tools/gate.sh` renders 77 as neither pass
# nor fail. A check that cannot run has not passed.
#
# QEMU is bounded by an explicit `kill -9` watcher and never by `perl -e alarm`: QEMU
# installs its own SIGALRM handler and outlives it (`W-060`). `timeout` is not on this
# machine.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

# The expectation is per-program, and an unknown one is refused rather than run.
#
# This parameter used to accept any name against a single hardcoded expectation, so
# `check-sassembly-identity.sh bare-metal` reported "QEMU did not print the expected text"
# — literally true, since `bare-metal` prints nothing, and it reads as "the pipeline is
# broken" when it means "this check has no expectation for what you asked". One message
# for two causes sends the reader to the wrong fix (`W-069`, `C-002c`). Found by
# a peer session, using the parameter as a mutation.
PROGRAM=${1:-namaste}
case "$PROGRAM" in
  namaste)    EXPECT_SUBSTRING="नमस्ते संसार" ;;
  bare-metal)
    # It writes only the finisher. Asserting "printed nothing" would pass on a program
    # that failed to start, so this check has no honest expectation for it and says so.
    echo "  REFUSED  spec/bare-metal.sas writes no characters, so 'the same text out of"
    echo "           both runtimes' has nothing to compare. Identity for it needs a"
    echo "           different assertion than this check makes — exit status, not output."
    exit 1 ;;
  *)
    echo "  REFUSED  no expectation is recorded for spec/$PROGRAM.sas."
    echo "           This check asserts that a KNOWN program produces KNOWN text on both"
    echo "           paths. Running it against an unknown one would report a pipeline"
    echo "           failure for a missing expectation. Add the expected text above."
    exit 1 ;;
esac

missing=
command -v cargo               >/dev/null 2>&1 || missing="$missing cargo"
command -v node                >/dev/null 2>&1 || missing="$missing node"
command -v qemu-system-riscv64 >/dev/null 2>&1 || missing="$missing qemu-system-riscv64"
rustc --print target-list 2>/dev/null | grep -qx wasm32-unknown-unknown \
  || missing="$missing wasm32-unknown-unknown"
if [ -n "$missing" ]; then
  echo "SKIPPED:$missing not available — absent tooling on this machine, not a tree fault"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77
fi

work=$(mktemp -d)
trap 'rm -f "$work"/* 2>/dev/null || true' EXIT

# ---------------------------------------------------------------- 1. the bytes
say() { printf '  %-34s %s\n' "$1" "$2"; }

cargo run -q -p sadhana -- "spec/$PROGRAM.sas" "$work/direct.elf" >/dev/null 2>&1 || {
  echo "  FAIL  sadhana could not assemble spec/$PROGRAM.sas"; exit 1; }

# ONE GATE RUN, ONE PAGE BUILD — the CONSUMING half. `check-sassembly-page.sh` builds
# this same page for its own assertions; under `gate.sh` it now runs first and hands this
# one over, so the ~25 s build is paid once instead of twice over one identical file.
#
# THE SELFTEST RUNS HERE AND NOT IN THE PRODUCER. This is the side that DECIDES whether a
# foreign page is believed, so a library whose refusals have stopped refusing turns this
# check's three assertions into assertions about somebody else's tree. Publishing, by
# contrast, is advisory and cannot fail.
./tools/lib/page-handoff.sh --selftest >/dev/null || {
  echo "  FAIL  tools/lib/page-handoff.sh fails its own selftest, so its verdict about whether"
  echo "        an offered page is this tree's cannot be believed. Run it directly to see which"
  echo "        probe went red; this check will not accept a handed-over page on a broken proof."
  exit 1; }
. ./tools/lib/page-handoff.sh

# NOT `page=$(page_handoff_accept ...)`. A command substitution is a SUBSHELL and the
# decline REASON is set inside it, so that spelling printed "building its own" with an
# empty line under it — the three states back to two, in the one case that matters.
# Measured live on a prov naming another tree before this read as it does.
# shellcheck disable=SC2086
if page_handoff_accept $PAGE_HANDOFF_SIDE; then
  page=$page_handoff_page
  say "the page" "handed over, proved this tree's"
  say "" "$page"
else
  # A DECLINE IS NOT A FAULT and it is not silent either. Every standalone run lands here
  # with "SASSEMBLY_PAGE_HANDOFF is unset", which is the invariant: this check must never
  # depend on the other having run.
  say "the page" "building its own"
  say "" "$page_handoff_why"
  page=$work/page.html
  # 77 IS A SKIP, NOT A FAILURE — `W-087`'s convention, which this call was the one
  # place not to honour. `build-sassembly-web.sh` exits 77 when the wasm32 target is
  # absent, saying so itself: "Exiting 77 rather than 0 — a build that cannot run has
  # not succeeded." This turned that into `exit 1`, so a missing toolchain component
  # rendered as a FAILED gate step.
  #
  # It was not cosmetic. `gate.sh`'s "sassembly identity" step is one of the reds that
  # has been blocking `salvage()` — the rail reaches the gate, the gate fails, the
  # cycle stashes instead of landing. Measured 2026-09-16: FAILED in every retained
  # salvage verdict, on a host with no `wasm32-unknown-unknown` installed.
  #
  # `run()` in `gate.sh` already renders 77 as SKIPPED with a reason and feeds
  # `GATE_STRICT`, so propagating it is all that was needed to make an absent
  # toolchain say what it is.
  # `set +e` AROUND IT — `:35` sets `-eu`, so a bare failing command exits the script
  # HERE and `rc=$?` never runs. Caught by tracing: exit 77 with EMPTY output, so
  # `gate.sh` would have rendered "SKIPPED — it did not say why". Third instance of
  # this exact shape tonight, twice in code I had just written.
  set +e
  sh tools/build-sassembly-web.sh "$page" >/dev/null 2>&1
  rc=$?
  set -e
  if [ "$rc" -eq 77 ]; then
    # `not a tree fault` IS THE CLASSIFICATION `salvage` GREPS FOR, and it is the
    # same phrase check-fetch-log.sh carries — an absent `wasm32-unknown-unknown`
    # is a fault of THIS MACHINE that no commit caused and no patch can fix, and
    # holding a verified change hostage to it is exactly what loop-agent.sh:481
    # says not to do. Unmarked, it declined ten consecutive recovery cycles whose
    # gates had otherwise passed (2026-09-20). It does NOT make the skip a pass:
    # `GATE_STRICT=1` still reds on it where the toolchain is guaranteed.
    echo "SKIPPED: the page build cannot run here (wasm32 target absent) — a toolchain fault of this machine, not a tree fault; identity unverified."
    exit 77
  elif [ "$rc" -ne 0 ]; then
    echo "  FAIL  the page build failed (rc=$rc); run it alone to see why"; exit 1
  fi
fi

# The page embeds each ELF as base64 under its program name.
#
# The shape gained a level in `F-001g`: `PROGRAMS` was `{name: "<base64>"}` and is
# now `{name: {"kind": "proof"|"application", "elf": "<base64>"}}`, because the page
# now runs an application as well as the OS proofs. Parsing the OBJECT rather than
# regexing for a bare string means the next field added here does not break this
# check silently — and the old regex DID break, matching nothing and reporting "no
# embedded ELF named namaste" for a page that carried it perfectly well.
node -e '
  const fs = require("fs");
  const src = fs.readFileSync(process.argv[1], "utf8");
  const name = process.argv[2];
  const decl = src.match(/const PROGRAMS = (\{[\s\S]*?\});/);
  if (!decl) { console.error("no PROGRAMS declaration in the generated page"); process.exit(1); }
  // The declaration is JS, not JSON: it carries a trailing comma, which JSON.parse
  // rejects. Strip trailing commas before object/array close rather than hand-rolling
  // a parser -- the alternative was the regex this replaced, which broke silently.
  const programs = JSON.parse(decl[1].replace(/,(\s*[}\]])/g, "$1"));
  const entry = programs[name];
  if (!entry) {
    console.error("no embedded program named " + name +
                  " -- page carries: " + Object.keys(programs).join(", "));
    process.exit(1);
  }
  const b64 = typeof entry === "string" ? entry : entry.elf;
  if (!b64) { console.error("program " + name + " carries no elf field"); process.exit(1); }
  const m = [null, b64];
  fs.writeFileSync(process.argv[3], Buffer.from(m[1], "base64"));
' "$page" "$PROGRAM" "$work/embedded.elf" || {
  echo "  FAIL  could not extract the embedded ELF from the generated page"; exit 1; }

a=$(shasum -a 256 "$work/direct.elf"   | cut -d' ' -f1)
b=$(shasum -a 256 "$work/embedded.elf" | cut -d' ' -f1)
if [ "$a" != "$b" ]; then
  echo "  FAIL  the page does not carry the bytes sadhana emits"
  echo "        sadhana  $a"
  echo "        page     $b"
  echo "        The demo's claim is that ONE artefact runs in two places. It does not."
  exit 1
fi
say "one artefact, both paths" "sha256 $(echo "$a" | cut -c1-16)"

# ---------------------------------------------------- 2. those bytes, on hardware
qemu-system-riscv64 -machine virt -bios none -nographic \
  -kernel "$work/direct.elf" > "$work/qemu.out" 2>&1 &
qpid=$!
( sleep 20; kill -9 "$qpid" 2>/dev/null ) & watcher=$!
wait "$qpid" 2>/dev/null || true
kill "$watcher" 2>/dev/null || true
wait "$watcher" 2>/dev/null || true

# `-nographic` ends every line with CR; an anchored match silently fails without this.
qemu_out=$(tr -d '\r' < "$work/qemu.out")
case "$qemu_out" in
  *"$EXPECT_SUBSTRING"*) say "qemu-system-riscv64" "printed the expected text" ;;
  *) echo "  FAIL  QEMU did not print the expected text"
     echo "        got: $(printf '%s' "$qemu_out" | head -3)"; exit 1 ;;
esac

# ------------------------------------------------- 3. the SAME bytes, in the VM
wasm=crates/yantra-wasm/target/wasm32-unknown-unknown/release/yantra_wasm.wasm
wasm_out=$(node -e '
  const fs = require("fs");
  (async () => {
    const elf = fs.readFileSync(process.argv[2]);
    const { instance } = await WebAssembly.instantiate(fs.readFileSync(process.argv[1]), {});
    const e = instance.exports, mem = () => new Uint8Array(e.memory.buffer);
    const at = e.yantra_alloc(elf.length);
    mem().set(elf, at);
    const code = e.yantra_run(1 << 20, 1000000);
    const out = new TextDecoder().decode(
      mem().slice(e.yantra_out_ptr(), e.yantra_out_ptr() + e.yantra_out_len()));
    if (code !== 0) { console.error("halt code " + code); process.exit(1); }
    process.stdout.write(out);
  })();
' "$wasm" "$work/embedded.elf") || {
  echo "  FAIL  the wasm module could not run the embedded ELF"; exit 1; }

case "$wasm_out" in
  *"$EXPECT_SUBSTRING"*) say "yantra, wasm" "printed the expected text" ;;
  *) echo "  FAIL  the wasm module did not print the expected text"
     echo "        got: $(printf '%s' "$wasm_out" | head -3)"; exit 1 ;;
esac

# The two runtimes must agree on more than "both contained the phrase". Trailing
# whitespace differs — QEMU's console adds none, the UART stream carries the program's
# own newline — so compare the text the program actually emitted.
if [ "$(printf '%s' "$qemu_out" | tr -d '\n\r ')" != "$(printf '%s' "$wasm_out" | tr -d '\n\r ')" ]; then
  echo "  FAIL  the two runtimes printed different text from the same bytes"
  echo "        qemu: $(printf '%s' "$qemu_out" | head -1)"
  echo "        wasm: $(printf '%s' "$wasm_out" | head -1)"
  exit 1
fi
say "both runtimes agree" "same bytes in, same text out"

echo "identity holds for spec/$PROGRAM.sas."
