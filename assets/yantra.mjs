// Hand-written glue for the yantra wasm module. No wasm-bindgen, no generated bindings.
// The whole protocol is a handful of exported functions and one shared linear memory.
export async function load(wasmBytes) {
  const { instance } = await WebAssembly.instantiate(wasmBytes, {});
  return new Yantra(instance);
}
export class Yantra {
  constructor(instance) {
    this.e = instance.exports;
    // The module exports its own memory; that buffer is the only thing JS and Rust share.
    this.mem = () => new Uint8Array(this.e.memory.buffer);
  }
  // Boot the image: reset vector, S-mode, no loader and no supervisor. `out` is the
  // machine's own UART and `surface` is empty, because nothing granted one.
  run(elf, { ram = 1 << 20, budget = 1_000_000 } = {}) {
    this.#write(elf);
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
  host(elf, { ram = 1 << 22, budget = 200 } = {}) {
    this.#write(elf);
    return this.#read(this.e.yantra_host(ram, budget), true);
  }
  #write(elf) {
    const at = this.e.yantra_alloc(elf.length);
    this.mem().set(elf, at);          // JS writes the ELF straight into wasm memory
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
    };
  }
}
