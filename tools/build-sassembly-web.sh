#!/bin/sh
# Build the Sassembly-in-a-browser page — task `F-001`.
#
# ## Why this generates rather than commits, and writes OUTSIDE the repository
#
# The whole claim is that **the browser runs the bytes `sadhana` emits**. A committed
# `sassembly.html` is a copy of those bytes taken at some past moment, and a copy can go
# stale against the assembler with nothing to notice — the hashes stop matching and the
# demo quietly becomes a different artefact wearing the same filename.
#
# Generating it makes the property true *by construction*: the ELF in the page came out of
# `sadhana` seconds earlier, in this run, from the tracked source. What is tracked is the
# template, the glue and this script — the things a person wrote.
#
# **And it is written outside the working tree, which is not tidiness.** The first version
# wrote `the generated sassembly page` and gitignored it. The gate then went red: `cargo run -p
# secrets` reads untracked files too, and a 37 KB base64 module contains `ASIA…` by chance,
# which is the shape of an AWS session token. Two findings, one red tree, from an artefact
# that is not source.
#
# The fix is not to teach the scanner to skip a path — a scanner with a blind spot is a
# place to hide a secret. It is that a generated demo has no business inside a repository
# whose gate reads everything in it. Pass a path to put it somewhere specific; the default
# is a fresh temporary directory.
#
# ## Exit 77 when it cannot run
#
# This needs the `wasm32-unknown-unknown` target, which is not part of a default toolchain.
# On a machine without it the honest answer is "did not run", not "passed" — `W-087` found
# 29 skip paths across 27 scripts that said the latter. 77 is GNU Automake's SKIP code and
# `the internal gate tooling` renders it as neither pass nor fail.
#
# Usage: tools/build-sassembly-web.sh [output.html]   (from the repository root)
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$root"

command -v cargo >/dev/null 2>&1 || {
  echo "SKIPPED: cargo is not installed, so nothing can be built"
  echo "         Exiting 77 rather than 0 — a build that cannot run has not succeeded."
  exit 77; }

# `rustup target list --installed` is the question that matters. Asking whether `rustup`
# exists is a different one: a toolchain installed another way still compiles.
#
# TWO CAUSES, TWO VERDICTS, AND THIS USED TO CONFLATE THEM. The target being
# absent is a fault of the MACHINE (77, skip); the crate failing to compile is a
# fault of the TREE (1, red). One `if` reported both as "the target is not
# available", and `2>/dev/null` threw away the compiler's reason.
#
# MEASURED 2026-09-24: on a machine WITH the target installed, `yantra-wasm`
# failed with two `error[E0004]: non-exhaustive patterns: &Halt::BeyondRam not
# covered` — `yantra` grew a variant and these matches did not. The check said
# "the wasm32-unknown-unknown target is not available" and told the reader to
# run `rustup target add`, which was already done and changed nothing.
# `check-sassembly-identity.sh` propagated the 77 as SKIPPED, so a real defect
# rode through the gate wearing "not a tree fault" for as long as nobody looked.
if ! rustc --print target-list 2>/dev/null | grep -qx wasm32-unknown-unknown; then
  echo "SKIPPED: the wasm32-unknown-unknown target is not available"
  echo "         rustup target add wasm32-unknown-unknown"
  echo "         Exiting 77 rather than 0 — a build that cannot run has not succeeded."
  exit 77
fi
# Stderr is KEPT. Whatever rustc says is the only thing that can tell a reader
# which of the two they have.
# YANTRA_WASM=<path>: the yantra wasm module to use. The module builder (yantra/Rust) is retired;
# unset, there is no subject to build: SKIPPED (77), never a PASS.
if [ -z "${YANTRA_WASM:-}" ]; then
  echo "SKIPPED: YANTRA_WASM unset (no in-tree yantra wasm builder)"; exit 77
fi

wasm=$YANTRA_WASM
[ -f "$wasm" ] || { echo "  FAIL  the wasm build reported success and produced no module"; exit 1; }

