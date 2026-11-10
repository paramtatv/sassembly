// The browser glue agrees with `sadhana-wasm` about the ASSEMBLER — task `F-004`.
//
// `tools/check-yantra-glue.mjs` is the twin of this file and covers `web/yantra.mjs`.
// It was written because `yantra_input_alloc` landed at `1119c792` and the glue
// called it from nowhere: "a capability no caller can name is not a capability."
//
// THE SAME HOLE WAS STILL OPEN ON THE OTHER HALF OF THE PAIR, and this file is why
// it is not. MEASURED 2026-10-05: `web/sadhana.mjs` is imported by NOTHING, inlined
// by NOTHING, and read by NO check. `build-sassembly-web.sh:246` splices exactly one
// module into the page — `*@@GLUE@@*) sed 's/^export //' web/yantra.mjs` — and
// `web/sassembly.template.html` declares `@@WASM@@`, `@@PROGRAMS@@` and `@@GLUE@@`
// and no fourth placeholder. So `web/sadhana.mjs` (80 lines) is the ONLY caller
// anywhere of `sadhana-wasm`'s six exports, and until now no instrument could say
// whether the two agreed. `cargo` does not read it; `clippy` does not read it.
//
// ## Why a stand-in module and not the real wasm
//
// Same reason as the twin: the thing under test is the GLUE's half of the protocol
// — which exports it calls, with WHAT LENGTH, in what order, what it copies and what
// it hands back on a refusal. None of that is observable from a real module's return
// value. A glue that slices the source short still gets an ELF back; it is just the
// wrong program. The stand-in records the calls, so the property can be ASSERTED.
//
// The stand-in is held to `crates/sadhana-wasm/src/lib.rs`: a real
// `WebAssembly.Memory`, three statics that OUTLIVE the call that set them the way
// `INPUT`/`OUTPUT`/`ERROR` do, `sadhana_alloc` filling `len` NUL octets and handing
// back the pointer, `sadhana_assemble` taking the BYTE length and reassembling the
// base from `(hi << 32) | lo`, and 0-with-OUTPUT / 1-with-ERROR as the two answers.
//
// ## The CONTROLS are what make the arms mean anything
//
// A stand-in can agree with a broken glue. So each arm that could be vacuous also
// runs against a glue with ONE thing wrong, and MUST go red there:
//   `preByteLen`  passes `source.length` where the protocol wants the UTF-8 byte
//                 length. Every Devanagari source is 3 octets per character, so this
//                 is not a corner case — it truncates every real input to a third.
//   `preView`     hands back a view over the module's buffer instead of a copy.
//   `preGeneric`  replaces the assembler's diagnostics with the word "failed",
//                 which `web/sadhana.mjs:70` names as the one unacceptable answer.
// If a control passes, this file cannot see the difference and says so.

import { readFileSync } from 'node:fs';
import { Sadhana, loadSadhana } from '../web/sadhana.mjs';

const enc = new TextEncoder();
const dec = new TextDecoder();
let failed = 0;
const fail = (what, saw) => {
  console.error(`  FAIL  ${what}`);
  if (saw !== undefined) console.error(`        saw: ${saw}`);
  failed++;
};
const ok = (what) => console.log(`  ok    ${what}`);
const eqBytes = (a, b) => a.length === b.length && a.every((v, i) => v === b[i]);

