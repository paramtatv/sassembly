//! ॥ EVERY `.t1` FILE IN THE TREE IS ACCOUNTED FOR, AND SOME ARE COUNTED BY
//! NOTHING ॥ `W-279`, rowless.
//!
//! Three instruments count `.t1` files and each counts a different subset. The
//! difference has never been asserted, so "the corpus" means whichever
//! population the speaker last looked at:
//!
//! | population | what it is | what it misses |
//! |---|---|---|
//! | `T1_SOURCES` = 21 | `crates/sadhana-t1/src/` — the self-hosting corpus | 14 |
//! | `every_t1_source()` = 26 | walks `crates/` | the 9 under `spec/` |
//! | the tree = 35 | every `.t1` under the repo root | — |
//!
//! **The nine programs under `spec/` are outside EVERY instrument**, including
//! `t1_modules.rs`'s lexer-refusal ratchet, whose glob stops at `crates/`. That
//! ratchet's own margin says *"a T1 source the T1 lexer cannot read cannot
//! reach ELF through T1"* and *"every removal is progress and every addition is
//! a defect"* — a rule it cannot apply to the six files that ARE the
//! demonstration, nor to the three in `spec/rung/` that ARE the cross-module
//! measurement.
//!
//! The three figures in that table are LIVE and have moved since this file was
//! written: the corpus was 20, `crates/` 25 and the tree 31 on 2026-09-08. The
//! paragraph below quotes the 20 and 25 as they stood then, because that
//! mismatch is what the finding was.
//!
//! This was found by being handed "25, not 20" and counting. **25 is
//! `every_t1_source()`'s population reported as the tree's** — the same shape
//! as a scan reporting "who calls this" while computing "who calls this in
//! `src`", and it would have been inherited had it not been checked.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the crate sits two levels under the repo root")
        .to_path_buf()
}

/// Every `.t1` under `dir`, repo-relative and sorted. Build output and `.git`
/// are skipped — a stray `.t1` copied into a target directory is not corpus and
/// would make this count the machine's leavings.
fn t1_under(dir: &Path) -> Vec<String> {
    fn walk(d: &Path, root: &Path, out: &mut Vec<String>) {
        let Ok(entries) = std::fs::read_dir(d) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            let name = e.file_name();
            let name = name.to_string_lossy();
            if p.is_dir() {
                if name.starts_with('.') || name == "target" {
                    continue;
                }
                walk(&p, root, out);
            } else if p.extension().is_some_and(|x| x == "t1") {
                out.push(
                    p.strip_prefix(root)
                        .unwrap_or(&p)
                        .to_string_lossy()
                        .into_owned(),
                );
            }
        }
    }
    let root = repo_root();
    let mut v = Vec::new();
    walk(dir, &root, &mut v);
    v.sort();
    v
}

