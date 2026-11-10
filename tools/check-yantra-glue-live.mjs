// The REAL `yantra-wasm` answers the way `check-yantra-glue.mjs`'s stand-in says
// it does — task `F-004`.
//
// ## Why this exists beside a check that is already green
//
// `check-yantra-glue.mjs` drives a STAND-IN module, because the properties it
// pins — which exports the glue calls, with what lengths, and what it leaves
// behind between two runs — are not observable from a real module's return
// value. The cost of a stand-in is that it can be WRONG ABOUT THE REAL ONE: a
// glue calling an export that does not exist, or a `plan_input` port that
// answers a code the real one does not, passes there and breaks on a page.
//
// This closes that, and it is the arm that measures the defect NATIVELY rather
// than by construction: the last two checks run the same two-program sequence
// THROUGH the glue and AROUND it, and the one that goes around must be REFUSED,
// because the module statics still hold the first program's slab.
//
// It needs a built module and will not build one — see the wrapper.
import { readFileSync } from 'node:fs';
import { load, Yantra } from '../web/yantra.mjs';

// TWO PROGRAMS AND NOT ONE, and the second is only here for the HIGH WATER. One
// subject cannot tell a counter from a constant: `namaste` stores nothing to RAM, so
// its honest water is `0` — which is the state `#water()` exists to keep apart from
// `null`, and also the state a broken export that answers a fixed `0` would pass.
// `<storer.elf>` is a program whose water is NOT zero, so the pair pins the reading
// as the PROGRAM's rather than the module's.
const [, , wasmPath, elfPath, storerPath] = process.argv;
if (!wasmPath || !elfPath || !storerPath) {
  console.error('usage: check-yantra-glue-live.mjs <module.wasm> <program.elf> <storer.elf>');
  process.exit(2);
}
const y = await load(readFileSync(wasmPath));
const elf = readFileSync(elfPath);
const storer = readFileSync(storerPath);
// `run()`'s default, named because two water arms are statements ABOUT it.
const RAM = 1 << 20;
const enc = new TextEncoder();
const NAME = 'परीक्षा';
let bad = 0;
const say = (ok, what, extra) => {
  console.log(`  ${ok ? 'ok  ' : 'FAIL'}  ${what}${extra === undefined ? '' : ` — ${extra}`}`);
  if (!ok) bad++;
};

// The exports the glue names must EXIST. `no_mangle` is what keeps them in a
// release cdylib, and nothing else in the tree asserts they survived it.
for (const n of ['yantra_input_alloc', 'yantra_input_name_alloc',
  'yantra_steps_known', 'yantra_steps_lo', 'yantra_steps_hi',
  'yantra_water_known', 'yantra_water',
  // The stamp pair, which this list did not name until 2026-10-05 even though
  // `get rev()` calls BOTH — see the provenance block at the foot of this file.
  'yantra_rev_ptr', 'yantra_rev_len'])
  say(typeof y.e[n] === 'function', `the real module exports ${n}`);

// A run with no input is unchanged — this is the page's existing behaviour, and
// the glue now calls both allocs with 0 on the way to it.
const plain = y.run(elf);
say(plain.code === 0, 'no input: the program still runs', `code ${plain.code} ${JSON.stringify(plain.out)}`);
say(plain.out.length > 0, 'and still prints what it printed', JSON.stringify(plain.out));

// ———— the step count, on the real module ————————————————————————————————
// `#steps` recombines two `u32`s and consults `_known` first. The stand-in says a
// run that executed has a reading; here is whether the REAL one does, and whether
// the glue's arithmetic matches the raw halves it read them from.
say(typeof plain.steps === 'number', 'a real run has a step reading', `${plain.steps}`);
say(plain.steps > 0, 'and it is not zero — this program executed instructions');
{
  const raw = (y.e.yantra_steps_hi() >>> 0) * 2 ** 32 + (y.e.yantra_steps_lo() >>> 0);
  say(plain.steps === raw, 'the glue\'s figure is the raw halves recombined', `glue ${plain.steps} raw ${raw}`);
}
// The same image twice must retire the same count. This is `T-102` as the glue sees
// it, and it also catches a reading that is a stale static rather than this run's.
say(y.run(elf).steps === plain.steps, 'the same program retires the same count twice');

