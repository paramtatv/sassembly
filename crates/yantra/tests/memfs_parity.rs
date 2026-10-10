//! **THE IN-MEMORY FILE ROOT AGREES WITH THE DISK ROOT, program for program.**
//!
//! `patra::MemFs` (what `yantra-wasm` hands a browser run) and `yantra-run --files DIR`
//! must give the SAME halt status and the SAME retired-instruction count for the same
//! image: a file window that cost the program a different number of steps would break
//! T-102 between venues. Programs are compiled by the `.t1` compiler (interpreted), as
//! `t1_files_flag.rs` does.
//!
//! When `MEMFS_ELF_DIR` is set, the ELFs are also left there for
//! `tools/check-yantra-memfs.mjs`, which drives the same programs through the real wasm.

//! The program's answer is its finisher status, which the halt line carries
//! (`status: Some(N)`), because `yantra-run` exits 1 on any non-zero status.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};
use std::process::Command;

const FUEL: u64 = 80_000_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn arena(vs: Vec<Value>) -> Value {
    Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vs)))
}

const READS: &str = "मण्डलम् पठकः ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः मार्गः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् कपत्रम् इति ।
    चरः पात्रम् ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ६४ आदि
        पात्रम् अङ्कः क्रमः अन्तः भवति ० ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    चरः स्थितिः ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    स्थितिः अङ्कः ० अन्तः भवति ९९९ ।
    चरः अवगणना ॱॱ न६४ भवति अष्टकॱपत्रम् आरभ्य मार्गः ऽ पात्रम् ऽ स्थितिः समाप्तम् ।
    चरः गणना ॱॱ न६४ भवति स्थितिः अङ्कः ० अन्तः ।
    यदि गणना अधिकम् ६४ आदि
        प्रत्यागमनम् गणना ।
    इति
    चरः योगफलम् ॱॱ न६४ भवति ० ।
    चरः ज ॱॱ न६४ भवति ० ।
    यावत् ज न्यूनम् गणना आदि
        योगफलम् भवति योगफलम् योगः पात्रम् अङ्कः ज अन्तः ।
        ज भवति ज योगः १ ।
    इति
    प्रत्यागमनम् योगफलम् ।
इति
";

const ROUND_TRIP: &str = "मण्डलम् लेखकः ॥
आयातः अष्टक ।

सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि
    चरः मार्गः ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् खपत्रम् इति ।
    चरः पात्रम् ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    पात्रम् अङ्कः ० अन्तः भवति १० ।
    पात्रम् अङ्कः १ अन्तः भवति २० ।
    पात्रम् अङ्कः २ अन्तः भवति ३० ।
    पात्रम् अङ्कः ३ अन्तः भवति ४० ।
    चरः स्थितिः ॱॱ अङ्कः अन्तः न६४ भवति ० ।
    स्थितिः अङ्कः ० अन्तः भवति ९९९ ।
    चरः अवगणनक ॱॱ न६४ भवति अष्टकॱपत्रलेखनम् आरभ्य मार्गः ऽ पात्रम् ऽ स्थितिः समाप्तम् ।
    चरः लिखितम् ॱॱ न६४ भवति स्थितिः अङ्कः ० अन्तः ।
    यदि लिखितम् अधिकम् ४ आदि
        प्रत्यागमनम् लिखितम् ।
    इति
    ॰ AND BACK. A fresh buffer, so nothing the write left behind can be read.
    चरः पुनःपात्रम् ॱॱ अङ्कः अन्तः अ८ भवति ० ।
    चरः क्रमः ॱॱ न६४ भवति ० ।
    यावत् क्रमः न्यूनम् ३२ आदि
        पुनःपात्रम् अङ्कः क्रमः अन्तः भवति ० ।
        क्रमः भवति क्रमः योगः १ ।
    इति
    स्थितिः अङ्कः ० अन्तः भवति ९९९ ।
    चरः अवगणनख ॱॱ न६४ भवति अष्टकॱपत्रम् आरभ्य मार्गः ऽ पुनःपात्रम् ऽ स्थितिः समाप्तम् ।
    चरः गणना ॱॱ न६४ भवति स्थितिः अङ्कः ० अन्तः ।
    यदि गणना अधिकम् ३२ आदि
        प्रत्यागमनम् गणना ।
    इति
    चरः योगफलम् ॱॱ न६४ भवति ० ।
    चरः ज ॱॱ न६४ भवति ० ।
    यावत् ज न्यूनम् गणना आदि
        योगफलम् भवति योगफलम् योगः पुनःपात्रम् अङ्कः ज अन्तः ।
        ज भवति ज योगः १ ।
    इति
    प्रत्यागमनम् योगफलम् ।
