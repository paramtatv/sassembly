#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# t1-run.sh — A DROP-IN FOR yantra-run that runs the image natively where it can.
#
#   tools/t1-run.sh [yantra-run flags] IMAGE.elf [args...]
#
# Same environment contract as yantra-run (YANTRA_INPUT, _NAME, _ENTRY, _TRACE, YANTRA_RAM):
# the native runtime implements it and refuses as yantra-run refuses. IMAGE is translated
# for this host (tools/t1-native-lib.sh; cached by the image's sha256, the translator's
# version and ARG0 = IMAGE as given, which is what yantra-run hands the program as argument
# 0), run, and its exit status returned with yantra-run's meaning (0 = a finisher with status
# 0, 1 = any other halt or refusal; the runtime's own 2 for an unreadable input / unmappable
# RAM / missing tag is reported as 1, as yantra-run reports those).
#
# IT RUNS yantra-run INSTEAD, and says why on stderr, when:
#   - a yantra-run flag is given (--events, --record-events, --net-*, --listen, --smp: none
#     has a native counterpart; --version, --source-stamp: they print stamps compiled into the
#     yantra-run binary; with one at hand they ARE yantra-run's answer (H4), else the native runner's own); --files DIR runs natively
#     (YANTRA_FILES=DIR), unless DIR is not a directory;
#   - the environment asks for a yantra-only feature (none is left on Linux: YANTRA_STEPS is native, the translation
#     counts steps; YANTRA_WATERMARK, YANTRA_VERDICT and YANTRA_SCANOUT are native translations that keep the store
#     high-water mark, count yantra's steps and write the verdict file, or run slot 0's GPU, as yantra-run does;
#     YANTRA_VIRTIO_DEFER is native; YANTRA_WATERMARK and YANTRA_SCANOUT are yantra's on a macOS host; YANTRA_VERDICT there is written by this script from the native halt line, finisher or fault, without a steps count);
#   - this host has no native target, or the translator refuses the image;
#   - the native run reaches something the translator refuses AT RUN TIME (an instruction
#     it does not lower, e.g. an ecall for threads or a CSR read; an MMIO load or store at a
#     device yantra answers by its state: the counter, the socket, virtio-mmio slots 0 and 2..7;
#     a jump to a word yantra would execute but the translation does not
#     hold; a store into the code). A word yantra does NOT execute (Unimplemented, ebreak's
#     Breakpoint, sret to pc 0, a fetch below or past RAM) and a WAIT with no event source halt
#     natively exactly as yantra-run does, and are not rerun. Its output is held back
#     until it finishes, so such a run is discarded whole and yantra-run's is the only one
#     seen (stdin, if the program reads it, is not replayed).
# THE NATIVE-ONLY DEVICES (H15): PARAM, the file operations (append … write at), stdin, environment
# and spawn (0x1000_2020..0x1000_2078) have no yantra-run store. On the yantra route (forced, or a
# rerun) a halt at one is passed through and REFUSED by name, exit 3 — the route's limit, not a verdict.
# YANTRA_RECORD_EVENTS=<log> (the native clock's recorder) is honoured on both routes: natively it
# writes the log of the clock waits, on the fallback it becomes `yantra-run --record-events <log>`.
# THE SOCKET (W-377; tools/native-spike/SOCKET.md): `--record-events LOG --listen 127.0.0.1:PORT`
# (yantra-run's own flags, after an optional --files DIR) runs natively on Linux and macOS when PORT
# is canonical decimal 0..65535; any other address, or Windows, goes to yantra-run, which judges
# it. The native contract is three variables, set here and nowhere else:
#   YANTRA_LISTEN=127.0.0.1:PORT    yantra-run's --listen (the runtime binds at load)
#   YANTRA_RECORD_EVENTS=<tmp log>  the socket log (its header, then s=<hex> / s=end per wait)
#   YANTRA_SOCK_SENT=<tmp file>     every octet the program sent; its sha256 replaces the
#                                   "@YANTRA_SOCK_SENT@" marker of the runtime's "socket:" line
# stderr is STREAMED for these runs (held only until "socket: listening on …", so a harness reads
# the port live), stdout goes straight out, and the log is published to LOG. A native-only halt
# BEFORE the listener is bound reruns under yantra-run as below; AFTER it, octets may have crossed
# the connection, so it is a TERMINAL refusal (exit 1), never a rerun.
# THE FRAMES DEVICE (L8/C4; tools/native-spike/NET.md): `--record-events LOG --net-peer PATH` with any of
# --net-mac, --net-max-frame, --net-max-tx-frames, --net-max-rx-frames, --net-timeout (FLAG VALUE or
# FLAG=VALUE) runs natively on x86_64 Linux; --net-tap, any other flag or value yantra-run would judge,
# --files, another host or a threaded image go to yantra-run; YANTRA_VIRTIO_DEFER=N is native (the vd translation). The native contract:
#   YANTRA_NET_PEER, _MAC, _FILTER, _MAX_FRAME, _MAX_TX, _MAX_RX, _TIMEOUT   the device's settings
#   YANTRA_RECORD_EVENTS=<tmp log>  the frames log (its header, then n=<wait>:0:<hex|none> per wait)
#   YANTRA_NET_SENT=<tmp file>      every frame sent (u16 BE length + frame); its sha256 replaces the
#                                   "@YANTRA_NET_SENT@" marker of the runtime's "net:" line
# stderr is streamed once the peer is "connected"/"listening"; after that a native-only halt is a
# TERMINAL refusal (exit 1): frames may have crossed, so it is never rerun.
# env: SAS, SAS_BIN (as tools/t1-build.sh), SASSEMBLY_NATIVE_CACHE, T1_RUN_ROUTE=yantra (force yantra-run),
#      SASSEMBLY_NATIVE_THREADS=N (fork-join regions on N threads, Linux x86-64 / aarch64; default: the CPUs; 0 serial)
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
[ "${T1_RUN_REQUIRE_NATIVE:-}" = 1 ] || [ -x "$BIN/yantra-run" ] || { [ -x "$HOME/.sansos-bin/yantra-run" ] && BIN=$HOME/.sansos-bin; }  # archived build (hosts that have one); never under REQUIRE_NATIVE
. "$HERE/t1-native-lib.sh" || { echo "$(basename "$0"): REFUSED — cannot load $HERE/t1-native-lib.sh" >&2; exit 2; }
type validate_switches >/dev/null 2>&1 || { echo "$(basename "$0"): REFUSED — $HERE/t1-native-lib.sh defines no validate_switches" >&2; exit 2; }
validate_switches   # every switch, up front, in THIS shell: a bad value is exit 64 whatever branch or subshell would read it

