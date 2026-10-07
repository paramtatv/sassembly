#!/bin/sh
# The device tree OpenSBI hands us is real, and we read it big-endian — tasks
# `C-001b1`, `C-001b2a`, `C-001b2b1`, `C-001b2b2` and `C-001b2b3`,
# doc 11 §7.1.2.
#
# ## Why a single number would prove nothing
#
# `spec/fdt-header.sas` prints eight lines. The first four are the FDT magic,
# its total size in hex, the same total size in decimal (`दशमलवम्`, C-001b2a —
# RAM sizes are read by people, and 0x40000000 is not a quantity anyone weighs),
# and the number of nodes in the structure block (`सञ्चरणम्`, C-001b2b1). The
# magic is self-checking — `d00dfeed` read the right way round, `edfe0dd0` read
# the wrong way — and that alone justifies the four separate byte loads in
# `वाचनम्`, because RISC-V is little-endian and the FDT is not.
#
# The size is the harder claim, and a program that printed a constant would
# pass any check that looked at one boot. QEMU can dump the tree it builds
# (`-machine virt,dumpdtb=…`), but that dump is NOT what the payload sees:
# OpenSBI adds its own reserved-memory nodes before handing the tree on, so the
# two numbers legitimately differ and comparing them directly would fail on a
# correct program.
#
# So the check is on the RELATIONSHIP, across two machines that must produce
# different trees:
#
#   * with `-smp 1` and `-smp 4`, QEMU's own dump changes (more CPU nodes);
#   * our reading must change too — a constant would not;
#   * and `ours - dump` must be the SAME on both, because OpenSBI's additions
#     do not depend on the CPU count.
#
# The decimal line is checked against the hex one rather than against a
# constant, for the same reason: the hex reading already has to track the
# machine, so tying the two together makes `दशमलवम्` answer to a number that
# changes with `-smp`. A decimal routine checked against a literal would pass
# while printing a remembered string.
#
# The node count is checked the same way and for the same reason. Its oracle is
# a second walk of the dump, written in Python from the same five tokens — a
# different language reading a different tree, so agreeing on the DIFFERENCE is
# evidence and not a tautology. It has to be the relationship again: OpenSBI
# adds nodes of its own before handing the tree on, so our count is legitimately
# higher than the dump's on every machine.
#
# Measured 2026-08-17: dump 0x13a4/0x1fbc, ours 0x17c4/0x23dc, delta 0x420 both
# times; nodes 31/40 in the dump against 34/43 read on the machine, delta 3 both
# times. Any of these going wrong is a different defect, and each is named
# separately below rather than folded into one pass/fail.
#
# Two mutations of the walk were made and both were caught: skipping a property
# by 8 bytes instead of 12, and dropping the four-byte alignment after a node
# name. Each derails the walk onto an unrecognised token, which stops it — and
# the count printed was 1 on both machines rather than 34 and 43.
#
# ## The name it found (C-001b2b2)
#
# The fifth and sixth lines are the `/memory` node's full name and which node it was,
# counting from one. Its oracle is the same second walk, asked the same
# question of the dump (`fdt-nodes.py --memory`), and the two are held to the
# relationship rule again: the names must be equal, while the ordinals differ
# by OpenSBI's insertions and that difference must not move.
#
# The name is where a stored answer would show, and it cannot be one here: the
# key in `spec/fdt-header.sas` is seven octets — `memory` and a terminator,
# bound to `b"memory"` by `crates/sadhana/tests/fdt_key.rs` — and the whole
# data section of the image is those seven bytes, while the name printed is
# fifteen characters long. Measured by mutation, 2026-08-17: changing one octet
# of the key to spell `lemory` printed `?` and `0`, and changing it to `cpus`
# printed `cpus` at ordinal 9 against the dump's 6 — same delta of 3, a name
# read from the machine, and a walk that found a different node because it was
# told to.
#
# What the machine cannot check here is the `@`-or-terminator rule that stops
# `memory-controller` from matching, because this tree has no such sibling.
# That one is asserted in `fdt_key.rs` instead, against the real name.
#
# ## How much RAM there is (C-001b2b3)
#
# The last two lines are RAM's base and size in decimal, read from that node's
# `reg`. This is the one reading here that is checked DIRECTLY rather than by a
# delta, and it can be: OpenSBI adds nodes to the tree but does not rewrite
# `/memory`'s `reg`, so our value and the dump's must be equal, not merely
# parallel. It is also the only number in this file a person can check without
# reading a tree at all — QEMU was told `-m`, and the size must be that.
#
# The two machines differ in `-m` as well as `-smp`, and the second has 5 GiB
# for a specific reason: below 4 GiB the high word of every `reg` cell is zero,
# so a 32-bit read of a 64-bit cell agrees with a correct one. Measured by
# mutation, 2026-08-17: dropping the high half of the size read left the 256 MiB
# machine passing and printed 1073741824 for 5 GiB — the low word alone, a
# plausible size, and wrong by a factor of five.
#
# Two more mutations, both caught on both machines: changing one octet of the
# `reg` key to spell `req`, and reading the strings-block offset from the
# header's fifth word instead of its fourth. Each prints `0` and `0`, which is
# what the program says when it walked the whole tree and matched no property —
# a different defect from matching the wrong one, and it says so.
set -eu