# ── THE SECOND MODULE: THE ASSEMBLER, which this script did not build ───────
#
# CO-sw (2026-10-19): the module is the NATIVE one, tools/t1-interp/wasm/build-sadhana.sh
# (spec/entry/assemble.t1 -> anuvada --target wasm32 -> zero-import seal, ABI sadhana-abi.wat);
# crates/sadhana-wasm was retired to archive/rust/sadhana-wasm. SADHANA_WASM=<path> uses that module instead.
# build-sadhana.sh defaults its own ANUVADA_RUN/ZEROIMP_RUN; exit 77 = a needed binary is truly absent
# (t1_image, yantra-run, node; it names which): SKIPPED, never a PASS.
# The module's sadhana_rev is `tools/src-rev.sh --t1-assembler` (assemble.t1 + crates/sadhana-t1/src/*.t1), and the page ELFs
# below are assembled by the SAME T1 assembler (the as.elf beside the wasm) under yantra-run, so both stamps name one source.
# If only a bare SADHANA_WASM is given (no .elf beside it) the ELFs come from `cargo run -p sadhana`, stamped with
# crates/sadhana/src: the page then reads PROVENANCE: REV/MISMATCH (honest: the module was not built from that source).
if [ -z "${SADHANA_WASM:-}" ]; then
  _sw=target/native-wasm/sadhana_wasm.wasm; mkdir -p target/native-wasm
  _rc=0; sh tools/t1-interp/wasm/build-sadhana.sh "$_sw" >"$_sw.log" 2>&1 || _rc=$?
  [ "$_rc" -ne 77 ] || { rm -f "$_sw.log"; echo "SKIPPED: native sadhana wasm cannot build (ANUVADA_RUN/ZEROIMP_RUN/t1_image)"; exit 77; }
  [ "$_rc" -eq 0 ] || { tail -5 "$_sw.log"; rm -f "$_sw.log"; echo "  FAIL  native sadhana wasm build rc=$_rc"; exit 1; }
  rm -f "$_sw.log"; SADHANA_WASM=$_sw
fi
sadhana_wasm=$SADHANA_WASM
[ -f "$sadhana_wasm" ] || {
  echo "  FAIL  the native sadhana wasm build reported success and produced no module"; exit 1; }

# ── WHICH ASSEMBLER ASSEMBLED THE ELFs — the figure the page did not have ───
#
# The page re-assembles each program's Devanagari in the tab and compares the octets
# against the ELF below. Three identical programs prove AGREEMENT TODAY and prove
# NOTHING about a page served tomorrow from a stale `sadhana-wasm`: two copies of the
# same wrong assembler agree with each other perfectly. So the page carries the stamp
# of the NATIVE assembler that assembled its ELFs, and the module carries its own
# (sealed in by `the Rust build script (removed in v1.2)`), and the page says whether they match.
#
# TAKEN FROM THE TREE AND NOT FROM THE BINARY, and the asymmetry is deliberate: the
# `cargo run -q -p sadhana` below compiles from this same `crates/sadhana/src`, so the
# tree's stamp IS that binary's provenance, whereas the wasm's must be sealed because a
# `.wasm` can arrive from anywhere. 77 (no sha256 tool) becomes `unknown` rather than a
# stamp that happens to match — the page renders that as a third state and not as
# agreement.
# The page ELFs are assembled by the T1 assembler (spec/entry/assemble.t1 image, `${SADHANA_WASM%.wasm}.elf`) when that image
# sits beside the wasm; its stamp is the T1-source one the wasm carries. Otherwise cargo's `sadhana` and its own crate stamp.
T1ASM=${SADHANA_WASM%.wasm}.elf
YR=target/release/yantra-run
if [ -s "$T1ASM" ] && { [ -x "$YR" ]; }; then
  asm=t1; native_rev=$(tools/src-rev.sh --t1-assembler 2>/dev/null) || native_rev=
  native_src="spec/entry/assemble.t1 + crates/sadhana-t1/src, T1 assembler"
else
  asm=cargo; native_rev=$(tools/src-rev.sh crates/sadhana/src 2>/dev/null) || native_rev=
  native_src="crates/sadhana/src, native side"