// ———— the stand-in module ————————————————————————————————————————————————
// Mirrors `crates/sadhana-wasm/src/lib.rs`. `log` is what the glue is judged on.
// ॥ THE STAMP LIVES AT A NON-ZERO OFFSET INSIDE A FILLER REGION, AND THAT IS THE ARM ॥
// `REV` is deliberately NOT a plausible git-ish stamp a hard-wired glue could have
// guessed, and `REV_AT` is deliberately not where anything else in this module is:
// the filler around it is `Z`, so a read at the wrong PLACE comes back as `Z`s and a
// read of the wrong LENGTH comes back with one. Nothing in the real module pins
// either — `sadhana-wasm` places the stamp wherever the linker put the static, and a
// glue reading a fixed offset or `len+1` still answers 16 plausible characters
// against it. That is the half `check-sadhana-glue-live.mjs` CANNOT reach, and the
// reason the pair is here as well as there.
const REV = 'f00dcafebeef1234';
const REV_FILL_AT = 65536;        // the second page, clear of `bump` and of `OUT_AT`
const REV_FILL_LEN = 512;
const REV_AT = REV_FILL_AT + 42;  // not a page edge, not 0, not where a buffer lands
const REV_FILLER = 0x5a;          // 'Z'

function standIn({ refuse = null, rev = REV, revExports = 'both' } = {}) {
  const memory = new WebAssembly.Memory({ initial: 2 });
  const u8 = () => new Uint8Array(memory.buffer);
  let bump = 1024; // 0 is reserved: an offset of 0 would read as "no buffer".
  const log = [];
  // Storage that PERSISTS past the call that set it, because that is what a
  // `static Mutex<Vec<u8>>` does. +8 so two adjacent buffers cannot overlap.
  const put = (bytes) => {
    const ptr = bump;
    bump += bytes.length + 8;
    u8().set(bytes, ptr);
    return { ptr, len: bytes.length };
  };
  let input = { ptr: 0, len: 0 };
  let output = { ptr: 0, len: 0 };
  let error = { ptr: 0, len: 0 };
  // OUTPUT GETS A FIXED REGION AND THE OTHER TWO DO NOT, AND THAT IS NOT TIDINESS.
  // `web/sadhana.mjs:93` states the premise the copy exists for in so many words —
  // "the next call reuses this memory" — so the stand-in has to reuse it. THE FIRST
  // VERSION OF THIS FILE PUT OUTPUT ON THE BUMP ALLOCATOR TOO, and arm 4's own
  // control caught it immediately: `CONTROL BLIND: "arm 4" passes against preView`.
  // A bump allocator that never hands the same octets out twice makes a view over
  // the previous answer indistinguishable from a copy of it, so the arm asserted
  // nothing. Reported rather than silently repaired because it is the reason the
  // controls are here at all.
  const OUT_AT = 65536 - 512; // inside the 2-page memory, clear of `bump`
  const putOut = (bytes) => {
    u8().fill(0, OUT_AT, OUT_AT + 512);
    u8().set(bytes.subarray(0, 512), OUT_AT);
    return { ptr: OUT_AT, len: Math.min(bytes.length, 512) };
  };
  const exports = {
    memory,
    // `input.clear(); for _ in 0..len { input.push('\0') }` — `len` NUL octets,
    // then `as_mut_ptr()`. The NUL fill is load-bearing: it is what makes a
    // SHORT write from the glue visible as NULs rather than as stale source.
    sadhana_alloc(len) {
      log.push(['alloc', len]);
      input = put(new Uint8Array(len));
      return input.ptr;
    },
    // `let source = &input[..len]` — a BYTE slice of a `String`. A `len` past
    // what `alloc` reserved is a Rust panic, so the stand-in throws rather than
    // reading octets the module could not have read.
    sadhana_assemble(len, base_lo, base_hi) {
      if (len > input.len) throw new RangeError(`byte index ${len} out of range for a ${input.len}-octet source`);
      const source = dec.decode(u8().slice(input.ptr, input.ptr + len));
      const base = (BigInt(base_hi >>> 0) << 32n) | BigInt(base_lo >>> 0);
      log.push(['assemble', len, base, source]);
      output = { ptr: 0, len: 0 }; // out.clear()
      error = { ptr: 0, len: 0 };  // error.clear()
      if (refuse) {
        error = put(enc.encode(refuse));
        return 1;
      }
      // The "ELF": the base and the source the module actually saw, so an arm
      // can assert BOTH arrived whole rather than that something came back.
      output = putOut(enc.encode(`ELF base=0x${base.toString(16)} src=${source}`));
      return 0;
    },
    sadhana_out_ptr: () => output.ptr,
    sadhana_out_len: () => output.len,
    sadhana_err_ptr: () => error.ptr,
    sadhana_err_len: () => error.len,
  };
  // `sadhana_rev_ptr`/`sadhana_rev_len` — `crates/sadhana-wasm/build.rs` seals the
  // stamp in as a `&'static str`, so the pair is a FIXED (ptr, len) over octets that
  // were there before any call and are never rewritten. `revExports` deletes one half
  // or both, because the glue's contract has three answers and the third is reached
  // only by a module that does not have the export.
  {
    const stamp = enc.encode(rev);
    u8().fill(REV_FILLER, REV_FILL_AT, REV_FILL_AT + REV_FILL_LEN);
    u8().set(stamp, REV_AT);
    // LOGGED, because the strongest control here — a glue with the stamp hard-wired —
    // is wrong about the CALLS and not only about the value, and over a real module
    // its value is right. The log is the only instrument that can see it by content.
    if (revExports === 'both' || revExports === 'ptr') exports.sadhana_rev_ptr = () => { log.push(['rev_ptr']); return REV_AT; };
    if (revExports === 'both' || revExports === 'len') exports.sadhana_rev_len = () => { log.push(['rev_len']); return stamp.length; };
  }
  return { instance: { exports }, log, u8 };
}