इति
";
fn build(src: &str, module: &str) -> Vec<u8> {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    it.call(
        "शृङ्खलाॱप्रवेशन्यासः",
        vec![octets(module.as_bytes()), octets("मुख्यम्".as_bytes())],
        1_000_000_000,
    )
    .expect("the entry is named");
    it.call(
        "शृङ्खलाॱमण्डलानिप्रतिबिम्बम्",
        vec![
            arena(vec![octets(src.as_bytes())]),
            arena(vec![octets(module.as_bytes())]),
            Value::Int(1),
        ],
        FUEL,
    )
    .expect("मण्डलानिप्रतिबिम्बम् runs")
    .octets()
    .map(|o| o.as_slice().to_vec())
    .unwrap_or_default()
}

fn dev(mut n: usize) -> String {
    const D: [char; 10] = ['०', '१', '२', '३', '४', '५', '६', '७', '८', '९'];
    if n == 0 {
        return "०".into();
    }
    let mut s = Vec::new();
    while n > 0 {
        s.push(D[n % 10]);
        n /= 10;
    }
    s.iter().rev().collect()
}

/// `src` with the path literal `old` replaced by the ASCII path `ascii` followed by `क`.
fn with_ascii_path(src: &str, old: &str, ascii: &str) -> String {
    let mut a = ascii.to_string();
    while !a.len().is_multiple_of(3) {
        a.insert(0, '/'); // a leading `//` is the same absolute path
    }
    let letters = a.len() / 3 + 1;
    let mut lit = String::from("उक्तम् ");
    lit.push_str(&"क".repeat(letters));
    lit.push_str(" इति ।");
    for (i, b) in a.bytes().enumerate() {
        lit.push_str(&format!(
            "\n    मार्गः अङ्कः {} अन्तः भवति {} ।",
            dev(i),
            dev(usize::from(b))
        ));
    }
    let out = src.replace(old, &lit);
    assert_ne!(out, src, "the fixture did not change");
    out
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("memfs-parity-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&d).ok();
    std::fs::create_dir_all(d.join("root")).expect("scratch");
    d
}

/// Native: the shipped binary, `--files root`. Returns (finisher status, steps).
fn native(d: &Path, elf: &Path) -> (u64, u64) {
    let o = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .arg("--files")
        .arg(d.join("root"))
        .arg(elf)
        .output()
        .expect("yantra-run runs");
    let err = String::from_utf8_lossy(&o.stderr).into_owned();
    let st = err
        .lines()
        .find(|l| l.starts_with("halt: Finisher"))
        .and_then(|l| {
            l[l.find("Some(").unwrap() + 5..]
                .split(')')
                .next()?
                .parse()
                .ok()
        })
        .unwrap_or_else(|| panic!("no finisher status: {err}"));
    let steps = err
        .lines()
        .find_map(|l| l.strip_prefix("steps: ")?.split(' ').next()?.parse().ok())
        .unwrap_or_else(|| panic!("no steps line: {err}"));
    (st, steps)
}

/// In memory: the library machine with `patra_mem` set. Returns (status, steps, fs).
fn in_memory(elf: &[u8], seed: &[(&str, &[u8])]) -> (u64, u64, yantra::patra::MemFs) {
    let mut fs = yantra::patra::MemFs::default();
    for (n, d) in seed {
        assert_eq!(fs.seed(n, d), yantra::patra::Status::Wrote(d.len() as u64));
    }
    let mut m =
        yantra::Machine::load_elf_spanning(elf, 8 << 20, yantra::Span::FileBacked).expect("loads");
    m.patra_mem = Some(fs);
    let mut out: Vec<u8> = Vec::new();
    let reason = m.run(10_000_000, &mut out);
    let st = match reason {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("did not finish: {other:?}"),
    };
    (st, m.time, m.patra_mem.take().unwrap())
}

fn keep(name: &str, elf: &[u8]) {
    if let Some(dir) = std::env::var_os("MEMFS_ELF_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(Path::new(&dir).join(name), elf).unwrap();
    }
}

#[test]
fn a_write_round_trip_has_the_same_status_and_steps_in_memory_and_on_disk() {
    let d = scratch("wr");
    let elf = build(ROUND_TRIP, "लेखकः");
    keep("roundtrip.elf", &elf);
    let p = d.join("w.elf");
    std::fs::write(&p, &elf).unwrap();
    let (ns, nsteps) = native(&d, &p);
    let (ms, msteps, fs) = in_memory(&elf, &[]);
    assert_eq!((ms, msteps), (ns, nsteps));
    assert_eq!(ms, 100);
    assert_eq!(fs.nth(0), Some(("खपत्रम्", &[10u8, 20, 30, 40][..])));
    assert_eq!(
        std::fs::read(d.join("root/खपत्रम्")).unwrap(),
        [10, 20, 30, 40]
    );
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_seeded_read_has_the_same_status_and_steps() {
    let d = scratch("rd");
    std::fs::write(d.join("root/कपत्रम्"), b"abc\n").unwrap();
    let elf = build(READS, "पठकः");
    keep("reads.elf", &elf);
    let p = d.join("r.elf");
    std::fs::write(&p, &elf).unwrap();
    let (ns, nsteps) = native(&d, &p);
    let (ms, msteps, _) = in_memory(&elf, &[("कपत्रम्", b"abc\n")]);
    assert_eq!((ms, msteps), (ns, nsteps));
    assert_eq!(ms, 304);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn an_escape_is_refused_by_name_with_the_same_steps() {
    let d = scratch("esc");
    std::fs::write(d.join("क"), b"abc\n").unwrap();
    let refused = yantra::patra::Status::Refused.word() & 0xFFFF_FFFF_FFFF;
    for (tag, prefix) in [("dotdot", "../"), ("abs", "/")] {
        let src = with_ascii_path(READS, "उक्तम् कपत्रम् इति ।", prefix);
        let elf = build(&src, "पठकः");
        keep(&format!("esc-{tag}.elf"), &elf);
        let p = d.join("e.elf");
        std::fs::write(&p, &elf).unwrap();
        let (ns, nsteps) = native(&d, &p);
        let (ms, msteps, fs) = in_memory(&elf, &[("क", b"abc\n")]);
        assert_eq!((ms, msteps), (ns, nsteps), "{tag}");
        assert_eq!(ms, refused, "{tag}");
        assert_eq!(fs.len(), 1, "{tag}: nothing written");
    }
    std::fs::remove_dir_all(&d).ok();
}

// ── the refusal classes the two roots share, end to end ──
//
// Each case compiles a real program, runs it under `--files` on a disk laid out like the
// memory root, and in memory, and requires the same status AND the same steps. The cap
// class has no disk twin (a disk has no cap), so it is checked against its expected word.

/// `src` with the path literal replaced by the RAW octets `raw` followed by `क`; the
/// padding goes at the END (before the `क`) so a leading `/` is never invented.
fn with_raw_path(src: &str, old: &str, raw: &[u8]) -> String {
    let mut a = raw.to_vec();
    while !a.len().is_multiple_of(3) {
        a.push(b'p');
    }
    let mut lit = String::from("उक्तम् ");
    lit.push_str(&"क".repeat(a.len() / 3 + 1));
    lit.push_str(" इति ।");
    for (i, b) in a.iter().enumerate() {
        lit.push_str(&format!(
            "\n    मार्गः अङ्कः {} अन्तः भवति {} ।",
            dev(i),
            dev(usize::from(*b))
        ));
    }
    let out = src.replace(old, &lit);
    assert_ne!(out, src, "the fixture did not change");
    out
}

fn word(s: yantra::patra::Status) -> u64 {
    s.word() & 0xFFFF_FFFF_FFFF
}

/// Run `src` on disk (files laid out under root) and in memory (`seed`); both must
/// finish with `expect` and the same step count.
fn both(tag: &str, src: &str, module: &str, seed: &[(&str, &[u8])], expect: u64) {
    let d = scratch(tag);
    for (n, data) in seed {
        let p = d.join("root").join(n);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, data).unwrap();
    }
    let elf = build(src, module);
    let p = d.join("x.elf");
    std::fs::write(&p, &elf).unwrap();
    let (ns, nsteps) = native(&d, &p);
    let (ms, msteps, _) = in_memory(&elf, seed);
    assert_eq!((ms, msteps), (ns, nsteps), "{tag}: memory vs disk");
    assert_eq!(ms, expect, "{tag}: status");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn notfound_is_the_same_in_memory_and_on_disk() {
    both(
        "nf",
        READS,
        "पठकः",
        &[],
        word(yantra::patra::Status::NotFound),
    );
}

#[test]
fn too_large_is_the_same_in_memory_and_on_disk() {
    // READS offers a 64-octet buffer; a 100-octet file does not fit.
    let big = [7u8; 100];
    both(
        "tl",
        READS,
        "पठकः",
        &[("कपत्रम्", &big)],
        word(yantra::patra::Status::TooLarge(0)),
    );
}

#[test]
fn bad_path_and_nul_are_the_same_in_memory_and_on_disk() {
    let bad = with_raw_path(READS, "उक्तम् कपत्रम् इति ।", &[0xff]);
    both(
        "bp",
        &bad,
        "पठकः",
        &[],
        word(yantra::patra::Status::BadPath),
    );
    let nul = with_raw_path(READS, "उक्तम् कपत्रम् इति ।", &[0]);
    both(
        "nul",
        &nul,
        "पठकः",
        &[],
        word(yantra::patra::Status::NotFound),
    );
}

#[test]
fn a_missing_parent_is_the_same_in_memory_and_on_disk() {
    let rd = with_raw_path(READS, "उक्तम् कपत्रम् इति ।", b"dd/");
    both(
        "mp-r",
        &rd,
        "पठकः",
        &[],
        word(yantra::patra::Status::NotFound),
    );
    let wr = with_raw_path(ROUND_TRIP, "उक्तम् खपत्रम् इति ।", b"dd/");
    both(
        "mp-w",
        &wr,
        "लेखकः",
        &[],
        word(yantra::patra::Status::NotWritten),
    );
    // CONTROL: with the directory present the write lands.
    both("mp-c", &wr, "लेखकः", &[("dd/keep", b"k")], 100);
}

#[test]
fn a_write_over_the_cap_is_not_written_and_leaves_nothing() {
    let elf = build(ROUND_TRIP, "लेखकः");
    // 4 octets of data plus the name plus a 64-octet entry overhead exceed a total of 20.
    let mut m =
        yantra::Machine::load_elf_spanning(&elf, 8 << 20, yantra::Span::FileBacked).expect("loads");
    m.patra_mem = Some(yantra::patra::MemFs::with_caps(20, 20).unwrap());
    let mut out: Vec<u8> = Vec::new();
    let st = match m.run(10_000_000, &mut out) {
        yantra::Halt::Finisher {
            status: Some(s), ..
        } => s,
        other => panic!("did not finish: {other:?}"),
    };
    assert_ne!(st, 100, "the write was served over the cap");
    assert_eq!(m.patra_mem.take().unwrap().len(), 0);
}

#[test]
fn a_write_to_a_directory_or_under_a_file_is_not_written_in_both() {
    let nw = word(yantra::patra::Status::NotWritten);
    // The path is `ddp` + `क` (the forced last letter): make that a directory.
    let dir = with_raw_path(ROUND_TRIP, "उक्तम् खपत्रम् इति ।", b"dd");
    both("wd", &dir, "लेखकः", &[("ddpक/keep", b"k")], nw);
    // `seedp/` + `क`: `seedp` is a FILE, so nothing can live under it.
    let under = with_raw_path(ROUND_TRIP, "उक्तम् खपत्रम् इति ।", b"seedp/");
    both("wf", &under, "लेखकः", &[("seedp", b"k")], nw);
    let rd = with_raw_path(READS, "उक्तम् कपत्रम् इति ।", b"seedp/");
    both(
        "rf",
        &rd,
        "पठकः",
        &[("seedp", b"k")],
        word(yantra::patra::Status::NotFound),
    );
}