// ———— the high water, on the real module ——————————————————————————————————
// `#water()` is the same three states as `#steps()` with one difference, and the
// difference is the whole reason this block cannot be a copy of the one above: a
// water of `0` IS A READING. `namaste` writes only to the UART and the finisher,
// both MMIO, so NOTHING it does passes through the sink's `stored` arm — its honest
// water is zero, and the stand-in cannot prove the real module agrees.
{
  // The raw export is a static over the LAST `yantra_run`, so this re-runs rather
  // than reading `plain`'s figure against statics two runs newer.
  const r = y.run(elf);
  say(typeof r.water === 'number', 'a real run has a high-water reading', `${r.water}`);
  say(r.water === 0, 'and namaste STORES NOTHING to RAM, so the honest reading is 0', `${r.water}`);
  say(y.e.yantra_water_known() === 1,
    'the module says it HAS that reading — 0 is a value here, not the absence of one');
  say(r.water === (y.e.yantra_water() >>> 0), 'the glue\'s figure is the raw export',
    `glue ${r.water} raw ${y.e.yantra_water() >>> 0}`);
  // The two readings share a shape and nothing else. A glue that returned the step
  // count under the name `water` would pass every arm above it.
  say(r.water !== r.steps, 'and it is NOT the step count', `water ${r.water} steps ${r.steps}`);
}

// ———— A SECOND PROGRAM, BECAUSE ONE SUBJECT CANNOT TELL A COUNTER FROM A CONSTANT ————
// Every arm above is green for a `yantra_water` that always answers 0 with
// `_known` always 1. This is the arm that is not: the storer's water is nonzero,
// namaste's is zero, and no constant satisfies both. Run on the SAME instance and
// with namaste AFTER it, so the figure is also proved to be THIS run's rather than
// a static the previous run left behind — the `WATER` residue `lib.rs:206` clears.
{
  const st = y.run(storer);
  say(st.code === 0, 'the storer runs clean on the real module', `code ${st.code} ${st.halt}`);
  say(st.water > 0, 'a program that STORES reads a NONZERO high water', `${st.water}`);
  say(st.water <= RAM, 'and it is inside the RAM the run was given', `${st.water} <= ${RAM}`);
  say(st.water !== st.steps, 'and it is not that program\'s step count either',
    `water ${st.water} steps ${st.steps}`);
  const back = y.run(elf);
  say(back.water === 0, 'and namaste after it reads 0 AGAIN — the water is this run\'s',
    `${back.water} after ${st.water}`);
}

// A HOSTED run has no counter to give. `yantra_host` clears `STEPS` (`lib.rs:355`),
// so this is null WHATEVER code comes back — `namaste.sas` is a boot proof and will
// not load as an application at all, which is exactly the case where handing back
// the previous `yantra_run`'s number would be a lie about a run that never happened.
{
  const h = y.host(elf);
  say('steps' in h, 'a hosted result still HAS the field, so a page reads null not undefined');
  say(h.steps === null, 'and it is null — host::Hosted carries no counter', `code ${h.code}`);
  // `yantra_host` clears `WATER` beside `STEPS` (`lib.rs:437`). This is the state a
  // glue folding `0` into `null` would be INDISTINGUISHABLE from above, which is why
  // the zero arm and this one are both required.
  say('water' in h, 'and the water field is present on a hosted result too');
  say(h.water === null, 'and IT is null — a hosted run measured no water at all', `${h.water}`);
}

// An unnamed slab: the real `plan_input`, not the stand-in's port of it.
const unnamed = y.run(elf, { input: enc.encode('x') });
say(unnamed.code === 13, 'an unnamed slab is refused, code 13', `code ${unnamed.code}`);
say(/module name is passed, never guessed/.test(unnamed.halt), 'and says so in the native words');
// `yantra_run` clears `STEPS` before `plan_input` and sets it only after `m.run`, so a
// REFUSAL has no reading. The stand-in was written the other way round first and
// reported the previous run's count here; this is the arm that settled it.
say(unnamed.steps === null, 'a refused run reports no step reading', `${unnamed.steps}`);
// Same for the water, and NOT the same argument: a refused run really did store
// nothing, so `0` would be the arithmetically true figure and still a lie — it would
// say a run happened and touched nothing where no run happened at all.
say(unnamed.water === null, 'and no high-water reading either — not 0', `${unnamed.water}`);