root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT

command -v qemu-system-riscv64 >/dev/null 2>&1 || {
  echo "SKIPPED: qemu-system-riscv64 is not installed"
  echo "         Exiting 77 rather than 0 — a check that cannot run has not passed."
  exit 77; }

# ---- bounding QEMU, and why it is spelled out this way ------------------
#
# `AGENT-BRIEF` requires an explicit kill, never `perl -e 'alarm'` (`W-060`:
# QEMU installs its own SIGALRM handler and outlives it). This check had NO
# bound at all, and the absence was unfalsifiable rather than harmless: every
# uncompressed image shuts itself down, so the missing guard never had to fire.
# The COMPRESSED `fdt-header` image does not shut down. a peer session waited on
# it for 7h25m at 99% CPU. Had `--संक्षिप्त` become the default, the gate would
# not have failed — it would have STOPPED, with no diagnostic and a hang
# indistinguishable from a slow machine.
#
# Three constructs are avoided here deliberately, each for a measured reason:
#
#   NO command substitution around QEMU. `$( )` does not return until every
#   process holding the pipe closes it, so a hung QEMU hangs the shell even
#   after `tail` already has what it needs. That is what the old `out=$(qemu …
#   | tail -8)` did. Output goes to a file and is read afterwards.
#
#   NO `( sleep N; kill … ) &` watchdog. Killing the subshell reaps the
#   subshell and leaves the forked `sleep` orphaned, still holding the write
#   end — so every run took N seconds however fast it really was. A watchdog
#   that silently becomes the floor is a no-op that is not a no-op. The poll
#   loop below forks nothing that outlives an iteration.
#
#   NO `pkill -f qemu-system-riscv64`. This host runs other agents' QEMUs and
#   the gate's own; a pattern kill takes them too, and `W-044`/`W-047`/`W-049`
#   are three rows about pattern-kills matching the killer's own argv. Only the
#   pid this function started is ever signalled.
QEMU_LIMIT=${QEMU_LIMIT:-30}

# run_qemu <outfile> <qemu args…> — 0 if it exited, 1 if it had to be killed.
run_qemu() {
  _out=$1
  shift
  qemu-system-riscv64 "$@" </dev/null >"$_out" 2>&1 &
  _q=$!
  _w=0
  while kill -0 "$_q" 2>/dev/null && [ "$_w" -lt "$QEMU_LIMIT" ]; do
    sleep 1
    _w=$((_w + 1))
  done
  if kill -0 "$_q" 2>/dev/null; then
    kill -9 "$_q" 2>/dev/null || true
    wait "$_q" 2>/dev/null || true
    return 1
  fi
  wait "$_q" 2>/dev/null || true
  return 0
}

cargo run --quiet --manifest-path "$root/Cargo.toml" -p sadhana -- \
  --स्थान ०षोड्८०२००००० "$root/spec/fdt-header.sas" "$tmp/fdt.elf" >/dev/null

magic=""
prev_delta=""
prev_ours=""
prev_node_delta=""
prev_nodes=""
prev_mem_delta=""
prev_base=""
prev_size=""
fail=0

