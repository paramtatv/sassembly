//! **THE SELF-IMAGE RUNG TAKES ITS ENTRY FROM THE INPUT NAME** (owner override
//! 2026-10-09: entry-from-input).
//!
//! `स्वपरीक्षास्वप्रतिबिम्बम्` reads `निवेशमण्डलनाम`. With a NUL in it the name is
//! `module NUL routine` and that pair is the image's entry; with none the pair is
//! the compiler's own, as it always was. A NUL cannot travel in an environment
//! variable, so these tests inject the name with `yantra::input::inject`, which
//! takes bytes.
//!
//! Needs a Stage 1 image built from this tree (`SAS_STAGE1_ELF=<path>`), so it is
//! ignored by default; the interpreted half needs only the sources.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use yantra::Machine;

const MOD: &str = "निवेशक";
const R7: &str = "स्वपरीक्षाशून्या";
const R9: &str = "स्वपरीक्षाद्वितीया";
const ABSENT_M: &str = "अनुपस्थितः";
const ABSENT_R: &str = "स्वपरीक्षैका";
const FIXTURE: &str = "मण्डलम् निवेशक ॥
सार्वजनिक वृत्तिः स्वपरीक्षाशून्या ददाति न६४ आदि
    प्रत्यागमनम् ७ ।
इति
सार्वजनिक वृत्तिः स्वपरीक्षाद्वितीया ददाति न६४ आदि
    प्रत्यागमनम् ९ ।
इति
";

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn blob() -> Vec<u8> {
    blob_of(&[(MOD, FIXTURE)])
}

fn blob_of(parts: &[(&str, &str)]) -> Vec<u8> {
    let mut b = Vec::new();
    for (n, t) in parts {
        b.extend_from_slice(n.as_bytes());
        b.push(0);
        b.extend_from_slice(t.as_bytes());
        b.push(0);
    }
    b
}

/// The compiler's own entry pair, FIXED here (copied from the rung's source by
/// script) and not read back from it: a test that follows the source follows a
/// mutation of it. `default_pair_is_the_one_in_the_source` pins the two together.
const DEFAULT_M: &str = "शृङ्खला";
const DEFAULT_R: &str = "स्वपरीक्षास्वप्रतिबिम्बम्";

fn default_pair() -> (String, String) {
    (DEFAULT_M.to_string(), DEFAULT_R.to_string())
}

#[test]
fn default_pair_is_the_one_in_the_source() {
    let src = std::fs::read_to_string(root().join("crates/sadhana-t1/src/shrinkhala.t1")).unwrap();
    let want = format!("भवति प्रवेशन्यासः आरभ्य उक्तम् {DEFAULT_M} इति ऽ उक्तम् {DEFAULT_R} इति समाप्तम्");
    assert!(
        src.contains(&want),
        "the default call in shrinkhala.t1 is not the pinned pair"
    );
}

/// A corpus whose module is the DEFAULT pair, returning 5.
fn default_blob() -> Vec<u8> {
    let (m, r) = default_pair();
    let text = format!("मण्डलम् {m} ॥\nसार्वजनिक वृत्तिः {r} ददाति न६४ आदि\n    प्रत्यागमनम् ५ ।\nइति\n");
    blob_of(&[(&m, &text)])
}

fn name(m: &str, r: &str) -> Vec<u8> {
    let mut n = m.as_bytes().to_vec();
    n.push(0);
    n.extend_from_slice(r.as_bytes());
    n
}

/// (halt status, sink) of Stage 1 run natively on `blob` named `nm`.
fn native(nm: &[u8], trace: u64) -> (u64, Vec<u8>) {
    native_on(&blob(), nm, trace)
}

fn native_on(blob: &[u8], nm: &[u8], trace: u64) -> (u64, Vec<u8>) {
    let elf = std::fs::read(std::env::var("SAS_STAGE1_ELF").expect("SAS_STAGE1_ELF")).unwrap();
    let mut m = Machine::load_elf(&elf, 2_684_354_560).expect("stage 1 loads");
    let base = m.base;
    yantra::input::inject(&mut m.mem, base, blob, nm, trace).expect("inject");
    let mut out = Vec::new();
    match m.run(200_000_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => (s, out),
        other => panic!("stage 1 did not finish: {other:?}"),
    }
}

/// The image between markers 230 and 231, or empty.
fn image_of(sink: &[u8]) -> Vec<u8> {
    match sink.windows(4).position(|w| w == b"\x7fELF") {
        Some(o) => sink[o..sink.len() - 1].to_vec(),
        None => Vec::new(),
    }
}

fn run_image(elf: &[u8]) -> u64 {
    let mut m = Machine::load_elf(elf, yantra::ram_for(elf)).expect("image loads");
    let mut out = Vec::new();
    match m.run(200_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("image did not finish: {other:?}"),
    }
}

fn has(hay: &[u8], needle: &str) -> bool {
    hay.windows(needle.len()).any(|w| w == needle.as_bytes())
}

