#!/bin/sh
# tools/lib/page-handoff.sh — ONE GATE RUN, ONE PAGE BUILD. Sourced, not run.
#
# ## The cost this removes, measured
#
# `tools/gate.sh` runs two steps over the same artefact. `check-sassembly-identity.sh`
# calls `build-sassembly-web.sh` into its own `mktemp -d`; `check-sassembly-page.sh`
# calls it into a second `mktemp -d` of its own (1,332,569 octets, rc 0 on this host).
# Nothing reads the other's. So the page build — ~25 s on a warm `yantra-wasm`, the
# hour-long wasm build cold — is paid TWICE in one gate, for two steps that assert
# different things about one identical file.
#
# ## Why a handed-over page needs a PROOF and not an assumption
#
# `e01c05f5` is the precedent one layer up: a cached `stage1.elf` whose `stage1.log`
# named a sibling worktree, A DIFFERENT WORKTREE, and three siblings
# quoting it as this tree's. A page handed between steps is the same hazard with a
# shorter fuse. So the producer records a CONTENT DIGEST of the tree it built from and
# the consumer recomputes it; a mismatch declines the page and builds its own.
#
# NOT the build stamp. `build-stamp.rs` (retired to archive/rust/build-stamp/) is explicit that it names HEAD AND NOT THE
# TREE — a binary built from a dirty worktree prints a commit it is not — so a stamp
# comparison would accept a page built before an uncommitted edit. The digest here is
# every byte of every file `git` can see:
#
#     git ls-files -z --cached --others --exclude-standard | xargs -0 shasum -a 256 | shasum
#
# Tracked AND untracked-not-ignored, content and path, in `ls-files`' sorted order. 1,423
# files, 344 ms measured on this host against a 25 s build. A deleted tracked file still
# appears in the listing and produces no hash line, so a deletion moves the digest too —
# and `page_hash_stream_digest` below is why that is now a COUNT a decline can name and
# not only a hash that moved.
#
# ## It is a no-op unless a gate offers one, DELIBERATELY
#
# `SASSEMBLY_PAGE_HANDOFF` unset — which is every standalone run of either check — makes
# `page_handoff_publish` write nothing and `page_handoff_accept` decline with a reason.
# NEITHER CHECK MAY DEPEND ON THE OTHER HAVING RUN: both still build their own page when
# handed nothing, and that is what probe 8 of the selftest holds.
#
# Its own selftest is `./tools/lib/page-handoff.sh --selftest` — eighteen probes, and EVERY
# ONE of `page_handoff_accept`'s seven refusals now has an arm, four of which are handovers
# the producer must report as LOST rather than losing silently, and two of which are hash
# streams shorter than the listing they came from. The count read FIVE refusals until the
# clause for a prov that NAMES NO TREE OR NO SIDE DIGEST got the two probes it never had,
# and SIX until this tree's own digest being unobtainable got the third.

# `<sha256>-<listed>-<hashed>` over a NUL-separated listing on stdin. The two counts are
# SEPARATE FACTS and that is the whole reason this is its own function.
#
# THE COUNT THIS FILE USED TO CARRY COULD NOT SEE THE THING ITS OWN MARGIN CLAIMED IT SAW.
# It read: *"the FILE COUNT is appended to the hash and it is not decoration: `xargs`
# batches at ARG_MAX, and a batch lost to a full pipe would silently shorten the stream
# the second `shasum` reads. A count beside the hash makes that visible."* It does not.
# The count was taken from a SECOND, INDEPENDENT `git ls-files` run, so a batch lost
# INSIDE the hashing pipeline leaves it untouched: 1,423 both before and after, and only
# the hash moves — which is exactly the "differs for no stated reason" the margin said it
# had ruled out. One listing counted, a different stream hashed, and the count vouching
# for the stream it never read.
#
# So `<hashed>` is counted off the hash lines THEMSELVES, in the same pass as the hash:
# the stream is captured once, then counted and hashed from the same bytes. `<listed>` is
# still the listing's own NUL count, and the pair is the instrument — they AGREE on a
# whole tree (1,423 and 1,423, measured on this host, one `xargs` batch: 49,606 octets of
# names against an ARG_MAX of 1,048,576, so 21× of headroom) and they DISAGREE exactly
# when a file in the listing produced no hash line. A deletion is such a case and moves
# the digest, as before; a lost batch is now such a case too, and says so as a number.
#
# Capturing into a variable does NOT move the hash: `$( )` strips the trailing newlines
# and `printf '%s\n'` restores the one that `shasum`'s last line ended with, so the bytes
# the outer `shasum` reads are the pipeline's bytes. Verified by content, not by argument
# — this checkout hashes to `bc6bff43…303a` on both paths.
#
# Hashed > listed is therefore the instrument reporting its OWN breakage rather than the
# tree's state (a file name containing a newline is the one way to reach it, which is why
# `<listed>` is a NUL count and never `wc -l`), and probe 1-order holds it.
page_hash_stream_digest() {
  _ph_s=$(xargs -0 shasum -a 256 2>/dev/null)
  [ -n "$_ph_s" ] || return 1
  _ph_c=$(printf '%s\n' "$_ph_s" | grep -c '^' | tr -d ' ')
  _ph_h=$(printf '%s\n' "$_ph_s" | shasum -a 256 | cut -d' ' -f1)
  [ "${#_ph_h}" -eq 64 ] || return 1
  printf '%s %s\n' "$_ph_h" "$_ph_c"
}

