#!/usr/bin/env bash
# run as `sh tools/...` (dash) too: this file needs bash — re-exec under it
[ -n "${BASH_VERSION:-}" ] || exec bash "$0" "$@"
# W-302 — THE SELF-HOSTED IMAGE ASSEMBLES EACH spec/*.sas TO THE LOADED IMAGE Rust `sadhana` WROTE. No cargo.
#
# Twin of the retired crates/sadhana-t1/tests/w302_loaded_identity census. The oracle (Rust `sadhana`,
# IDENTITY RULED (a) 2026-10-03) is FROZEN in tools/check-w302-native.tsv: per program and load address, the
# sha256 of Rust's image restricted to ELF-header octets 0..40 and 48..58, every program header and every
# PT_LOAD's bytes (tools/check-w302-native.pl; the section table is excluded by the ruling). This script runs the
# native chain (tools/sadhana-native.sh: the t1-built wrapper image under tools/t1-run.sh) on every row, three ways
# (image from source; object per source via --वस्तु then --संयोजय; the two-object LINK), and compares projections.
# A row whose golden is REFUSED must be refused by the native side too (namaste-main alone). A MISSING output is
# a failure, never a match.
#
# NEGATIVE CONTROL (always run, first): one projected octet of the first golden is flipped; the comparison must
# say DIFFER, and a not-an-ELF file must be REFUSED. If either passes, the check is vacuous and fails.
# Regenerate the table (needs a Rust sadhana, W302_SADHANA=<bin>): tools/check-w302-native.sh --regen
# Env: W302_ONLY=<regex> restricts the rows run (smoke), W302_NATIVE_MODES="src obj" (default both),
# W302_JOBS=<n> parallel runs (default: core count).
# Exits 0 all identical, 1 any failure, 77 cannot run.
(eval ': <(:)') 2>/dev/null || exec /usr/bin/env bash "$0" "$@"
set -uo pipefail
cd "$(dirname "$0")/.."
export LC_ALL=C.UTF-8 2>/dev/null || true
GOLD=tools/check-w302-native.tsv
PROJ="perl tools/check-w302-native.pl"
command -v perl >/dev/null 2>&1 || { echo "CANNOT RUN: perl"; exit 77; }
WORK=$(mktemp -d "${TMPDIR:-/tmp}/w302-native.XXXXXX") || exit 77
trap 'rm -rf "$WORK"' EXIT

if [ "${1:-}" = "--regen" ]; then
  [ -n "${W302_SADHANA:-}" ] && [ -x "$W302_SADHANA" ] || { echo "FAIL: --regen needs W302_SADHANA=<rust sadhana>"; exit 1; }
  {
    printf '# W-302 frozen oracle: sha256 of Rust sadhana image under ruling (a) (tools/check-w302-native.pl). program(+program)\taddr(- = default)\tsha256|REFUSED\n'
    while IFS=$'\t' read -r name addr; do
      a=(); [ "$addr" != "-" ] && a=(--स्थान "$addr")
      rm -f "$WORK/r.elf"; srcs=(); IFS=+ read -ra ps <<<"$name"; for p in "${ps[@]}"; do srcs+=("spec/$p.sas"); done
      "$W302_SADHANA" ${a[@]+"${a[@]}"} "${srcs[@]}" "$WORK/r.elf" >/dev/null 2>&1
      if [ -s "$WORK/r.elf" ]; then h=$($PROJ "$WORK/r.elf") || h=REFUSED; else h=REFUSED; fi
      printf '%s\t%s\t%s\n' "$name" "$addr" "$h"
    done < <(
      D=०षोड्८०२०००००; E=०षोड्८०४०००००
      for p in bare-metal jump-table lib-mudraka namaste namaste-main; do printf '%s\t-\n' "$p"; done
      printf 'namaste-main+lib-mudraka\t-\n'
      printf 'atithi\t-\natithi\t०षोड्२०००००००\n'
      for p in boot-sbi fdt-header virtio-blk virtio-console virtio-net; do printf '%s\t%s\n' "$p" "$D"; done
      for p in arena boot-counter bootloader buddy capability compositor context crossing freelist higher-half irq journal kernel mapping merge milestone-k1 milestone-k2 network notification pager-handoff pager paging powerbox pty root-task schedule second-space split stress-ram stress timer trap-seam trap untyped user-mode virtio-gpu virtio-sound; do printf '%s\t%s\n%s\t%s\n' "$p" "$D" "$p" "$E"; done
    )
  } > "$GOLD"
  echo "wrote $GOLD ($(grep -a -vc '^#' "$GOLD") rows)"; exit 0
fi

[ -s "$GOLD" ] || { echo "FAIL: $GOLD missing"; exit 1; }
# Probe: can the native side run at all? 77 => could not run.
./tools/sadhana-native.sh spec/namaste.sas "$WORK/probe.elf" >"$WORK/probe.log" 2>&1; prc=$?
[ "$prc" -eq 77 ] && { echo "CANNOT RUN: no Stage 1 / wrapper image for sadhana-native (exit 77)"; exit 77; }
[ "$prc" -eq 0 ] || { echo "FAIL: sadhana-native probe exit $prc"; tail -5 "$WORK/probe.log"; exit 1; }