say() { echo "t1-run: $*" >&2; }
yantra() {
    # T1_RUN_REQUIRE_NATIVE=1: a caller that must prove a NATIVE run (tools/selfcheck, land) gets a
    # refusal, never a silent yantra-run whose halt line would look the same
    if switch T1_RUN_REQUIRE_NATIVE; then say "REFUSED — native required (T1_RUN_REQUIRE_NATIVE) but: $1"; exit 3; fi
    say "route yantra-run — $1"
    # YANTRA_RECORD_EVENTS=<log> is the native twin of `yantra-run --record-events <log>`; yantra-run
    # itself ignores the variable, so the fallback spells it as the flag (a drop-in, and the log is yantra's)
    if [ -n "${YANTRA_RECORD_EVENTS:-}" ]; then
        # only for an image that declares the event interface: yantra-run refuses --record-events on
        # an image with no SASEVENT tag (exit 1) where native runs it (8f N1); the tag's 8 octets
        # read "SASEVENT" in the image
        case ${orig[0]} in -*) ;; *)
            if LC_ALL=C grep -qa SASEVENT "${orig[0]}" 2>/dev/null; then
                orig=(--record-events "$YANTRA_RECORD_EVENTS" "${orig[@]}")
            fi ;;
        esac
    fi
    [ -x "$BIN/yantra-run" ] || { say "REFUSED — no yantra-run at $BIN to fall back to"; exit 1; }
    # THE NATIVE-ONLY DEVICES (H15, SAS-LIMITS-FIX L20a): yantra-run has no PARAM, file operation
    # (append … write at), stdin, environment or spawn store, and halts at the first one with
    # `halt: Device { …, why: "virtio-mmio slot 1 overlaps the file window …" }`. That halt is the
    # route's limit, not the program's verdict: it is passed through as yantra-run wrote it, then
    # REFUSED by name, exit 3 (as T1_RUN_REQUIRE_NATIVE) — never yantra-run's 1, which a native
    # run of the same image would not give. stdout and stdin stay yantra-run's own.
    local mark rc
    mark=$(mktemp "${TMPDIR:-/tmp}/t1-run-y.XXXXXX") || { say "REFUSED — no temporary file"; exit 1; }
    trap 'rm -f "$mark"' EXIT; trap 'exit 130' INT; trap 'exit 143' TERM   # every caller has dropped $work's trap
    { "$BIN/yantra-run" "${orig[@]}" 2>&1 1>&3 3>&- | awk -v mark="$mark" '
        function hex(v,  h) { h = ""; v = v + 0; while (v > 0) { h = substr("0123456789abcdef", v % 16 + 1, 1) h; v = int(v / 16) } return h == "" ? "0" : h }   # (mawk: %x stops at 2^31)
        BEGIN { n["8224"] = "PARAM"; n["8232"] = "file append"; n["8240"] = "file delete"
                n["8248"] = "file rename"; n["8256"] = "file list"; n["8264"] = "file stat"
                n["8272"] = "mkdir"; n["8280"] = "file read at"; n["8288"] = "file write at"
                n["8296"] = "stdin"; n["8304"] = "environment"; n["8312"] = "spawn" }
        { print; fflush() }
        /^halt: Device \{ pc: [0-9]+, addr: 26844[0-9]+, why: "virtio-mmio slot 1 overlaps the file window/ {
            s = $0; sub(/^halt: Device \{ pc: /, "", s); pc = s; sub(/,.*/, "", pc)
            sub(/^[0-9]+, addr: /, "", s); a = s; sub(/,.*/, "", a)
            o = a - 268435456
            if ((o "") in n) {
                printf "t1-run: REFUSED — native-only device %s (0x%s) at pc 0x%s: yantra-run has no such store; run it natively (tools/native-spike/LIMITS.md)\n", n[o ""], hex(a), hex(pc)
                fflush(); print "" > mark
            }
        }' >&2; } 3>&1
    rc=${PIPESTATUS[0]}
    if [ -s "$mark" ]; then rm -f "$mark"; exit 3; fi
    rm -f "$mark"
    exit "$rc"
}
# ॥ THE EVENT LOG OF A REPLAY (`--events LOG`; SL5, K3) — read here as yantra-run reads it ॥
# (crates/yantra/src/input.rs: parse_event_log, parse_thread_log, refuse_torn_final_record).
# events_read LOG fills ev_n (the records), ev_at (the `@N` records among them) and ev_fmt (each record
# as 16 octets, little-endian: the kind, 0 a value or `t=` record and 1 an `@N`, then the value, as
# printf escapes), or answers 1 with ev_why. It takes a log WHOLE or not at all: what it does not take
# goes to yantra-run, which runs it or refuses it in its own words. So it refuses MORE than yantra-run
# does (a `+` sign, a byte past ASCII, a path whose Debug form is not itself in quotes), never less.
ev_word() {   # ev_word WORD: ev_hex = the 16 hex digits of a u64 record word (t= stripped), or 1
    local w=$1 d a b lo hi
    case $w in
        0[xX]*)
            d=${w#??}; d=${d//_/}
            case $d in ''|*[!0123456789abcdefABCDEF]*) return 1 ;; esac
            while [ "${d#0}" != "$d" ]; do d=${d#0}; done
            [ ${#d} -le 16 ] || return 1
            ev_hex=$(printf '%16s' "$d"); ev_hex=${ev_hex// /0} ;;
        *)
            d=${w//_/}
            case $d in ''|*[!0123456789]*) return 1 ;; esac
            while [ "${d#0}" != "$d" ]; do d=${d#0}; done
            [ ${#d} -le 20 ] || return 1
            if [ ${#d} -gt 10 ]; then a=${d:0:${#d}-10} b=${d: -10}; else a=0 b=${d:-0}; fi
            a=$((10#$a)) b=$((10#$b))
            [ ${#d} -lt 20 ] || [ "$a" -lt 1844674407 ] || { [ "$a" -eq 1844674407 ] && [ "$b" -le 3709551615 ]; } || return 1
            lo=$((b + a * 1410065408))                # a * 10^10 = a * (2 * 2^32 + 1410065408)
            hi=$((2 * a + (lo >> 32))); lo=$((lo & 0xFFFFFFFF))
            ev_hex=$(printf '%08x%08x' "$hi" "$lo") ;;
    esac
}
ev_le() {     # ev_le HEX16: its 8 octets, little-endian, as printf escapes
    local h=$1 i o=
    for i in 14 12 10 8 6 4 2 0; do o=$o'\x'${h:i:2}; done
    printf '%s' "$o"
}
# a socket log (W-377): its header, or its first record an s= one (yantra-run's is_socket_log). sock_parse is
# yantra-run's parse_socket_log in its words: sk_n = every record, END included; a refusal sets ev_why and returns 1
is_sock_log() {
    local line t
    LC_ALL=C grep -aqF '# yantra-run --record-events --listen (W-377)' "$1" && return 0
    while IFS= read -r line || [ -n "$line" ]; do
        t=${line#"${line%%[!$' \t\r']*}"}
        case $t in ''|'#'*) continue ;; 's='*) return 0 ;; *) return 1 ;; esac
    done < "$1"
    return 1
}
sock_parse() {
    local log=$1 line t n=0 ended=0 body last c
    sk_n=0
    if [ -s "$log" ] && [ -n "$(tail -c 1 "$log")" ]; then
        last=$(tail -n 1 "$log"); last=${last#"${last%%[!$' \t\r']*}"}; last=${last%"${last##*[!$' \t\r']}"}
        case $last in ''|'#'*) ;; *) ev_why="line $(grep -ac '' "$log"): \"$last\" has no terminating newline — a torn final record (a live run stopped mid-write), refused rather than replayed"; return 1 ;; esac
    fi
    while IFS= read -r line || [ -n "$line" ]; do
        n=$((n + 1))
        t=${line#"${line%%[!$' \t\r']*}"}; t=${t%"${t##*[!$' \t\r']}"}
        case $t in ''|'#'*) continue ;; esac
        case $t in
            's='*) ;;
            '@'*) ev_why="line $n: \"$t\" is a THREAD record (W-376) — a socket log holds only s=<hex> and s=end lines (W-377: no mixed logs)"; return 1 ;;
            't='*) ev_why="line $n: \"$t\" is a CLOCK record (W-375) — a socket log holds only s=<hex> and s=end lines (W-377: no mixed logs)"; return 1 ;;
            *) ev_why="line $n: \"$t\" is not a socket record — a socket log holds only s=<hex> and s=end lines (W-377: no mixed logs)"; return 1 ;;
        esac
        body=${t#s=}
        if [ "$ended" -gt 0 ]; then ev_why="line $n: \"$t\" comes after the s=end at line $ended — the peer had closed, so no record can follow it"; return 1; fi
        if [ "$body" = end ]; then ended=$n; sk_n=$((sk_n + 1)); continue; fi
        [ -n "$body" ] || { ev_why="line $n: an empty s= — a record carries 1 to 4096 octets, and the peer's close is s=end"; return 1; }
        [ $(( ${#body} % 2 )) -eq 0 ] || { ev_why="line $n: s= with ${#body} hex digits — two per octet, so the length must be even"; return 1; }
        case $body in *[!0123456789abcdef]*)
            c=${body//[0123456789abcdef]/}; c=${c:0:1}
            ev_why="line $n: '$c' in s= is not a lowercase hex digit (0-9, a-f)"; return 1 ;; esac
        [ ${#body} -le 8192 ] || { ev_why="line $n: s= carries $(( ${#body} / 2 )) octets — a record holds at most 4096, the live host's read size"; return 1; }
        sk_n=$((sk_n + 1))
    done < "$log"
    return 0
}
# a frames-device log (yantra-run --record-events with --net-peer): header "# yantra-net v1 k=v ...", then n=K:0:<hex|none>
is_net_log() { LC_ALL=C grep -aq '^# yantra-net v1' "$1"; }
# net_parse LOG: header into n_mac/n_filter/n_maxf/n_maxtx/n_maxrx, hex records into $work/frames (u16 BE length + frame);
# a "none" record needs no file entry (the rx gate is deterministic); sets ev_n to the record count
net_parse() {
    local log=$1 line t kv k v n=0 hx len
    n_mac=02:53:41:4e:53:00 n_filter=0 n_maxf=1514 n_maxtx=65536 n_maxrx=65536 n_tmo=30000
    : > "$work/frames"
    while IFS= read -r line || [ -n "$line" ]; do
        t=${line#"${line%%[!$' \t\r']*}"}; t=${t%"${t##*[!$' \t\r']}"}
        case $t in
            '# yantra-net v1'*)
                for kv in ${t#'# yantra-net v1'}; do
                    k=${kv%%=*}; v=${kv#*=}
                    case $k in
                        mac) n_mac=$v ;;
                        max-frame) n_maxf=$v ;;
                        max-tx) n_maxtx=$v ;;
                        max-rx) n_maxrx=$v ;;
                        filter) case $v in 1|on|true|yes) n_filter=1 ;; *) n_filter=0 ;; esac ;;
                    esac
                done ;;
            ''|'#'*) ;;
            n=*:0:none) n=$((n + 1)) ;;
            n=*:0:*)
                hx=${t##*:}
                case $hx in ''|*[!0123456789abcdef]*) ev_why="line: a frame that is not lowercase hex"; return 1 ;; esac
                [ $(( ${#hx} % 2 )) -eq 0 ] || { ev_why="line: odd hex length"; return 1; }
                len=$(( ${#hx} / 2 )); [ "$len" -le 65535 ] || { ev_why="line: frame too long"; return 1; }
                printf "$(printf '\\x%02x\\x%02x' $((len >> 8)) $((len & 255)))" >> "$work/frames"
                printf "$(printf '%s' "$hx" | sed 's/../\\x&/g')" >> "$work/frames"
                n=$((n + 1)) ;;
            *) ev_why="a record this reader does not take: $t"; return 1 ;;
        esac
    done < "$log"
    ev_n=$n
}
events_read() {
    local log=$1 line t n=0 last k
    ev_n=0 ev_at=0 ev_fmt=
    case $log in ''|*[!-ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._/+,=@%:~\ ]*)
        ev_why="its path is not plain ASCII (yantra-run writes it in Debug form)"; return 1 ;; esac
    [ -f "$log" ] && [ -r "$log" ] || { ev_why="not a readable file"; return 1; }
    if LC_ALL=C grep -aq "$(printf '[^\t\r -~]')" "$log"; then ev_why="a byte that is not printable ASCII"; return 1; fi
    if LC_ALL=C grep -aqF 'was granted a file root' "$log" && [ -z "${YANTRA_FILES:-}" ]; then
        ev_why="recorded with --files"; return 1
    fi
    if [ -s "$log" ] && [ -n "$(tail -c 1 "$log")" ]; then            # no final newline: a torn record?
        last=$(tail -n 1 "$log"); last=${last#"${last%%[!$' \t\r']*}"}
        case $last in ''|'#'*) ;; *) ev_why="a torn final record"; return 1 ;; esac
    fi
    while IFS= read -r line || [ -n "$line" ]; do
        t=${line#"${line%%[!$' \t\r']*}"}; t=${t%"${t##*[!$' \t\r']}"}
        case $t in ''|'#'*) continue ;; esac
        case $t in
            @*)
                k=${t#@}
                case $k in
                    0[xX]*) k=${k#??}; case $k in ''|*[!0123456789abcdefABCDEF]*) ev_why="line $((n+1))"; return 1 ;; esac
                            while [ "${k#0}" != "$k" ]; do k=${k#0}; done; [ ${#k} -le 8 ] || { ev_why="thread number"; return 1; }
                            ev_hex=$(printf '%16s' "$k"); ev_hex=${ev_hex// /0} ;;
                    *) case $k in ''|*[!0123456789]*) ev_why="line $((n+1))"; return 1 ;; esac
                       while [ "${k#0}" != "$k" ]; do k=${k#0}; done; [ ${#k} -le 10 ] || { ev_why="thread number"; return 1; }
                       [ $((10#${k:-0})) -le 4294967295 ] || { ev_why="thread number"; return 1; }
                       ev_hex=$(printf '%016x' $((10#${k:-0}))) ;;
                esac
                [ "$ev_at" = 0 ] && ev_atwhy="line $((n+1)): \"$t\""
                ev_fmt=$ev_fmt'\x01\x00\x00\x00\x00\x00\x00\x00'$(ev_le "$ev_hex"); ev_at=$((ev_at + 1)) ;;
            s=*) ev_sockrec="line $((n+1)): \"$t\" is a SOCKET record (W-377: \`s=<hex>\` or \`s=end\`) — this is a socket log, read only when its first record is one; refused rather than skipped"; return 2 ;;
            *)
                ev_word "${t#t=}" || { ev_why="line $((n+1)) is not a record this reader takes"; return 1; }
                ev_fmt=$ev_fmt'\x00\x00\x00\x00\x00\x00\x00\x00'$(ev_le "$ev_hex") ;;
        esac
        n=$((n + 1))
    done < "$log"
    ev_n=$n
}
# --attest (R5-S1, verbose only): the fork-join runtime writes its one fj_attest line to a private file, and it is
# printed to stderr after the run; a thread count above the cap (64) is refused here, exit 64
attest=
if [ "${1-}" = --attest ]; then attest=1; shift; fi
orig=("$@")
unset YANTRA_LISTEN YANTRA_SOCK_REPLAY YANTRA_SOCK_SENT YANTRA_REPLAY_EVENTS   # the native contracts are set below, never inherited
unset YANTRA_NET_PEER YANTRA_NET_MAC YANTRA_NET_FILTER YANTRA_NET_MAX_FRAME YANTRA_NET_MAX_TX YANTRA_NET_MAX_RX \
    YANTRA_NET_TIMEOUT YANTRA_NET_SENT YANTRA_NET_REPLAY        # the frames device's native contract: set below, never inherited
[ $# -gt 0 ] || { say "usage: t1-run.sh [--files DIR] [yantra-run flags] IMAGE.elf [args...]"; exit 64; }
[ "${T1_RUN_ROUTE:-}" = yantra ] && yantra "T1_RUN_ROUTE=yantra"
# --listen refusals, in yantra-run's own words and before anything is read or bound (W-377): the flag
# is accepted only right after --record-events <log>; the address must be 127.0.0.1:PORT or [::1]:PORT
if [ "$1" = --listen ] || { [ "$1" = --events ] && [ $# -ge 3 ] && [ "$3" = --listen ]; }; then
    echo "socket: refused — --listen is accepted only right after --record-events <log>: a live socket run that leaves no log cannot be replayed (W-377; refused before any bind, nothing ran)" >&2
    exit 1
fi
# --files DIR (first, as yantra-run takes it): the native runtime's file window takes its root from
# YANTRA_FILES (7a983fe4). A DIR that is not a directory stays yantra-run's, which refuses it at load
# where the native runtime would refuse each request.
# yantra-run's own front-half refusals (a bare --files, a DIR that is missing or not a directory) are
# answered here in its words when no yantra-run binary exists: the binary is retired, the refusals are not
if [ "$1" = --files ] && [ ! -x "$BIN/yantra-run" ]; then
    [ $# -ge 2 ] || { echo "usage: yantra-run --files DIR [other flags] <program.elf> [args...] (t1-run.sh --files DIR)" >&2; exit 1; }
    [ -d "$2" ] || { if [ -e "$2" ]; then echo "files: $2: not a directory (refused at load; nothing ran)" >&2; else echo "files: $2: No such file or directory (refused at load; nothing ran)" >&2; fi; exit 1; }
    [ $# -ge 3 ] || { echo "usage: yantra-run --files DIR [other flags] <program.elf> [args...] (t1-run.sh --files DIR)" >&2; exit 1; }
fi
if [ "$1" = --files ] && [ $# -ge 3 ]; then
    [ -d "$2" ] || yantra "--files $2 is not a directory (yantra-run refuses it at load)"
    export YANTRA_FILES=$2
    shift 2
fi
# --events LOG (first, or after --files DIR, as yantra-run takes it): A REPLAY RUNS NATIVELY (SL5,
# NATIVE-GAPS B3). The log is read by events_read (above); its records reach the runtime through
# YANTRA_REPLAY_EVENTS, and a wait takes the next one in place of the clock (rt_next_rec). The runtime
# names the tag's RAM offset at startup, or refuses a missing or duplicated tag (rerun under
# yantra-run, which refuses it at load); its private log of what it delivered says how many were.
# yantra-run's own lines are written from those: the load line, "N of N delivered", and the
# refusals of a SHORTER or LONGER log, which outrank the halt (exit 1).
ev_log= ev_sock= sk_n=0 ev_net= ev_nrec=0 work=
if [ "$1" = --events ] && [ $# -ge 3 ]; then
    case $2 in -*) yantra "--events $2: a log path that reads as a flag (yantra-run judges it)" ;; esac
    ev_sock=
    case $2 in ''|*[!-ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._/+,=@%:~\ ]*) ;; *)
        if [ -f "$2" ] && [ -r "$2" ] && is_sock_log "$2"; then
            ev_sock=1
            sock_parse "$2" || { echo "events: refused — the socket log \"$2\": $ev_why" >&2; exit 1; }
        fi ;;
    esac
    ev_net=
    if [ -z "$ev_sock" ] && [ -f "$2" ] && [ -r "$2" ] && is_net_log "$2"; then
        ev_net=1
        [ "$(uname -s) $(uname -m)" = "Linux x86_64" ] || yantra "a frames-device log replays natively on x86_64 Linux only (NET.md)"
        [ -z "${YANTRA_FILES:-}" ] || yantra "--files with the frames device (NET.md: phase 2)"
        work=$(mktemp -d "${TMPDIR:-/tmp}/t1-run.XXXXXX") || yantra "no temporary directory"
        trap 'rm -rf "$work"' EXIT
        net_parse "$2" || yantra "--events $2: $ev_why (yantra-run judges it)"
        ev_nrec=$ev_n; ev_at=0
    elif [ -z "$ev_sock" ]; then
        ev_rc=0; events_read "$2" || ev_rc=$?
        if [ "$ev_rc" = 2 ]; then echo "events: refused — the log \"$2\": $ev_sockrec" >&2; exit 1; fi
        [ "$ev_rc" = 0 ] || yantra "--events $2: $ev_why (yantra-run judges it)"
    fi
    ev_log=$2
    [ -z "$ev_net" ] || net=1
    shift 2
    ev_thr=; LC_ALL=C grep -qa SASTHRDS "$1" 2>/dev/null && ev_thr=1
    [ -z "$ev_net" ] || [ -z "$ev_thr" ] || yantra "a threaded image with the frames device (yantra-run judges it)"
    # a lone tag (SASTHRDS without SASTHRID, or the reverse) is a load refusal in yantra-run and in wasm-host.mjs (PD47 "lone"); the binary is retired, the refusal is not
    if [ ! -x "$BIN/yantra-run" ]; then
        ev_hid=; LC_ALL=C grep -qa SASTHRID "$1" 2>/dev/null && ev_hid=1
        if [ -n "$ev_thr" ] && [ -z "$ev_hid" ]; then
            echo "the image declares SASTHRDS without SASTHRID: a threaded image must declare both — the thread count and the slot the host writes each thread's number into (nothing ran)" >&2; exit 1
        fi
        if [ -z "$ev_thr" ] && [ -n "$ev_hid" ]; then
            echo "the image declares SASTHRID without SASTHRDS: a threaded image must declare both — the thread count and the slot the host writes each thread's number into (nothing ran)" >&2; exit 1
        fi
    fi
    # an image with no SASEVENT tag has no event interface: yantra-run refuses the log at load (exit 1, no verdict); the binary is retired, the refusal is not
    if [ -n "$ev_net" ] && ! LC_ALL=C grep -qa SASEVENT "$1" 2>/dev/null; then
        echo "events: refused — no event interface in $1 (no SASEVENT tag; nothing ran)" >&2; exit 1
    fi
    if [ -n "$ev_sock" ] && ! LC_ALL=C grep -qa SASEVENT "$1" 2>/dev/null; then
        echo "events: refused — a socket run needs the event slot: no event interface in $1 (no SASEVENT tag; nothing ran)" >&2; exit 1
    fi
    if [ ! -x "$BIN/yantra-run" ] && [ -z "$ev_thr" ] && ! LC_ALL=C grep -qa SASEVENT "$1" 2>/dev/null; then
        echo "events: refused — no event interface in $1 (SASEVENT tag missing or duplicated; nothing ran)" >&2; exit 1
    fi
    # a threaded image: the load refusals of yantra-run (threads.t1a, kinds 5 and 6), judged here because the binary is retired
    if [ -z "$ev_sock" ] && [ -n "$ev_thr" ] && [ ! -x "$BIN/yantra-run" ]; then
        thr_cnt=$(LC_ALL=C grep -boa SASTHRDS "$1" | head -1); thr_cnt=${thr_cnt%%:*}
        thr_cnt=$(od -An -v -tu1 -j $((thr_cnt + 8)) -N8 "$1" | awk '{ for (i = 1; i <= NF; i++) v += $i * 256 ^ (i - 1) } END { printf "%.0f", v }')
        if [ "$thr_cnt" = 0 ] || [ "$thr_cnt" -gt 64 ] 2>/dev/null; then
            echo "threads: refused — the THRDS count is $thr_cnt: an image declares 1 to 64 threads (nothing ran)" >&2; exit 1
        fi
        if ! LC_ALL=C grep -qa SASEVENT "$1" 2>/dev/null; then
            echo "threads: refused — a threaded image needs the event interface — no event interface in this image: the event tag (0x544e455645534153) appears at no word. It was built without the event global — declare it, or do not pass an event log (nothing ran)" >&2; exit 1
        fi
    fi
    if [ -z "$ev_sock" ] && [ "$ev_at" -gt 0 ] && [ -z "$ev_thr" ] && [ ! -x "$BIN/yantra-run" ]; then
        echo "events: refused — the log \"$ev_log\": $ev_atwhy is a THREAD record (W-376: \`@N\` runs thread N) — this is a threaded log, and a single-thread run has no thread to schedule; refused rather than skipped" >&2; exit 1
    fi
    if [ -z "$ev_sock" ] && [ "$ev_at" -gt 0 ] && [ -z "$ev_thr" ]; then
        yantra "--events $ev_log holds thread records and the image is not threaded (yantra-run refuses it)"
    fi
    [ "$(uname -s)" = Linux ] || [ "$(uname -s)" = Darwin ] || yantra "--events runs natively on Linux and macOS"
fi
# --version / --source-stamp: describe the native runner itself (C6, 2026-10-10)
case $#:$1 in
    1:--version | 1:--source-stamp)
        # yantra-run prints the stamp compiled into ITS binary: with one at hand, its answer is the answer (H4)
        [ -x "$BIN/yantra-run" ] && ! switch T1_RUN_REQUIRE_NATIVE && yantra "$1 prints the stamp compiled into yantra-run" ;;
esac
case $#:$1 in
    1:--version)
        say "route native"
        c=$(git -C "$HERE" rev-parse HEAD 2>/dev/null || cat "$HERE/compiler-image.sha256" 2>/dev/null | cut -d" " -f1 || echo unknown)
        echo "version: $c" >&2
        exit 0
        ;;
    1:--source-stamp)
        say "route native"
        s=$(sh "$HERE/src-rev.sh" "$HERE/../tools/native-spike" 2>/dev/null || echo unknown)
        echo "source: $s" >&2
        exit 0
        ;;
esac
sock= sock_log=
if [ "$1" = --record-events ] && [ $# -ge 4 ] && [ "$3" = --listen ]; then
    case $4 in
        127.0.0.1:[0-9]* | \[::1\]:[0-9]*) ;;
        *) echo "socket: refused — --listen \"$4\" is not 127.0.0.1:PORT or [::1]:PORT; only the loopback is served and a name is never looked up (refused before any bind; nothing ran)" >&2
           exit 1 ;;
    esac
fi
if [ "$1" = --record-events ] && [ $# -ge 5 ] && [ "$3" = --listen ]; then
    case $(uname -s) in Linux|Darwin) ;; *) yantra "--listen runs natively on Linux and macOS only (SOCKET.md)" ;; esac
    case $4 in
        127.0.0.1:0 | 127.0.0.1:[1-9] | 127.0.0.1:[1-9][0-9] | 127.0.0.1:[1-9][0-9][0-9] | \
        127.0.0.1:[1-9][0-9][0-9][0-9] | 127.0.0.1:[1-9][0-9][0-9][0-9][0-9]) ;;
        *) yantra "--listen $4 is not 127.0.0.1:PORT (yantra-run judges it)" ;;
    esac
    [ "${4#127.0.0.1:}" -le 65535 ] || yantra "--listen $4: the port is past 65535 (yantra-run judges it)"
    sock=$4 sock_log=$2
    shift 4
fi
net=${ev_net:+1} net_log=    # a frames-log replay (--events) has already set net
if [ "$1" = --record-events ] && [ $# -ge 4 ]; then
    case $3 in --net-*)
        n_peer= n_mac=02:53:41:4e:53:00 n_filter=0 n_maxf=1514 n_maxtx=65536 n_maxrx=65536 n_tmo=30000
        net_log=$2
        shift 2
        # canonical decimal, at most 18 digits (yantra-run judges anything else)
        num() { case $1 in ''|*[!0-9]*|0?*) return 1 ;; esac; [ ${#1} -le 18 ]; }
        while [ $# -gt 0 ]; do
            case $1 in --net-*) ;; *) break ;; esac
            case $1 in
                *=*) f=${1%%=*} v=${1#*=}; shift ;;
                *) f=$1; [ $# -ge 2 ] || yantra "$f has no value (yantra-run judges it)"; v=$2; shift 2 ;;
            esac
            case $f in
                --net-peer) [ -n "$v" ] && [ "$(printf %s "$v" | LC_ALL=C wc -c)" -lt 108 ] ||
                                yantra "--net-peer $v: not a socket path of 1..107 octets"
                            n_peer=$v ;;
                --net-mac)  v=$(printf %s "$v" | tr A-F a-f)
                            case $v in [0-9a-f][0-9a-f]:[0-9a-f][0-9a-f]:[0-9a-f][0-9a-f]:[0-9a-f][0-9a-f]:[0-9a-f][0-9a-f]:[0-9a-f][0-9a-f]) ;;
                                *) yantra "--net-mac $v is not six hex octets (yantra-run judges it)" ;; esac
                            n_mac=$v n_filter=1 ;;
                --net-max-frame) num "$v" && [ "$v" -ge 14 ] && [ "$v" -le 65535 ] || yantra "--net-max-frame $v (yantra-run judges it)"; n_maxf=$v ;;
                --net-max-tx-frames) num "$v" || yantra "--net-max-tx-frames $v (yantra-run judges it)"; n_maxtx=$v ;;
                --net-max-rx-frames) num "$v" || yantra "--net-max-rx-frames $v (yantra-run judges it)"; n_maxrx=$v ;;
                --net-timeout) num "$v" && [ "$v" -gt 0 ] && [ "$v" -le 1000000000000 ] || yantra "--net-timeout $v (yantra-run judges it)"; n_tmo=$v ;;
                *) yantra "$f has no native counterpart (NET.md: phase 2)" ;;
            esac
        done
        [ -n "$n_peer" ] || yantra "a frames run with no --net-peer (the TAP backend is phase 2, NET.md)"
        [ "$(uname -s) $(uname -m)" = "Linux x86_64" ] || yantra "the frames device runs natively on x86_64 Linux only (NET.md)"
        [ -z "${YANTRA_FILES:-}" ] || yantra "--files with the frames device (NET.md: phase 2)"
        [ $# -gt 0 ] || yantra "no image"
        case $1 in -*) yantra "the yantra-run flag $1 has no native counterpart" ;; esac
        [ -r "$1" ] && ! LC_ALL=C grep -qa SASTHRDS "$1" || yantra "$1: a threaded or unreadable image (yantra-run judges it)"
        net=1
        ;;
    esac
fi
case $1 in -*) yantra "the yantra-run flag $1 has no native counterpart" ;; esac
# YANTRA_WATERMARK runs natively on Linux (the translation that keeps the store high-water mark, wmt below);
# on a macOS host the translator has no such mode, so it stays yantra's
wmt=
if [ -n "${YANTRA_WATERMARK+x}" ]; then
    case "$(uname -s)" in Linux) wmt=wm ;; *) yantra "YANTRA_WATERMARK is a yantra-only feature on this host" ;; esac
