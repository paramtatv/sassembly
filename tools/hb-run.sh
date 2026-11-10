#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# tools/hb-run.sh [-p PROGRESS_FILE] CMD [ARGS...] — drop-in for `perl tools/with-timeout.pl SECS CMD...` without the clock:
# runs CMD in its own process group and blocks until it EXITS (event), returning its status. No deadline, no kill-on-slow.
# A signal to this wrapper kills only that group. See tools/lib/hbwatch.sh.
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$here/lib/hbwatch.sh"
args=(); [ "${1:-}" = -p ] && { args=(-p "$2"); shift 2; }
hb_run "${args[@]}" -- "$@"
