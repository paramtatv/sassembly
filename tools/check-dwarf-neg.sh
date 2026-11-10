#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# Negative control for check-dwarf.sh: the guard passes on the real image and
# the check FAILS (rc 1, not 77, not 0) on a mutated one. Exit 0 = both hold.
set -u
cd "$(dirname "$0")/.."
bash tools/check-dwarf.sh >/dev/null; g=$?
[ "$g" = 77 ] && { echo "CANNOT RUN: check-dwarf.sh could not run"; exit 77; }
[ "$g" = 0 ] || { echo "FAIL guard: unmutated check-dwarf rc=$g"; exit 1; }
DWARF_MUTATE=1 bash tools/check-dwarf.sh >/dev/null 2>&1; m=$?
[ "$m" = 1 ] || { echo "FAIL negative control: mutated image gave rc=$m, want 1"; exit 1; }
echo "dwarf-neg: guard rc=0, mutated rc=1"
