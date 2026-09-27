// Hand-written glue for the sadhana wasm module — the assembler, in the browser.
// The twin of `yantra.mjs` and deliberately the same shape: no wasm-bindgen, no
// generated bindings, one shared linear memory and a handful of exported functions.
//
// The pair is the whole pipeline. `Sadhana.assemble(source)` turns Devanagari
// assembly into an ELF; `Yantra.run(elf)` executes it. Nothing else is needed to
// put Sassembly in a browser tab, and neither module imports anything from the
// host — `WebAssembly.instantiate(bytes, {})` with an EMPTY import object is not
// an oversight, it is the claim: no syscalls, no clock, no network.
export async function load(wasmBytes) {
  const { instance } = await WebAssembly.instantiate(wasmBytes, {});
  return new Sadhana(instance);
}

export class Sadhana {
  // The QEMU `virt` reset vector: where a bare-metal proof is linked.
  static BARE_METAL = 0x80200000;
  // Where an application is linked — `spec/application-load.tsv`, the same number
  // `tools/build-sassembly-web.sh` passes the native CLI as `--स्थान`.
  static APP_LOAD = 0x20000000;

  constructor(instance) {
    this.e = instance.exports;
    this.mem = () => new Uint8Array(this.e.memory.buffer);
  }

  // Assemble Devanagari source, linked at `base`.
  //
  // The default is the QEMU `virt` reset vector, which is right for a bare-metal
  // proof and WRONG for a guest: an application is linked at `Sadhana.APP_LOAD`
  // because 0x80200000 is inside the supervisor's own gigabyte, and `yantra.host`
  // refuses it by name rather than hand an application the supervisor's pages.
  // Pass the address that matches how you intend to run it.
  //
  // Returns `{ ok: true, elf }` or `{ ok: false, error }`, where `error` is the
  // assembler's own diagnostics, one per line, each naming a line and a byte —
  // the refusal the language is built around, handed through unchanged. A caller
  // that shows a generic "failed" in its place has thrown away the lesson.
  assemble(source, base = Sadhana.BARE_METAL) {
    const bytes = new TextEncoder().encode(source);
    // sadhana_alloc reserves LEN BYTES and hands back where to put them; the
    // length is passed again to assemble because the module holds a String and
    // only the caller knows how much of it was filled.
    const at = this.e.sadhana_alloc(bytes.length);
    this.mem().set(bytes, at);

    const dec = new TextDecoder();
    const span = (ptr, len) => this.mem().slice(ptr, ptr + len);

    // Split because a wasm export cannot take a u64. `>>> 0` keeps both halves
    // unsigned; a bare `>>` would sign-extend 0x80200000 into a negative.
    const lo = Number(BigInt(base) & 0xffffffffn) >>> 0;
    const hi = Number(BigInt(base) >> 32n) >>> 0;

    if (this.e.sadhana_assemble(bytes.length, lo, hi) !== 0) {
      return {
        ok: false,
        error: dec.decode(span(this.e.sadhana_err_ptr(), this.e.sadhana_err_len())),
      };
    }
    // A copy, not a view: the next call reuses this memory, and a Uint8Array
    // over the module's buffer would also be detached by any wasm growth.
    return {
      ok: true,
      elf: new Uint8Array(span(this.e.sadhana_out_ptr(), this.e.sadhana_out_len())),
    };
  }
}
