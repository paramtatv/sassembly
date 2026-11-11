// THE GENERATED PAGE'S OWN SCRIPT RUNS, AND THE RIGHT PANE FILLS — task `F-004`.
//
// ## What was executed by nothing
//
// `web/yantra.mjs` has two checks (`check-yantra-glue.sh` against a stand-in,
// `check-yantra-glue-live.sh` against the real module). The page's own
// `<script type="module">` in `web/sassembly.template.html` had NEITHER, and
// `check-sassembly-identity.sh` does not run it: it regexes `PROGRAMS` out of
// the page and drives `yantra_run` through the RAW exports, so the page's
// wiring — the button loop, the two pane ids, `show()`'s empty-reason branch,
// the `kind === 'app'` switch between `host()` and `run()` — is never entered.
// A wrong element id, a renamed result field, or a call to a glue method that
// no longer exists would be green across the whole gate and broken on open.
//
// THE INLINING IS PART OF WHAT THIS CHECKS. The page does not import the glue
// — `build-sassembly-web.sh` splices it in through `sed 's/^export //'`, so the
// page carries a TRANSFORMED copy. An `export default` or an `export { … }`
// block in `web/yantra.mjs` would survive that sed as a syntax error inside a
// page that every existing check still passes, because none of them parses it.
//
// ## The fed button was asserted as a RUN and never as a FEEDING
//
// The T1 button is the only one here that hands a program data — a packed
// one-source corpus on `निवेशमण्डल`. Its arms asked that the page run the image
// and print a grouped count; NOTHING asked that the slab arrived. `embedded[t1]
// .input` and `.inputName` were read by no arm, and the halt arm wanted only
// `exit status`, which an unfed compiler also prints. Measured on one host, same
// image and same RAM: fed → status 1601 and 115,647,978 retired; unfed → status
// 1202 and 70. Two non-zero statuses, two `exit status` lines, and only the step
// count tells them apart. So the slab is now asserted as DATA (present, named,
// framed, answerable to the source the caption names) and the count against a
// FLOOR that an unfed run cannot reach. Both, because neither sees the other's
// state: the floor catches the unfed image (70) and misses a well-framed EMPTY
// source (115,133,943 — 0.4% off the fed figure), which only the framing catches.
//
// ## The DOM is a stand-in and it is ANSWERABLE TO THE HTML
//
// There is no browser here, so `document` is a shim — but a shim that invents
// an element for any id asked of it would pass a page that names an element the
// HTML does not contain, which is the one failure most worth catching. So the
// ids are PARSED OUT OF THE PAGE'S OWN HTML and nothing else resolves:
// `getElementById` of an unknown id answers `null`, as a browser does, and the
// page then throws. Likewise `querySelectorAll` implements the one selector the
// page uses and REFUSES any other rather than answering an empty list — an
// instrument with two states where the truth has three hides its own breakage.
//
// Usage: check-sassembly-page.mjs <page.html>
import { readFileSync, writeFileSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const [, , pagePath] = process.argv;
if (!pagePath) {
  console.error('usage: check-sassembly-page.mjs <page.html>');
  process.exit(2);
}
const page = readFileSync(pagePath, 'utf8');

let bad = 0;
const say = (ok, what, extra) => {
  console.log(`  ${ok ? 'ok  ' : 'FAIL'}  ${what}${extra === undefined ? '' : ` — ${extra}`}`);
  if (!ok) bad++;
};
const die = (why) => {
  console.error(`  FAIL  ${why}`);
  process.exit(1);
};

// ---------------------------------------------------------------- the shim
// Only the ids the HTML declares. `id="x"` and `id='x'` both; the template uses
// double quotes, and accepting one spelling would make a correct page look wrong.
const declared = new Set(
  [...page.matchAll(/\sid=["']([^"']+)["']/g)].map((m) => m[1]),
);
if (!declared.size) die('no id= attribute in the page at all — the shim would serve nothing');

const mkEl = (tag) => ({
  tag,
  textContent: '',
  className: '',
  onclick: null,
  children: [],
  append(...kids) { this.children.push(...kids); },
  click() {
    if (typeof this.onclick !== 'function') throw new Error(`<${tag}> has no onclick to click`);
    this.onclick();
  },
});

const byId = new Map();
const created = [];
globalThis.document = {
  getElementById(id) {
    // A browser answers null for an id the document does not contain. So does this.
    if (!declared.has(id)) return null;
    if (!byId.has(id)) byId.set(id, mkEl('div'));
    return byId.get(id);
  },
  createElement(tag) {
    const el = mkEl(tag);
    created.push(el);
    return el;
  },
  querySelectorAll(sel) {
    if (sel !== 'button') {
      throw new Error(
        `the DOM shim implements querySelectorAll('button') and nothing else; ` +
          `the page asked for '${sel}'. Teach the shim rather than letting it answer [].`,
      );
    }
    return created.filter((el) => el.tag === 'button');
  },
};
say(globalThis.document.getElementById('surely-no-such-id') === null,
  'the shim answers null for an id the page does not declare');

// --------------------------------------------- the page's script, unmodified
const m = page.match(/<script type="module">([\s\S]*?)<\/script>/);
if (!m) die('the page carries no <script type="module"> — nothing to run');
const body = m[1];
say(/const PROGRAMS = \{/.test(body), 'the extracted script is the page\'s own');

// Written to a file and imported rather than `eval`ed: the body is a MODULE —
// top-level `await` and all — and that is how the browser parses it.
const dir = mkdtempSync(join(tmpdir(), 'sas-page-'));
const mod = join(dir, 'page-script.mjs');
writeFileSync(mod, body);
// Removed on both paths: the body carries the base64 module, so each run would
// leave a quarter-megabyte in the temp directory, and this runs hourly in the
// deep gate. Not inside the repository for the reason `build-sassembly-web.sh`
// writes its page outside one: `tools/check-secrets-native.sh` reads untracked files,
// and 37 KB of base64 contains `ASIA…` by chance.
try {
  await import(pathToFileURL(mod).href);
} catch (e) {
  rmSync(dir, { recursive: true, force: true });
  die(`the page's own script threw: ${e && e.message}\n        ${e && e.stack}`);
}
rmSync(dir, { recursive: true, force: true });
say(typeof globalThis.__result === 'function', 'it ran to the end and exposed __result()');

const embedded = JSON.parse(
  body.match(/const PROGRAMS = (\{[\s\S]*?\});/)[1].replace(/,(\s*[}\]])/g, '$1'),
);
const names = Object.keys(embedded);
const buttons = created.filter((el) => el.tag === 'button');
say(buttons.length === names.length, 'one button per embedded program',
  `${names.length} programs, ${buttons.length} buttons`);

const surface = byId.get('surface'), uart = byId.get('uart'), halt = byId.get('halt');
const stepsEl = byId.get('steps');
if (!stepsEl) die('the page declares no id="steps" — the step count the glue returns is shown nowhere');
const sizingEl = byId.get('sizing');
if (!sizingEl) die('the page declares no id="sizing" — the T1 image\'s memory ask is shown nowhere');
const assemblerEl = byId.get('assembler'), asmEl = byId.get('asm');
if (!assemblerEl) die('the page declares no id="assembler" — whether sadhana-wasm is in this tab is stated nowhere');
if (!asmEl) die('the page declares no id="asm" — the in-tab assembly verdict is shown nowhere');
const interpEl = byId.get('interp');
if (!interpEl) die('the page declares no id="interp" — WHICH `yantra` ran these programs is '
  + 'stated nowhere, and the sizing figures beside them were measured by a different binary');
// THE LABEL IS NOT ALWAYS `<name>.sas`. The T1 image is an ELF the build was HANDED, so
// its button is labelled bare; looking it up as `t1-image.sas` would report a missing
// button on a correct page. The kind decides, the same way the handler's does.
const label = (name) => (embedded[name].kind === 't1' ? name : `${name}.sas`);
const grpTop = (x) => String(x).replace(/\B(?=(\d{3})+(?!\d))/g, ',');
const press = (name) => {
  const b = buttons.find((x) => x.textContent === label(name));
  if (!b) die(`no button labelled ${label(name)} — the page carries ${buttons.map((x) => x.textContent).join(', ')}`);
  // A HANDLER THAT THROWS IS A VERDICT, NOT A CRASH. Left uncaught, a page
  // calling a glue method that no longer exists exits on a bare node stack
  // trace — red, but it names neither the button nor the check that was
  // running, and a gate log is where that is read. Measured with the glue's
  // `host` renamed: `TypeError: y.hostX is not a function` and four frames of
  // this file, with no line saying which program's button was pressed.
  try {
    b.click();
  } catch (e) {
    die(`pressing ${name}.sas threw: ${e && e.message}\n        ` +
        `The page's handler calls something the glue does not provide.`);
  }
  return { surface: surface.textContent, uart: uart.textContent, halt: halt.textContent,
    surfaceFilled: surface.className === 'pane', uartFilled: uart.className === 'pane',
    ok: halt.className === 'ok',
    steps: stepsEl.textContent, stepsNA: stepsEl.className === 'na',
    sizing: sizingEl.textContent, sizingNA: sizingEl.className === 'na',
    sizingClass: sizingEl.className,
    asm: asmEl.textContent, asmClass: asmEl.className };
};

