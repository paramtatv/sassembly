#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
{ [ -n "${BASH_VERSION:-}" ] && ! shopt -oq posix; } || exec bash "$0" "$@"  # bash --posix (macOS sh) rejects < <(...)
# t1-build.sh — THE EVERYDAY BUILD (owner order): t1_image's arguments, Stage 1's compiler,
# natively where a native Stage 1 exists.
#
#   tools/t1-build.sh [--spec-root D] [--compiler D] [--load F.t1]... --entry MODULE ROUTINE \
#                     -o OUT.elf [F.t1 ...]
#
# THE SOURCES ARE EXACTLY WHAT t1_image COMPILES: its positionals (`--load` only feeds
# t1_image's interpreter). With no positional, the `--load` files are the sources. A module
# such as अष्टक that a source imports but no source declares is NOT packed: the Stage 1
# rung resolves it as t1_image does, and packing it changes the image (measured on the
# Uru kernel at 3e245f49: 22,976 octets alone, equal to t1_image's; 26,270 with अष्टक).
# `--compiler D` names the compiler's .t1 sources (default $SAS/crates/sadhana-t1/src) and `--spec-root D`
# the spec tree its build reads (default $SAS/spec), as for t1_image. BOTH TAKE EFFECT: they enter the
# compiler key (t1-native-lib.sh compiler_key), so a different compiler or spec is a different Stage 1,
# built from them; the t1_image route and the cold Stage 1 build pass them on. A missing or unusable
# directory is REFUSED (exit 1) before anything runs — never a silent fall back to the default
# (tools/check-t1-build-flags.sh holds both negative controls). A Stage 1 for a non-default spec root
# is never made from the seed chain (the seed's tables are baked in): cache or T1_BUILD_COLD only.
#
# THE ROUTE, first that can run (printed on stderr):
#   native    the host's translation of Stage 1 (x86-64 / aarch64 Linux, x86-64 / arm64
#             macOS) by tools/native-spike/t1/anuvada.t1;
#   yantra    Stage 1 under yantra-run;
#   t1_image  t1_image itself (no Stage 1 could be had).
# Stage 1 and its translations are CACHED as tools/t1-native-lib.sh describes (keys name the
# sources, never a host binary: a cache seeded from another host is still never stale).
# A missing Stage 1 (a new compiler key) is made NATIVELY from a trusted earlier image: P(corpus) = A,
# A(corpus) = B, B(corpus) = C, B == C, in seconds; t1_image's cold 25-70 minutes (2,423 s measured on
# x86_64 Linux host) only with T1_BUILD_COLD=1. A missing translation, seconds.
#
# Stage 1 runs with YANTRA_INPUT = the packed sources and YANTRA_INPUT_ENTRY = "MODULE ROUTINE";
# its sink's ELF is cut out exactly as tools/fixpoint.sh does; any status but 1200 is refused.
#
# env: SAS              the Sassembly checkout whose compiler is used (default: this one)
#      SAS_BIN          where yantra-run and t1_image are (default: $SAS/target/release)
#      T1_BUILD_ROUTE   native | yantra | t1_image: force a route
#      T1_BUILD_CACHED_ONLY=1  never build Stage 1 or a translation; exit 77 if absent
#      T1_BUILD_COLD=1  allow the COLD builds (t1_image for Stage 1, or for the program). Without it a missing Stage 1 is made
#                       natively from a trusted earlier image (t1-native-lib.sh stage1_native: seconds), or exit 77
#      T1_STAGE1_SEED   that trusted earlier image, by hand (default: the recorded image, else a pinned ancestor's)
set -uo pipefail
# the REAL directory of this script (follow symlinks; readlink -f is absent on old macOS): a symlinked
# or copied script must find its lib or REFUSE (exit 2), never carry on without the switches
self=$0
while [ -L "$self" ]; do
    d=$(cd -P "$(dirname "$self")" && pwd -P) || exit 2
    l=$(readlink "$self") || exit 2
    case $l in /*) self=$l ;; *) self=$d/$l ;; esac
done
HERE=$(cd -P "$(dirname "$self")" && pwd -P) || { echo "$(basename "$0"): cannot resolve its own directory" >&2; exit 2; }
ROOT=$(dirname "$HERE")
SAS=${SAS:-$ROOT}
BIN=${SAS_BIN:-$SAS/target/release}
. "$HERE/t1-native-lib.sh" || { echo "$(basename "$0"): REFUSED — cannot load $HERE/t1-native-lib.sh" >&2; exit 2; }
type validate_switches >/dev/null 2>&1 || { echo "$(basename "$0"): REFUSED — $HERE/t1-native-lib.sh defines no validate_switches" >&2; exit 2; }
validate_switches   # every switch, up front, in THIS shell: a bad value is exit 64 whatever branch or subshell would read it

say() { echo "t1-build: $*" >&2; }
die() { echo "t1-build: REFUSED — $*" >&2; exit 1; }
# BEGIN relay_refusal
# G0N-D1601: Stage 1 names the refusal in marker 234 (shrinkhala.t1, स्वपरीक्षास्वप्रतिबिम्बम्): stage,
# exit, line, aksara (LE32 each), then the first failed module's name and the reason, each LE32 length +
# octets, ending the sink (its last octet may be the halt's). D1601c: the sink is relayed only up to the
# start of the validated 234 record (max 2000 octets), so the raw record never reaches the console;
# C14: a bounded-valid marker-235 site record (u32le length <= 2000 + octets) is stripped likewise;
# its text is relayed as one decoded `t1-build: SITE <text>` line (tools/t1tests/refuse.sh); marker 233 sits before it, and
# tools/selfcheck/build-refuse.sh reads it from the relay. No valid 234
# record: the old 2000-octet relay. Args: sink status. Writes the relay and the REFUSED line to stdout.
relay_refusal() {
    local sink=$1 st=${2:-} off=2000 line="" res sf rf cf
    if [ -n "$st" ]; then
        # POSIX od + awk record walker (no python): awk prints the cut offset, then the site / refusal lines as printf formats
        # of octal escapes ("S <fmt>", "R <fmt>"), so UTF-8 text reaches the console octet-for-octet (invalid UTF-8 -> U+FFFD)
        res=$(LC_ALL=C od -An -v -tu1 "$sink" 2>/dev/null | LC_ALL=C awk -v status="$st" '
function u32(p) { return b[p] + 256 * b[p + 1] + 65536 * b[p + 2] + 16777216 * b[p + 3] }
function esc(c) { return sprintf("\\%03o", c) }
function lit(c) { return (c >= 48 && c <= 57) || (c >= 65 && c <= 90) || (c >= 97 && c <= 122) || c == 32 || c == 58 || c == 46 || c == 45 }
# decode b[s..e-1] as UTF-8 (Python rules; invalid subpart -> U+FFFD); sets BAD on invalid, NP on a non-printable code point
function dec(s, e,  i, c, n, lo, hi, k, ok, x, l, h, cp, r) {
    r = ""; BAD = 0; NP = 0; i = s
    while (i < e) {
        c = b[i]
        if (c < 128) { if (c < 32 || c == 127) NP = 1; r = r (lit(c) ? sprintf("%c", c) : esc(c)); i++; continue }
        if (c >= 194 && c <= 223) { n = 1; lo = 128; hi = 191 }
        else if (c == 224) { n = 2; lo = 160; hi = 191 }
        else if (c == 237) { n = 2; lo = 128; hi = 159 }
        else if (c >= 225 && c <= 239) { n = 2; lo = 128; hi = 191 }
        else if (c == 240) { n = 3; lo = 144; hi = 191 }
        else if (c >= 241 && c <= 243) { n = 3; lo = 128; hi = 191 }
        else if (c == 244) { n = 3; lo = 128; hi = 143 }
        else { BAD = 1; r = r "\\357\\277\\275"; i++; continue }
        ok = 1
        for (k = 1; k <= n; k++) {
            if (i + k >= e) { ok = 0; break }
            x = b[i + k]; l = (k == 1) ? lo : 128; h = (k == 1) ? hi : 191
            if (x < l || x > h) { ok = 0; break }
        }
        if (!ok) { BAD = 1; r = r "\\357\\277\\275"; i += k; continue }
        if (n == 1) cp = (c - 192) * 64 + (b[i + 1] - 128)
        else if (n == 2) cp = (c - 224) * 4096 + (b[i + 1] - 128) * 64 + (b[i + 2] - 128)
        else cp = (c - 240) * 262144 + (b[i + 1] - 128) * 4096 + (b[i + 2] - 128) * 64 + (b[i + 3] - 128)
        # Python str.isprintable false classes (Cc Cf Zs Zl Zp Co and noncharacters); unassigned code points are not detected
        if ((cp >= 128 && cp <= 160) || cp == 173 || (cp >= 1536 && cp <= 1541) || cp == 1564 || cp == 1757 || cp == 1807 || cp == 2274 || cp == 5760 || cp == 6158 || (cp >= 8192 && cp <= 8207) || (cp >= 8232 && cp <= 8239) || (cp >= 8287 && cp <= 8303) || cp == 12288 || (cp >= 57344 && cp <= 63743) || (cp >= 64976 && cp <= 65007) || cp == 65279 || (cp >= 65529 && cp <= 65531) || cp % 65536 >= 65534 || (cp >= 917504 && cp <= 921599) || cp >= 983040 || cp == 69821 || cp == 69837 || (cp >= 113824 && cp <= 113827) || (cp >= 119155 && cp <= 119162)) NP = 1
        for (k = 0; k <= n; k++) r = r esc(b[i + k])
        i += n + 1
    }
    return r
}
{ for (i = 1; i <= NF; i++) b[n++] = $i + 0 }
END {
    if (status !~ /^[+-]?[0-9]+$/) exit 1
    st = status + 0; found = 0; I = n
    for (k = n - 1; k >= 0; k--) {
        if (b[k] != 234) continue
        j = k + 1
        if (j + 20 > n) continue
        stage = u32(j); ex = u32(j + 4); ln = u32(j + 8); ak = u32(j + 12); nl = u32(j + 16); j += 20
        if (stage < 1 || stage > 9 || ex != st || j + nl + 4 > n) continue
        ns = j; j += nl; rl = u32(j); j += 4
        if (j + rl > n) continue
        rs = j; j += rl
        if (n - j > 1) continue
        found = 1; I = k; break
    }
    off = I; site = 0
    for (k = I - 1; k >= 0; k--) {
        if (b[k] == 235 && k + 5 <= I) {
            sl = u32(k + 1)
            if (sl <= 2000 && (k + 5 + sl == I || (!found && k + 5 + sl == I - 1))) { off = k; site = 1; ss = k + 5; break }
        }
    }
    if (!found && off == I) exit 1
    printf "%d\n", off
    if (site) { t = dec(ss, ss + sl); if (!BAD && !NP) print "S t1-build: SITE " t }
    # trace-3 cause record (marker 233, exit + shape LE32) ends exactly where the site (or the 234 record) starts; it names the
    # Refusal variant of the first failed source: exit 11 = सङ्कलनवर्गविरोधभेद = Refusal::FileMismatch of the oracle (V-005)
    if (off >= 9 && b[off - 9] == 233) {
        cex = u32(off - 8); csh = u32(off - 4)
        printf "C t1-build: CAUSE %s (first failed exit %.0f shape %.0f)\n", (cex == 11 ? "FileMismatch" : "exit " cex), cex, csh
    }
    if (found) {
        nm = nl ? dec(ns, ns + nl) : "?"; rn = rl ? dec(rs, rs + rl) : "?"
        printf "R t1-build: REFUSED %s:%.0f:%.0f %s (exit %.0f stage %.0f)\n", nm, ln, ak, rn, ex, stage
    }
}' || true)
        if [ -n "$res" ]; then
            off=$(printf '%s\n' "$res" | sed -n 1p)
            sf=$(printf '%s\n' "$res" | sed -n 's/^S //p')
            rf=$(printf '%s\n' "$res" | sed -n 's/^R //p')
            cf=$(printf '%s\n' "$res" | sed -n 's/^C //p')
            [ -z "$sf" ] || line=$(printf "$sf")
            if [ -n "$cf" ]; then
                [ -z "$line" ] || line="$line
"
                line=$line$(printf "$cf")
            fi
            if [ -n "$rf" ]; then
                [ -z "$line" ] || line="$line
"
                line=$line$(printf "$rf")
            fi
            [ "$off" -gt 2000 ] && off=2000
        fi
    fi
    head -c "$off" "$sink"
    [ -z "$line" ] || printf '%s\n' "$line"
}
# END relay_refusal

cannot() { echo "CANNOT RUN: $*" >&2; exit 77; }

# a flag's directory, absolute, or REFUSED: never an empty or missing value taken as the default
flag_dir() {
    [ -n "$2" ] || die "$1 needs a directory"
    [ -d "$2" ] || die "$1 $2: no such directory"
    (cd -P "$2" && pwd -P) || die "$1 $2: cannot enter it"
}
loads=(); srcs=(); out=; mod=; rtn=; etext=
while [ $# -gt 0 ]; do
    case $1 in
        --compiler|--spec-root)
            [ $# -ge 2 ] || die "$1 needs a directory"
            case $2 in -*) die "$1 needs a directory, not the flag $2" ;; esac
            d=$(flag_dir "$1" "$2") || exit 1
            if [ "$1" = --compiler ]; then T1_COMPILER_SRC=$d; else T1_SPEC_ROOT=$d; fi
            shift 2 ;;
        --load) loads+=("$2"); shift 2 ;;
        --entry) mod=$2; rtn=$3; shift 3 ;;
        --emit-text) etext=$2; shift 2 ;;   # C26a: keep the raw sink (module TEXT at trace level 2), no ELF
        -o) out=$2; shift 2 ;;
        --no-predict|--accept-divergence) die "$1 is a t1_image flag this wrapper does not take (it predicts nothing)" ;;
        -*) die "unknown flag $1" ;;
        *) srcs+=("$1"); shift ;;
    esac
done
# the compiler: its .t1 sources, one of them the Stage 1 entry module शृङ्खला
compgen -G "$T1_COMPILER_SRC/*.t1" > /dev/null || die "--compiler $T1_COMPILER_SRC holds no .t1 sources"
grep -a -l -m1 '^मण्डलम् शृङ्खला ॥' "$T1_COMPILER_SRC"/*.t1 > /dev/null 2>&1 \
    || die "--compiler $T1_COMPILER_SRC: no source declares मण्डलम् शृङ्खला (the Stage 1 entry module)"
# the spec root: every file the compiler key names as load-bearing (tools/compiler-key-spec.txt `key spec/...`)
while read -r kind path _; do
    [ "$kind" = key ] || continue
    case $path in spec/*) ;; *) continue ;; esac
    case $path in *'*'*|*'?'*|*/) continue ;; esac
    [ -f "$T1_SPEC_ROOT/${path#spec/}" ] || die "--spec-root $T1_SPEC_ROOT has no ${path#spec/} (a file the compiler reads)"
