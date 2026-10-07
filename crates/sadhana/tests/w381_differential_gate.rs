//! **`W-381` PART B — `t1_image`'S DIFFERENTIAL GATE, EACH PATH RED FIRST.**
//!
//! With `--entry`, `t1_image` runs the built image under `yantra-run` (as a
//! child, beside itself) and compares it with the interpreted predict by
//! `sadhana::t1::agreement::agree`. Disagreement, an inconclusive run or a build
//! with no comparison is REFUSED: the image goes to `<out>.refused`, nothing is
//! left at `<out>`, and the exit is [`DIVERGED`]; `--accept-divergence <reason>`
//! writes it and records the reason in `<out>.provenance`.
//!
//! Every test here was RED against `t1_image` before the gate (origin/main
//! 07a4ed43): the flag did not exist, the divergent build exited 0 and wrote
//! `<out>`, and there was no sidecar.
//!
//! THE NATIVE HALF NEEDS `yantra-run` BUILT BESIDE `t1_image` FROM THE SAME
//! COMMIT — `cargo test -p sadhana -p yantra` (the landing gate's scope) builds
//! both into one target directory. Run alone, `cargo test -p sadhana` has no
//! `yantra-run` there and these tests say so rather than skipping.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The gate's refusal exit, as `t1_image.rs` declares it.
const DIVERGED: i32 = 97;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn scratch(slot: &str) -> PathBuf {
    let d = root().join("target/w381-gate").join(slot);
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).expect("the scratch directory is creatable");
    d
}

fn runner_beside() -> PathBuf {
    let r = Path::new(env!("CARGO_BIN_EXE_t1_image")).with_file_name("yantra-run");
    assert!(
        r.is_file(),
        "{} is missing: build it in this target directory (cargo test -p sadhana -p yantra, \
         or cargo build -p yantra --bin yantra-run with the same profile)",
        r.display()
    );
    r
}

/// Build `src` (module `module`) with `--load` and `--entry <module> मुख्यम्`.
fn build(slot: &str, module: &str, src: &str, extra: &[&str]) -> (Option<i32>, String, PathBuf) {
    let (code, text, _, out) = build_to(slot, module, src, extra, None, &[]);
    (code, text, out)
}

/// `build`, with `-o` at `to` when given; answers the raw stdout as well.
fn build_to(
    slot: &str,
    module: &str,
    src: &str,
    extra: &[&str],
    to: Option<&Path>,
    env: &[(&str, &str)],
) -> (Option<i32>, String, Vec<u8>, PathBuf) {
    runner_beside();
    let d = scratch(slot);
    let file = d.join(format!("{module}.t1"));
    std::fs::write(&file, src).expect("fixture written");
    let out = to.map_or_else(|| d.join("out.elf"), Path::to_path_buf);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_t1_image"));
    cmd.current_dir(root())
        .arg("--spec-root")
        .arg(root().join("spec"))
        .arg("--compiler")
        .arg(root().join("crates/sadhana-t1/src"))
        .arg("--load")
        .arg(&file)
        .args(["--entry", module, "मुख्यम्"])
        .args(extra)
        .envs(env.iter().copied())
        .arg("-o")
        .arg(&out)
        .arg(&file);
    let r = cmd.output().expect("t1_image runs");
    let mut text = String::from_utf8_lossy(&r.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&r.stderr));
    (r.status.code(), text, r.stdout, out)
}