fi
# YANTRA_VERDICT runs natively on Linux (the translation that counts yantra's steps, keeps the step ceiling YANTRA_STEPS
# and writes the verdict file, vdt below; it keeps the store high-water mark too); on any other host the plain native run is used and the verdict file is written from its halt line (vdsh, below; no steps line)
vdt=
vdsh=
if [ -n "${YANTRA_VERDICT+x}" ]; then
    case "$(uname -s)" in Linux) vdt=vd; wmt= ;; *) vdsh=1 ;; esac
fi
# YANTRA_SCANOUT runs natively on Linux (slot 0's GPU in the translation, sct below; tools/native-spike/scanout_dev.py)
sct=
if [ -n "${YANTRA_SCANOUT+x}" ]; then
    case "$(uname -s)" in Linux) sct=sc ;; *) yantra "YANTRA_SCANOUT is a yantra-only feature on this host" ;; esac
    [ -z "$wmt" ] && [ -z "$vdt" ] || yantra "YANTRA_SCANOUT with YANTRA_WATERMARK or YANTRA_VERDICT is yantra's"
    [ -z "$ev_sock$sock" ] || yantra "YANTRA_SCANOUT with a socket is yantra's"
fi
# YANTRA_VIRTIO_DEFER=N (L33) is native: the frames device defers its completions by N words (a translation that
# ticks the clock before every word, vdn below), and every other native run says yantra-run's load line itself
# (vdline); a value yantra-run refuses (not a decimal of at most 2^64 - 1) is yantra-run's to refuse
vdn= vdline=
if [ -n "${YANTRA_VIRTIO_DEFER+x}" ]; then
    case $YANTRA_VIRTIO_DEFER in ''|*[!0-9]*) yantra "YANTRA_VIRTIO_DEFER=$YANTRA_VIRTIO_DEFER is not a decimal number (yantra-run judges it)" ;; esac
    vdd=${YANTRA_VIRTIO_DEFER#"${YANTRA_VIRTIO_DEFER%%[!0]*}"}
    if [ ${#vdd} -gt 20 ] || { [ ${#vdd} -eq 20 ] && [[ $vdd > 18446744073709551615 ]]; }; then
        yantra "YANTRA_VIRTIO_DEFER=$YANTRA_VIRTIO_DEFER is past 2^64 - 1 (yantra-run judges it)"
    fi
    if [ -n "$vdd" ]; then
        vdn=$vdd vdline="virtio: completions deferred by $vdd instruction(s) (YANTRA_VIRTIO_DEFER)"
    fi
fi
img=$1; shift
# STEP CEILING: native code honours YANTRA_STEPS exactly as yantra-run does (anuvada counts steps).
# Unset, both routes get the budget the compiler runs get (fixpoint.sh Stage 2), not yantra-run's
# bare 1,000,000, so a long program ends the same way on either route.
export YANTRA_STEPS="${YANTRA_STEPS:-4000000000000}"
# a missing, unreadable or directory IMAGE: yantra-run's own text (its `expect("read the ELF")`, which
# panics and exits 134), said here without the panic, exit 1 (SL3)
if [ ! -r "$img" ] || [ -d "$img" ]; then
    if [ -d "$img" ]; then e='code: 21, kind: IsADirectory, message: "Is a directory"'
    elif [ -e "$img" ]; then e='code: 13, kind: PermissionDenied, message: "Permission denied"'
    else e='code: 2, kind: NotFound, message: "No such file or directory"'
    fi
    echo "read the ELF: Os { $e }" >&2
    exit 1
fi
[ -n "$(host_target)" ] || yantra "no native target for $(uname -s) $(uname -m)"
# FORK-JOIN BY DEFAULT (tools/native-spike/FORKJOIN.md): the fork-join translation, whose समान्तरॱचालनम्
# regions run on threads where the image calls the marker — SASSEMBLY_NATIVE_THREADS=N threads, unset: the CPUs
# this process may use (at most 64; small regions serial, SASSEMBLY_NATIVE_FJ_MIN). 0: serial, the plain
# translation. Linux x86-64 / aarch64; macOS is serial for now (t1-native-lib translate: no fork-join there)
fjt=fj
case ${SASSEMBLY_NATIVE_THREADS-} in
    0) fjt= ;;
    '') unset SASSEMBLY_NATIVE_THREADS ;;       # empty is unset: the native runtime then counts the CPUs
    *[!0-9]*) say "REFUSED — SASSEMBLY_NATIVE_THREADS=$SASSEMBLY_NATIVE_THREADS: a thread count (0 = serial, unset = the CPUs)"; exit 64 ;;
    *) fjt=fj ;;
