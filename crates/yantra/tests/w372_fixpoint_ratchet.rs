//! **`W-372`: NO FIXPOINT IMAGE MAY WAIT — THE RATCHET ADR-0040 REQUIRES, IMAGE HALF.**
//!
//! ADR-0040 ("The counter, and the trap that is not the obvious one"): if the compiler's
//! own image ever touched the machine/host contract that Option C adds, Stage 2 could
//! differ from Stage 1 by self-reference, and the fixpoint would stop being a statement
//! about the compiler. The ratchet is therefore on the IMAGE the fixpoint runs, not on a
//! spelling in the sources.
//!
//! **WHY NOT A TOKEN COUNT (coordinator ruling 2026-10-04).** The wait intrinsic has no
//! name yet — `W-373` chooses it — and the ADR's placeholder is already an identifier
//! stem in the corpus (13 bare uses plus compounds), so a substring count would be red on
//! day one for the wrong reason. The token half moves into `W-373`, matched on whole
//! tokens of the name it chooses.
//!
//! **THE WAIT HALF IS ALREADY ENFORCED, DYNAMICALLY, SINCE `W-370`.** A Stage 2 image
//! that stores to `WAIT` halts `Halt::Wait` instead of reaching the finisher, and
//! `tools/fixpoint.sh` refuses any Stage 2 log without `halt: Finisher` (exit 2). Nothing
//! pinned that the refusal stays there, so this test runs THE REAL SCRIPT, with the real
//! `yantra-run`, on a stand-in Stage 1: a `t1_image` stub that copies a hand-built image
//! to `-o`. Remove the refusal and the waiting image sails on to the byte comparison —
//! this test then reds, because the message it requires is gone.
//!
//! **THE CONTROL** is the same harness on an image that only writes the SUCCESS
//! finisher: it must get PAST the Stage 2 refusals and stop later, at "the sink holds no
//! ELF". Without it, a harness that broke for any reason (no python, no corpus, the stub
//! not executable) would satisfy the refusal test by failing early.

use std::path::{Path, PathBuf};
use std::process::Command;

use sadhana::kosha;
use yantra::input::{EVENT_TAG, INPUT_TAG, NAME_TAG, TRACE_TAG};
use yantra::threads::{THREAD_ID_TAG, THREADS_TAG};
use yantra::{COUNTER, FINISHER, SOCK, WAIT};

fn i(op: u32, rd: u32, f3: u32, rs1: u32, imm: i32) -> u32 {
    op | rd << 7 | f3 << 12 | rs1 << 15 | ((imm as u32) & 0xfff) << 20
}
fn s(op: u32, f3: u32, rs1: u32, rs2: u32, imm: i32) -> u32 {
    let imm = imm as u32;
    op | (imm & 0x1f) << 7 | f3 << 12 | rs1 << 15 | rs2 << 20 | (imm >> 5 & 0x7f) << 25
}
fn lui(rd: u32, imm: u32) -> u32 {
    0x37 | rd << 7 | (imm & 0xffff_f000)
}
fn addi(rd: u32, rs1: u32, imm: i32) -> u32 {
    i(0x13, rd, 0, rs1, imm)
}

/// `x5 = WAIT`. Asserted, so a move of the constant reds here rather than aiming the
/// program at some other address.
fn wait_address() -> [u32; 2] {
    assert_eq!(WAIT, 0x1000_0100, "the encoding below spells this address");
    [lui(5, 0x1000_0000), addi(5, 5, 0x100)]
}

/// The SUCCESS finisher, four words.
fn finish() -> [u32; 4] {
    [
        lui(10, FINISHER as u32),
        lui(11, 0x5000),
        addi(11, 11, 0x555),
        s(0x23, 2, 10, 11, 0),
    ]
}

/// An image whose text is `code` followed by the three input tags `fixpoint.sh`'s Stage 2
/// requires (it always passes `YANTRA_INPUT`), each with its slot word after it — so the
/// run reaches the program instead of refusing at injection for an unrelated reason.
fn image(code: &[u32]) -> Vec<u8> {
    image_with(code, &[])
}

