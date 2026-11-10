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
# crates/sadhana/** , Cargo.lock, the spec/ files that
# tools/compiler-key-spec.txt keys and that list itself — the sources t1_image and Stage 1 come
# from (both are deterministic, so the same key means the same octets on every host). A tree
# without the list keys all of spec/** (the key before 2026-10-10).
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
#   ubuntu-local  x86_64-linux   builds everything: Stage 1 and the translator by t1_image
#                                (cold, end to end: 2,423 s), the native translator by the
#                                .t1 translator under yantra-run; then all native.
#   Jetson        aarch64-linux  cache seeded from ubuntu (Stage 1, translator ELF); the native
#                                translator made by yantra-run built there from 3e245f49 (the
#                                older ones there predate W-381); build 13 s cold, then native.
#   Mac mini      aarch64-macos  as the Jetson (yantra-run built in ~/sansos-work, cache under
#                                ~/sansos-work by SASSEMBLY_NATIVE_CACHE); build 33 s cold.
#   Intel Mac     x86_64-macos   no cargo, no yantra-run: cache seeded, the native translator
#                                made by anuvada.py; build 8 s cold. t1-run's fallbacks name
#                                their reason and then refuse (no yantra-run to fall back to).
#
# needs: ROOT (this checkout's root), SAS (the compiler checkout), BIN (yantra-run, t1_image)
#        T1_COMPILER_SRC / T1_SPEC_ROOT: the compiler's .t1 sources and the spec root its build reads. SHELL variables
#        set HERE to $SAS's own (never read from the environment: an inherited value must not silently change the
#        compiler); tools/t1-build.sh's --compiler / --spec-root override them after this file is sourced. The key names
#        their files as crates/sadhana-t1/src/<name> and spec/<rel> wherever they live, so the default key is unchanged.

CACHE=${SASSEMBLY_NATIVE_CACHE:-$HOME/.cache/sassembly-native}
T1_COMPILER_SRC=$SAS/crates/sadhana-t1/src
T1_SPEC_ROOT=$SAS/spec
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
_sha256() { if command -v sha256sum >/dev/null 2>&1; then sha256sum; else shasum -a 256; fi; }
sha() { _sha256 < "$1" | { read -r h _; printf '%s\n' "$h"; }; }
strsha() { printf %s "$1" | _sha256 | { read -r h _; printf '%s\n' "$h"; }; }
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