# The digest of this worktree's content, cached for the life of the process.
#
# Cached because the consumer takes it twice (selftest, then the real decision) and the
# producer once, and because a tree that changes mid-script is not a case this can
# usefully straddle: the page was built from whatever was there, and 344 ms of
# re-reading would not make that more true.
_page_tree_digest_cache=
page_tree_digest() {
  if [ -n "$_page_tree_digest_cache" ]; then
    printf '%s\n' "$_page_tree_digest_cache"
    return 0
  fi
  git rev-parse --is-inside-work-tree >/dev/null 2>&1 || return 1
  # Counted off the NUL stream, not `wc -l`: a file name containing a newline would
  # make a line count disagree with the thing being hashed.
  _ph_n=$(git ls-files -z --cached --others --exclude-standard | tr -cd '\0' | wc -c | tr -d ' ')
  [ "${_ph_n:-0}" -gt 0 ] || return 1
  _ph_d=$(git ls-files -z --cached --others --exclude-standard | page_hash_stream_digest) \
    || return 1
  _page_tree_digest_cache="${_ph_d% *}-$_ph_n-${_ph_d#* }"
  printf '%s\n' "$_page_tree_digest_cache"
}

# One sha256 over the named side artefacts, in the order given, or `-` for none.
#
# The page is not the whole handover. `check-sassembly-identity.sh` loads the wasm module
# from `crates/yantra-wasm/target/...` — a SIDE EFFECT of the page build and not a thing
# inside the page — and `target/` is ignored, so the tree digest above is blind to it. A
# missing artefact hashes to its own absence and moves this, which is the honest answer.
page_side_digest() {
  [ "$#" -gt 0 ] || { printf '%s\n' -; return 0; }
  for _ph_a in "$@"; do
    if [ -f "$_ph_a" ]; then shasum -a 256 "$_ph_a"; else printf 'ABSENT  %s\n' "$_ph_a"; fi
  done | shasum -a 256 | cut -d' ' -f1
}

# The page build's outputs that live OUTSIDE the page, named ONCE so the producer and the
# consumer cannot disagree about the list. A list that differed between the two sides
# would decline every handover forever and say "the side artefacts do not match", which
# reads as a stale page rather than as a typo in a path.
#
# Both modules, because both are outputs of `build-sassembly-web.sh` (`:75`, `:99`) and
# `check-sassembly-identity.sh:183` loads the yantra one straight out of `target/` — which
# is ignored, so no amount of tree digest can see it change.
#
# Word-split on use, which is why it is a string and not quoted at the call sites.
PAGE_HANDOFF_SIDE="${YANTRA_WASM:-target/native-wasm/yantra_wasm.wasm} target/native-wasm/sadhana_wasm.wasm"