// A NAMED slab on a program that declares no input globals: `inject` refuses it,
// also 13 — and the WORDS must differ, or one code is hiding two causes.
const named = y.run(elf, { input: enc.encode('x'), inputName: NAME });
say(named.code === 13, 'a named slab this program cannot take is refused, code 13', `code ${named.code}`);
say(named.halt !== unnamed.halt, 'and for a visibly different reason');
say(/SASINPUT/.test(named.halt), 'naming the tag it looked for');
say(named.water === null, 'and this refusal reports no high water either', `${named.water}`);

// THE CARRY-OVER, measured both ways on the real module.
const after = y.run(elf);
say(after.code === 0, 'the run after a fed one is clean — the glue cleared the slab', `code ${after.code}`);
say(after.out === plain.out, 'and prints what it printed before any input existed');
say(after.steps === plain.steps, 'and retires what it retired — the refusals left no residue');
say(after.water === plain.water, 'and touches what it touched', `${after.water} vs ${plain.water}`);

y.run(elf, { input: enc.encode('x'), inputName: NAME });
// Around the glue, the way `web/yantra.mjs` did before this change: write the
// image, call `yantra_run`, touch no input export.
new Uint8Array(y.e.memory.buffer).set(elf, y.e.yantra_alloc(elf.length));
const around = y.e.yantra_run(1 << 20, 1_000_000);
say(around === 13, 'AROUND the glue the previous run\'s slab is still there, code 13', `code ${around}`);
say(after.code !== around, 'so the two paths DISAGREE on one module — the clearing is load-bearing');

// ———— THE STALE-BUILD REFUSALS, ON THE REAL MODULE'S OWN EXPORTS ——————————
// `check-yantra-glue.mjs` arms 11 and 18 delete the counter exports from the
// STAND-IN and require the glue to THROW rather than report `null`. That throw
// read as unfalsifiable here: a real module that HAS the export cannot reach the
// branch, so the two arms were pinned by the stand-in's port of the ABI alone —
// which is the one thing this file exists to stop being true of anything.
//
// It is not unfalsifiable. `Yantra` takes an `{exports}` object, so the REAL
// instance's exports MINUS one pair is a real stale module in every respect the
// branch can see: same memory, same `yantra_run`, same program, one export gone.
//
// THE CONTROL IS FIRST AND IS NOT OPTIONAL. A `Yantra` rebuilt over the
// UNSTRIPPED exports must still run clean, or a throw below would be evidence
// about the reconstruction rather than about the missing pair.
{
  const copy = (...drop) => {
    const e = {};
    for (const k of Object.keys(y.e)) if (!drop.includes(k)) e[k] = y.e[k];
    return new Yantra({ exports: e });
  };
  // The run itself is expected to SUCCEED in all three cases — `yantra_run`
  // returns 0 and `#read` then throws on the way out. The defect being caught is
  // a report about a run that happened, not a failure to run.
  const threw = (g) => { try { g.run(elf); return null; } catch (err) { return err.message; } };

  say(threw(copy()) === null,
    'a glue rebuilt over the real exports, nothing dropped, still runs — the control');

  const noWater = threw(copy('yantra_water_known', 'yantra_water'));
  say(noWater !== null, 'the real module MINUS its water exports is REFUSED, not reported as null');
  say(/predates the high-water/.test(noWater || ''),
    'and the words name the high-water counter', JSON.stringify(noWater));

  // `yantra_steps_known` alone, with `_lo`/`_hi` left in place: the gate the glue
  // consults is the one that has to be missing for this to fire.
  const noSteps = threw(copy('yantra_steps_known'));
  say(noSteps !== null, 'and minus `yantra_steps_known` it is refused too');
  say(/predates the step counter/.test(noSteps || ''),
    'with the STEP counter named instead', JSON.stringify(noSteps));
  // Two stale builds are two diagnoses. One message for both would send whoever
  // reads it to the wrong half of `lib.rs`, and both arms above would still pass.
  say(noWater !== noSteps, 'and the two stale builds do NOT report the same cause');
}