// The page auto-clicks `namaste` on load, so this is already the state. Asserted
// before anything else is pressed, because the auto-click is itself a shape:
// `.find(…).click()` throws on undefined if the proof list is ever renamed.
const NAMASTE = 'नमस्ते संसार';
say(uart.textContent.includes(NAMASTE), 'on load the page has already run namaste',
  JSON.stringify(uart.textContent.slice(0, 24)));

// ------------------------------------------------- a BOOT PROOF that prints
const n = press('namaste');
say(n.ok && n.uart.includes(NAMASTE), 'a boot proof prints on the machine\'s UART', n.halt);
say(n.uartFilled, 'and the UART pane is in its FILLED state, not its reason state');
// THE HALT LINE'S COLOUR IS NOT ITS TEXT, and asserting only the colour left a
// hole: with `r.halt` renamed to `r.h` the pane reads the string "undefined",
// `r.code` is untouched so the class is still `ok`, and every other arm here
// passed. Two of the three states were checked and the third — present, green,
// and saying nothing — was the one a renamed result field produces.
say(n.halt.length > 0 && n.halt !== 'undefined',
  'and the halt line carries the machine\'s own words, not a missing field', JSON.stringify(n.halt.slice(0, 28)));
// THE MUTATION THIS MUST REFUSE is the two panes merged. `host()` and `run()`
// write different buffers and the page's own note calls that "the whole
// difference between the two kinds of program"; a page that showed `out` in
// THE STEP COUNT, which is the figure `F-004` measured and no page could show. A
// RENAMED result field reads as the string "undefined" here and a dropped one as the
// empty string, so both are named rather than only the digits being looked for — the
// same hole the halt line's colour-only assertion had.
say(/[0-9]/.test(n.steps) && !/undefined/.test(n.steps),
  'a boot proof shows its retired-instruction count', JSON.stringify(n.steps));
say(!n.stepsNA, 'and the line is in its FIGURE state, not its reason state');
say(/^[0-9]{1,3}(,[0-9]{3})* instructions retired$/.test(n.steps),
  'in the shape "<count> instructions retired"', JSON.stringify(n.steps));