/// [`image`], with more `(tag, slot word)` pairs after the three input tags.
fn image_with(code: &[u32], more: &[(u64, u64)]) -> Vec<u8> {
    let mut text: Vec<u8> = code.iter().flat_map(|w| w.to_le_bytes()).collect();
    for tag in [INPUT_TAG, NAME_TAG, TRACE_TAG] {
        text.extend_from_slice(&tag.to_le_bytes());
        text.extend_from_slice(&0u64.to_le_bytes());
    }
    for (tag, slot) in more {
        text.extend_from_slice(&tag.to_le_bytes());
        text.extend_from_slice(&slot.to_le_bytes());
    }
    kosha::write(&text)
}

fn fresh_dir(what: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "w372-{what}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    dir
}

/// What `tools/fixpoint.sh` did with `stage1` standing in for Stage 1.
struct Verdict {
    code: Option<i32>,
    stderr: String,
    stage2_log: String,
}

/// Run the real `tools/fixpoint.sh` with `SASSEMBLY_BIN` pointing at a directory holding
/// a `t1_image` stub (copies `stage1` to its `-o` argument) and the real `yantra-run`.
fn fixpoint_on(what: &str, stage1: &[u8]) -> Verdict {
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tools/fixpoint.sh");
    fixpoint_script_on(what, stage1, &script)
}