#[test]
#[ignore = "probe: needs SAS_STAGE1_ELF, a Stage 1 image built from this tree"]
fn native_stage1_takes_module_and_routine_from_the_input_name() {
    let (s, out) = native(&name(MOD, R7), 0);
    assert_eq!(s, 1200, "built");
    assert_eq!(run_image(&image_of(&out)), 7);
    let (s, out) = native(&name(MOD, R9), 0);
    assert_eq!(s, 1200);
    assert_eq!(
        run_image(&image_of(&out)),
        9,
        "the routine came from the input, not a constant"
    );
}

#[test]
#[ignore = "probe: needs SAS_STAGE1_ELF, a Stage 1 image built from this tree"]
fn native_stage1_without_a_nul_keeps_its_own_entry() {
    // No NUL: the compiler's own pair, present in this corpus, built and run.
    let (s, out) = native_on(&default_blob(), MOD.as_bytes(), 0);
    assert_eq!(s, 1200, "the default pair builds");
    assert_eq!(
        run_image(&image_of(&out)),
        5,
        "and it is the default routine that ran"
    );
    // The same corpus without the pair is refused at the link.
    let (s, _) = native("शृङ्खला".as_bytes(), 0);
    assert_eq!(s, 1601);
}

#[test]
#[ignore = "probe: needs SAS_STAGE1_ELF, a Stage 1 image built from this tree"]
fn native_stage1_refuses_a_missing_module_or_routine_by_name() {
    let (s, out) = native(&name(ABSENT_M, R7), 1);
    assert_eq!(s, 1601, "missing module");
    assert!(
        has(&out, ABSENT_M) || has(&out, R7),
        "the refusal names the entry; sink {out:?}"
    );
    // An empty module or an empty routine is refused by the rung itself, 1601.
    assert_eq!(native(&name(MOD, ""), 0).0, 1601, "empty routine");
    assert_eq!(native(&name("", R7), 0).0, 1601, "empty module");
    // The FIRST NUL splits: the routine is everything after it, NULs included, and
    // the refusal names that whole run.
    let mut two = name(MOD, R7);
    two.push(0);
    two.extend_from_slice(R9.as_bytes());
    let (s, out) = native(&two, 1);
    assert_eq!(s, 1601, "routine with a NUL in it is absent");
    let mut want = R7.as_bytes().to_vec();
    want.push(0);
    want.extend_from_slice(R9.as_bytes());
    assert!(
        out.windows(want.len()).any(|w| w == want.as_slice()),
        "split at the first NUL; sink {out:?}"
    );
    let (s, out) = native(&name(MOD, ABSENT_R), 1);
    assert_eq!(s, 1601, "missing routine");
    assert!(
        has(&out, ABSENT_R),
        "the refusal names the routine; sink {out:?}"
    );
}

#[test]
#[ignore = "probe: interprets the whole compiler; needs SAS_STAGE1_ELF too, and fails without it"]
fn the_interpreter_agrees_with_native() {
    let dir = root().join("crates/sadhana-t1/src");
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    paths.sort();
    let refs: Vec<&Path> = paths.iter().map(PathBuf::as_path).collect();
    let call_on = |blob: &[u8], nm: &[u8]| -> (i128, Vec<u8>) {
        let mut it = Interpreter::load_paths(&refs, &root().join("spec")).expect("load");
        for (g, v) in [
            ("निवेशपाठः", Value::Octets(Octets::new(blob))),
            ("निवेशमण्डलनाम", Value::Octets(Octets::new(nm))),
            ("निवेशानुरेखणम्", Value::Int(0)),
        ] {
            assert!(it.set_global(g, v));
        }
        let r = it
            .call("शृङ्खलाॱस्वपरीक्षास्वप्रतिबिम्बम्", vec![], 80_000_000_000)
            .expect("rung runs");
        let Value::Int(n) = r else { panic!("{r:?}") };
        (n, it.sink().to_vec())
    };
    let call = |nm: &[u8]| call_on(&blob(), nm);
    let (s, dout) = call_on(&default_blob(), MOD.as_bytes());
    assert_eq!(s, 1200, "the default pair builds");
    assert_eq!(run_image(&image_of(&dout)), 5, "the default routine ran");
    let (s, out) = call(&name(MOD, R9));
    assert_eq!(s, 1200);
    assert_eq!(run_image(&image_of(&out)), 9);
    let (s, _) = call(&name(ABSENT_M, R7));
    assert_eq!(s, 1601);
    assert_eq!(call(&name(MOD, "")).0, 1601, "empty routine");
    assert_eq!(call(&name("", R7)).0, 1601, "empty module");
    // THE CROSS-ENGINE CHECK IS NOT OPTIONAL: no SAS_STAGE1_ELF fails, never skips.
    let _ = std::env::var("SAS_STAGE1_ELF").expect("SAS_STAGE1_ELF is required: the native half");
    let (_, nout) = native(&name(MOD, R9), 0);
    assert_eq!(
        image_of(&nout),
        image_of(&out),
        "interpreter and native images are byte-identical"
    );
}

