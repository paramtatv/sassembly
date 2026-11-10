#!/bin/sh
# W-302: stand in for `cargo run -p sadhana -- <args>` with the .t1 chain.
#
# tools/check-w302-qemu.sh puts a `cargo` in front of the real one on PATH. That
# `cargo` hands every `run ... -p sadhana -- <args>` here and forwards everything
# else, unchanged, to the real cargo. So each tools/check-*.sh runs AS WRITTEN —
# its own QEMU command, its own success string — on images the SELF-HOSTED .t1
# image wrote (spec/entry/assemble.t1, built with `t1_image --entry`), and no
# Rust assembler is involved in any image it boots.
#
# The sadhana argument forms the 52 QEMU scripts use, and what each becomes:
#
#   [--स्थान N] a.sas [b.sas ...] out.elf
#       the wrapper image under yantra-run: the sources on the input channel
#       (several are joined by NUL — object per source, then one link), N as its
#       argument in ASCII decimal, its stdout the ELF.
#   --वस्तु in.sas out.o
#       the wrapper image in MODE १ (its second argument): the source on the
#       input channel, its stdout a RELOCATABLE ELF object written by the .t1
#       chain (`वस्तुलेखनम्` in spec/entry/assemble.t1) — `.text`, `.rela.text`,
#       `.data`, `.rela.data`, `.bss`, `.symtab`, `.strtab`, `.shstrtab`.
#   --संयोजय a.o [b.o ...] out.elf
#       the wrapper image in MODE २: the object files end to end on the input
#       channel, read back by the .t1 chain (`वस्तुपठनम्`) and linked there.
#       An input that is not an ELF relocatable object is refused here by name.
#   --संक्षिप्त ...
#       the wrapper image in MODE ४: one build, RVC target chosen before it
#       (image from sources only; refused with --वस्तु/--संयोजय).
#
# Anything else is refused by name rather than guessed. Each call is appended to
# $W302_SHIM_LOG as one line, so the driver can show that a PASS was fed.
set -u

log() {
    if [ -n "${W302_SHIM_LOG:-}" ]; then printf '%s\n' "$1" >>"$W302_SHIM_LOG"; fi
}
refuse() {
    log "REFUSED $1"
    printf 'w302-shim: %s\n' "$1" >&2
    exit 1
}

# One UTF-8 character from the front of $1 (BSD awk and GNU awk agree under C).
first_char() {
    LC_ALL=C awk -v s="$1" 'BEGIN {
        for (i = 128; i < 256; i++) hi[sprintf("%c", i)] = i
        b = substr(s, 1, 1); n = 1
        if (b in hi) { v = hi[b]; if (v >= 240) n = 4; else if (v >= 224) n = 3; else if (v >= 192) n = 2 }
        printf "%s", substr(s, 1, n) }'
}

# Python-style repr of a list of strings: ['a', 'b']
repr_list() {
    out=[ sep=
    for x in "$@"; do out="$out$sep'$x'"; sep=", "; done
    printf '%s]' "$out"
}

# numeral TEXT -> NUMERAL: the two spellings the scripts pass,
# ०षोड्<hex digits> and decimal (Devanagari digits only).
numeral() {
    nt=$1
    case $nt in
        "०षोड्"*) nbase=16 nbody=${nt#"०षोड्"} ;;
        *) nbase=10 nbody=$nt ;;
    esac
    [ -n "$nbody" ] || refuse "--स्थान '$nt': no digits"
    nrest=$nbody nval=0
    while [ -n "$nrest" ]; do
        case $nrest in
            "०"*) d=0 nrest=${nrest#"०"} ;;
            "१"*) d=1 nrest=${nrest#"१"} ;;
            "२"*) d=2 nrest=${nrest#"२"} ;;
            "३"*) d=3 nrest=${nrest#"३"} ;;
            "४"*) d=4 nrest=${nrest#"४"} ;;
            "५"*) d=5 nrest=${nrest#"५"} ;;
            "६"*) d=6 nrest=${nrest#"६"} ;;
            "७"*) d=7 nrest=${nrest#"७"} ;;
            "८"*) d=8 nrest=${nrest#"८"} ;;
            "९"*) d=9 nrest=${nrest#"९"} ;;
            *) refuse "--स्थान '$nt': '$(first_char "$nrest")' is not a digit this shim reads" ;;
        esac
        nval=$(( nval * nbase + d ))
    done
    NUMERAL=$nval
}

LOAD_DEFAULT=2147483648

load="" mode=image compressed=0 debug=0
ARGS_REPR=$(repr_list "$@")
n=$#
while [ "$n" -gt 0 ]; do
    a=$1; shift; n=$((n - 1))
    case $a in
        "--स्थान")
            [ "$n" -ge 1 ] || refuse "--स्थान needs a value"
            numeral "$1"; load=$NUMERAL
            shift; n=$((n - 1)) ;;
        "--वस्तु") mode=object ;;
        "--संयोजय") mode=link ;;
        "--संक्षिप्त") compressed=1 ;;
        "-g") debug=1 ;;
        --*) refuse "$a: a sadhana flag this shim does not stand in for" ;;
        *) set -- "$@" "$a" ;;
    esac
done