esac
case ${SASSEMBLY_NATIVE_FJ_MIN-0} in
    *[!0-9]*|'') say "REFUSED — SASSEMBLY_NATIVE_FJ_MIN=$SASSEMBLY_NATIVE_FJ_MIN: indices per thread (0 = no threshold)"; exit 64 ;;
esac
[ -n "$wmt$vdt" ] && fjt=
[ -n "$sct" ] && fjt=
if [ -n "$net" ]; then
    [ -z "$vdt" ] || yantra "YANTRA_VERDICT with a socket is yantra's"
    nat=$(translate "$img" "$img" "" "" net "$wmt" "$vdt" "$sct" "$vdn") || yantra "the translator refused $img"
else
    nat=$(translate "$img" "$img" "" "$fjt" "" "$wmt" "$vdt" "$sct") || yantra "the translator refused $img"
fi

if [ -z "$work" ]; then
    work=$(mktemp -d "${TMPDIR:-/tmp}/t1-run.XXXXXX") || yantra "no temporary directory"
    trap 'rm -rf "$work"' EXIT
fi
if [ -n "$attest" ]; then
    case ${SASSEMBLY_NATIVE_THREADS-0} in *[!0-9]*) ;; *) [ "${SASSEMBLY_NATIVE_THREADS:-0}" -le 64 ] || { say "REFUSED — SASSEMBLY_NATIVE_THREADS=$SASSEMBLY_NATIVE_THREADS: above the cap of 64"; exit 64; } ;; esac
    export SASSEMBLY_NATIVE_FJATTEST="$work/attest"
