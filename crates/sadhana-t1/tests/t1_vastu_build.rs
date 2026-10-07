//! **BUILDING AN OBJECT'S SYMBOLS IN `.t1`** — the half of `kosha::object`
//! (kosha.rs:209) that turns a program's labels into symbols a linker can read.
//!
//! `संयोजनम्` is a complete whole-program linker in this corpus and had no `.t1`
//! caller, because nothing in `.t1` could make its argument. This is the first
//! half of the thing that can.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// `CHAIN` is the front end plus the emitter; it carries neither `kosha.t1`
/// nor `samyojana.t1`, so `संयोजन` is not loaded by it. The mirror check below
/// needs the LINKER, which reads `spec/relocations-riscv64.tsv` at run time —
/// so this adds the two. When the walk lands they belong in `CHAIN` itself.
fn with_linker() -> Vec<(String, String)> {
    let mut names: Vec<String> = CHAIN.iter().map(|(n, _)| (*n).to_string()).collect();
    for extra in ["kosha.t1", "samyojana.t1"] {
        if !names.iter().any(|n| n == extra) {
            names.push(extra.to_string());
        }
    }
    names
        .into_iter()
        .map(|n| {
            let t =
                std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(&n))
                    .unwrap_or_else(|e| panic!("{n}: {e}"));
            (n, t)
        })
        .collect()
}

fn assembler() -> Interpreter {
    let texts = with_linker();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    let mut it = Interpreter::load(&refs, &spec_root()).expect("the chain loads");
    for (r, fuel) in [
        ("वाक्यविभागॱआरम्भः", 200_000_000u64),
        ("वाक्यविभागॱसंज्ञाकुलपठनम्", 2_000_000_000),
        ("वाक्यविभागॱनिर्देशकोशपठनम्", 2_000_000_000),
    ] {
        it.call(r, vec![], fuel)
            .unwrap_or_else(|e| panic!("{r}: {e:?}"));
    }
    it
}

fn field(v: &Value, name: &str) -> Value {
    match v {
        Value::Record(r) => r
            .borrow()
            .get(name)
            .cloned()
            .unwrap_or_else(|| panic!("no field `{name}`")),
        other => panic!("not a record: {other:?}"),
    }
}

fn text(v: &Value) -> String {
    match v {
        Value::Octets(o) => String::from_utf8_lossy(o.as_slice()).into_owned(),
        other => panic!("not octets: {other:?}"),
    }
}