[ "$#" -ge 2 ] || refuse "want inputs and an output, got $ARGS_REPR"
for out; do :; done
cnt=$(( $# - 1 ))
i=0
while [ "$i" -lt "$cnt" ]; do set -- "$@" "$1"; shift; i=$((i + 1)); done
out=$1; shift                      # now only the inputs remain
if [ "$compressed" = 1 ] && [ "$mode" != image ]; then
    refuse "--संक्षिप्त: image from sources only (SAD-C4d1), not with --वस्तु/--संयोजय"
fi
if [ "$debug" = 1 ] && [ "$mode" != image ]; then
    refuse "-g: DWARF from sources only (image mode), not with --वस्तु/--संयोजय"
fi
if [ "$mode" = object ] && { [ "$#" -ne 1 ] || [ -n "$load" ]; }; then
    refuse "--वस्तु takes one source and no --स्थान, got $ARGS_REPR"
fi

TMP=$(mktemp -d "${TMPDIR:-/tmp}/w302shim.XXXXXX") || exit 1
trap 'rm -rf "$TMP"' EXIT
PAYLOAD=$TMP/payload
: >"$PAYLOAD"
nparts=0
if [ "$debug" = 1 ]; then
    # -g: the source names (as given) travel first, one NUL-separated part each, then the sources
    for p in "$@"; do
        printf '%s\000' "$p" >>"$PAYLOAD"
    done
fi
for p in "$@"; do
    size=$(wc -c <"$p") || exit 1
    size=$(( size + 0 ))
    if [ "$mode" = link ]; then
        # e_ident: ELF64 little-endian, e_type ET_REL — the only input mode २ reads
        head6=$(od -An -v -tx1 -N6 "$p" | tr -d ' \n')
        etype=$(dd if="$p" bs=1 skip=16 count=2 2>/dev/null | od -An -v -tx1 | tr -d ' \n')
        if [ "$size" -lt 64 ] || [ "$head6" != 7f454c460201 ] || [ "$etype" != 0100 ]; then
            refuse "--संयोजय $p: not an ELF64 relocatable object"
        fi
    else
        nonul=$(LC_ALL=C tr -d '\000' <"$p" | wc -c)
        if [ "$(( nonul + 0 ))" -ne "$size" ]; then
            refuse "$p holds a NUL octet, which the input channel uses as the separator"
        fi
        [ "$nparts" -gt 0 ] && printf '\000' >>"$PAYLOAD"
    fi
    cat "$p" >>"$PAYLOAD"
    nparts=$(( nparts + 1 ))
done

case $mode in object) what=object ;; *) what=image ;; esac
if [ -z "$load" ]; then loadarg=$LOAD_DEFAULT; else loadarg=$load; fi
case $mode in image) modeno=0; [ "$compressed" = 1 ] && modeno=4 ;; object) modeno=1 ;; *) modeno=2 ;; esac
[ "$debug" = 1 ] && modeno=$(( modeno + 8 ))

b=${out##*/}
stem=${b%.*}
if [ "$stem" = "$b" ] || [ -z "$stem" ] || [ -z "${stem#"${stem%%[!.]*}"}" ]; then stem=$b; fi
[ -n "$stem" ] || stem=w302

(
    unset YANTRA_INPUT_TRACE
    YANTRA_INPUT=$PAYLOAD YANTRA_INPUT_NAME=$stem YANTRA_RAM=${W302_YANTRA_RAM-1073741824} \
        YANTRA_STEPS=4000000000000
    export YANTRA_INPUT YANTRA_INPUT_NAME YANTRA_RAM YANTRA_STEPS
    "$W302_YANTRA_RUN" "$W302_NATIVE" "$loadarg" "$modeno" >"$TMP/stdout" 2>"$TMP/stderr"
)
rc=$?
status=$(awk '/status: Some\(/ { s = $0; sub(/.*status: Some\(/, "", s); sub(/\).*/, "", s); v = s } END { printf "%s", v }' "$TMP/stderr")
# yantra-run still exits 0 when the program's writes reached its own input
# (yantra-run.rs, the COLLISION line); the census refuses that, and so does
# this shim: an image built from corrupted input is not a fed PASS.
collision=$(awk '/COLLISION/ { s = $0; sub(/^[ \t\r\n]+/, "", s); sub(/[ \t\r\n]+$/, "", s); print s; exit }' "$TMP/stderr")
if [ -n "$collision" ]; then
    refuse "the .t1 chain's input was overwritten while building $out: $collision"
fi
ocnt=$(wc -c <"$TMP/stdout"); ocnt=$(( ocnt + 0 ))
if [ "$rc" -ne 0 ] || [ "$status" != 0 ] || [ "$ocnt" -eq 0 ]; then
    lastline=$(awk 'NF { l = $0 } END { sub(/[ \t\r]+$/, "", l); printf "%s", l }' "$TMP/stderr")
    refuse "the .t1 chain wrote no $what for $out: status ${status:-None}, exit $rc, $ocnt octets — $lastline"
fi
cat "$TMP/stdout" >"$out" || exit 1
if [ -n "$load" ]; then
    if [ "$load" -eq 0 ]; then where=0x0; else where=$(printf '%#x' "$load"); fi
else
    where=default
fi
if [ "$mode" = object ]; then tag=OBJECT; else tag=IMAGE; fi
if [ "$mode" = link ]; then kind=object; verb=linked; else kind=source; verb=assembled; fi
log "$tag $out $ocnt octets from $nparts $kind(s) at $where"
printf '%s: %s file(s) %s by the .t1 chain (W-302 shim)\n' "$out" "$nparts" "$verb" >&2