// ॥ THAT REGEX CANNOT SEE THE GROUPING AND MUST NOT BE READ AS IF IT COULD ॥ Every
// program embedded in this page retires a THREE-DIGIT count, so `(,[0-9]{3})*` matches
// zero groups and the line passes with the separator removed entirely. The figure
// `F-004` wants on this page is 115,647,978, which has two. So the page's OWN `group`
// expression is lifted out of the script verbatim and evaluated on that number — not a
// reimplementation of it here, and not a mutated page.
{
  const m = body.match(/const group = (n => [^\n]*);/);
  if (!m) die('the page no longer defines a `group` expression — the grouping cannot be checked');
  const group = new Function(`return (${m[1]})`)();
  say(group(115647978) === '115,647,978', 'the page groups the T1 image\'s own count', group(115647978));
  say(group(184) === '184', 'and leaves a count under a thousand alone', group(184));
  say(group(1000) === '1,000', 'with the first separator at exactly a thousand', group(1000));
  // `toLocaleString` would group by whatever locale the browser is in, so a quoted
  // figure would not read the same in every tab. The test is for a CALL and not for the
  // word: the first spelling of this matched the template's own margin explaining why it
  // does not use it, and reported a red on a correct page.
  say(!/\.toLocaleString\s*\(/.test(body), 'and never CALLS toLocaleString, so every tab agrees');
}
say(!n.surface.includes(NAMASTE), 'and the surface pane does NOT carry the UART\'s text');
say(!n.surfaceFilled, 'the surface pane says WHY it is empty', JSON.stringify(n.surface.slice(0, 32)));

// --------------------------- an APPLICATION: the panes swap, and that is the ABI
const a = press('atithi');
say(a.ok, 'an application exits 0 under the loader and supervisor', a.halt);
say(a.surfaceFilled && a.surface.length > 0, 'it wrote to the surface it was HANDED',
  JSON.stringify(a.surface.slice(0, 24)));
say(!a.uartFilled, 'and the machine\'s UART stayed empty — it cannot address a device',
  JSON.stringify(a.uart.slice(0, 32)));
// An application has NO reading — `yantra_host` clears the counter. The line must say
// so: a `0` would claim the machine retired nothing, and a blank would look like the
// page failed to fill it.
say(a.stepsNA, 'an application\'s step line is in its REASON state');
say(!/[0-9]/.test(a.steps) && a.steps.length > 0,
  'and carries words rather than a number or a blank', JSON.stringify(a.steps.slice(0, 32)));
// ॥ A STRINGIFIED NULL HAS NO DIGITS EITHER ॥ Measured by mutation: with the reason
// branch deleted, the page renders `group(null) + ' instructions retired'` — "null
// instructions retired", which the assertion above accepts because it is looking for
// digits. The three words a dropped field renders as are therefore named outright.
say(!/\b(null|undefined|NaN)\b/.test(a.steps),
  'and not a stringified absence — the reason branch is really there', JSON.stringify(a.steps.slice(0, 32)));
say(a.steps !== n.steps, 'so the two kinds of program say different things about the counter');
say(!a.uart.includes(a.surface), 'the UART pane does NOT carry the surface\'s text');
say(a.surface !== n.uart, 'the two kinds of program fill two different panes');

// ------------------- a PROOF THAT PRINTS NOTHING: empty is the result, not a fault
const bm = press('bare-metal');
say(bm.ok, 'a proof that writes no characters is still a SUCCESS, not a red', bm.halt);
// A proof that PRINTS NOTHING still retires instructions: the one case where the step
// count is the only evidence on the page that anything ran at all.
say(/[0-9]/.test(bm.steps) && !bm.stepsNA,
  'a proof that writes no characters still shows a count — the only sign it ran', JSON.stringify(bm.steps));
say(!bm.uartFilled && !bm.surfaceFilled, 'with both panes in their reason state',
  JSON.stringify(bm.uart.slice(0, 32)));

// ------------- THE SIZING LINE AND THE MEMORY REFUSAL — `F-004`'s fourth button
// The line has two states and the three cheap programs are the REASON one: a megabyte is
// not something a device is asked about, and a blank there would read as a page that
// failed to fill it. Same hole as the step line's: a renamed field renders the word
// "undefined" and a dropped one the empty string, so both are named.
say(bm.sizingNA, 'the sizing line is in its REASON state for a program nobody sizes');
say(bm.sizing.length > 0 && !/\b(null|undefined|NaN)\b/.test(bm.sizing),
  'and carries words rather than a stringified absence', JSON.stringify(bm.sizing.slice(0, 40)));
// NOT `!/[0-9]/`: the reason names `T1`, which is a digit, and the first spelling of
// this reported a red on a correct page for that. What must be absent is a MEASURED
// FIGURE — grouped, or at minimum more than one digit long. A name is not a figure.
say(!/[0-9]{2,}/.test(bm.sizing) && !/[0-9],[0-9]{3}/.test(bm.sizing),
  'and quotes no measured figure, because none was measured for it',
  JSON.stringify(bm.sizing.match(/[0-9][0-9,]*/g)));

// ॥ THE REFUSAL IS TESTED WITHOUT A DEVICE THAT REFUSES ॥ No machine running this check
// will fail a 538 MB `memory.grow`, and a branch no gate enters is a branch that rots —
// this one existed because the ALTERNATIVE is a bare `RuntimeError` out of Rust's
// `unreachable`, which names neither the figure nor the device. So the page's own
// `reserve` is lifted out VERBATIM, as `group` is above, and driven against a stand-in
// memory in its three states. Not a reimplementation of it here: a mutated or deleted
// `reserve` must red, and a copy in this file would still pass.
{
  const m = body.match(/const reserve = \(octets\) => \{[\s\S]*?\n\};/);
  if (!m) die('the page no longer defines `reserve` — a device that cannot give the memory would get a bare wasm trap');
  const g = new Function(`return (${body.match(/const group = (n => [^\n]*);/)[1]})`)();
  const mk = (bytes, grow) => ({ e: { memory: { buffer: { byteLength: bytes }, grow } } });
  const lift = (yy) => new Function('y', 'group', 'PAGE', `${m[0]} return reserve;`)(yy, g, 65536);

  const refusing = lift(mk(1 << 20, () => { throw new RangeError('out of memory'); }));
  const verdict = refusing(538529272);
  say(typeof verdict === 'string' && verdict.length > 0,
    'a device that refuses the growth gets a VERDICT, not a thrown trap', typeof verdict);
  say(typeof verdict === 'string' && verdict.includes('538,529,272'),
    'and the verdict NAMES the figure that was refused, grouped',
    JSON.stringify(String(verdict).slice(0, 56)));
  say(typeof verdict === 'string' && /RangeError/.test(verdict),
    'and names what the device said, so the reason is not invented');
  say(typeof verdict === 'string' && /NOTHING WAS RUN/.test(verdict),
    'and says NOTHING RAN — a refusal is not a result about the compiler');

  let grown = null;
  const giving = lift(mk(1 << 20, (d) => { grown = d; return 16; }));
  say(giving(538529272) === null, 'a device that gives it gets no verdict and the run proceeds');
  say(grown === Math.ceil((538529272 - (1 << 20)) / 65536),
    'and the growth asked for is the shortfall in whole pages, not the whole ask', grown);

  let asked = false;
  const already = lift(mk(1 << 30, () => { asked = true; return 0; }));
  say(already(538529272) === null && !asked,
    'and memory already big enough is not grown at all — three states, not two');
}
// THE CALL SITE, not just the function: `reserve` could be perfect and never reached.
say(/const refused = reserve\(/.test(body) && /if \(refused\)/.test(body),
  'and the T1 arm CALLS reserve and branches on its answer');

// ------------------- the fourth button itself, when an image was NAMED to the build
// `T1_IMAGE` unset is the ordinary page and NOT a hole: the deep gate builds it that way
// and the three buttons above are the whole claim. So this arm reports VACUOUS rather
// than passing silently — `W-087`'s finding was 29 skip paths that said "passed".
// The fresh page's T1 press, for the water refusal arm at the bottom of this file.
// `null` when the fourth button is absent, and the arm reports itself UNRUN rather than
// green — the same rule as every other t1 arm here.
let freshT1 = null;
const t1Name = names.find((n) => embedded[n].kind === 't1');
if (!t1Name) {
  console.log('  --    no t1-kind program embedded (T1_IMAGE unset at build) — the fourth');
  console.log('        button\'s run arms are UNRUN, not passed. Rebuild with T1_IMAGE set.');
} else {
  const t = embedded[t1Name];
  say(typeof t.ram === 'number' && typeof t.touches === 'number' && typeof t.budget === 'number',
    'the build measured ram, touches and budget as numbers, not prose',
    `${t.ram}/${t.touches}/${t.budget}`);
  // ── ॥ `ram` IS ANSWERABLE TO THE IMAGE'S OWN HEADERS — `touches` IS NOT ॥ ──────
  // The arm above asks that the three figures be NUMBERS. A number is not a
  // measurement, and this is the one button whose whole claim is a figure: `askedFor`
  // reads `p.ram` and `p.touches` straight off the embedded record, so a build that
  // filled them from prose — `512 MB` because somebody remembered it that way — would
  // render a sizing line IDENTICAL in shape to the measured one, with a ratio, a
  // grouping and a megabyte rendering that all agree with each other. Every arm below
  // would stay green: they compare the line against `t.ram`/`t.touches` and would be
  // comparing prose with itself.
  //
  // So the figure is re-derived HERE from the ELF THE PAGE CARRIES, which is the one
  // second instrument available without running anything. `build-sassembly-web.sh:207`
  // is right that a second copy of the loader's span arithmetic is a second thing to
  // drift — but that is an argument about where the BUILD gets the number, not about
  // whether a checker may ask the image. `crates/yantra/src/lib.rs:1310` refuses with
  // `max(memsz, filesz)` of the first segment that will not fit, so `ram` must be one
  // of the embedded ELF's own PT_LOAD spans, and it must cover all of them or the
  // page's own run could not have loaded.
  //
  // ॥ AND `touches` HAS NO SUCH DERIVATION, WHICH IS SAID RATHER THAN PAPERED OVER ॥
  // The obvious floor — the high water must at least cover the octets the loader COPIED
  // in, `filesz` — IS FALSE HERE, and measuring it is what settled it: `Sink::stored`
  // in `crates/yantra/src/bin/yantra-run.rs:24` advances the water on STORES ONLY. The
  // load copy is a `copy_from_slice` into `m.mem` and no instruction fetch or read moves
  // it either, so a run that stores nothing low reports a high water under `filesz`
  // legitimately. `touches` therefore rests on the ratio arm and on the interpreter
  // stamp, and closing it needs a watermark export out of `yantra-wasm` so the tab's own
  // run can be compared against the native one. That is not this arm, and this arm does
  // not pretend to be it.
  const elfBytes = Buffer.from(t.elf ?? '', 'base64');
  const loads = (() => {
    if (elfBytes.length < 64 || elfBytes.readUInt32LE(0) !== 0x464c457f) return null;
    const phoff = Number(elfBytes.readBigUInt64LE(0x20));
    const phentsize = elfBytes.readUInt16LE(0x36);
    const phnum = elfBytes.readUInt16LE(0x38);
    if (phnum === 0 || phoff + phnum * phentsize > elfBytes.length) return null;
    const out = [];
    for (let i = 0; i < phnum; i += 1) {
      const o = phoff + i * phentsize;
      if (elfBytes.readUInt32LE(o) !== 1) continue; // PT_LOAD and nothing else
      const filesz = Number(elfBytes.readBigUInt64LE(o + 32));
      const memsz = Number(elfBytes.readBigUInt64LE(o + 40));
      out.push({ vaddr: Number(elfBytes.readBigUInt64LE(o + 16)), span: Math.max(memsz, filesz) });
    }
    return out.length ? out : null;
  })();
  // NOT A `say`: with no parse there is no second instrument, and an arm that answered
  // `false` here would read as "the figure is wrong" when the truth is "nothing asked".
  if (loads === null) {
    die('the embedded t1 ELF carries no readable PT_LOAD header, so `ram` is answerable '
      + 'to nothing on this page and the sizing line is prose by default');
  }
  // `includes` AND the cover, because they fail differently: a figure lifted from a
  // small segment of a multi-segment image is a real span and still too little to load.
  const base = Math.min(...loads.map((s) => s.vaddr));
  const answerable = (ram) => loads.some((s) => s.span === ram)
    && loads.every((s) => (s.vaddr - base) + s.span <= ram);
  say(answerable(t.ram),
    'and `ram` is one of the embedded ELF\'s own PT_LOAD spans and covers them all — '
      + 'the figure was MEASURED off this image, not typed beside it',
    `${t.ram} vs spans ${JSON.stringify(loads.map((s) => s.span))}`);
  // ॥ THE CASE THAT MUST STILL BE REFUSED ॥ The arm above is worth nothing unless it
  // DISCRIMINATES, and a predicate that accepted anything would be green on this image
  // too. So the same derivation is asked of the figures a prose build would carry: the
  // round half-gigabyte somebody remembers, a plausible truncation, and the true figure
  // off by one in each direction — which is the hardest of the four, because it is the
  // shape a hand-edited record actually takes.
  // (`typed`, not `prose`: the name is taken forty lines down by the page's own text.)
  const typed = [
    ['512 MiB, the round figure prose carries', 512 * 1024 * 1024],
    ['538,000,000, a plausible truncation', 538_000_000],
    ['the true span plus one', t.ram + 1],
    ['the true span minus one', t.ram - 1],
  ];
  const accepted = typed.filter(([, v]) => answerable(v));
  say(accepted.length === 0,
    `and a hardcoded figure beside this very ELF is REFUSED by it — all ${typed.length} of them`,
    accepted.length === 0
      ? typed.map(([w]) => w).join('; ')
      : `ACCEPTED: ${accepted.map(([w, v]) => `${w} (${v})`).join('; ')}`);
  // ॥ AND THE DOCTORED FIGURE RENDERS A LINE EVERY OTHER ARM CALLS GOOD ॥
  // `would look identical` is the claim the two arms above rest on, and a claim is not an
  // assertion. So the page's OWN `askedFor` is lifted out verbatim — as `reserve` and
  // `group` are above, and not reimplemented here, or a mutated one would still pass —
  // and handed the hardcoded figure. The line it returns is then put to every shape test
  // the sizing arms below apply: a grouped octet figure, a megabyte rendering that does
  // not round past it, a ratio, the LOAD claim. All four hold, because they are all
  // self-consistent with `p.ram` whatever `p.ram` is. THAT is why the ELF had to be read.
  {
    const am = body.match(/const askedFor = \(p\) =>[\s\S]*?`;\n/);
    if (!am) die('the page no longer defines `askedFor` — the sizing line cannot be driven here');
    const gm = body.match(/const group = (n => [^\n]*);/);
    if (!gm) die('the page no longer defines `group` — the sizing line cannot be driven here');
    const lift = new Function('group', `${am[0]} return askedFor;`)(
      new Function(`return (${gm[1]})`)(),
    );
    const doctored = { ram: 512 * 1024 * 1024, touches: t.touches, source: t.source };
    const line = lift(doctored);
    const shape = [
      [/[0-9]{1,3}(,[0-9]{3})+ octets of linear memory asked for/, 'a grouped octet figure'],
      [new RegExp(`\\(${(doctored.ram / 1e6).toFixed(1)} MB\\)`), 'an agreeing megabyte rendering'],
      [/[0-9]+× tail/, 'a ratio'],
      [/Demanded at LOAD/, 'the LOAD claim'],
    ];
    const missing = shape.filter(([re]) => !re.test(line));
    say(missing.length === 0,
      'and the page\'s own `askedFor`, handed that hardcoded figure, renders a sizing line '
        + 'carrying every shape the arms below ask of the measured one',
      missing.length === 0 ? JSON.stringify(line.slice(0, 60))
        : `MISSING: ${missing.map(([, w]) => w).join('; ')}`);
    say(!answerable(doctored.ram) && answerable(t.ram),
      'so the embedded ELF is the ONLY thing on this page that tells that line from this '
        + 'one — the shape arms cannot, and were never the instrument for it');
  }
  // ── ॥ THE SLAB ITSELF, WHICH NOTHING HERE USED TO LOOK AT ॥ ────────────────
  // This button is the only one on the page that FEEDS a program, and until now not
  // one arm asserted the food arrived. `embedded[t1].input` and `.inputName` were
  // never read; the halt arm below asks only for `exit status`, and a compiler handed
  // NOTHING also halts with a status. MEASURED on this host, same image and same RAM
  // (538,529,272), the two states that `/exit status/` cannot tell apart:
  //   fed    `YANTRA_INPUT=one.blob`  →  status 1601, 115,647,978 retired
  //   unfed  no `YANTRA_INPUT` at all →  status 1202,          70 retired
  // Both are non-zero, both print `exit status`, and both leave the page looking like
  // it ran the compiler. So the slab is asserted as DATA here and as a FIGURE below.
  say(typeof t.input === 'string' && t.input.length > 0,
    'the build carried a slab to the page — the one program here that is fed, is fed',
    `${t.input === undefined ? 'no input field' : `${t.input.length} b64 chars`}`);
  // THE NAME IS THE ENGINE'S REQUIREMENT AND NOT A FIGURE, AND THIS LABEL SAID
  // OTHERWISE. It read "names the module it attributes those octets to" — attribution
  // that arm E of `check-t1-image-wasm-budget.sh` has since DISPROVED. Same image and
  // same RAM (538,529,272), under `शृङ्खला`, under `lib`, and under a name matching
  // nothing: status 1601, 115,647,978 retired, 5,174,861 high water, the same two octets
  // out, all three. Stage 1's entry (`स्वपरीक्षास्वप्रतिबिम्बम्`) takes each module's
  // name out of the blob's own `name NUL text NUL` frame and never loads the
  // `निवेशमण्डलनाम` global the host writes. `run()` still REQUIRES it beside `input`, as
  // `yantra-run` does, so the field must be PRESENT — and it is answerable to nothing,
  // which is what the next two arms are about.
  say(typeof t.inputName === 'string' && t.inputName.length > 0,
    'and carries the module name the engine REQUIRES beside input — required, and unread',
    JSON.stringify(t.inputName));
  // THE SLAB IS ANSWERABLE TO THE SOURCE THE CAPTION NAMES. `tools/pack-corpus.sh`
  // frames each source as `name NUL text NUL`, so the first field is a module name and
  // it must be the basename of `t.source` — otherwise the page's line "One source on
  // the input channel: <source>" is prose about a file these octets did not come from.
  const slab = Buffer.from(t.input ?? '', 'base64');
  const field0 = slab.indexOf(0) < 0 ? null : slab.subarray(0, slab.indexOf(0)).toString('utf8');
  say(field0 !== null && typeof t.source === 'string' && t.source.endsWith(`/${field0}.t1`),
    'and the slab\'s own first framed field is the source the caption names',
    `${JSON.stringify(field0)} vs ${JSON.stringify(t.source)}`);
  // ॥ AND WHEN THE TWO NAMES DIFFER, THE PAGE MUST NOT SHOW A READER THE UNREAD ONE ॥
  // `inputName` is `शृङ्खला` on this build and the frame says `lib`, so the page carries
  // two names for one module and only ONE of them is the compiled unit's. Prose naming
  // the other would be a caption about a module that nothing in the guest ever reads —
  // the figure arm E disproved, printed as fact. CONDITIONED ON THE TWO DIFFERING, and
  // that is not a convenience: with `T1_MODULE=lib` the name IS the frame's, the caption
  // `lib.t1` is then TRUE, and an unconditional substring test would red on the page
  // whose caption is correct while the `lib` in `lib.t1` matched. No `\b` here on
  // purpose — a word boundary does not work against Devanagari, so the arm is a plain
  // containment under a condition that makes containment mean something.
  // MEASURED 2026-10-05: the template names `lib.t1` and never `शृङ्खला`, so this is
  // green on the page as written. It exists because nothing stopped the next caption
  // from naming it, which is exactly what `F-004`'s own `Next:` asked be checked.
  // The `<script>` bodies are EXCLUDED: `PROGRAMS` legitimately carries the name as
  // DATA, and data is what the arm above asserts.
  const prose = page.replace(/<script[\s\S]*?<\/script>/g, '');
  // `where()` exists so the FAILING line does not read "absent". The first draft of this
  // arm printed the same detail on both verdicts, so a red said `"शृङ्खला" absent from
  // 3,617 octets` while reddening BECAUSE it was present — a verdict and its evidence
  // contradicting each other on one line. It quotes the surrounding text instead.
  const where = (hay, needle) => {
    const i = hay.indexOf(needle);
    return i < 0 ? null : JSON.stringify(hay.slice(Math.max(0, i - 28), i + needle.length + 28));
  };
  const inProse = where(prose, t.inputName);
  say(t.inputName === field0 || inProse === null,
    'and the page\'s own prose does not name the module name the image never reads',
    t.inputName === field0
      ? `name equals the frame's (${JSON.stringify(field0)}) — naming it would be true`
      : inProse === null
        ? `${JSON.stringify(t.inputName)} absent from ${prose.length} octets outside <script>`
        : `FOUND in the prose: ${inProse}`);
  // `name NUL text NUL` — so the text is what remains, and this arm is NOT redundant
  // with the step floor below. MEASURED by injecting `lib NUL NUL` — the right name
  // over an EMPTY source — into a built page: the image retires 115,133,943, which is
  // 0.4% off the fed figure and a hundredfold past any floor. A compiler handed a
  // well-framed empty source does nearly all of its work; only the frame says so.
  say(slab.length > (field0?.length ?? 0) + 2,
    'and carries source text after the name, not an empty frame', `${slab.length} octets`);
  say(t.ram > t.touches * 10,
    'and the declared span is still an order of magnitude past the high water',
    `${Math.round(t.ram / t.touches)}x`);
  const r = press(t1Name);
  say(!r.sizingNA, 'the T1 button puts the sizing line in its FIGURE state');
  say(r.sizing.includes(`${Math.round(t.ram / t.touches)}×`),
    'and the line names the ratio with the figures the build measured',
    JSON.stringify(r.sizing.slice(0, 72)));
  // THE TWO RENDERINGS OF THE SAME ASK MUST NOT DISAGREE. `Math.round` made
  // 538,529,272 read as "539 MB" — a megabyte figure larger than the octet figure
  // printed beside it, which is the one way a caption can contradict itself.
  say(r.sizing.includes(`(${(t.ram / 1e6).toFixed(1)} MB)`),
    'and its megabyte rendering does not round past the octet figure beside it',
    r.sizing.match(/\([0-9.]+ MB\)/)?.[0]);
  say(r.sizing.includes('LOAD'),
    'and says the span is demanded at LOAD — which is why however little it touches is no help');
  // ── ॥ `touches` GETS ITS SECOND READING, AND THE PAGE ASSERTS THE TWO AGREE ॥ ──
  // Of the figures on this line, `touches` was the one with NO second opinion: the build
  // measured it with the native `yantra-run` and the page quoted it, so the page compared
  // it with itself. `yantra_water_known`/`yantra_water` landed in `yantra-wasm` at
  // `1ad329c2` and `web/yantra.mjs` reads them now, so the TAB measures the same quantity
  // with the same arithmetic and the line carries both. Same ELF, same slab, same span and
  // same budget on both sides, so the verdict here is EQUALITY.
  //
  // The expectation is grouped by `grpTop` rather than through the page's own `group`:
  // this arm states what the figure should be, and lifting the page's formatter would make
  // it state whatever the page formats.
  say(r.sizing.includes(`This tab touched ${grpTop(t.touches)} — the same octet`),
    'the sizing line carries THIS TAB\'s own high water and it is the native figure exactly',
    JSON.stringify(r.sizing.slice(-110)));
  say(!/DISAGREEMENT|NO HIGH WATER/.test(r.sizing),
    'so neither the disagreement branch nor the no-reading branch was the one taken');
  say(r.sizingClass === '',
    'and the line is in its AGREEING state — `bad` is reserved for the comparison failing',
    `class=${JSON.stringify(r.sizingClass)}`);
  freshT1 = { halt: r.halt, steps: r.steps, uart: r.uart, sizing: r.sizing };
  // This is a `run()`, so there IS a reading, and it is the figure F-004 wanted on the
  // page: eight or nine digits with the grouping the `group` arms above checked.
  say(!r.stepsNA && /^[0-9]{1,3}(,[0-9]{3})+ instructions retired$/.test(r.steps),
    'the T1 image shows a GROUPED retired-instruction count — the figure no page could show',
    JSON.stringify(r.steps));
  // The halt line is RED here on purpose: `lib.t1` declares nothing, so the compiler
  // answers with a status. A green halt would mean the page is not running the compiler.
  say(/exit status/.test(r.halt), 'and the halt line carries the compiler\'s own status',
    JSON.stringify(r.halt.slice(0, 48)));
  // ॥ AND THE COUNT MUST BE PAST WHAT AN UNFED COMPILER CAN REACH ॥ The grouped-count
  // arm above is about FORMAT and catches the unfed run only BY LUCK: 70 has no comma
  // group, so it fails that regex — but a starved path that happened to retire 1,200
  // would be green there and is the same defect. This arm is about MAGNITUDE, and its
  // floor is a measurement and not a round number chosen to pass: unfed, this image
  // stops at 70 instructions (it reads its `निवेश…` globals, finds nothing, and
  // answers), where fed it retires 115,647,978 — six orders between them. One million
  // is inside that gap by a hundredfold on both sides, so it does not pin a status
  // code or a step count that moves whenever the compiler does.
  const retired = Number((r.steps.match(/^([0-9,]+) instructions retired$/)?.[1] ?? '0')
    .replace(/,/g, ''));
  say(retired > 1_000_000,
    'and the count is past what an UNFED run reaches — the slab was placed in the guest\'s RAM',
    `${retired.toLocaleString('en-US')} retired, floor 1,000,000, unfed measures 70`);
  say(!/no instructions were retired/.test(r.steps),
    'and this device gave the memory, so the refusal branch was NOT the one taken');
  // THE PROSE ARM ABOVE CANNOT SEE A PANE, because rendered text is nowhere in the
  // HTML — it is written by `show()` at run time. So the same condition is asked again
  // of everything the button actually PUT on the screen. These two do not overlap: a
  // caption lives in the template and a pane line is built from `PROGRAMS`, and the
  // page already prints `t.source` into one of them, which is the line that would
  // acquire a module name if anybody decided the panes should be more informative.
  const shown = [r.surface, r.uart, r.halt, r.steps, r.sizing].join('\n');
  const inPane = where(shown, t.inputName);
  say(t.inputName === field0 || inPane === null,
    'and no pane the T1 button fills names it either — not in the template, not at run time',
    t.inputName === field0
      ? `name equals the frame's (${JSON.stringify(field0)})`
      : inPane === null
        ? `${JSON.stringify(t.inputName)} absent from ${shown.length} octets of filled panes`
        : `FOUND on a pane: ${inPane}`);
}

// ─────────────── THE ASSEMBLER IN THE TAB, AND THE PAGE WITHOUT ONE ──────────────
//
// Until today `web/sadhana.mjs` was spliced into NOTHING: the build named one glue
// placeholder and the template declared three. So the assembler half of the pair was
// present, plausible and read by no instrument — the same state `yantra_input_alloc`
// was in. These arms are over the page that now carries it.
//
// WHAT MAKES THIS MORE THAN `check-sadhana-glue-live.sh`: that check drives the module
// directly and compares its octets against a NATIVE assembly it performs itself. This
// compares the in-tab assembly against THE ELF THIS PAGE ACTUALLY RAN, one line above
// in the same handler. The two can differ — the page could ship an ELF from one base
// and assemble at another, which is exactly the divergence `11965282` closed — and only
// an arm inside the page can see it.
const assemblerPresent = /sadhana-wasm is in this tab/.test(assemblerEl.textContent);
say(assemblerEl.textContent.length > 0,
  'the page states whether an assembler is in this tab, on load and without a press',
  JSON.stringify(assemblerEl.textContent.slice(0, 34)));
if (assemblerPresent) {
  say(assemblerEl.className === 'ok' && /[0-9],[0-9]{3}/.test(assemblerEl.textContent),
    'and when it is present the line names its size, grouped',
    JSON.stringify(assemblerEl.textContent.slice(0, 48)));
  // ── PROVENANCE, AND NOT ONLY AGREEMENT ──────────────────────────────────────
  // The arms below press every button and assert IDENTICAL. That is AGREEMENT
  // between the tab's assembler and the ELFs beside it, and two copies of one
  // stale `sadhana` agree with each other perfectly — so agreement is the one
  // thing this cannot be allowed to rest on. The line must also name WHICH
  // assembler, from the stamp sealed into the module and the stamp the build
  // recorded for the native one, and the mutation block at the end of this file
  // is what proves the figure can say NO.
  say(/PROVENANCE: SAME ASSEMBLER — [0-9a-f]{16}\b/.test(assemblerEl.textContent),
    'and names the ONE assembler both the module and the ELFs came from, by stamp',
    JSON.stringify((/PROVENANCE: [^,]{0,44}/.exec(assemblerEl.textContent) || ['absent'])[0]));
  // EVERY `.sas` PROGRAM, not a sampled one: the base is what diverged last time and
  // the two kinds get it from different places — a proof from `kosha::LOAD_ADDRESS`
  // and an application from `spec/application-load.tsv`. One kind green would hide
  // the other entirely, which is how `atithi` stayed identical through a defect that
  // changed both proofs.
  for (const name of names.filter((n) => embedded[n].kind !== 't1')) {
    const r = press(name);
    say(r.asmClass === 'ok' && /IDENTICAL/.test(r.asm),
      `the tab re-assembles ${name}.sas to the very octets the page just RAN`,
      JSON.stringify(r.asm.slice(0, 56)));
  }
} else {
  say(false,
    'the page this check built carries no assembler, so its agreement arms have no subject',
    'build-sassembly-web.sh is supposed to splice sadhana-wasm — this is a FAIL and not a skip');
}

// ── THE CASE THAT MUST STILL BE REFUSED: glue spliced, module NOT ────────────────
//
// A page carrying the assembler GLUE and not its WASM must report an ABSENT assembler
// — not a working one, and not a crash. Both failures are easy and neither is visible
// from the arms above: `loadSadhana(un(""))` on an empty string is a
// `CompileError` that takes the whole page down before a button exists, and a glue
// spliced with no module at all would leave every line reading as though the browser
// had assembled something.
//
// DRIVEN BY MUTATION, because no build produces this page: `build-sassembly-web.sh`
// always splices both. So the body is re-run with `SADHANA_WASM_B64` emptied, which is
// precisely the page someone gets by adding one placeholder and forgetting the other.
{
  const mutated = body.replace(/^const SADHANA_WASM_B64 = "[^"]*";$/m,
    'const SADHANA_WASM_B64 = "";');
  say(mutated !== body && /const SADHANA_WASM_B64 = "";/.test(mutated),
    'the no-module page can be constructed — the template binds its module on one line');
  byId.clear();
  created.length = 0;
  const d2 = mkdtempSync(join(tmpdir(), 'sas-page-nowasm-'));
  const m2 = join(d2, 'page-script.mjs');
  writeFileSync(m2, mutated);
  let threw = null;
  try {
    await import(pathToFileURL(m2).href);
  } catch (e) {
    threw = e;
  }
  rmSync(d2, { recursive: true, force: true });
  say(threw === null,
    'a page whose assembler glue is spliced without its wasm module still OPENS',
    threw ? `it threw ${threw.message}` : 'no throw');
  if (threw === null) {
    const a = byId.get('assembler'), as = byId.get('asm');
    say(/\bABSENT\b/.test(a.textContent),
      'and says the assembler is ABSENT, by that word',
      JSON.stringify(a.textContent.slice(0, 44)));
    say(!/is in this tab/.test(a.textContent) && a.className === 'na',
      'and does NOT read as a page that assembles');
    // The auto-clicked proof has already filled `#asm` by here, so this is the real
    // per-program line and not an untouched element.
    say(as.textContent.length > 0 && !/IDENTICAL/.test(as.textContent)
        && !/DIVERGED/.test(as.textContent),
      'and claims neither agreement nor divergence, because nothing was re-assembled',
      JSON.stringify(as.textContent.slice(0, 50)));
    say(byId.get('uart').textContent.includes(NAMASTE),
      'and the programs still RUN — their ELFs were assembled natively at build time');
  }
}

// ── THE CASE THAT MUST STILL BE REFUSED: A STALE ASSEMBLER THAT STILL AGREES ────
//
// This is the failure the arms above cannot reach, and it is the one a generated page
// actually has: served months later from a `sadhana-wasm` built from a different
// `crates/sadhana/src` than the native `sadhana` that assembled the ELFs travelling
// with it. EVERY BUTTON STILL READS `IDENTICAL` IN THAT PAGE — the wasm re-assembles
// the source its own way and the ELF beside it was assembled the same way, because
// both came out of the same stale build. Agreement is self-consistency; it is not
// provenance. So the only instrument that can see it is the pair of stamps, and the
// only way to test that instrument is to make the stamps disagree.
//
// TWO MUTATIONS, because staleness arrives in two shapes:
//   REV      the recorded native stamp differs from the module's sealed one
//   NO STAMP the module has no `sadhana_rev_*` export at all — a wasm from before
//            stamping existed, which answers nothing rather than answering wrong
// Both must be reported, both while all programs still match, and a `.na` on either
// would be the page calling a gap a result.
const rerun = async (label2, mutate, expect, wantClass = 'bad') => {
  const mutated = mutate(body);
  say(mutated !== body, `the ${label2} page can be constructed by mutation`);
  byId.clear();
  created.length = 0;
  const d = mkdtempSync(join(tmpdir(), 'sas-page-prov-'));
  const m = join(d, 'page-script.mjs');
  writeFileSync(m, mutated);
  let threw = null;
  try {
    await import(pathToFileURL(m).href);
  } catch (e) {
    threw = e;
  }
  rmSync(d, { recursive: true, force: true });
  say(threw === null, `a page with a ${label2} assembler still OPENS`,
    threw ? `it threw ${threw.message}` : 'no throw');
  if (threw !== null) return;
  const a2 = byId.get('assembler');
  say(expect.test(a2.textContent), `and says so: ${expect}`,
    JSON.stringify(a2.textContent.slice(-150)));
  say(a2.className === wantClass,
    wantClass === 'bad'
      ? 'and is RED over it — a page that cannot name its own assembler has not earned '
        + 'the claim'
      : `and the line is \`${wantClass}\` — a GAP takes neither the green of SAME nor `
        + 'the red of STALE, because it is not a verdict about staleness at all',
    `class=${a2.className}`);
  // THE WHOLE POINT, asserted and not assumed: the programs still agree. If this ever
  // goes the other way the mutation has broken the assembler instead of its
  // provenance, and the arms above would be proving nothing about staleness at all.
  const btns = created.filter((el) => el.tag === 'button');
  const asm2 = byId.get('asm');
  let identical = 0;
  for (const b of btns) {
    if (b.textContent.endsWith('.sas')) {
      b.click();
      if (/IDENTICAL/.test(asm2.textContent)) identical += 1;
    }
  }
  say(identical === names.filter((n) => embedded[n].kind !== 't1').length,
    `and EVERY program still re-assembles IDENTICAL in that very page (${identical})`,
    'which is exactly why agreement cannot be the figure');
};

// ── THE INTERPRETER NAMES ITSELF ON LOAD, LIKE THE ASSEMBLER ───────────────────
//
// Asserted on the page as built, before any mutation, so the refusal arms below have a
// green starting point to be a refusal OF. `.textContent` and not only the class: a line
// that is the right colour and says nothing is the hole the halt line already taught.
say(/SAME|is [0-9a-f]{16}/.test(interpEl.textContent),
  'the page names WHICH `yantra` ran these programs, by stamp',
  JSON.stringify(interpEl.textContent.slice(0, 64)));
say(interpEl.className === 'ok',
  'and is GREEN over it — the module in the tab and the native run that measured the '
    + 'sizing figures are one `crates/yantra/src`', `class=${interpEl.className}`);

// ── THE CASE THAT MUST STILL BE REFUSED: A STALE INTERPRETER THAT IS STILL RIGHT ─
//
// The assembler's refusal below rests on every program still re-assembling IDENTICAL.
// The interpreter's is WORSE AND MUST BE TESTED DIFFERENTLY, because there is no
// comparison on the page to be self-consistent: the halt status, the step count and the
// UART text are this module's OWN output, so a stale `yantra-wasm` does not disagree with
// anything — it simply is not the binary whose `ram` and `touches` the sizing line quotes.
//
// So the assertion is the exact opposite shape of a bug hunt: the three figures must come
// out BYTE FOR BYTE what the unmutated page produced, AND the page must be red anyway.
// If a mutation ever moved one of them, it would have broken the interpreter rather than
// its provenance, and the arm would be proving nothing about staleness.
//
// Captured from the auto-clicked proof before anything else runs, so the baseline is the
// page as built and not a later press.
const freshRun = { halt: n.halt, steps: n.steps, uart: n.uart };
const rerunInterp = async (label2, mutate, expect, wantClass = 'bad') => {
  const mutated = mutate(body);
  say(mutated !== body, `the ${label2} interpreter page can be constructed by mutation`);
  byId.clear();
  created.length = 0;
  const d = mkdtempSync(join(tmpdir(), 'sas-page-interp-'));
  const m2 = join(d, 'page-script.mjs');
  writeFileSync(m2, mutated);
  let threw = null;
  try {
    await import(pathToFileURL(m2).href);
  } catch (e) {
    threw = e;
  }
  rmSync(d, { recursive: true, force: true });
  say(threw === null, `a page with a ${label2} interpreter still OPENS`,
    threw ? `it threw ${threw.message}` : 'no throw');
  if (threw !== null) return;
  const i2 = byId.get('interp');
  say(expect.test(i2.textContent), `and says so: ${expect}`,
    JSON.stringify(i2.textContent.slice(0, 150)));
  say(i2.className === wantClass,
    wantClass === 'bad'
      ? 'and is RED over it — a page that cannot name the interpreter that ran it has not '
        + 'earned the figures beside it'
      : `and the line is \`${wantClass}\` — a GAP takes neither the green of SAME nor `
        + 'the red of STALE, because it is not a verdict about staleness at all',
    `class=${i2.className}`);
  // ॥ AND ALL THREE RUN FIGURES ARE STILL EXACTLY RIGHT ॥ This is the whole point. Not
  // "still plausible" and not "still green" — identical to the fresh page's, because a
  // verdict that only fired when a run looked wrong would never fire on the real failure.
  const got = { halt: byId.get('halt').textContent, steps: byId.get('steps').textContent,
    uart: byId.get('uart').textContent };
  for (const k of ['halt', 'steps', 'uart']) {
    say(got[k] === freshRun[k],
      `and the ${k} is byte-for-byte what the fresh page produced — a stale interpreter `
        + 'that is still right about three small programs is the state this cannot rest on',
      JSON.stringify(got[k].slice(0, 48)));
  }
  say(got.uart.includes(NAMASTE),
    'and the UART text is still the Devanagari itself, not an empty pane that would have '
      + 'made the red look earned');
};

await rerunInterp(
  'stale-stamp',
  (b) => b.replace(/^const YANTRA_NATIVE_REV = "[^"]*";$/m,
    'const YANTRA_NATIVE_REV = "0000000000000000";'),
  /STALE INTERPRETER/,
);
// A MODULE WITH NO STAMP AT ALL — mutating the glue's loader and not the data, for the
// same reason as the assembler's: no build makes such a module any more, and copying the
// exports without the two stamp functions is what a pre-stamping `yantra_wasm.wasm`
// instantiates to. It still RUNS everything. That is the trap.
await rerunInterp(
  'unstamped-module',
  (b) => b.replace('  return new Yantra(instance);',
    '  const ex = { ...instance.exports };\n'
    + '  delete ex.yantra_rev_ptr; delete ex.yantra_rev_len;\n'
    + '  return new Yantra({ exports: ex });'),
  /exports NO STAMP/,
);

// ── ॥ A CONSTANT WATERMARK MUST RED THE SIZING LINE AND NOTHING ELSE ॥ ──────────
//
// The shape is `rerunInterp`'s and the reason is the same: the page must go RED while
// every other figure stays BYTE FOR BYTE what the fresh page produced. A verdict that
// only fired when a run also LOOKED wrong would never fire on the real failure, which is
// an interpreter that is confidently wrong about one quantity and right about the rest.
//
// The mutation is in the glue's LOADER and not in the data, like the unstamped-module
// arms: `yantra_water` is replaced with a function answering one constant, which is
// exactly what a module measuring the last store instead of the high water, or carrying a
// stale static, hands back. `yantra_water_known` is left alone — the point is a reading
// that EXISTS and is wrong, not a missing one, which `check-yantra-glue.mjs`'s arm 18
// already covers.
//
// It needs the fourth button, so it is UNRUN and says so when `T1_IMAGE` was unset.
if (freshT1 === null) {
  console.log('  ----  the water refusal arm is UNRUN: no T1 button on this page '
    + '(build with T1_IMAGE set to run it)');
} else {
  const mutated = body.replace('  return new Yantra(instance);',
    '  const ex = { ...instance.exports };\n'
    + '  ex.yantra_water = () => 4096;\n'
    + '  return new Yantra({ exports: ex });');
  say(mutated !== body, 'the constant-watermark page can be constructed by mutation');
  byId.clear();
  created.length = 0;
  const d = mkdtempSync(join(tmpdir(), 'sas-page-water-'));
  const m3 = join(d, 'page-script.mjs');
  writeFileSync(m3, mutated);
  let threw = null;
  try {
    await import(pathToFileURL(m3).href);
  } catch (e) {
    threw = e;
  }
  rmSync(d, { recursive: true, force: true });
  say(threw === null, 'a page with a constant watermark still OPENS',
    threw ? `it threw ${threw.message}` : 'no throw');
  if (threw === null) {
    const b2 = created.filter((el) => el.tag === 'button')
      .find((x) => x.textContent === label(t1Name));
    if (!b2) die('the mutated page carries no T1 button — the arm cannot press it');
    b2.click();
    const sz = byId.get('sizing'), hl = byId.get('halt'), st = byId.get('steps');
    const ut = byId.get('uart');
    say(/DISAGREEMENT: this tab touched 4,096/.test(sz.textContent),
      'and the sizing line NAMES the disagreement with the constant in it',
      JSON.stringify(sz.textContent.slice(-130)));
    say(sz.className === 'bad',
      'and is RED over it — the two engines were handed the same program and do not agree',
      `class=${sz.className}`);
    // The native figure must still be there. A line that dropped it would be red for a
    // reason the reader cannot check.
    say(sz.textContent.includes(`not the ${grpTop(embedded[t1Name].touches)} measured natively`),
      'and still quotes the native figure it disagrees WITH, so the red can be checked');
    // ॥ AND EVERY OTHER FIGURE IS UNMOVED ॥
    const got = { halt: hl.textContent, steps: st.textContent, uart: ut.textContent };
    for (const k of ['halt', 'steps', 'uart']) {
      say(got[k] === freshT1[k],
        `and the ${k} is byte-for-byte what the fresh page's T1 press produced — the `
          + 'watermark is the only thing this mutation moved',
        JSON.stringify(got[k].slice(0, 48)));
    }
    // And the arm DISCRIMINATES: a constant that happens to BE the right figure must be
    // green, or the arm would be asserting "any wrapper reds" rather than "a wrong
    // watermark reds".
    byId.clear();
    created.length = 0;
    const d2 = mkdtempSync(join(tmpdir(), 'sas-page-water-ok-'));
    const m4 = join(d2, 'page-script.mjs');
    writeFileSync(m4, body.replace('  return new Yantra(instance);',
      '  const ex = { ...instance.exports };\n'
      + `  ex.yantra_water = () => ${embedded[t1Name].touches};\n`
      + '  return new Yantra({ exports: ex });'));
    let threw2 = null;
    try { await import(pathToFileURL(m4).href); } catch (e) { threw2 = e; }
    rmSync(d2, { recursive: true, force: true });
    say(threw2 === null, 'a page whose wrapper answers the RIGHT figure still opens',
      threw2 ? `it threw ${threw2.message}` : 'no throw');
    if (threw2 === null) {
      created.filter((el) => el.tag === 'button')
        .find((x) => x.textContent === label(t1Name)).click();
      const sz2 = byId.get('sizing');
      say(sz2.className === '' && !/DISAGREEMENT/.test(sz2.textContent),
        'and is GREEN — so the arm above reds on the WRONG figure and not on the wrapper',
        `class=${JSON.stringify(sz2.className)}`);
    }
  }
}

await rerun(
  'stale-stamp',
  (b) => b.replace(/^const SADHANA_NATIVE_REV = "[^"]*";$/m,
    'const SADHANA_NATIVE_REV = "0000000000000000";'),
  /STALE ASSEMBLER/,
);
// A MODULE WITH NO STAMP AT ALL. Mutating the glue's loader and not the data, because
// no build makes such a module any more: the exports object is copied without the two
// stamp functions, which is what a pre-stamping `sadhana_wasm.wasm` instantiates to.
// It still assembles — that is the trap.
await rerun(
  'unstamped-module',
  (b) => b.replace('  return new Sadhana(instance);',
    '  const ex = { ...instance.exports };\n'
    + '  delete ex.sadhana_rev_ptr; delete ex.sadhana_rev_len;\n'
    + '  return new Sadhana({ exports: ex });'),
  /PROVENANCE: NO STAMP/,
);

// ── ॥ THE FOURTH VERDICT: A GAP IS NOT A RESULT, AND IT IS NOT A REFUSAL EITHER ॥ ──
//
// The three arms above drive SAME (the unmutated page), STALE and NO STAMP. The
// provenance figure has FOUR states and `grep -c UNKNOWN` over this file was **0**, so
// the one state that is NEITHER `ok` NOR `bad` was executed by nothing — on either half.
//
// UNKNOWN is the arm reached when one side's build could not take a stamp: `build.rs` in
// both `*-wasm` crates falls back to the literal seven octets `unknown` on a machine with
// no sha256 tool (`crates/sadhana-wasm/build.rs:32`, `crates/yantra-wasm/build.rs:45`),
// and `build-stamp.rs:78` (retired to archive/rust/build-stamp/) does the same for the native side. It is `na` by design —
// `web/sassembly.template.html:162` calls it "a gap and not a verdict" — and that is
// exactly why it needs arms the other three cannot give it:
//
//   A GREEN HERE WOULD BE THE WORST OUTCOME ON THE PAGE. `ok` on this line reads as "the
//   same assembler", which is the strongest claim the page makes, asserted from two stamps
//   that establish nothing. `bad` would be the opposite error and cheaper — it cries
//   staleness where staleness is merely unknowable — but it is still a verdict the stamps
//   did not earn.
//
// TWO ENTRY CONDITIONS, OR-ED TOGETHER, so one arm cannot cover the state:
//   theirs   the recorded NATIVE stamp is `unknown` or EMPTY — a data mutation, and
//            precisely what the build writes when the native side had no sha256
//   mine     the MODULE's own sealed stamp is `unknown` — a loader mutation
// Both, on both halves, is six arms; and every one of them must hold the invariant the
// three arms above hold, that the page is otherwise completely right.
//
// WHY `mine` IS A GETTER OVERRIDE AND NOT OCTETS IN MEMORY: no build on this host can be
// made without sha256, and the subject here is `provenance()`'s CLASSIFICATION of the
// value, not the read that produces it. The read itself — the two exports, the pointer,
// the length, and the seven-octet `unknown` case specifically — is pinned by
// `check-sadhana-glue.mjs` and `check-yantra-glue.mjs` against stand-ins that PLACE the
// stamp. So the two halves of `*_rev_*` are each asserted where they live, and neither
// check is asserting the other's half on trust.

// ── the ELFs' stamp is `unknown`: the native side had no sha256 ────────────────
await rerun(
  'unstampable-native',
  (b) => b.replace(/^const SADHANA_NATIVE_REV = "[^"]*";$/m,
    'const SADHANA_NATIVE_REV = "unknown";'),
  /PROVENANCE: UNKNOWN — this tab's assembler is [0-9a-f]{16} and the ELFs' is unknown;/,
  'na',
);
// ── and the EMPTY spelling, which is a different octet string and the same gap ─
// `!theirs` is its own disjunct in the template and answers `unrecorded` rather than
// echoing the empty string, so an arm that only drove `"unknown"` would leave the `||`
// untested and a page that printed `the ELFs' is ;` would pass.
await rerun(
  'unrecorded-native',
  (b) => b.replace(/^const SADHANA_NATIVE_REV = "[^"]*";$/m,
    'const SADHANA_NATIVE_REV = "";'),
  /PROVENANCE: UNKNOWN — this tab's assembler is [0-9a-f]{16} and the ELFs' is unrecorded;/,
  'na',
);
// ── the MODULE's own stamp is `unknown`: the wasm build had no sha256 ──────────
await rerun(
  'unstampable-module',
  (b) => b.replace('  return new Sadhana(instance);',
    '  const s0 = new Sadhana(instance);\n'
    + '  Object.defineProperty(s0, "rev", { get: () => "unknown" });\n'
    + '  return s0;'),
  /PROVENANCE: UNKNOWN — this tab's assembler is unknown and the ELFs' is [0-9a-f]{16};/,
  'na',
);