# compiler_key: sha256 over "compiler-key v2\0" then, per file, 8-byte BE len(name), name, 8-byte BE len(data), data. Files = core
# (compiler *.t1, crates/sadhana/**, Cargo.lock) + spec/** (the keyed ones) + tools/compiler-key-spec.txt, each group sorted by path
# COMPONENTS (byte order, "/" sorting below every other octet). POSIX sh + awk + od-free printf, no python (N48); byte-identical to
# the python it replaced (tools/test-compiler-key.sh K8 pins a golden key). No compiler here ($SAS holds no *.t1): no key, exit 1.
ck_be8() {   # 8-byte big-endian N as raw octets, in shell arithmetic only (no fork)
    ck_n=$1; ck_o=; ck_i=0
    while [ "$ck_i" -lt 8 ]; do
        ck_b=$((ck_n % 256)); ck_n=$((ck_n / 256)); ck_o="\\$((ck_b / 64))$((ck_b / 8 % 8))$((ck_b % 8))$ck_o"; ck_i=$((ck_i + 1))
    done
    printf "$ck_o"
}
ck_ls() {   # DIR : its files, relative, sorted by path components, OS junk (.DS_Store, Thumbs.db, ._*) dropped: it differs per host (a6 2026-10-17)
    ( cd "$1" 2>/dev/null && find . -type f -print ) | sed 's|^\./||' \
        | awk -F/ '{ b = $NF; if (b == ".DS_Store" || b == "Thumbs.db" || substr(b, 1, 2) == "._") next; print }' \
        | tr '/' '\001' | sort | tr '\001' '/'
}
compiler_key_compute() (
    LC_ALL=C; export LC_ALL
    ck_root=$SAS ck_comp=$T1_COMPILER_SRC ck_spec=$T1_SPEC_ROOT ck_tab=$(printf '\t')
    # NO COMPILER HERE ($SAS is not a Sassembly checkout, e.g. a tools-only extraction): no key at all, never a key of nothing
    # (2026-10-12: an empty-key cache made every sandboxed selfcheck translate cold and time out).
    [ -n "$(find "$ck_comp" -maxdepth 1 -name '*.t1' 2>/dev/null | head -n 1)" ] || exit 1
    ck_t=$(mktemp -d "${TMPDIR:-/tmp}/ckey.XXXXXX") || exit 1
    trap 'rm -rf "$ck_t"' EXIT
    # the NAME a file is keyed under: a compiler source and a spec file under their canonical place in a checkout, wherever
    # --compiler / --spec-root put them (so the default key is the key it always was, octet for octet)
    ck_lst=$ck_root/tools/compiler-key-spec.txt
    {
        ck_ls "$ck_comp" | awk -v d="$ck_comp" '/\.t1$/ && !/\// { print "crates/sadhana-t1/src/" $0 "\t" d "/" $0 }'
        ck_ls "$ck_root/crates/sadhana" | awk -v d="$ck_root/crates/sadhana" '{ print "crates/sadhana/" $0 "\t" d "/" $0 }'
        [ ! -f "$ck_root/Cargo.lock" ] || printf 'Cargo.lock\t%s/Cargo.lock\n' "$ck_root"
    } > "$ck_t/core"
    ck_ls "$ck_spec" | awk '{ print "spec/" $0 }' > "$ck_t/spec.names"
    if [ -f "$ck_lst" ]; then
        # v2: $SAS/tools/compiler-key-spec.txt names the spec files the compiler build reads (tools/check-compiler-key.sh proves it).
        # PATH = a file; "dir/" = a subtree; a glob's * and ? never cross "/". key wins; an unclassified file is KEYED. A MALFORMED
        # list fails CLOSED: all of spec/ is keyed (with the list, as always) and stderr says why. (Same rules as tools/compiler-key-classify.sh.)
        awk -v lst="$ck_lst" '
        function rx(p,   i, c, r) {          # glob -> anchored regex; * and ? never cross "/"
            r = "^"
            for (i = 1; i <= length(p); i++) {
                c = substr(p, i, 1)
                if (c == "*") r = r "[^/]*"
                else if (c == "?") r = r "[^/]"
                else if (c == "\\") r = r "\\\\"
                else if (c == "^") r = r "\\^"
                else if (c !~ "[A-Za-z0-9_]") r = r "[" c "]"
                else r = r c
            }
            return r "$"
        }
        function hit(path, kind,   i) {
            for (i = 1; i <= n; i++) if (k[i] == kind) {
                if (t[i] == "D") { if (index(path, p[i]) == 1) return 1 }
                else if (t[i] == "G") { if (path ~ g[i]) return 1 }
                else if (path == p[i]) return 1
            }
            return 0
        }
        FILENAME == lst {
            sub(/#.*/, ""); gsub(/\r/, " ")
            if (NF == 0) next
            if (NF == 2 && ($1 == "key" || $1 == "skip") && index($2, "spec/") == 1) {
                n++; k[n] = $1; p[n] = $2
                if (substr($2, length($2)) == "/") t[n] = "D"
                else if ($2 ~ /[*?\[]/) { t[n] = "G"; g[n] = rx($2) }
                else t[n] = "F"
            } else bad = bad (bad == "" ? "" : ", ") FNR
            next
        }
        { if (!said && bad != "") { print "t1-native: tools/compiler-key-spec.txt line(s) [" bad "] malformed: keying ALL of spec/" > "/dev/stderr"; said = 1 }
          if (bad != "" || hit($0, "key") || !hit($0, "skip")) print }
        ' "$ck_lst" "$ck_t/spec.names" > "$ck_t/spec.kept"
        awk '{ print $0 "\t" sp "/" substr($0, 6) }' sp="$ck_spec" "$ck_t/spec.kept" >> "$ck_t/core"
        printf 'tools/compiler-key-spec.txt\t%s\n' "$ck_lst" >> "$ck_t/core"
        ck_v=2
    else
        # a tree from before the list (2026-10-10): the v1 key, octet for octet, so its cache entries stay valid
        awk '{ print $0 "\t" sp "/" substr($0, 6) }' sp="$ck_spec" "$ck_t/spec.names" >> "$ck_t/core"
        ck_v=1
    fi
    : > "$ck_t/files"
    while IFS=$ck_tab read -r ck_name ck_path; do
        [ -f "$ck_path" ] && [ -r "$ck_path" ] || exit 1
        ck_size=$(wc -c < "$ck_path") || exit 1
        printf '%s\t%s\t%s\n' "$ck_name" "$ck_path" $((ck_size)) >> "$ck_t/files"
    done < "$ck_t/core"
    {
        if [ "$ck_v" = 2 ]; then
            printf 'compiler-key v2\0'
            while IFS=$ck_tab read -r ck_name ck_path ck_size; do   # every field LENGTH-PREFIXED: no two (name, octets) sequences can hash the same stream
                ck_be8 ${#ck_name}; printf '%s' "$ck_name"; ck_be8 "$ck_size"; cat "$ck_path"
            done < "$ck_t/files"
        else
            while IFS=$ck_tab read -r ck_name ck_path ck_size; do
                printf '%s\0' "$ck_name"; cat "$ck_path"; printf '\0'
            done < "$ck_t/files"
        fi
    } | if command -v sha256sum > /dev/null 2>&1; then sha256sum; else shasum -a 256; fi | cut -d' ' -f1
)
# compiler_key: the key, computed once per run. T1_CKEY_PIN="<key>|<compiler dir>|<spec root>" (set by t1-build.sh for its own
# process, and by psx-rerun.sh for a whole run) answers for exactly that compiler dir and spec root; any other pair is computed.
# WHY: the compute is ~2 s of sh+awk and one t1-build made ~6 calls of it (record_image, stage1_ok x3, stage1, stage1_have): 12 s per build.
compiler_key() {
    _cp=${T1_CKEY_PIN:-}
    case $_cp in "${_cp%%|*}|${T1_COMPILER_SRC:-}|${T1_SPEC_ROOT:-}") printf "%s\n" "${_cp%%|*}"; return 0 ;; esac
    compiler_key_compute
}

# record_image: the image sha this tree's committed tools/compiler-image.sha256 records FOR THIS compiler key, or nothing
record_image() {
    local img k
    read -r img k < "$SAS/tools/compiler-image.sha256" 2>/dev/null || return 1
    [ -n "$img" ] && [ "$k" = "$(compiler_key)" ] && echo "$img"
}
# stage1_ok FILE: a sealed Stage 1 entry that, when this tree's record names an image for this key, IS that image. A sidecar
# only proves the octets were not damaged, not that the entry was made right (8f F1: a cache entry poisoned under an unchanged
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
# corpus (8f F1): this lib, pack-corpus.sh, the oracle anuvada.py, with-timeout.pl and the SEED image's octets
stage1_native_path() {
    local p
    p=$(trusted_seed) || return 1
    echo "$CACHE/stage1/$(compiler_key)-n$(strsha "$(sha "$ROOT/tools/t1-native-lib.sh")$(sha "$ROOT/tools/pack-corpus.sh")$(sha "$ORACLE")$(sha "$ROOT/tools/with-timeout.pl")$(sha "$p")" | cut -c1-32)/stage1.elf"
}

# cut_elf SINK OUT: the ELF a Stage 1 run wrote to its sink (from the first \x7fELF, minus the trailing octet), as tools/fixpoint.sh
cut_elf() {
    local off len
    off=$(LC_ALL=C grep -abo "$(printf '\177ELF')" "$1" | head -n 1 | cut -d: -f1)
    [ -n "$off" ] || return 1
    len=$(wc -c < "$1" | tr -d ' ')
    head -c $((len - 1)) "$1" | tail -c +$((off + 1)) > "$2"   # RXC-SINK: reader last (tail|head SIGPIPEs under pipefail)
}

# trusted_seed: a TRUSTED earlier compiler image to make this compiler's Stage 1 from, natively (never t1_image): echoes its path
#   1. T1_STAGE1_SEED, an image named by hand;
#   2. the image $SAS/tools/compiler-image.sha256 records (committed), wherever this host holds it: a pinned
#      ${FIXPOINT_CACHE:-~/.sansos-fixpoint-cache}/*.elf or a cached Stage 1;
#   3. the pinned image of the nearest ancestor of $SAS's HEAD (land.sh and the nightly pin main's images there), NEVER one older
#      than the last commit that changed compiler-image.sha256 (the re-key): that is refused, with "seed too old".
# Every candidate is checked against its .sha256 sidecar. A cache seeded from another host works the same (stage1/*).
trusted_seed() {
    local fpc=${FIXPOINT_CACHE:-$HOME/.sansos-fixpoint-cache} want f c rk past stale d dirs hosts
    # cache dirs: $fpc, then each host work-disk dir tools/seed-hosts.txt names (land.sh ships there; never the root disk)
    dirs=$fpc
    # POSIX only (no process substitution, no ${//}): sourced by scripts that run under sh (dash; bash --posix on macOS)
    hosts=$(sed 's/#.*//' "$SAS/tools/seed-hosts.txt" 2>/dev/null)
    while read -r _ d _; do
        [ -n "$d" ] || continue; case $d in "~"*) d=$HOME${d#\~};; esac; [ -d "$d" ] && dirs="$dirs
$d"
    done <<EOF
$hosts
EOF
    want=$(cut -d' ' -f1 "$SAS/tools/compiler-image.sha256" 2>/dev/null)
    rk=$(git -C "$SAS" log -n 1 --format=%H HEAD -- tools/compiler-image.sha256 2>/dev/null)
    if [ -n "${T1_STAGE1_SEED:-}" ]; then
        # a hand/recorded seed is honoured unless it IS a pinned ancestor image (named <commit>.elf) older than the re-key and
        # not the recorded image: that is the same stale seed rule 3 refuses (2026-10-13: a seed recorded at 7e70aa68 reached
        # stage B halt 861 while 3652f541's stage1.elf, the recorded image, works). It is then replaced by rules 2 and 3.
        c=${T1_STAGE1_SEED##*/}; c=${c%.elf}
        if [ -s "$T1_STAGE1_SEED" ]; then
            if [ -n "$rk" ] && [ "$(sha "$T1_STAGE1_SEED")" != "$want" ] && case $c in ????????????????????????????????????????) true;; *) false;; esac \
               && git -C "$SAS" rev-list HEAD 2>/dev/null | awk -v c="$c" -v rk="$rk" '$0==rk{past=1;next} past&&$0==c{f=1} END{exit f?0:1}'; then
                echo "t1-native: T1_STAGE1_SEED ${T1_STAGE1_SEED##*/} is a pin older than the last re-key ${rk%"${rk#????????????}"}: refused, replaced by the recorded/nearest valid image" >&2
            else echo "$T1_STAGE1_SEED"; return 0; fi
        else return 1; fi
    fi
    if [ -n "$want" ]; then
        while read -r d; do for f in "$d"/*.elf; do
            [ -s "$f" ] && [ "$(cut -d' ' -f1 "$f.sha256" 2>/dev/null)" = "$want" ] && [ "$(sha "$f")" = "$want" ] && { echo "$f"; return 0; }
        done; done <<EOF_DIRS
$dirs
EOF_DIRS
        for f in "$CACHE"/stage1/*/stage1.elf; do
            [ -s "$f" ] && [ "$(cut -d' ' -f1 "$f.sha256" 2>/dev/null)" = "$want" ] && [ "$(sha "$f")" = "$want" ] && { echo "$f"; return 0; }
        done
    fi
    # rule 3 REFUSES a pin older than the last re-key: the commit that last changed tools/compiler-image.sha256 (RK). An image
    # pinned before RK is a compiler from before the recorded one; building today's corpus with it can be miscompiled (2026-10-13:
    # a pin 248 commits back halted the chain, 861). Walking the history newest first, the pin at RK itself is the oldest allowed.
    past=0 stale=
    for c in $(git -C "$SAS" rev-list -n 2000 HEAD 2>/dev/null); do
        f=
        while read -r d; do [ -s "$d/$c.elf" ] && { f=$d/$c.elf; break; }; done <<EOF_DIRS
$dirs
EOF_DIRS
        if [ -n "$f" ] && [ "$(cut -d' ' -f1 "$f.sha256" 2>/dev/null)" = "$(sha "$f")" ]; then
            [ "$past" = 0 ] && { echo "$f"; return 0; }
            stale=$f; break
        fi
        [ "$c" = "$rk" ] && past=1
    done
    [ -n "$stale" ] && echo "t1-native: seed too old: the only pinned ancestor image is ${stale##*/}, older than the last re-key ${rk%"${rk#????????????}"} of tools/compiler-image.sha256; ship the recorded image ${want%"${want#????????????}"} to this host's fixpoint cache ($fpc) or set T1_STAGE1_SEED" >&2
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
    sh "$ROOT/tools/pack-corpus.sh" "$w/corpus.blob" "$T1_COMPILER_SRC"/*.t1 > "$w/pack.log" || { echo "t1-native: packing failed" >&2; tail -n 40 "$w/pack.log" >&2; rm -rf "$w"; return 1; }
    cur=$p
    for st in A B C; do
        nat=$(translate "$cur" stage1.elf) || break
        env -u YANTRA_INPUT_ENTRY YANTRA_INPUT="$w/corpus.blob" YANTRA_INPUT_NAME=शृङ्खला YANTRA_RAM=2684354560 YANTRA_STEPS=10000000000000 \
            perl "$ROOT/tools/with-timeout.pl" "${T1_STAGE_TIMEOUT:-1800}" "$nat" > "$w/$st.sink" 2> "$w/$st.log"
        [ "$(grep -ac '^halt: ' "$w/$st.log")" = 1 ] && grep -aqx 'halt: Finisher { value: [0-9]*, status: Some(1200) }' "$w/$st.log" || break
        cut_elf "$w/$st.sink" "$w/$st.elf" && [ -s "$w/$st.elf" ] || break
        cur=$w/$st.elf
    done
    if [ "$cur" = "$w/C.elf" ] && cmp -s "$w/B.elf" "$w/C.elf"; then
        mkdir -p "${s1%/*}" && t=$(tmpname "$s1") && cp "$w/B.elf" "$t" && publish "$t" "$s1"; rc=$?
        [ "$rc" = 0 ] && echo "t1-native: Stage 1 $(sha "$s1" | cut -c1-12) (B == C$(cmp -s "$w/A.elf" "$w/B.elf" && echo ', A == B too'))" >&2
        rm -rf "$w"; return $rc
    fi
    echo "t1-native: the native Stage 1 from $p did NOT hold (stage $st refused, or B != C)" >&2
    tail -n 40 "$w/$st.log" >&2 2>/dev/null
    rm -rf "$w"
    return 1
}

# the .t1 translator ELF, built NATIVELY by this compiler's cached Stage 1 when absent (never t1_image): echoes its path, or
# fails, and then every translation falls back to anuvada.py (the oracle: byte-identical, seconds)
translator_elf() {
    local d=$CACHE/translator/$(compiler_key)-$(sha "$TRANSLATOR") s1 nat tmp T1N_ORACLE_ONLY=1
    if fresh "$d/anu.elf"; then echo "$d/anu.elf"; return 0; fi
    switch T1_BUILD_CACHED_ONLY && return 1
    # only from a Stage 1 this cache key can vouch for: the canonical entry, or a native one the RECORD names. Otherwise no
    # translator ELF is published under this key (8f F1), and translations fall back to the oracle (byte-identical)
    s1=$CACHE/stage1/$(compiler_key)/stage1.elf
    stage1_ok "$s1" 2>/dev/null || { s1=$(stage1_native_path) && record_image > /dev/null && stage1_ok "$s1" 2>/dev/null; } || return 1
    nat=$(translate "$s1" stage1.elf) || return 1
    echo "t1-native: building the .t1 translator natively by Stage 1 $(sha "$s1" | cut -c1-12) -> $d/anu.elf" >&2
    mkdir -p "$d"; tmp=$(tmpname "$d/anu.elf")
    sh "$ROOT/tools/pack-corpus.sh" "$tmp.blob" "$TRANSLATOR" > /dev/null &&
        env -u YANTRA_INPUT_NAME YANTRA_INPUT="$tmp.blob" YANTRA_INPUT_ENTRY="अनुवाद मुख्यम्" YANTRA_RAM=2684354560 YANTRA_STEPS=10000000000000 \
            perl "$ROOT/tools/with-timeout.pl" "${T1_STAGE_TIMEOUT:-1800}" "$nat" > "$tmp.sink" 2> "$tmp.log"
    if grep -aq 'status: Some(1200)' "$tmp.log" 2>/dev/null && cut_elf "$tmp.sink" "$tmp" && [ -s "$tmp" ]; then
        rm -f "$tmp.blob" "$tmp.sink" "$tmp.log"
    else mv -f "$tmp.log" "$d/build.log" 2>/dev/null; rm -f "$tmp" "$tmp.blob" "$tmp.sink"; echo "t1-native: translator build failed; see $d/build.log" >&2; return 1; fi
    publish "$tmp" "$d/anu.elf" && echo "$d/anu.elf"
}

# image_base IMAGE: the lowest PT_LOAD p_vaddr of an RV64 ELF, in hex (the base cell the translation is made for)
image_base() {
    local f=$1 po pn pe i t v best=
    po=$(od -An -t u8 -j 32 -N 8 "$f" | tr -d ' ') || return 1
    pe=$(od -An -t u2 -j 54 -N 2 "$f" | tr -d ' '); pn=$(od -An -t u2 -j 56 -N 2 "$f" | tr -d ' ')
    i=0
    while [ "$i" -lt "$pn" ]; do
        t=$(od -An -t u4 -j $((po + i * pe)) -N 4 "$f" | tr -d ' ')
        if [ "$t" = 1 ]; then
            v=$(od -An -t u8 -j $((po + i * pe + 16)) -N 8 "$f" | tr -d ' ')
            if [ -z "$best" ] || [ "$v" -lt "$best" ]; then best=$v; fi
        fi
        i=$((i + 1))
    done
    printf '%x' "$best"
}

# translate IMAGE ARG0 [NOSELF] [FJ] [NET] [WM] [VD] [SC] [VDF]: echoes the native file's path, or fails; with NOSELF the
# native translator is not used (it is what is being made); with NET the frames device is compiled in
# (tools/native-spike/NET.md: anuvada.py only, x86_64-linux only); WM non-empty: the translation that keeps the
# store high-water mark and says it at exit when YANTRA_WATERMARK is set (Linux x86-64 / aarch64 only; the
# .t1 translator's trace + 131072, the oracle's --watermark), its own cache entry; no fork-join with it;
# SC non-empty: slot 0's GPU is compiled in (YANTRA_SCANOUT; scanout_dev.py, L33), anuvada.py only (the
# oracle, ANUVADA_SC=1), its own cache entry, no fork-join, never with NET or WM
# VDF non-empty (with NET): the frames device that defers its completions (YANTRA_VIRTIO_DEFER; the oracle's
# ANUVADA_VD=1: SAFE blocks that tick the deferral's clock before every word), its own cache entry
translate() {
    local img=$1 arg0=$2 noself=${3:-} fj=${4:-} net=${5:-} wm=${6:-} vd=${7:-} sc=${8:-} vdf=${9:-} tgt code tsha dir out anu nat tmp log fjk= fjenv= netenv= scenv= wmk= wmflag= vdk=
    read -r tgt code <<EOF
$(host_target)
EOF
    [ -n "$tgt" ] || return 1
    if [ -n "$wm" ]; then
        fj=
        case $tgt in
            x86_64-linux) code=131134 ;;
            aarch64-linux) code=131255 ;;
            *) return 1 ;;
        esac
        wmk=-wm; wmflag=--watermark
    fi
    # YANTRA_VERDICT (L33): the translation that counts yantra's steps and writes the verdict file (trace 262144 + the
    # target, the oracle's --verdict); it keeps the store high-water mark too, so it replaces the watermark one
    if [ -n "$vd" ]; then
        fj=
        case $tgt in
            x86_64-linux) code=262206 ;;
            aarch64-linux) code=262327 ;;
            *) return 1 ;;
        esac
        wmk=; wmflag=--verdict; vdk=-vd
    fi
    # FORK-JOIN (tools/native-spike/FORKJOIN.md): a translation whose regions may run on threads is the
    # .t1 translator's trace + 65536 (the oracle's ANUVADA_FORKJOIN=1), Linux targets only; its own cache entry
    if [ -n "$fj" ]; then
        case $tgt in
            x86_64-linux|aarch64-linux) code=$((${code:-0} + 65536)); [ "$tgt" = x86_64-linux ] && code=65598
                fjk=-fj; fjenv=ANUVADA_FORKJOIN=1 ;;
        esac
    fi
    # the key covers BOTH translators AND the compiler key (the translator ELF is made by the compiler):
    # a host may translate with the oracle (anuvada.py), and an edit to any of them must never
    # reuse a cached translation
    if [ -n "$net" ]; then
        [ "$tgt" = x86_64-linux ] || return 1
        fjk=-net; fjenv=; netenv=ANUVADA_NET=1
        [ -z "$vdf" ] || { fjk=-netvd; netenv="ANUVADA_NET=1 ANUVADA_VD=1"; }
    fi
    if [ -n "$sc" ]; then
        [ -z "$wm" ] && [ -z "$vd" ] || return 1
        case $tgt in x86_64-linux|aarch64-linux) ;; *) return 1 ;; esac
        if [ -n "$net" ]; then fjk=$fjk-sc; scenv=ANUVADA_SC=1   # the frames device and slot 0's GPU together (x86_64)
        else fjk=-sc; fjenv=; scenv=ANUVADA_SC=1; net=$sc; fi
    fi
    local ck; ck=$(compiler_key) && [ -n "$ck" ] || { echo "t1-native: REFUSED — no compiler key: $T1_COMPILER_SRC holds no .t1 sources (set SAS to the checkout, or --compiler)" >&2; return 1; }
    tsha=$(strsha "$ck$(sha "$TRANSLATOR")$(sha "$ORACLE")")
    [ -n "$sc" ] && tsha=$(strsha "$tsha$(sha "$(dirname "$ORACLE")/scanout_dev.py")")
    # the image's link base is part of the key (it is in the image sha too; named so a key never hides it)
    dir=$CACHE/$(sha "$img")/$tgt-$tsha-$(strsha "$arg0" | cut -c1-16)-b$(image_base "$img")$fjk$wmk$vdk
    out=$dir/image.native
    if fresh "$out"; then echo "$out"; return 0; fi
    switch T1_BUILD_CACHED_ONLY && return 1
    mkdir -p "$dir"
    tmp=$(tmpname "$out"); log=$tmp.log   # per-process: concurrent cold translations never share a file
    if [ -z "$net" ] && [ -z "$noself" ] && [ -z "${T1N_ORACLE_ONLY:-}" ] && nat=$(native_translator) ; then
        echo "t1-native: translating $(basename "$img") for $tgt by the native translator" >&2
        env -u YANTRA_INPUT_ENTRY YANTRA_INPUT="$img" YANTRA_INPUT_NAME="$arg0" YANTRA_INPUT_TRACE="$code" \
            YANTRA_RAM=1073741824 YANTRA_STEPS=10000000000000 "$nat" > "$tmp" 2> "$log"
    elif [ -z "$net" ] && [ -z "${T1N_ORACLE_ONLY:-}" ] && [ -x "$BIN/yantra-run" ] && anu=$(translator_elf); then
        echo "t1-native: translating $(basename "$img") for $tgt by the .t1 translator under yantra-run" >&2
        env -u YANTRA_INPUT_ENTRY YANTRA_INPUT="$img" YANTRA_INPUT_NAME="$arg0" YANTRA_INPUT_TRACE="$code" \
            YANTRA_RAM=1073741824 YANTRA_STEPS=10000000000000 "$BIN/yantra-run" "$anu" > "$tmp" 2> "$log"
    else
        echo "t1-native: translating $(basename "$img") for $tgt by anuvada.py (the oracle)" >&2
        env $fjenv $netenv $scenv python3 "$ORACLE" --target "$tgt" $wmflag "$img" "$tmp" "$arg0" 2> "$log" &&
            echo "halt: Finisher { value: 21845, status: Some(0) }" >> "$log"
    fi
    grep -aq "status: Some(0)" "$log" && [ -s "$tmp" ] ||
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