fn with(out: &Path, suffix: &str) -> PathBuf {
    let mut s = out.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

fn provenance(out: &Path) -> String {
    std::fs::read_to_string(with(out, ".provenance")).unwrap_or_default()
}

/// A program both engines answer alike: prints `A`, returns ५.
const AGREES: &str = "मण्डलम् सम्मतः ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱमुद्रणम् ६५ ।
    प्रत्यागमनम् ५ ।
इति
";

/// `~/b1-safety-probes/p12_octet_param_cow.t1` (module `परकर्तृ`), verbatim: the
/// interpreter answers १ and the image ९ — an octet-run parameter is
/// copy-on-write only in the interpreter (`W-381`'s ratchet, row p12, stage
/// "other"). It was p14 (i64::MAX + 1) until `W-381` stage 3 made the
/// interpreter wrap to 64 bits and the two engines AGREE on it.
const DIVERGES: &str = r#"मण्डलम् परकर्तृ ॥

वृत्तिः ख आदाय क ॱॱ अङ्कः अन्तः अ८ ददाति न६४ आदि
    क अङ्कः ० अन्तः भवति ९ ।
    प्रत्यागमनम् ० ।
इति

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    क अङ्कः ० अन्तः भवति १ ।
    क अङ्कः १ अन्तः भवति २ ।
    चरः उ ॱॱ न६४ भवति ख क ।
    प्रत्यागमनम् क अङ्कः ० अन्तः ।
इति
"#;

/// `~/b1-safety-probes/p08_fresh_read0.t1`, verbatim: a refusal on BOTH engines
/// (the interpreter's "entry 0 is outside an arena of 0", the image's FAIL-form
/// `0x355`) — the SAME refusal, so the gate passes it.
const SAME_REFUSAL: &str = r#"मण्डलम् परकर्तृ ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    प्रत्यागमनम् क अङ्कः ० अन्तः ।
इति
"#;

/// `crates/yantra/tests/t1_user_input_interface.rs`'s `READS_ITS_INPUT`, READ
/// FROM THERE and not copied: it sums the octets of the input it was given. A
/// copy in this directory declared the input channel's tags, and
/// `t1_paradigm_names.rs` reads every `.rs` here as a CALLER of the corpus names
/// it mentions — so the copy cleared three tags that list keeps on purpose
/// (their consumer is `yantra-run`). One statement of the fixture, and none here.
fn reads_its_input() -> String {
    let f = root().join("crates/yantra/tests/t1_user_input_interface.rs");
    let text = std::fs::read_to_string(&f).expect("yantra's input-interface test");
    let start = "const READS_ITS_INPUT: &str = \"";
    let a = text.find(start).expect("READS_ITS_INPUT is declared there") + start.len();
    let b = a + text[a..].find("\n\";").expect("its closing quote") + 1;
    text[a..b].to_string()
}

#[test]
fn the_refusal_exit_is_unused_elsewhere() {
    // 1 t1_image's own failures; 64/66/75/81..88 yantra-run; 59/85/89/90/91/93
    // the QEMU exits of the canary and the refusal statuses (0x35b's 91 and
    // `W-381` stage 4's out-of-bounds store, 0x35d, 93); 77 the landing gate.
    for taken in [
        1, 59, 64, 66, 75, 77, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 93,
    ] {
        assert_ne!(DIVERGED, taken);
    }
    let src = std::fs::read_to_string(root().join("crates/sadhana/src/bin/t1_image.rs"))
        .expect("t1_image.rs");
    assert!(
        src.contains(&format!("const DIVERGED: u8 = {DIVERGED};")),
        "t1_image.rs no longer declares the exit this file pins"
    );
}

#[test]
fn an_agreeing_build_is_written_and_recorded_agreed() {
    let (code, text, out) = build("agree", "सम्मतः", AGREES, &[]);
    assert_eq!(code, Some(0), "{text}");
    assert!(out.is_file(), "{text}");
    assert!(!with(&out, ".refused").exists());
    assert!(text.contains("gate:     AGREED"), "{text}");
    let p = provenance(&out);
    assert!(p.starts_with("verdict: AGREED\n"), "{p}");
    assert!(p.contains("image_sha256: "), "{p}");
    assert!(p.contains("source_sha256: "), "{p}");
    assert!(
        p.contains("yantra_run: ") && !p.contains("yantra_run: -"),
        "{p}"
    );
}

#[test]
fn a_divergent_build_is_refused_and_left_at_refused() {
    let (code, text, out) = build("diverge", "परकर्तृ", DIVERGES, &[]);
    assert_eq!(code, Some(DIVERGED), "{text}");
    assert!(
        !out.exists(),
        "a refused build left {}: {text}",
        out.display()
    );
    assert!(with(&out, ".refused").is_file(), "{text}");
    let p = provenance(&out);
    assert!(p.starts_with("verdict: REFUSED\n"), "{p}");
    assert!(p.contains("cause: the engines DISAGREE"), "{p}");
    assert!(p.contains("predict: finished, status 1"), "{p}");
    assert!(p.contains("native: finished, status 9"), "{p}");
}

#[test]
fn a_stale_image_does_not_survive_a_refused_build() {
    let d = scratch("stale");
    let out = d.join("out.elf");
    std::fs::write(&out, b"an earlier build").unwrap();
    // `build` makes its own scratch; reuse the slot path it computes.
    let (code, text, out2) = {
        let file = d.join("परकर्तृ.t1");
        std::fs::write(&file, DIVERGES).unwrap();
        let r = Command::new(env!("CARGO_BIN_EXE_t1_image"))
            .current_dir(root())
            .arg("--spec-root")
            .arg(root().join("spec"))
            .arg("--compiler")
            .arg(root().join("crates/sadhana-t1/src"))
            .arg("--load")
            .arg(&file)
            .args(["--entry", "परकर्तृ", "मुख्यम्", "-o"])
            .arg(&out)
            .arg(&file)
            .output()
            .expect("t1_image runs");
        let mut t = String::from_utf8_lossy(&r.stdout).into_owned();
        t.push_str(&String::from_utf8_lossy(&r.stderr));
        (r.status.code(), t, out.clone())
    };
    assert_eq!(code, Some(DIVERGED), "{text}");
    assert!(
        !out2.exists(),
        "the earlier build's file survived a refusal"
    );
}

#[test]
fn an_accepted_divergence_is_written_and_its_reason_recorded() {
    let reason = "p12 copies on write only in the interpreter (W-381, row p12)";
    let (code, text, out) = build(
        "accept",
        "परकर्तृ",
        DIVERGES,
        &["--accept-divergence", reason],
    );
    assert_eq!(code, Some(0), "{text}");
    assert!(out.is_file(), "{text}");
    let p = provenance(&out);
    assert!(p.starts_with("verdict: ACCEPTED\n"), "{p}");
    assert!(p.contains(&format!("reason: {reason}\n")), "{p}");
    assert!(p.contains("cause: the engines DISAGREE"), "{p}");
}

#[test]
fn an_empty_reason_is_a_usage_error() {
    let (code, text, out) = build("empty", "सम्मतः", AGREES, &["--accept-divergence", "  "]);
    assert_eq!(code, Some(1), "{text}");
    assert!(
        text.contains("--accept-divergence needs a reason"),
        "{text}"
    );
    assert!(!out.exists());
}

#[test]
fn the_same_refusal_on_both_engines_agrees() {
    let (code, text, out) = build("same-refusal", "परकर्तृ", SAME_REFUSAL, &[]);
    assert_eq!(code, Some(0), "{text}");
    assert!(provenance(&out).starts_with("verdict: AGREED\n"), "{text}");
}

#[test]
fn no_predict_without_an_accept_is_refused_unchecked() {
    let (code, text, out) = build("no-predict", "सम्मतः", AGREES, &["--no-predict"]);
    assert_eq!(code, Some(DIVERGED), "{text}");
    assert!(!out.exists());
    assert!(provenance(&out).contains("cause: UNCHECKED"), "{text}");
    let (code, text, out) = build(
        "no-predict-accept",
        "सम्मतः",
        AGREES,
        &[
            "--no-predict",
            "--accept-divergence",
            "a test of the UNCHECKED record",
        ],
    );
    assert_eq!(code, Some(0), "{text}");
    let p = provenance(&out);
    assert!(p.starts_with("verdict: UNCHECKED\n"), "{p}");
    assert!(
        p.contains("reason: a test of the UNCHECKED record\n"),
        "{p}"
    );
}

#[test]
fn an_entry_the_interpreter_cannot_call_is_refused_unchecked() {
    runner_beside();
    let d = scratch("positional");
    let file = d.join("सम्मतः.t1");
    std::fs::write(&file, AGREES).unwrap();
    let out = d.join("out.elf");
    let go = |extra: &[&str]| {
        let r = Command::new(env!("CARGO_BIN_EXE_t1_image"))
            .current_dir(root())
            .arg("--spec-root")
            .arg(root().join("spec"))
            .arg("--compiler")
            .arg(root().join("crates/sadhana-t1/src"))
            .args(["--entry", "सम्मतः", "मुख्यम्"])
            .args(extra)
            .arg("-o")
            .arg(&out)
            .arg(&file)
            .output()
            .expect("t1_image runs");
        let mut t = String::from_utf8_lossy(&r.stdout).into_owned();
        t.push_str(&String::from_utf8_lossy(&r.stderr));
        (r.status.code(), t)
    };
    let (code, text) = go(&[]);
    assert_eq!(code, Some(DIVERGED), "{text}");
    assert!(text.contains("predict:  none"), "{text}");
    let (code, text) = go(&[
        "--accept-divergence",
        "positional entry, measured elsewhere",
    ]);
    assert_eq!(code, Some(0), "{text}");
    assert!(provenance(&out).starts_with("verdict: UNCHECKED\n"));
}

/// A stand-in `yantra-run` that answers every question with `says` on stderr.
fn fake_runner(slot: &str, says: &str) -> String {
    let d = scratch(slot);
    let fake = d.join("yantra-run");
    std::fs::write(&fake, format!("#!/bin/sh\nprintf '{says}' >&2\n")).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&fake, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    fake.to_string_lossy().into_owned()
}

#[test]
fn a_runner_from_other_sources_or_none_is_refused() {
    let zeros = "0".repeat(32);
    let fake_s = fake_runner("runner", &format!("version: 0000000\\nsource: {zeros}\\n"));
    let (code, text, out) = build("runner-other", "सम्मतः", AGREES, &["--yantra-run", &fake_s]);
    assert_eq!(code, Some(DIVERGED), "{text}");
    assert!(
        text.contains(&format!("has source stamp \"{zeros}\"")),
        "{text}"
    );
    assert!(!out.exists());
    let (code, text, _) = build(
        "runner-none",
        "सम्मतः",
        AGREES,
        &["--yantra-run", "/nonexistent/yantra-run"],
    );
    assert_eq!(code, Some(DIVERGED), "{text}");
    assert!(
        text.contains("no yantra-run at /nonexistent/yantra-run"),
        "{text}"
    );
}

/// Whether `bytes` holds an ELF header anywhere.
fn has_elf(bytes: &[u8]) -> bool {
    bytes.windows(4).any(|w| w == b"\x7fELF")
}

#[test]
fn a_refused_image_never_reaches_a_device() {
    // Control first: an AGREEING build at `-o /dev/stdout` does put the ELF on
    // stdout, so the assertion below can see one when it is there.
    let dev = Path::new("/dev/stdout");
    let (code, text, stdout, _) = build_to("dev-agree", "सम्मतः", AGREES, &[], Some(dev), &[]);
    assert_eq!(code, Some(0), "{text}");
    assert!(
        has_elf(&stdout),
        "the control wrote no ELF to stdout: {text}"
    );
    // The subject: a DIVERGENT build at the same device writes no octet of it.
    let (code, text, stdout, _) = build_to("dev-diverge", "परकर्तृ", DIVERGES, &[], Some(dev), &[]);
    assert_eq!(code, Some(DIVERGED), "{text}");
    assert!(
        !has_elf(&stdout),
        "a REFUSED image reached /dev/stdout: {text}"
    );
    assert!(text.contains("write:    nothing — REFUSED"), "{text}");
    assert!(text.contains("The image was withheld"), "{text}");
}

#[test]
fn a_runner_with_no_source_stamp_never_matches() {
    // Owner: "un-stamped build pairs must no longer evaluate as match passes".
    // A runner that says `unknown`, and one that gives no stamp at all (an older
    // build), are both refused — whatever this binary's own stamp is.
    for (slot, says) in [
        ("runner-unknown", "version: unknown\\nsource: unknown\\n"),
        ("runner-unstamped", "version: unknown\\n"),
    ] {
        let fake_s = fake_runner(slot, says);
        let (code, text, out) = build(
            &format!("{slot}-run"),
            "सम्मतः",
            AGREES,
            &["--yantra-run", &fake_s],
        );
        assert_eq!(code, Some(DIVERGED), "{slot}: {text}");
        assert!(
            text.contains("cannot be shown to be this tree's"),
            "{slot}: {text}"
        );
        assert!(!out.exists(), "{slot}");
    }
    // This side: the binary refuses its OWN `unknown` too. It cannot be built
    // stamp-less from inside a test, so the rule is read where it is stated.
    let src = std::fs::read_to_string(root().join("crates/sadhana/src/bin/t1_image.rs"))
        .expect("t1_image.rs");
    assert!(
        src.contains("if mine == \"unknown\" || theirs.is_empty() || theirs == \"unknown\""),
        "t1_image.rs no longer refuses an `unknown` source stamp on both sides"
    );
}

#[test]
fn the_two_binaries_carry_one_source_stamp() {
    let t = Command::new(env!("CARGO_BIN_EXE_t1_image"))
        .arg("--source-stamp")
        .output()
        .expect("t1_image runs");
    let mine = String::from_utf8_lossy(&t.stdout).trim().to_string();
    let y = Command::new(runner_beside())
        .arg("--source-stamp")
        .output()
        .expect("yantra-run runs");
    let theirs = String::from_utf8_lossy(&y.stderr).trim().to_string();
    assert_eq!(
        mine.len(),
        32,
        "a 32-hex stamp (two src-rev halves), got {mine:?}"
    );
    assert!(mine.bytes().all(|b| b.is_ascii_hexdigit()), "{mine:?}");
    assert_eq!(theirs, format!("source: {mine}"));
    assert!(y.stdout.is_empty(), "yantra-run's stdout is the payload's");
}

/// Counts to two million: ~8M native steps, past `yantra-run`'s default million.
const LONG: &str = "मण्डलम् सम्मतः ॥

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् २०००००० आदि
        क्रमः भवति क्रमः योगः १ ।
    इति
    प्रत्यागमनम् ७ ।
इति
";

#[test]
fn the_native_half_has_its_own_budget() {
    // A program past a million native steps builds with the default budget, even
    // when the caller's environment says YANTRA_STEPS=1000.
    let (code, text, _, out) = build_to(
        "budget-default",
        "सम्मतः",
        LONG,
        &[],
        None,
        &[("YANTRA_STEPS", "1000")],
    );
    assert_eq!(code, Some(0), "{text}");
    assert!(text.contains("gate:     AGREED"), "{text}");
    assert!(out.is_file());
    assert!(provenance(&out).contains("native_steps: 400000000000\n"));
    // `--native-steps` replaces it; too small is INCONCLUSIVE, refused, and NAMED
    // so — never "the engines DISAGREE".
    let (code, text, out) = build("budget-small", "सम्मतः", LONG, &["--native-steps", "1000"]);
    assert_eq!(code, Some(DIVERGED), "{text}");
    assert!(!out.exists());
    let p = provenance(&out);
    assert!(
        p.contains("cause: INCONCLUSIVE — the native run did not finish (step limit 1000;"),
        "{p}"
    );
    assert!(
        !p.contains("DISAGREE") && !text.contains("DISAGREE"),
        "{text}"
    );
}

/// `tools/src-rev.sh` over `rel` (relative to the tree root): its exit and stamp.
fn src_rev(rel: &str) -> (Option<i32>, String) {
    let r = Command::new("sh")
        .arg(root().join("tools/src-rev.sh"))
        .arg(rel)
        .output()
        .expect("sh runs");
    (
        r.status.code(),
        String::from_utf8_lossy(&r.stdout).trim().to_string(),
    )
}

#[test]
fn an_awkward_file_name_is_hashed_and_moves_the_stamp() {
    // The first src-rev aborted at a quote and printed the EMPTY input's hash with
    // exit 0, and dropped a name with a space: a stamp that did not move.
    for (slot, name) in [("quote", "0'x"), ("space", "a b"), ("newline", "n\nl")] {
        let d = scratch(&format!("srcrev-{slot}"));
        std::fs::write(d.join("lib.rs"), "fn main() {}\n").unwrap();
        std::fs::write(d.join(name), "one").unwrap();
        let rel = format!("target/w381-gate/srcrev-{slot}");
        let (code, first) = src_rev(&rel);
        assert_eq!(code, Some(0), "{slot}");
        assert_eq!(first.len(), 16, "{slot}: {first:?}");
        assert_ne!(first, "e3b0c44298fc1c14", "{slot}: the empty input's hash");
        std::fs::write(d.join(name), "two").unwrap();
        let (_, second) = src_rev(&rel);
        assert_ne!(
            first, second,
            "{slot}: editing {name:?} left the stamp unmoved"
        );
    }
}

#[test]
fn one_tree_stamps_alike_on_every_host() {
    // Apple's /sbin/sha256sum prints a newline or backslash name unlike GNU
    // sha256sum and shasum, so the Mac and Linux stamped one tree differently.
    // PINNED: GNU sha256sum on ubuntu and shasum on the Mac both print this
    // (the earlier script on the Mac printed 59ca2f2e80ba5065).
    let d = scratch("srcrev-pin");
    std::fs::write(d.join("lib.rs"), "fn main() {}\n").unwrap();
    std::fs::write(d.join("n\nl"), "one").unwrap();
    std::fs::write(d.join("b\\s"), "two").unwrap();
    std::fs::write(d.join("q'u\"o te"), "three").unwrap();
    let (code, stamp) = src_rev("target/w381-gate/srcrev-pin");
    assert_eq!(code, Some(0));
    assert_eq!(
        stamp, "5073ad7a8ec59d62",
        "this host's hasher stamps unlike the others"
    );
}

#[test]
fn a_file_that_cannot_be_hashed_is_no_stamp() {
    let d = scratch("srcrev-unreadable");
    std::fs::write(d.join("lib.rs"), "fn main() {}\n").unwrap();
    let f = d.join("locked.rs");
    std::fs::write(&f, "x").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&f, std::fs::Permissions::from_mode(0o000)).unwrap();
    }
    if std::fs::read(&f).is_ok() {
        eprintln!("SKIPPED: this user can read a mode-000 file (root?), so no hash can fail");
        return;
    }
    let (code, stamp) = src_rev("target/w381-gate/srcrev-unreadable");
    assert_ne!(code, Some(0), "a failed hash exited 0 with {stamp:?}");
    assert!(stamp.is_empty(), "a failed hash printed {stamp:?}");
}