await rerunInterp(
  'unstampable-native',
  (b) => b.replace(/^const YANTRA_NATIVE_REV = "[^"]*";$/m,
    'const YANTRA_NATIVE_REV = "unknown";'),
  /interpreter that measured the sizing figures is unknown; one side could not be stamped/,
  'na',
);
await rerunInterp(
  'unrecorded-native',
  (b) => b.replace(/^const YANTRA_NATIVE_REV = "[^"]*";$/m,
    'const YANTRA_NATIVE_REV = "";'),
  /interpreter that measured the sizing figures is unrecorded; one side could not be stamped/,
  'na',
);
await rerunInterp(
  'unstampable-module',
  (b) => b.replace('  return new Yantra(instance);',
    '  const y0 = new Yantra(instance);\n'
    + '  Object.defineProperty(y0, "rev", { get: () => "unknown" });\n'
    + '  return y0;'),
  /the interpreter in this tab is unknown and the interpreter that measured the sizing/,
  'na',
);

// ── ॥ THE CONTROL: A VERDICT WRONGLY CLASSED `ok` MUST BE RED ॥ ────────────────
//
// The six arms above assert `className === 'na'`. On their own that proves nothing about
// whether the assertion is LOAD-BEARING: if the page painted every gap green they would
// all have to fail, and the only way to know they would is to build that page and watch
// the predicate reject it.
//
// AND THE ASSEMBLER HALF IS WHERE THAT MISTAKE IS ONE CHARACTER AWAY. Its UNKNOWN arm is
// the only one of the four that returns a BARE STRING where the other three return a
// `[text, class]` pair, and the destructure special-cases it:
//
//   const [provText, provClass] = typeof prov === 'string' ? [prov, 'na'] : (prov || …)
//
// Both halves of that line are a hazard, and they fail in OPPOSITE directions:
//   the default   `'na'` written `'ok'` — the gap keeps its full text and goes GREEN,
//                 so a reader sees the page's strongest claim over two stamps that
//                 establish nothing. No arm but the class arm can see it.
//   the branch    dropped altogether — a bare string destructures CHARACTER BY
//                 CHARACTER, so `provText` is one space and `provClass` is `'P'`. The
//                 verdict text is gone and the class is not a class at all. No arm but
//                 the TEXT arm can see that one.
// Both controls therefore assert the page comes out WRONG in the specific way named, and
// the six arms above are what refuse it.
const control = async (label2, mutate, probe) => {
  const mutated = mutate(body);
  say(mutated !== body, `the control page (${label2}) can be constructed by mutation`);
  byId.clear();
  created.length = 0;
  const d = mkdtempSync(join(tmpdir(), 'sas-page-prov-control-'));
  const m5 = join(d, 'page-script.mjs');
  writeFileSync(m5, mutated);
  let threw = null;
  try {
    await import(pathToFileURL(m5).href);
  } catch (e) {
    threw = e;
  }
  rmSync(d, { recursive: true, force: true });
  say(threw === null, `the control page (${label2}) OPENS`,
    threw ? `it threw ${threw.message}` : 'no throw');
  if (threw === null) probe();
};

