#!/bin/sh
# sadhana-native.sh — `cargo run -p sadhana -- <args>` WITHOUT Rust: the .t1 assembler chain
# (spec/entry/assemble.t1 + crates/sadhana-t1/src/*.t1) built by tools/t1-build.sh and run by
# tools/t1-run.sh, through tools/w302-sadhana-shim.sh (argument forms and refusals are the shim's).
#
#   tools/sadhana-native.sh [--स्थान N] a.sas [b.sas ...] out.elf
#   tools/sadhana-native.sh --वस्तु in.sas out.o | --संयोजय a.o [b.o ...] out.elf
#
# The wrapper image is cached by the sha256 of its sources:
#   $SADHANA_NATIVE_CACHE (default ${XDG_CACHE_HOME:-$HOME/.cache}/sadhana-native)/<key>.elf
# env: SADHANA_NATIVE_IMAGE=<elf>  use this prebuilt wrapper image (no build)
#      T1_BUILD_CACHED_ONLY=1      never build Stage 1 / the image; exit 77 if absent
#      W302_YANTRA_RUN=<runner>    replaces the default runner tools/t1-run.sh
# Exit 77 when no wrapper image can be had (a missing Stage 1 is "could not run", not a failure).
# --संक्षिप्त (RVC) is image-from-sources only (assemble.t1 mode ४); with --वस्तु/--संयोजय the shim refuses.
set -u
here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd) || exit 2
root=$(dirname "$here")

if [ -n "${SADHANA_NATIVE_IMAGE:-}" ]; then
    img=$SADHANA_NATIVE_IMAGE
    [ -s "$img" ] || { echo "sadhana-native: SADHANA_NATIVE_IMAGE=$img is not a non-empty file" >&2; exit 77; }
else
    srcs=$(ls "$root"/crates/sadhana-t1/src/*.t1 "$root"/spec/entry/assemble.t1) || exit 2
    sumcmd=sha256sum; command -v sha256sum >/dev/null 2>&1 || sumcmd="shasum -a 256"
    key=$(cat $srcs "$root/tools/w302-sadhana-shim.sh" | $sumcmd | cut -c1-16)
    cache=${SADHANA_NATIVE_CACHE:-${XDG_CACHE_HOME:-$HOME/.cache}/sadhana-native}
    img=$cache/$key.elf
    if [ ! -s "$img" ]; then
        mkdir -p "$cache" || exit 77
        tmpimg=$img.$$
        (cd "$root" && "$here/t1-build.sh" --spec-root spec --compiler crates/sadhana-t1/src \
            --entry पाठसेतुः मुख्यम् -o "$tmpimg" \
            crates/sadhana-t1/src/*.t1 spec/entry/assemble.t1 >&2)
        rc=$?
        if [ "$rc" -ne 0 ] || [ ! -s "$tmpimg" ]; then
            rm -f "$tmpimg"
            echo "sadhana-native: no wrapper image (t1-build exit $rc)" >&2
            [ "$rc" -eq 77 ] && exit 77
            exit 1
        fi
        mv "$tmpimg" "$img" || exit 1
    fi
fi

W302_NATIVE=$img
W302_YANTRA_RUN=${W302_YANTRA_RUN:-$here/t1-run.sh}
export W302_NATIVE W302_YANTRA_RUN
exec sh "$here/w302-sadhana-shim.sh" "$@"