fi
[ -n "$native_rev" ] || native_rev=unknown

# ── WHICH INTERPRETER MEASURED THE SIZING FIGURES — the weaker position ─────
#
# The assembler's stamp above compares two things that AGREE: the page re-assembles
# each program and the octets either match or do not. The interpreter has no such
# second opinion. The halt status, the step count and the UART text on the page are
# produced by `yantra-wasm` ALONE, and the sizing line's `ram` and `touches` were
# measured by the NATIVE `yantra-run` this script runs below — a DIFFERENT BINARY.
# So a stale `yantra-wasm` shows a correct halt, a correct count and correct text
# while the figures beside them belong to an interpreter it is not, and no arm on
# this page could see it. This stamp is what makes that visible.
#
# Taken from the tree for the same reason as the assembler's: the native `yantra-run`
# used below is built from this `crates/yantra/src`, so the tree's stamp IS that
# binary's provenance, whereas the wasm's must be sealed because a `.wasm` can arrive
# from anywhere.
native_yantra_rev=$(tools/src-rev.sh crates/yantra/src 2>/dev/null) || native_yantra_rev=
[ -n "$native_yantra_rev" ] || native_yantra_rev=unknown

# The programs the page offers, in three kinds, because the page runs them three ways.
#
# **Boot proofs** are machines: entered at a reset vector in S-mode, writing to a device.
# Deliberately a short, named list rather than every `spec/*.sas` — most of the 36 in
# `spec/programs.tsv` need SYSTEM, FENCE or AMO, which this interpreter does not implement
# (see `crates/yantra`'s module documentation for why), and a page whose buttons mostly
# report `Unimplemented` demonstrates the opposite of the claim.
PROOFS="namaste bare-metal"

# **Applications** are guests: placed by a loader, running in U-mode, writing to a surface
# the supervisor handed them. `spec/atithi.sas` is the one program here that is one, and
# until `F-001g` this script excluded it BY NAME because the page had no loader and no
# supervisor. `yantra::host` is that arrangement and `yantra_host` is the export that
# spends it.
APPLICATIONS="atithi"

# Where an application is linked. NOT `sadhana`'s default of `0x80000000`: that is the
# QEMU `virt` reset vector — right for a boot proof and wrong for a guest — and it is the
# supervisor's own gigabyte, which `yantra::host` refuses by name rather than quietly
# handing an application the supervisor's pages. Written in the language, which is how
# `--स्थान` reads a number. `the Rust host source (removed in v1.2)` links it here too.
APP_LOAD=$(awk '$1 == "0x2000_0000" {print $2}' spec/application-load.tsv)
APP_LOAD_HEX=$(awk '$1 == "0x2000_0000" {gsub(/_/,"",$1); print $1}' spec/application-load.tsv)

out=$(mktemp -d)
trap 'rm -rf "$out" 2>/dev/null || true' EXIT; trap 'exit 143' INT TERM