/// Makes a FIFO at `p` (`mkfifo`; no FFI in this crate's tests).
#[cfg(unix)]
fn mkfifo(p: &Path) {
    let ok = Command::new("mkfifo").arg(p).status().expect("mkfifo runs");
    assert!(ok.success(), "mkfifo {}", p.display());
}

#[cfg(unix)]
#[test]
fn a_fifo_at_o_is_written_through_and_not_replaced() {
    use std::os::unix::fs::FileTypeExt;
    // AGREED: the FIFO is still a FIFO, and its reader gets the ELF.
    let d = scratch("fifo-agree-pipe");
    let fifo = d.join("pipe");
    mkfifo(&fifo);
    let f2 = fifo.clone();
    let reader = std::thread::spawn(move || std::fs::read(f2).expect("the FIFO reads"));
    let (code, text, _, _) = build_to("fifo-agree", "सम्मतः", AGREES, &[], Some(&fifo), &[]);
    // CHECKED BEFORE THE JOIN: a build that REPLACED the FIFO with a file leaves
    // the reader blocked on the orphaned FIFO forever, so joining first turned
    // that regression into a hang, not a red (seen running the red on 048727c4).
    assert!(
        std::fs::symlink_metadata(&fifo)
            .unwrap()
            .file_type()
            .is_fifo(),
        "the FIFO at -o was replaced by a file: {text}"
    );
    let got = reader.join().unwrap();
    assert_eq!(code, Some(0), "{text}");
    assert!(has_elf(&got), "the FIFO's reader got no ELF: {text}");
    // REFUSED: no octet reaches the reader. The test opens the FIFO for writing
    // itself after the build, so the reader sees an end and not a hang.
    let d = scratch("fifo-diverge-pipe");
    let fifo = d.join("pipe");
    mkfifo(&fifo);
    let f2 = fifo.clone();
    let reader = std::thread::spawn(move || std::fs::read(f2).expect("the FIFO reads"));
    let (code, text, _, _) = build_to("fifo-diverge", "परकर्तृ", DIVERGES, &[], Some(&fifo), &[]);
    // Checked before the join, as above.
    assert!(
        std::fs::symlink_metadata(&fifo)
            .unwrap()
            .file_type()
            .is_fifo(),
        "the FIFO at -o was replaced by a file: {text}"
    );
    drop(std::fs::OpenOptions::new().write(true).open(&fifo).unwrap());
    let got = reader.join().unwrap();
    assert_eq!(code, Some(DIVERGED), "{text}");
    assert!(
        got.is_empty(),
        "a REFUSED image reached the FIFO: {} octets",
        got.len()
    );
}