fn yantra_run(envs: &[(&str, String)], stage1: &str) -> std::process::Output {
    let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_yantra-run"));
    c.arg(stage1)
        .env_remove("YANTRA_INPUT")
        .env_remove("YANTRA_INPUT_NAME")
        .env_remove("YANTRA_INPUT_ENTRY")
        .env("YANTRA_RAM", "2684354560")
        .env("YANTRA_STEPS", "200000000000");
    for (k, v) in envs {
        c.env(k, v);
    }
    c.output().expect("yantra-run runs")
}

#[test]
#[ignore = "probe: needs SAS_STAGE1_ELF, a Stage 1 image built from this tree"]
fn yantra_run_builds_a_non_default_entry_from_yantra_input_entry() {
    let stage1 = std::env::var("SAS_STAGE1_ELF").expect("SAS_STAGE1_ELF");
    let dir = std::env::temp_dir().join(format!("sas-entry-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let blob_path = dir.join("one.blob");
    std::fs::write(&blob_path, blob()).unwrap();
    let b = blob_path.to_string_lossy().into_owned();
    for (r, want) in [(R7, 7), (R9, 9)] {
        let out = yantra_run(
            &[
                ("YANTRA_INPUT", b.clone()),
                ("YANTRA_INPUT_ENTRY", format!("{MOD} {r}")),
            ],
            &stage1,
        );
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(err.contains("status: Some(1200)"), "built; stderr {err}");
        assert_eq!(run_image(&image_of(&out.stdout)), want);
    }
    // A missing routine is refused at the link.
    let out = yantra_run(
        &[
            ("YANTRA_INPUT", b.clone()),
            ("YANTRA_INPUT_ENTRY", format!("{MOD} {ABSENT_R}")),
        ],
        &stage1,
    );
    assert!(String::from_utf8_lossy(&out.stderr).contains("status: Some(1601)"));
    // Malformed, and combined with the name: refused by name, nothing ran.
    for bad in [
        MOD.to_string(),
        format!("{MOD}  {R7}"),
        format!(" {R7}"),
        format!("{MOD} {R7} x"),
    ] {
        let out = yantra_run(
            &[
                ("YANTRA_INPUT", b.clone()),
                ("YANTRA_INPUT_ENTRY", bad.clone()),
            ],
            &stage1,
        );
        assert!(!out.status.success(), "{bad:?} must be refused");
        assert!(
            String::from_utf8_lossy(&out.stderr).contains("YANTRA_INPUT_ENTRY"),
            "{bad:?}"
        );
    }
    // Not UTF-8, and over the 4096-octet limit: refused by name.
    {
        use std::os::unix::ffi::OsStrExt;
        let mut bad = std::ffi::OsString::from(MOD);
        bad.push(std::ffi::OsStr::from_bytes(b" \xff\xfe"));
        let mut c = std::process::Command::new(env!("CARGO_BIN_EXE_yantra-run"));
        let out = c
            .arg(&stage1)
            .env_remove("YANTRA_INPUT_NAME")
            .env("YANTRA_INPUT", &b)
            .env("YANTRA_INPUT_ENTRY", &bad)
            .output()
            .unwrap();
        assert!(!out.status.success());
        let err = String::from_utf8_lossy(&out.stderr);
        assert!(
            err.contains("YANTRA_INPUT_ENTRY") && err.contains("not UTF-8"),
            "{err}"
        );
    }
    let long = format!("{MOD} {}", "x".repeat(4097 - MOD.len() - 1)); // 4097 octets: one over
    let out = yantra_run(
        &[("YANTRA_INPUT", b.clone()), ("YANTRA_INPUT_ENTRY", long)],
        &stage1,
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("4096"));
    let ok4096 = format!("{MOD} {}", "x".repeat(4096 - MOD.len() - 1));
    let out = yantra_run(
        &[("YANTRA_INPUT", b.clone()), ("YANTRA_INPUT_ENTRY", ok4096)],
        &stage1,
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("status: Some(1601)"),
        "exactly 4096 octets is allowed"
    );
    let out = yantra_run(
        &[
            ("YANTRA_INPUT", b),
            ("YANTRA_INPUT_ENTRY", format!("{MOD} {R7}")),
            ("YANTRA_INPUT_NAME", MOD.to_string()),
        ],
        &stage1,
    );
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("both set"));
}

/// A Saṃpuṭa archive names its own input, so YANTRA_INPUT_ENTRY beside --smp is a
/// conflict, refused with 64 and by name. No Stage 1 needed: it is refused before
/// the file is parsed.
#[test]
fn yantra_input_entry_is_refused_beside_an_archive() {
    let f = std::env::temp_dir().join(format!("sas-smp-{}.bin", std::process::id()));
    std::fs::write(&f, b"not an archive").unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .arg("--smp")
        .arg(&f)
        .env_remove("YANTRA_INPUT")
        .env_remove("YANTRA_INPUT_NAME")
        .env("YANTRA_INPUT_ENTRY", format!("{MOD} {R7}"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&out.stderr).contains("YANTRA_INPUT_ENTRY"));
}