# THREE STATES ON THIS SIDE TOO — published, LOST, and never offered.
#
# The reason this exists is the one `page_handoff_accept` records twenty lines below: an
# instrument with two states where the truth has three hides its own breakage. Every
# failure path in `page_handoff_publish` used to be a bare `return 0`, so a handoff that
# was OFFERED and then LOST — the page absent, the directory gone with the lock that held
# it, the copy refused — printed exactly what a standalone run with nothing offered
# prints: NOTHING. Measured before this was written, on one tree, all rc 0 and all with
# empty output: a page that does not exist, an unwritable handoff directory, and no
# handoff at all. Three different truths, one blank line.
#
# And the consumer cannot recover which: it answers "the producing step did not run, or
# could not build one" — one message for two causes, and the producer is the only side
# that knows. The cost of each is the same (the later step builds its own page) and that
# is precisely why it must be SAID: a lost handover is invisible in the gate's pass/fail
# by construction, so if it is not written down it is not anywhere.
#
# THE NOT-OFFERED CASE STAYS SILENT AND SETS NO REASON. It is not a loss — it is the
# invariant (probe 8), and every standalone run of `check-sassembly-page.sh` takes it. A
# line there would be noise on the common path, which is how a signal stops being read.
page_handoff_lost=
_page_handoff_lost() {
  page_handoff_lost=$1
  printf '  THE PAGE WAS NOT HANDED ON — the later step will build its own: %s\n' "$1"
  return 0
}

# page_handoff_publish <page> [side-artefact...]
#
# ALWAYS rc 0, and the reason for any loss in `$page_handoff_lost`. The producer's own
# assertions have already passed by the time it calls this; a failure to hand the page on
# is a lost optimisation and must never be reported as a failed check. Every path here
# ends in a value, like `build-stamp.rs` (retired to archive/rust/build-stamp/).
page_handoff_publish() {
  _ph_page=$1; shift
  page_handoff_lost=
  [ -n "${SASSEMBLY_PAGE_HANDOFF:-}" ] || return 0
  # SET BUT GONE. The gate puts this directory inside `$LOCK`, which `W-145`'s trap
  # removes and another gate may reclaim — so "offered, then taken away" is a state this
  # tree can actually reach, and it is not the same fact as "nobody offered".
  [ -d "$SASSEMBLY_PAGE_HANDOFF" ] || {
    _page_handoff_lost "the offered directory $SASSEMBLY_PAGE_HANDOFF is not there — the gate's lock may have been reclaimed under this run"
    return 0; }
  [ -f "$_ph_page" ] || {
    _page_handoff_lost "the page to hand on does not exist: $_ph_page"
    return 0; }
  _ph_tree=$(page_tree_digest) || {
    _page_handoff_lost "this tree's own content digest could not be taken, so the page can be vouched for by nothing"
    return 0; }
  _ph_side=$(page_side_digest "$@") || {
    _page_handoff_lost "the side artefacts could not be digested"
    return 0; }
  cp "$_ph_page" "$SASSEMBLY_PAGE_HANDOFF/page.part" 2>/dev/null || {
    _page_handoff_lost "the page could not be copied into $SASSEMBLY_PAGE_HANDOFF"
    return 0; }
  { printf 'tree %s\nside %s\n' "$_ph_tree" "$_ph_side" \
      >"$SASSEMBLY_PAGE_HANDOFF/prov.part"; } 2>/dev/null || {
    _page_handoff_lost "the prov could not be written beside the page in $SASSEMBLY_PAGE_HANDOFF"
    return 0; }
  # PAGE FIRST, PROV LAST, both by rename. The consumer refuses a page with no prov
  # beside it, so this order can only ever lose the handover — never grant one that
  # is half written.
  mv "$SASSEMBLY_PAGE_HANDOFF/page.part" "$SASSEMBLY_PAGE_HANDOFF/page.html" 2>/dev/null || {
    _page_handoff_lost "the page could not be renamed into place in $SASSEMBLY_PAGE_HANDOFF"
    return 0; }
  mv "$SASSEMBLY_PAGE_HANDOFF/prov.part" "$SASSEMBLY_PAGE_HANDOFF/prov" 2>/dev/null || {
    _page_handoff_lost "the prov could not be renamed into place, so the page beside it is UNVOUCHED and will be refused"
    return 0; }
  printf '  handed the page to the later steps of this gate: %s\n' "$SASSEMBLY_PAGE_HANDOFF/page.html"
  printf '  vouched for by tree %s\n' "$_ph_tree"
  return 0
}