progs=""
for p in $PROOFS $APPLICATIONS; do
  # The kind travels to the page: it chooses `yantra_host` over `yantra_run`, and which
  # of the two output panes is the one that should have filled.
  #
  # `at` is expanded UNQUOTED on purpose — it is two arguments or none, and `set --`
  # cannot be used here because `$1` is this script's own output path, read below.
  case " $APPLICATIONS " in
    *" $p "*) kind=app;   at="--स्थान $APP_LOAD" ;;
    *)        kind=proof; at= ;;
  esac
  [ -f "spec/$p.sas" ] || { echo "  FAIL  spec/$p.sas does not exist"; exit 1; }
  if [ "$asm" = t1 ]; then
    # assemble.t1 image: source on the input channel, ARGUMENT 1 = load address in ASCII decimal (absent = 0x8000_0000), ELF on stdout
    case $kind in app) ld=$((APP_LOAD_HEX)) ;; *) ld= ;; esac
    # shellcheck disable=SC2086
    env -u YANTRA_INPUT_ENTRY YANTRA_INPUT="spec/$p.sas" YANTRA_INPUT_NAME="$p" YANTRA_RAM=1073741824 \
      "$YR" "$T1ASM" $ld >"$out/$p.elf" 2>"$out/$p.log" && [ -s "$out/$p.elf" ] || {
      echo "  FAIL  the T1 assembler could not assemble spec/$p.sas"; sed 's/^/        /' "$out/$p.log" 2>/dev/null; exit 1; }
  else
  # shellcheck disable=SC2086
  "$root/tools/sadhana-native.sh" $at "spec/$p.sas" "$out/$p.elf" >/dev/null 2>&1 || {
    echo "  FAIL  sadhana could not assemble spec/$p.sas"; exit 1; }
  fi
  b64=$(base64 < "$out/$p.elf" | tr -d '\n')
  # ॥ THE SOURCE TRAVELS WITH THE ELF, AND IT TRAVELS AS BASE64 ॥ The ELF was on the
  # page and the source was not, so nothing in the tab could be assembled — the glue
  # had no subject. It is base64 and not a JSON string because the source is Devanagari
  # with embedded newlines: a shell that had to escape `"`, `\` and every newline into
  # a JS literal is a second encoder to get wrong, and `atob` is already on the page for
  # the ELF. `sas` and NOT `source`: the T1 entry already uses `source` for a PATH.
  sas=$(base64 < "spec/$p.sas" | tr -d '\n')
  progs="$progs\"$p\": {\"kind\": \"$kind\", \"elf\": \"$b64\", \"sas64\": \"$sas\"},"
  printf '  %-12s %-5s %s bytes  sha256 %s\n' "$p" "$kind" \
    "$(wc -c < "$out/$p.elf" | tr -d ' ')" \
    "$(shasum -a 256 "$out/$p.elf" | cut -c1-16)"
done

