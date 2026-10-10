# t1-native-lib.sh — sourced by tools/t1-build.sh and tools/t1-run.sh: the host's native
# target, and translations of RV64 images for it, cached.
#
# THE CACHE (${SASSEMBLY_NATIVE_CACHE:-~/.cache/sassembly-native}), every entry keyed by what
# makes it, so a key never names stale octets and a cache copied between hosts stays valid:
#   stage1/<compiler key>/stage1.elf        Stage 1: made natively from a trusted earlier image (stage1_native),
#                                            or by t1_image over the compiler only with T1_BUILD_COLD=1
#   translator/<compiler key>-<anuvada.t1 sha256>/anu.elf    the .t1 translator, built natively by that Stage 1
#   <image sha256>/<target>-<key of compiler key + both translators>-<ARG0 sha256[:16]>/image.native
#                                            a translation (ARG0 is in its octets)
# <compiler key> = sha256 over the names and octets of crates/sadhana-t1/src/*.t1,
# crates/sadhana/** , Cargo.lock, crates/sanskrit-text/** , the spec/ files that
# tools/compiler-key-spec.txt keys and that list itself — the sources t1_image and Stage 1 come
# from (both are deterministic, so the same key means the same octets on every host). A tree
# without the list keys all of spec/** and no sanskrit-text (the key before 2026-10-10).
# Every file carries a .sha256 beside it, checked on use.
#
# A TRANSLATION is made by the first translator that can run, all three byte-identical
# (tools/native-spike: the .t1 translator equals anuvada.py, octet for octet):
#   1. the native translator (the .t1 translator translated for this host) from the cache;
#   2. the .t1 translator ELF under yantra-run;
#   3. tools/native-spike/anuvada.py (the oracle; needs only python3).
# Seeding a host that cannot build: rsync -a ~/.cache/sassembly-native/ HOST:.cache/sassembly-native/
#
# ROUTES PER HOST, measured 2026-10-09 with the compiler at 3e245f49 (sieve: t1-build's image
# a6b8467a… on every host, equal to the yantra route's; t1-run prints 1270607 as yantra-run):
#   Linux x86_64  x86_64-linux   builds everything: Stage 1 and the translator by t1_image
#                                (cold, end to end: 2,423 s), the native translator by the
#                                .t1 translator under yantra-run; then all native.
#   Linux arm64   aarch64-linux  cache seeded from the x86_64 host (Stage 1, translator ELF); the native
#                                translator made by yantra-run built there from 3e245f49 (the
#                                older ones there predate W-381); build 13 s cold, then native.
#   Apple arm64   aarch64-macos  as Linux arm64 (yantra-run built in a work directory, cache under
#                                that directory by SASSEMBLY_NATIVE_CACHE); build 33 s cold.
#   Intel macOS   x86_64-macos   no cargo, no yantra-run: cache seeded, the native translator
#                                made by anuvada.py; build 8 s cold. t1-run's fallbacks name
#                                their reason and then refuse (no yantra-run to fall back to).
#
# needs: ROOT (this checkout's root), SAS (the compiler checkout), BIN (yantra-run, t1_image)

CACHE=${SASSEMBLY_NATIVE_CACHE:-$HOME/.cache/sassembly-native}
TRANSLATOR=$ROOT/tools/native-spike/t1/anuvada.t1
ORACLE=$ROOT/tools/native-spike/anuvada.py

