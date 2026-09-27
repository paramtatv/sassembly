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
# wrote `web/sassembly.html` and gitignored it. The gate then went red: `cargo run -p
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
# `tools/gate.sh` renders it as neither pass nor fail.
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
if ! cargo build -p yantra-wasm --manifest-path crates/yantra-wasm/Cargo.toml \
     --target wasm32-unknown-unknown --release; then
  echo "  FAIL  yantra-wasm does not compile for wasm32-unknown-unknown."
  echo "        The target IS installed — the error above is the tree's, not this"
  echo "        machine's, and exiting 1 rather than 77 is what says so."
  exit 1
fi

wasm=crates/yantra-wasm/target/wasm32-unknown-unknown/release/yantra_wasm.wasm
[ -f "$wasm" ] || { echo "  FAIL  the wasm build reported success and produced no module"; exit 1; }

# The programs the page offers, in two kinds, because the page runs them two ways.
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
# `--स्थान` reads a number. `crates/yantra/tests/host.rs` links it here too.
APP_LOAD=$(awk '$1 == "0x2000_0000" {print $2}' spec/application-load.tsv)

out=$(mktemp -d)
trap 'rm -f "$out"/*.elf "$out"/*.b64 "$out"/programs.js 2>/dev/null || true' EXIT

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
  # shellcheck disable=SC2086
  cargo run -q -p sadhana -- $at "spec/$p.sas" "$out/$p.elf" >/dev/null 2>&1 || {
    echo "  FAIL  sadhana could not assemble spec/$p.sas"; exit 1; }
  b64=$(base64 < "$out/$p.elf" | tr -d '\n')
  progs="$progs\"$p\": {\"kind\": \"$kind\", \"elf\": \"$b64\"},"
  printf '  %-12s %-5s %s bytes  sha256 %s\n' "$p" "$kind" \
    "$(wc -c < "$out/$p.elf" | tr -d ' ')" \
    "$(shasum -a 256 "$out/$p.elf" | cut -c1-16)"
done

wasm_b64=$(base64 < "$wasm" | tr -d '\n')

# Substituted line by line rather than with `awk -v`, which cannot carry a multi-line
# value — it reported `newline in string` and produced nothing. The base64 alphabet is
# `A-Za-z0-9+/=`, so `|` is safe as the sed delimiter and no `&` can appear to be
# misread as "the whole match".
#
# The glue is a tracked file inlined rather than imported: the page must open from
# `file://` with no server, and a module import would need one.
# `$1` if given, else a fresh directory that no gate will ever read.
dest=${1:-$(mktemp -d)/sassembly.html}
mkdir -p "$(dirname "$dest")"
: > "$dest"
while IFS= read -r line || [ -n "$line" ]; do
  case "$line" in
    *@@GLUE@@*)     sed 's/^export //' web/yantra.mjs >> "$dest" ;;
    *@@WASM@@*)     printf '%s\n' "$line" | sed "s|@@WASM@@|$wasm_b64|" >> "$dest" ;;
    *@@PROGRAMS@@*) printf '%s\n' "$line" | sed "s|@@PROGRAMS@@|$progs|" >> "$dest" ;;
    *)              printf '%s\n' "$line" >> "$dest" ;;
  esac
done < web/sassembly.template.html

printf '  %-12s %s bytes\n' "wasm" "$(wc -c < "$wasm" | tr -d ' ')"
echo "wrote $dest ($(wc -c < "$dest" | tr -d ' ') bytes) — open it directly, no server needed"
