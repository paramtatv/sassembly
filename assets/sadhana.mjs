// Hand-written glue for the sadhana wasm module — the assembler, in the browser.
// The twin of `yantra.mjs` and deliberately the same shape: no wasm-bindgen, no
// generated bindings, one shared linear memory and a handful of exported functions.
//
// The pair is the whole pipeline. `Sadhana.assemble(source)` turns Devanagari
// assembly into an ELF; `Yantra.run(elf)` executes it. Nothing else is needed to
// put Sassembly in a browser tab, and neither module imports anything from the
// host — `WebAssembly.instantiate(bytes, {})` with an EMPTY import object is not
// an oversight, it is the claim: no syscalls, no clock, no network.
//
// ॥ THE LOADER IS `loadSadhana` AND NOT `load`, AND THAT IS NOT A STYLE CHOICE ॥
// `yantra.mjs` also declares a top-level `load`. The page does not IMPORT either
// glue — `build-sassembly-web.sh` strips `^export ` and pastes the body into the
// template's own `<script type="module">` — so the day it splices BOTH, which is
// exactly what `F-004`'s in-browser assembler needs, two top-level `load`s land in
// one module scope and the page dies with a `SyntaxError` before a line of it runs.
// Neither glue's own check could see that: each compared itself against the
// TEMPLATE and never against the other glue. `tools/check-{yantra,sadhana}-glue.sh`
// arm 3 is the reader that now does, and this rename is what it is green on.
// THE ASSEMBLER GLUE IS THE ONE THAT MOVED because `load` is the name the shipped
// page already binds (`sassembly.template.html:6`) and the spliced one keeps it.
export async function loadSadhana(wasmBytes) {
  const { instance } = await WebAssembly.instantiate(wasmBytes, {});
  return new Sadhana(instance);
}

export class Sadhana {
  // ॥ WHERE A BARE-METAL PROOF IS LINKED, AND IT IS `kosha::LOAD_ADDRESS` ॥
  // This read `0x80200000` and called itself "the QEMU `virt` reset vector",
  // which is wrong twice. `crates/sadhana/src/kosha.rs:17` says the `virt`
  // machine's reset vector under `-bios none` jumps to `0x8000_0000`, and
  // `kosha.rs:46` makes that `LOAD_ADDRESS` — `sadhana`'s default, which
  // `tools/build-sassembly-web.sh` takes for a proof by passing no `--स्थान`
  // at all. `0x80200000` is the OpenSBI payload address: right when firmware
  // is already sitting at the default, and NOT what the page's own ELFs are.
  //
  // MEASURED 2026-10-05 BY CONTENT, not argued. `loadSadhana` over the real
  // `sadhana-wasm` module, `assemble(spec/namaste.sas)` at this default:
  //   this glue, at 0x80200000     sha256 a28bccab110408cb, 904 octets
  //   the page's ELF (no --स्थान)  sha256 4032492520bbed48, 904 octets
  //   native `--स्थान ०षोड्८०२०००००`  sha256 a28bccab110408cb
  // Same length, different bytes, and the third line is what proves the base
  // was the WHOLE divergence. So the day `build-sassembly-web.sh` splices this
  // glue, the page would have assembled a different image from the one it
  // ships and called the pair a pipeline. `atithi` was identical throughout,
  // because an application's base comes from `APP_LOAD` and arm 8 had been
  // reading THAT against its spec since the file was written. Arm 11 is the
  // same assertion for this constant, which nothing was making.
  static BARE_METAL = 0x80000000;
  // Where an application is linked — `spec/application-load.tsv`, the same number
  // `tools/build-sassembly-web.sh` passes the native CLI as `--स्थान`.
  static APP_LOAD = 0x20000000;

  constructor(instance) {
    this.e = instance.exports;
    this.mem = () => new Uint8Array(this.e.memory.buffer);
  }

  // ॥ WHICH ASSEMBLER THIS MODULE IS, AND `null` WHEN IT WILL NOT SAY ॥
  //
  // `sadhana_rev_*` hands back the 16-hex stamp `crates/sadhana-wasm/build.rs`
  // sealed in over `crates/sadhana/src`. The page compares it against the stamp
  // the build recorded for the NATIVE `sadhana` that assembled the ELFs it ships:
  // three programs re-assembling octet-for-octet is AGREEMENT, and agreement
  // between two copies of the same stale assembler looks exactly the same.
  //
  // THREE ANSWERS AND NOT TWO, because the truth has three. A stamp is a stamp;
  // `'unknown'` is the module saying its build could not take one (no sha256 tool
  // on that machine); `null` is the module having NO SUCH EXPORT, which is the
  // staleness this is here for — a `sadhana_wasm.wasm` built before stamping
  // existed, spliced into a page today, answers nothing at all. A loader that
  // treated a missing export as "matches" would hide precisely that module.
  //
  // BOTH HALVES OF THE PAIR ARE PROBED, AND THE ONE-SIDED FORM WAS A HALF-GUARD.
  // This read `typeof this.e.sadhana_rev_ptr !== 'function'` and then CALLED
  // `sadhana_rev_len()` unguarded, so the three answers were only three when the
  // pair was all-or-nothing. Measured 2026-10-05 over the real module: exports
  // minus `sadhana_rev_len` alone makes this getter THROW
  // `this.e.sadhana_rev_len is not a function` — a JS TypeError out of the
  // accessor whose entire contract is to answer `null` instead. The page reads
  // `s.rev` to print its provenance line; a throw there kills the script before
  // any of it runs, which turns a provenance GAP into a blank page and is the one
  // outcome the `null` is written to avoid. Cheaper to probe both than to argue
  // that the two are always shipped together.
  get rev() {
    if (typeof this.e.sadhana_rev_ptr !== 'function'
        || typeof this.e.sadhana_rev_len !== 'function') return null;
    const ptr = this.e.sadhana_rev_ptr(), len = this.e.sadhana_rev_len();
    return new TextDecoder().decode(this.mem().slice(ptr, ptr + len));
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
