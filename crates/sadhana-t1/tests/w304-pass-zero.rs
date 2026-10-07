//! **W-304 — ONE GATED SOURCE ENTRY, AND BOTH SOURCE DRIVERS GO THROUGH IT.**
//!
//! # What this file settles
//!
//! `उक्तम् SAS इति` is refused by the Rust front end (`lex_t1`) and was compiled
//! by the `.t1` chain. The repertoire walk is `अक्षरकोशॱपरिधिदोषः` (graded in
//! `w304-pass-zero-scan.rs` and `sadhana/tests/t1_repertoire_twin.rs`); this
//! file is about WHERE it runs.
//!
//! Until 2026-10-01 it ran only inside `शृङ्खलाॱपठनम्`. `chain.rs`'s `Front` —
//! `t1_build`, `frontend` and the yantra drivers — never reaches `पठनम्`:
//! `Front::lex` called `पदविभागॱपदविभाग` directly. Measured on main before this
//! change (the W-304 replay's probe B1): `Front` lexed `उक्तम् SAS इति` to 24
//! tokens and parsed ONE declaration with no error, while `पठनम्` refused it.
//!
//! A peer session's placement ruling (62a94a9f): a NEW routine,
//! `अक्षरकोशॱपरिधिपदविभाग`, runs the gate on the source octets and then
//! delegates to `पदविभागॱपदविभाग`; `Front::lex` names it (a call-site rename,
//! no gate logic in Rust) and `पठनम्` delegates to it, so there is ONE gate.
//! `पदविभाग` itself stays UNGATED, because `वाक्यविभागॱसङ्कलनम्` lexes the
//! emitted `.sas` text through it.
//!
//! # Provenance
//!
//! The cases are `agent/tick`'s (rescue ref
//! `rescue/w304-tick-2026-10-01e-99606658`, `w304-pass-zero.rs`), re-pointed at
//! the ruled entry. Its offsets were ONE-BASED (`सङ्ग्रहविरामः`, ० for clean);
//! here the record is `परिधिदोषस्थितम्` (बूल) beside `परिधिदोषस्थानम्` (a
//! ZERO-BASED `न६४`), so every `+ 1` the tick wrote is gone and the "clean"
//! answer is the बूल, not a zero position. Its code-point sweeps are not carried:
//! `t1_repertoire_twin.rs` grades the same predicate over 1,116 code points.