// ———— the three controls, each one change from `web/sadhana.mjs` ——————————
// Spelled out rather than monkey-patched, so the diff from the real glue is
// readable in one place.
class PreByteLen extends Sadhana {
  assemble(source, base = Sadhana.BARE_METAL) {
    const bytes = enc.encode(source);
    const at = this.e.sadhana_alloc(bytes.length);
    this.mem().set(bytes, at);
    const lo = Number(BigInt(base) & 0xffffffffn) >>> 0;
    const hi = Number(BigInt(base) >> 32n) >>> 0;
    // THE ONE CHANGE: `source.length` is CHARACTERS.
    if (this.e.sadhana_assemble(source.length, lo, hi) !== 0) return { ok: false, error: 'x' };
    const p = this.e.sadhana_out_ptr(), l = this.e.sadhana_out_len();
    return { ok: true, elf: new Uint8Array(this.mem().slice(p, p + l)) };
  }
}
class PreView extends Sadhana {
  assemble(source, base = Sadhana.BARE_METAL) {
    const r = super.assemble(source, base);
    if (!r.ok) return r;
    // THE ONE CHANGE: a view over the module's buffer, not a copy.
    const p = this.e.sadhana_out_ptr(), l = this.e.sadhana_out_len();
    return { ok: true, elf: this.mem().subarray(p, p + l) };
  }
}
class PreGeneric extends Sadhana {
  assemble(source, base = Sadhana.BARE_METAL) {
    const r = super.assemble(source, base);
    // THE ONE CHANGE: the diagnostics are thrown away.
    return r.ok ? r : { ok: false, error: 'failed' };
  }
}