# ── A THIRD KIND, `t1`: the compiler image itself, NAMED and never built ────
#
# `tools/fixpoint.sh` Stage 1 is ~1560 s. A page build that spent twenty-six minutes
# before its first byte is a page build nobody runs, and the figures this button rests on
# — the declared extent and the high water — are properties of the ABI and of how much
# work one source is, neither of which moves between builds. So the image is an INPUT,
# named by `T1_IMAGE`, exactly as `the internal gate tooling` takes it.
#
# ABSENT `T1_IMAGE` THERE IS NO FOURTH BUTTON AND THAT IS NOT A SKIP. The page is
# complete without it; the deep gate builds it that way. `T1_IMAGE` set and unusable is a
# different thing and is NOT quiet: a path that does not exist is the caller's error (1),
# and a machine without python3 cannot pack a corpus (77, `W-087`'s convention).
#
# The three figures travel to the page as DATA and not as prose, because the page must
# name them and a number retyped into a caption is a number that goes stale:
#   ram      what the image demands for its declared span, read off the native loader's
#            OWN refusal rather than computed here — a second implementation of the
#            loader's span arithmetic is a second thing to drift.
#   touches  the high water of a real run of the same image on the same corpus, which is
#            what makes the ratio a measurement rather than an estimate.
#   budget   the u32 ceiling. `yantra_run` takes two u32s, so this is the page's whole
#            reach, and arm D of the budget check proves 2^32 is refused and not wrapped.
if [ -n "${T1_IMAGE:-}" ]; then
  [ -f "$T1_IMAGE" ] || {
    echo "  FAIL  T1_IMAGE=$T1_IMAGE names no file."
    echo "        Unset it for a page without the fourth button; a named image that is"
    echo "        not there is a caller's error and exiting 0 over it would hide the ask."
    exit 1; }
  command -v python3 >/dev/null 2>&1 || {
    echo "SKIPPED: python3 is absent, so the one-source corpus cannot be packed"
    echo "         Exiting 77 rather than 0 — the T1 button was asked for and not built."
    exit 77; }

  # ONE SOURCE, and the smallest in the corpus — `check-t1-image-wasm-budget.sh`'s own
  # choice, so the page and the check quote figures about the same work. PACKED BY
  # `tools/pack-corpus.sh` and never by a local copy of its `name NUL text NUL` framing.
  T1_SOURCE=crates/sadhana-t1/src/lib.t1
  # `T1_MODULE` IS REQUIRED AND UNREAD, AND THAT IS MEASURED, not assumed. `yantra-run`
  # refuses `YANTRA_INPUT` without `YANTRA_INPUT_NAME`, so one must be passed; but Stage 1's
  # entry (`स्वपरीक्षास्वप्रतिबिम्बम्`) takes each module's name out of the blob's own
  # `name NUL text NUL` frame — `lib` here — and never loads the `निवेशमण्डलनाम` global the
  # host writes. Arm E of `the internal gate tooling` pins it: the same run under
  # a name matching nothing retires the same count, halts on the same status and prints the
  # same octets. So the `inputName` this script puts on the page is the runner's required
  # companion to `input` and NOT a figure — the arm that ties the frame to `t.source` is
  # what asserts where the name actually comes from.
  T1_MODULE=शृङ्खला
  [ -f "$T1_SOURCE" ] || { echo "  FAIL  $T1_SOURCE does not exist"; exit 1; }
  sh tools/pack-corpus.sh "$out/one.blob" "$T1_SOURCE" >/dev/null 2>&1 || {
    echo "  FAIL  pack-corpus.sh could not pack $T1_SOURCE"; exit 1; }

  # Arm C of the budget check is what licenses the page to quote a native figure at all:
  # the two engines retire the SAME count on this image and this corpus. So the native
  # runner is not optional here — it is where `ram` and `touches` come from.
  native=target/release/yantra-run
  [ -x "$native" ] || {
    echo "  FAIL  yantra-run does not build, so the image's extent cannot be read"; exit 1; }

  # The declared extent: base-to-end over EVERY PT_LOAD at max(filesz, memsz), which is what
  # `Span::Declared` insists RAM cover. It used to be scraped from the FIRST "needs N bytes"
  # of a tiny-RAM refusal; since W-363 the image is two segments (text R+X, data+bss R+W) so
  # that first refusal names the TEXT segment (filesz 640724) and the ~512 MiB bss demand is
  # never reached. The native loader stays the judge: the extent is PROVEN exact against it
  # (RAM=extent loads, RAM=extent-1 is refused), so a wrong formula fails instead of passing.
  # elf_extent FILE: base-to-end over every PT_LOAD at max(filesz, memsz) (ELF64 little-endian; od reads host order and every host
  # here is little-endian). Prints nothing when the headers cannot be read or there is no PT_LOAD.
  elf_extent() {
    _f=$1; _sz=$(wc -c < "$_f" | tr -d ' ')
    [ "$_sz" -ge 58 ] || return 1
    _po=$(od -An -t u8 -j 32 -N 8 "$_f" | tr -d ' ')
    set -- $(od -An -t u2 -j 54 -N 4 "$_f"); _pe=$1; _pn=$2
    case "$_po$_pe$_pn" in ''|*[!0-9]*) return 1 ;; esac
    _i=0; _lo=; _hi=
    while [ "$_i" -lt "$_pn" ]; do
      _b=$((_po + _i * _pe)); _i=$((_i + 1))
      [ $((_b + 56)) -le "$_sz" ] || return 1
      _t=$(od -An -t u4 -j "$_b" -N 4 "$_f" | tr -d ' ')
      [ "$_t" = 1 ] || continue
      set -- $(od -An -t u8 -j $((_b + 16)) -N 8 "$_f"); _va=$1
      set -- $(od -An -t u8 -j $((_b + 32)) -N 16 "$_f"); _fz=$1; _mz=$2
      [ "$_fz" -ge "$_mz" ] && _n=$_fz || _n=$_mz
      [ -n "$_lo" ] && [ "$_lo" -le "$_va" ] || _lo=$_va
      [ -n "$_hi" ] && [ "$_hi" -ge $((_va + _n)) ] || _hi=$((_va + _n))
    done
    [ -z "$_lo" ] || echo $((_hi - _lo))
  }
  t1_ram=$(elf_extent "$T1_IMAGE")
  case "$t1_ram" in
    ''|*[!0-9]*) echo "  FAIL  could not read $T1_IMAGE's declared extent from its program headers"; exit 1 ;;
  esac
  ext_probe() {  # $1 = RAM; prints the native loader's refusal line, if any
    YANTRA_INPUT="$out/one.blob" YANTRA_INPUT_NAME="$T1_MODULE" YANTRA_RAM="$1" YANTRA_STEPS=1 \
      "$native" "$T1_IMAGE" 2>&1 >/dev/null | grep -a 'raise it' | head -1 || :
  }
  [ -z "$(ext_probe "$t1_ram")" ] || { echo "  FAIL  the native loader refuses $T1_IMAGE at its computed extent $t1_ram:"
    ext_probe "$t1_ram" | sed 's/^/        /'; exit 1; }
  [ -n "$(ext_probe "$((t1_ram - 1))")" ] || { echo "  FAIL  the native loader accepts $T1_IMAGE at extent-1, so $t1_ram is not the exact extent"; exit 1; }

  # `|| :` because this run does NOT exit 0 — `lib.t1` declares nothing and the compiler
  # says so with a status, which is the image SPEAKING and not a failure of this build.
  # Measured 2026-10-04: ~10 s.
  YANTRA_INPUT="$out/one.blob" YANTRA_INPUT_NAME="$T1_MODULE" YANTRA_RAM="$t1_ram" \
    YANTRA_STEPS=4294967295 YANTRA_WATERMARK=1 \
    "$native" "$T1_IMAGE" >/dev/null 2>"$out/t1.log" || :
  t1_touches=$(sed -n 's/^ram: high water \([0-9][0-9]*\) of .*/\1/p' "$out/t1.log" | tail -1)
  case "$t1_touches" in
    ''|*[!0-9]*) echo "  FAIL  no high water on the native T1 run, so the page cannot name the ratio"
                 sed 's/^/        /' "$out/t1.log"; exit 1 ;;
  esac

  progs="$progs\"t1-image\": {\"kind\": \"t1\", \"elf\": \"$(base64 < "$T1_IMAGE" | tr -d '\n')\", \
\"input\": \"$(base64 < "$out/one.blob" | tr -d '\n')\", \"inputName\": \"$T1_MODULE\", \
\"ram\": $t1_ram, \"touches\": $t1_touches, \"budget\": 4294967295, \"source\": \"$T1_SOURCE\"},"
  printf '  %-12s %-5s %s bytes  sha256 %s\n' "t1-image" "t1" \
    "$(wc -c < "$T1_IMAGE" | tr -d ' ')" "$(shasum -a 256 "$T1_IMAGE" | cut -c1-16)"
  printf '  %-12s %-5s demands %s · touches %s · %sx\n' "" "" \
    "$t1_ram" "$t1_touches" "$((t1_ram / t1_touches))"
