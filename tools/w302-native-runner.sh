#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# W-302 runner: a yantra-run stand-in that runs the image natively (tools/t1-run.sh).
# Select with W302_RUNNER=native (check-w302-native.sh, check-w302-qemu.sh), or
# hand its path to W302_YANTRA_RUN. Same argv/env contract as yantra-run.
exec "$(dirname "$0")/t1-run.sh" "$@"