# `<hart count> <MiB of RAM>`. The MiB figure is what QEMU is told; the byte
# count it must produce is computed below rather than written beside it, so
# there is no second number here to get wrong. 5 GiB on the second machine is
# deliberate: below 4 GiB the high word of every reg cell is zero, and a
# 32-bit read of a 64-bit cell would agree with a correct one on every machine.
for machine in "1 256" "4 5120"; do
  set -- $machine
  n=$1
  mib=$2
  want_size=$(python3 -c "print($mib * 1024 * 1024)")

  run_qemu "$tmp/dump$n.log" -machine "virt,dumpdtb=$tmp/d$n.dtb" -smp "$n" \
    -m "${mib}M" -nographic -bios default || {
      echo "  FAIL  TIMEOUT: the device-tree dump for -smp $n was killed after ${QEMU_LIMIT}s"
      echo '        -machine dumpdtb writes the blob and exits, so a run killed here'
      echo '        means QEMU never reached that point, not that the bound is tight.'
      fail=1
      continue; }
  dump=$(python3 -c "import struct,sys;print('%08x'%struct.unpack('>I',open('$tmp/d$n.dtb','rb').read()[4:8])[0])")
  dump_nodes=$(python3 "$root/tools/fdt-nodes.py" "$tmp/d$n.dtb")
  dump_mem=$(python3 "$root/tools/fdt-nodes.py" --memory "$tmp/d$n.dtb")
  dump_mem_at=${dump_mem%% *}
  dump_mem_name=${dump_mem#* }
  dump_reg=$(python3 "$root/tools/fdt-nodes.py" --reg "$tmp/d$n.dtb")
  dump_base=${dump_reg%% *}
  dump_size=${dump_reg#* }

  # THE ONE THAT HUNG. This was `out=$(qemu … | tail -8)`; the substitution
  # cannot return while QEMU holds the pipe, so an image that never shuts down
  # stops the check rather than failing it.
  run_qemu "$tmp/run$n.log" -machine virt -smp "$n" -m "${mib}M" -nographic \
    -bios default -kernel "$tmp/fdt.elf" || {
      echo "  FAIL  TIMEOUT: spec/fdt-header.sas ran past ${QEMU_LIMIT}s at -smp $n and was killed"
      echo '        The program never called the SBI shutdown, so QEMU would have run'
      echo '        for ever. That is a hang, not a slow machine: raising the bound'
      echo '        cannot turn it green.'
      sed 's/^/        /' "$tmp/run$n.log" | tail -8
      fail=1
      continue; }
  out=$(tail -8 "$tmp/run$n.log")
  magic=$(echo "$out" | sed -n 1p | tr -d ' \r')
  ours=$(echo "$out" | sed -n 2p | tr -d ' \r')
  dec=$(echo "$out" | sed -n 3p | tr -d ' \r')
  nodes=$(echo "$out" | sed -n 4p | tr -d ' \r')
  mem_name=$(echo "$out" | sed -n 5p | tr -d ' \r')
  mem_at=$(echo "$out" | sed -n 6p | tr -d ' \r')
  base=$(echo "$out" | sed -n 7p | tr -d ' \r')
  size=$(echo "$out" | sed -n 8p | tr -d ' \r')

  # The magic is the endianness check, and it is the same on every machine.
  if [ "$magic" != "d00dfeed" ]; then
    if [ "$magic" = "edfe0dd0" ]; then
      echo "  FAIL  magic read little-endian ($magic) — वाचनम् is assembling the bytes backwards"
    else
      echo "  FAIL  magic is $magic, not d00dfeed — the FDT pointer in अर्थ१ is not a device tree"
    fi
    fail=1
  fi

  # The same number said twice. Division is the one arithmetic here that can be
  # off by a digit and still look like a plausible size.
  want=$(python3 -c "print(int('$ours',16))" 2>/dev/null || echo "?")
  if [ "$dec" != "$want" ]; then
    echo "  FAIL  decimal is $dec, hex $ours is $want — दशमलवम् and छापनम् disagree about one reading"
    fail=1
  fi

  # The name, against a second walk of the dump. `?` is what the program prints
  # when it reached FDT_END without matching, which is a different defect from
  # matching the wrong node and says so.
  if [ "$mem_name" = "?" ]; then
    echo "  FAIL  no node matched the key — the walk never saw $dump_mem_name, which is in the tree"
    fail=1
  elif [ "$mem_name" != "$dump_mem_name" ]; then
    echo "  FAIL  matched $mem_name, QEMU's tree has $dump_mem_name — the match landed on the wrong node"
    fail=1
  fi

  # RAM itself (C-001b2b3). Unlike the tree's total size, the reg property is
  # not touched by OpenSBI, so these two are compared DIRECTLY rather than by a
  # delta — and the size is also compared with what QEMU was told, which is the
  # one number in this file a person can check without reading a tree at all.
  if [ "$base" != "$dump_base" ]; then
    echo "  FAIL  RAM base is $base, QEMU's tree says $dump_base — reg's address cells are not being read"
    fail=1
  fi
  if [ "$size" != "$dump_size" ]; then
    echo "  FAIL  RAM size is $size, QEMU's tree says $dump_size — reg's size cells are not being read"
    fail=1
  fi
  if [ "$size" != "$want_size" ]; then
    echo "  FAIL  RAM size is $size, QEMU was given -m ${mib}M = $want_size bytes"
    fail=1
  fi

  delta=$(python3 -c "print('%x'%(int('$ours',16)-int('$dump',16)))" 2>/dev/null || echo "?")
  node_delta=$(python3 -c "print($nodes-$dump_nodes)" 2>/dev/null || echo "?")
  mem_delta=$(python3 -c "print($mem_at-$dump_mem_at)" 2>/dev/null || echo "?")
  printf "  smp=%s  mem=%sM  qemu=%s  ours=%s  dec=%s  delta=%s\n" \
    "$n" "$mib" "$dump" "$ours" "$dec" "$delta"
  printf "          nodes qemu=%s  ours=%s  delta=%s\n" "$dump_nodes" "$nodes" "$node_delta"
  printf "          /memory %s at qemu=%s  ours=%s  delta=%s\n" \
    "$mem_name" "$dump_mem_at" "$mem_at" "$mem_delta"
  printf "          RAM base=%s  size=%s  (-m %sM)\n" "$base" "$size" "$mib"

  if [ -n "$prev_delta" ]; then
    # OpenSBI's additions do not depend on the CPU count, so this is constant.
    if [ "$delta" != "$prev_delta" ]; then
      echo "  FAIL  ours-minus-qemu changed ($prev_delta -> $delta); the size is not being read from the tree"
      fail=1
    fi
    # A constant would survive the delta check only if the dump were constant
    # too, which it is not — so this catches a hardcoded number directly.
    if [ "$ours" = "$prev_ours" ]; then
      echo "  FAIL  our reading did not change between -smp 1 and -smp 4 — it is a constant, not a measurement"
      fail=1
    fi
    # The walk, held to exactly the same standard: OpenSBI's extra nodes do not
    # depend on the CPU count either, so the gap to the dump is constant while
    # both counts move.
    if [ "$node_delta" != "$prev_node_delta" ]; then
      echo "  FAIL  nodes-minus-qemu changed ($prev_node_delta -> $node_delta); सञ्चरणम् is not walking the tree it was given"
      fail=1
    fi
    if [ "$nodes" = "$prev_nodes" ]; then
      echo "  FAIL  our node count did not change between -smp 1 and -smp 4 — the walk is not reaching the cpu nodes"
      fail=1
    fi
    # /memory sits ahead of the cpu nodes, so its ordinal is the same on both
    # machines — but only if the walk reaches it the same way on both, and a
    # walk that mis-steps before it moves this without moving the total.
    if [ "$mem_delta" != "$prev_mem_delta" ]; then
      echo "  FAIL  /memory's ordinal moved against QEMU's ($prev_mem_delta -> $mem_delta); the match is counting a different walk"
      fail=1
    fi
    # The base is where RAM starts on this machine and does not depend on how
    # much of it there is; the size is the whole point of asking. One must not
    # move and the other must, which is what tells a reading from a constant.
    if [ "$base" != "$prev_base" ]; then
      echo "  FAIL  RAM base moved ($prev_base -> $base) while only -m changed — the address cells are being read as size"
      fail=1
    fi
    if [ "$size" = "$prev_size" ]; then
      echo "  FAIL  RAM size did not change between -m ${mib}M and the previous machine — it is a constant, not a reading"
      fail=1
    fi
  fi
  prev_delta=$delta
  prev_ours=$ours
  prev_node_delta=$node_delta
  prev_nodes=$nodes
  prev_mem_delta=$mem_delta
  prev_base=$base
  prev_size=$size
done

[ "$fail" -eq 0 ] || { echo; echo "the device tree is not being read."; exit 1; }
echo
echo "ok  the FDT OpenSBI passes is found, its magic reads d00dfeed big-endian,"
echo "    its size tracks the machine (delta to QEMU's own dump is constant),"
echo "    the decimal printing agrees with the hex on both machines, the"
echo "    structure walk counts nodes that track the machine the same way,"
echo "    it finds /memory by name — the same node a second walk of QEMU's"
echo "    own dump finds, at the same place in the tree — and it reads that"
echo "    node's reg through the strings block: RAM's base and size in decimal,"
echo "    agreeing with the dump and with what -m asked for on both machines"
