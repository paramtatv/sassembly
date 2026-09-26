# The hop ladder across all 20 sources — a W-279 measurement

**THERE IS NO ROW W-283 AND THIS FILE USED TO CLAIM ONE.** Its title read
"W-283 prediction" and the highest row is W-282, on main and on every branch.
I invented an id for work that had none, and a phantom id in live text is worse
than no id: it reads as real to whoever greps for it and gets planned against.
The parked ladder is a **measurement under W-279**, not a row of its own — the
standing rule is that no new row opens without the owner.

The commit subject that first carried the phantom (`6219bc96`) will say W-283
forever; history is not rewritten for this. This paragraph is what a reader
finds instead.


**Written before the run, at `origin/main 9d497745`.** Registered so the
measurement can falsify it rather than be described by it.

## The members, not the count

| predicted | files |
|---|---|
| `Hops(5)` | `ashtaka.t1` (measured), `lex.t1`, `kosha.t1`, `ast.t1`, `vastu.t1` |
| `CouldNotRun` | `lib.t1` — declares no `मण्डलम्`, so there is no module name to pass |
| `Hops(0)` | the other 14: `artha` `encode` `ir` `nidana` `parse` `samyojana` `sanchaya` `sanskrit_text` `shrinkhala` `unparse` `utsarjana` `vakyavibhaga` `vishlesana` `yantrotsarjana` |

5 + 1 + 14 = 20.

## The basis

`STATE.md:263` records **4 of 18 modules reach EMIT** — `अष्टक`, `पदविभाग`,
`कोश`, `वास्तु`. Anything that does not reach emit cannot reach hop 1, so the
14 land at hop 0 by that table alone. The open question is whether the four that
DO emit carry through assemble/link/load/run the way `ashtaka` does.

## I DISAGREE WITH THE STATED MODEL, AND THIS IS THE FALSIFIABLE PART

The brief says the ladder is **monotone by import depth**. The counterexample is
already in the record and needs no run:

| file | imports | routines | declarations | emits? |
|---|---|---|---|---|
| `kosha.t1` | **1** | 4 | 11 | **YES** |
| `sanskrit_text.t1` | **1** | 37 | 94 | no |
| `utsarjana.t1` | **1** | 32 | 59 | no |
| `vishlesana.t1` | **1** | 35 | 44 | no |

Four files at import depth 1; one emits and three do not. Import depth cannot
separate them. What does separate them is **size and the feature set a source
uses** — `kosha` is the smallest module in the corpus at 11 declarations, and the
three that fail are 4-9× larger. `ashtaka` (16 decls) and `lex` (28) are likewise
small; `ast`/`vastu` are data-only, which is fewer features still.

**So the prediction is: hop reach tracks the FEATURES a source uses, and import
depth correlates only because deeper files are also bigger.** If the run shows a
clean monotone-by-depth distribution with `kosha` as the sole anomaly, the depth
model survives and mine is wrong. If the failures cluster by size or by which
constructs appear, mine holds.

## Where I am least confident

`ast.t1` and `vastu.t1` emit and build objects (W-279), but they are **data-only
— zero routines**. Whether a module with no code links into a loadable image and
runs is untested. **A plausible alternative is `Hops(3)`**: the object builds, the
image writer refuses or produces something `load_elf` rejects. I predict 5 and
would not be surprised by 3, and the ladder distinguishes them without
reconstruction.

`lex.t1` emits 128,356 octets — 2.9× `ashtaka`. If any encoder limit exists it
shows there first, so `Hops(2)` is the alternative worth naming.

## What the run cannot settle

Nothing here moves the tracked denominator on its own. Twenty sources measured is
a **distribution**, not a claim that the corpus self-hosts; a source at hop 5 in
isolation has not been compiled *by the chain compiling itself*.
