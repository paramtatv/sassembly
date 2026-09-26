//! Run an ELF natively and print what it wrote to the UART — the control for the browser.
//!
//! The browser arm is only evidence if the same interpreter, on the same artefact, is
//! known to produce the right bytes somewhere it can be checked easily.
use std::process::ExitCode;
use yantra::{DEFAULT_STEPS, Output};

/// The UART bytes, plus the highest RAM offset any store reached — printed
/// with `YANTRA_WATERMARK` set, so a run says how much of RAM it touched.
struct Sink {
    out: Vec<u8>,
    high_water: usize,
}

impl Output for Sink {
    fn putc(&mut self, byte: u8) {
        self.out.push(byte);
    }
    fn stored(&mut self, at: usize, width: usize) {
        if at + width > self.high_water {
            self.high_water = at + width;
        }
    }
}
fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: yantra-run <program.elf>");
        return ExitCode::FAILURE;
    };
    let image = std::fs::read(&path).expect("read the ELF");

    // F-020 (W-119): Refuse application images instead of booting and faulting.
    if let (Ok(tsv), Some(stem)) = (
        std::fs::read_to_string("spec/programs.tsv"),
        std::path::Path::new(&path).file_stem(),
    ) {
        let sas_name = format!("{}.sas", stem.to_string_lossy());
        let is_app = tsv.lines().any(|line| {
            let mut parts = line.split('\t');
            parts.next() == Some(&sas_name) && parts.next() == Some("app")
        });
        if is_app {
            eprintln!(
                "{path}: this image is an application and must be run by yantra-host, not yantra-run"
            );
            return ExitCode::FAILURE;
        }
    }

    // RAM IS SIZED FROM THE IMAGE, with the product's default as the floor: the
    // image's own segments say how far it reaches (its record region is a
    // PT_LOAD with a memsz), and a compiler's image reaches further than 20 MiB
    // once its runs grow. HEADROOM above the last segment is for the stack and
    // whatever the startup places past the loaded extent. `YANTRA_RAM=<octets>`
    // overrides both for a diagnostic run; `YANTRA_WATERMARK=1` prints how much
    // of RAM the run actually wrote, which is how the region constant is sized.
    // The sizing itself is `yantra::ram_for`, the one statement the census's
    // runner shares.
    let ram = std::env::var("YANTRA_RAM")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or_else(|| yantra::ram_for(&image));
    let mut m = match yantra::Machine::load_elf(&image, ram) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };
    // THE INPUT CHANNEL (`yantra::input`) — a file placed in RAM before the
    // first instruction, for a program that declares the interface. No new
    // `ecall`: the machine still answers exactly two. `YANTRA_INPUT` is the file,
    // `YANTRA_INPUT_NAME` its module name (passed, not parsed, so the host and
    // the interpreter's `--input-name` cannot derive it two different ways),
    // `YANTRA_INPUT_TRACE=<level>` sets the program's trace level (1 markers, 2 text).
    //
    // A REFUSAL STOPS THE RUN. A program run without its input would compile
    // an empty source and halt looking like a verdict about that source.
    // Where the injected slab begins, so the run can be checked afterwards for
    // an allocator that grew into it — see the collision check below.
    let mut slab_at: Option<usize> = None;
    if let Some(input) = std::env::var_os("YANTRA_INPUT") {
        let text = match std::fs::read(&input) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("input: YANTRA_INPUT {input:?}: {e}");
                return ExitCode::FAILURE;
            }
        };
        let Some(name) = std::env::var_os("YANTRA_INPUT_NAME") else {
            eprintln!(
                "input: YANTRA_INPUT needs YANTRA_INPUT_NAME — the module name is passed, never guessed"
            );
            return ExitCode::FAILURE;
        };
        // A LEVEL, not a switch: 1 = the encoder's marker trace, 2 = each
        // module's generated Sassembly text, for diffing the two engines line
        // by line. Anything unparsable is 0 — no trace — rather than a guess.
        let trace: u64 = std::env::var("YANTRA_INPUT_TRACE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        let base = m.base;
        match yantra::input::inject(
            &mut m.mem,
            base,
            &text,
            name.to_string_lossy().as_bytes(),
            trace,
        ) {
            Ok(r) => {
                eprintln!(
                    "input: {} octets of {:?} as module {:?}, tags at {:x?}, RAM {} -> {}",
                    text.len(),
                    input,
                    name,
                    r.tags_at,
                    r.old_top,
                    r.new_top
                );
                slab_at = Some(r.old_top);
            }
            Err(e) => {
                eprintln!("input: refused — {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    let mut sink = Sink {
        out: Vec::new(),
        high_water: 0,
    };
    // YANTRA_STEPS=<n>: the step ceiling, for a compile that legitimately outruns
    // the default (a native compile of a module with routines does).
    let steps = std::env::var("YANTRA_STEPS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_STEPS);
    let halt = m.run(steps, &mut sink);
    use std::io::Write;
    std::io::stdout().write_all(&sink.out).unwrap_or(());
    eprintln!("halt: {halt:?}");
    if std::env::var_os("YANTRA_WATERMARK").is_some() {
        eprintln!("ram: high water {} of {} octets", sink.high_water, ram);
    }
    // THE COLLISION CHECK, ALWAYS ON WHEN THERE IS AN INPUT. The record
    // allocator is a bump cursor with no upper bound, so it can grow past the
    // image, through the headroom, and INTO the injected slab — measured
    // 2026-09-21 on the first native self-image build, which reached 559,504,480
    // octets over a 555,254,376-octet old top and then ran off RAM. An
    // allocator that overwrites its own input mid-read compiles corrupted
    // source and halts looking like an ordinary result, so the one number that
    // can tell is said out loud: the store high-water mark against the slab.
    if let Some(at) = slab_at
        && sink.high_water > at
    {
        eprintln!(
            "input: COLLISION — the program stored up to octet {} and the injected input \
                 begins at {at}; it may have overwritten its own source. Raise YANTRA_RAM.",
            sink.high_water
        );
    }
    ExitCode::SUCCESS
}