// ———— the stamp controls, each one change from `web/sadhana.mjs:85` ———————
// Four glues that all answer SOMETHING plausible over a real module and are each
// wrong about one clause of the protocol, plus the half-guard the live check's
// companion arm covers from the other side.
class PreRevFixedPtr extends Sadhana {
  // THE ONE CHANGE: a hard-coded offset instead of `sadhana_rev_ptr()`.
  get rev() {
    const len = this.e.sadhana_rev_len();
    return new TextDecoder().decode(this.mem().slice(REV_FILL_AT, REV_FILL_AT + len));
  }
}
class PreRevFixedLen extends Sadhana {
  // THE ONE CHANGE: 16 octets assumed, instead of `sadhana_rev_len()`. A stamp IS
  // 16 hex characters — until the build could not take one and the module says
  // `unknown`, which is 7.
  get rev() {
    const ptr = this.e.sadhana_rev_ptr();
    return new TextDecoder().decode(this.mem().slice(ptr, ptr + 16));
  }
}
class PreRevLong extends Sadhana {
  // THE ONE CHANGE: `len + 1`. The classic, and invisible against a real module
  // whose stamp happens to be followed by a NUL.
  get rev() {
    const ptr = this.e.sadhana_rev_ptr(), len = this.e.sadhana_rev_len();
    return new TextDecoder().decode(this.mem().slice(ptr, ptr + len + 1));
  }
}
class PreRevHardWired extends Sadhana {
  // THE ONE CHANGE: the module is not read at all. This is the control the live
  // arms structurally cannot have — over the real module its answer is RIGHT.
  get rev() { return 'a29d3ed0848ecc1e'; }
}
class PreRevHalfGuard extends Sadhana {
  // THE ONE CHANGE: `web/sadhana.mjs` as it was before `5d7d5c35` — the pair probed
  // on `_ptr` alone, then `_len()` called unguarded.
  get rev() {
    if (typeof this.e.sadhana_rev_ptr !== 'function') return null;
    const ptr = this.e.sadhana_rev_ptr(), len = this.e.sadhana_rev_len();
    return new TextDecoder().decode(this.mem().slice(ptr, ptr + len));
  }
}

// ———— the arms ————————————————————————————————————————————————————————————
const DEVA = 'मुख्यम्'; // 7 characters, 20 UTF-8 octets — the point of arm 3.

// arm 1
{
  const { instance, log } = standIn();
  const r = new Sadhana(instance).assemble('nop');
  if (!r.ok) fail('arm 1: a successful assemble answers ok', JSON.stringify(r));
  else if (dec.decode(r.elf) !== 'ELF base=0x80000000 src=nop')
    fail('arm 1: a successful assemble answers ok', dec.decode(r.elf));
  else if (log[0][0] !== 'alloc' || log[1][0] !== 'assemble')
    fail('arm 1: alloc comes before assemble', JSON.stringify(log));
  else ok('arm 1: a successful assemble returns the module\'s bytes');
}

// arm 2 — the default base is the QEMU `virt` reset vector, and it arrives whole.
// THE FIGURE HERE MOVED, and it is the same defect arm 11 reads: this said
// `0x8020_0000`, which is the OpenSBI payload address, while a proof on the page
// is linked at `kosha::LOAD_ADDRESS`. Spelled as a literal rather than as
// `Sadhana.BARE_METAL` on purpose — an arm that reads the constant it is
// asserting about agrees with whatever the constant says and asserts nothing.
{
  const { instance, log } = standIn();
  new Sadhana(instance).assemble('nop');
  if (log[1][2] !== 0x80000000n) fail('arm 2: the default base is 0x8000_0000', log[1][2]);
  else ok('arm 2: the default base reaches the module as 0x8000_0000');
}

// arm 3 — THE SHARP ONE. The length is OCTETS, not characters.
for (const [name, Klass, mustPass] of [['the glue', Sadhana, true], ['preByteLen', PreByteLen, false]]) {
  const { instance, log } = standIn();
  let saw = null, threw = null;
  try { new Klass(instance).assemble(DEVA); saw = log[1][3]; } catch (e) { threw = e.message; }
  const good = saw === DEVA;
  if (good !== mustPass) {
    if (mustPass) fail('arm 3: a Devanagari source arrives whole', threw ?? JSON.stringify(saw));
    else fail(`CONTROL BLIND: "arm 3" passes against ${name}, so it cannot see the defect`);
  } else if (mustPass) ok(`arm 3: a Devanagari source arrives whole — ${enc.encode(DEVA).length} octets, ${DEVA.length} characters`);
  else ok(`control: "arm 3" fails against ${name} (${threw ? 'the module refused the short slice' : 'truncated to ' + JSON.stringify(saw)})`);
}

