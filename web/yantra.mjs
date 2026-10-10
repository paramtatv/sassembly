// Hand-written glue for the yantra wasm module. No wasm-bindgen, no generated bindings.
// The whole protocol is a handful of exported functions and one shared linear memory.
export async function load(wasmBytes) {
  const { instance } = await WebAssembly.instantiate(wasmBytes, {});
  return new Yantra(instance);
}
const EMPTY = new Uint8Array(0);
const enc = new TextEncoder();
export class Yantra {
  constructor(instance) {
    this.e = instance.exports;
    // The module exports its own memory; that buffer is the only thing JS and Rust share.
    this.mem = () => new Uint8Array(this.e.memory.buffer);
  }

  // ॥ WHICH INTERPRETER THIS MODULE IS, AND `null` WHEN IT WILL NOT SAY ॥
  //
  // `yantra_rev_*` hands back the 16-hex stamp `crates/yantra-wasm/build.rs` sealed in
  // over `crates/yantra/src`. The page compares it against the stamp the build recorded
  // for the NATIVE `yantra` whose `ram` and `touches` the sizing line quotes.
  //
  // THE INTERPRETER HAS NO SECOND OPINION TO DIFFER FROM, which is why this matters more
  // here than for the assembler. Every button re-assembles its source and compares the
  // octets, so a wrong assembler can at least be caught disagreeing; the halt status, the
  // step count and the UART text are THIS module's own output and nothing else on the page
  // produces them. A stale interpreter that happens to be right about three small programs
  // is indistinguishable from a fresh one by any figure the page shows.
  //
  // THREE ANSWERS AND NOT TWO, because the truth has three. A stamp is a stamp;
  // `'unknown'` is the module saying its build could not take one (no sha256 tool on that
  // machine); `null` is the module having NO SUCH EXPORT, which is the staleness this is
  // here for — a `yantra_wasm.wasm` built before stamping existed, spliced into a page
  // today, answers nothing at all. A loader that treated a missing export as "matches"
  // would hide precisely that module.
  //
  // BOTH HALVES OF THE PAIR ARE PROBED, AND THE ONE-SIDED FORM WAS A HALF-GUARD.
  // The twin defect of `web/sadhana.mjs`'s, fixed in the same cycle because the
  // pair is one subject: this tested `yantra_rev_ptr` and then called
  // `yantra_rev_len()` unguarded, so the three answers were three only while the
  // pair was all-or-nothing. Measured 2026-10-05 over the real module, exports
  // minus `yantra_rev_len` alone: `THREW this.e.yantra_rev_len is not a
  // function`, out of the accessor whose whole contract is to answer `null`
  // instead. The page reads `y.rev` at top level, so that is a blank tab in
  // place of a named provenance gap.
  get rev() {
    if (typeof this.e.yantra_rev_ptr !== 'function'
        || typeof this.e.yantra_rev_len !== 'function') return null;
    const ptr = this.e.yantra_rev_ptr(), len = this.e.yantra_rev_len();
    return new TextDecoder().decode(this.mem().slice(ptr, ptr + len));
  }
  // Boot the image: reset vector, S-mode, no loader and no supervisor. `out` is the
  // machine's own UART and `surface` is empty, because nothing granted one.
  //
  // `input` is the octets a program reads out of `निवेशमण्डल` and `inputName` is the module
  // name it attributes them to — both or neither. A string name is encoded UTF-8,
  // which is what the native runner passes `YANTRA_INPUT_NAME` as.
  run(elf, { ram = 1 << 20, budget = 1_000_000, input = null, inputName = null } = {}) {
    this.#write(elf);
    this.#feed(input, inputName);
    return this.#read(this.e.yantra_run(ram, budget));
  }
  // Host the image as an APPLICATION: a loader places it, a supervisor answers its calls,
  // and it never leaves U-mode. `surface` is what it wrote to the surface it was handed;
  // `out` is the machine's UART, which stays EMPTY because an application cannot address
  // a device (ADR-0015 A4). They are separate fields for that reason and must be shown
  // separately — merged, they say the opposite of what the ABI guarantees.
  //
  // The codes are a different space from run()'s: 0 exited 0, 1 exited non-zero, 2
  // faulted out of U-mode, 3 the machine stopped, 4 out of budget, 6 would not load.
  // 4 MiB and 200 steps are the defaults `crates/yantra/tests/host.rs` uses.
  //
  // THERE IS NO `input` HERE AND PASSING ONE IS AN ERROR, not an ignored option.
  // `yantra_host` reads neither input buffer — it calls `host::host` and never
  // `input::inject` — so a slab handed to this entry point would be written into
  // linear memory, never placed in the guest's RAM, and the program would compile
  // an empty source and halt looking like a verdict about it. Silently dropping it
  // is the one outcome worth refusing for.
  host(elf, { ram = 1 << 22, budget = 200, input = null, inputName = null } = {}) {
    if (input !== null || inputName !== null) {
      throw new Error(
        'host(): an application is not fed through the input channel — yantra_host places ' +
          'no slab. Use run() for a program that reads निवेशमण्डल.',
      );
    }
    this.#write(elf);
    return this.#read(this.e.yantra_host(ram, budget), true);
  }
  // ॥ AN IN-MEMORY FILE ROOT, FOR A BROWSER RUN ॥
  // A tab has no directory for `--files`, so the program's file window is served from a
  // root held in the module (`yantra::patra::MemFs`): same path rules, same refusals by
  // name, caps instead of a disk. `memfsEnable()` makes an EMPTY root (and discards any
  // earlier one, so call it before EVERY run that should start clean); `memfsPut` seeds a
  // file; after `run()` the files the program wrote are `memfsFiles()`. Without
  // `memfsEnable()` every file request is refused, exactly as natively without `--files`.
  // `caps` is `{ total, file }` in octets; the defaults are 64 MiB and 16 MiB (names and
  // per-entry overhead count against the total; at most 4096 files). The root is CARRIED
  // across runs until the next memfsEnable.
  memfsEnable(caps = null) {
    if (caps === null) this.e.yantra_memfs_enable();
    else if (this.e.yantra_memfs_enable_caps(caps.total >>> 0, caps.file >>> 0) !== 0) {
      throw new Error('memfsEnable: the per-file cap is above the total cap');
    }
  }
  memfsDisable() { this.e.yantra_memfs_disable(); }
  // Throws NAMING the refusal; the codes are `yantra_memfs_put`'s.
  memfsPut(name, data) {
    const n = typeof name === 'string' ? enc.encode(name) : name;
    const at = this.e.yantra_memfs_alloc(n.length + data.length);
    this.mem().set(n, at);
    this.mem().set(data, at + n.length);
    const code = this.e.yantra_memfs_put(at, n.length, at + n.length, data.length);
    if (code !== 0) {
      const why = {
        1: 'no root enabled', 2: 'span outside scratch', 3: 'name is not UTF-8',
        4: 'refused (path escapes the root)', 5: 'not written (over a cap)',
      }[code] ?? `code ${code}`;
      throw new Error(`memfsPut(${typeof name === 'string' ? name : '<bytes>'}): ${why}`);
    }
  }
  // Copies: the views are over linear memory, which the next call may grow or reuse.
  memfsFiles() {
    const dec = new TextDecoder();
    const out = [];
    for (let i = 0, n = this.e.yantra_memfs_count(); i < n; i++) {
      const np = this.e.yantra_memfs_name(i), nl = this.e.yantra_memfs_name_len(i);
      const dp = this.e.yantra_memfs_data(i), dl = this.e.yantra_memfs_data_len(i);
      out.push({
        name: dec.decode(this.mem().slice(np, np + nl)),
        data: this.mem().slice(dp, dp + dl),
      });
    }
    return out;
  }
  #write(elf) {
    const at = this.e.yantra_alloc(elf.length);
    this.mem().set(elf, at);          // JS writes the ELF straight into wasm memory
  }
  // ॥ A RUN WITH NO INPUT MUST CLEAR THE LAST ONE'S ॥ The slab and its name are
  // MODULE STATICS (`lib.rs:109`, `:118`) and outlive the call that set them, so
  // feeding one program and then running a second with no input would place the
  // FIRST program's octets in the second's RAM. Natively each run is a process and
  // starts with no input, so this re-establishes that: the allocs are called on
  // every run, with 0 when there is nothing, and `yantra_input_alloc(0)` is
  // documented to clear the slab.
  //
  // A SLAB WITH NO NAME IS PASSED THROUGH AND NOT CHECKED HERE. `plan_input`
  // refuses it with code 13 and that refusal is already native parity; a second
  // copy of the rule in JS is a second thing to drift.
  #feed(input, name) {
    const octets = (v) => (v === null ? EMPTY : typeof v === 'string' ? enc.encode(v) : v);
    const slab = octets(input), named = octets(name);
    // Each write re-reads `mem()`: an alloc can grow linear memory, which detaches
    // the previous `ArrayBuffer` and leaves any view over it zero-length.
    const at = this.e.yantra_input_alloc(slab.length);
    if (slab.length) this.mem().set(slab, at);
    const nameAt = this.e.yantra_input_name_alloc(named.length);
    if (named.length) this.mem().set(named, nameAt);
  }
  // ॥ THE STEP COUNT IS THREE-STATE AND THE GLUE MUST NOT FLATTEN IT ॥
  // `yantra_steps_lo`/`_hi` (`lib.rs:410`, `:416`) are one `u64` split in two, and the
  // pair reads `0:0` BOTH after a run that never happened AND after a program that
  // retired nothing. `yantra_steps_known` is the module's own way of telling those
  // apart, so this consults it FIRST and reports `null` for "no reading" — never `0`.
  //
  // `host()` NEVER has a reading: `host::Hosted` carries no counter, so `yantra_host`
  // clears `STEPS` before it runs (`lib.rs:355`). `steps` is therefore `null` on every
  // hosted result — but the field is PRESENT, so a page reads a null it can caption
  // rather than an `undefined` it will print as the word.
  #steps() {
    // A module without the exports is a THIRD thing, not "no reading". Returning
    // `null` for it would make a stale `.wasm` look like an ordinary hosted run, and
    // the page would caption a build failure as a property of the ABI.
    if (typeof this.e.yantra_steps_known !== 'function') {
      throw new Error(
        'this wasm module exports no yantra_steps_known — it predates the step counter. ' +
          'Reporting null here would hide a stale build behind "no reading".',
      );
    }
    if (this.e.yantra_steps_known() !== 1) return null;
    const hi = this.e.yantra_steps_hi() >>> 0, lo = this.e.yantra_steps_lo() >>> 0;
    // A Number and not a BigInt, because the page formats this and `JSON.stringify`
    // THROWS on a BigInt. `hi * 2**32 + lo` is exact while the count stays under
    // 2**53 — the T1 image retires 1.2e8 and ten FLAC frames 9.4e10, both far inside
    // it. Past that a double rounds, so refuse rather than display a wrong figure
    // that still looks like a measurement.
    const steps = hi * 2 ** 32 + lo;
    if (!Number.isSafeInteger(steps)) {
      throw new Error(
        `the step count hi=${hi} lo=${lo} is past 2**53; a double cannot hold it exactly, ` +
          'so no honest Number is available for it.',
      );
    }
    return steps;
  }
  // ॥ THE HIGH WATER IS THE SAME THREE STATES, AND IT IS NOT THE STEP COUNT ॥
  // `yantra_water` (`lib.rs:517`) is the highest RAM offset any store of the last
  // `yantra_run` reached, measured by the module's `Watermarked` sink with the SAME
  // arithmetic as `bin/yantra-run.rs:26`. That is the whole point of the export: the
  // page quotes a native `touches` beside it, and two different rules would make the
  // comparison meaningless.
  //
  // `yantra_water_known` is consulted FIRST, because `0` IS A LEGITIMATE READING here —
  // a program that stores nothing really did touch nothing — and is not the absence of
  // one. `host()` never has a reading: `yantra_host` clears `WATER` beside `STEPS`
  // (`lib.rs:436`), so `water` is `null` on every hosted result with the field present.
  //
  // ONE WORD AND NO `lo`/`hi` PAIR, so there is no 2**53 ceiling to refuse at: a store
  // is bounds-checked against `ram`, itself a `u32`, and the module's own narrowing is
  // `u32::try_from(..).ok()` — an impossible over-wide water arrives ABSENT, never as a
  // truncated small number that would read as a measurement.
  #water() {
    // A module without the exports is the THIRD thing, exactly as for `#steps()`: a
    // stale `.wasm` spliced into today's page. Returning `null` would caption a build
    // failure as an ordinary hosted run.
    if (typeof this.e.yantra_water_known !== 'function') {
      throw new Error(
        'this wasm module exports no yantra_water_known — it predates the high-water ' +
          'counter. Reporting null here would hide a stale build behind "no reading".',
      );
    }
    if (this.e.yantra_water_known() !== 1) return null;
    return this.e.yantra_water() >>> 0;
  }
  #read(code, hosted = false) {
    const dec = new TextDecoder();
    const span = (ptr, len) => dec.decode(this.mem().slice(ptr, ptr + len));
    return {
      code,
      hosted,
      out: span(this.e.yantra_out_ptr(), this.e.yantra_out_len()),
      surface: span(this.e.yantra_surface_ptr(), this.e.yantra_surface_len()),
      halt: span(this.e.yantra_halt_ptr(), this.e.yantra_halt_len()),
      steps: this.#steps(),
      water: this.#water(),
    };
  }
}