# page_handoff_accept [side-artefact...] — rc 0 and the page's path in
# `$page_handoff_page`; rc 1 and the reason in `$page_handoff_why`.
#
# THREE STATES AND NOT TWO: accepted, declined-with-a-reason, and built-here. A boolean
# would make "no gate offered one" and "the offered page is from another tree" print the
# same nothing, and the second is the `wt-raminject` fault.
#
# THE ANSWER IS A VARIABLE AND NOT STDOUT, AND THAT IS THE WHOLE POINT. The first spelling
# printed the path, so the natural caller was `if page=$(page_handoff_accept ...)` — A
# SUBSHELL, which threw `$page_handoff_why` away on the way out. Measured live: a page
# vouched by another tree was correctly refused and the check printed an EMPTY reason, so
# the three states collapsed back to two in the one case that matters. The selftest did not
# see it because its probes called this directly, which is the shape the checks use now.
page_handoff_why=
page_handoff_page=
page_handoff_accept() {
  page_handoff_why=
  page_handoff_page=
  if [ -z "${SASSEMBLY_PAGE_HANDOFF:-}" ]; then
    page_handoff_why="no page was offered — SASSEMBLY_PAGE_HANDOFF is unset, which is every standalone run"
    return 1
  fi
  if [ ! -f "$SASSEMBLY_PAGE_HANDOFF/page.html" ]; then
    page_handoff_why="the handoff directory holds no page — the producing step did not run, or could not build one"
    return 1
  fi
  if [ ! -f "$SASSEMBLY_PAGE_HANDOFF/prov" ]; then
    page_handoff_why="a page with NO prov beside it: UNVOUCHED, so nothing but assumption says it is this tree's"
    return 1
  fi
  _ph_wtree=$(awk '$1=="tree" {print $2; exit}' "$SASSEMBLY_PAGE_HANDOFF/prov")
  _ph_wside=$(awk '$1=="side" {print $2; exit}' "$SASSEMBLY_PAGE_HANDOFF/prov")
  if [ -z "$_ph_wtree" ] || [ -z "$_ph_wside" ]; then
    page_handoff_why="the prov beside the page names no tree or no side digest, so it vouches for nothing"
    return 1
  fi
  if ! _ph_ntree=$(page_tree_digest); then
    page_handoff_why="this tree's own content digest could not be taken here, so the offered page cannot be proved to be its"
    return 1
  fi
  if [ "$_ph_wtree" != "$_ph_ntree" ]; then
    page_handoff_why="the offered page was built from ANOTHER TREE: it vouches $_ph_wtree, this tree is $_ph_ntree"
    return 1
  fi
  _ph_nside=$(page_side_digest "$@")
  if [ "$_ph_wside" != "$_ph_nside" ]; then
    page_handoff_why="the tree matches but the build's side artefacts do not: vouched $_ph_wside, found $_ph_nside"
    return 1
  fi
  page_handoff_page=$SASSEMBLY_PAGE_HANDOFF/page.html
  return 0
}

