//! **`yantra-run --files DIR`, THE BINARY, END TO END (v1.0.1).**
//!
//! `t1_file_end_to_end.rs` proves the window through the LIBRARY with
//! `Machine.patra_root` set by the test. This runs the shipped BINARY, because the
//! flag is the only way a user can set that field: the programs are compiled by
//! the `.t1` compiler (interpreted, as that file does) into an ELF on disk and
//! `yantra-run` is executed on it. Without `--files` every request is refused by
//! name; with it, DIR is the root and nothing outside it is reachable.
//!
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

struct Ran {
    code: Option<i32>,
    stderr: String,
}

impl Ran {
    /// The finisher status the halt line carries, if it finished with one.
    fn status(&self) -> Option<u64> {
        let line = self
            .stderr
            .lines()
            .find(|l| l.starts_with("halt: Finisher"))?;
        let at = line.find("Some(")? + 5;
        line[at..].split(')').next()?.parse().ok()
    }
}

fn scratch(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "files-flag-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("a clock after 1970")
            .as_nanos()
    ));
    std::fs::create_dir_all(d.join("root")).expect("scratch");
    d
}

fn elf(dir: &Path, name: &str, src: &str, module: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, build(src, module)).expect("write the ELF");
    p
}

fn run(args: &[&std::ffi::OsStr]) -> Ran {
    let o = Command::new(env!("CARGO_BIN_EXE_yantra-run"))
        .args(args)
        .output()
        .expect("yantra-run runs");
    Ran {
        code: o.status.code(),
        stderr: String::from_utf8_lossy(&o.stderr).into_owned(),
    }
}

fn os(s: &str) -> &std::ffi::OsStr {
    std::ffi::OsStr::new(s)
}

const SUM: u64 = 304; // `abc\n`