use sadhana::lex::lex_t1;
use sadhana::t1::chain::{self, CHAIN, Front};
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn spec_root() -> PathBuf {
    repo_root().join("spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// One statement inside a whole module — the W-304 replay's probe wrapper, so
/// the sources here are the sources that were measured on main and on the tick.
fn module(line: &str) -> String {
    format!(
        "मण्डलम् परीक्षणम् ॥\n\nसार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n{line}\n    \
         प्रत्यागमनम् २१ ।\nइति\n"
    )
}

/// The probes that must be REFUSED, each on the repertoire, each at the first
/// `S`. B2 is the hole the string-blind walk had: the `॰` inside the literal
/// used to cut `SAS` off as margin and the source scanned CLEAN.
const REFUSED: &[(&str, &str)] = &[
    (
        "B1 SAS in a literal",
        "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् SAS इति ।",
    ),
    (
        "B2 a ॰ then SAS in a literal",
        "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् ॰ SAS इति ।",
    ),
    (
        "B4 a Latin letter glued into a word of a literal",
        "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् कSक इति ।",
    ),
    (
        "B5 SAS after a literal इति pair",
        "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् इति इति SAS इति ।",
    ),
];

/// The probes that must be ACCEPTED: the control and ASCII in a margin, in the
/// three positions a margin can take. A gate that refused everything would make
/// every refusal above green on its own.
const ACCEPTED: &[(&str, &str)] = &[
    ("P0 the control", "    चरः प ॱॱ न६४ भवति ० ।"),
    (
        "A1 a margin on its own line",
        "    ॰ ASCII margin (x+y) = 3;",
    ),
    (
        "A2 a trailing margin",
        "    चरः प ॱॱ न६४ भवति ० ।   ॰ trailing ASCII x.y",
    ),
    ("A3 a margin glued to a word", "    चरः प ॱॱ न६४ भवति ० ।॰x"),
    (
        "A5 a margin naming उक्तम्",
        "    चरः प ॱॱ न६४ भवति ० ।  ॰ see उक्तम् SAS here",
    ),
];

fn octets(s: &str) -> Value {
    Value::Octets(Octets::new(s.as_bytes()))
}

/// `अक्षरकोशॱपरिधिपदविभाग` — the gated entry — on one source: tokens written.
fn gated_lex(it: &mut Interpreter, src: &str) -> i128 {
    it.call("अक्षरकोशॱपरिधिपदविभाग", vec![octets(src)], 4_000_000_000)
        .unwrap_or_else(|e| panic!("परिधिपदविभाग refused to run: {e:?}"))
        .as_int()
        .expect("परिधिपदविभाग answers an अङ्क")
}

/// `शृङ्खलाॱपठनम्` on one source — declarations read, `0` for a refusal.
fn read(it: &mut Interpreter, src: &str) -> i128 {
    it.call("शृङ्खलाॱपठनम्", vec![octets(src)], 4_000_000_000)
        .unwrap_or_else(|e| panic!("शृङ्खलाॱपठनम् refused to run: {e:?}"))
        .as_int()
        .expect("पठनम् answers an अङ्क")
}

/// The gate's record: `Some(offset)` when it refused, `None` when it did not.
/// Read through `global`, which answers `None` for a name no module declared, so
/// a misspelt name panics here instead of reading as "clean".
fn record(it: &Interpreter) -> Option<usize> {
    let stopped = match it.global("परिधिदोषस्थितम्") {
        Some(Value::Bool(b)) => *b,
        other => panic!("अक्षरकोश declares परिधिदोषस्थितम् as a बूल; read {other:?}"),
    };
    let at = it
        .global("परिधिदोषस्थानम्")
        .and_then(Value::as_int)
        .expect("अक्षरकोश declares परिधिदोषस्थानम्");
    stopped.then(|| usize::try_from(at).expect("an offset is non-negative"))
}

/// **`Front` — the driver that never reaches `पठनम्` — REFUSES.** Through
/// `Front::lex` itself, not a replica of it, so repointing that call back at
/// `पदविभागॱपदविभाग` reds this test (main answered 24 tokens for B1).
#[test]
fn front_lex_refuses_every_probe_the_rust_lexer_refuses() {
    for (what, line) in REFUSED {
        let src = module(line);
        let rust = format!("{:?}", lex_t1(&src).expect_err("lex_t1 refuses it"));
        assert!(
            rust.contains("outside the doc 15 repertoire"),
            "{what}: the premise — the Rust lexer refuses it ON THE REPERTOIRE: {rust}"
        );
        let mut front = Front::load(&spec_root()).expect("the front end loads");
        assert_eq!(
            front.lex(&src),
            Ok(0),
            "{what}: Front::lex must refuse it before a token is written"
        );
    }
    for (what, line) in ACCEPTED {
        let src = module(line);
        assert!(
            lex_t1(&src).is_ok(),
            "{what}: the premise — lex_t1 accepts it"
        );
        let mut front = Front::load(&spec_root()).expect("the front end loads");
        let tokens = front.lex(&src).expect("Front::lex runs");
        assert!(
            tokens > 0,
            "{what}: Front::lex refused a source the Rust lexer accepts"
        );
        assert_eq!(
            front.parse(),
            Ok(1),
            "{what}: and it parses to its one declaration"
        );
    }
}

/// **BOTH DRIVERS GO THROUGH ONE GATE, AND THE RECORD NAMES THE OCTET.** The
/// gated entry and `पठनम्` answer the same zero-based offset — Rust's own
/// `find('S')`, never a constant — and `chain::refusal_site` renders it. Every
/// refusal is followed by a clean source on the same interpreter, the only
/// order that can catch a record that is latched rather than reset.
#[test]
fn the_gated_entry_and_pathanam_report_the_same_octet_and_reset() {
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the whole chain loads");
    let clean = module(ACCEPTED[0].1);
    for (what, line) in REFUSED {
        let src = module(line);
        let want = src.find('S').expect("each probe holds an S");

        assert_eq!(gated_lex(&mut it, &src), 0, "{what}: the entry refuses");
        assert_eq!(
            record(&it),
            Some(want),
            "{what}: at the first S, zero-based"
        );

        assert!(
            gated_lex(&mut it, &clean) > 0,
            "{what}: then the control lexes"
        );
        assert_eq!(record(&it), None, "{what}: and the record was RESET");

        assert_eq!(read(&mut it, &src), 0, "{what}: पठनम् refuses it too");
        assert_eq!(record(&it), Some(want), "{what}: through the same gate");
        assert_eq!(
            chain::refusal_site(&it),
            Some(format!("repertoire: octet {want} is outside R-15-1")),
            "{what}: and the refusal says why"
        );
    }
    for (what, line) in ACCEPTED {
        let src = module(line);
        assert_eq!(read(&mut it, &src), 1, "{what}: पठनम् reads one declaration");
        assert_eq!(record(&it), None, "{what}: and the gate recorded nothing");
    }
}

/// **AN UNCLOSED `उक्तम्` IS NOT A REPERTOIRE REFUSAL — the third state.**
/// Probe B3 carries ASCII INSIDE the unclosed literal; the string-blind walk on
/// main refused it as the repertoire at the `S` (octet 259), a plausible number
/// for the wrong cause. `lex_t1` refuses the LINE at the `उक्तम्` and never
/// checks a word of it, so the gate must let it through to the lexer and the
/// parser, which refuse it by structure.
#[test]
fn an_unclosed_literal_passes_the_gate_and_the_parser_refuses_it() {
    for (what, line) in [
        (
            "B3 an unclosed literal holding SAS",
            "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् SAS",
        ),
        (
            "B6 an unclosed literal, all Devanagari",
            "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् क",
        ),
    ] {
        let src = module(line);
        let rust = format!("{:?}", lex_t1(&src).expect_err("lex_t1 refuses it"));
        assert!(
            rust.contains("closes on this line") && !rust.contains("repertoire"),
            "{what}: the premise — Rust refuses on STRUCTURE only: {rust}"
        );

        let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the whole chain loads");
        assert!(
            gated_lex(&mut it, &src) > 0,
            "{what}: the gate lets it through"
        );
        assert_eq!(
            record(&it),
            None,
            "{what}: and records no repertoire refusal"
        );
        assert_eq!(
            it.global("परिधिदोषकारणम्").and_then(Value::as_int),
            it.global("परिधिउक्तदोषः").and_then(Value::as_int),
            "{what}: the walk named the unclosed literal as its cause"
        );

        let mut front = Front::load(&spec_root()).expect("the front end loads");
        assert!(
            front.lex(&src).expect("Front::lex runs") > 0,
            "{what}: Front lexes it"
        );
        assert!(
            front.parse().is_err(),
            "{what}: and the parser refuses it — the structure verdict the gate defers to"
        );
    }
}

/// `source` with every `॰` margin cut — unless the line opens a string before
/// it, where `॰` is data. Line-local and conservative: it errs by leaving text
/// IN, which can only red an emptiness assertion, never green it falsely.
fn code_only(text: &str) -> String {
    text.lines()
        .map(|line| match line.find('॰') {
            Some(at) if !line[..at].contains("उक्तम्") => &line[..at],
            _ => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// **THE `.sas` LEXER IS NOT GATED, ON PURPOSE (a peer session's ruling).**
/// `वाक्यविभागॱसङ्कलनम्` lexes EMITTED text through `पदविभागॱवाक्यपदविभाग` (the
/// T0 entry since W-366 (b); `पदविभाग`, the T1 entry, shares its walk), and a
/// gate there would walk every module's assembly a second time and report an
/// offset into text no author wrote. So the T0 entry lexes octets the gate refuses,
/// and it leaves the gate's record alone. The `.sas` path keeps its own guard in
/// the assembler (`one_latin_letter_in_the_emitted_text_is_refused_by_the_
/// assembler_with_its_line`).
#[test]
fn the_sas_lexer_stays_ungated_and_leaves_the_record_alone() {
    let code = code_only(&source("vakyavibhaga.t1"));
    assert!(
        code.contains("पदविभागॱवाक्यपदविभाग मूल"),
        "the premise: वाक्यविभाग lexes .sas through पदविभागॱवाक्यपदविभाग, the T0 entry"
    );
    assert!(
        !code.contains("परिधिपदविभाग"),
        "वाक्यविभाग must not lex emitted .sas through the gated source entry"
    );
    // The control for the cut: the same file uncut DOES name पदविभाग, and a cut
    // that dropped every line would pass the line above vacuously.
    assert!(
        code.lines().count() > 1000,
        "the cut kept the code of vakyavibhaga.t1"
    );

    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the whole chain loads");
    let refused = module(REFUSED[0].1);
    assert_eq!(
        gated_lex(&mut it, &refused),
        0,
        "the gated entry refuses it"
    );
    let at = record(&it);
    assert!(at.is_some(), "and records where");

    // Emitted-text shape: a line the gate would refuse, straight into the T0 entry.
    let emitted = "    योगः कम् X ।\n    उक्तम् SAS इति\n";
    let tokens = it
        .call("पदविभागॱवाक्यपदविभाग", vec![octets(emitted)], 4_000_000_000)
        .expect("पदविभाग runs")
        .as_int()
        .expect("an अङ्क");
    assert!(
        tokens > 0,
        "पदविभाग itself must lex text holding a byte the gate refuses — it answered \
         {tokens}, so the .sas path is gated"
    );
    assert_eq!(
        record(&it),
        at,
        "and पदविभाग must not touch the gate's record — it belongs to the source entry"
    );
}