# ── selftest ───────────────────────────────────────────────────────────────────
# Runs only when this file is EXECUTED, never when sourced. Eighteen probes, and SEVEN of
# them are refusals — all seven of `accept`'s: a library that only ever proved the happy
# path would hand over a foreign page exactly as cheerfully.
case ${0##*/} in page-handoff.sh)
  [ "${1:-}" = --selftest ] || { echo "$0: sourced, not run. Pass --selftest to exercise it."; exit 2; }
  fails=0
  st=$(mktemp -d) || exit 1
  trap 'rm -rf "$st"' EXIT
  bad() { fails=$((fails + 1)); echo "  FAIL  selftest $1"; }

  # 1 and 2 are about the digest itself: it must exist and it must not move under a
  # second reading. An unstable digest would decline every handover and the double
  # build would come back silently as "the page was from another tree".
  d1=$(page_tree_digest) || bad "1: this checkout yields no tree digest at all"
  # Checked by LENGTH and not by a glob of question marks: the first spelling of this
  # probe miscounted them by one and reported a perfectly well-formed digest as malformed.
  d1h=${d1%%-*}; d1rest=${d1#*-}; d1listed=${d1rest%-*}; d1hashed=${d1rest#*-}
  case $d1h in *[!0-9a-f]*|'') bad "1-hex: '$d1h' is not lowercase hex" ;; esac
  case $d1listed in *[!0-9]*|'') bad "1-listed: '$d1listed' is not a file count" ;; esac
  case $d1hashed in *[!0-9]*|'') bad "1-hashed: '$d1hashed' is not a file count" ;; esac
  [ "${#d1h}" -eq 64 ] || bad "1-width: the hash half is ${#d1h} characters, not 64 ('$d1')"
  # THREE fields and not two: the second count is the one the margin above could not
  # produce, and a digest that has lost it would still satisfy every probe written for
  # the two-field spelling.
  case $d1 in *-*-*) ;; *) bad "1-fields: '$d1' is not <hash>-<listed>-<hashed>" ;; esac
  # Hashed ABOVE listed is not a fact about the tree — it is this instrument reporting its
  # own breakage, and it must not pass as a count.
  [ "${d1hashed:-0}" -le "${d1listed:-0}" ] \
    || bad "1-order: $d1hashed hash lines off a listing of $d1listed ('$d1')"
  d2=$(page_tree_digest)
  [ "$d1" = "$d2" ] || bad "2: the digest moved between two readings ('$d1' then '$d2')"

  printf 'a page\n' >"$st/page.html"
  printf 'a wasm\n' >"$st/side.wasm"
  SASSEMBLY_PAGE_HANDOFF=$st/handoff
  export SASSEMBLY_PAGE_HANDOFF
  mkdir -p "$SASSEMBLY_PAGE_HANDOFF"
  page_handoff_publish "$st/page.html" "$st/side.wasm" >/dev/null

  # 3: the round trip. If this is the only probe that passes the library is a cache with
  # no proof in it.
  # CALLED DIRECTLY and never in `$( )`, because that is the convention the checks must
  # use: a subshell would discard both `page_handoff_page` and `page_handoff_why`, and the
  # earlier spelling of this probe did exactly that and so could not see the decline
  # reasons going blank.
  # SEEDED, because an unseeded probe cannot see the clause it is testing: `page_handoff_why`
  # is empty from its own initialiser at this point, so "an accepted page carries no reason"
  # would hold even with the reset inside the function deleted. Verified by mutation — with
  # the seed the deletion reds 3-why, without it the selftest stays green.
  page_handoff_why=a-stale-reason-from-an-earlier-decline
  page_handoff_accept "$st/side.wasm" \
    || bad "3: a page this tree just published was refused — '$page_handoff_why'"
  [ "$page_handoff_page" = "$SASSEMBLY_PAGE_HANDOFF/page.html" ] \
    || bad "3-path: accept answered '$page_handoff_page'"
  [ -z "$page_handoff_why" ] || bad "3-why: an ACCEPTED page carries a reason: '$page_handoff_why'"

  # 4: THE wt-raminject CASE. A page vouched by a tree that is not this one must be
  # refused by name, and the reason must say which two trees disagreed.
  sed 's/^tree .*/tree 0000000000000000000000000000000000000000000000000000000000000000-1/' \
    "$SASSEMBLY_PAGE_HANDOFF/prov" >"$st/p4" && cp "$st/p4" "$SASSEMBLY_PAGE_HANDOFF/prov"
  if page_handoff_accept "$st/side.wasm"; then
    bad "4: a page vouched by ANOTHER tree was accepted"
  else
    case $page_handoff_why in *"ANOTHER TREE"*) ;; *) bad "4-why: '$page_handoff_why'" ;; esac
  fi

  # 5: the tree matches and the SIDE artefact does not. This is the module the identity
  # check loads out of `target/`, which no tree digest can see.
  page_handoff_publish "$st/page.html" "$st/side.wasm" >/dev/null
  printf 'a DIFFERENT wasm\n' >"$st/side.wasm"
  if page_handoff_accept "$st/side.wasm"; then
    bad "5: a page whose side artefact has changed under it was accepted"
  else
    case $page_handoff_why in *"side artefacts do not"*) ;; *) bad "5-why: '$page_handoff_why'" ;; esac
  fi

  # 6: prov present, page gone. 7: page present, prov gone — UNVOUCHED, and it is the
  # one an optimisation written in a hurry would wave through.
  page_handoff_publish "$st/page.html" "$st/side.wasm" >/dev/null
  rm -f "$SASSEMBLY_PAGE_HANDOFF/page.html"
  page_handoff_accept "$st/side.wasm" && bad "6: a prov with no page was accepted"
  page_handoff_publish "$st/page.html" "$st/side.wasm" >/dev/null
  rm -f "$SASSEMBLY_PAGE_HANDOFF/prov"
  if page_handoff_accept "$st/side.wasm"; then
    bad "7: a page with NO prov was accepted"
  else
    case $page_handoff_why in *UNVOUCHED*) ;; *) bad "7-why: '$page_handoff_why'" ;; esac
  fi

  # 8: THE INVARIANT THE WHOLE CHANGE RESTS ON. With no gate offering a handoff, accept
  # declines, so each check builds its own page and neither depends on the other.
  page_handoff_publish "$st/page.html" "$st/side.wasm" >/dev/null
  unset SASSEMBLY_PAGE_HANDOFF
  if page_handoff_accept "$st/side.wasm"; then
    bad "8: a page was accepted with SASSEMBLY_PAGE_HANDOFF unset"
  else
    case $page_handoff_why in *unset*) ;; *) bad "8-why: '$page_handoff_why'" ;; esac
  fi

  # ── 9-13: THE PRODUCER'S THIRD STATE ───────────────────────────────────────
  # Probes 1-8 are all about the CONSUMER's verdict. The producer had two states where
  # the truth has three, and nothing above could see it: every probe that publishes does
  # so with `>/dev/null`, so a publish that lost the page silently passed all of them.
  #
  # STDOUT IS CAPTURED BY REDIRECTION AND NEVER BY `$( )`. A command substitution is a
  # subshell and `page_handoff_lost` is set inside it — the same hazard that made a
  # foreign page get refused with a blank reason, recorded at `page_handoff_accept`. With
  # `>file` the call stays in this shell, so one call yields BOTH the variable and the
  # line, and the two can be asserted against each other.
  SASSEMBLY_PAGE_HANDOFF=$st/handoff
  export SASSEMBLY_PAGE_HANDOFF
  mkdir -p "$SASSEMBLY_PAGE_HANDOFF"
  printf 'a wasm\n' >"$st/side.wasm"

  # 9: THE CONTROL, and it runs first for the reason probe 3-why needed a seed — a
  # `page_handoff_lost` that is merely never set would satisfy every arm below with the
  # reset at the top of `publish` deleted. Seeded with a stale loss, so a successful
  # publish must CLEAR it.
  page_handoff_lost=a-stale-loss-from-an-earlier-publish
  page_handoff_publish "$st/page.html" "$st/side.wasm" >"$st/o9"
  [ -z "$page_handoff_lost" ] \
    || bad "9: a SUCCESSFUL publish reports a loss: '$page_handoff_lost'"
  grep -aq 'handed the page to the later steps' "$st/o9" \
    || bad "9-said: a successful publish did not say so: '$(cat "$st/o9")'"
  grep -aq 'NOT HANDED ON' "$st/o9" && bad "9-noloss: a successful publish printed a loss line"

  # 10: the page named does not exist. Measured before this arm existed: rc 0, output
  # EMPTY, indistinguishable from probe 13.
  page_handoff_publish "$st/absent.html" "$st/side.wasm" >"$st/o10"
  case $page_handoff_lost in
    *"does not exist"*) ;;
    *) bad "10: an absent page was not reported as lost: '$page_handoff_lost'" ;;
  esac
  grep -aq 'NOT HANDED ON' "$st/o10" \
    || bad "10-said: an absent page printed no loss line: '$(cat "$st/o10")'"

  # 11: the handoff directory is there and cannot be written. The `cp` path, which was
  # the second of the three blank lines.
  mkdir -p "$st/ro"
  SASSEMBLY_PAGE_HANDOFF=$st/ro
  chmod a-w "$st/ro"
  page_handoff_publish "$st/page.html" "$st/side.wasm" >"$st/o11"
  chmod u+w "$st/ro"
  case $page_handoff_lost in
    *"could not be copied"*) ;;
    *) bad "11: an unwritable handoff directory was not reported as lost: '$page_handoff_lost'" ;;
  esac
  [ -f "$st/ro/page.html" ] && bad "11-wrote: a page appeared in an unwritable directory"

  # 12: SET BUT GONE — the state the gate can actually reach, because its handoff lives
  # inside `$LOCK` and `W-145`'s trap removes that. NOT the same fact as probe 13, and
  # before this arm the two printed the same nothing.
  SASSEMBLY_PAGE_HANDOFF=$st/never-made
  page_handoff_publish "$st/page.html" "$st/side.wasm" >"$st/o12"
  case $page_handoff_lost in
    *"is not there"*) ;;
    *) bad "12: a vanished handoff directory was not reported as lost: '$page_handoff_lost'" ;;
  esac

  # 13: NOT OFFERED — SILENT, AND IT MUST STAY SILENT. This is probe 8's invariant from
  # the producer's side: every standalone run of `check-sassembly-page.sh` lands here, so
  # a loss line would be printed on the common path and stop being read. Seeded again,
  # because the clause under test is that `publish` CLEARS the reason rather than that it
  # never wrote one.
  unset SASSEMBLY_PAGE_HANDOFF
  page_handoff_lost=a-stale-loss-that-must-not-survive
  page_handoff_publish "$st/page.html" "$st/side.wasm" >"$st/o13"
  [ -z "$page_handoff_lost" ] \
    || bad "13: no handoff offered is reported as a LOSS: '$page_handoff_lost'"
  [ ! -s "$st/o13" ] || bad "13-quiet: a standalone run printed '$(cat "$st/o13")'"

  # ── 14-15: THE SHORT STREAM ─────────────────────────────────────────────────
  # Probes 1-13 are all about a WHOLE tree, where listed and hashed agree, so none of them
  # can tell the two counts apart — and that is precisely how the old one-count digest
  # carried a margin claiming to see a lost `xargs` batch while being blind to it. These
  # two drive `page_hash_stream_digest` from an explicit listing instead, which is the
  # only way to present it with a stream SHORTER than its listing without reaching into
  # the real path to break it.

  # 14: one name in the listing that hashes to nothing. The count must drop to the lines
  # actually produced, and the hash must move as well — a count alone would be a second
  # chance to be wrong in the same place.
  printf 'one\n' >"$st/s1"; printf 'two\n' >"$st/s2"
  full=$(printf '%s\0' "$st/s1" "$st/s2" | page_hash_stream_digest) \
    || bad "14: two readable files yield no stream digest at all"
  short=$(printf '%s\0' "$st/s1" "$st/s2" "$st/absent-from-disk" | page_hash_stream_digest) \
    || bad "14-rc: a listing with one absent name yielded no digest; it has two hash lines"
  [ "${full#* }" = 2 ] || bad "14-full: two files counted as '${full#* }'"
  [ "${short#* }" = 2 ] \
    || bad "14-short: three names, two hash lines, counted as '${short#* }' — the count is of the LISTING and not of the stream, which is the defect this replaced"
  [ "${full%% *}" = "${short%% *}" ] \
    || bad "14-hash: an absent name changed the hash, so the stream is not the two files"

  # 15: the stream lost WHOLE. `rc 1` and no digest, because the alternative is hashing
  # nothing to a fixed value — and a constant is a digest that matches every other tree
  # that also failed. `publish` turns this into its LOST state and `accept` into a
  # decline, both of which already have arms above.
  printf '%s\0' "$st/absent-from-disk" | page_hash_stream_digest >"$st/o15" 2>&1 \
    && bad "15: a listing that hashes to nothing still yielded a digest: '$(cat "$st/o15")'"
  [ ! -s "$st/o15" ] || bad "15-quiet: a lost stream printed '$(cat "$st/o15")'"

  # ── 16-17: A PROV THAT NAMES NOTHING ───────────────────────────────────────
  # The sixth refusal clause — "names no tree or no side digest" — had no probe, and the
  # file header counted five refusals as though the count were complete. It is reachable:
  # a prov written by an OLDER SPELLING of the digest lines, or one truncated between the
  # two `printf` fields, presents a prov that is PRESENT and vouches for nothing.
  #
  # BOTH ARMS ASSERT THE REASON AND NOT ONLY THE REFUSAL, because refusal alone cannot see
  # the clause: with it deleted an empty `_ph_wtree` falls through to the tree comparison
  # and is refused as ANOTHER TREE, and an empty `_ph_wside` to the side comparison. The
  # page would still be declined — by a clause that names the wrong fault, which is the
  # `wt-raminject` reason-blanking defect in a quieter form.
  SASSEMBLY_PAGE_HANDOFF=$st/handoff
  export SASSEMBLY_PAGE_HANDOFF
  mkdir -p "$SASSEMBLY_PAGE_HANDOFF"
  printf 'a page\n' >"$st/page.html"
  printf 'a wasm\n' >"$st/side.wasm"
  page_handoff_publish "$st/page.html" "$st/side.wasm" >/dev/null

  # 16: the tree line under an older name. `awk '$1=="tree"'` matches nothing, so the
  # vouched tree is EMPTY while the prov file itself is perfectly present and non-empty.
  sed 's/^tree /worktree /' "$SASSEMBLY_PAGE_HANDOFF/prov" >"$st/p16" \
    && cp "$st/p16" "$SASSEMBLY_PAGE_HANDOFF/prov"
  [ -s "$SASSEMBLY_PAGE_HANDOFF/prov" ] \
    || bad "16-setup: the rewritten prov is empty, so probe 7's clause would answer instead"
  if page_handoff_accept "$st/side.wasm"; then
    bad "16: a prov that names NO TREE was accepted"
  else
    case $page_handoff_why in
      *"vouches for nothing"*) ;;
      *) bad "16-why: a prov naming no tree was refused for the wrong reason: '$page_handoff_why'" ;;
    esac
  fi

  # 17: the tree is this tree's and the SIDE line is gone — the truncation `publish` can
  # only produce if its `printf` is cut short, and the half the tree digest cannot cover.
  page_handoff_publish "$st/page.html" "$st/side.wasm" >/dev/null
  grep -av '^side ' "$SASSEMBLY_PAGE_HANDOFF/prov" >"$st/p17" \
    && cp "$st/p17" "$SASSEMBLY_PAGE_HANDOFF/prov"
  grep -aq '^tree ' "$SASSEMBLY_PAGE_HANDOFF/prov" \
    || bad "17-setup: the surviving prov names no tree either, so this is probe 16 twice"
  if page_handoff_accept "$st/side.wasm"; then
    bad "17: a prov with a good tree and NO SIDE digest was accepted"
  else
    case $page_handoff_why in
      *"vouches for nothing"*) ;;
      *) bad "17-why: a prov naming no side digest was refused for the wrong reason: '$page_handoff_why'" ;;
    esac
  fi

  # ── 18: THE DIGEST THAT CANNOT BE TAKEN ────────────────────────────────────
  # The SEVENTH ground, and the one the count of six did not include: a prov that names
  # both halves, over a tree whose OWN digest cannot be computed here. `page_tree_digest`
  # returns 1 when the stream hashes to nothing (probe 15), so this arm is the consumer's
  # half of that failure and it is the only remaining `return 1` in `accept` with no probe.
  #
  # DRIVEN BY OVERRIDING `page_tree_digest` INSIDE A SUBSHELL, which is the one place in
  # this selftest where a subshell is the point rather than the hazard: the override must
  # not survive into any later probe, and sh has no portable way to put the real function
  # back. So the REASON is printed and read off a file instead of crossing the boundary in
  # `$page_handoff_why` — the same discipline probes 9-13 state for stdout, inverted.
  page_handoff_publish "$st/page.html" "$st/side.wasm" >/dev/null
  grep -aq '^side ' "$SASSEMBLY_PAGE_HANDOFF/prov" \
    || bad "18-setup: the republished prov has no side line, so probe 17's clause answers"
  ( page_tree_digest() { return 1; }
    if page_handoff_accept "$st/side.wasm"; then
      printf 'ACCEPTED\n'
    else
      printf '%s\n' "$page_handoff_why"
    fi ) >"$st/o18"
  case $(cat "$st/o18") in
    ACCEPTED) bad "18: a page was accepted while this tree's own digest could not be taken" ;;
    *"could not be taken here"*) ;;
    *) bad "18-why: the unobtainable digest was refused for the wrong reason: '$(cat "$st/o18")'" ;;
  esac
  # AND THE OVERRIDE IS GONE: a leaked one would make every probe after this read a tree
  # with no digest, and there is nothing after this but the verdict.
  page_tree_digest >/dev/null || bad "18-leak: page_tree_digest still fails after the probe"

  if [ "$fails" -ne 0 ]; then
    echo "page-handoff: $fails of 18 probes FAILED, so nothing that sources this can be believed."
    exit 1
  fi
  echo "page-handoff: 18 probes ok (digest shape and stability, the round trip, ALL SEVEN refusals —"
  echo "              another tree, a changed side artefact, no page, no prov, no handoff offered,"
  echo "              a prov that vouches for nothing (no tree named, or no side digest), and this"
  echo "              tree's own digest unobtainable —"
  echo "              the producer's three states: handed on, LOST with a reason, not offered —"
  echo "              and the two counts: a stream shorter than its listing, and one lost whole)"
  exit 0 ;;
esac