// ———— THE PROVENANCE STAMP, AND THE PROBE COVERED HALF THE PAIR ———————————
// `web/yantra.mjs`'s `get rev()` is the engine half of the page's provenance
// line, and NOTHING here read it: `yantra_rev_ptr` and `yantra_rev_len` were
// absent from the export list above and from every arm, so the three answers the
// accessor promises (a 16-hex stamp, `unknown`, `null` for a module with no such
// export) were asserted by no instrument on this tree. The only reader anywhere
// was `check-sassembly-page.mjs`, over a GENERATED page.
//
// AND IT WAS THE TWIN DEFECT OF `web/sadhana.mjs`'s, which is why both moved in
// one cycle. The guard tested `yantra_rev_ptr` and then called `yantra_rev_len()`
// unguarded. Measured here before the fix: the real exports minus
// `yantra_rev_len` ALONE threw `this.e.yantra_rev_len is not a function` — a JS
// TypeError out of the accessor written to answer `null`, which the page reads at
// top level, so it is a blank tab rather than a named gap. The reconstruction
// mechanism is the stale-build block's above and is not re-derived.
{
  const copy = (...drop) => {
    const e = {};
    for (const k of Object.keys(y.e)) if (!drop.includes(k)) e[k] = y.e[k];
    return new Yantra({ exports: e });
  };
  // THREE OUTCOMES, because a reader with two where the truth has three hides its
  // own breakage: a string, `null`, or the message when it THREW. A throw is not a
  // `null`.
  const rev = (g) => { try { return g.rev; } catch (err) { return `THREW ${err.message}`; } };

  const real = rev(y);
  say(typeof real === 'string' && /^([0-9a-f]{16}|unknown)$/.test(real),
    'the real module stamps itself — 16 hex octets, or the word `unknown`', JSON.stringify(real));
  // THE CONTROL, AND IT IS NOT OPTIONAL: a getter hard-wired to `null` would pass
  // every stripped arm below, and only the untouched reconstruction can tell.
  say(rev(copy()) === real,
    'and a Yantra rebuilt over the real exports, nothing dropped, reads the SAME stamp '
      + '— the control, so a null below is about the missing export', JSON.stringify(rev(copy())));

  // Both gone: a `yantra_wasm.wasm` from before stamping existed. IT STILL RUNS,
  // asserted and not assumed — that is the trap the stamp exists for, and a
  // reconstruction that had broken the engine would make the `null` prove nothing.
  const both = copy('yantra_rev_ptr', 'yantra_rev_len');
  say(rev(both) === null,
    'minus BOTH stamp exports it answers `null`, not a stamp and not a throw',
    JSON.stringify(rev(both)));
  say(both.run(elf).code === 0,
    'and an unstamped module STILL RUNS — agreement is self-consistency, not provenance');

  // EACH HALF SEPARATELY. One `typeof` repaired on one side is exactly the defect.
  for (const half of ['yantra_rev_ptr', 'yantra_rev_len']) {
    say(rev(copy(half)) === null,
      `minus \`${half}\` ALONE it is \`null\` too — the probe covers the PAIR`,
      JSON.stringify(rev(copy(half))));
  }
}

if (bad) {
  // NOT ALL OF THESE ARE THE STAND-IN'S. The provenance arms above have no
  // counterpart in `check-yantra-glue.mjs` at all — they are this file's own
  // reading of `web/yantra.mjs` against the real module — so a banner that
  // blamed the stand-in for every red would send whoever reads it to a file
  // with nothing in it on the subject.
  console.error(`\n  FAIL  ${bad} check(s) — web/yantra.mjs disagrees with the real module, or the`);
  console.error('        stand-in in check-yantra-glue.mjs is wrong about it. The arm names which.');
  process.exit(1);
}
console.log('the stand-in agrees with the real module on every shape it pins.');
