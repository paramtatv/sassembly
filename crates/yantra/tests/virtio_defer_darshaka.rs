//! THE MUTANT C-015 OWES: a driver that reads the used index ONCE after QueueNotify passes
//! on a device that completes inside the notify store and fails on one that completes
//! later — QEMU's. `spec/darshaka.t1` was that driver until it polled (G 11 on QEMU 10.1).
//! yantra's virtio-gpu completes synchronously by default, so before `YANTRA_VIRTIO_DEFER`
//! nothing on yantra could tell the two drivers apart.
//!
//! This builds C-015's own draw program (`spec/virtio-gpu-draw.t1` over
//! `spec/darshaka.t1`) twice — once as it is, once with its bounded poll cut to ONE read
//! (the pre-poll-fix shape) — and runs each on `yantra-run` with the completion
//! synchronous (`YANTRA_VIRTIO_DEFER` unset) and deferred (`=1000`). The driver's code is
//! its exit status and its `G <code>` line:
//!
//! | driver        | synchronous | deferred              |
//! | ------------- | ----------- | --------------------- |
//! | the real one  | G 0         | G 0                   |
//! | one read only | G 0         | G 10 (command 0 never seen used) |
//!
//! The top-right cell is the whole point: the single-read driver PASSES the synchronous
//! device, which is how it reached QEMU before anything caught it.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The poll in `प्रदर्शनम्`: up to 10^7 reads of the used index. The mutant reads once.
const POLL: &str = "यावत् पठनानि न्यूनम् १००००००० आदि";
const ONE_READ: &str = "यावत् पठनानि न्यूनम् १ आदि";
/// The draw program's region needs RAM past 0xA800_0000 (darshaka.t1's margin).
const RAM: &str = "738197504";

fn scratch() -> PathBuf {
    // pid AND a clock: a pid is REUSED and these roots are never removed (W-301).
    let d = std::env::temp_dir().join(format!(
        "virtio-defer-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos()
    ));
    std::fs::create_dir_all(&d).expect("scratch dir");
    d
}

fn build(t1_image: &Path, root: &Path, driver: &Path, out: &Path) {
    let spec = root.join("spec");
    let draw = spec.join("virtio-gpu-draw.t1");
    let built = Command::new(t1_image)
        .arg("--spec-root")
        .arg(&spec)
        .arg("--compiler")
        .arg(root.join("crates/sadhana-t1/src"))
        .arg("--load")
        .arg(driver)
        .arg("--load")
        .arg(&draw)
        .args(["--entry", "प्रथमबिन्दु", "मुख्यम्", "-o"])
        .arg(out)
        .arg(driver)
        .arg(&draw)
        .output()
        .expect("t1_image runs");
    assert!(
        out.exists(),
        "t1_image built no image from {}:\n{}{}",
        driver.display(),
        String::from_utf8_lossy(&built.stdout),
        String::from_utf8_lossy(&built.stderr)
    );
}

/// The driver's `G` code on yantra-run, given the program one argument (so it draws).
fn g_code(yantra_run: &Path, elf: &Path, defer: Option<&str>) -> String {
    let mut c = Command::new(yantra_run);
    c.arg(elf)
        .arg("screen")
        .env("YANTRA_RAM", RAM)
        .env("YANTRA_STEPS", "4000000000");
    c.env_remove("YANTRA_VIRTIO_DEFER")
        .env_remove("YANTRA_INPUT")
        .env_remove("YANTRA_SCANOUT");
    if let Some(d) = defer {
        c.env("YANTRA_VIRTIO_DEFER", d);
    }
    let o = c.output().expect("yantra-run runs");
    let text =
        String::from_utf8_lossy(&o.stdout).into_owned() + &String::from_utf8_lossy(&o.stderr);
    text.lines()
        .find_map(|l| l.strip_prefix("G ").map(str::to_string))
        .unwrap_or_else(|| {
            panic!(
                "no `G` line from {} (defer {defer:?}):\n{text}",
                elf.display()
            )
        })
}

#[test]
#[ignore = "MEASUREMENT, ~1 min — and it PASSES. Ignored because it builds two ELFs with \
 t1_image and shells out to yantra-run, which a per-commit gate has no release binaries for. \
 `tools/deep-gate.sh` names it with --include-ignored, beside t1_arguments"]
fn a_single_read_driver_passes_a_synchronous_device_and_fails_a_deferred_one() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let bin = root.join("target/release");
    let (t1_image, yantra_run) = (bin.join("t1_image"), bin.join("yantra-run"));
    if !t1_image.exists() || !yantra_run.exists() {
        // A HONEST SKIP that says what is missing rather than passing silently.
        eprintln!("skipped: build with `cargo build --release -p sadhana -p yantra` first");
        return;
    }
    let dir = scratch();
    let real = root.join("spec/darshaka.t1");
    let source = std::fs::read_to_string(&real).expect("read spec/darshaka.t1");
    assert_eq!(
        source.matches(POLL).count(),
        1,
        "the mutant's anchor — darshaka.t1's bounded poll — must occur exactly once"
    );
    let mutant = dir.join("darshaka.t1");
    std::fs::write(&mutant, source.replace(POLL, ONE_READ)).expect("write the mutant");

    let (good, bad) = (dir.join("real.elf"), dir.join("one-read.elf"));
    build(&t1_image, &root, &real, &good);
    build(&t1_image, &root, &mutant, &bad);

    let got = [
        g_code(&yantra_run, &good, None),
        g_code(&yantra_run, &good, Some("1000")),
        g_code(&yantra_run, &bad, None),
        g_code(&yantra_run, &bad, Some("1000")),
    ];
    assert_eq!(
        got,
        ["0", "0", "0", "10"].map(str::to_string),
        "[real sync, real deferred, one-read sync, one-read deferred]: the one-read driver \
         must pass the synchronous device and fail the deferred one with code 10 (command 0 \
         has no used entry), while the real driver passes both"
    );
}