fi
# a socket run's side file ($work/sent) must be out of the guest's reach: a --files root at or above
# $work would let the native-only window operations read or append it (8f low), so such a run is
# yantra-run's (no side file there)
if [ -n "$sock" ] && [ -n "${YANTRA_FILES:-}" ]; then
    froot=$(cd -P "$YANTRA_FILES" 2>/dev/null && pwd -P) || froot=
    wreal=$(cd -P "$work" && pwd -P)
    inside=
    [ "$froot" = / ] && inside=1                        # the root of the filesystem holds everything
    case $wreal/ in "$froot"/*) inside=1 ;; esac
    [ -n "$inside" ] && { rm -rf "$work"; trap - EXIT; yantra "--files ${YANTRA_FILES} contains t1-run's private side file"; }
fi
say "route native ($nat)"
# the halts only yantra-run can finish: "native: …" and the runtime's own "halt: BadAccess (…)".
# yantra's own forms are native's answer too, and kept: "halt: BadAccess { pc, addr }" (a UART or
# FINISHER load, any load or store below base at no device) and "halt: Device { pc, addr, why }"
# (a load, or a store but patra's four, in virtio-mmio slot 1, the file window's slot)
native_only() { grep -aq '^native: \(not in milestone 1\|jump to\|refused a store\|guest RAM could not be mapped\)\|^halt: BadAccess (' "$work/err"; }
why_native() { grep -a '^native: \|halt: BadAccess (' "$work/err" | head -1; }
rerun() {
    say "the native run reached what the translator refuses at run time: $(why_native)"
    rm -rf "$work"
    trap - EXIT
    yantra "rerun whole under yantra-run"
}
if [ -n "$sock" ]; then
    sent_sha() { if command -v sha256sum >/dev/null; then sha256sum; else shasum -a 256; fi | cut -d' ' -f1; }
    # stderr: held until the listener is announced, then streamed line by line; the report's
    # marker becomes the sha256 of what the program sent (yantra-run's "sent sha256")
    sock_filter() {
        local line live=
        while IFS= read -r line || [ -n "$line" ]; do
            printf '%s\n' "$line" >> "$work/err"
            case $line in
                "socket: "*" sent sha256 @YANTRA_SOCK_SENT@")
                    line=${line%@YANTRA_SOCK_SENT@}$(cat "$work/sent" 2>/dev/null | sent_sha) ;;
            esac
            if [ -z "$live" ]; then
                case $line in
                    "socket: listening on "*) live=1; : > "$work/live"; [ -f "$work/held" ] && cat "$work/held" >&2 ;;
                    *) printf '%s\n' "$line" >> "$work/held"; continue ;;
                esac
            fi
            printf '%s\n' "$line" >&2
        done
    }
    # yantra-run says its YANTRA_VIRTIO_DEFER line at load, before the listener is bound
    [ -z "$vdline" ] || { printf '%s\n' "$vdline" > "$work/held"; printf '%s\n' "$vdline" > "$work/err"; }
    exec 3>&1
    YANTRA_LISTEN=$sock YANTRA_RECORD_EVENTS="$work/events.log" YANTRA_SOCK_SENT="$work/sent" \
        "$nat" "$@" 2>&1 >&3 3>&- | sock_filter
    rc=${PIPESTATUS[0]}
    exec 3>&-
    if [ ! -f "$work/live" ]; then
        native_only && rerun                     # nothing was bound: the rerun is whole
        [ -f "$work/err" ] && cat "$work/err" >&2
    else
        [ -f "$work/events.log" ] && cp "$work/events.log" "$sock_log"
        if native_only; then
            say "socket: REFUSED — the native run reached what the translator refuses at run time ($(why_native)) after the listener was bound; it is NOT rerun under yantra-run, which would serve the connection a second time (W-377, SOCKET.md)"
            rc=1
        fi
    fi
    [ "$rc" -eq 2 ] && rc=1
    exit "$rc"
fi
if [ -n "$ev_net" ]; then
    # A FRAMES-LOG REPLAY: the runtime reads the frames file (YANTRA_NET_REPLAY: u16 BE length + frame per delivered
    # record) through the ordinary receive path; the rx gate is deterministic, so "none" records have no entry. A log
    # that ends while the device could still receive takes the runtime's closed-refusal (a divergence, not a pass).
    sent_sha() { if command -v sha256sum >/dev/null; then sha256sum; else shasum -a 256; fi | cut -d' ' -f1; }
    : > "$work/sent"
    YANTRA_NET_REPLAY="$work/frames" YANTRA_NET_MAC=${n_mac//:/} YANTRA_NET_FILTER=$n_filter YANTRA_NET_MAX_FRAME=$n_maxf \
        YANTRA_NET_MAX_TX=$n_maxtx YANTRA_NET_MAX_RX=$n_maxrx YANTRA_NET_TIMEOUT=$n_tmo YANTRA_NET_SENT="$work/sent" \
        "$nat" "$@" > "$work/out" 2> "$work/err"
    rc=$?
    native_only && rerun
    cat "$work/out"
    refused=
    while IFS= read -r line; do
        case $line in
            "net: "*" sent sha256 @YANTRA_NET_SENT@") line=${line%@YANTRA_NET_SENT@}$(sent_sha < "$work/sent") ;;
            "events: REFUSED"*|"net: refused"*) refused=1 ;;
        esac
        printf '%s\n' "$line"
    done < "$work/err" >&2
    if [ -z "$refused" ] && [ "$rc" -eq 0 ]; then
        echo "events: $ev_nrec frames-device records from \"$ev_log\" replayed" >&2
    fi
    [ "$rc" -eq 2 ] && rc=1
    exit "$rc"
fi
if [ -n "$ev_sock" ]; then
    # A SOCKET-LOG REPLAY (W-377 phase 2): the runtime maps the log (YANTRA_SOCK_REPLAY), delivers one record per
    # wait, writes no record log; "received in R records" in its report line is the delivered count (END included)
    sent_sha() { if command -v sha256sum >/dev/null; then sha256sum; else shasum -a 256; fi | cut -d' ' -f1; }
    : > "$work/sent"
    YANTRA_SOCK_REPLAY=$ev_log YANTRA_SOCK_SENT="$work/sent" "$nat" "$@" > "$work/out" 2> "$work/err"
    rc=$?
    native_only && rerun
    cat "$work/out"
    delivered=0 refused=
    while IFS= read -r line; do
        case $line in
            "socket: "*" sent sha256 @YANTRA_SOCK_SENT@") line=${line%@YANTRA_SOCK_SENT@}$(sent_sha < "$work/sent") ;;
        esac
        case $line in
            "socket: "*" octets received in "*" records, "*) d=${line#*received in }; delivered=${d%% *} ;;
            "events: REFUSED"*) refused=1 ;;
        esac
        printf '%s\n' "$line"
    done < "$work/err" >&2
    if [ -z "$refused" ] && [ "$rc" -eq 0 ]; then
        halt=$(grep -a '^halt: ' "$work/err" | head -1); halt=${halt#halt: }
        if [ "$delivered" -lt "$sk_n" ]; then
            echo "events: REFUSED — the socket log is LONGER than the run: $delivered of $sk_n records delivered, $((sk_n - delivered)) unconsumed; the program halted $halt without waiting again, so this log does not describe this run" >&2
            exit 1
        fi
        echo "events: $delivered of $sk_n socket records delivered, one per wait" >&2
    fi
    [ "$rc" -eq 2 ] && rc=1
    exit "$rc"
fi
# a threaded replay whose log does not fit the run: the runtime only says so (exit 2); yantra-run named the record (threads.t1a
# sutra-lagu / sutra-dirgha / sutra-nishedha). The run's own shape comes from one live recorded run (round-robin: @k once per
# segment of thread k); the log is then walked against it. Prints the refusal and exits 1; returns when the walk finds no misfit.
thr_judge() {
    local live="$work/live.log" cnt
    cnt=$(od -An -v -tu1 -j $(( $(LC_ALL=C grep -boa SASTHRDS "$img" | head -1 | cut -d: -f1) + 8 )) -N8 "$img" | awk '{ for (i = 1; i <= NF; i++) v += $i * 256 ^ (i - 1) } END { printf "%.0f", v }')
    rm -f "$live"
    YANTRA_RECORD_EVENTS="$live" "$nat" "$@" > /dev/null 2>&1 < /dev/null; [ -s "$live" ] || return 0
    awk -v T="$cnt" -v LIVE="$live" '
    function recs(f, arr,   n, t) { n = 0
        while ((getline t < f) > 0) { sub(/^[ \t\r]+/, "", t); sub(/[ \t\r]+$/, "", t); if (t == "" || t ~ /^#/) continue; arr[n++] = (t ~ /^@/) ? "@" (substr(t, 2) + 0) : "v"; }
        close(f); return n }
    function refuse(i, why) { printf "events: REFUSED — record %d does not fit the run — record %d %s\n", i, i, why > "/dev/stderr"; exit 1 }
    function short(i, last, N) { printf "events: REFUSED — the log is SHORTER than the run: record %d is due at the decision after %s and the log holds %d — nothing was resumed, rather than run a thread nobody scheduled or hand it a value nobody delivered\n", i, (last < 0 ? "the start" : "thread " last), N > "/dev/stderr"; exit 1 }
    BEGIN {
        L = recs(LIVE, lv); for (i = 0; i < L; i++) if (lv[i] ~ /^@/) seg[substr(lv[i], 2) + 0]++
        N = recs(ARGV[1], rec); i = 0; last = -1
        for (k = 0; k < T; k++) { if (!seg[k]) seg[k] = 1; st[k] = 0; run[k] = 0 }
        for (;;) {
            alive = 0; for (k = 0; k < T; k++) if (st[k] != 2) alive = 1
            if (!alive) {
                if (i < N) { printf "events: REFUSED — the log is LONGER than the run: every thread ended after %d of %d records, %d unconsumed, so this log does not describe this run\n", i, N, N - i > "/dev/stderr"; exit 1 }
                exit 0 }
            if (i >= N) short(i, last, N)
            if (rec[i] == "v") refuse(i, "is the value, and a decision point is due: the next record must be `@N`, the thread to run")
            K = substr(rec[i], 2) + 0
            if (K >= T) refuse(i, "is `@" K "`, and the image declares " T " threads (0 to " T - 1 ")")
            if (st[K] == 2) refuse(i, "is `@" K "`, and thread " K " has ended")
            i++; last = K
            if (st[K] == 1) {
                if (i >= N) short(i, last, N)
                if (rec[i] != "v") refuse(i, "is `" rec[i] "`, and thread " K " is resuming from a wait: its value record is due")
                i++ }
            run[K]++; st[K] = (run[K] >= seg[K]) ? 2 : 1 }
    }' "$ev_log"
    local jrc=$?
    [ "$jrc" = 0 ] || { cat "$work/out"; exit 1; }
}
if [ -n "$ev_log" ]; then
    # THE REPLAY: the records in, the runtime's own log of what it delivered out (yantra-run ignores
    # YANTRA_RECORD_EVENTS beside --events, so a caller's is neither written nor read)
    printf "$ev_fmt" > "$work/records" || yantra "cannot write the replay's records"
    YANTRA_REPLAY_EVENTS="$work/records" YANTRA_RECORD_EVENTS="$work/events.log" "$nat" "$@" > "$work/out" 2> "$work/err"
    rc=$?
    if [ -n "$ev_thr" ] && [ ! -x "$BIN/yantra-run" ] && grep -aq 'a threaded replay whose log does not fit this run' "$work/err"; then thr_judge; fi
    native_only && rerun
    delivered=$(grep -ac '^t=' "$work/events.log" 2>/dev/null) || delivered=0
    halt=$(grep -a '^halt: ' "$work/err" | head -1); halt=${halt#halt: }
    refusal=
    load=
    # A THREADED replay (SASTHRDS) reaches here only when its log fits the run: the runtime exits 2
    # on one that does not, and yantra-run judges it (above). yantra-run's thread lines: the load
    # line, and, when every thread ENDED (no thread halted or overran), "R of R … consumed" after
    # the threads' own end lines and before the end rule's verdict.
    thr=$ev_thr
    thr_end=; [ -n "$thr" ] && ! grep -qa '^threads: thread [0-9]* halted \|^threads: BeyondOwnStack ' "$work/err" && thr_end=1
    while IFS= read -r line; do
        if [ -n "$thr" ]; then
            case $line in
                "native: replay: SASEVENT tag at RAM offset "*)
                    printf 'events: %s thread-log records from "%s", SASEVENT tag at %#x\n' "$ev_n" "$ev_log" "${line##* }" ;;
                "halt: "*|"threads: thread "*" ended with status "*)
                    [ -n "$thr_end" ] && echo "events: $ev_n of $ev_n thread-log records consumed"
                    thr_end=; printf '%s\n' "$line" ;;
                *) printf '%s\n' "$line" ;;
            esac
            continue
        fi
        case $line in
            "native: replay: SASEVENT tag at RAM offset "*)
                printf 'events: %s records from "%s", SASEVENT tag at %#x\n' "$ev_n" "$ev_log" "${line##* }" ;;
            "halt: "*)
                case $halt in
                    "Wait { pc: "*" }")
                        pc=${halt#Wait \{ pc: }; pc=${pc% \}}
                        refusal=$(printf 'the log is SHORTER than the run: wait index %s (the store at pc %#x) has no record; the log holds %s — the program was NOT resumed, rather than let it read a word nobody delivered' "$delivered" "$pc" "$ev_n") ;;
                    *)
                        if [ "$delivered" -lt "$ev_n" ]; then
                            case $halt in
                                Finisher*|Shutdown*) refusal="the log is LONGER than the run: $delivered of $ev_n records delivered, $((ev_n - delivered)) unconsumed; the program halted $halt without waiting again, so this log does not describe this run" ;;
                                *) refusal="the run stopped at $halt with $((ev_n - delivered)) of $ev_n records unconsumed; the log was not exhausted because the program did not finish" ;;
                            esac
                        else
                            echo "events: $delivered of $ev_n delivered, one per wait"
                        fi ;;
                esac
                printf '%s\n' "$line" ;;
            *) printf '%s\n' "$line" ;;
        esac
    done < "$work/err" > "$work/err2"
    cat "$work/out"
    cat "$work/err2" >&2
    if [ -n "$refusal" ]; then echo "events: REFUSED — $refusal" >&2; exit 1; fi
    [ "$rc" -eq 2 ] && rc=1
    exit "$rc"
fi
if [ -n "$net" ]; then
    sent_sha() { if command -v sha256sum >/dev/null; then sha256sum; else shasum -a 256; fi | cut -d' ' -f1; }
    # stderr: held until the peer is up, then streamed; the report's marker becomes the sha256 of
    # every frame sent (u16 BE length + frame: yantra-run's "sent sha256")
    net_filter() {
        local line live=
        while IFS= read -r line || [ -n "$line" ]; do
            printf '%s\n' "$line" >> "$work/err"
            case $line in
                "net: "*" sent sha256 @YANTRA_NET_SENT@")
                    line=${line%@YANTRA_NET_SENT@}$(cat "$work/sent" 2>/dev/null | sent_sha) ;;
            esac
            if [ -z "$live" ]; then
                case $line in
                    "net: peer "*" listening" | "net: peer "*" connected")
                        live=1; : > "$work/live"; [ -f "$work/held" ] && cat "$work/held" >&2 ;;
                    *) printf '%s\n' "$line" >> "$work/held"; continue ;;
                esac
            fi
            printf '%s\n' "$line" >&2
        done
    }
    exec 3>&1
    YANTRA_NET_PEER=$n_peer YANTRA_NET_MAC=${n_mac//:/} YANTRA_NET_FILTER=$n_filter YANTRA_NET_MAX_FRAME=$n_maxf \
        YANTRA_NET_MAX_TX=$n_maxtx YANTRA_NET_MAX_RX=$n_maxrx YANTRA_NET_TIMEOUT=$n_tmo \
        YANTRA_RECORD_EVENTS="$work/events.log" YANTRA_NET_SENT="$work/sent" \
        "$nat" "$@" 2>&1 >&3 3>&- | net_filter
    rc=${PIPESTATUS[0]}
    exec 3>&-
    if [ ! -f "$work/live" ]; then
        native_only && rerun                     # no peer was reached: the rerun is whole
        [ -f "$work/err" ] && cat "$work/err" >&2
    else
        # a rendezvous refusal ran nothing, and yantra-run then leaves no log at all
        grep -a -q '^net: refused — .*(nothing ran)$' "$work/err" ||
            { [ -f "$work/events.log" ] && cp "$work/events.log" "$net_log"; }
        if native_only; then
            say "net: REFUSED — the native run reached what the translator refuses at run time ($(why_native)) after the peer was reached; it is NOT rerun under yantra-run, which would exchange the frames a second time (NET.md)"
            rc=1
        fi
    fi
    [ "$rc" -eq 2 ] && rc=1
    exit "$rc"
fi
# a native recording goes to a temporary and is published only by a run that completes natively:
# a run that faults to yantra-run leaves no partial native log (yantra-run writes its own)
# K1 H12: the run is held (a refusal discards it whole), but a run STOPPED from outside (timeout,
# ^C, a hangup) is not one to rerun: what it had written goes out, as yantra-run's streamed output
# would have, and the exit is the signal's. The native flushes at each newline, so the held file
# has every finished line. A background job's stdin would be /dev/null: it is handed ours (fd 4),
# and since it ignores SIGINT, every signal is passed on as TERM.
release() {
    kill -TERM "$npid" 2>/dev/null
    wait "$npid"
    native_only || { cat "$work/out"; cat "$work/err" >&2; }
    exit $((128 + $1))
}
exec 4<&0
if [ -n "${YANTRA_RECORD_EVENTS:-}" ]; then
    YANTRA_RECORD_EVENTS="$work/events.log" "$nat" "$@" <&4 > "$work/out" 2> "$work/err" &
else
    "$nat" "$@" <&4 > "$work/out" 2> "$work/err" &
fi
npid=$!
trap 'release 15' TERM
trap 'release 2' INT
trap 'release 1' HUP
wait "$npid"
rc=$?
trap - TERM INT HUP
exec 4<&-
# a native-only halt reruns under yantra-run (native_only, above)
native_only && rerun
[ -n "${YANTRA_RECORD_EVENTS:-}" ] && [ -f "$work/events.log" ] && cp "$work/events.log" "$YANTRA_RECORD_EVENTS"
cat "$work/out"
[ -z "$vdline" ] || printf '%s\n' "$vdline" >&2
cat "$work/err" >&2
if [ -n "$vdsh" ]; then
    # non-Linux native verdict file: the halt line of the native run (no step count exists here, so no steps line)
    hl=$(sed -n 's/^halt: //p' "$work/err" | head -1)
    case $hl in
        "Finisher { value: "*) v=${hl#Finisher \{ value: }; v=${v%%,*}
            s=$(printf '%s' "$hl" | sed -n 's/.*status: Some(\([0-9]*\)).*/\1/p'); [ -n "$s" ] || s=-
            vdtxt="finisher $v $s" ;;
        Wait*) vdtxt=wait ;;
        StepLimit*) vdtxt=steplimit ;;
        SpinForever*) vdtxt=spin ;;
        *) vdtxt="fault $hl" ;;
    esac
    printf '%s\n' "$vdtxt" >"$YANTRA_VERDICT" || { say "verdict: $YANTRA_VERDICT: cannot write"; exit 1; }
fi
[ -z "$attest" ] || { if [ -s "$work/attest" ]; then cat "$work/attest" >&2; else echo "fj_attest v1 absent (no fork-join runtime in this run)" >&2; fi; }
[ "$rc" -eq 2 ] && rc=1
exit "$rc"