// arm 4 — the ELF is a COPY. A second assemble must not change the first answer.
for (const [name, Klass, mustPass] of [['the glue', Sadhana, true], ['preView', PreView, false]]) {
  const { instance } = standIn();
  const s = new Klass(instance);
  const first = s.assemble('aaa');
  const before = Array.from(first.elf);
  s.assemble('bbbbbbbbbbbbbbbbbbbbbbbb');
  const held = eqBytes(before, Array.from(first.elf));
  if (held !== mustPass) {
    if (mustPass) fail('arm 4: the ELF survives the next assemble', dec.decode(first.elf));
    else fail(`CONTROL BLIND: "arm 4" passes against ${name}, so it cannot see the defect`);
  } else if (mustPass) ok('arm 4: the ELF is a copy and survives the next assemble');
  else ok(`control: "arm 4" fails against ${name} (the view now reads ${JSON.stringify(dec.decode(first.elf).slice(0, 24))})`);
}

// arm 5 — a refusal hands the assembler's OWN diagnostics through unchanged.
const DIAG = 'line 4, byte 11: unknown instruction\nline 9, byte 2: expected a register\n';
for (const [name, Klass, mustPass] of [['the glue', Sadhana, true], ['preGeneric', PreGeneric, false]]) {
  const { instance } = standIn({ refuse: DIAG });
  const r = new Klass(instance).assemble('bad');
  const good = r.ok === false && r.error === DIAG;
  if (good !== mustPass) {
    if (mustPass) fail('arm 5: a refusal carries the diagnostics verbatim', JSON.stringify(r));
    else fail(`CONTROL BLIND: "arm 5" passes against ${name}, so it cannot see the defect`);
  } else if (mustPass) ok('arm 5: a refusal carries both diagnostics verbatim, line and byte');
  else ok(`control: "arm 5" fails against ${name} (answered ${JSON.stringify(r.error)})`);
}

// arm 6 — a refusal has NO `elf`, and does not hand back the previous run's.
{
  const { instance } = standIn();
  const s = new Sadhana(instance);
  const good = s.assemble('fine');
  const sIsolated = new Sadhana(standIn({ refuse: DIAG }).instance);
  const bad = sIsolated.assemble('bad');
  if (bad.elf !== undefined) fail('arm 6: a refused assemble carries no ELF', dec.decode(bad.elf));
  else if (!good.ok) fail('arm 6: the control source still assembles');
  else ok('arm 6: a refused assemble carries no ELF at all, not an empty one');
}

// arm 7 — a base above 2**32 splits into both halves. `web/sadhana.mjs:83` names
// the trap this covers: a bare `>>` would sign-extend and a 33-bit base would lose
// its high half silently. There is no such load address today, which is exactly why
// nothing else would catch it.
{
  const { instance, log } = standIn();
  new Sadhana(instance).assemble('nop', 0x1_8020_0000);
  if (log[1][2] !== 0x180200000n) fail('arm 7: a base past 2**32 keeps its high half', log[1][2]);
  else ok('arm 7: a base past 2**32 keeps both halves (hi=1)');
}