# switch NAME: 0 when the env var NAME is unset, empty or 0; 1 when it is 1; any other value is
# REFUSED by name (a typo like "yes" or "0 " must never silently change the route)
switch() {
    local v; eval "v=\${$1:-}"
    case $v in ''|0) return 1 ;; 1) return 0 ;; esac
    refuse_switch "$1" "$v"
}
refuse_switch() { echo "t1-native: REFUSED — $1=$2: use 1 (on) or 0/unset (off)" >&2; exit 64; }
# validate_switches: EVERY switch, called once at the top of the MAIN shell of t1-run.sh and t1-build.sh —
# not on the branch that happens to read it, and never inside a $(...) where the exit would be swallowed
validate_switches() {
    local n v
    for n in T1_RUN_REQUIRE_NATIVE T1_BUILD_CACHED_ONLY T1_BUILD_COLD; do
        eval "v=\${$n:-}"
        case $v in ''|0|1) ;; *) refuse_switch "$n" "$v" ;; esac
    done
}
# tmpname FINAL: a temp name unique to this process, beside FINAL (same filesystem: mv is an atomic rename)
tmpname() { echo "$1.tmp.$$.$RANDOM"; }
# publish TMP FINAL: seal and rename TMP into place atomically. Concurrent makers of one entry
# race harmlessly (the content is deterministic): if FINAL appeared meanwhile and is identical,
# accept it; if it DIFFERS, refuse. No locks.
publish() {
    local t=$1 f=$2
    if fresh "$f"; then
        if cmp -s "$t" "$f"; then rm -f "$t"; return 0; fi
        echo "t1-native: $f appeared meanwhile with DIFFERENT content; refusing" >&2; rm -f "$t"; return 1
    fi
    chmod 755 "$t" && sha "$t" > "$f.sha256.$$" && mv -f "$f.sha256.$$" "$f.sha256" && mv -f "$t" "$f"
}
sha() { python3 -c 'import hashlib,sys; print(hashlib.sha256(open(sys.argv[1],"rb").read()).hexdigest())' "$1"; }
strsha() { python3 -c 'import hashlib,sys; print(hashlib.sha256(sys.argv[1].encode()).hexdigest())' "$1"; }
fresh() { [ -s "$1" ] && [ "$(sha "$1")" = "$(cat "$1.sha256" 2>/dev/null)" ]; }

# "<target> <trace code>" of this host, or nothing
host_target() {
    case "$(uname -s) $(uname -m)" in
        "Linux x86_64") echo "x86_64-linux 0" ;;
        "Linux aarch64") echo "aarch64-linux 183" ;;
        "Darwin arm64") echo "aarch64-macos 268" ;;
        "Darwin x86_64") echo "x86_64-macos 263" ;;
    esac
}

compiler_key() {
    python3 - "$SAS" <<'PY'
import hashlib, pathlib, re, sys
root = pathlib.Path(sys.argv[1])
core = sorted(root.glob("crates/sadhana-t1/src/*.t1")) + sorted(p for p in (root / "crates/sadhana").rglob("*") if p.is_file()) \
    + [root / "Cargo.lock"]
spec = sorted(p for p in (root / "spec").rglob("*") if p.is_file())
lst = root / "tools/compiler-key-spec.txt"
h = hashlib.sha256()
if not lst.is_file():
    # a tree from before the list (2026-10-10): the v1 key, octet for octet, so its cache entries stay valid
    for p in core + spec:
        h.update(str(p.relative_to(root)).encode() + b"\0" + p.read_bytes() + b"\0")
    print(h.hexdigest()); sys.exit(0)
# v2: $SAS/tools/compiler-key-spec.txt names the spec files the compiler build reads (tools/check-compiler-key.sh proves it).
# PATH = a file; "dir/" = a subtree; a glob's * and ? never cross "/". key wins; an unclassified file is KEYED. A MALFORMED
# list fails CLOSED: all of spec/ is keyed (with the list and sanskrit-text, as always) and stderr says why.
def matcher(p):
    if p.endswith("/"): return lambda r: r.startswith(p)
    if any(c in p for c in "*?["):
        rx = re.compile("".join("[^/]*" if c == "*" else "[^/]" if c == "?" else re.escape(c) for c in p) + r"\Z")
        return lambda r: rx.match(r) is not None
    return lambda r: r == p
rules, bad = [], []
for n, l in enumerate(lst.read_text(encoding="utf-8").splitlines(), 1):
    w = l.split("#", 1)[0].split()
    if not w: continue
    if len(w) == 2 and w[0] in ("key", "skip") and w[1].startswith("spec/"): rules.append((w[0], matcher(w[1])))
    else: bad.append(n)
if bad:
    print("t1-native: tools/compiler-key-spec.txt line(s) %s malformed: keying ALL of spec/" % bad, file=sys.stderr)
else:
    def hit(rel, kind): return any(k == kind and m(rel) for k, m in rules)
    spec = [p for p in spec if hit(p.relative_to(root).as_posix(), "key") or not hit(p.relative_to(root).as_posix(), "skip")]
files = core + spec + [lst] + sorted(p for p in (root / "crates/sanskrit-text").rglob("*") if p.is_file())
h.update(b"compiler-key v2\0")
for p in files:   # every field LENGTH-PREFIXED: no two (name, octets) sequences can hash the same stream
    n, d = str(p.relative_to(root)).encode(), p.read_bytes()
    h.update(len(n).to_bytes(8, "big") + n + len(d).to_bytes(8, "big") + d)
print(h.hexdigest())
PY
}