// The destructure's one line, verbatim from the template. Asserted present before it is
// mutated: a template edit that reflows it would otherwise silently stop constructing
// these two controls, and `mutated !== body` is the only thing that would say so.
const PROV_DESTRUCTURE = "const [provText, provClass] = typeof prov === 'string' "
  + "? [prov, 'na'] : (prov || ['', 'na']);";
say(body.includes(PROV_DESTRUCTURE),
  'the page carries the bare-string special case the two controls below mutate',
  JSON.stringify(PROV_DESTRUCTURE.slice(0, 48)));

await control(
  'a gap painted GREEN',
  (b) => b.replace(/^const SADHANA_NATIVE_REV = "[^"]*";$/m,
    'const SADHANA_NATIVE_REV = "unknown";')
    .replace(PROV_DESTRUCTURE, PROV_DESTRUCTURE.replace("[prov, 'na']", "[prov, 'ok']")),
  () => {
    const a3 = byId.get('assembler');
    say(a3.className === 'ok' && /PROVENANCE: UNKNOWN/.test(a3.textContent),
      'a page that defaults the gap to `ok` reads UNKNOWN in GREEN — which is what the '
        + 'six class arms above REFUSE, and they could not be refusing it if it were '
        + 'unreachable', `class=${a3.className}`);
  },
);
await control(
  'the bare-string branch dropped',
  (b) => b.replace(/^const SADHANA_NATIVE_REV = "[^"]*";$/m,
    'const SADHANA_NATIVE_REV = "unknown";')
    .replace(PROV_DESTRUCTURE,
      "const [provText, provClass] = (prov || ['', 'na']);"),
  () => {
    const a3 = byId.get('assembler');
    say(!/PROVENANCE/.test(a3.textContent) && a3.className === 'P',
      'and dropping the branch destructures the bare string character by character — the '
        + 'verdict text is GONE and the class is `P`, which the TEXT arms above refuse',
      `class=${JSON.stringify(a3.className)} tail=${JSON.stringify(a3.textContent.slice(-12))}`);
  },
);
// The interpreter's UNKNOWN arm returns a pair, so its hazard is not the destructure —
// it is the literal in the pair. Same control, same direction: green where a gap belongs.
const INTERP_NA = "same interpreter is not established either way. The run below is still "
  + "a real run.',\n    'na'];";
say(body.includes(INTERP_NA),
  'the page carries the interpreter gap\'s own class literal',
  JSON.stringify("'na'];"));
await control(
  'the interpreter gap painted GREEN',
  (b) => b.replace(/^const YANTRA_NATIVE_REV = "[^"]*";$/m,
    'const YANTRA_NATIVE_REV = "unknown";')
    .replace(INTERP_NA, INTERP_NA.replace("'na'];", "'ok'];")),
  () => {
    const i3 = byId.get('interp');
    say(i3.className === 'ok' && /not established either way/.test(i3.textContent),
      'the interpreter half fails the same way and its class arm refuses it too',
      `class=${i3.className}`);
  },
);

if (bad) {
  console.error(`\n  FAIL  ${bad} check(s) — the generated page does not do what it says.`);
  process.exit(1);
}
console.log(`the page's own script runs and ${names.length} programs fill the pane each one owns.`);