/// **TWO LABELS, ONE GLOBAL, AND THEIR ADDRESSES MUST DIFFER.**
///
/// Every assertion here is one a plausible wrong builder passes without:
/// `B-069d2b` put every symbol at ० — right only for a label that happens to be
/// first — and a builder that ignores `॥ वैश्विकम् ॥` marks everything local,
/// which links inside one object and fails only across two.
#[test]
fn the_programs_labels_become_symbols_with_their_own_addresses() {
    let mut it = assembler();
    let src = concat!(
        "॥ वैश्विकम् मुख्यम् ॥\n",
        "मुख्यम्ॱॱ\n",
        "योगः अर्थ०म् शून्यःन १न ।\n",
        "द्वितीयम्ॱॱ\n",
        "योगः अर्थ०म् शून्यःन २न ।\n",
    );
    let n = it
        .call(
            "वाक्यविभागॱसङ्कलनम्",
            vec![Value::Octets(Octets::new(src.as_bytes()))],
            8_000_000_000,
        )
        .expect("सङ्कलनम्")
        .as_int()
        .unwrap_or(0);
    assert!(n > 0, "the assembler read no statements from:\n{src}");

    let program = it
        .call("वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000)
        .expect("कार्यक्रमरचना");
    let target = it
        .global("सङ्केतनॱअसङ्कुचितम्")
        .cloned()
        .unwrap_or(Value::Int(0));
    let places = it
        .call(
            "सङ्केतनॱस्थानविन्यासः",
            vec![program.clone(), target],
            2_000_000_000,
        )
        .expect("स्थानविन्यासः");
    let symbols = it
        .call("सङ्केतनॱवस्तुसंज्ञासङ्ग्रहः", vec![program, places], 2_000_000_000)
        .expect("वस्तुसंज्ञासङ्ग्रहः");

    let rows = match &symbols {
        Value::Arena(a) => a.borrow().clone(),
        other => panic!("वस्तुसंज्ञासङ्ग्रहः answered {other:?}, not an arena"),
    };
    let live: Vec<&Value> = rows
        .iter()
        .filter(|v| matches!(v, Value::Record(_)))
        .collect();
    assert_eq!(
        live.len(),
        2,
        "two labels, two symbols; got {} from {rows:?}",
        live.len()
    );

    let names: Vec<String> = live.iter().map(|v| text(&field(v, "नाम"))).collect();
    assert_eq!(names, vec!["मुख्यम्", "द्वितीयम्"], "in written order");

    // THE GLOBAL FLAG COMES FROM `॥ वैश्विकम् ॥` AND NOT FROM BEING FIRST.
    let globals: Vec<bool> = live
        .iter()
        .map(|v| matches!(field(v, "वैश्विकम्"), Value::Bool(true)))
        .collect();
    assert_eq!(
        globals,
        vec![true, false],
        "`मुख्यम्` is pushed global and `द्वितीयम्` is not"
    );

    // BOTH ARE IN .text, AND THEIR ADDRESSES DIFFER — the `B-069d2b` assertion.
    let places: Vec<i128> = live
        .iter()
        .map(|v| field(v, "स्थापनम्").as_int().unwrap_or(-1))
        .collect();
    assert_eq!(
        places,
        vec![1, 1],
        "both labels are in ॱपाठ (Placement::Text)"
    );

    let values: Vec<i128> = live
        .iter()
        .map(|v| field(v, "मूल्यम्").as_int().unwrap_or(-1))
        .collect();
    assert_eq!(values[0], 0, "the first label stands at the object's base");
    assert!(
        values[1] > values[0],
        "the second label must stand AFTER the first: got {values:?} — every \
         symbol at ० is `B-069d2b`, which links cleanly and branches wrong"
    );
    println!("METRIC t1_object_symbols {} values {values:?}", live.len());
}

/// **THE MIRRORED RELOCATION NUMBERS EQUAL WHAT THE SPEC SAYS.**
///
/// `सङ्केतन` cannot import `संयोजन` — that is the cycle — so it mirrors the four
/// relocation types as integers, exactly as `संयोजन` mirrors `स्थापन` and
/// `RelSection` and for the stated reason. **A mirror nothing checks is a
/// hardcode that drifts in silence**, and `spec/relocations-riscv64.tsv` is
/// DERIVED — regenerated from what `riscv64-elf-as` emits — so it can move
/// without anyone editing this tree by hand.
///
/// `संयोजनॱभेदाङ्कः` reads that file at run time. This asserts the constants
/// against it, which is the only thing that makes them a mirror rather than a
/// guess.
#[test]
fn the_mirrored_relocation_numbers_equal_the_spec() {
    let mut it = assembler();
    for (konstant, name) in [
        ("प्रतीक्षादत्तभेदः", "R_RISCV_64"),
        ("प्रतीक्षाजालभेदः", "R_RISCV_JAL"),
        ("प्रतीक्षोपरिभेदः", "R_RISCV_PCREL_HI20"),
        ("प्रतीक्षाधोभेदः", "R_RISCV_PCREL_LO12_I"),
    ] {
        let mirrored = it
            .global(konstant)
            .and_then(Value::as_int)
            .unwrap_or_else(|| panic!("no constant `{konstant}`"));
        let from_spec = it
            .call(
                "संयोजनॱभेदाङ्कः",
                vec![Value::Octets(Octets::new(name.as_bytes()))],
                200_000_000,
            )
            .unwrap_or_else(|e| panic!("भेदाङ्कः {name}: {e:?}"))
            .as_int()
            .unwrap_or_else(|| {
                panic!("`{name}` is not in spec/relocations-riscv64.tsv — the mirror names a type the spec does not")
            });
        assert_eq!(
            mirrored, from_spec,
            "{konstant} mirrors {name}: this file says {mirrored}, the spec says {from_spec}"
        );
    }
}

/// **AN UNDEFINED JUMP TARGET IS RECORDED WITH `R_RISCV_JAL`, NOT MERELY
/// RECORDED.** Before this row the arena held a name and an offset, so every
/// pending looked alike and a linker had no rule to patch by.
#[test]
fn an_undefined_name_records_which_relocation_patches_it() {
    let mut it = assembler();
    // `लङ्घनम् <dest>म् <target>य् ।` — the `य्` sigil is the jump target,
    // which is the `भूमिका असमम् ४` guard the pending site sits behind.
    let src = "लङ्घनम् शून्यःम् अन्यत्रम्य् ।\n";
    let n = it
        .call(
            "वाक्यविभागॱसङ्कलनम्",
            vec![Value::Octets(Octets::new(src.as_bytes()))],
            8_000_000_000,
        )
        .expect("सङ्कलनम्")
        .as_int()
        .unwrap_or(0);
    assert!(n > 0, "the assembler read no statements from:\n{src}");
    let program = it
        .call("वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000)
        .expect("कार्यक्रमरचना");
    // `वस्तुसङ्केतनम्` sets प्रतीक्षाग्रहणम्, so an undefined name is RECORDED
    // rather than refused — that is the whole difference between an object and
    // an executable.
    let _ = it.call("सङ्केतनॱवस्तुसङ्केतनम्", vec![program], 2_000_000_000);
    let n = it
        .global("प्रतीक्षासूचकाङ्क")
        .and_then(Value::as_int)
        .unwrap_or(0);
    assert!(
        n > 0,
        "no pending was recorded for `अन्यत्रम्`, which nothing defines.\n  \
         अन्तिमसङ्केतनदोषः: {:?}\n  प्रतीक्षाग्रहणम्: {:?}\n  आज्ञासूचकाङ्क: {:?}",
        it.global("अन्तिमसङ्केतनदोषः"),
        it.global("प्रतीक्षाग्रहणम्"),
        it.global("आज्ञासूचकाङ्क"),
    );
    let rows = match it.global("प्रतीक्षाकोश") {
        Some(Value::Arena(a)) => a.borrow().clone(),
        other => panic!("प्रतीक्षाकोश is an arena, not {other:?}"),
    };
    // ONE-BASED: the appender advances the cursor and then writes at it.
    let first = &rows[1];
    let kind = field(first, "भेदः").as_int().unwrap_or(-1);
    let jal = it
        .global("प्रतीक्षाजालभेदः")
        .and_then(Value::as_int)
        .unwrap_or(-2);
    assert_eq!(
        kind, jal,
        "a jump to an undefined name is R_RISCV_JAL; got {kind}"
    );
    assert_eq!(
        field(first, "कोष्ठकम्").as_int().unwrap_or(-1),
        1,
        "and its offset counts from ॱपाठ"
    );
    println!("METRIC t1_pending_kind {kind}");
}

/// **`संयोजन`'s PLACEMENT CONSTANTS ARE `वास्तु ॱ स्थापन` PLUS ONE, AND THAT IS
/// DELIBERATE.**
///
/// The enum is zero-based (`पाठ्यम्` reads `Int(0)`) and so is Rust's
/// `Placement`; the mirror at `samyojana.t1:46` runs १ २ ३ … ६. A fresh record
/// is `भवति ०`, so `स्थापनम्` starts at 0 — and `स्थापनाधारः` has arms for १ २ ३
/// and falls through to `शून्यम्`. **With Text=0 an UNFILLED record would read
/// as `.text` and be placed at the text base; with Text=1 it is refused.** The
/// offset buys the distinction between UNSET and THE FIRST VARIANT.
///
/// **THIS PINS THE RELATIONSHIP AND NOT EITHER SIDE.** `mirror == enum` fails
/// today and would invite exactly the wrong repair; asserting the mirror alone
/// is unfalsifiable. The change this is actually for is someone "fixing" the
/// off-by-one in ONE place — the plausible-looking edit a reviewer waves
/// through, and the only one of the three a naive equality test would encourage.
///
/// A variant that answers `None` is not a failure: `W-224` gives a bare key
/// only to a name every declaring module agrees on, so `अनिर्दिष्टम्` — declared
/// elsewhere too — has none. The COUNT of pairs actually compared is asserted,
/// because a loop that skips every case passes.
#[test]
fn the_placement_mirror_is_the_enum_plus_one() {
    // No `mut`: this reads globals and calls nothing.
    let it = assembler();
    let pairs = [
        ("पाठ्यम्", "पाठनिवेशः"),
        ("दत्तम्", "दत्तनिवेशः"),
        ("शून्यक्षेत्रम्", "बीजनिवेशः"),
        ("शोधनपङ्क्तिः", "शोधनपङ्क्तिनिवेशः"),
        ("अनिर्दिष्टम्", "अनिर्दिष्टनिवेशः"),
        ("अन्यत्", "अन्यनिवेशः"),
    ];
    let mut compared = 0;
    let mut skipped: Vec<&str> = Vec::new();
    for (variant, mirrored) in pairs {
        let Some(ordinal) = it.global(variant).and_then(Value::as_int) else {
            skipped.push(variant);
            continue;
        };
        let m = it
            .global(mirrored)
            .and_then(Value::as_int)
            .unwrap_or_else(|| panic!("`{mirrored}` is not declared — the mirror lost a member"));
        assert_eq!(
            m,
            ordinal + 1,
            "`{mirrored}` must be `{variant}` + 1: the enum says {ordinal}, the \
             mirror says {m}. If this fired because someone renumbered ONE side, \
             the fix is both sides or neither — 0 is `unset` and `स्थापनाधारः` \
             refuses it, which is the whole point of the offset."
        );
        compared += 1;
    }
    assert!(
        compared >= 4,
        "only {compared} of 6 placements were compared (skipped {skipped:?}) — \
         a loop that skips every case passes"
    );
    println!("METRIC t1_placement_pairs_compared {compared} skipped {skipped:?}");
}

/// **THE OBJECT THIS FILE BUILDS GOES INTO `संयोजनम्` AND LINKS.**
///
/// That is the whole point of the unit. `संयोजनम्` is a complete whole-program
/// linker in this corpus and had no `.t1` caller, because nothing in `.t1`
/// could make a `वास्तुॱवस्तु`. This test makes one and links it — with no
/// Rust stage building the record, only Rust calling in.
#[test]
fn the_built_object_links() {
    let mut it = assembler();
    let src = concat!("॥ वैश्विकम् मुख्यम् ॥\n", "मुख्यम्ॱॱ\n", "योगः अर्थ०म् शून्यःन १न ।\n",);
    let n = it
        .call(
            "वाक्यविभागॱसङ्कलनम्",
            vec![Value::Octets(Octets::new(src.as_bytes()))],
            8_000_000_000,
        )
        .expect("सङ्कलनम्")
        .as_int()
        .unwrap_or(0);
    assert!(n > 0, "the assembler read no statements");

    let program = it
        .call("वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000)
        .expect("कार्यक्रमरचना");
    let target = it.global("असङ्कुचितम्").cloned().unwrap_or(Value::Int(0));
    let places = it
        .call(
            "सङ्केतनॱस्थानविन्यासः",
            vec![program.clone(), target],
            2_000_000_000,
        )
        .expect("स्थानविन्यासः");
    let text_octets = it
        .call("सङ्केतनॱवस्तुसङ्केतनम्", vec![program.clone()], 2_000_000_000)
        .expect("वस्तुसङ्केतनम्");
    assert!(
        text_octets.octets().is_some(),
        "the encoder refused: {text_octets:?}"
    );

    let object = it
        .call(
            "सङ्केतनॱवस्तुरचना",
            vec![program, places, text_octets],
            2_000_000_000,
        )
        .expect("वस्तुरचना");
    let obj = match &object {
        Value::Record(_) => object.clone(),
        other => panic!(
            "वस्तुरचना refused: {other:?}\n  अन्तिमवस्तुदोषः: {:?}",
            it.global("अन्तिमवस्तुदोषः")
        ),
    };

    // THE SYMBOL TABLE: index ० is the null symbol, index १ is the `ॱपाठ`
    // SECTION symbol, and the label follows both.
    //
    // SUPERSEDED, AND KEPT RATHER THAN QUIETLY EDITED: this margin read "index
    // ० is the null symbol, and the label follows it" until the `%pcrel_lo12`
    // emitter landed. That was true and is now false — a `%pcrel_lo` names the
    // ADDRESS OF ITS MATCHING `%pcrel_hi` rather than the target, so the object
    // must carry a section symbol for it to name, and `वस्तुसंज्ञासारणी` mints
    // one at index १ before every label.
    //
    // THIS TEST CAUGHT THE SHIFT, WHICH IS THE POINT OF PINNING AN INDEX. The
    // Rust twin's margin at `kosha.rs:253` records the same hazard from the
    // other side: a symbol inserted after the index lookup existed left "every
    // text relocation naming the symbol one place below the one it meant". The
    // relocations here are built in the same pass as the table, so they moved
    // together — and a test reading a FIXED index is what proves it, because a
    // by-name lookup would have passed either way and said nothing.
    let syms = match field(&obj, "संज्ञाः") {
        Value::Arena(a) => a.borrow().clone(),
        other => panic!("संज्ञाः is an arena, not {other:?}"),
    };
    assert!(
        syms.len() >= 3,
        "null symbol, section symbol, and the label: {syms:?}"
    );
    assert_eq!(text(&field(&syms[0], "नाम")), "", "index ० is nameless");

    // The section symbol is nameless BY DESIGN and that is load-bearing twice:
    // `संयोजनॱसंज्ञास्थानम्` SKIPS an empty name when computing addresses —
    // correct, a section symbol is a base and not a definition — while
    // `लेखसंज्ञा` finds it by INDEX for relocations. Asserting it is LOCAL
    // matters too: pushed global it would be exported and could collide across
    // objects, where fifteen of them meet in one image.
    assert_eq!(
        text(&field(&syms[1], "नाम")),
        "",
        "index १ is the `ॱपाठ` section symbol and carries no name of its own"
    );
    assert!(
        matches!(field(&syms[1], "वैश्विकम्"), Value::Bool(false)),
        "the section symbol is LOCAL: {:?}",
        field(&syms[1], "वैश्विकम्")
    );

    assert_eq!(text(&field(&syms[2], "नाम")), "मुख्यम्");
    assert!(
        matches!(field(&syms[2], "वैश्विकम्"), Value::Bool(true)),
        "`मुख्यम्` was pushed global"
    );

    // AND IT LINKS. `संयोजनम्` takes an ARRAY of objects.
    let objects = Value::Arena(std::rc::Rc::new(std::cell::RefCell::new(vec![obj])));
    let linked = it
        .call("संयोजनॱसंयोजनम्", vec![objects], 8_000_000_000)
        .expect("संयोजनम् runs");
    assert!(
        !matches!(linked, Value::Nil),
        "संयोजनम् refused the object this file built.\n  संयोजनदोषसूचकाङ्क: {:?}",
        it.global("संयोजनदोषसूचकाङ्क")
    );
    println!("METRIC t1_linked_object_symbols {}", syms.len());
}

/// **`वैश्विकत्वम्` ANSWERS EXACTLY THE SAME FOR EVERY EDGE** (symbol-lookup step 1).
///
/// The routine compared each candidate global as a SLICE — a fresh block with
/// its octets copied — and now compares it IN PLACE through `परिधिसाम्यम्`. The answer
/// must not move: this pins it on an equal name (first and later in the
/// arena), a name a global is a prefix of, a byte prefix of a global, a name
/// that differs from a global in its LAST octet only (what a compare one octet
/// short calls equal), the empty name (index ० and the section symbol are both
/// nameless), and a label that is not pushed global. A characterisation test:
/// it passed on the slice compare before the change and passes after it.
#[test]
fn the_global_test_answers_the_same_on_every_edge() {
    let routine = "सङ्केतनॱवैश्विकत्वम्";
    let built = |src: &str| -> (Interpreter, Value) {
        let mut it = assembler();
        let n = it
            .call(
                "वाक्यविभागॱसङ्कलनम्",
                vec![Value::Octets(Octets::new(src.as_bytes()))],
                8_000_000_000,
            )
            .expect("सङ्कलनम्")
            .as_int()
            .unwrap_or(0);
        assert!(n > 0, "the assembler read no statements from:\n{src}");
        let program = it
            .call("वाक्यविभागॱकार्यक्रमरचना", vec![], 20_000_000)
            .expect("कार्यक्रमरचना");
        (it, program)
    };
    let ask = |it: &mut Interpreter, program: &Value, name: &[u8]| -> bool {
        match it
            .call(
                routine,
                vec![program.clone(), Value::Octets(Octets::new(name))],
                200_000_000,
            )
            .unwrap_or_else(|e| panic!("{routine}: {e:?}"))
        {
            Value::Bool(b) => b,
            other => panic!("{routine} answered {other:?}, not a बूल"),
        }
    };

    let first = "मुख्यम्";
    let second = "द्वितीयम्";
    // One differing in its last octet only: same length, every octet but the last equal.
    let mut last_off = first.as_bytes().to_vec();
    *last_off.last_mut().unwrap() ^= 1;
    let mut longer = first.as_bytes().to_vec();
    longer.extend_from_slice(second.as_bytes());
    let short = &second.as_bytes()[..second.len() - 1];

    // (1) ONE GLOBAL, ONE LOCAL LABEL — the program of the test above.
    let src1 = format!(
        "॥ वैश्विकम् {first} ॥\n{first}ॱॱ\nयोगः अर्थ०म् शून्यःन १न ।\n{second}ॱॱ\nयोगः अर्थ०म् शून्यःन २न ।\n"
    );
    let (mut it, p) = built(&src1);
    let mut answers = Vec::new();
    for (what, name, want) in [
        ("the global itself", first.as_bytes(), true),
        ("a label not pushed global", second.as_bytes(), false),
        ("the empty name", &b""[..], false),
        ("the global then more octets", &longer[..], false),
        (
            "the global with its last octet changed",
            &last_off[..],
            false,
        ),
    ] {
        let got = ask(&mut it, &p, name);
        assert_eq!(got, want, "program 1, {what}: {routine} answered {got}");
        answers.push(got);
    }

    // (2) TWO GLOBALS, the second label pushed FIRST, so a match on the first
    // label is found only after the scan steps past a non-matching candidate.
    let src2 = format!(
        "॥ वैश्विकम् {second} ॥\n॥ वैश्विकम् {first} ॥\n{first}ॱॱ\nयोगः अर्थ०म् शून्यःन १न ।\n{second}ॱॱ\nयोगः अर्थ०म् शून्यःन २न ।\n"
    );
    let (mut it, p) = built(&src2);
    for (what, name, want) in [
        ("the global pushed second", first.as_bytes(), true),
        ("the global pushed first", second.as_bytes(), true),
        ("the empty name", &b""[..], false),
        ("a byte prefix of a global", short, false),
        ("one global then the other", &longer[..], false),
        ("a global with its last octet changed", &last_off[..], false),
    ] {
        let got = ask(&mut it, &p, name);
        assert_eq!(got, want, "program 2, {what}: {routine} answered {got}");
        answers.push(got);
    }
    assert_eq!(answers.len(), 11, "every edge was asked");
    println!(
        "METRIC t1_global_test_edges {} true {}",
        answers.len(),
        answers.iter().filter(|b| **b).count()
    );
}