/// `t1_image -o /dev/stdout` with its stdout REDIRECTED TO A FILE, the shell's
/// `> x.elf`: there `/dev/stdout` resolves to a regular file.
fn build_to_stdout_file(slot: &str, module: &str, src: &str) -> (Option<i32>, String, Vec<u8>) {
    runner_beside();
    let d = scratch(slot);
    let file = d.join(format!("{module}.t1"));
    std::fs::write(&file, src).expect("fixture written");
    let sink = d.join("x.elf");
    let r = Command::new(env!("CARGO_BIN_EXE_t1_image"))
        .current_dir(root())
        .arg("--spec-root")
        .arg(root().join("spec"))
        .arg("--compiler")
        .arg(root().join("crates/sadhana-t1/src"))
        .arg("--load")
        .arg(&file)
        .args(["--entry", module, "मुख्यम्", "-o", "/dev/stdout"])
        .arg(&file)
        .stdout(std::fs::File::create(&sink).expect("the sink opens"))
        .output()
        .expect("t1_image runs");
    let text = String::from_utf8_lossy(&r.stderr).into_owned();
    (
        r.status.code(),
        text,
        std::fs::read(&sink).unwrap_or_default(),
    )
}

#[test]
fn dev_stdout_redirected_to_a_file_takes_only_an_agreed_image() {
    let (code, text, got) = build_to_stdout_file("stdout-file-agree", "सम्मतः", AGREES);
    assert_eq!(code, Some(0), "{text}");
    assert!(has_elf(&got), "the redirected file holds no ELF: {text}");
    let (code, text, got) = build_to_stdout_file("stdout-file-diverge", "परकर्तृ", DIVERGES);
    assert_eq!(code, Some(DIVERGED), "{text}");
    assert!(
        !has_elf(&got),
        "a REFUSED image reached the redirected file: {text}"
    );
}

#[cfg(unix)]
#[test]
fn a_symlink_to_a_device_is_not_replaced() {
    let d = scratch("devlink-pipe");
    let link = d.join("null");
    std::os::unix::fs::symlink("/dev/null", &link).unwrap();
    let (code, text, _, _) = build_to("devlink", "सम्मतः", AGREES, &[], Some(&link), &[]);
    assert_eq!(code, Some(0), "{text}");
    assert!(
        std::fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the symlink to /dev/null was replaced: {text}"
    );
}

#[test]
fn the_native_half_is_given_the_same_input() {
    let d = scratch("input-file");
    let input = d.join("in.bin");
    std::fs::write(&input, b"abc").unwrap();
    let i = input.to_string_lossy().into_owned();
    let (code, text, out) = build(
        "input",
        "निवेशक",
        &reads_its_input(),
        &["--input", &i, "--input-name", "निवेशक"],
    );
    assert_eq!(code, Some(0), "{text}");
    let p = provenance(&out);
    // 97 + 98 + 99: the predict AND the image read the three octets.
    assert!(p.contains("predict: finished, status 294"), "{p}");
    assert!(p.contains("native: finished, status 294"), "{p}");
    assert!(p.contains("input_sha256: "), "{p}");
}