# record_image: the image sha this tree's committed tools/compiler-image.sha256 records FOR THIS compiler key, or nothing
record_image() {
    local img k
    read -r img k < "$SAS/tools/compiler-image.sha256" 2>/dev/null || return 1
    [ -n "$img" ] && [ "$k" = "$(compiler_key)" ] && echo "$img"
}
# stage1_ok FILE: a sealed Stage 1 entry that, when this tree's record names an image for this key, IS that image. A sidecar
# only proves the octets were not damaged, not that the entry was made right (review F1: a cache entry poisoned under an unchanged
# key was trusted by its sidecar alone)
stage1_ok() {
    local want
    fresh "$1" || return 1
    want=$(record_image) || return 0
    [ "$(sha "$1")" = "$want" ] && return 0
    echo "t1-native: IGNORING $1: it is $(sha "$1" | cut -c1-12), but tools/compiler-image.sha256 records ${want%"${want#????????????}"} for this key" >&2
    return 1
}
# stage1_have: the Stage 1 to build with, or fail: the canonical entry stage1/<key>/ (seeded by a landing, the nightly or a
# cold t1_image build), else a NATIVELY made one (stage1_native) under stage1/<key>-n<native key>/, each checked by stage1_ok
stage1_have() {
    local k f
    k=$(compiler_key)
    f=$CACHE/stage1/$k/stage1.elf; stage1_ok "$f" 2>/dev/null && { echo "$f"; return 0; }
    f=$(stage1_native_path) && stage1_ok "$f" 2>/dev/null && { echo "$f"; return 0; }
    return 1
}
# stage1_native_path: where a NATIVELY made Stage 1 of this compiler lives. Its key adds everything that route reads besides the
# corpus (review F1): this lib, pack-corpus.py, the oracle anuvada.py, with-timeout.pl and the SEED image's octets
stage1_native_path() {
    local p
    p=$(trusted_seed) || return 1
    echo "$CACHE/stage1/$(compiler_key)-n$(strsha "$(sha "$ROOT/tools/t1-native-lib.sh")$(sha "$ROOT/tools/pack-corpus.py")$(sha "$ORACLE")$(sha "$ROOT/tools/with-timeout.pl")$(sha "$p")" | cut -c1-32)/stage1.elf"
}

# cut_elf SINK OUT: the ELF a Stage 1 run wrote to its sink (from the first \x7fELF, minus the trailing octet), as tools/fixpoint.sh
cut_elf() {
    python3 - "$1" "$2" <<'PY'
import sys
s = open(sys.argv[1], "rb").read()
o = s.find(b"\x7fELF")
if o < 0: sys.exit(1)
open(sys.argv[2], "wb").write(s[o:len(s) - 1])
PY
}

# trusted_seed: a TRUSTED earlier compiler image to make this compiler's Stage 1 from, natively (never t1_image): echoes its path
#   1. T1_STAGE1_SEED, an image named by hand;
#   2. the image $SAS/tools/compiler-image.sha256 records (committed), wherever this host holds it: a pinned
#      ${FIXPOINT_CACHE:-~/.sansos-fixpoint-cache}/*.elf or a cached Stage 1;
#   3. the pinned image of the nearest ancestor of $SAS's HEAD (the project's landing and nightly jobs pin main's images there).
# Every candidate is checked against its .sha256 sidecar. A cache seeded from another host works the same (stage1/*).
trusted_seed() {
    local fpc=${FIXPOINT_CACHE:-$HOME/.sansos-fixpoint-cache} want f c
    if [ -n "${T1_STAGE1_SEED:-}" ]; then [ -s "$T1_STAGE1_SEED" ] && echo "$T1_STAGE1_SEED"; return; fi
    want=$(cut -d' ' -f1 "$SAS/tools/compiler-image.sha256" 2>/dev/null)
    if [ -n "$want" ]; then
        for f in "$fpc"/*.elf "$CACHE"/stage1/*/stage1.elf; do
            [ -s "$f" ] && [ "$(cut -d' ' -f1 "$f.sha256" 2>/dev/null)" = "$want" ] && [ "$(sha "$f")" = "$want" ] && { echo "$f"; return 0; }
        done
    fi
    for c in $(git -C "$SAS" rev-list -n 2000 HEAD 2>/dev/null); do
        f=$fpc/$c.elf
        [ -s "$f" ] && [ "$(cut -d' ' -f1 "$f.sha256" 2>/dev/null)" = "$(sha "$f")" ] && { echo "$f"; return 0; }
    done
    return 1
}

# stage1_native S1: make Stage 1 for this compiler at S1 (stage1_native_path) FROM trusted_seed's image P, natively, in seconds:
#   A = P(corpus), B = A(corpus), C = B(corpus), each translated for this host (by the oracle: no translator ELF of this key
#   exists yet); B == C byte-identical or REFUSE; B is published. The landing's chain (tools/land-remote.sh) is the same three
#   stages. Fails (1) when there is no trusted image or the chain does not hold; the caller decides whether a cold build is allowed.
stage1_native() {
    local s1=$1 p w cur st nat t rc T1N_ORACLE_ONLY=1
    p=$(trusted_seed) || return 1
    w=$(mktemp -d "${TMPDIR:-/tmp}/t1-stage1.XXXXXX") || return 1
    echo "t1-native: making Stage 1 natively from the trusted image $p (A, B, C; B == C) -> $s1" >&2
    python3 "$ROOT/tools/pack-corpus.py" "$w/corpus.blob" "$SAS"/crates/sadhana-t1/src/*.t1 > "$w/pack.log" || { echo "t1-native: packing failed ($w)" >&2; return 1; }
    cur=$p
    for st in A B C; do
        nat=$(translate "$cur" stage1.elf) || break
        env -u YANTRA_INPUT_ENTRY YANTRA_INPUT="$w/corpus.blob" YANTRA_INPUT_NAME=शृङ्खला YANTRA_RAM=2684354560 \
            perl "$ROOT/tools/with-timeout.pl" "${T1_STAGE_TIMEOUT:-1800}" "$nat" > "$w/$st.sink" 2> "$w/$st.log"
        [ "$(grep -c '^halt: ' "$w/$st.log")" = 1 ] && grep -qx 'halt: Finisher { value: [0-9]*, status: Some(1200) }' "$w/$st.log" || break
        cut_elf "$w/$st.sink" "$w/$st.elf" && [ -s "$w/$st.elf" ] || break
        cur=$w/$st.elf
    done
    if [ "$cur" = "$w/C.elf" ] && cmp -s "$w/B.elf" "$w/C.elf"; then
        mkdir -p "${s1%/*}" && t=$(tmpname "$s1") && cp "$w/B.elf" "$t" && publish "$t" "$s1"; rc=$?
        [ "$rc" = 0 ] && echo "t1-native: Stage 1 $(sha "$s1" | cut -c1-12) (B == C$(cmp -s "$w/A.elf" "$w/B.elf" && echo ', A == B too'))" >&2
        rm -rf "$w"; return $rc
    fi
    echo "t1-native: the native Stage 1 from $p did NOT hold (stage $st refused, or B != C); logs in $w" >&2
    return 1
}

# the .t1 translator ELF, built NATIVELY by this compiler's cached Stage 1 when absent (never t1_image): echoes its path, or
# fails, and then every translation falls back to anuvada.py (the oracle: byte-identical, seconds)
translator_elf() {
    local d=$CACHE/translator/$(compiler_key)-$(sha "$TRANSLATOR") s1 nat tmp T1N_ORACLE_ONLY=1
    if fresh "$d/anu.elf"; then echo "$d/anu.elf"; return 0; fi
    switch T1_BUILD_CACHED_ONLY && return 1
    # only from a Stage 1 this cache key can vouch for: the canonical entry, or a native one the RECORD names. Otherwise no
    # translator ELF is published under this key (review F1), and translations fall back to the oracle (byte-identical)
    s1=$CACHE/stage1/$(compiler_key)/stage1.elf
    stage1_ok "$s1" 2>/dev/null || { s1=$(stage1_native_path) && record_image > /dev/null && stage1_ok "$s1" 2>/dev/null; } || return 1
    nat=$(translate "$s1" stage1.elf) || return 1
    echo "t1-native: building the .t1 translator natively by Stage 1 $(sha "$s1" | cut -c1-12) -> $d/anu.elf" >&2
    mkdir -p "$d"; tmp=$(tmpname "$d/anu.elf")
    python3 "$ROOT/tools/pack-corpus.py" "$tmp.blob" "$TRANSLATOR" > /dev/null &&
        env -u YANTRA_INPUT_NAME YANTRA_INPUT="$tmp.blob" YANTRA_INPUT_ENTRY="अनुवाद मुख्यम्" YANTRA_RAM=2684354560 \
            perl "$ROOT/tools/with-timeout.pl" "${T1_STAGE_TIMEOUT:-1800}" "$nat" > "$tmp.sink" 2> "$tmp.log"
    if grep -q 'status: Some(1200)' "$tmp.log" 2>/dev/null && cut_elf "$tmp.sink" "$tmp" && [ -s "$tmp" ]; then
        rm -f "$tmp.blob" "$tmp.sink" "$tmp.log"
    else mv -f "$tmp.log" "$d/build.log" 2>/dev/null; rm -f "$tmp" "$tmp.blob" "$tmp.sink"; echo "t1-native: translator build failed; see $d/build.log" >&2; return 1; fi
    publish "$tmp" "$d/anu.elf" && echo "$d/anu.elf"
}

# translate IMAGE ARG0 [NOSELF]: echoes the native file's path, or fails; with NOSELF the
# native translator is not used (it is what is being made)
translate() {
    local img=$1 arg0=$2 noself=${3:-} tgt code tsha dir out anu nat tmp log
    read -r tgt code <<< "$(host_target)"
    [ -n "$tgt" ] || return 1
    # the key covers BOTH translators AND the compiler key (the translator ELF is made by the compiler):
    # a host may translate with the oracle (anuvada.py), and an edit to any of them must never
    # reuse a cached translation
    tsha=$(strsha "$(compiler_key)$(sha "$TRANSLATOR")$(sha "$ORACLE")")
    dir=$CACHE/$(sha "$img")/$tgt-$tsha-$(strsha "$arg0" | cut -c1-16)
    out=$dir/image.native
    if fresh "$out"; then echo "$out"; return 0; fi
    switch T1_BUILD_CACHED_ONLY && return 1
    mkdir -p "$dir"
    tmp=$(tmpname "$out"); log=$tmp.log   # per-process: concurrent cold translations never share a file
    if [ -z "$noself" ] && [ -z "${T1N_ORACLE_ONLY:-}" ] && nat=$(native_translator) ; then
        echo "t1-native: translating $(basename "$img") for $tgt by the native translator" >&2
        env -u YANTRA_INPUT_ENTRY YANTRA_INPUT="$img" YANTRA_INPUT_NAME="$arg0" YANTRA_INPUT_TRACE="$code" \
            YANTRA_RAM=1073741824 "$nat" > "$tmp" 2> "$log"
    elif [ -z "${T1N_ORACLE_ONLY:-}" ] && [ -x "$BIN/yantra-run" ] && anu=$(translator_elf); then
        echo "t1-native: translating $(basename "$img") for $tgt by the .t1 translator under yantra-run" >&2
        env -u YANTRA_INPUT_ENTRY YANTRA_INPUT="$img" YANTRA_INPUT_NAME="$arg0" YANTRA_INPUT_TRACE="$code" \
            YANTRA_RAM=1073741824 YANTRA_STEPS=10000000000000 "$BIN/yantra-run" "$anu" > "$tmp" 2> "$log"
    else
        echo "t1-native: translating $(basename "$img") for $tgt by anuvada.py (the oracle)" >&2
        python3 "$ORACLE" --target "$tgt" "$img" "$tmp" "$arg0" 2> "$log" &&
            echo "halt: Finisher { value: 21845, status: Some(0) }" >> "$log"
    fi
    grep -q "status: Some(0)" "$log" && [ -s "$tmp" ] ||
        { mv -f "$log" "$dir/translate.log"; echo "t1-native: the translator refused $(basename "$img"); see $dir/translate.log" >&2; rm -f "$tmp"; return 1; }
    rm -f "$log"
    publish "$tmp" "$out" && echo "$out"
}

# the native translator for this host: the translation of the .t1 translator ELF
native_translator() {
    local anu
    anu=$(translator_elf) || return 1
    translate "$anu" anu.elf noself
}