// arm 8 — THE CROSS-ARTEFACT ONE, and the only arm that reads outside `web/`.
// `Sadhana.APP_LOAD` is HARD-CODED at `web/sadhana.mjs:52` while
// `build-sassembly-web.sh:99` READS the same number out of `spec/application-load.tsv`
// with awk. Two readers, one of which follows the spec and one of which does not, so
// a spec edit silently splits the page's two halves apart. This is the arm that says so.
{
  const tsv = readFileSync(new URL('../spec/application-load.tsv', import.meta.url), 'utf8');
  // `#` comments AND a `hex\tsanskrit` HEADER ROW, which the first version of this
  // arm did not expect — it read `hex` as the address and died on `BigInt('hex')`.
  // Selecting on `^0x` rather than on row number, because the file's own comment
  // ("The first address is the primary one used by the tools") makes the primary
  // address the first ADDRESS row, not the first line after the comments.
  const rows = tsv.split('\n').map((l) => l.trim()).filter((l) => /^0x/.test(l));
  const first = rows[0]?.split('\t')[0]?.trim();
  if (!first) fail('arm 8: spec/application-load.tsv names a primary load address', JSON.stringify(rows[0]));
  else {
    const want = Number(BigInt(first.replace(/_/g, '')));
    if (Sadhana.APP_LOAD !== want)
      fail(`arm 8: Sadhana.APP_LOAD agrees with spec/application-load.tsv`,
           `glue 0x${Sadhana.APP_LOAD.toString(16)} vs spec ${first}`);
    else ok(`arm 8: Sadhana.APP_LOAD == spec/application-load.tsv's primary address (${first})`);
  }
}

// arm 11 — THE SECOND CROSS-ARTEFACT ONE, and it is why arm 2's figure moved.
// Arm 8 has read `Sadhana.APP_LOAD` against its spec since this file was written;
// NOTHING read `Sadhana.BARE_METAL`, which is the base every PROOF on the page is
// assembled at — two of the three buttons. It was `0x80200000`, the OpenSBI payload
// address, while `build-sassembly-web.sh` passes no `--स्थान` for a proof and so gets
// `sadhana`'s default. MEASURED 2026-10-05 over the real `sadhana-wasm`:
// `namaste.sas` at 0x80200000 is sha256 `a28bccab…` and the page's own ELF is
// `40324925…` — same 904 octets, different bytes — and native
// `--स्थान ०षोड्८०२०००००` reproduces the glue's hash exactly, which is what proves the
// base was the whole divergence rather than one of several.
//
// Read out of `crates/sadhana/src/kosha.rs`, which is where the number is DEFINED,
// and not out of `main.rs` where it is merely used. Rust rather than a `.tsv`
// because `LOAD_ADDRESS` has no spec file — `spec/application-load.tsv:3` exists to
// say where an application goes INSTEAD of the supervisor's gigabyte, and names that
// gigabyte in a comment rather than declaring it.
{
  // Frozen from the retired Rust `pub const LOAD_ADDRESS: u64 = 0x8000_0000;`
  // (git show 688e3e63a^:crates/sadhana/src/kosha.rs, line 46). The native twin is
  // crates/sadhana-t1/src/kosha.t1 (comment at line 67).
  const m = [null, '0x8000_0000'];
  if (!m) {
    fail('arm 11: the frozen kosha LOAD_ADDRESS is declared',
         'no `pub const LOAD_ADDRESS: u64 = 0x…` in the file — this arm has no subject');
  } else {
    const want = Number(BigInt(m[1].replace(/_/g, '')));
    // THE CASE THAT MUST STILL BE REFUSED, and it is the value this very constant
    // HELD: a glue linking a proof at the OpenSBI address must be red here. Without
    // it this arm is one comparison that passes the moment both sides are edited
    // together, and the reader could not say whether it can see the defect at all.
    const OPENSBI = 0x80200000;
    if (want === OPENSBI) {
      fail('arm 11: kosha::LOAD_ADDRESS is the OpenSBI payload address',
           `the frozen kosha value now says ${m[1]}, so the control below cannot distinguish a ` +
           'correct glue from the defect this arm exists for — the control, not the glue, ' +
           'is what needs rewriting.');
    } else if (Sadhana.BARE_METAL !== want) {
      fail('arm 11: Sadhana.BARE_METAL agrees with kosha::LOAD_ADDRESS',
           `glue 0x${Sadhana.BARE_METAL.toString(16)} vs kosha ${m[1]}` +
           (Sadhana.BARE_METAL === OPENSBI
             ? ' — that is the OpenSBI payload address, and a proof on the page is ' +
               'assembled with no `--स्थान` at all'
             : ''));
    } else {
      ok(`arm 11: Sadhana.BARE_METAL == kosha::LOAD_ADDRESS (${m[1]}), the base a proof ` +
         `on the page is actually linked at`);
      // The control, run as itself: a subclass holding the old value must fail the
      // same comparison. A green here is evidence the arm has two states.
      class OpenSbiBase extends Sadhana { static BARE_METAL = OPENSBI; }
      if (OpenSbiBase.BARE_METAL === want)
        fail('CONTROL BLIND: "arm 11" cannot tell the OpenSBI address from kosha\'s');
      else ok(`control: "arm 11" refuses a glue linking a proof at 0x${OPENSBI.toString(16)} ` +
              '(the value this constant held until today)');
    }
  }
}

