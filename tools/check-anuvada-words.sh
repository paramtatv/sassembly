#!/bin/sh
# tools/check-anuvada-words.sh — the socket words anuvada.py prints are socket's, lib.rs's and
# yantra-run's, octet for octet (SOCKET.md §5): every WHY_* text and the WHY_ set, SOCKET_LIMIT_EXCEEDED,
# SOCKET_LOG_HEADER (+ newline), the files grant line (+ newline), the register address SOCK, SOCK_LEN,
# the latches EMPTY/END/UNREAD and the two caps RECORD_CAP/SESSION_CAP. POSIX sh + awk, no Python, no Rust.
# (Replaces rtprobe/test_sock_words.py.) rc 0 match; rc 1 with a named diff on any mismatch; rc 77 when a
# source file is absent. Env AW_ROOT overrides the repo root (mutant controls run on a scratch copy).
set -u
LC_ALL=C; export LC_ALL
R=${AW_ROOT:-$(cd "$(dirname "$0")/.." && pwd)}
PY=$R/tools/native-spike/anuvada.py
SK=$R/crates/yantra/src/socket.rs
LB=$R/crates/yantra/src/lib.rs
YR=$R/crates/yantra/src/bin/yantra-run.rs
T=$(mktemp -d "${TMPDIR:-/tmp}/anuvada-words.XXXXXX") || exit 77
trap 'rm -rf "$T"' EXIT
# the Rust sources left the tree (R3-YAN); the pinned frozen oracle commit keeps them: an absent file is read from there
fz=$(cat "$R/tools/frozen-rust-oracle.sha" 2>/dev/null)
for f in "$SK" "$LB" "$YR"; do
    [ -f "$f" ] && continue
    rel=${f#"$R"/}; g=$T/fz/$rel
    mkdir -p "$(dirname "$g")" && [ -n "$fz" ] && git -C "$R" show "$fz:$rel" >"$g" 2>/dev/null && [ -s "$g" ] \
        || { echo "SKIP check-anuvada-words: absent $f and not in frozen oracle ${fz:-(no pin)}"; exit 77; }
    case $rel in */socket.rs) SK=$g;; */lib.rs) LB=$g;; *) YR=$g;; esac
done
[ -f "$PY" ] || { echo "SKIP check-anuvada-words: absent $PY"; exit 77; }

# ord table shared by both extractors; each string prints as NAME<TAB>hex octets, each int as NAME<TAB>=n
ORD='BEGIN { for (i = 1; i < 256; i++) ord[sprintf("%c", i)] = i
             H = "0123456789abcdef" }
function hx(n) { return substr(H, int(n / 16) + 1, 1) substr(H, n % 16 + 1, 1) }
function hv(s,   i, v, c) { v = 0; for (i = 1; i <= length(s); i++) { c = index(H, substr(s, i, 1)); v = v * 16 + c - 1 } return v }
function num(s) { gsub(/[ _]/, "", s); if (s ~ /^0x[0-9a-f]+$/) return hv(substr(s, 3)); if (s ~ /^[0-9]+$/) return s + 0; return "?" }
function expr(s,   p, a, b) { p = index(s, "<<"); if (p) { a = num(substr(s, 1, p - 1)); b = num(substr(s, p + 2)); if (a == "?" || b == "?") return "?"; return a * (2 ^ b) } return num(s) }'

