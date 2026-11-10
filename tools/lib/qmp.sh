#!/bin/sh
# QMP client in POSIX sh + nc -U (replaces the python shot.py / qmp.py heredocs; N2).
# . tools/lib/qmp.sh ; qmp_open SOCK DIR || fail ; qmp_cmd JSON -> reply line in $QMP_REPLY, rc 0 on "return", 1 on "error"/EOF ; qmp_close
# No sleeps: the socket wait is a bounded count of /bin/test -S (a step budget), not a timer.
# Events ("event" lines) are skipped until a "return" or "error" line, as the python clients did.
qmp_open() {  # $1 = socket, $2 = scratch dir (must exist, ours)
    _qmp_dir=$2; _qmp_in=$2/qmp.in; _qmp_out=$2/qmp.out
    rm -f -- "$_qmp_in" "$_qmp_out"; mkfifo "$_qmp_in" "$_qmp_out" || return 77
    _qmp_try=0
    while [ "$_qmp_try" -lt 20000 ]; do
        /bin/test -S "$1" && break
        _qmp_try=$((_qmp_try + 1))
    done
    /bin/test -S "$1" || { echo "could not connect to the QMP socket" >&2; return 1; }
    nc -U "$1" < "$_qmp_in" > "$_qmp_out" 2>/dev/null &
    QMP_PID=$!
    exec 7> "$_qmp_in" 8< "$_qmp_out"
    IFS= read -r QMP_REPLY <&8 || return 1      # the greeting
    qmp_cmd '{"execute":"qmp_capabilities"}'
}
qmp_cmd() {  # $1 = one JSON line
    printf '%s\n' "$1" >&7 || return 1
    while IFS= read -r QMP_REPLY <&8; do
        case $QMP_REPLY in
            *'"error"'*) return 1 ;;
            *'"return"'*) return 0 ;;
        esac
    done
    echo "QMP closed mid-command" >&2; return 1
}
qmp_close() {
    exec 7>&- 8<&-
    [ -n "${QMP_PID:-}" ] && { kill "$QMP_PID" 2>/dev/null; wait "$QMP_PID" 2>/dev/null; }
    QMP_PID=
    return 0
}
# qmp_status -> prints the run state ("running", "paused", ...) from query-status
qmp_status() {
    qmp_cmd '{"execute":"query-status"}' || return 1
    printf '%s\n' "$QMP_REPLY" | sed -n 's/.*"status": *"\([^"]*\)".*/\1/p'
}
# qmp_shot SOCK DIR OUT [FORMAT] : connect, negotiate, screendump, leave. rc 0 ok; message on stderr otherwise.
qmp_shot() {
    qmp_open "$1" "$2" || { qmp_close; echo "QMP negotiation failed" >&2; return 1; }
    if [ -n "${4:-}" ]; then
        qmp_cmd "{\"execute\":\"screendump\",\"arguments\":{\"filename\":\"$3\",\"format\":\"$4\"}}"
    else
        qmp_cmd "{\"execute\":\"screendump\",\"arguments\":{\"filename\":\"$3\"}}"
    fi
    _qs=$?
    [ "$_qs" -eq 0 ] || echo "QMP error: $QMP_REPLY" >&2
    qmp_close
    return "$_qs"
}
# qmp_pause SOCK DIR OUT : for a machine started -action shutdown=pause. Prints "legacy X", "status Y", "dump ok|<error>".
# No poll: when the machine is still running the next lines are read blocking until the STOP event arrives.
qmp_pause() {
    qmp_open "$1" "$2" || { qmp_close; return 1; }
    qmp_cmd '{"execute":"human-monitor-command","arguments":{"command-line":"info qtree"}}'
    printf '%s\n' "$QMP_REPLY" | sed 's/\\n/\
/g' | awk '
        { sub(/^[ \t]+/, ""); t = $0 }
        t ~ /^dev: virtio-mmio/ { nb = 0 }
        { blk[nb++] = t }
        t ~ /^dev: virtio-gpu-device/ { for (i = 0; i < nb; i++) if (blk[i] ~ /^force-legacy/) { v = blk[i]; sub(/^.*= /, "", v); legacy = v; break } }
        END { print "legacy", (legacy == "" ? "unknown" : legacy) }'
    _st=$(qmp_status)
    while [ "$_st" = running ]; do
        IFS= read -r _ev <&8 || break
        case $_ev in *'"STOP"'*|*'"SHUTDOWN"'*) _st=$(qmp_status) ;; esac
    done
    echo "status ${_st:-?}"
    if qmp_cmd "{\"execute\":\"screendump\",\"arguments\":{\"filename\":\"$3\",\"format\":\"ppm\"}}"; then
        echo "dump ok"
    else
        echo "dump $(printf '%s' "$QMP_REPLY" | sed -n 's/.*"error": *\(.*\)}$/\1/p')"
    fi
    qmp_cmd '{"execute":"quit"}'
    qmp_close
}
