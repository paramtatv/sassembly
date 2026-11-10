# tools/lib/hbwatch.sh — sourced. Event-driven job watch (owner 2026-10-19): NO wall-clock deadline, NO sleep-poll.
#   hb_run [-p PROGRESS_FILE] [-c CALLBACK] -- CMD [ARGS...]
# CMD runs in its OWN process group. The waiter blocks on events, never on a clock:
#   exit event  — `wait $pid` returns when the child ends; CALLBACK "done" RC fires; hb_run returns RC.
#   beat event  — with -p, a `tail -f` child appends one line per progress-file growth to a fifo the waiter drains (blocking read);
#                 CALLBACK "beat" fires per line. If the beat source ends while CMD lives (progress file removed), CALLBACK "nobeat"
#                 fires; nothing is killed: a slow job is a running job, only an exited job or a signal to the waiter ends it.
#   signal      — TERM/INT/HUP to the waiter kills ONLY the recorded pgid (TERM then KILL); returns 128+sig, never a verdict.
# CALLBACK is a function/command name run as: CALLBACK EVENT ARG. The job's own exit status is the verdict, nothing else.
hb_run() {
    local prog="" cb="" pid tpid="" fifo="" rc sig=0
    while [ $# -gt 0 ]; do case $1 in -p) prog=$2; shift 2 ;; -c) cb=$2; shift 2 ;; --) shift; break ;; *) break ;; esac; done
    [ $# -gt 0 ] || { echo "hb_run: no command" >&2; return 2; }
    set -m
    "$@" & pid=$!
    set +m
    trap 'sig=15; kill -TERM -- -"$pid" 2>/dev/null; kill -TERM "$pid" 2>/dev/null' TERM INT HUP
    if [ -n "$prog" ]; then
        fifo=$(mktemp -u "${TMPDIR:-/tmp}/hb.XXXXXX") && mkfifo "$fifo" &&
        { tail -f -n 0 "$prog" > "$fifo" 2>/dev/null & tpid=$!
          exec 8<"$fifo"
          ( while IFS= read -r _ <&8; do [ -n "$cb" ] && "$cb" beat "$prog"; done
            kill -0 "$pid" 2>/dev/null && [ -n "$cb" ] && "$cb" nobeat "$prog" ) & }
    fi
    while :; do
        wait "$pid"; rc=$?
        kill -0 "$pid" 2>/dev/null || break      # a trapped signal interrupts wait; re-wait until the child is really gone
        [ "$sig" = 15 ] && { kill -KILL -- -"$pid" 2>/dev/null; wait "$pid" 2>/dev/null; rc=143; break; }
    done
    [ -n "$tpid" ] && kill "$tpid" 2>/dev/null
    [ -n "$fifo" ] && { exec 8<&-; rm -f "$fifo"; }
    trap - TERM INT HUP
    [ -n "$cb" ] && "$cb" done "$rc"
    return "$rc"
}