# anuvada.py: ("WHY_X", b"...") rows, NAME = b"..." rows, and the first `SOCK.. = ..` int rows
awk "$ORD"'
function pystr(s, q,   i, c, out, d) {
    out = ""
    for (i = 1; i <= length(s); i++) {
        c = substr(s, i, 1)
        if (c == q) break
        if (c == "\\") {
            d = substr(s, i + 1, 1)
            if (d == "x") { out = out tolower(substr(s, i + 2, 2)); i += 3 }
            else if (d == "n") { out = out "0a"; i++ }
            else { out = out hx(ord[d]); i++ }
        } else out = out hx(ord[c])
    }
    return out
}
{
    if (match($0, /^[ ]*\("WHY_[A-Z_]+", b["\x27]/)) {
        line = substr($0, 1, RLENGTH); q = substr(line, RLENGTH, 1)
        split(line, w, "\""); print w[2] "\t" pystr(substr($0, RLENGTH + 1), q)
    } else if (match($0, /^(SOCKET_LIMIT_EXCEEDED|SOCKET_LOG_HEADER|FILES_GRANT_LINE) = b["\x27]/)) {
        line = substr($0, 1, RLENGTH); q = substr(line, RLENGTH, 1)
        print substr(line, 1, index(line, " ") - 1) "\t" pystr(substr($0, RLENGTH + 1), q)
    } else if ($0 ~ /^SOCK[A-Z_]*(, SOCK[A-Z_]+)* = / && $0 !~ /= [({]/) {
        k = index($0, " = "); nn = split(substr($0, 1, k - 1), ks, ", "); vv = split(substr($0, k + 3), vs, ", ")
        if (nn != vv) next
        for (i = 1; i <= nn; i++) if (!(ks[i] in seen)) { seen[ks[i]] = 1; print ks[i] "\t=" expr(vs[i]) }
    }
}' "$PY" | sed -e 's/^SOCK_EMPTY	/EMPTY	/' -e 's/^SOCK_END	/END	/' -e 's/^SOCK_UNREAD	/UNREAD	/' \
    -e 's/^SOCK_RECORD_CAP	/RECORD_CAP	/' -e 's/^SOCK_SESSION_CAP	/SESSION_CAP	/' \
    | awk -F'\t' '/^(WHY_|SOCKET_|FILES_GRANT_LINE|SOCK\t|SOCK_LEN\t|EMPTY\t|END\t|UNREAD\t|RECORD_CAP\t|SESSION_CAP\t)/' \
    | sort -u >"$T/py"

# Rust: `const NAME: &str = "..."` (multi-line, `\` continuation) and `const NAME: uN = a [<< b];`
rsx() {
    awk "$ORD"'
    function rest(s,   i, c, d) {
        for (i = 1; i <= length(s); i++) {
            c = substr(s, i, 1)
            if (skip) { if (c == " " || c == "\t") continue; skip = 0 }
            if (!open) { if (c == "\"") open = 1; continue }
            if (c == "\"") { print name "\t" acc; name = ""; open = 0; return }
            if (c == "\\") {
                d = substr(s, i + 1, 1)
                if (i == length(s)) { skip = 1; return }
                if (d == "n") acc = acc "0a"; else acc = acc hx(ord[d])
                i++
            } else acc = acc hx(ord[c])
        }
    }
    name != "" { rest($0); next }
    match($0, /const [A-Z_]+: &str =/) {
        name = substr($0, RSTART + 6, RLENGTH - 6); name = substr(name, 1, index(name, ":") - 1)
        acc = ""; open = 0; skip = 0; rest(substr($0, RSTART + RLENGTH)); next
    }
    match($0, /const [A-Z_]+: u([0-9]+|size) = [^;]+;/) {
        s = substr($0, RSTART + 6, RLENGTH - 6); n = substr(s, 1, index(s, ":") - 1)
        v = substr(s, index(s, "=") + 2); sub(/;$/, "", v)
        if (!(n in seen)) { seen[n] = 1; print n "\t=" expr(v) }
    }' "$1"
}
{
    rsx "$SK" | awk -F'\t' '/^(WHY_|SOCKET_)/ || /^(SOCK_LEN|EMPTY|END|UNREAD|RECORD_CAP|SESSION_CAP)\t/'
    rsx "$LB" | awk -F'\t' '/^SOCK\t/'
    rsx "$YR" | awk -F'\t' '/^FILES_GRANT_LINE\t/'
} | awk -F'\t' -v OFS='\t' '$1 == "SOCKET_LOG_HEADER" || $1 == "FILES_GRANT_LINE" { $2 = $2 "0a" } { print $1 "\t" $2 }' \
  | sort -u >"$T/rs"

if [ ! -s "$T/py" ] || [ ! -s "$T/rs" ]; then echo "FAIL check-anuvada-words: extracted nothing (py=$(wc -l <"$T/py") rs=$(wc -l <"$T/rs"))"; exit 1; fi
if diff "$T/py" "$T/rs" >"$T/d"; then
    echo "PASS check-anuvada-words: $(grep -a -c '^WHY_' "$T/py") refusals, limit word, log header, grant line, addresses, latches, caps ($(wc -l <"$T/py") items)"
    exit 0
fi
echo "FAIL check-anuvada-words: anuvada.py (<) vs Rust source (>), NAME<TAB>hex-octets or =int:"
cut -c1-200 "$T/d"
exit 1
