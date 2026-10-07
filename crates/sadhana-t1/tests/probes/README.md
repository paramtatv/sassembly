# Falsifier probes for BACKLOG rows

**These are not cargo tests.** Cargo auto-discovers test targets only from `.rs`
files directly in `tests/`, so no `.rs` here becomes a target — checked with
`cargo metadata`, which lists no target for `zz_w304_probe.rs`. They are the
*falsifiers named by BACKLOG rows*, kept where the rows can cite them.

> **BUT "INERT" WAS WRONG, AND IT WAS MY OWN WORD.** This paragraph used to say
> "everything in this subdirectory is inert". That is true of **cargo's target
> discovery** and false of the tree, and the gap is the whole difference between
> a `.rs` file and a `.t1` one. **The 31 `.t1` files this directory HELD WERE
> corpus input** (25 from `096f46e9`, mine; 6 from `22eedbc3`, a peer session's). Eleven instruments recurse `crates/` collecting every path
> whose extension is `t1` — among them `t1_modules.rs:42`
> (`every_t1_source()`, which feeds the frozen-keyword guard and the
> lexer-refusal ratchet), `t1_sources.rs:4466`, `t1_unreached_modules.rs:70`,
> `t1_population.rs:170`, `t1_transcriptions.rs:317`,
> `t1_paradigm_names.rs:1271`, `w306c_width_claims.rs:103`,
> `sadhana/tests/paradigm_t0.rs:466`, `sadhana/tests/t1_scrape_bounds.rs:142`,
> `metrics/src/main.rs:5836` and `weight/src/main.rs:251`. **NOT ONE of them
> excludes this directory** — the single `probes` string in `t1_sources.rs` is
> the unrelated phrase "the four acceptance probes".
>
> So `crates/` held 21 corpus sources plus 5 in `textapp/src/text/` when those
> walks were written, and holds **57 `.t1` files** now. All 31 files here declare
> a module (`मण्डलम्`), so each enters those walks as a module-declaring source.
> The lexer-refusal ratchet's own margin reads *"every removal is progress and
> every addition is a defect"* — by that rule, adding 31 is 31 defects.
>
> Verifying inertness against **cargo** and writing it as inertness against
> **the tree** is the error to avoid repeating here. Before adding a file to
> this directory, ask what walks its extension, not what builds it.

## Why this directory exists

Rows W-356, W-357 and W-359 cited `/path/to/w359/`, `/path/to/w356-probe/`
and a worktree that was removed the same evening. A row whose falsifier cannot be
run is a row nobody can refute.

**The first repair of that was itself wrong, twice over**, and it is recorded here
because the shape recurs:

1. The probes were copied to `agent-handovers/sassembly-probes-2026-10-01/`, which
   **git does not track at all** — `git ls-files` returns zero files under
   `agent-handovers/`. So the citations moved from one path outside the repo to
   another path outside the repo.
2. The old absolute paths were never *removed* from the rows, only added to. The
   rows cited both.
3. Two source files, `bisect.tsv` and `grow.t1probe`, were left behind entirely, so even
   the incomplete copy was incomplete.

Measured convention, for anyone adding a falsifier: of 38 distinct paths cited by
rows, **36 are tracked by git**. Cite a tracked path or the row has no falsifier.

## Build them with `--load` or the run is ONE-SIDED

    t1_image --spec-root spec --compiler crates/sadhana-t1/src \
             --load <probe> --entry <module> मुख्यम् -o out.elf <probe>
    yantra-run out.elf                                  # native
    ... --predict-only -o out.elf <probe>               # interpreted; -o is STILL required

With `--entry`, `t1_image` is a DIFFERENTIAL GATE since `W-381`: it runs the
built image under the `yantra-run` beside it and compares that with the
predict; when they disagree the image is left at `out.elf.refused`, nothing is
at `out.elf`, and the exit is **97**. `--accept-divergence <reason>` writes it
anyway and records the reason in `out.elf.provenance` — the way to build a
probe whose whole point is a known divergence.

Without `--load` there is no predict: since `W-343` the log SAYS so —
`predict:  none — entry module … is named positionally (…) and not with
--load, …` — and since `W-381` such a build has nothing to compare and is
REFUSED unless `--accept-divergence <reason>` records why it may stand
(verdict `UNCHECKED`). `--no-predict` asks for the native half alone, on
purpose (the line reads `predict:  skipped (--no-predict)`), and needs the same
`--accept-divergence <reason>`. One hazard `--load` carries: the module
joins the interpreter that is RUNNING THE COMPILER, so a module name equal to
one of the compiler's own locals breaks the build — a module `प` stops it at
`sarani.t1:30`, where `योजनम्` has a parameter `प`.

`t1_image` lives in the **`sadhana`** package, not `sadhana-t1`. Its exit code
is the verdict since `W-343`: nonzero on a usage error (measured: `--bogus`
exits 1), on a source that failed to compile, and on an import no source
declares — and 97 (`W-381`) on a build the differential gate refuses. This paragraph used to say it "exits 0 on a usage error"; `usage()`
returns `FAILURE` and did before `W-343` too.

## W-359 — a callee's allocation is invisible to the caller

| probe | shape | interpreted | native |
|---|---|---|---|
| `callee.t1probe` | caller's array is `भवति ०`, **no storage**; callee writes index ५ | ९९ | **०** |
| `prewrite.t1probe` | identical **plus** the caller writes index ० first | ९९ | ९९ |
| `samefr.t1probe` | the same write in the **declaring** frame | ९९ | — |
| `append<N>.t1` | same-frame append sweep | — | ७ at every N |

### Since the ruled fix (option 4, 2026-10-04)

A run parameter grows only in a routine that RETURNS it on every path; in every
other routine a store at or past the parameter's length is refused by name —
interpreted *"entry i is past the end of the parameter `p`, a run of n, and `r`
does not return it (W-359)"*, natively the finisher word `0x359`
(`प्राचलसीमानिषेधः`), in FAIL form since W-381 (`0x3333 | 0x359 << 16`, status `Some(0x359)`). So `callee.t1probe`, `prewrite.t1probe` and
the `w359_grow*` / `w359_two_levels` probes are now REFUSED on both engines —
the divergence they measured is closed by refusing the shape. Their asserted
successors, run by `crates/yantra/tests/w359_param_growth.rs`:

| probe | shape | both engines |
|---|---|---|
| `w359_past_end.t1probe` | non-returning callee writes index ५ of a run of १ | refused |
| `w359_fresh_param.t1probe` | the same into a fresh (nil) run, index ० | refused |
| `w359_within.t1probe` | caller sized the run to ६; index ५ | १०५ |
| `w359_kosha_shape.t1probe` | the archive project `kosha.t1:106` — index १९ of a run of १५ | refused |
| `w359_dropped.t1probe` | a grow-and-return call as a bare statement | refused at load / typecheck |
| `w359_bound_elsewhere.t1probe` | the grown run bound to ANOTHER name | refused at load / typecheck |
| `w359_bound.t1probe` | the grown run assigned back to its own name, twice | २ |
| `w359_tail_return.t1probe` | tail-returned, so the caller grows-and-returns too | ३ |
| `w359_bad_tail.t1probe` | tail-returned from a routine that answers another run too | refused |
| `w359_remote_*.t1probe` | the drop across modules (store's grows flag) | refused |
| `w359_alias.t1probe` | a store through a local alias of a parameter | refused |
| `w359_alias_read.t1probe` | the alias bound and only read | ७ |
| `w359_octet_param.t1probe` | an octet-run parameter written past its end | refused |
| `w359_two_params.t1probe` | one parameter grown and returned, the other written in length | ७४ |
| `w359_two_params_past.t1probe` | the same, the other written past its end | refused |
| `w359_nil_return.t1probe` | grows, may answer `शून्यम्` — so NOT grow-and-return | refused |

Rule (A) of the final ruling: a grow-and-return call's result is assigned back to
the SAME name passed for the grown parameter, or tail-returned from a routine that
itself grows and returns that name; nothing else. A nil return does not count as
returning the parameter. Red-first readings on the tree before the change:
`w359_red_first.tsv`.

The cost pair (acceptance (a)) is generated by the test, not kept as a file:
a non-returning callee looping N times with and without one in-length store
into its parameter, N = 256 and 512.

`prewrite.t1probe` WAS the important one: the **negative control**, one statement apart
from `callee.t1probe`, and before the fix it AGREED (९९ on both engines). A diverging
probe on its own cannot show it is measuring anything. **Since `W-359` it no longer
agrees by answering — it agrees by REFUSING**: its callee writes index ५ of a run of
१ in a routine that does not return it, so both engines refuse the store
(`w359_past_end.t1probe` is the asserted form).

The mechanism is **an allocation or reallocation performed in a callee is invisible
to the caller, which keeps a stale handle** — native only; the interpreter is
correct. The 128/129 element boundary reported separately is where a *growing*
array reallocates; capacity zero is the same defect at the other end.

**`samefr.t1probe` no longer builds** (measured on `5dc20320`): 59,999,309 steps, then
link stops with "the product returned an empty image" and refusals naming `Arena`
with a `Nil` first slot — probably the same no-storage `भवति ०` hole. So the row's
own negative control is broken *by the defect it would disprove*, which is why
`prewrite.t1probe` was written instead of repairing it.

**The append sweep cannot report an absence inside its gap.** N = 2, 3, 4, 8, 16,
**64, 256**, 1024 … 16384 jumps straight over 128/129, so its "७ at every size"
was silent about that window by construction. Log spacing is the default way
anyone builds a sweep.

## W-356 / W-361 — compile cost

`fit240.t1probe`, `fit600.t1probe`, `fit1200.t1probe` embed N literals; `grow.t1probe` runs 2,000
appends at **runtime** instead, because `fit240.t1probe` executes only 6 instructions
and cannot see the emitted side at all. **The fit probes measure COMPILE steps and
must never be set beside a decode or execution figure.**

`axis.tsv` — 15 legs, one probe each at 240 literals, host pinned:

    51598b3c    350,519,225   FAST        db1ed3c8  350,519,525  FAST
    3d905c37  1,285,948,398   SLOW        11 others  ~1.281-1.286 G  SLOW

`db1ed3c8` is an ancestor of `3d905c37`; the range between them holds two commits
and **only `3d905c37` touches `crates/sadhana-t1/src`**. So 3.669x is attributed to
it by ancestry over a single candidate.

> **`51598b3c` IS NOT ON `origin` — CITE `b8ba2717` INSTEAD, WHICH IS THE SAME
> COMPILER BYTE FOR BYTE.** `51598b3c` was a trunk-local commit (the W-333/W-335
> id collision), dropped on 2026-10-02 once its content was confirmed already on
> main as W-335; it survives only at `refs/rescue/trunk-w333-51598b3c`, a ref on
> one Mac. So its row **cannot be reproduced from a clone** — a dangling citation
> inside a tracked file, which is the same class these probes were moved here to
> fix, one level deeper: the FILE is in the repo and its KEY is not.
>
> **THE SUBSTITUTION IS EXACT, NOT APPROXIMATE.** `b8ba2717` is `51598b3c`'s
> parent, IS on `origin/main`, and the two differ by **one line in BACKLOG.tsv**
> — nothing that reaches a build. Verified by content rather than by arithmetic:
>
>     crates/sadhana-t1/src   41ec399e63b871cb7b1d5660b4b17f87ec4b0dd8   (both)
>     spec                    254ca0802a90628ff2451f1edcff11ddfd201844   (both)
>
> So **350,519,225 above is reproducible at `b8ba2717`** with no offset and no
> caveat. Found by a peer session, whose SCALING.md provenance line cited the same
> dead commit and so handed every figure in that document an unobtainable tree.
>
> **CITE A SUBTREE HASH BESIDE A COMMIT.** A subtree hash is content-addressed and
> survives the commit being dropped; a commit key does not. That is the durable
> form of this citation and the reason the two hashes are written out here.
>
> `db1ed3c8` — the other fast-tier commit — was separately verified on a SECOND
> HOST by a peer session (Ubuntu, detached worktrees, host `t1_image` from
> `34c9712a`): fit240 = **350,519,525**, equal to the Mac to the instruction.
> That is T-102's host-independence tested rather than assumed, on a different
> OS, libc and binary format.
>
> **A ZSH TRAP THAT PRODUCES A CONFIDENT FALSE NEGATIVE**, worth copying because
> it cost a peer session two wrong answers: `"$c:crates/sadhana-t1/src"` has `:c` eaten
> as a HISTORY MODIFIER, so a sweep comparing subtrees silently compares garbage
> and reports "no match in all 3,080 commits". **Brace the ref:
> `"${c}:crates/..."`.** The broken instrument did not fail — it produced the more
> alarming story.
>
> **AND THE HOST IS AN INTEL CORE i9-9980HK, NOT APPLE SILICON.** `uname -m`
> answering `x86_64` is NOT sufficient, because Rosetta reports `x86_64` to a
> translated process; the reading needs `hw.optional.arm64` (ABSENT here) and
> `sysctl.proc_translated` (unset here). Checked 2026-10-02.
>
> **AND THE `51598b3c` → `db1ed3c8` GAP IS A CONSTANT, NOT NOISE.** +300 steps at
> N = 240 *and* at N = 600 — a fixed one-time cost, since it does not scale with
> the input. It leaves the exponent untouched: n^0.9103 either way, to four
> decimal places. So the fast tier is two slightly different trees and the
> linearity finding is indifferent to which one is used.

`fit.tsv` — two trees at N = 240/600/1200:

    N      51598b3c        1860a18f         ratio     local exponent
    240    350,519,225     1,280,993,383    3.655x    51598b3c: n^0.910 then n^0.994  (LINEAR)
    600    807,133,695     4,283,753,460    5.307x    1860a18f: n^1.317 then n^1.863  (CLIMBING)
    1200   1,607,313,520   15,579,164,380   9.693x

**DO NOT READ THAT AS "3d905c37 INTRODUCED THE SUPERLINEARITY."** `1860a18f` does
**not contain** `3d905c37` — `merge-base --is-ancestor` answers NO. The exponent
change is real and **unattributed**, and specifically not attributable to that
commit by this data. `axis.tsv` says the same thing quietly: ~1.281 G legs are slow
*without* `3d905c37` and ~1.2859 G legs are slow *with* it, so it adds about 0.39%
on top of a slowness that already exists on lines which "forked at `ab3d0d74` and
never received the speedup".

The experiment that settles it, four builds: **`fit600.t1probe` and `fit1200.t1probe` at
`e22a87a3` (= `3d905c37^`) and at `3d905c37`.** Two predictions are registered
before the run — that the exponent *changes* there (a peer session) and that it is
*unchanged* with only the constant moving (a peer session). One of them dies.

### SETTLED 2026-10-02 — the exponent CHANGES across that one commit

Run by a peer session on Ubuntu, host `t1_image` from `34c9712a`, every build showing
exactly one `steps:` line with the per-leg `rev-parse` guard green. Recomputed
independently here from their raw counts:

    tree        N=240          N=600          N=1200          exponents
    e22a87a3      350,519,525    807,133,995   1,607,313,820   n^0.910  n^0.994   LINEAR
    3d905c37    1,285,948,398  4,296,196,171  15,604,210,323   n^1.316  n^1.861   CLIMBING
    ratio             3.669x         5.323x          9.708x

**So 3d905c37 did not add a constant — it introduced the SUPERLINEARITY.**
a peer session predicted a constant-only move and withdrew it on this data; the
prediction that survived was that the exponent changes. Fitting log(ratio)
against log(N) over those three points gives `ratio ≈ e^-2.009 · N^0.594`, which
predicts **13.11x at N = 2,234** against **13.2x measured** on W-361's own input
`pariksha_r` — 0.7% out of sample. *Caveat kept deliberately:* that is a fit
extrapolated 1.9x past its largest point onto a DIFFERENT SOURCE, so content
varies as well as size. Strong evidence, not proof.

**AND IT RESOLVES THE `1860a18f` PUZZLE ABOVE.** `1860a18f` reads
1,280,993,383 / 4,283,753,460 / 15,579,164,380 — within **0.39%, 0.29% and
0.16%** of `3d905c37` at the three sizes, with exponents differing by 0.0010 and
0.0019. Two trees that do not contain each other cannot agree that closely by
accident: they are running the SAME growth path. `1860a18f`'s line forked at
`ab3d0d74` and "never received the speedup", so it has the slow path natively,
and `3d905c37` RE-INTRODUCED that path onto the fast line by inlining it per
append site. **The fast tier was the exception, not the baseline** — which is
why the earlier note here could not attribute the exponent to the commit and was
right not to try.

Consequence for the rows: **W-361, W-346 and W-356 are one defect**, and
W-356's loop-only inlining is the remedy for all three.

### And then confirmed on W-361's OWN input, which needed no fit at all

    tree        pariksha_r (2,300 lines, 2,234 literal statements)
    e22a87a3       3,729,504,906
    3d905c37      49,208,073,454      ratio 13.1943x

**The whole-window Mac pair was 13.1944x** (`51598b3c` 3,729,479,156 against
`34c9712a` 49,208,290,125). So ONE parent/child pair reproduces the entire
fourteen-commit window **to three decimals** — 0.0011% apart, recomputed here.
The twelve later commits contribute 216,671 steps, **0.0004%** of the slow
figure; the cross-host offset on the fast tier is 25,750 steps, 0.00069%.

That is the strongest form available: not a fit, not a bisect, but a
single-candidate measurement on the row's own observable, with everything after
the candidate sitting in the fourth decimal.

**W-361's four-probe bisect was never run, and did not need to be.** It was filed
to search a fourteen-commit window; the answer was already in this directory's
preserved data plus one parent/child pair, and the first leg settled it. The
probes paid for themselves after being nearly lost to session scratch.

### The acceptance check for the fix is binary

`pariksha_t10` (10,346 lines) **must compile again** on the fixed tree. Today
`34c9712a` cannot compile it at all — 268.7 G steps, then a load-dependent
refusal and an empty image — because this commit's per-site image growth pushed
the compiler's own size ceiling from above 11,017 lines to below 10,346. A
restored ceiling is a pass/fail with no figure to argue about, and it is
independent of both the n^1.3→n^1.9 exponent and the GPU-driver project's `seema.t1` staying at
~165 M executed.

## `bisect.tsv` is the record of an ERROR, not data

Seven legs all reading ~1.28 G, because every one forked at `ab3d0d74` and none
carried the speedup; `046718da` read **identically to its parent**, which was
misread as innocence. A bisect whose legs do not differ is measuring its fork
point. Kept so the mistake stays visible.

## W-355 / W-359 probes from a peer session (added 2026-10-02)

`w355_len.t1probe`, `w355_read.t1probe`, `w359_two_levels.t1probe`, `w359_grow127/128/129.t1`, with
both-engine results in `w355_w359_results.tsv`. W-355's previous citation
(`/path/to/scratch/probe0/`) no longer
exists; the worktree was removed. These are those probes recreated from their exact
sources and RE-MEASURED on 096f46e9, not copied results.