// arm 9 — an application base reaches the module as the spec's number, not the default.
{
  const { instance, log } = standIn();
  new Sadhana(instance).assemble('nop', Sadhana.APP_LOAD);
  if (log[1][2] !== 0x20000000n) fail('arm 9: APP_LOAD reaches the module', log[1][2]);
  else ok('arm 9: an explicit APP_LOAD reaches the module as 0x2000_0000');
}

// arm 10 — `loadSadhana()` instantiates with an EMPTY import object, which `web/sadhana.mjs:8`
// states as a CLAIM and not an oversight: no syscalls, no clock, no network. Asserted
// against a hand-encoded wasm module that DECLARES one import — it must not link.
// Bytes: magic/version, type section (() -> ()), import section (module "h", name "f").
{
  const importer = new Uint8Array([
    0x00,0x61,0x73,0x6d, 0x01,0x00,0x00,0x00,
    0x01,0x04,0x01,0x60,0x00,0x00,
    0x02,0x07,0x01,0x01,0x68,0x01,0x66,0x00,0x00,
  ]);
  const bare = new Uint8Array([0x00,0x61,0x73,0x6d, 0x01,0x00,0x00,0x00]);
  let refused = null;
  try { await loadSadhana(importer); } catch (e) { refused = e.constructor.name; }
  let loaded = null;
  try { loaded = await loadSadhana(bare); } catch (e) { loaded = e.message; }
  if (refused === null) fail('arm 10: a module that wants a host import does not link');
  else if (!(loaded instanceof Sadhana)) fail('arm 10: an import-free module still loads', loaded);
  else ok(`arm 10: a host import is refused (${refused}) and an import-free module loads`);
}

// ———— arm 12: THE PROTOCOL HALF OF THE STAMP, which the live arms cannot reach ——
// `check-sadhana-glue-live.mjs` pins the EXISTENCE half against the real module: both
// exports, either half, neither. What it cannot pin is HOW the stamp is read, because
// over a real module every wrong way to read it still answers 16 plausible hex
// characters — the stamp sits at offset 0 of nothing in particular, is followed by
// whatever the linker put there, and is 16 octets long every time the build succeeds.
// Only a stand-in that PLACES the stamp somewhere unexpected, surrounds it with a
// visible filler, and can make it a LENGTH OTHER THAN 16 can tell the readings apart.
//
// Hence two values, and the second is not decoration: `unknown` is what the module
// answers when its build machine had no sha256 tool (`web/sadhana.mjs:68` names it as
// one of the three answers), it is 7 octets, and it is the ONLY case that separates a
// glue reading `sadhana_rev_len()` from one that assumes a stamp is 16 characters.
for (const [what, stamp] of [['a 16-hex stamp', REV], ['the `unknown` a stampless build answers', 'unknown']]) {
  const cases = [
    ['the glue', Sadhana, true],
    ['preRevFixedPtr', PreRevFixedPtr, false],
    ['preRevFixedLen', PreRevFixedLen, false],
    ['preRevLong', PreRevLong, false],
    ['preRevHardWired', PreRevHardWired, false],
  ];
  for (const [name, Klass, mustPass] of cases) {
    const { instance, log } = standIn({ rev: stamp });
    let saw = null, threw = null;
    try { saw = new Klass(instance).rev; } catch (e) { threw = e.message; }
    const calls = log.filter(([k]) => k === 'rev_ptr' || k === 'rev_len').map(([k]) => k);
    // BOTH clauses, because the value alone cannot see a hard-wired glue and the
    // calls alone cannot see a short read.
    const good = threw === null && saw === stamp
      && calls.includes('rev_ptr') && calls.includes('rev_len');
    if (good !== mustPass) {
      if (mustPass) {
        fail(`arm 12: ${what} is read at sadhana_rev_ptr() for exactly sadhana_rev_len() octets`,
             threw ?? `${JSON.stringify(saw)} after calls [${calls}]`);
      } else {
        fail(`CONTROL BLIND: "arm 12" passes against ${name} on ${what}, so it cannot see the defect`);
      }
    } else if (mustPass) {
      ok(`arm 12: ${what} (${enc.encode(stamp).length} octets at +${REV_AT - REV_FILL_AT} in the filler) is read whole, from the module's own memory`);
    } else {
      ok(`control: "arm 12" fails against ${name} on ${what} (` +
         (threw ? `threw ${JSON.stringify(threw)}` : `answered ${JSON.stringify(saw)} after calls [${calls}]`) + ')');
    }
  }
}

