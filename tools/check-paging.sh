#!/bin/sh
# Paging is on and the machine is still running — task `C-001c2`, doc 11 §7.1.3.
#
# ## What surviving proves
#
# `spec/paging.sas` builds an Sv39 root table of 512 gigapage entries, each
# mapping a virtual gigabyte onto the identical physical one, then writes
# `satp` with MODE=8 and that table's page number and fences the old
# translations away. The instruction after the `csrrw` is fetched THROUGH the
# table it just installed, so a wrong root PPN, a missing V bit or a page
# whose A/D bits the hardware refuses to fill faults immediately: the machine
# stops and nothing more is printed.
#
# That is why the program prints its table's address BEFORE turning
# translation on and the other two numbers after. One line means paging killed
# it; three mean it is running with every fetch, load and store translated.
#
# ## Why the numbers are derived from the first one
#
# `satp` holds a page NUMBER, not an address, so `satp` is only right relative
# to where the table was actually placed — and where it lands is decided by the
# linker, not by this file. Checking it against a constant would pass a program
# that installed a table somewhere else entirely, and would fail the moment
# anything ahead of `ॱरिक्त` in the image changed size.
#
# So the table address is read from the program's own first line, checked to be
# page-aligned and to lie inside the `.bss` the ELF reserves (`readelf`, an
# oracle that reads the file rather than trusting the program), and the other
# two expectations are computed from it:
#
#   * `satp`  must be (8 << 60) | (table >> 12);
#   * the third line is the table's own covering entry, read back at an address
#     that is now virtual: ((table >> 30) << 28) | 0xcf.
#
# The second machine is the same program linked at a different address. The
# table moves with it, so every one of these numbers must move too — which is
# what tells a reading from a stored answer. Measured 2026-08-17: the table at
# 0x80201000 and 0x80401000, satp 0x8000000000080201 and 0x8000000000080401.
#
# The flag byte 0xcf is V|R|W|X|A|D and is the one authored number here: it
# comes from the privileged specification's table, not from any oracle we can
# ask. A/D are set by hand because a hart is permitted to fault rather than
# fill them, and that fault would land on the very first fetch after `satp`.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }
command -v riscv64-elf-readelf >/dev/null 2>&1 || {
  echo "SKIPPED: riscv64-elf-readelf is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

fail=0
prev_table=""

# `<devanagari load address> <the same in hex>`. Both are written out because
# the assembler takes one and the report reads the other; neither is derived
# from the program.
for machine in "०षोड्८०२००००० 0x80200000" "०षोड्८०४००००० 0x80400000"; do
  set -- $machine
  at=$1
  at_hex=$2

  cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
    --स्थान "$at" "$root/spec/paging.sas" "$tmp/paging.elf" >/dev/null

  # Where the image reserves its buffer, read out of the file by a tool that
  # has never run our program.
  # The section is found by NAME, not by index: `kosha` emits only the
  # sections a program uses (`B-071`), so `.bss` is not always the same number.
  # `[ 2]` is two awk fields on one machine and one on another, which is why
  # the columns are counted from the name rather than from the left.
  bss=$(riscv64-elf-readelf -S -W "$tmp/paging.elf" \
        | awk '{ for (i = 1; i <= NF; i++) if ($i == ".bss") print $(i+2), $(i+4) }')
  bss_addr=${bss%% *}
  bss_size=${bss##* }

  # The run is BOUNDED, and that is not caution — it is the failure this check
  # exists to see. A program that faults with no trap handler installed does
  # not exit: the hart takes the fault, takes it again on the handler, and the
  # machine sits there. Measured 2026-08-17 by clearing the V bit in the entry
  # (207 to 206): QEMU printed the first line and then ran for ever. Piped to
  # `tail`, an unbounded run is a check that never returns, which in an
  # unattended loop is worse than a red one. `timeout` is not on macOS, so the
  # wait is spelled out.
  qemu-system-riscv64 -machine virt -smp 1 -m 256M -nographic \
    -bios default -kernel "$tmp/paging.elf" </dev/null >"$tmp/out" 2>&1 &
  qpid=$!
  waited=0
  while kill -0 "$qpid" 2>/dev/null && [ "$waited" -lt 30 ]; do
    sleep 1
    waited=$((waited + 1))
  done
  if kill -0 "$qpid" 2>/dev/null; then
    kill "$qpid" 2>/dev/null || true
    echo "  FAIL  $at_hex did not shut down within ${waited}s — the hart is faulting, not running"
  fi
  wait "$qpid" 2>/dev/null || true

  # The console is a serial line, so every line arrives with a carriage return
  # on it. Stripping that here rather than at each reading keeps the count and
  # the three numbers reading the same text.
  out=$(tail -3 "$tmp/out" | tr -d '\r')
  lines=$(echo "$out" | grep -c '^[0-9a-f]\{16\}$' || true)
  table=$(echo "$out" | sed -n 1p | tr -d ' \r')
  satp=$(echo "$out" | sed -n 2p | tr -d ' \r')
  entry=$(echo "$out" | sed -n 3p | tr -d ' \r')

  if [ "$lines" -ne 3 ]; then
    echo "  FAIL  $at_hex printed $lines of 3 lines — the machine stopped when translation came on"
    echo "$out" | sed 's/^/        /'
    fail=1
    continue
  fi

  want_satp=$(python3 -c "print('%016x' % ((8 << 60) | (int('$table',16) >> 12)))")
  want_entry=$(python3 -c "print('%016x' % (((int('$table',16) >> 30) << 28) | 0xcf))")
  aligned=$(python3 -c "print(int('$table',16) % 4096)")
  inside=$(python3 -c "t=int('$table',16); b=int('$bss_addr',16); print(int(b <= t and t + 4096 <= b + int('$bss_size',16)))")

  if [ "$aligned" != "0" ]; then
    echo "  FAIL  the table is at $table, which is not page-aligned — satp keeps a page number and the low bits are lost"
    fail=1
  fi
  if [ "$inside" != "1" ]; then
    echo "  FAIL  the table is at $table, outside the .bss the ELF reserves ($bss_addr + $bss_size)"
    fail=1
  fi
  if [ "$satp" != "$want_satp" ]; then
    echo "  FAIL  satp is $satp, not $want_satp — MODE=8 with the page number of $table"
    fail=1
  fi
  if [ "$entry" != "$want_entry" ]; then
    echo "  FAIL  the covering entry reads $entry, not $want_entry — the load did not come back through the table"
    fail=1
  fi

  printf "  at=%s  bss=%s+%s  table=%s  satp=%s  entry=%s\n" \
    "$at_hex" "$bss_addr" "$bss_size" "$table" "$satp" "$entry"

  # Linked somewhere else, the table has to be somewhere else. A stored answer
  # survives every check above and not this one.
  if [ -n "$prev_table" ] && [ "$table" = "$prev_table" ]; then
    echo "  FAIL  the table is at $table at both link addresses — that is a constant, not where it was placed"
    fail=1
  fi
  prev_table=$table
done

[ "$fail" -eq 0 ] || { echo; echo "paging is not on."; exit 1; }
echo
echo "ok  Sv39 is on: 512 identity gigapages, satp holds MODE=8 and the root"
echo "    table's own page number, the fetch after the write is translated,"
echo "    and a load through the new mapping reads the entry that covers the"
echo "    table itself — at two link addresses, so every number moved with it"