fail=0 n=0
# verdict GOLDEN IMAGE -> prints ok|DIFFER|REFUSED-both|MISSING
verdict() {
  local g=$1 img=$2 h
  if [ ! -s "$img" ]; then [ "$g" = REFUSED ] && echo REFUSED-both || echo MISSING; return; fi
  if ! h=$($PROJ "$img"); then h=REFUSED; fi
  if [ "$g" = REFUSED ]; then echo "DIFFER(golden refused, native produced)"; return; fi
  [ "$h" = "$g" ] && echo ok || { [ "$h" = REFUSED ] && echo "REFUSED(native)" || echo DIFFER; }
}

# --- negative control
g1=$(awk -F'\t' '!/^#/ && $3!="REFUSED"{print $3; exit}' "$GOLD")
bad="${g1%?}$([ "${g1: -1}" = 0 ] && echo 1 || echo 0)"
printf 'not an elf' > "$WORK/junk.elf"
if [ "$(verdict "$bad" "$WORK/junk.elf")" = ok ] || [ "$($PROJ "$WORK/junk.elf" >/dev/null 2>&1; echo $?)" != 2 ]; then
  echo "FAIL: negative control is vacuous (junk image accepted)"; exit 1; fi
./tools/sadhana-native.sh spec/namaste.sas "$WORK/nc.elf" >/dev/null 2>&1
if [ "$(verdict "$bad" "$WORK/nc.elf")" != DIFFER ]; then echo "FAIL: negative control is vacuous (mutated golden matched)"; exit 1; fi
echo "negative control: mutated golden -> DIFFER, junk image -> REFUSED (as required)"

modes=${W302_NATIVE_MODES:-"src obj"}
# one N NAME ADDR GOLD MODE -> writes "ok|FAIL  NAME @ ADDR [MODE]: VERDICT" to $WORK/v.N (runs are independent;
# they fan out W302_JOBS wide, default the core count, so the census fits a 300 s budget — L-C6R1-SPLIT car-alone).
one() {
  local n=$1 name=$2 addr=$3 gold=$4 m=$5 out="$WORK/o$1.elf" a=() srcs=() ps=() objs=() k=0 okobj=1 s v
  [ "$addr" != "-" ] && a=(--स्थान "$addr")
  IFS=+ read -ra ps <<<"$name"; for s in "${ps[@]}"; do srcs+=("spec/$s.sas"); done
  if [ "$m" = src ]; then
    ./tools/sadhana-native.sh ${a[@]+"${a[@]}"} "${srcs[@]}" "$out" >/dev/null 2>&1
  else
    for s in "${srcs[@]}"; do k=$((k+1)); ./tools/sadhana-native.sh --वस्तु "$s" "$WORK/o$n.$k.o" >/dev/null 2>&1 || okobj=0; objs+=("$WORK/o$n.$k.o"); done
    [ $okobj = 1 ] && ./tools/sadhana-native.sh ${a[@]+"${a[@]}"} --संयोजय "${objs[@]}" "$out" >/dev/null 2>&1
  fi
  v=$(verdict "$gold" "$out")
  case $v in ok|REFUSED-both) echo "ok    $name @ $addr [$m]: $v";; *) echo "FAIL  $name @ $addr [$m]: $v";; esac > "$WORK/v.$n"
}
export -f one verdict; export WORK PROJ
J=${W302_JOBS:-$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)}
while IFS=$'\t' read -r name addr gold; do
  case $name in '#'*|'') continue;; esac
  [ -n "${W302_ONLY:-}" ] && ! [[ $name$'\t'$addr =~ $W302_ONLY ]] && continue
  for m in $modes; do n=$((n+1)); printf '%s\0%s\0%s\0%s\0%s\0' "$n" "$name" "$addr" "$gold" "$m"; done
done < "$GOLD" > "$WORK/jobs"
n=$(tr -cd '\0' < "$WORK/jobs" | wc -c); n=$((n/5))
xargs -0 -n 5 -P "$J" bash -c 'one "$@"' one < "$WORK/jobs"
i=0; while [ $i -lt "$n" ]; do i=$((i+1))
  if [ -s "$WORK/v.$i" ]; then cat "$WORK/v.$i"; grep -a -q '^FAIL' "$WORK/v.$i" && fail=1; else echo "FAIL  run $i: no verdict"; fail=1; fi
done
[ "$n" -gt 0 ] || { echo "FAIL: no rows ran"; exit 1; }
if [ "$fail" -eq 0 ]; then echo "PASS: native image matches Rust's frozen loaded image on all $n runs"; exit 0; fi
echo "FAIL: native loaded-image identity does not hold"; exit 1