fi

wasm_b64=$(base64 < "$wasm" | tr -d '\n')
sadhana_wasm_b64=$(base64 < "$sadhana_wasm" | tr -d '\n')

# Substituted line by line rather than with `awk -v`, which cannot carry a multi-line
# value — it reported `newline in string` and produced nothing.
#
# AND NOT WITH `sed` EITHER, WHICH IS WHAT THIS USED TO DO. A `sed` script is an
# ARGUMENT, so the value has to fit the exec argument list, and the T1 image does not:
# measured 2026-10-04, `sed: Argument list too long` on 1.9 MB of base64 — after which
# the loop carried on and wrote a 218 KB page with the placeholder still in it, exiting 0.
# The split is done with parameter expansion and `printf` instead, both shell builtins
# with no argv to overflow, and the three pieces are written in order. The WASM arm does
# the same even though 290 KB still fits today: the module only grows.
#
# The glue is a tracked file inlined rather than imported: the page must open from
# `file://` with no server, and a module import would need one.
# `$1` if given, else a fresh directory that no gate will ever read.
dest=${1:-$(mktemp -d)/sassembly.html}
mkdir -p "$(dirname "$dest")"
# One line, one placeholder, a value of any size: everything before it, the value, then
# everything after it. `${l%%"$k"*}` and `${l#*"$k"}` are expansions and `printf` is a
# builtin, so no part of the value is ever an argument to an exec'd program.
splice() {
  l=$1; k=$2; v=$3
  printf '%s' "${l%%"$k"*}" >> "$dest"
  printf '%s' "$v" >> "$dest"
  printf '%s\n' "${l#*"$k"}" >> "$dest"
}
: > "$dest"
while IFS= read -r line || [ -n "$line" ]; do
  case "$line" in
    *@@GLUE@@*)     sed 's/^export //' web/yantra.mjs >> "$dest" ;;
    # THE ASSEMBLER GLUE, and the arm that did not exist until today. Same `sed`,
    # pasted into the SAME module scope — which is why `web/sadhana.mjs`'s loader is
    # `loadSadhana` and not `load`, and why `tools/check-glue-collision.sh` reads the
    # PAIR rather than each glue against the template. Ordered after `@@GLUE@@` in the
    # template because the page's `const s = ... loadSadhana(...)` follows both.
    *@@SADHANA_GLUE@@*) sed 's/^export //' web/sadhana.mjs >> "$dest" ;;
    # ORDER MATTERS AND IT IS THE LONGER PATTERN FIRST: `@@SADHANA_WASM@@` CONTAINS
    # `@@WASM@@` as a substring, so a `*@@WASM@@*` arm placed above it would match the
    # assembler's line and splice the INTERPRETER's module into it. `case` takes the
    # first matching arm, so this one is written first and that is load-bearing.
    *@@SADHANA_WASM@@*) splice "$line" @@SADHANA_WASM@@ "$sadhana_wasm_b64" ;;
    # The native assembler's stamp. 16 hex or `unknown`, never absent: the template
    # compares it against the module's own and a missing value would read as a match.
    *@@SADHANA_REV@@*) splice "$line" @@SADHANA_REV@@ "$native_rev" ;;
    # The native interpreter's stamp. Same contract: 16 hex or `unknown`, never
    # absent, because the template compares it and an empty value would read as a match.
    *@@YANTRA_REV@@*) splice "$line" @@YANTRA_REV@@ "$native_yantra_rev" ;;
    *@@WASM@@*)     splice "$line" @@WASM@@ "$wasm_b64" ;;
    *@@PROGRAMS@@*) splice "$line" @@PROGRAMS@@ "$progs" ;;
    *)              printf '%s\n' "$line" >> "$dest" ;;
  esac