#[test]
fn files_grants_the_root_and_the_program_reads_inside_it() {
    let d = scratch("rd");
    std::fs::write(d.join("root/कपत्रम्"), b"abc\n").unwrap();
    let e = elf(&d, "r.elf", READS, "पठकः");
    let r = run(&[os("--files"), d.join("root").as_os_str(), e.as_os_str()]);
    assert_eq!(r.status(), Some(SUM), "stderr: {}", r.stderr);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn without_the_flag_the_same_program_is_refused_by_name() {
    let d = scratch("no");
    std::fs::write(d.join("root/कपत्रम्"), b"abc\n").unwrap();
    let e = elf(&d, "r.elf", READS, "पठकः");
    let r = run(&[e.as_os_str()]);
    let s = r.status().expect("it finished");
    assert_ne!(s, SUM, "no flag, yet the file was read");
    assert_ne!(s, 999, "the host never answered");
    // And the same refusal word the library test sees for a rootless machine.
    assert_eq!(
        s,
        (yantra::patra::Status::Refused.word() & 0xFFFF_FFFF_FFFF)
    );
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn files_writes_inside_the_root_and_reads_it_back() {
    let d = scratch("wr");
    let e = elf(&d, "w.elf", ROUND_TRIP, "लेखकः");
    let r = run(&[os("--files"), d.join("root").as_os_str(), e.as_os_str()]);
    assert_eq!(r.status(), Some(100), "stderr: {}", r.stderr);
    assert_eq!(
        std::fs::read(d.join("root/खपत्रम्")).unwrap(),
        vec![10u8, 20, 30, 40]
    );
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn without_the_flag_a_write_creates_nothing() {
    let d = scratch("nw");
    let e = elf(&d, "w.elf", ROUND_TRIP, "लेखकः");
    let r = run(&[e.as_os_str()]);
    assert_ne!(r.status(), Some(100), "stderr: {}", r.stderr);
    assert!(!d.join("root/खपत्रम्").exists());
    assert!(!d.join("खपत्रम्").exists());
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_program_naming_dotdot_cannot_reach_a_file_outside_the_root() {
    let d = scratch("dd");
    // The secret `क` is OUTSIDE root/, next to it. ASCII is outside the source repertoire,
    // so the path `../क` is built by overwriting the first three octets of `कक` (the
    // same six octets, which keeps the run's length) with `.`, `.`, `/`.
    std::fs::write(d.join("क"), b"abc\n").unwrap();
    let src = READS.replace(
        "उक्तम् कपत्रम् इति ।",
        "उक्तम् कक इति ।\n    मार्गः अङ्कः ० अन्तः भवति ४६ ।\n    मार्गः अङ्कः १ अन्तः भवति ४६ ।\n    मार्गः अङ्कः २ अन्तः भवति ४७ ।",
    );
    assert_ne!(src, READS, "the fixture did not change");
    let e = elf(&d, "dd.elf", &src, "पठकः");
    let r = run(&[os("--files"), d.join("root").as_os_str(), e.as_os_str()]);
    let s = r.status().expect("it finished");
    assert_ne!(s, SUM, "the program read the file outside the root");
    assert_eq!(
        s,
        (yantra::patra::Status::Refused.word() & 0xFFFF_FFFF_FFFF)
    );
    // CONTROL: the same file, named without the climb, inside the root, is read.
    std::fs::write(d.join("root/कपत्रम्"), b"abc\n").unwrap();
    let e2 = elf(&d, "ok.elf", READS, "पठकः");
    let r2 = run(&[os("--files"), d.join("root").as_os_str(), e2.as_os_str()]);
    assert_eq!(r2.status(), Some(SUM));
    std::fs::remove_dir_all(&d).ok();
}

#[cfg(unix)]
#[test]
fn a_symlink_in_the_root_to_a_file_outside_is_refused_by_the_binary() {
    let d = scratch("ln");
    std::fs::write(d.join("secret"), b"abc\n").unwrap();
    std::os::unix::fs::symlink(d.join("secret"), d.join("root/कपत्रम्")).unwrap();
    let e = elf(&d, "r.elf", READS, "पठकः");
    let r = run(&[os("--files"), d.join("root").as_os_str(), e.as_os_str()]);
    let s = r.status().expect("it finished");
    assert_ne!(s, SUM, "a symlink led out of the root");
    assert_eq!(
        s,
        (yantra::patra::Status::Refused.word() & 0xFFFF_FFFF_FFFF)
    );
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_missing_or_non_directory_root_is_refused_at_load() {
    let d = scratch("bad");
    let e = elf(&d, "r.elf", READS, "पठकः");
    let missing = d.join("nope");
    let r = run(&[os("--files"), missing.as_os_str(), e.as_os_str()]);
    assert_eq!(r.code, Some(1));
    assert!(r.stderr.contains("refused at load"), "{}", r.stderr);
    assert!(!r.stderr.contains("halt:"), "it ran: {}", r.stderr);
    let file = d.join("plain");
    std::fs::write(&file, b"x").unwrap();
    let r = run(&[os("--files"), file.as_os_str(), e.as_os_str()]);
    assert_eq!(r.code, Some(1));
    assert!(r.stderr.contains("not a directory"), "{}", r.stderr);
    assert!(!r.stderr.contains("halt:"), "it ran: {}", r.stderr);
    let r = run(&[os("--files")]);
    assert_eq!(r.code, Some(1));
    assert!(r.stderr.contains("usage"), "{}", r.stderr);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn files_is_read_before_the_event_flags_and_does_not_disturb_them() {
    let d = scratch("ev");
    std::fs::write(d.join("root/कपत्रम्"), b"abc\n").unwrap();
    let e = elf(&d, "r.elf", READS, "पठकः");
    // This image declares no event interface, so the log is refused AT LOAD by the events
    // check, which is the proof that `--files DIR` was consumed and `--events` still parsed
    // as the next flag. File requests are not events, so they add no log records.
    let log = d.join("empty.log");
    std::fs::write(&log, b"").unwrap();
    let r = run(&[
        os("--files"),
        d.join("root").as_os_str(),
        os("--events"),
        log.as_os_str(),
        e.as_os_str(),
    ]);
    assert_eq!(r.status(), None);
    assert!(r.stderr.contains("no event interface"), "{}", r.stderr);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn files_after_the_image_belongs_to_the_program_and_grants_nothing() {
    let d = scratch("late");
    std::fs::write(d.join("root/कपत्रम्"), b"abc\n").unwrap();
    let e = elf(&d, "r.elf", READS, "पठकः");
    let r = run(&[e.as_os_str(), os("--files"), d.join("root").as_os_str()]);
    assert_ne!(r.status(), Some(SUM), "a late --files granted a root");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn help_documents_the_flag() {
    let r = run(&[]);
    assert!(r.stderr.contains("--files DIR"), "{}", r.stderr);
}

// ── v1.0.1 follow-up: binary-level cases for the three mutations the library tests catch ──
//
// ASCII is outside the source repertoire, so a path with `/` or `.` is built by writing its
// ASCII octets over the front of a filler of `क` (three octets each); the last letter stays.

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
    while a.len() % 3 != 0 {
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

#[test]
fn an_absolute_path_naming_a_file_inside_the_root_is_refused_by_the_binary() {
    let d = scratch("abs");
    let root = d.join("root").canonicalize().unwrap();
    std::fs::write(root.join("क"), b"abc\n").unwrap();
    let abs = format!("{}/", root.to_str().unwrap());
    let src = with_ascii_path(READS, "उक्तम् कपत्रम् इति ।", &abs);
    let e = elf(&d, "abs.elf", &src, "पठकः");
    let r = run(&[os("--files"), root.as_os_str(), e.as_os_str()]);
    let s = r.status().expect("it finished");
    assert_ne!(s, SUM, "an absolute path was served");
    assert_eq!(s, yantra::patra::Status::Refused.word() & 0xFFFF_FFFF_FFFF);
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_write_through_a_symlink_in_the_root_leaves_the_outside_file_alone() {
    let d = scratch("wl");
    std::fs::write(d.join("outside"), b"KEEP").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(d.join("outside"), d.join("root/खपत्रम्")).unwrap();
    #[cfg(not(unix))]
    return;
    let e = elf(&d, "w.elf", ROUND_TRIP, "लेखकः");
    let r = run(&[os("--files"), d.join("root").as_os_str(), e.as_os_str()]);
    assert_ne!(r.status(), Some(100), "stderr: {}", r.stderr);
    assert_eq!(std::fs::read(d.join("outside")).unwrap(), b"KEEP");
    std::fs::remove_dir_all(&d).ok();
}

#[test]
fn a_write_naming_dotdot_creates_nothing_outside_the_root() {
    let d = scratch("wd");
    let src = with_ascii_path(ROUND_TRIP, "उक्तम् खपत्रम् इति ।", "../");
    let e = elf(&d, "w.elf", &src, "लेखकः");
    let r = run(&[os("--files"), d.join("root").as_os_str(), e.as_os_str()]);
    assert_ne!(r.status(), Some(100), "stderr: {}", r.stderr);
    assert!(!d.join("क").exists(), "a file was created outside the root");
    assert!(!d.join("root/क").exists());
    std::fs::remove_dir_all(&d).ok();
}
