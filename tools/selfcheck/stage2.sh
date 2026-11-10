#!/bin/sh
# STAGE 2 (P9-B host.t1), the one copy shared by selfcheck/run.sh and t1tests/mutants.sh: a case that compiled a SECOND program leaves
# its image in its files root under the fixed name below; run it and append one `ENTRY|<result>` line to the case's stdout (the result =
# the entry's int, relayed as the Finisher status; 0 = pass word 21845). A case without that file is untouched, so every existing digest
# is unchanged.   usage: stage2.sh ROOT OUT ERR2 RUNNER... (RUNNER gets `--files ROOT IMAGE` appended; its env is the caller's)
root=$1 out=$2 err2=$3; shift 3
h2="$root/अतिथेयप्रतिमा"
[ -f "$h2" ] || exit 0
"$@" --files "$root" "$h2" > /dev/null 2> "$err2"
v2=$(sed -n 's/^halt: Finisher { value: \([0-9]*\), status: .*$/\1/p' "$err2")
s2=$(sed -n 's/^halt: Finisher { value: [0-9]*, status: Some(\([0-9]*\)) }$/\1/p' "$err2")
if [ "$v2" = 21845 ]; then r2=0; elif [ -n "$s2" ]; then r2=$s2; else r2="none(${v2:-nohalt})"; fi
printf 'ENTRY|%s\n' "$r2" >> "$out"
