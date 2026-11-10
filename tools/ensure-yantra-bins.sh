#!/bin/sh
# Sourced by the yantra-era native checks: `. "$root/tools/ensure-yantra-bins.sh"; ensure_bins DIR [t1_image]` makes
# DIR/yantra-run present (DIR = <target>/release) so a fresh job tree (ev rotation, run-best) never exits 77 for
# "not built" (EVFX 11-03). The yantra crates left the tree (FIN2RED 11-04: the old in-tree cargo build hit the missing
# crates/sadhana-t1/Cargo.toml, rc77 on every host), so yantra-run is $SC_YANTRA (explicit override) or the L28 interpreter
# (tools/yantra-oracle.sh: yi.elf behind the yantra-run CLI), written into DIR. No cargo, no ~/.sansos-bin. t1_image is checked only when asked for.
# Fails closed: nothing found is exit 77 with the reason named, never a pass.
ensure_bins() { # DIR [t1_image]
  _d=$1
  if [ ! -x "$_d/yantra-run" ]; then
    if [ -n "${SC_YANTRA:-}" ] && [ -x "$SC_YANTRA" ]; then mkdir -p "$_d" && ln -sf "$SC_YANTRA" "$_d/yantra-run" || { echo "SKIPPED: cannot link yantra-run into $_d"; exit 77; }
    else sh "$root/tools/yantra-oracle.sh" "$_d" >/dev/null || { echo "SKIPPED: no L28 oracle (tools/yantra-oracle.sh failed)"; exit 77; }; fi
  fi
  [ -x "$_d/yantra-run" ] || { echo "SKIPPED: $_d/yantra-run missing"; exit 77; }
  [ "${2:-}" = t1_image ] && { [ -x "$_d/t1_image" ] || { echo "SKIPPED: $_d lacks t1_image (retired crate; no build)"; exit 77; }; }
  return 0
}
