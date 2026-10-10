// The in-memory file root, driven through the REAL wasm module and the real glue.
// usage: node check-yantra-memfs.mjs <yantra_wasm.wasm> <elf-dir>
// The ELFs are compiled by the .t1 compiler (crates/yantra/tests/memfs_parity.rs leaves
// them in MEMFS_ELF_DIR); the native-vs-memory status/steps parity is asserted there.
import { readFileSync } from 'node:fs';
import { load } from '../web/yantra.mjs';

const [wasm, dir] = process.argv.slice(2);
const enc = new TextEncoder();
let failed = 0;
const check = (ok, what, saw) => {
  if (ok) return;
  console.error(`  FAIL  ${what}${saw === undefined ? '' : `\n        saw: ${saw}`}`);
  failed++;
};
const status = (r) => /exit status (\d+)/.exec(r.halt)?.[1];
const REFUSED = (0xFFFF_FFFF_FFFF_FFFFn - 3n) & 0xFFFF_FFFF_FFFFn; // Status::Refused.word() low 48 bits
const elf = (n) => new Uint8Array(readFileSync(`${dir}/${n}`));
const y = await load(readFileSync(wasm));
const opts = { ram: 8 << 20, budget: 4_000_000_000 };

// 1. A program writes a file; the host reads the exact bytes back.
y.memfsEnable();
let r = y.run(elf('roundtrip.elf'), opts);
check(status(r) === '100', 'roundtrip finishes 100', r.halt);
let files = y.memfsFiles();
check(files.length === 1 && files[0].name === 'खपत्रम्', 'one file, right name', JSON.stringify(files.map((f) => f.name)));
check(files[0]?.data.length === 4 && [10, 20, 30, 40].every((b, i) => files[0].data[i] === b), 'bytes 10,20,30,40', files[0]?.data);

// 2. A seeded file is readable by the program.
y.memfsEnable();
y.memfsPut('कपत्रम्', enc.encode('abc\n'));
r = y.run(elf('reads.elf'), opts);
check(status(r) === '304', 'seeded read sums to 304', r.halt);
// CONTROL: the same program with nothing seeded does not.
y.memfsEnable();
r = y.run(elf('reads.elf'), opts);
check(status(r) !== '304', 'control: unseeded read must not give 304', r.halt);

// 3. Escapes are refused BY NAME, both in the program and at the seeding API.
for (const n of ['esc-dotdot.elf', 'esc-abs.elf']) {
  y.memfsEnable();
  y.memfsPut('क', enc.encode('abc\n'));
  r = y.run(elf(n), opts);
  check(BigInt(status(r) ?? -1) === REFUSED, `${n} is Refused`, r.halt);
}
y.memfsEnable();
for (const bad of ['../x', '/etc/passwd', 'a/../../x', '']) {
  let msg = null;
  try { y.memfsPut(bad, enc.encode('x')); } catch (e) { msg = e.message; }
  check(msg && /refused/.test(msg), `memfsPut(${JSON.stringify(bad)}) refused`, msg);
}
check(y.memfsFiles().length === 0, 'nothing stored by refused puts');

// 4. Caps: over the file cap is named, and no root at all refuses the program.
let incMsg = null;
try { y.memfsEnable({ total: 4, file: 8 }); } catch (e) { incMsg = e.message; }
check(incMsg && /above the total/.test(incMsg), 'file cap above total refused', incMsg);
y.memfsEnable({ total: 8, file: 4 });
let capMsg = null;
try { y.memfsPut('big', new Uint8Array(5)); } catch (e) { capMsg = e.message; }
check(capMsg && /cap/.test(capMsg), 'over the file cap is named', capMsg);
// The two caps are not interchangeable: file 100, total 1000 takes two 90-octet files and
// refuses a 150-octet one; swapped (file 1000, total 100) would refuse the first.
y.memfsEnable({ total: 1000, file: 100 });
y.memfsPut('one', new Uint8Array(90));
y.memfsPut('two', new Uint8Array(90));
let c2 = null;
try { y.memfsPut('big', new Uint8Array(150)); } catch (e) { c2 = e.message; }
check(c2 && /cap/.test(c2), 'file cap 100 refuses 150', c2);
check(y.memfsFiles().map((f) => f.name).join() === 'one,two', 'listing is in name order, two files');
y.memfsDisable();
r = y.run(elf('roundtrip.elf'), opts);
check(status(r) !== '100', 'no root: the write is refused', r.halt);
check(y.memfsFiles().length === 0, 'no root: no files');

if (failed) { console.error(`${failed} FAILED`); process.exit(1); }
console.log('yantra memfs: all arms pass');
