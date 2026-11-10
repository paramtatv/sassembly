#!/bin/sh
# tools/yantra-oracle.sh DIR -> writes DIR/yantra-run, a drop-in for the retired Rust yantra-run that runs the L28 T1
# interpreter (tools/t1-interp: yi.elf built by mk.sh + t1-build.sh, cached by the sha256 of its authoring sources) and
# prints that path. The shim speaks ELF [args] and --files DIR; the host-mode flags (--events, --net-*, --listen, --smp)
# are CAR-H's rows. Never writes into the repo. Exit 0 path on stdout; 77 could not run (reason on stderr).
root=$(cd "$(dirname "$0")/.." && pwd)
d=${1:-}; [ -n "$d" ] || { echo "usage: yantra-oracle.sh DIR" >&2; exit 2; }
ti=$root/tools/t1-interp
sum() { if command -v sha256sum >/dev/null 2>&1; then sha256sum; else shasum -a 256; fi; }
k=$( { echo FP=1; cat "$ti"/authoring/* "$ti/mk.sh" "$ti/mkmods.sh" "$ti/dn.sh" 2>/dev/null; } | sum | cut -c1-16 ) || exit 77
c=${YANTRA_ORACLE_CACHE:-$HOME/.cache/yantra-oracle}/yi-$k
if [ ! -s "$c/yi.elf" ]; then
  mkdir -p "$c.tmp.$$" || { echo "yantra-oracle: cannot make $c.tmp.$$" >&2; exit 77; }
  FP=1 sh "$ti/mk.sh" "$c.tmp.$$/mods" >&2 || { echo "yantra-oracle: mk.sh failed" >&2; exit 77; }
  # shellcheck disable=SC2046
  ( cd "$root" && sh tools/t1-build.sh --entry यन्त्रकेन्द्र मुख्यम् -o "$c.tmp.$$/yi.elf" $(cat "$c.tmp.$$/mods/MODULES") ) >&2 \
    || { echo "yantra-oracle: t1-build of yi.elf failed" >&2; exit 77; }
  mv "$c.tmp.$$" "$c" 2>/dev/null || [ -s "$c/yi.elf" ] || exit 77   # a peer won the race: use its copy
  [ -d "$c.tmp.$$" ] && rm -rf -- "$c.tmp.$$"
fi
[ -s "$c/yi.elf" ] || { echo "yantra-oracle: no yi.elf" >&2; exit 77; }
mkdir -p "$d" || exit 77
{ printf '#!/bin/sh\n# L28 oracle wrapper (tools/yantra-oracle.sh); yi %s\n' "$k"
  printf 'YI_ELF=%s\nexport YI_ELF\nexec sh %s/yantra-run "$@"\n' "$c/yi.elf" "$ti"; } > "$d/yantra-run.$$" && chmod +x "$d/yantra-run.$$" \
  && mv -f "$d/yantra-run.$$" "$d/yantra-run" || exit 77
echo "$d/yantra-run"
