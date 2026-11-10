// The browser assembler emits the page's OWN ELF, octet for octet — `F-004`.
//
// Driven by `tools/check-sadhana-glue-live.sh`, which assembles the reference
// ELFs natively and locates an already-built wasm module. See that file for why
// it does not build one and for the 78-vs-77 distinction.
//
// ## WHY BYTES AND NOT A CONSTANT
//
// `check-sadhana-glue.mjs` arm 11 compares `Sadhana.BARE_METAL` against
// `kosha::LOAD_ADDRESS`. That caught the divergence it was written for, and it is
// two numbers agreeing — not the claim. A glue that truncated a Devanagari source
// at character count, lost the high half of a 33-bit base, or handed back a view
// of the previous answer keeps every constant correct and changes the ELF. The
// stand-in arms cover each of those against a MODULE THAT IS NOT THE ONE SHIPPED;
// this one asks the real module and compares against what `sadhana` actually
// emitted seconds earlier.
//
// ## THE CASE THAT MUST STILL BE REFUSED, and it is the whole reason for arm R
//
// An identity assertion is the easiest check in the world to make vacuously true:
// hand it the same bytes twice and it is green forever. So the SAME proof is also
// assembled at the OpenSBI payload address — `0x80200000`, the value
// `Sadhana.BARE_METAL` held until 2026-10-05 — and that result MUST NOT match the
// page's ELF. Measured: 904 octets either way, sha256 `a28bccab…` against
// `40324925…`. If arm R ever goes green the identity arms above mean nothing,
// because the base stopped reaching the module at all.
//
// Usage: check-sadhana-glue-live.mjs <module.wasm> <dir-of-reference-elfs>
import { readFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { Sadhana, loadSadhana } from '../web/sadhana.mjs';

const [, , wasmPath, refDir] = process.argv;
if (!wasmPath || !refDir) {
  console.error('usage: check-sadhana-glue-live.mjs <module.wasm> <dir-of-reference-elfs>');
  process.exit(2);
}

let failed = 0;
const fail = (what, saw) => {
  console.error(`  FAIL  ${what}`);
  if (saw !== undefined) console.error(`        saw: ${saw}`);
  failed++;
};
const ok = (what) => console.log(`  ok    ${what}`);
const sha = (b) => createHash('sha256').update(b).digest('hex');

// A FLOOR, for the reason `check-glue-collision.sh:81` has one: a reader that
// found no reference ELF would run no arm and print nothing but a banner, which
// is the failure mode an identity check is most likely to have.
const elfs = readdirSync(refDir).filter((f) => f.endsWith('.elf')).sort();
if (elfs.length < 3) {
  console.error(`  FAIL  found ${elfs.length} reference ELF(s) in ${refDir}; the wrapper writes 3.`);
  console.error('        A reader with no subject would print a green banner, so this is its own red.');
  process.exit(1);
}

const sa = await loadSadhana(readFileSync(wasmPath));
// The base each kind is linked at, FROM THE GLUE, because that is what is under
// test — the wrapper passes the same two to the native CLI from the spec and from
// `kosha`'s default, and the two paths meeting is the assertion.
const base = (name) => (name === 'atithi' ? Sadhana.APP_LOAD : Sadhana.BARE_METAL);

for (const f of elfs) {
  const name = f.replace(/\.elf$/, '');
  const native = readFileSync(join(refDir, f));
  const src = readFileSync(new URL(`../spec/${name}.sas`, import.meta.url), 'utf8');
  const r = sa.assemble(src, base(name));
  // A REFUSAL IS NOT A MISMATCH AND MUST NOT BE REPORTED AS ONE. The glue hands
  // the assembler's own diagnostics through unchanged; printing "the bytes
  // differ" over a source the module would not accept names the wrong defect.
  if (!r.ok) {
    fail(`${name}: the module refused a source \`sadhana\` just assembled natively`,
         JSON.stringify(r.error.slice(0, 160)));
    continue;
  }
  const mine = sha(r.elf), theirs = sha(native);
  if (mine !== theirs) {
    fail(`${name}: the browser assembler's ELF is the native one`,
         `glue ${mine.slice(0, 16)} (${r.elf.length} octets) vs native ` +
         `${theirs.slice(0, 16)} (${native.length} octets) at base ` +
         `0x${base(name).toString(16)}` +
         (r.elf.length === native.length
           ? ' — SAME LENGTH, so a size check would have passed this'
           : ''));
  } else {
    ok(`${name}: assembled in the glue at 0x${base(name).toString(16)} — ` +
       `${r.elf.length} octets, sha256 ${mine.slice(0, 16)}, octet for octet the native ELF`);
  }
}

// ———— arm R: THE CASE THAT MUST BE REFUSED —————————————————————————————————
// The identity arms above are vacuous if the base never reaches the module, and
// nothing in them can tell. So the same proof at the OpenSBI payload address must
// NOT match. 904 octets either way on 2026-10-05, which is why this is a hash and
// not a length.
{
  const OPENSBI = 0x80200000;
  if (Sadhana.BARE_METAL === OPENSBI) {
    fail('arm R: the glue links a proof somewhere other than the OpenSBI address',
         `Sadhana.BARE_METAL is 0x${OPENSBI.toString(16)}, so this control compares a ` +
         'base against itself and cannot distinguish anything. The glue is the defect ' +
         'here, and check-sadhana-glue.mjs arm 11 is where it is named.');
  } else {
    const native = readFileSync(join(refDir, 'namaste.elf'));
    const src = readFileSync(new URL('../spec/namaste.sas', import.meta.url), 'utf8');
    const r = sa.assemble(src, OPENSBI);
    if (!r.ok) {
      fail('arm R: the same source still assembles at the OpenSBI address',
           'the module refused it, so the control says nothing about the base: ' +
           JSON.stringify(r.error.slice(0, 120)));
    } else if (sha(r.elf) === sha(native)) {
      fail('CONTROL BLIND: "namaste" at 0x80200000 is byte-identical to the page\'s ELF',
           `the base is not reaching the module at all — every identity arm above is ` +
           `vacuous. ${r.elf.length} octets, sha256 ${sha(r.elf).slice(0, 16)}`);
    } else {
      ok(`control: the same proof at 0x${OPENSBI.toString(16)} is NOT the page's ELF ` +
         `(${r.elf.length} octets either way, sha256 ${sha(r.elf).slice(0, 16)} vs ` +
         `${sha(native).slice(0, 16)}) — the base reaches the module, so identity means something`);
    }
  }
}

// ———— THE PROVENANCE STAMP, ON THE REAL MODULE'S OWN EXPORTS ————————————————
// `web/sadhana.mjs`'s `get rev()` has THREE answers — a 16-hex stamp, the word
// `unknown`, or `null` for a module with no `sadhana_rev_*` export at all — and
// until now NOT ONE of them was read by this pair of checks. `check-sadhana-glue.mjs`
// does not even give its stand-in the two exports, so `sa.rev` over the stand-in
// answers `null` and no arm notices; the only reader anywhere was
// `check-sassembly-page.mjs`, over a GENERATED page, which is not on this tree
// until someone has built one. So the accessor the page's provenance line is built
// on was in the state `yantra_input_alloc` was in from `1119c792`.
//
// This is the twin of `check-yantra-glue-live.mjs`'s stale-build arms and the
// reasoning is theirs, not re-derived: `Sadhana` takes an `{exports}` object, so the
// REAL instance's exports MINUS the stamp pair is a real pre-stamping module in
// every respect the branch can see — same memory, same `sadhana_assemble`, same
// source, the export gone.
//
// THE CONTROL IS FIRST AND IS NOT OPTIONAL. A getter that answered `null` for
// everything would pass every stripped arm below; the only thing that can tell is
// the unstripped reconstruction still reading the stamp.
{
  const copy = (...drop) => {
    const e = {};
    for (const k of Object.keys(sa.e)) if (!drop.includes(k)) e[k] = sa.e[k];
    return new Sadhana({ exports: e });
  };
  // `null` for a missing export, a string otherwise, and the message when it THREW
  // — three outcomes, because an accessor with two states where the truth has three
  // hides its own breakage. A throw is NOT a `null`: it is the page dying.
  const rev = (g) => { try { return g.rev; } catch (err) { return `THREW ${err.message}`; } };

  const real = rev(sa);
  if (typeof real !== 'string' || !/^([0-9a-f]{16}|unknown)$/.test(real)) {
    fail('arm S: the real module answers a stamp — 16 hex octets, or the word `unknown`',
         `${JSON.stringify(real)} — this arm has no subject, so every stripped arm ` +
         'below would pass against a getter that answers nothing at all');
  } else {
    ok(`arm S: the real module stamps itself ${JSON.stringify(real)}, read at ` +
       `sadhana_rev_ptr()/_len() out of its own memory`);
  }

  const control = rev(copy());
  if (control !== real) {
    fail('arm S: a Sadhana rebuilt over the real exports, nothing dropped, reads the ' +
         'same stamp — the control', `${JSON.stringify(control)} vs ${JSON.stringify(real)}`);
  } else {
    ok('control: the reconstruction over the untouched exports reads the SAME stamp, ' +
       'so a null below is about the missing export and not about the rebuild');
  }

  // Both halves gone: a `sadhana_wasm.wasm` from before stamping existed. IT STILL
  // ASSEMBLES — that is the trap, and it is asserted rather than assumed, because a
  // reconstruction that had broken the assembler would make `null` prove nothing.
  const both = copy('sadhana_rev_ptr', 'sadhana_rev_len');
  if (rev(both) !== null) {
    fail('arm S: the real module MINUS its stamp pair answers `null`, not a stamp and ' +
         'not a throw', JSON.stringify(rev(both)));
  } else {
    const r = both.assemble(readFileSync(new URL('../spec/namaste.sas', import.meta.url), 'utf8'));
    if (!r.ok) {
      fail('arm S: and an unstamped module STILL ASSEMBLES',
           'it refused the source, so the rebuild broke the assembler and the `null` ' +
           'above is evidence about this reader: ' + JSON.stringify(r.error.slice(0, 120)));
    } else {
      ok(`arm S: minus BOTH stamp exports it answers \`null\` and still assembles ` +
         `(${r.elf.length} octets) — which is the trap: agreement is not provenance`);
    }
  }

  // ———— THE CASE THAT MUST STILL BE REFUSED: HALF THE PAIR ——————————————————
  // The guard read `typeof this.e.sadhana_rev_ptr !== 'function'` ALONE and then
  // called `sadhana_rev_len()`, so the three answers were three only while the pair
  // was all-or-nothing. Measured here on 2026-10-05, before the fix: dropping
  // `sadhana_rev_len` by itself threw `this.e.sadhana_rev_len is not a function` out
  // of the accessor whose whole contract is to answer `null` instead — and the page
  // reads `s.rev` at top level, so that throw is a blank tab rather than a named
  // gap. EACH HALF SEPARATELY, because one `typeof` repaired on one side is exactly
  // the defect this is here for.
  for (const half of ['sadhana_rev_ptr', 'sadhana_rev_len']) {
    const got = rev(copy(half));
    if (got !== null) {
      fail(`arm S: minus \`${half}\` ALONE the glue answers \`null\``,
           `${JSON.stringify(got)} — the stamp probe covers only the other half of ` +
           'the pair, so half a pre-stamping module is a JS TypeError and not a ' +
           'provenance gap. web/sadhana.mjs `get rev()` is where it is named.');
    } else {
      ok(`arm S: minus \`${half}\` alone it is \`null\` too — the probe covers the PAIR`);
    }
  }
}

if (failed) {
  console.error(`\n${failed} check(s) failed: the browser assembler does not emit the page's own bytes.`);
  process.exit(1);
}
console.log(`
all arms green: the real sadhana-wasm module, driven through web/sadhana.mjs,
emits the SAME OCTETS as native \`sadhana\` for every program the page carries —
both proofs at kosha::LOAD_ADDRESS and the application at the spec's address —
and a proof at the OpenSBI address is refused as different, so the base is
reaching the module and the identity is not a comparison of bytes with themselves.`);