/// [`fixpoint_on`], running `script` — the real one, or a mutated copy.
fn fixpoint_script_on(what: &str, stage1: &[u8], script: &Path) -> Verdict {
    let dir = fresh_dir(what);
    let bin = dir.join("bin");
    std::fs::create_dir_all(&bin).unwrap();
    let elf = dir.join("stage1-stand-in.elf");
    std::fs::write(&elf, stage1).unwrap();
    let stub = bin.join("t1_image");
    std::fs::write(
        &stub,
        format!(
            "#!/bin/sh\n# W-372 stand-in: copy a prepared image to the -o argument.\n\
             while [ $# -gt 0 ]; do\n  if [ \"$1\" = -o ]; then cp '{}' \"$2\"; exit $?; fi\n  shift\n\
             done\necho 'stub t1_image: no -o' >&2\nexit 3\n",
            elf.display()
        ),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    std::os::unix::fs::symlink(env!("CARGO_BIN_EXE_yantra-run"), bin.join("yantra-run")).unwrap();

    let out_dir = dir.join("out");
    let out = Command::new("bash")
        .arg(script)
        .arg(&out_dir)
        .env("SASSEMBLY_BIN", &bin)
        // Stage 2's default RAM is 2.5 GiB for the real compiler; the stand-in needs
        // only enough to hold the packed corpus.
        .env("YANTRA_RAM", (64u64 << 20).to_string())
        .env("YANTRA_STEPS", "100000")
        .output()
        .expect("fixpoint.sh runs");
    let stage2_log = std::fs::read_to_string(out_dir.join("stage2.log")).unwrap_or_default();
    let v = Verdict {
        code: out.status.code(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned()
            + &String::from_utf8_lossy(&out.stdout),
        stage2_log,
    };
    let _ = std::fs::remove_dir_all(&dir);
    v
}

/// THE CONTROL. An image that only finishes gets past every Stage 2 refusal and stops at
/// the comparison, because its sink holds no ELF. If this reds, the harness is broken and
/// the refusal test below proves nothing.
#[test]
fn control_an_image_that_only_finishes_passes_the_stage2_refusals() {
    let v = fixpoint_on("control", &image(&finish()));
    assert!(
        v.stage2_log.contains("halt: Finisher"),
        "the stand-in reached its finisher:\n{}\n--- stage2.log\n{}",
        v.stderr,
        v.stage2_log
    );
    assert!(
        !v.stderr.contains("did not reach a finisher"),
        "a finishing image must not be refused as non-finishing:\n{}",
        v.stderr
    );
    assert!(
        v.stderr.contains("the sink holds no ELF"),
        "the script got as far as the comparison:\n{}",
        v.stderr
    );
}

/// THE WAIT HALF. An image that stores to `WAIT` and would then write the SUCCESS
/// finisher: `fixpoint.sh` must refuse it at Stage 2, exit 2, naming the missing finisher.
#[test]
fn a_stage2_image_that_waits_is_refused_by_the_fixpoint() {
    let [a, b] = wait_address();
    let mut code = vec![a, b, s(0x23, 3, 5, 0, 0)];
    code.extend(finish());
    let v = fixpoint_on("waits", &image(&code));
    assert!(
        v.stage2_log.contains("halt: Wait"),
        "the stand-in halted at its wait:\n{}\n--- stage2.log\n{}",
        v.stderr,
        v.stage2_log
    );
    assert_eq!(
        v.code,
        Some(2),
        "a stage that failed is exit 2:\n{}",
        v.stderr
    );
    assert!(
        v.stderr.contains("Stage 2 did not reach a finisher"),
        "fixpoint.sh's refusal, by its own words:\n{}",
        v.stderr
    );
}

/// THE COUNTER HALF (`W-374` defines the address). A counter read does not halt — it is
/// one instruction answering a number — so the run reaches its finisher and the WAIT
/// refusal above cannot see it. `yantra-run` reports every answered read, and
/// `fixpoint.sh` refuses a Stage 2 whose log carries that report, exit 2, by name.
#[test]
fn a_stage2_image_that_reads_the_counter_is_refused_by_the_fixpoint() {
    assert_eq!(
        COUNTER, 0x1000_0108,
        "the encoding below spells this address"
    );
    // x5 = COUNTER; lw x6, 0(x5) — the shape `अष्टकॱउपकरणचतुरष्टकाहारः` lowers to.
    let mut code = vec![lui(5, 0x1000_0000), addi(5, 5, 0x108), i(0x03, 6, 2, 5, 0)];
    code.extend(finish());
    let v = fixpoint_on("counter", &image(&code));
    assert!(
        v.stage2_log.contains("halt: Finisher"),
        "a counter read does not halt; the stand-in finished:\n{}\n--- stage2.log\n{}",
        v.stderr,
        v.stage2_log
    );
    assert_eq!(
        v.code,
        Some(2),
        "a stage that failed is exit 2:\n{}",
        v.stderr
    );
    assert!(
        v.stderr
            .contains("Stage 2 read the retired-instruction counter"),
        "fixpoint.sh's refusal, by its own words:\n{}\n--- stage2.log\n{}",
        v.stderr,
        v.stage2_log
    );
}

/// THE THREADS HALF (`W-376`, ADR-0040 addendum §6). A threaded image's run is a function of
/// its schedule as well as its input, so a compiler image that declared `SASTHRDS` would
/// make the fixpoint a statement about a schedule. `yantra-run` prints `threads: N` whenever
/// the tag is found — before it refuses a threaded image given no event log, at load — and
/// `fixpoint.sh` refuses a Stage 2 whose log carries that line, exit 2, by name.
fn threaded_stand_in() -> Vec<u8> {
    // auipc sp, 0; addi sp, sp, 0 — the startup's two sp words — then the finisher.
    let mut code = vec![0x17 | 2 << 7, addi(2, 2, 0)];
    code.extend(finish());
    image_with(
        &code,
        &[(THREADS_TAG, 1), (THREAD_ID_TAG, 0), (EVENT_TAG, 0)],
    )
}

#[test]
fn a_stage2_image_that_declares_threads_is_refused_by_the_fixpoint() {
    let v = fixpoint_on("threads", &threaded_stand_in());
    assert!(
        v.stage2_log.lines().any(|l| l == "threads: 1"),
        "yantra-run said the image is threaded:\n{}\n--- stage2.log\n{}",
        v.stderr,
        v.stage2_log
    );
    assert_eq!(
        v.code,
        Some(2),
        "a stage that failed is exit 2:\n{}",
        v.stderr
    );
    assert!(
        v.stderr.contains("Stage 2 is a THREADED image"),
        "fixpoint.sh's refusal, by its own words:\n{}\n--- stage2.log\n{}",
        v.stderr,
        v.stage2_log
    );
}

/// THE MUTATION CONTROL: a COPY of `fixpoint.sh` with the threads check cut out, run from
/// a scratch root that links the real `crates/` and `tools/pack-corpus.py`, no longer
/// names the threaded image — so the test above is red exactly when the check is gone.
#[test]
fn a_fixpoint_without_the_threads_check_does_not_name_the_threaded_image() {
    let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(real.join("tools/fixpoint.sh")).expect("fixpoint.sh");
    let begin = text
        .find("# ── W-376")
        .expect("the threads check's margin opens with `# ── W-376`");
    let end = begin
        + text[begin..]
            .find("\nfi\n")
            .expect("and the check ends at its `fi`")
        + "\nfi\n".len();
    let mutant = format!("{}{}", &text[..begin], &text[end..]);
    assert!(!mutant.contains("^threads:"), "the mutant still checks");
    let root = fresh_dir("mutant-root");
    std::fs::create_dir_all(root.join("tools")).unwrap();
    std::fs::write(root.join("tools/fixpoint.sh"), mutant).unwrap();
    std::os::unix::fs::symlink(
        real.join("tools/pack-corpus.py").canonicalize().unwrap(),
        root.join("tools/pack-corpus.py"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        real.join("crates").canonicalize().unwrap(),
        root.join("crates"),
    )
    .unwrap();
    let v = fixpoint_script_on(
        "mutant",
        &threaded_stand_in(),
        &root.join("tools/fixpoint.sh"),
    );
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        !v.stderr.contains("THREADED"),
        "the mutant named the threaded image anyway:\n{}",
        v.stderr
    );
    assert!(
        v.stderr.contains("Stage 2 did not reach a finisher"),
        "the mutant ran to its next refusal:\n{}",
        v.stderr
    );
}

/// THE SOCKET HALF (`W-377`, sockets addendum §6.1). Stage 2 runs with no socket, so an
/// image that reaches the window halts `Device` and never finishes; `yantra-run` says
/// `socket: …` whenever the device is configured or touched, and `fixpoint.sh` refuses a
/// Stage 2 whose log carries that line — FIRST, naming the cause the missing finisher
/// alone would not.
fn socket_stand_in() -> Vec<u8> {
    assert_eq!(SOCK, 0x1000_0110, "the encoding below spells this address");
    // x5 = SOCK; sw x0, 0(x5) — a NEXT store, the shape `अष्टकॱउपकरणचतुरष्टकनिधानम्` lowers to.
    let mut code = vec![lui(5, 0x1000_0000), addi(5, 5, 0x110), s(0x23, 2, 5, 0, 0)];
    code.extend(finish());
    image(&code)
}

#[test]
fn a_stage2_image_that_touches_the_socket_is_refused_by_the_fixpoint() {
    let v = fixpoint_on("socket", &socket_stand_in());
    assert!(
        v.stage2_log
            .lines()
            .any(|l| l.starts_with("socket: touched at 0x10000110")),
        "yantra-run said the image touched the socket:\n{}\n--- stage2.log\n{}",
        v.stderr,
        v.stage2_log
    );
    assert_eq!(
        v.code,
        Some(2),
        "a stage that failed is exit 2:\n{}",
        v.stderr
    );
    assert!(
        v.stderr.contains("Stage 2 touched the SOCKET device"),
        "fixpoint.sh's refusal, by its own words:\n{}\n--- stage2.log\n{}",
        v.stderr,
        v.stage2_log
    );
}

/// THE MUTATION CONTROL for the socket half: a COPY of `fixpoint.sh` with the W-377 check
/// cut out no longer names the socket, and refuses only as "no finisher".
#[test]
fn a_fixpoint_without_the_socket_check_does_not_name_the_socket() {
    let real = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let text = std::fs::read_to_string(real.join("tools/fixpoint.sh")).expect("fixpoint.sh");
    let begin = text
        .find("# ── W-377")
        .expect("the socket check's margin opens with `# ── W-377`");
    let end = begin
        + text[begin..]
            .find("\nfi\n")
            .expect("and the check ends at its `fi`")
        + "\nfi\n".len();
    let mutant = format!("{}{}", &text[..begin], &text[end..]);
    assert!(!mutant.contains("^socket:"), "the mutant still checks");
    let root = fresh_dir("socket-mutant-root");
    std::fs::create_dir_all(root.join("tools")).unwrap();
    std::fs::write(root.join("tools/fixpoint.sh"), mutant).unwrap();
    std::os::unix::fs::symlink(
        real.join("tools/pack-corpus.py").canonicalize().unwrap(),
        root.join("tools/pack-corpus.py"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        real.join("crates").canonicalize().unwrap(),
        root.join("crates"),
    )
    .unwrap();
    let v = fixpoint_script_on(
        "socket-mutant",
        &socket_stand_in(),
        &root.join("tools/fixpoint.sh"),
    );
    let _ = std::fs::remove_dir_all(&root);
    assert!(
        !v.stderr.contains("SOCKET"),
        "the mutant named the socket anyway:\n{}",
        v.stderr
    );
    assert!(
        v.stderr.contains("Stage 2 did not reach a finisher"),
        "the mutant ran to its next refusal:\n{}",
        v.stderr
    );
}