done < <(grep -a -v '^[[:space:]]*#' "$SAS/tools/compiler-key-spec.txt" 2>/dev/null)
spec_default() { [ "$(cd -P "$T1_SPEC_ROOT" && pwd -P)" = "$(cd -P "$SAS/spec" 2>/dev/null && pwd -P)" ]; }
comp_default() { [ "$(cd -P "$T1_COMPILER_SRC" && pwd -P)" = "$(cd -P "$SAS/crates/sadhana-t1/src" 2>/dev/null && pwd -P)" ]; }
spec_default || say "spec root $T1_SPEC_ROOT"
comp_default || say "compiler $T1_COMPILER_SRC"
[ -n "$out" ] || [ -n "$etext" ] || die "no -o OUT.elf"
[ -n "$mod" ] && [ -n "$rtn" ] || die "no --entry MODULE ROUTINE"
[ ${#srcs[@]} -gt 0 ] || srcs=(${loads[@]+"${loads[@]}"})
[ ${#srcs[@]} -gt 0 ] || die "no sources (positional or --load)"
for f in "${srcs[@]}"; do [ -r "$f" ] || die "cannot read $f"; done
# R3P1d: the image is a function of the SET of sources, never of their order (W-224, research/22 section 8).
# Stage 1 packs positionally, so the one canonical order is made here: byte order of the path (LC_ALL=C), duplicates dropped.
canon=$(printf '%s\n' "${srcs[@]}" | LC_ALL=C sort -u) || die "cannot order the sources"
srcs=(); while IFS= read -r f; do srcs+=("$f"); done <<< "$canon"

# One-word rule (TAGSCAN-SPEC section 7): no image leaves this script holding a loader tag word twice.
# rc 77 (the check could not run) is not a pass but not a build failure either; rc 1 refuses the image.
tag_words() {
    sh "$ROOT/tools/check-tag-words.sh" "$out" > "$out.tagwords.log" 2>&1
    case $? in 0) rm -f "$out.tagwords.log" ;; 77) say "tag-words check could not run (exit 77)" ;;
        *) cat "$out.tagwords.log" >&2; die "one-word rule: $out holds a tag word more than once — refused" ;; esac
}

# ---------------------------------------------------------------- the t1_image route
via_t1_image() {
    say "route t1_image ($BIN/t1_image)"
    [ -x "$BIN/t1_image" ] || die "no t1_image at $BIN"
    local args=(--spec-root "$T1_SPEC_ROOT" --compiler "$T1_COMPILER_SRC")
    for f in ${loads[@]+"${loads[@]}"}; do args+=(--load "$f"); done
    rm -f "$out"
    "$BIN/t1_image" "${args[@]}" --entry "$mod" "$rtn" -o "$out" "${srcs[@]}" > "$out.build.log" 2>&1
    local rc=$?
    [ "$rc" -eq 0 ] && [ -s "$out" ] || { tail -20 "$out.build.log" >&2; die "t1_image exited $rc (log $out.build.log)"; }
    tag_words
    say "wrote $out ($(wc -c < "$out" | tr -d ' ') octets) by t1_image"
    exit 0
}
[ -n "$etext" ] && [ "${T1_BUILD_ROUTE:-}" = t1_image ] && die "--emit-text has no t1_image route"
[ "${T1_BUILD_ROUTE:-}" = t1_image ] && via_t1_image

# ---------------------------------------------------------------- Stage 1, keyed by its sources
# one compiler_key compute per build (and none when psx-rerun pinned it): the callee calls it ~6 times, ~2 s each
_cp=${T1_CKEY_PIN:-}
case $_cp in "${_cp%%|*}|${T1_COMPILER_SRC:-}|${T1_SPEC_ROOT:-}") ;; *) _ck=$(compiler_key_compute) && [ -n "$_ck" ] && export T1_CKEY_PIN="$_ck|${T1_COMPILER_SRC:-}|${T1_SPEC_ROOT:-}" ;; esac
stage1() {
    S1DIR=$CACHE/stage1/$(compiler_key)
    S1=$S1DIR/stage1.elf
    stage1_ok "$S1" && return 0       # sealed AND, when the record names this key's image, that image (8f F1)
    local n=
    # the seed chain bakes in the seed's own spec tables: it makes a Stage 1 only for the default spec root
    if spec_default; then n=$(stage1_native_path) || n=; else say "a non-default --spec-root: no native Stage 1 from the seed chain"; fi
    if [ -n "$n" ] && stage1_ok "$n"; then S1=$n; return 0; fi
    switch T1_BUILD_CACHED_ONLY && cannot "no cached Stage 1 for this compiler ($S1)"
    if [ -n "$n" ] && stage1_native "$n"; then   # from a trusted earlier image, natively, in seconds (t1-native-lib.sh)
        stage1_ok "$n" || die "the natively made Stage 1 $n is not the image tools/compiler-image.sha256 records for this key"
        S1=$n; return 0
    fi
    switch T1_BUILD_COLD || { say "no cached Stage 1 and no trusted earlier image to make it from natively (T1_STAGE1_SEED, the image tools/compiler-image.sha256 records, a pinned ancestor); T1_BUILD_COLD=1 builds it with t1_image (25-70 min)"; return 1; }
    [ -x "$BIN/t1_image" ] || { say "no cached Stage 1 and no t1_image to build one (seed the cache: see t1-native-lib.sh)"; return 1; }
    say "building Stage 1 once for this compiler (t1_image, 25-70 min) -> $S1"
    mkdir -p "$S1DIR" || return 1
    rm -f "$S1" "$S1.sha256"          # an entry stage1_ok refused is not to be kept
    local tmp; tmp=$(tmpname "$S1")   # one temp per process: concurrent cold builds never share a file
    # the default compiler keeps its historical repo-relative spelling (names enter nothing, but the log stays as it was)
    local c=crates/sadhana-t1/src; comp_default || c=$T1_COMPILER_SRC
    ( cd "$SAS" && "$BIN/t1_image" --spec-root "$T1_SPEC_ROOT" --compiler "$c" \
        --entry शृङ्खला स्वपरीक्षास्वप्रतिबिम्बम् -o "$tmp" "$c"/*.t1 ) > "$S1DIR/stage1.log.$$" 2>&1
    if [ -s "$tmp" ]; then rm -f "$S1DIR/stage1.log.$$"; publish "$tmp" "$S1" && { stage1_ok "$S1" || die "the t1_image Stage 1 is not the image tools/compiler-image.sha256 records for this key"; }
    else mv -f "$S1DIR/stage1.log.$$" "$S1DIR/stage1.log"; rm -f "$tmp"; say "Stage 1 build failed; see $S1DIR/stage1.log"; return 1; fi
}

# ---------------------------------------------------------------- the host's native Stage 1
native() {
    read -r NATIVE_TGT _ <<< "$(host_target)"
    [ -n "$NATIVE_TGT" ] || return 1
    NATIVE=$(translate "$S1" stage1.elf) && return 0
    switch T1_BUILD_CACHED_ONLY && cannot "no cached $NATIVE_TGT translation of Stage 1"
    return 1
}

route=${T1_BUILD_ROUTE:-auto}
case $route in auto|native|yantra) ;; *) die "T1_BUILD_ROUTE=$route (native | yantra | t1_image)" ;; esac
if ! stage1; then
    [ -z "$etext" ] || die "--emit-text needs a Stage 1 (no t1_image route)"
    [ "$route" = auto ] || die "route $route needs Stage 1, which could not be had"
    switch T1_BUILD_COLD || cannot "no Stage 1 could be had, and the t1_image route is a cold build: T1_BUILD_COLD=1 allows it (or T1_BUILD_ROUTE=t1_image)"
    say "no Stage 1 could be had; falling back"
    via_t1_image
fi
if [ "$route" != yantra ]; then
    if native; then route=native
    elif [ "$route" = native ]; then die "route native: no native Stage 1 for this host"
    else route=yantra; fi
fi
if [ "$route" = yantra ] && ! [ -x "$BIN/yantra-run" ]; then
    [ -z "$etext" ] || die "--emit-text: no yantra-run at $BIN"
    [ "${T1_BUILD_ROUTE:-auto}" = auto ] || die "route yantra: no yantra-run at $BIN"
    via_t1_image
fi

# ---------------------------------------------------------------- pack, run, cut out the ELF
work=$(mktemp -d "${TMPDIR:-/tmp}/t1-build.XXXXXX") || die "no temporary directory"
trap 'rm -rf "$work"' EXIT
sh "$ROOT/tools/pack-corpus.sh" "$work/input.blob" "${srcs[@]}" > "$work/pack.log" || die "packing failed"
if [ "$route" = native ]; then
    say "route native ($NATIVE_TGT: $NATIVE; Stage 1 $S1)"
    env -u YANTRA_INPUT_NAME ${etext:+YANTRA_INPUT_TRACE=2} YANTRA_INPUT="$work/input.blob" YANTRA_INPUT_ENTRY="$mod $rtn" \
        YANTRA_RAM=2684354560 YANTRA_STEPS=10000000000000 "$NATIVE" > "$work/sink" 2> "$work/log"
else
    say "route yantra ($BIN/yantra-run $S1)"
    env -u YANTRA_INPUT_NAME ${etext:+YANTRA_INPUT_TRACE=2} YANTRA_INPUT="$work/input.blob" YANTRA_INPUT_ENTRY="$mod $rtn" \
        YANTRA_RAM=2684354560 YANTRA_STEPS=200000000000 "$BIN/yantra-run" "$S1" > "$work/sink" 2> "$work/log"
fi
status=$(sed -n 's/.*status: Some(\([0-9][0-9]*\)).*/\1/p' "$work/log" | tail -1)
if [ -n "$etext" ]; then cp "$work/sink" "$etext" || die "cannot write $etext"; echo "${status:-NONE}" > "$etext.status"; cp "$work/log" "$etext.log" 2>/dev/null; say "wrote raw sink $etext ($(wc -c < "$etext" | tr -d " ") octets), trace level 2, Stage 1 status ${status:-NONE}"; exit 0; fi
if [ "${status:-}" != 1200 ]; then
    tail -20 "$work/log" >&2
    relay_refusal "$work/sink" "${status:-}" >&2
    die "Stage 1 halted with status ${status:-NONE}, not 1200 (BUILT) — no image written"
fi
elf_off() { # SINK: byte offset of the first "\177ELF", found in bounded windows (never one unbounded grep line); empty when none
    local m o win=1048576 n
    m=$(printf '\177ELF')
    n=$(wc -c < "$1" | tr -d ' ')
    while :; do
        o=$(LC_ALL=C grep -abo -m1 "$m" "$1" 2>/dev/null | cut -d: -f1 | head -n 1)   # K2b: no head|grep pipe (EPIPE/SIGPIPE under load)
        [ -n "$o" ] && { echo "$o"; return 0; }
        [ "$win" -ge "$n" ] && return 1
        win=$((win * 8))
    done
}
cut_elf() { # SINK OUT: copy SINK from its first "\177ELF" to the byte before the last; rc 1 when no ELF
    local off len
    off=$(elf_off "$1")
    [ -n "$off" ] || return 1
    len=$(wc -c < "$1" | tr -d ' ')
    # RXC-SINK: head first, tail (the reader of everything) last - never `tail | head -c`: head exits after its count, the
    # trailing marker octet is then written to a closed pipe, tail dies of SIGPIPE and `set -o pipefail` makes rc 141 (read as "no ELF" under load)
    head -c $((len - 1)) "$1" | tail -c +$((off + 1)) > "$2"
}
cut_elf "$work/sink" "$out" || {
    # K2b: say WHAT the sink holds (no retry loop): head octets in hex, the sink kept for the post-mortem
    kept=${TMPDIR:-/tmp}/t1-build-nosink.$$; cp "$work/sink" "$kept" 2>/dev/null
    die "the sink holds no ELF (sink $(wc -c < "$work/sink" | tr -d ' ') octets, status ${status:-NONE}, head $(od -An -tx1 -N16 "$work/sink" | tr -s ' \n' ' '), kept $kept, log tail: $(tail -n 3 "$work/log" | tr '\n' '|'))"
}
chmod +x "$out"
tag_words
say "wrote $out ($(wc -c < "$out" | tr -d ' ') octets) by Stage 1, route $route"