/// `T1_SOURCES` as `crates/yantra/tests/paradigm/pins.rs` DECLARES it, read
/// from the source because this crate cannot link another crate's test module.
///
/// Declared against derived is the only comparison here that can drift: both
/// sides of a nested-path walk agree by construction, and a pin written by hand
/// does not.
fn t1_sources_pin() -> usize {
    let path = repo_root().join("crates/yantra/tests/paradigm/pins.rs");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
    let head = "pub const T1_SOURCES: usize = ";
    let at = text.find(head).unwrap_or_else(|| {
        panic!("pins.rs must declare T1_SOURCES — it has moved or been renamed")
    }) + head.len();
    let digits: String = text[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits
        .parse()
        .unwrap_or_else(|e| panic!("T1_SOURCES is not a number: {digits:?}: {e}"))
}

/// `.t1` files no instrument in this tree counts, with what each one is.
///
/// EXACT, not a ceiling — a tenth would slip past a `<=`, and the whole
/// finding is that a population can grow where nothing is looking. It DID grow:
/// the six demonstration programs were the whole list until `spec/rung/`
/// arrived with the cross-module pair, and this list going red is how that
/// arrival was noticed rather than inherited.
const COUNTED_BY_NOTHING: &[(&str, &str)] = &[
    (
        "spec/demo/क्रमगुणितम्.t1",
        "factorial — a demonstration program",
    ),
    ("spec/demo/द्विगुणम्.t1", "double — a demonstration program"),
    ("spec/demo/नमस्कारः.t1", "hello — a demonstration program"),
    (
        "spec/demo/महत्तमम्.t1",
        "greatest common divisor — a demonstration program",
    ),
    (
        "spec/demo/योगपर्यन्तम्.t1",
        "sum to n — a demonstration program",
    ),
    (
        "spec/demo/समविषमम्.t1",
        "even or odd — a demonstration program",
    ),
    // ॥ `examples/` — probes a user runs with the RELEASED stage1.elf (v1.0.1) ॥ They are
    // compiled by `examples/limits/run.sh` and `examples/networking/run.sh`, not by any
    // instrument of this crate, so none counts them.
    (
        "examples/limits/args.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/chk_add.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/chk_mul.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/chk_shl.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/chk_sub.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/counter.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/echo.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/file_read.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/file_write.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/hello.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p12.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p14.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p15.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p16.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p17.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p18.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p19.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p20.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p21.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p22.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/int_p23.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/mem_grow_256m.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/mem_grow_384m.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/mem_runs_500k.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/mem_runs_540k.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/thr2.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/thr64.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/thr65.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/wait0.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/wait3.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/wart_module.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/limits/wart_routine.t1",
        "limits probe run by examples/limits/run.sh",
    ),
    (
        "examples/networking/echo.t1",
        "networking example run by examples/networking/run.sh",
    ),
    // ॥ `spec/rung/` — W-279's cross-module measurement, and why it is not in
    // `spec/demo/` ॥ The demonstration's driver links ONE module object beside
    // the startup (`frontend/src/lib.rs:1079`), so a program spanning two
    // modules cannot answer its claim there; and the six above are pinned by
    // name in this very file. These three are reached by an explicit source
    // list instead, and nothing walks `spec/` for `.t1` — the two `read_dir`s
    // over `spec/` (`yantra/tests/application.rs:210`,
    // `metrics/src/main.rs:1585`) both filter on `.sas`.
    (
        "spec/rung/निजम्.t1",
        "the CALL CONTROL — the pair's arithmetic written inline, no module boundary crossed",
    ),
    (
        "spec/rung/पारम्.t1",
        "the CALLEE half of the cross-module pair",
    ),
    (
        "spec/rung/सेतुः.t1",
        "the CALLER half — one cross-module call and nothing else; carries the four runs in its margin",
    ),
    // ॥ `spec/net/` — the network library in .t1 (ADR-0048), a LIBRARY beside
    // `spec/darshaka.t1` and NOT in the compiler's corpus; built with t1_image by
    // `crates/yantra/tests/net_t1.rs` and `tools/check-net-t1.sh` ॥
    (
        "spec/net/sanchara-pankti.t1",
        "the virtio-net DRIVER (सञ्चारपङ्क्तिः) — ADR-0048; device intrinsics, native only",
    ),
    (
        "spec/net/sambandha.t1",
        "the Ethernet layer (सम्बन्धः) and the big-endian helpers — ADR-0048",
    ),
    (
        "spec/net/sthana-nirdesha.t1",
        "the ARP layer (स्थाननिर्देशः) — ADR-0048",
    ),
    (
        "spec/net/marga.t1",
        "the IPv4 layer (मार्गः) and the Internet checksum — ADR-0048",
    ),
    (
        "spec/net/marga-sandesha.t1",
        "the ICMP echo layer (मार्गसन्देशः) — ADR-0048",
    ),
    (
        "spec/net/nihsandhi-pinda.t1",
        "the UDP layer (निःसन्धिपिण्डः) — datagram, pseudo-header checksum, allow-list — ADR-0048",
    ),
    (
        "spec/net/nama-nirdesha.t1",
        "the DNS layer (नामनिर्देशः) — A query, response parse with bounded compression pointers — ADR-0048",
    ),
    (
        "spec/net/sanchara.t1",
        "the family module (सञ्चार) and the eight ruled members — ADR-0048",
    ),
    (
        "spec/net/net-gpu.t1",
        "a GPU draw then a ping in one run (सञ्चारचित्रपरीक्षा) — the two drivers' regions must not meet; built by net_t1.rs",
    ),
    (
        "spec/net/net-demo.t1",
        "the library's self-test, pinger and responder program (सञ्चारपरीक्षा) — built by crates/yantra/tests/net_t1.rs",
    ),
    (
        "spec/net/net-udp-demo.t1",
        "the UDP and DNS self-test, echo pair and DNS pair program (सन्देशपरीक्षा) — built by crates/yantra/tests/net_t1.rs and tools/check-net-t1.sh",
    ),
    // ॥ `spec/virtio-gpu.t1` — the C-009 `.t1` port (v0.5.0, "the GPU-driver project / the GPU driver"),
    // seam-free half: lays out C-009's controlq region and EMITS the octets;
    // discovery, init and notify wait on W-350's MMIO seam. Built and judged by
    // `crates/yantra/tests/virtio_gpu.rs` (octet equality against C-015's encode),
    // which reaches it by explicit path, not by walking `spec/`. ॥
    //
    // AND BEING BUILT BY A TEST IS NOT BEING COUNTED, which is the whole reason this
    // row belongs here rather than in some list of covered files: this list is
    // computed as `all − in_crates` (`:194`), so EVERY `.t1` outside `crates/` must
    // appear in it. All nine rows above are exercised too — `spec/rung/` IS W-279's
    // cross-module measurement and `spec/demo/` is run by the demonstration driver —
    // and they are listed all the same. "Counted" here means counted by a CENSUS
    // instrument (`T1_SOURCES`, `every_t1_source()`, the lexer-refusal ratchet), and
    // not one of the three walks `spec/`.
    (
        "spec/virtio-gpu.t1",
        "the C-009 virtio-gpu driver port, seam-free half",
    ),
    // ॥ `spec/darshaka.t1` and `spec/virtio-gpu-draw.t1` — row C-015, unified on
    // the GPU-driver project's v0.5.0 driver (owner, 2026-10-06) ॥ The driver and C-009's picture,
    // built and judged by `tools/check-virtio-gpu-t1.sh` on yantra's model and
    // QEMU's real virtio-gpu, which names both by explicit path. ॥
    (
        "spec/darshaka.t1",
        "C-015's virtio-gpu 2D driver, shared with the GPU-driver project's v0.5.0 render",
    ),
    (
        "spec/virtio-gpu-draw.t1",
        "C-015's picture — C-009's frame with one odd pixel, drawn by spec/darshaka.t1",
    ),
    // ॥ `spec/entry/` — W-302's native wrapper ॥ An ENTRY module built beside
    // the twenty-one corpus sources with `t1_image --entry`, so the self-hosted
    // image assembles a `.sas` from its input channel. Under `spec/` and not
    // `crates/` so that it does not enter the corpus walks. Built and judged by
    // `tools/check-w302-native.sh`, which names it by explicit path.
    (
        "spec/entry/assemble.t1",
        "W-302's native wrapper — the self-hosted image assembles a .sas read from its input channel",
    ),
];

#[test]
fn every_t1_file_in_the_tree_is_counted_by_some_instrument_or_recorded_here() {
    let root = repo_root();
    let all: BTreeSet<String> = t1_under(&root).into_iter().collect();
    let in_crates: BTreeSet<String> = t1_under(&root.join("crates")).into_iter().collect();
    let corpus: BTreeSet<String> = t1_under(&root.join("crates/sadhana-t1/src"))
        .into_iter()
        .collect();

    assert!(
        !all.is_empty() && corpus.len() >= 15,
        "the walk found {} files and {} corpus sources — it has stopped \
         reaching the tree and would pass over anything",
        all.len(),
        corpus.len()
    );

    // ॥ THE NESTING IS STRUCTURAL AND IS NOT ASSERTED, AND THE FIRST DRAFT
    // ASSERTED IT ॥
    //
    // `corpus ⊆ crates ⊆ tree` held `assert!(corpus.is_subset(&in_crates) &&
    // in_crates.is_subset(&all))` here. All three sets come from ONE walk over
    // NESTED PATHS — `crates/sadhana-t1/src` is under `crates`, which is under
    // the root — so the containment is guaranteed by the path expressions and
    // THE ASSERTION COULD NOT FAIL. A check that cannot go red reads exactly
    // like one that passed, which is the failure this whole file is about, and
    // it was written while writing about it.
    //
    // The counts are reported below instead. A metric that reports is honest;
    // an assertion that cannot fail is not.
    //
    // WHAT CAN DRIFT IS DERIVED AGAINST DECLARED, so that is what is checked:
    // the corpus walked off disk, against `T1_SOURCES` written by hand in
    // `crates/yantra/tests/paradigm/pins.rs`. Adding a `.t1` to the corpus
    // without moving the pin is a real event with a real failure.
    let pinned = t1_sources_pin();
    assert_eq!(
        corpus.len(),
        pinned,
        "`crates/sadhana-t1/src` holds {} `.t1` sources and \
         `paradigm/pins.rs` pins T1_SOURCES at {pinned}. One of them moved \
         without the other — the corpus is:\n  {}",
        corpus.len(),
        corpus.iter().cloned().collect::<Vec<_>>().join("\n  ")
    );

    let uncounted: BTreeSet<&String> = all.difference(&in_crates).collect();
    let recorded: BTreeSet<String> = COUNTED_BY_NOTHING
        .iter()
        .map(|(p, _)| (*p).to_string())
        .collect();

    let unrecorded: Vec<&&String> = uncounted
        .iter()
        .filter(|p| !recorded.contains(**p))
        .collect();
    assert!(
        unrecorded.is_empty(),
        "these `.t1` files are outside `crates/`, so NO instrument counts them — \
         not `T1_SOURCES`, not `every_t1_source()`, not the lexer-refusal \
         ratchet. Record each in COUNTED_BY_NOTHING with what it is, or bring \
         it inside an instrument's reach:\n  {}",
        unrecorded
            .iter()
            .map(|p| p.as_str())
            .collect::<Vec<_>>()
            .join("\n  ")
    );

    let gone: Vec<&str> = COUNTED_BY_NOTHING
        .iter()
        .map(|(p, _)| *p)
        .filter(|p| !all.contains(*p))
        .collect();
    assert!(
        gone.is_empty(),
        "COUNTED_BY_NOTHING records files that are no longer in the tree — drop \
         the rows:\n  {}",
        gone.join("\n  ")
    );

    println!("METRIC t1_files_in_tree {}", all.len());
    println!("METRIC t1_files_under_crates {}", in_crates.len());
    println!("METRIC t1_files_in_corpus {}", corpus.len());
    println!("METRIC t1_files_counted_by_nothing {}", uncounted.len());
}

/// The five `.t1` modules in `crates/textapp/src/text/`, and the module each
/// declares.
const TEXTAPP_MODULES: &[(&str, &str)] = &[
    ("ident.t1", "इडएनट"),
    ("nfc.t1", "एनएफसइ"),
    ("numeral.t1", "नउमएरअल"),
    ("segment.t1", "सएगमएनट"),
    ("tables.t1", "सारणी"),
];

/// ॥ NOTHING IN THE CORPUS IMPORTS THE `textapp` MODULES ॥
///
/// **This is the criterion, and it is the tree's own.** `W-149/D-002e`
/// (2026-08-30) repaired `kosha.t1` and `vastu.t1`, which arrived in the same
/// fork restoration with English transcribed letter by letter — `मण्डलम् कओश`.
/// Its reasoning:
///
/// > The callers had been writing proper Sanskrit the whole time … Only the
/// > definitions were transcribed, so every name here is **RECOVERED FROM THE
/// > CALL SITES** rather than chosen.
///
/// `kosha` was repaired because `samyojana.t1:28` and `encode.t1:4` imported
/// `कोश` and got nothing — a real caller demanding a real module. **The five
/// here have no callers at all, so there is no call site to recover a name
/// from.** That is what separates them from `kosha`, and transliteration alone
/// plainly does not: `kosha.t1` was transliterated and is now 216 lines of the
/// self-hosting corpus.
///
/// THIS TEST IS A TRIPWIRE, NOT A VERDICT. If any of these ever gains an
/// importer it has become required, and the classification in this margin is
/// wrong from that moment. Three of them also import `सअरणई` — independent
/// vowels `स-अ-र-ण-ई` where `tables.t1` declares `सारणी` with vowel signs, a
/// different string that nothing declares — recorded already in
/// `t1_modules.rs`'s `IMPORTS_NAMING_NO_DECLARED_MODULE`.
#[test]
fn no_corpus_source_imports_a_textapp_module() {
    let root = repo_root();
    let corpus = t1_under(&root.join("crates/sadhana-t1/src"));
    assert!(
        corpus.len() >= 15,
        "only {} corpus sources found; this tripwire would pass over anything",
        corpus.len()
    );

    let mut importers: Vec<String> = Vec::new();
    for rel in &corpus {
        let Ok(text) = std::fs::read_to_string(root.join(rel)) else {
            continue;
        };
        for (line_no, line) in text.lines().enumerate() {
            // A margin quoting an import is not an import. `॰` opens a comment
            // and `samyojana.t1:60` discusses `वास्तुॱपाठ्यम्` in prose.
            let code = line.split('॰').next().unwrap_or(line);
            for (file, module) in TEXTAPP_MODULES {
                if code.contains(&format!("आयातः {module}")) {
                    importers.push(format!("{rel}:{} imports `{module}` ({file})", line_no + 1));
                }
            }
        }
    }
    assert!(
        importers.is_empty(),
        "a corpus source now imports a `crates/textapp/src/text/` module, so it \
         is REQUIRED and the scaffolding classification above is wrong from this \
         moment — `W-149`'s criterion is whether callers exist:\n  {}",
        importers.join("\n  ")
    );
}