done < web/sassembly.template.html

# ॥ A PAGE WITH A PLACEHOLDER STILL IN IT IS NOT A PAGE ॥ Measured 2026-10-04: the
# `sed` this replaced died with `Argument list too long` on the T1 image's base64, the
# loop carried on, and this script wrote a 218 KB page carrying the literal string
# `@@PROGRAMS@@` and exited 0. `check-sassembly-page.sh` would then have run a page
# whose script is a syntax error and blamed the template. So the output is read back.
# `[A-Z_]` AND NOT `[A-Z]`, measured on this very change: `@@SADHANA_WASM@@` carries an
# underscore, so the old class matched NEITHER of the two placeholders added today and
# this guard — the one whose whole job is "a page with a placeholder still in it is not a
# page" — would have passed a page carrying both of them unsubstituted.
if grep -aq '@@[A-Z_]*@@' "$dest"; then
  echo "  FAIL  $dest still carries a placeholder — a substitution did not happen:"
  grep -ao '@@[A-Z_]*@@' "$dest" | sort -u | sed 's/^/        /'
  exit 1
fi

printf '  %-12s %s bytes\n' "wasm" "$(wc -c < "$wasm" | tr -d ' ')"
printf '  %-12s %s bytes\n' "sadhana-wasm" "$(wc -c < "$sadhana_wasm" | tr -d ' ')"
printf '  %-12s %s\n' "sadhana rev" "$native_rev ($native_src)"
printf '  %-12s %s\n' "yantra rev" "$native_yantra_rev (crates/yantra/src, native side)"
echo "wrote $dest ($(wc -c < "$dest" | tr -d ' ') bytes) — open it directly, no server needed"
