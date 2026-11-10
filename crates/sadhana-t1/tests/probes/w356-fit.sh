#!/bin/bash
set -u
H=$WT/target/release/t1_image
P=$WORK/w356-probe
# gen N [out]: write the N-line probe to out (default $P/fit$N.t1).
gen() {
  awk -v N="$1" 'BEGIN {
    split("०,१,२,३,४,५,६,७,८,९", D, ",")
    print "मण्डलम् बहुमूल्य" dev(N) " ॥"; print ""
    print "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि"
    print "    चरः सूची ॱॱ अङ्कः अन्तः न६४ भवति ० ।"
    for (i = 1; i <= N; i++) print "    सूची अङ्कः " dev(i) " अन्तः भवति " dev(i * 7 % 997) " ।"
    print ""; print "    प्रत्यागमनम् ० ।"; print "इति"
  }
  function dev(n,   s, k, r) {
    if (n == 0) return "०"
    s = n ""; r = ""
    for (k = 1; k <= length(s); k++) r = r D[substr(s, k, 1) + 1]
    return r
  }' > "${2:-$P/fit$1.t1}"
}
# table FILE: per-commit steps table and log-ratio exponents.
table() {
  awk '
  function com(n,   s, r) {
    s = sprintf("%.0f", n); r = ""
    while (length(s) > 3) { r = "," substr(s, length(s) - 2) r; s = substr(s, 1, length(s) - 3) }
    return s r
  }
  { if (!($1 in seen)) { seen[$1] = 1; order[++nc] = $1 }
    if (!(($1, $2 + 0) in st)) { cnt[$1]++; ns[$1, cnt[$1]] = $2 + 0 }
    st[$1, $2 + 0] = $3 + 0 }
  END {
    for (ci = 1; ci <= nc; ci++) {
      c = order[ci]; m = cnt[c]
      for (i = 1; i <= m; i++) for (j = i + 1; j <= m; j++)
        if (ns[c, j] < ns[c, i]) { t = ns[c, i]; ns[c, i] = ns[c, j]; ns[c, j] = t }
      print "=== " c " ==="
      for (i = 1; i <= m; i++) { n = ns[c, i]; printf "  N=%5d steps=%14s\n", n, com(st[c, n]) }
      if (m >= 2) {
        for (i = 1; i < m; i++) { a = ns[c, i]; b = ns[c, i + 1]
          printf "  %d->%d: n^%.3f\n", a, b, log(st[c, b] / st[c, a]) / log(b / a) }
        a = ns[c, 1]; b = ns[c, m]
        printf "  overall: n^%.3f\n", log(st[c, b] / st[c, a]) / log(b / a)
      }
    }
  }' "$1"
}
case ${1:-} in
  --gen) gen "$2" "${3:-}"; exit ;;
  --table) table "$2"; exit ;;
esac
echo "=== W-356 PRE-REGRESSION EXPONENT, fitted directly on 51598b3c ==="
echo "WHY: the exponent delta of 0.32 came from a two-point fit on the RATIO, and"
echo "  the arithmetic 'check' I ran on it was circular. This measures the base"
echo "  tree's own exponent so it can be compared with W-346's post-regression"
echo "  n^1.63 without deriving one from the other."
echo "PREDICTION: if the delta is real the base tree fits near n^1.31."
echo
for T in $WORK/w356-base $WORK/w347-before; do
  C=$(git -C "$T" rev-parse --short HEAD)
  echo "---------- tree $C ----------"
  for N in 240 600 1200; do
    gen "$N"
    S=$( { "$H" --spec-root "$T/spec" --compiler "$T/crates/sadhana-t1/src" \
             -o "$P/fit$N-$C.elf" "$P/fit$N.t1" 2>&1; } | grep -oE 'steps: *[0-9]+' | grep -oE '[0-9]+' )
    [ -n "$S" ] || { echo "  GUARD: no step count for N=$N on $C"; exit 9; }
    echo "  $C N=$N steps=$S"
    echo "$C $N $S" >> "$P/fit.tsv"
  done
done
echo
table "$P/fit.tsv"
date "+%H:%M:%S %Z END"