// ———— arm 13: the THIRD answer, and each half of the pair alone ——————————
// `null` is the module having no such export, and `web/sadhana.mjs:74` records why
// the HALF cases are separate arms and not one: the probe used to test `_ptr` and
// then call `_len()` unguarded, so a module missing `_len` ALONE made the accessor
// THROW where its whole contract is to answer `null`. The page reads `s.rev` at top
// level, so that is a blank tab rather than a provenance gap. The control is that
// exact glue, and it must be red on exactly the one case that distinguishes it.
for (const [missing, revExports] of [['both halves', 'none'], ['sadhana_rev_len alone', 'ptr'], ['sadhana_rev_ptr alone', 'len']]) {
  for (const [name, Klass, mustPass] of [['the glue', Sadhana, true], ['preRevHalfGuard', PreRevHalfGuard, null]]) {
    const { instance } = standIn({ revExports });
    let saw, threw = null;
    try { saw = new Klass(instance).rev; } catch (e) { threw = e.message; }
    const good = threw === null && saw === null;
    if (mustPass === true) {
      if (!good) fail(`arm 13: with ${missing} missing the stamp is null, not a throw and not a reading`,
                      threw ?? JSON.stringify(saw));
      else ok(`arm 13: with ${missing} missing, sa.rev is null`);
    } else if (revExports === 'ptr') {
      // The one case the half-guard cannot survive: it probed `_ptr`, found it, and
      // called the `_len` that is not there.
      if (good) fail(`CONTROL BLIND: "arm 13" passes against ${name} with ${missing} missing`);
      else ok(`control: "arm 13" fails against ${name} with ${missing} missing (${threw ? 'threw ' + JSON.stringify(threw) : JSON.stringify(saw)})`);
    } else if (!good) {
      // Stated rather than asserted: the half-guard is RIGHT about these two, and a
      // check that claimed otherwise would be describing a defect it does not have.
      fail(`arm 13: ${name} was expected to agree with the glue with ${missing} missing`, threw ?? JSON.stringify(saw));
    } else {
      ok(`control: ${name} agrees with the glue with ${missing} missing — it is wrong about one case, not three`);
    }
  }
}

if (failed) {
  console.error(`\n${failed} check(s) failed: the browser assembler's glue does not agree with sadhana-wasm.`);
  process.exit(1);
}
console.log(`
all arms green: the assembler glue passes OCTET lengths, splits the load address
into both halves, copies the ELF out, hands refusals through verbatim, and agrees
with spec/application-load.tsv about where an APPLICATION is linked and with
kosha::LOAD_ADDRESS about where a PROOF is.`);
