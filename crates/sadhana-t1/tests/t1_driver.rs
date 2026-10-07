//! **THE DRIVER — `शृङ्खला`, the `.t1` port of `chain.rs`'s stage sequence.**
//!
//! `chain.rs` is 1005 lines; 233 are the stages and 689 are the Rust↔interpreter
//! bridge, which a driver on the interpreter's own side does not need. This file
//! tests what replaces the 233.

use sadhana::t1::chain::CHAIN;
use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::path::{Path, PathBuf};

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

fn load(names: &[&str]) -> Interpreter {
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| ((*n).to_string(), source(n)))
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root()).unwrap_or_else(|e| panic!("{names:?} load: {e:?}"))
}

/// **THE LAST HOP FIRST.** The driver has to give the emitter a name table, and
/// the path is four hops: a routine's `SymbolId`, the declaration that symbol
/// names, that declaration's name TOKEN, and the token's TEXT.
///
/// This asserts the LAST one on its own, before any loop is built over it,
/// because it is the shape that has failed twice today by being RIGHT-LOOKING:
/// `कारकपदपठनम्` answered a plausible `असत्यम्` without consulting anything, and
/// `सङ्कलनम्` answered a plausible four while building nothing. A hop that
/// returns *some* octets would carry a loop all the way to a wrong name table.
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn a_tokens_text_is_the_text_and_not_merely_some_octets() {
    let mut it = load(&["lex.t1"]);
    // Three words and a daṇḍa. Token १ must be `मण्डलम्` — not empty, not the
    // whole line, not the next word.
    let src = "मण्डलम् क ॥\n";
    let n = it
        .call(
            "पदविभागॱपदविभाग",
            vec![octets(src.as_bytes())],
            2_000_000_000,
        )
        .expect("पदविभाग runs")
        .as_int()
        .unwrap_or(0);
    assert!(
        n >= 3,
        "the fixture lexes to at least three tokens, got {n}"
    );

    let tok = match it.global("चिह्नककोश") {
        Some(Value::Arena(a)) => a.borrow()[1].clone(),
        other => panic!("`चिह्नककोश` is an arena, not {other:?}"),
    };
    let text = it
        .call("पदविभागॱचिह्नकपाठः", vec![tok], 20_000_000)
        .expect("चिह्नकपाठः runs");
    let text = match text.octets() {
        Some(o) => String::from_utf8(o.as_slice().to_vec()).expect("UTF-8"),
        None => panic!("चिह्नकपाठः must answer octets, not {text:?}"),
    };
    assert_eq!(
        text, "मण्डलम्",
        "token १ of `{src}` is `मण्डलम्`; a hop that answers other octets would \
         carry a loop all the way to a wrong name table"
    );
}

/// The driver's first stage: source octets in, a declaration count out, through
/// `पदविभाग` and `व्याकर` with no Rust stage between them.
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn the_drivers_first_stage_lexes_and_parses() {
    // THE SHIPPED MANIFEST, not a copy of it. A hand-written list here would
    // pass this test while `CHAIN` was missing a module, which is the exact
    // failure this test exists to catch.
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");
    let src = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ ।\nइति\n";
    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 4_000_000_000)
        .expect("पठनम् runs")
        .as_int()
        .unwrap_or(0);
    assert!(
        decls > 0,
        "the source declares a module and a routine; the driver read {decls}"
    );

    // CONTROL: a source that lexes to nothing must answer ०, so the pass above
    // is the stages agreeing rather than this test accepting anything.
    let empty = it
        .call("शृङ्खलाॱपठनम्", vec![octets(b"")], 4_000_000_000)
        .expect("पठनम् runs on an empty source")
        .as_int()
        .unwrap_or(-1);
    assert_eq!(empty, 0, "an empty source has no declarations");
}

/// **THE FRONT HALF, STAGE BY STAGE, THROUGH `.t1` ALONE.** Source octets to
/// built IR: `पदविभाग`, `व्याकर`, `अर्थ` twice, `मध्यरूप`. No Rust stage between
/// them; `nirvahana` interpreting the modules is the bootstrap.
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn the_driver_carries_a_source_to_built_ir() {
    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "shrinkhala.t1",
    ]);
    let src = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ योगः ४ ।\nइति\n";

    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 4_000_000_000)
        .expect("पठनम् runs")
        .as_int()
        .unwrap_or(0);
    assert!(decls > 0, "the source parses to declarations");

    let ok = it
        .call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 8_000_000_000)
        .expect("निर्णयः runs");
    assert_eq!(
        ok,
        Value::Bool(true),
        "the source resolves and typechecks; `अर्थ` refused it"
    );

    let routines = it
        .call("शृङ्खलाॱरचना", vec![Value::Int(decls)], 8_000_000_000)
        .expect("रचना runs")
        .as_int()
        .unwrap_or(0);
    assert_eq!(routines, 1, "one routine declared, one built");

    // AND IT BUILT REAL INSTRUCTIONS, not an empty routine. `३ योगः ४` is two
    // constants and an add — the same shape `t1_execution.rs` pins — so a
    // driver that ran every stage and produced nothing fails here rather than
    // reporting a routine count and a hollow arena.
    let insts = it.global("आज्ञासूचकाङ्क").and_then(Value::as_int).unwrap_or(0);
    assert_eq!(
        insts, 3,
        "`३ योगः ४` is two ध्रुवाज्ञा and one योगाज्ञा; the driver built {insts}"
    );
}

/// **THE NAME TABLE, BUILT BY `.t1` FROM `.t1`.** Four hops — a routine's
/// SymbolId, the declaration it names, that declaration's name token, the
/// token's text — replacing what `paradigm_encode`'s `write_names_into_t1` does
/// from Rust.
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn the_driver_fills_the_emitters_name_table_itself() {
    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
        "shrinkhala.t1",
    ]);
    let src = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ योगः ४ ।\nइति\n";
    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 4_000_000_000)
        .expect("पठनम्")
        .as_int()
        .unwrap_or(0);
    it.call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 8_000_000_000)
        .expect("निर्णयः");
    it.call("शृङ्खलाॱरचना", vec![Value::Int(decls)], 8_000_000_000)
        .expect("रचना");

    let named = it
        .call(
            "शृङ्खलाॱनामसञ्चयः",
            vec![octets("क".as_bytes())],
            2_000_000_000,
        )
        .expect("नामसञ्चयः runs")
        .as_int()
        .unwrap_or(0);
    assert_eq!(named, 1, "one routine, one name recorded");

    // THE NAME IS THE NAME. A loop that recorded an entry per routine and put
    // the wrong text in it would pass the count above — the same right-looking
    // shape as a hop that answers some octets.
    let entry = match it.global("यन्त्रनामकोश") {
        Some(Value::Arena(a)) => a.borrow()[1].clone(),
        other => panic!("`यन्त्रनामकोश` is an arena, not {other:?}"),
    };
    let routine = match &entry {
        Value::Record(r) => r.borrow().get("वृत्तिनाम").cloned(),
        other => panic!("not a record: {other:?}"),
    };
    let routine = match routine {
        Some(v) => match v.octets() {
            Some(o) => String::from_utf8(o.as_slice().to_vec()).expect("UTF-8"),
            None => panic!("`वृत्तिनाम` is a run of octets, not {v:?}"),
        },
        None => panic!("the entry has no `वृत्तिनाम`"),
    };
    assert_eq!(routine, "ग", "the routine's name, read through four hops");
}

/// **THE JOIN, PROVEN RATHER THAN ARGUED: a `.t1` source becomes Sassembly text,
/// and that text goes straight into the assembler.**
///
/// Two halves each green is the oldest false positive there is, so this is one
/// test: `शृङ्खलाॱमण्डलसङ्कलनम्` emits, and `वाक्यविभागॱसङ्कलनम्` reads what it
/// emitted, with nothing between them. No Rust compiler stage is in the path —
/// `nirvahana` interpreting the modules is the bootstrap.
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn the_emitters_text_goes_straight_into_the_assembler() {
    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
        "vishlesana.t1",
        "ashtaka.t1",
        "encode.t1",
        "vakyavibhaga.t1",
        "shrinkhala.t1",
    ]);
    let src = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् ३ योगः ४ ।\nइति\n";

    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src.as_bytes()), octets("क".as_bytes())],
            60_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs");
    let text = match text.octets() {
        Some(o) => String::from_utf8(o.as_slice().to_vec()).expect("UTF-8"),
        None => panic!("the driver answered no octets: {text:?}"),
    };
    assert!(
        !text.is_empty(),
        "the driver emitted nothing — a stage refused and returned the empty run"
    );
    assert!(
        text.contains("कग"),
        "the emitted text carries the routine's label `कग`:\n{text}"
    );

    // AND THE ASSEMBLER READS IT. This is the join: the front half's output is
    // the back half's input, and nothing translates between them.
    it.call("वाक्यविभागॱआरम्भः", vec![], 200_000_000)
        .expect("आरम्भः");
    it.call("वाक्यविभागॱसंज्ञाकुलपठनम्", vec![], 2_000_000_000)
        .expect("संज्ञाकुलपठनम्");
    it.call("वाक्यविभागॱनिर्देशकोशपठनम्", vec![], 2_000_000_000)
        .expect("निर्देशकोशपठनम्");
    let statements = it
        .call(
            "वाक्यविभागॱसङ्कलनम्",
            vec![octets(text.as_bytes())],
            8_000_000_000,
        )
        .expect("सङ्कलनम् runs")
        .as_int()
        .unwrap_or(0);
    assert!(
        statements > 0,
        "the assembler read no statements from the emitter's own text:\n{text}"
    );
    let insts = it.global("आज्ञासूचकाङ्क").and_then(Value::as_int).unwrap_or(0);
    assert!(
        insts > 0,
        "the assembler split {statements} statements and built {insts} \
         instructions — a splitter with no reader behind it answers a plausible \
         number and builds nothing:\n{text}"
    );
    println!("METRIC t1_driver_emitted_octets {}", text.len());
    println!("METRIC t1_driver_assembled_instructions {insts}");
}

/// **FACT #1 — THE TRACKED NUMBER: sources compiling with `chain.rs` UNREACHED.**
///
/// `the_driver_fills_the_emitters_name_table_itself` above proves the four-hop
/// name path on a FOUR-LINE TOY with ONE routine. This is the same path on a
/// REAL corpus source, and the difference is the whole point: **one routine
/// cannot collide with itself**, so the toy exercises precisely the case that
/// cannot fail.
///
/// `ashtaka.t1` is the first rung of the ladder — **9 routines, ZERO imports,
/// zero cross-module calls.** It needs nothing this driver does not already
/// have: no symbol may come from another module, so a failure here is the `.t1`
/// chain failing to name its OWN routines.
///
/// # THIS IS EXPECTED TO FAIL, AND THE RED IS THE DELIVERABLE
///
/// Measured on `lex.t1`: all fifteen routines carry `वृत्ति ॱ नाम` = ०, because
/// `मध्यरूप` pushes every routine with `SymbolId(0)` — faithfully mirroring
/// `ir.rs:661/:715/:814`, which does the same — and `chain.rs:616` then writes
/// the real symbol back with the margin *"the symbol chosen here is written
/// back where the T1 emitter reads it"*. **On every Rust-orchestrated run the
/// field is true by the time anything looks. A `.t1`-only run had never
/// happened.**
///
/// So hop one reads ० for every routine, `संज्ञाघोषणाकोश[०+१]` answers the
/// file's FIRST DECLARATION for all of them, and the emitter refuses with
/// `LabelCollision` — loudly, as `नामसञ्चयः`'s own margin promises.
///
/// **The dependency is real and REMOVABLE**: `अर्थ` already holds every name and
/// symbol the mapping needs. What is missing is that `निर्णयः` builds the
/// resolver as a local and drops it.
///
/// # THE COUNT IS NOT THE ASSERTION
///
/// `नामसञ्चयः` answers what it PROCESSED, not what it WROTE — on `lex.t1` it
/// returned 15 while `यन्त्रनामकोश` held ONE entry. **A count assertion alone
/// would pass over exactly the defect this test exists to find**, so the table's
/// size and the names themselves are asserted beside it. Members, not a count.
#[test]
// THE ORIGINAL CAUSE IS FIXED AND THIS IS A DIFFERENT RED. Kept dated rather
// than rewritten, because the superseded sentence is the witness for what the
// naming unit actually moved:
//   was — मध्यरूप pushes SymbolId(0) (mirroring ir.rs) and chain.rs:616 writes
//   the real one back, so a .t1-only run reads ० for every routine and the
//   emitter refuses. Un-ignore when शृङ्खला fills वृत्ति ॱ नाम from the resolver.
// शृङ्खला now does exactly that: named ९, table_entries ९, nine distinct correct
// names. That clause is satisfied and its three assertions are GREEN.
// UN-IGNORED 2026-09-09, its own stated condition met — `उत्सर्जनम्` answers
// 44785 octets. Both superseded reasons kept, dated, because they are the
// record of what the unit moved and each was TRUE WHEN WRITTEN:
//   first — मध्यरूप pushes SymbolId(0) (mirroring ir.rs) and chain.rs:616 writes
//   the real one back, so a .t1-only run reads ० for every routine.
//   then — the three naming assertions passed and the emitter still refused the
//   filled table with यन्त्रनिषेधभेद ४ (UnnamedSymbol), raised on symbol ४,
//   `अष्टकदोषमस्ति`: a GLOBAL, and नामसञ्चयः walked routines only.
// The second reason listed five candidate raise sites and declined to guess
// among them; the answer was the population, not the site.
fn the_driver_names_every_symbol_of_ashtaka() {
    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
        "vishlesana.t1",
        "ashtaka.t1",
        "encode.t1",
        "vakyavibhaga.t1",
        "shrinkhala.t1",
    ]);
    let src = source("ashtaka.t1");
    println!("METRIC t1_fact1_source_octets {}", src.len());

    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 40_000_000_000)
        .expect("पठनम्")
        .as_int()
        .unwrap_or(0);
    assert!(decls > 0, "lex and parse answered {decls} for ashtaka.t1");

    let typed = it
        .call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 80_000_000_000)
        .expect("निर्णयः");
    assert!(
        matches!(typed, Value::Bool(true)),
        "resolve and typecheck answered {typed:?}"
    );

    let routines = it
        .call("शृङ्खलाॱरचना", vec![Value::Int(decls)], 80_000_000_000)
        .expect("रचना")
        .as_int()
        .unwrap_or(0);
    // NINE DECLARED, TEN BUILT (2026-09-14). `ir.t1:555` synthesises ONE growth
    // routine per module that needs one — `खण्डवृद्धिः` — and ashtaka declares six
    // runs, so it needs one. The old pin said `9` and had been red on main since
    // that landing. A bare `10` would not tell a synthesised routine from a
    // regression next time, so the count is asserted TOGETHER WITH the flag that
    // causes it: a module with no run declaration leaves the flag false and
    // builds exactly its declarations.
    let grew = matches!(it.global("वृद्धिवृत्तिप्रयुक्तम्"), Some(Value::Bool(true)));
    // NINE -> TEN DECLARED ON 2026-09-14: `मुद्रणम्`, the output channel's
    // declaration. TEN -> ELEVEN ON 2026-09-26: `पत्रम्`, the FILE window's
    // declaration (ADR-0041) — the second intrinsic to be lowered to a store
    // rather than a call, and the second to keep a body that answers its own
    // device address when no interception fires. ELEVEN -> TWELVE the same day:
    // `पत्रलेखनम्`, the WRITE, which differs from `पत्रम्` only in which address
    // its third store names.
    //
    // TWELVE -> FOURTEEN ON 2026-10-02, `W-350`: `उपकरणचतुरष्टकाहारः` and
    // `उपकरणचतुरष्टकनिधानम्`, the four-octet device READ and WRITE. They are the
    // same shape as `पत्रम्`/`पत्रलेखनम्` one layer down — a pair differing only
    // in which address the store names, each keeping a sentinel body that answers
    // its own device address when no interception fires. So fifteen built:
    // fourteen declared plus the synthesised `खण्डवृद्धिः`.
    //
    // This pin firing on a deliberate addition is what it is for, and it has now
    // done so twice. On 2026-09-26 it was the ONLY red in a 130-target run; on
    // 2026-10-02 it was the ONLY red in the first 88 targets of the gate for this
    // branch, naming a file only `W-350` touches — which is also what made three
    // branches gated in one tree attributable. It is the reason the count is
    // asserted TOGETHER WITH the growth flag: a bare number would not say which
    // of the two moved, and here the flag stayed true while the count moved by
    // exactly the two routines added.
    assert_eq!(
        (routines, grew),
        (15, true),
        "ashtaka.t1 declares fourteen routines and needs the synthesised खण्डवृद्धिः"
    );

    let named = it
        .call(
            "शृङ्खलाॱनामसञ्चयः",
            vec![octets("अष्टक".as_bytes())],
            20_000_000_000,
        )
        .expect("नामसञ्चयः runs")
        .as_int()
        .unwrap_or(0);
    println!("METRIC t1_fact1_named {named}");

    // THE TABLE, NOT THE RETURN VALUE. `नामसञ्चयः` counts what it walked; the
    // arena is what the emitter reads. On `lex.t1` these were 15 and 1.
    let table = match it.global("यन्त्रनामकोश") {
        Some(Value::Arena(a)) => a.borrow().clone(),
        other => panic!("`यन्त्रनामकोश` is an arena, not {other:?}"),
    };
    let entries: Vec<String> = table
        .iter()
        .skip(1)
        .filter_map(|e| match e {
            Value::Record(r) => r.borrow().get("वृत्तिनाम").and_then(|v| {
                v.octets()
                    .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
            }),
            _ => None,
        })
        .collect();
    println!("METRIC t1_fact1_table_entries {}", entries.len());
    println!("  names: {}", entries.join(" "));

    // THE SYMBOLS THE TABLE IS KEYED BY, beside the names it holds. The names
    // come from वृत्तिनामचिह्नककोश, a side table that is right by construction,
    // so a name assertion is structurally blind to a wrong KEY — nine correct
    // names on nine wrong symbols passes every assertion above it. The emitter
    // refuses symbol ४; whether ४ appears here is what separates "the driver
    // wrote the wrong keys" from "the emitter wants a symbol the driver's walk
    // never covers", and those want opposite fixes.
    let keys: Vec<i128> = table
        .iter()
        .skip(1)
        .filter_map(|e| match e {
            Value::Record(r) => match r.borrow().get("संज्ञा") {
                Some(Value::Int(n)) => Some(*n),
                _ => None,
            },
            _ => None,
        })
        .collect();
    println!("  symbols: {keys:?}");

    // FIFTEEN, NOT NINE: the module declares six globals and nine routines, and
    // the emitter's table is keyed over one symbol space holding both. Nine was
    // this test's own figure for a walk that covered routines only — a count
    // that was RIGHT ABOUT ITS WALK and wrong about the population, which is
    // why it passed while symbol ४, a global, went unnamed and refused.
    //
    // **FIFTEEN → EIGHTEEN (2026-09-14), and this pin also names rather than
    // counts.** The fifteen declarations are untouched; three synthesised
    // symbols joined them on 2026-09-13 and nothing re-pinned this test, so it
    // has been red on main for a day. `खण्डवृद्धिः` is the per-module growth
    // routine (`ir.t1:555`, id १००००००५ — the SAME id in every module that needs
    // one, since it is one symbol per module and not one per corpus), and
    // `रचनासूचकः`/`रचनाक्षेत्रम्` are the module-less record cursor and its
    // region, which every module that builds a record must label.
    //
    // Fifteen was itself the correction of a nine that was right about its walk
    // and wrong about its population. Eighteen would repeat the mistake if it
    // stayed a number: a count cannot say WHICH three joined.
    assert_eq!(
        named as usize,
        entries.len(),
        "नामसञ्चयः must record what the table HOLDS, not merely be told a count"
    );
    let synth: Vec<i128> = keys.iter().copied().filter(|k| *k >= 10_000_000).collect();
    let mut synth_sorted = synth.clone();
    synth_sorted.sort_unstable();
    assert_eq!(
        synth_sorted,
        vec![10_000_003, 10_000_004, 10_000_005],
        "the record cursor, its region, and the growth routine"
    );

    // THE SYMBOLS ARE THE WHOLE SPACE, ० TO १५ WITH ० THE MODULE ITSELF. This
    // is the assertion the name list cannot make: names come from a side table
    // that is right by construction, so fifteen correct names on fifteen wrong
    // keys satisfies every membership check below. Sorted because the two
    // populations are appended in two passes and their ORDER is not the claim.
    // १-१५ → २-१६ → २-१७ (2026-09-14, twice in one day) → २-१८ → २-१९
    // → २-२१ (2026-10-02, `W-350`'s `उपकरणचतुरष्टकाहारः` and
    // `उपकरणचतुरष्टकनिधानम्`, two at once)
    // (2026-09-26, `पत्रम्` then `पत्रलेखनम्`, ADR-0041): the module's own ordinals still run contiguous, one
    // per declaration; `मुद्रणम्` is the sixteenth and `पत्रम्` the seventeenth.
    // The first move was the gather reserving a leading symbol, the rest are
    // declarations added — and the CONTIGUITY is what tells those two apart,
    // because a reserved slot shifts the range and a new declaration extends
    // it. Both moves have now been seen, which is why the distinction is worth
    // keeping.
    //
    // THIS PIN IS THE SECOND ASSERTION IN THIS TEST AND IT WAS INVISIBLE UNTIL
    // THE FIRST WAS FIXED. A red names what tripped FIRST; the gate reporting
    // one failure meant one failure REACHED, not one present.
    // one per declaration, but they start at २. Ordinal १ is skipped in kosha
    // too — two modules, the same leading gap — so this is the gather reserving
    // a symbol ahead of the walk, not something about ashtaka.
    let mut ks: Vec<i128> = keys.iter().copied().filter(|k| *k < 10_000_000).collect();
    ks.sort_unstable();
    assert_eq!(
        ks,
        (2..=21).collect::<Vec<i128>>(),
        "symbols २-१९ contiguous, one per declaration after the module: {keys:?}"
    );

    // MEMBERS. Fifteen identical labels would satisfy both counts above and is
    // exactly what a symbol of ० produces — every declaration resolving to the
    // file's first.
    let mut got: Vec<&str> = entries.iter().map(String::as_str).collect();
    got.sort_unstable();
    let mut want = [
        "अष्टकनिषेधः",
        "अष्टकारम्भः",
        "अष्टकदैर्घ्य",
        "अष्टकयोजनम्",
        "शून्याष्टकयोजनम्",
        "अष्टकपाठयोजनम्",
        "अष्टकस्थापनम्",
        "अष्टकपाठः",
        "अष्टकखण्डः",
        // THE OUTPUT CHANNEL, 2026-09-14 — a `.t1` image had no way to write an
        // octet, and this is it. Both the `ir.t1` lowering and the interpreter
        // replace calls to it; the body it keeps answers २६८४३५४५६ so a broken
        // wire names itself rather than returning a plausible ०.
        "मुद्रणम्",
        // THE FILE WINDOW, 2026-09-26 (ADR-0041) — the SECOND channel out of a
        // program, and built to the same rule: `ir.t1` replaces the call with a
        // store, and the body it keeps answers २६८४४३६४८, its own device
        // address. The interpreter does NOT intercept this one, because it has
        // no RAM for the request run to point into — so an interpreted run
        // reaches the body and is told so.
        "पत्रम्",
        // AND THE WRITE, same day. It shares `पत्रम्`'s three runs and its two
        // remembered stores; only the third address differs, which is why it is
        // a separate declaration rather than a flag on the first.
        "पत्रलेखनम्",
        // W-350's device register window, 2026-10-02 — the two routines that
        // made the ordinal range above २-२१. THEY BELONG HERE AND NOT IN A
        // COUNT, which is this list's whole reason: a count cannot say WHICH
        // two joined, and the range assertion above cannot say what they are
        // called.
        "उपकरणचतुरष्टकाहारः",
        "उपकरणचतुरष्टकनिधानम्",
        // The six globals, `ashtaka.t1:41-50`, symbols २-७ (१-६ before the
        // 2026-09-14 shift). `अष्टकदोषमस्ति`, symbol ५ today and ४ then, is the
        // one the emitter named in its refusal.
        "अष्टककोश",
        "अष्टकसूचकाङ्क",
        "रिक्ताष्टकाः",
        "अष्टकदोषमस्ति",
        "अष्टकदोषस्थानम्",
        "अष्टकदोषनाम",
        // AND THE THREE SYNTHESISED, which no declaration in this file produces:
        // the per-module growth routine and the module-less record cursor pair.
        // They belong in the MEMBER list rather than in a count, so that the next
        // symbol the compiler invents fails here by name.
        "खण्डवृद्धिः",
        "रचनासूचकः",
        "रचनाक्षेत्रम्",
    ];
    want.sort_unstable();
    assert_eq!(
        got, want,
        "each declaration's OWN name, not fifteen of the first"
    );

    // **AND THE EMITTER ACCEPTS THE TABLE — the acceptance, not a proxy for it.**
    //
    // The three assertions above are about what the driver WROTE. This is the
    // only one that asks whether the thing that READS it agrees.
    // `यन्त्रचिह्नपरीक्षा` walks every routine and looks each up BY SYMBOL, and
    // a call emits `आह्वेयसंज्ञा` from the call site's own resolver symbol — so
    // a table keyed under the wrong symbols makes definition and call disagree
    // and the emitter answers an empty run.
    //
    // **NINE CORRECT NAMES ON NINE WRONG KEYS WOULD PASS EVERYTHING ABOVE.**
    // The name comes from `वृत्तिनामचिह्नककोश`, a per-routine side table that is
    // right by construction; the lookup supplies only the KEY. A name assertion
    // is structurally blind to a key defect, which is a limit of the guard and
    // not a hypothetical — it is why this line exists.
    let text = it
        .call("शृङ्खलाॱउत्सर्जनम्", vec![], 40_000_000_000)
        .expect("उत्सर्जनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    println!("METRIC t1_fact1_emitted_octets {}", text.len());
    // THE FIELDS, NOT ONLY THE VARIANT. Five sites raise UnnamedSymbol and the
    // variant is the same from all of them, so the variant alone cannot say
    // which — and the difference decides what to fix. `संख्या` carries the
    // offending symbol and `प्रवेशसंज्ञा` is the entry symbol, which is ० exactly
    // when the module has no मुख्यम् — ashtaka has none.
    //
    // **`पर्व` DOES NOT SEPARATE THEM EITHER, WHICH THIS MARGIN USED TO CLAIM.** It
    // said `पर्व` is "the मध्यरूप block (० for the label and entry sites, a real id
    // for a call)"; `यन्त्राह्वानोत्सर्जनम्` passes ० there too, so all five sites
    // write `पर्व ०`. Nor does `संख्या` when the offending symbol IS ० — the case a
    // missed lookup of symbol ० always produces. `यन्त्रोत्सर्जन` now passes a SITE
    // CODE in `लक्ष्य` at each of the five, read below by the name the module
    // publishes for it; `crates/sadhana-t1/tests/t1_refusal_site.rs` holds it, with
    // both codes mutated back to ० as its refused case.
    // ALL FOUR ARE READ BY BARE NAME: Interpreter::globals is keyed bare, and a
    // qualified key answers None, which prints indistinguishably from ०.
    for g in [
        "यन्त्रनिषेधभेद",
        "यन्त्रनिषेधसंख्या",
        "यन्त्रनिषेधपर्व",
        "यन्त्रनिषेधलक्ष्य",
        "यन्त्रप्रवेशसंज्ञा",
        // THE SITE, BY NAME. `लक्ष्य` above is the code; this is which of the five
        // routines it stands for, so the reading does not need the constant table.
        "यन्त्रनामस्थानचिह्नम्",
        "यन्त्रनामस्थानपरीक्षा",
        "यन्त्रनामस्थानाह्वानम्",
        "यन्त्रनामस्थानवृत्तिः",
        "यन्त्रनामस्थानप्रवेशः",
    ] {
        println!("  refusal {g} = {:?}", it.global(g));
    }
    assert!(
        !text.is_empty(),
        "the emitter refused a table this test just called correct — \
         भेद {:?}, symbol {:?}, block {:?}; ४ is UnnamedSymbol, ५ is LabelCollision",
        it.global("यन्त्रनिषेधभेद"),
        it.global("यन्त्रनिषेधसंख्या"),
        it.global("यन्त्रनिषेधपर्व"),
    );
}

/// **THE TWO NAME TABLES MUST BE THE SAME TABLE.** `chain.rs` builds one from
/// Rust and `शृङ्खलाॱनामसञ्चयः` now builds one from `.t1`; the emitter reads
/// whichever it is given, so "the `.t1` half works" means precisely that these
/// two agree — every symbol, every module, every name.
///
/// **THIS IS THE ASSERTION THE OTHER TEST CANNOT MAKE.** Fact #1 checks the
/// `.t1` table against a list written by hand from reading `ashtaka.t1`, so it
/// confirms my reading of the source. It cannot confirm that the emitter's
/// OTHER caller agrees, and a hand-written expectation that matches a wrong
/// convention is exactly the shape that passes while the product half diverges.
/// Here the expectation is the shipped Rust table, not my transcription.
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn the_t1_name_table_equals_the_rust_one() {
    use sadhana::t1::chain::Front;
    use std::collections::HashMap;

    let src = source("ashtaka.t1");

    // THE RUST HALF, through the shipped stages.
    let mut front = Front::load(&spec_root()).expect("Front loads");
    front.lex(&src).expect("lex");
    front.parse().expect("parse");
    front.resolve().expect("resolve");
    front.typecheck().expect("typecheck");
    front.build_ir().expect("build_ir");
    let rust = front.module("अष्टक", None).expect("module builds");
    let rust_names: HashMap<i128, (String, String)> = rust
        .names
        .iter()
        .map(|(s, (m, n))| (s.0 as i128, (m.clone(), n.clone())))
        .collect();

    // THE .t1 HALF, through the driver alone.
    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
        "shrinkhala.t1",
    ]);
    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 40_000_000_000)
        .expect("पठनम्")
        .as_int()
        .unwrap_or(0);
    it.call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 40_000_000_000)
        .expect("निर्णयः");
    it.call("शृङ्खलाॱरचना", vec![Value::Int(decls)], 40_000_000_000)
        .expect("रचना");
    it.call(
        "शृङ्खलाॱनामसञ्चयः",
        vec![octets("अष्टक".as_bytes())],
        40_000_000_000,
    )
    .expect("नामसञ्चयः");

    let table = match it.global("यन्त्रनामकोश") {
        Some(Value::Arena(a)) => a.borrow().clone(),
        other => panic!("`यन्त्रनामकोश` is an arena, not {other:?}"),
    };
    let t1_names: HashMap<i128, (String, String)> = table
        .iter()
        .skip(1)
        .filter_map(|e| match e {
            Value::Record(r) => {
                let r = r.borrow();
                let s = match r.get("संज्ञा") {
                    Some(Value::Int(n)) => *n,
                    _ => return None,
                };
                let txt = |k: &str| {
                    r.get(k)
                        .and_then(|v| v.octets())
                        .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
                        .unwrap_or_default()
                };
                Some((s, (txt("मण्डल"), txt("वृत्तिनाम"))))
            }
            _ => None,
        })
        .collect();

    println!("METRIC t1_nametable_rust_entries {}", rust_names.len());
    println!("METRIC t1_nametable_t1_entries {}", t1_names.len());

    // THE SYMBOL SETS FIRST, because a difference here is a different question
    // from a difference in the names: a missing symbol is a population the walk
    // does not cover, a wrong name on a right symbol is a bad hop.
    let mut rk: Vec<i128> = rust_names.keys().copied().collect();
    let mut tk: Vec<i128> = t1_names.keys().copied().collect();
    rk.sort_unstable();
    tk.sort_unstable();
    assert_eq!(
        tk, rk,
        "the two halves must name the SAME symbols; .t1 {tk:?} vs rust {rk:?}"
    );

    let mut wrong: Vec<String> = Vec::new();
    for (sym, want) in &rust_names {
        let got = &t1_names[sym];
        if got != want {
            wrong.push(format!("symbol {sym}: .t1 {got:?} vs rust {want:?}"));
        }
    }
    wrong.sort();
    assert!(
        wrong.is_empty(),
        "{} of {} entries disagree:\n  {}",
        wrong.len(),
        rust_names.len(),
        wrong.join("\n  ")
    );
}

/// **THE DRIVER'S OWN ENTRY POINT, not its stages one at a time.**
///
/// Every other test here calls `पठनम्`, `निर्णयः`, `रचना`, `नामसञ्चयः` and
/// `उत्सर्जनम्` individually, which means **none of them enters
/// `मण्डलसङ्कलनम्`** — the routine that sequences them and holds the driver's
/// guards. That was not a gap anyone reasoned about: it surfaced when an
/// inverted guard was expected to turn a test red and changed nothing at all.
///
/// A guard reasoned about and never exercised reads exactly like a guard that
/// works, so this test exists to give those guards a caller.
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn the_front_half_driver_compiles_a_module_through_its_own_entry_point() {
    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
        "shrinkhala.t1",
    ]);
    let src = source("ashtaka.t1");
    let text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src.as_bytes()), octets("अष्टक".as_bytes())],
            40_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    println!("METRIC t1_module_driver_octets {}", text.len());

    // THE GUARDS ANSWER रिक्तम् — an EMPTY text — for every refusal, so an
    // empty answer is the one observable they all share. That makes non-empty
    // the assertion, and it is why the inverted-guard control below is worth
    // running rather than reasoning about.
    assert!(
        !text.is_empty(),
        "the driver's own entry point refused ashtaka.t1: भेद {:?}, symbol {:?}",
        it.global("यन्त्रनिषेधभेद"),
        it.global("यन्त्रनिषेधसंख्या"),
    );

    // AND IT IS THE SAME TEXT THE STAGES PRODUCE BY HAND. `मण्डलसङ्कलनम्` is
    // meant to BE the sequence, not a second implementation of it; a driver
    // that ran the stages in a different order, or skipped one, would still
    // answer some non-empty text above.
    // **THIS IS A PIN, NOT A CROSS-PATH COMPARISON, AND THE MESSAGE BELOW MISLED
    // TWO READERS ON 2026-09-11.** The assertion compares `text.len()` to a
    // RECORDED LITERAL. This test calls the entry point ONCE; no staged path is
    // computed anywhere in it, so there is no second thing to diverge from. Its
    // red means *this number moved*, never *these two disagree* — and the first
    // diagnosis it produced was a paragraph about two driver paths that do not
    // exist. The prose above states the INTENT, which is true and is not what
    // the `assert_eq!` checks.
    //
    // **RE-TAKEN 44785 → 47186, 2026-09-11, W-283.** The store-to-a-global
    // lowering landed: a stub emits ONE constant where a lowered store emits
    // three instructions, so this number SHOULD rise and rising is not a defect.
    // `ashtaka.t1` has six module-level globals, of which two lower (twice each)
    // and three are refused by the arm's own scalar-integer guard. Measured on
    // the merged tree at `04ebba06`; `crates/sadhana-t1/src` and
    // `crates/sadhana/src` are byte-identical between that commit and this one,
    // so the figure transfers rather than being re-derived.
    //
    // **IT WILL MOVE AGAIN ON EVERY LOWERING THAT REACHES `ashtaka.t1`.** If you
    // are here because it went red, check the wall time and the by-cause census
    // first: this pin cannot tell a correct growth from a wrong one.
    // **RE-TAKEN 47186 → 47488, 2026-09-12, THE STORAGE MODEL'S THREE LANDINGS
    // (`W-284`, `W-285`, `W-287`), BY THE TRUNK.** +302 octets of emitted text.
    // The margin above says what to check before believing a rise, so: the
    // by-cause census is UNMOVED — `paradigm_ir_stubs` 2309 sites at
    // `b28b1142`, not re-measured today — and the wall time is 435s for this
    // binary's 13 tests, the same order as its recorded cost rather than 2x off
    // it. **THAT IS A WEAK CHECK AND IT IS STATED AS ONE:** this pin cannot
    // tell a correct growth from a wrong one, and nothing here establishes that
    // the 302 extra octets are the right instructions. What it does establish
    // is that the number moved for a reason the tree already records.
    //
    // AND THE MARGIN ABOVE EARNED ITS KEEP. This test's name and message read
    // as a cross-path comparison — "the entry point must produce what the
    // stages produce one by one" — and the trunk arrived intending to refuse to
    // re-found it for exactly that reason, a capability break being a different
    // problem from a stale count. The paragraph that says **THIS IS A PIN, NOT
    // A CROSS-PATH COMPARISON** settled it in one reading. A test's label is
    // not its contract; a margin that says so outright is worth its lines.
    // 47488 -> 49366 ON 2026-09-12, `W-293`: +1878 octets, and ashtaka.t1 has
    // THREE array globals. My change alters this text two ways — each array global
    // now emits a relocated pointer plus `ॱरिक्त` reserved space in place of one
    // quad, and each array-global READ lowers to a real load instead of a stub
    // constant.
    //
    // **I CAN ACCOUNT FOR ROUGHLY 828 + 450 OF THE 1878 AND NOT ALL OF IT.**
    // Measured from the emitted text: 7 new lines per global at ~376 octets less
    // the ~100-octet quad they replace, times three; plus 9 lines of converted
    // reads. The remainder is undecomposed, and decomposing it needs a
    // before/after emit from a binary built on the previous tree, which I did not
    // take. Saying so because a partial accounting quoted as a whole one is how a
    // number becomes inherited.
    //
    // The margin above already states the limit this re-take inherits: nothing
    // here establishes that the extra octets are the RIGHT instructions, only that
    // the number moved for a reason the tree records. That is what a pin is.
    // 49366 -> 49719 ON 2026-09-13, cause 22 `assign_name` (`agent/shared-arena-
    // witness`, `808580cf`): +353 octets. MEASURED, not inferred — the merge
    // census's `twin AGREE` line for ashtaka.t1 reads 49719 against 49366 on the
    // tree before it, and the by-cause table beside it shows shape २९
    // `assign_store_global` up by one on this source: `ashtaka.t1:56`'s
    // `अष्टकदोषनाम भवति नाम ।`, the one bare-name store to a run-typed global
    // in this file, now emits `वैश्विकस्थानाज्ञाभेद` then `स्थाननिधानाज्ञाभेद`
    // where it stubbed. The nil landing (`5dd5def0`) did NOT move this pin:
    // ashtaka carries no `शून्यम्`, and the trunk's per-source octet table
    // reads 49366 for it on both sides of that landing. Seven sources moved
    // with this one; this is the only one with a pinned count, so it is the
    // only one that says so loudly. Re-taken by the trunk as part of the
    // merge, because `paradigm_encode` does not run this crate.
    // 49719 -> 53303 ON 2026-09-13, `W-294` (2d, `8dd8f6be`): four narrow index
    // sites in ashtaka.t1 lower now — `गुणनम्` 4, of which two are reads and
    // emit `आहारःॱअ८`, two are writes and take the address form. 2d measured
    // 52950 on their base `046f3cfe` (49366 + their 3584); 6e's cause-22 site
    // gave 49719 (49366 + 353). THE MERGED VALUE IS A FOURTH NUMBER, MEASURED
    // from this assertion's failure on the merged tree — and it equals
    // 49719 + 52950 − 49366 exactly, which says the two landings' sites on this
    // source are DISJOINT. That equality is a reading about overlap, not the
    // way the number was obtained; had the sites shared an instruction the
    // arithmetic would have been wrong and only the measurement right.
    assert_eq!(
        text.len(),
        // 53303 -> 56947 on 2026-09-13, agent/runhdr: the run header at base-8 on every run adds the
        // header arms to ashtaka's octets; MEASURED by the narrowed census on this tree (T1_CORPUS=ashtaka, twin AGREE 56947).
        56947,
        "the entry point must produce what the stages produce one by one"
    );
}

/// **THE SECOND RUNG, AND THE ONLY CONTROLLED BEFORE/AFTER IN THE CORPUS.**
///
/// `lex.t1` is the one source with a RECORDED PRE-FIX READING. Measured
/// 2026-09-09 on a tree where `शृङ्खलाॱनामसञ्चयः` did not yet assign symbols,
/// bisected stage by stage with `chain.rs` unreached:
///
/// ```text
/// पठनम्        27 declarations       ok
/// निर्णयः      resolved, typechecked ok
/// रचना         15 routines built     ok
/// नामसञ्चयः    REPORTED 15, and the emitter's table HELD 1
/// उत्सर्जनम्    REFUSED — यन्त्रनिषेधभेद ५, LabelCollision
/// ```
///
/// **The refusal was `LabelCollision` (५), NOT `UnnamedSymbol` (४)**, which the
/// pattern at the time predicted. That anomaly is what made the mechanism
/// findable: `यन्त्रनामयोजनम्` is an UPSERT, so fifteen insertions carrying one
/// repeated symbol leave ONE entry — and fifteen upserts onto one key IS a
/// label collision.
///
/// **This column is written here because it existed only in a transcript.** A
/// before-reading held in a conversation becomes an unfalsifiable claim the
/// moment the conversation ends, and the delta this test reports rests on it.
///
/// So this is a DELTA rather than a second sample: 1 -> 25 on a source whose
/// old behaviour and old cause are both known. And it adds no new variable —
/// zero imports, same shape as `ashtaka` — so a red lands in the naming pass
/// rather than in a new layer. `kosha` comes next because it is the first rung
/// exercising a CROSS-MODULE symbol, which `ashtaka` and `lex` cannot test.
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn the_driver_names_every_symbol_of_lex() {
    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
        "shrinkhala.t1",
    ]);
    let src = source("lex.t1");
    println!("METRIC t1_lex_source_octets {}", src.len());

    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 40_000_000_000)
        .expect("पठनम्")
        .as_int()
        .unwrap_or(0);
    // 27 -> 31 ON 2026-09-13, the embed store: `lex.t1` declares FOUR new globals
    // — `समावेशनामकोश`, `समावेशपाठकोश`, `समावेशसीमाकोश`, `समावेशसंख्या` — the
    // arenas the host fills at load and `पदविभाग` reads to collapse an embed.
    // Routines unchanged at 15. MEASURED from this assertion's failure.
    assert_eq!(
        decls, 31,
        "the pre-fix column measured 27 declarations; 31 with the embed store"
    );

    let ok = it
        .call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 40_000_000_000)
        .expect("निर्णयः");
    assert_eq!(ok, Value::Bool(true), "`अर्थ` refused lex.t1");

    let routines = it
        .call("शृङ्खलाॱरचना", vec![Value::Int(decls)], 40_000_000_000)
        .expect("रचना")
        .as_int()
        .unwrap_or(0);
    assert_eq!(routines, 15, "the pre-fix column measured 15 routines");

    let named = it
        .call(
            "शृङ्खलाॱनामसञ्चयः",
            vec![octets("पदविभाग".as_bytes())],
            40_000_000_000,
        )
        .expect("नामसञ्चयः")
        .as_int()
        .unwrap_or(0);
    println!("METRIC t1_lex_named {named}");

    let table = match it.global("यन्त्रनामकोश") {
        Some(Value::Arena(a)) => a.borrow().clone(),
        other => panic!("`यन्त्रनामकोश` is an arena, not {other:?}"),
    };
    let entries: Vec<(i128, String)> = table
        .iter()
        .skip(1)
        .filter_map(|e| match e {
            Value::Record(r) => {
                let r = r.borrow();
                let s = match r.get("संज्ञा") {
                    Some(Value::Int(n)) => *n,
                    _ => return None,
                };
                let n = r
                    .get("वृत्तिनाम")
                    .and_then(|v| v.octets())
                    .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())?;
                Some((s, n))
            }
            _ => None,
        })
        .collect();
    println!("METRIC t1_lex_table_entries {}", entries.len());
    let mut shown: Vec<String> = entries.iter().map(|(s, n)| format!("{s}:{n}")).collect();
    shown.sort();
    println!("  {}", shown.join(" "));

    // 25 = ten globals + fifteen routines. The MODULE takes ० and is not an
    // entry, so the symbols are १..२५ with no gap — the same shape `ashtaka`
    // measured at १..१५.
    assert_eq!(
        named,
        // 25 -> 29 on 2026-09-13, the embed store: fourteen globals now — the ten
        // plus `समावेशनामकोश`, `समावेशपाठकोश`, `समावेशसीमाकोश`, `समावेशसंख्या` —
        // and fifteen routines. MEASURED from this assertion's failure.
        29,
        "fourteen globals and fifteen routines; table holds {}",
        entries.len()
    );
    assert_eq!(
        entries.len(),
        // 25 -> 29 on 2026-09-13, the embed store: the four new `lex.t1` globals
        // are emitter names and enter `यन्त्रनामकोश`. MEASURED from this failure.
        29,
        "the table must HOLD 29, not merely be told 29: {entries:?}"
    );

    // **CONTIGUITY IS THE WRONG INVARIANT AND `ashtaka` MADE IT LOOK RIGHT.**
    // A first version asserted `१..२५` contiguous and failed with `9` and `10`
    // absent: `lex.t1:152` and `:162` declare `सार्वजनिक संरचना चिह्नक` and
    // `संरचना दोष`, which take symbols in the same space and are TYPES. A record
    // type emits no code and no data and nothing relocates against it, so the
    // emitter never asks for its name and naming it would be writing an entry
    // no reader looks up. `ashtaka.t1` declares no `संरचना` — which is why its
    // symbols ran १..१५ with no gap, and why a rule generalised from it was
    // wrong the first time a source had a type in it.
    //
    // The invariant the consumer actually needs is **every symbol the emitter
    // ASKS FOR is named**, not every symbol that exists. So: the named set is
    // exactly the declaration indices that are globals or routines, and the
    // gaps are exactly the types.
    let mut keys: Vec<i128> = entries.iter().map(|(s, _)| *s).collect();
    keys.sort_unstable();
    // 1..=27 minus {9, 10}  ->  3..=29 minus {11, 12}  ON 2026-09-13, the gather
    // PORTED TO THE `.t1` DRIVER (`e58ec658`). +2 ELEMENTWISE, AND THE GAP MOVES
    // WITH IT — measured from this assertion's own failure, and confirmed by a
    // second reading on `e58ec658` (6e) of trees identical in every id-producing
    // file. Nothing missing, nothing extra: the count stays 25 and "the two
    // संरचना" is still exactly what is absent.
    //
    // WHY TWO: `शृङ्खला` now calls `घोषणासञ्चयॱसङ्ग्रहः`, which copies lex's own
    // declarations into the store, and `सञ्चयप्रकारबन्धः` mints a TYPE symbol per
    // store type entry BEFORE `कार्यक्रमनिर्णयः` walks the program. lex declares
    // two types — `संरचना चिह्नक`, `संरचना दोष` — so they take ids 1 and 2 and
    // lex's 27 declarations start at 3. They stay absent from `यन्त्रनामकोश`
    // because a type symbol is not an emitter name, which is why the gap
    // survives the shift instead of closing.
    //
    // THIS PIN AND ITS TWO SIBLINGS ARE THREE INSTRUMENTS ON ONE QUANTITY: the
    // twin test compares ids BETWEEN the two drivers (the port made it green with
    // no edit); this one pins ABSOLUTE ids (the port shifts it); the octet pin
    // pins emitted LENGTH (unmoved — a label is module ⧺ name, never module ⧺
    // id). An id is a counter position, and a change to what is counted first
    // moves two of the three and not the third.
    // 3..=29 -> 3..=33 on 2026-09-13, the embed store: four new globals in
    // `lex.t1`, declared AFTER the two संरचना, so the types keep 11 and 12 and
    // the range extends by four. MEASURED from this assertion's failure —
    // elementwise identical up to 29, then 30..=33 appended.
    let types: Vec<i128> = vec![11, 12]; // संरचना चिह्नक, संरचना दोष — after the two type symbols
    let want: Vec<i128> = (3..=33).filter(|s| !types.contains(s)).collect();
    assert_eq!(
        keys, want,
        "the named set is every declaration EXCEPT the two संरचना at 11 and 12"
    );

    // **`पदविभाग` IS BOTH THE MODULE NAME AND A ROUTINE NAME.** If
    // `अर्थॱनामनिर्णयः` resolved that bare name to the MODULE, the routine would
    // take symbol ० and collide — which the contiguity check above would catch
    // as a gap, but this names the suspect so a failure reads as a diagnosis
    // rather than a puzzle.
    let module_named: Vec<&(i128, String)> =
        entries.iter().filter(|(_, n)| n == "पदविभाग").collect();
    assert_eq!(
        module_named.len(),
        1,
        "the routine `पदविभाग` must be named exactly once and NOT as the module: {module_named:?}"
    );
    assert_ne!(
        module_named[0].0, 0,
        "the routine `पदविभाग` took the MODULE's symbol ०"
    );

    // **AND THE EMITTER ACCEPTS IT — the acceptance, not a property of the
    // table.** Everything above describes what the driver WROTE; this is the
    // only assertion that asks whether the thing that reads it agrees, and on
    // `ashtaka` it was the one that stayed red after three green ones.
    //
    // IT IS ALSO WHERE THE STRUCT QUESTION IS SETTLED RATHER THAN ARGUED. The
    // two `संरचना` are unnamed by design — a type emits nothing and nothing
    // relocates against it — but that is a claim about what the emitter needs,
    // and the emitter is right here to be asked. If it refuses with
    // `UnnamedSymbol ४` on symbol ९ or १०, the claim is wrong and types must be
    // named after all.
    let text = it
        .call("शृङ्खलाॱउत्सर्जनम्", vec![], 40_000_000_000)
        .expect("उत्सर्जनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    println!("METRIC t1_lex_emitted_octets {}", text.len());
    assert!(
        !text.is_empty(),
        "the emitter refused a table this test just called correct — \
         भेद {:?}, symbol {:?}, block {:?}",
        it.global("यन्त्रनिषेधभेद"),
        it.global("यन्त्रनिषेधसंख्या"),
        it.global("यन्त्रनिषेधपर्व"),
    );
}

/// **THE SAME CROSS-BOUNDARY CHECK, ON THE SECOND RUNG.**
///
/// `the_t1_name_table_equals_the_rust_one` does this for `ashtaka.t1`, where it
/// found the two halves identical. `lex.t1` is the harder case and the reason
/// to repeat it rather than assume it generalises: it declares two `संरचना`
/// that take symbols ९ and १०, and a routine `पदविभाग` sharing its module's
/// bare name. **If Rust and `.t1` disagree about either, they disagree here.**
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn the_t1_name_table_equals_the_rust_one_for_lex() {
    use sadhana::t1::chain::Front;
    use std::collections::HashMap;

    let src = source("lex.t1");

    let mut front = Front::load(&spec_root()).expect("Front loads");
    front.lex(&src).expect("lex");
    front.parse().expect("parse");
    front.resolve().expect("resolve");
    front.typecheck().expect("typecheck");
    front.build_ir().expect("build_ir");
    let rust = front.module("पदविभाग", None).expect("module builds");
    let rust_names: HashMap<i128, (String, String)> = rust
        .names
        .iter()
        .map(|(s, (m, n))| (s.0 as i128, (m.clone(), n.clone())))
        .collect();

    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
        "shrinkhala.t1",
    ]);
    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 40_000_000_000)
        .expect("पठनम्")
        .as_int()
        .unwrap_or(0);
    it.call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 40_000_000_000)
        .expect("निर्णयः");
    it.call("शृङ्खलाॱरचना", vec![Value::Int(decls)], 40_000_000_000)
        .expect("रचना");
    it.call(
        "शृङ्खलाॱनामसञ्चयः",
        vec![octets("पदविभाग".as_bytes())],
        40_000_000_000,
    )
    .expect("नामसञ्चयः");

    let table = match it.global("यन्त्रनामकोश") {
        Some(Value::Arena(a)) => a.borrow().clone(),
        other => panic!("`यन्त्रनामकोश` is an arena, not {other:?}"),
    };
    let t1_names: HashMap<i128, (String, String)> = table
        .iter()
        .skip(1)
        .filter_map(|e| match e {
            Value::Record(r) => {
                let r = r.borrow();
                let s = match r.get("संज्ञा") {
                    Some(Value::Int(n)) => *n,
                    _ => return None,
                };
                let txt = |k: &str| {
                    r.get(k)
                        .and_then(|v| v.octets())
                        .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
                        .unwrap_or_default()
                };
                Some((s, (txt("मण्डल"), txt("वृत्तिनाम"))))
            }
            _ => None,
        })
        .collect();

    println!("METRIC t1_lex_nametable_rust {}", rust_names.len());
    println!("METRIC t1_lex_nametable_t1 {}", t1_names.len());

    let mut rk: Vec<i128> = rust_names.keys().copied().collect();
    let mut tk: Vec<i128> = t1_names.keys().copied().collect();
    rk.sort_unstable();
    tk.sort_unstable();
    assert_eq!(
        tk, rk,
        "the two halves must name the SAME symbols; .t1 {tk:?} vs rust {rk:?}"
    );

    let mut wrong: Vec<String> = Vec::new();
    for (sym, want) in &rust_names {
        let got = &t1_names[sym];
        if got != want {
            wrong.push(format!("symbol {sym}: .t1 {got:?} vs rust {want:?}"));
        }
    }
    wrong.sort();
    assert!(
        wrong.is_empty(),
        "{} of {} entries disagree:\n  {}",
        wrong.len(),
        rust_names.len(),
        wrong.join("\n  ")
    );
}

/// **THE THIRD RUNG, AND THE FIRST CROSS-MODULE ONE.**
///
/// `ashtaka` and `lex` both declare zero qualified references, so neither could
/// test what happens when a routine CALLS another module's routine. `kosha.t1`
/// holds **18 code references to `अष्टकॱ`** (zero in margins) naming five
/// distinct callees — `अष्टकखण्डः`, `अष्टकदैर्घ्य`, `अष्टकपाठयोजनम्`,
/// `अष्टकयोजनम्`, `शून्याष्टकयोजनम्` — and every one is among the nine that
/// fact #1 named. **So a red here cannot be blamed on an unproven callee.**
///
/// `kosha` declares 9 things after the module: one `संरचना संज्ञा` at 1, four
/// globals at 2-5, four routines at 6-9.
///
/// **TWO OUTCOMES ARE REGISTERED AND BOTH ARE INFORMATIVE.** `नामसञ्चयः` walks
/// `मध्यरूप`'s arenas, which hold THIS SOURCE's declarations, and the emitter's
/// table is reset per source. So:
///
/// - **named 8, symbols {2..9}, emit SUCCEEDS** — a cross-module call does not
///   resolve through the name table at all, and the ladder's "1 dep" costs
///   nothing. The gap at 1 is the `संरचना`, as `lex` established.
/// - **named 8 and emit REFUSES with `UnnamedSymbol ४`** — the call site asks
///   for the callee's symbol and the table cannot answer, because the callee
///   belongs to another module and this walk never covered it. **That is the
///   cross-module gap, and the refused symbol names it.**
///
/// The second is what I expect. It is not a defect in the walk; it is the walk
/// meeting the first source whose emitter needs a symbol the source does not
/// declare — the same shape as the globals, one scope wider.
#[test]
// UN-IGNORED 2026-09-09, its own stated condition met — `उत्सर्जनम्` answers
// 41826 octets for `kosha.t1`. The superseded reason is kept because it is the
// record of what the third rung measured and what repaired it:
//
//   was — RED AT EMIT: ० octets, यन्त्रनिषेधभेद ४ on symbol ०, वृत्ति "", पर्व ०.
//   The naming was CORRECT (14 named, all four routines carrying नाम 14..17), so
//   the refusal was a CALL SITE asking for a callee symbol of ० at
//   yantrotsarjana.t1:1044, यन्त्राह्वानोत्सर्जनम्. NOT an unresolved call —
//   अनिर्णीताह्वानमस्ति was FALSE, so ir.t1 accepted the callee and left the
//   placeholder. Intra-module calls resolved (ashtaka calls its own routines
//   9/5/3 times and emitted 44785 octets), so it was specific to the 18
//   cross-module अष्टकॱ calls.
//
// **WHAT IT COSTS EVERY GATE, MEASURED — AND THE MEASUREMENT REFUSES.**
// Un-ignoring puts this in every whole-crate run, so the cost is owed to the
// other lanes. A/B on the file:
//
//     WITH kosha     11 passed, 16.47s
//     WITHOUT kosha  10 passed, 17.81s
//
// **The run is 1.34s FASTER with the extra test**, which is impossible as an
// effect and therefore says the variance exceeds the signal. `kosha` takes
// 4.42s run alone, but libtest runs a file's tests in PARALLEL and this one is
// not the critical path, so it overlaps. **The honest figure is "no measurable
// wall-clock cost", not a number** — a cargo A/B cannot measure a change of
// this size here, and quoting 4.42s as the cost would be quoting a serial
// figure for a parallel run.
//
// IF IT EVER REGRESSES, `t1_kosha_emitted_octets` is the metric: it went ० ->
// 41826 when `यन्त्रबाह्यनामसञ्चयः` landed, and it returns to ० if that pass stops
// naming externals.
//
// **THE REPAIR IS `यन्त्रबाह्यनामसञ्चयः`, AND IT IS NOT IN THIS FILE OR MINE.**
// It was `बाह्यनामसञ्चयः` when this margin was written; W-192 requires the
// `यन्त्र` module prefix and the riscv port ratchet enforces it. **The name and
// the margin move in the SAME commit that takes the rename** — a margin edited
// before the rename lands points at a name no tree has, which manufactures the
// tombstone rather than removing one.
// `yantrotsarjana.t1:1899` walks `मध्यरूपॱबाह्यसंज्ञासूचकाङ्क`, mints
// `बाह्यसंज्ञारम्भः + क्रमः` for each external not already in the table, and
// `:1920` calls it three lines into `यन्त्रमण्डलोत्सर्जनम्` — the module
// emitter's own entry, which every emitting path reaches, not just this driver.
// `शृङ्खलाॱउत्सर्जनम्` returns that routine, which is why this rung sees it.
fn the_driver_names_every_symbol_of_kosha() {
    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
        "shrinkhala.t1",
    ]);
    let src = source("kosha.t1");
    println!("METRIC t1_kosha_source_octets {}", src.len());

    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 40_000_000_000)
        .expect("पठनम्")
        .as_int()
        .unwrap_or(0);
    let ok = it
        .call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 40_000_000_000)
        .expect("निर्णयः");
    assert_eq!(
        ok,
        Value::Bool(true),
        "`अर्थ` refused kosha.t1 — and it resolves अष्टकॱ names, so this failing \
         means the CROSS-MODULE RESOLVE broke before emit was ever reached"
    );
    let routines = it
        .call("शृङ्खलाॱरचना", vec![Value::Int(decls)], 40_000_000_000)
        .expect("रचना")
        .as_int()
        .unwrap_or(0);
    // FIVE SINCE W-302: `प्रतिबिम्बध्वजाः` derives e_flags from the text.
    // SIX SINCE W-363: `दत्तपृष्ठाधारः` places the writable segment.
    assert_eq!(routines, 6, "kosha declares six routines");

    let named = it
        .call(
            "शृङ्खलाॱनामसञ्चयः",
            vec![octets("कोश".as_bytes())],
            40_000_000_000,
        )
        .expect("नामसञ्चयः")
        .as_int()
        .unwrap_or(0);

    let table = match it.global("यन्त्रनामकोश") {
        Some(Value::Arena(a)) => a.borrow().clone(),
        other => panic!("`यन्त्रनामकोश` is an arena, not {other:?}"),
    };
    let entries: Vec<(i128, String)> = table
        .iter()
        .skip(1)
        .filter_map(|e| match e {
            Value::Record(r) => {
                let r = r.borrow();
                let s = match r.get("संज्ञा") {
                    Some(Value::Int(n)) => *n,
                    _ => return None,
                };
                let n = r
                    .get("वृत्तिनाम")
                    .and_then(|v| v.octets())
                    .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())?;
                Some((s, n))
            }
            _ => None,
        })
        .collect();
    println!("METRIC t1_kosha_named {named}");
    println!("METRIC t1_kosha_table_entries {}", entries.len());
    let mut shown: Vec<String> = entries.iter().map(|(s, n)| format!("{s}:{n}")).collect();
    shown.sort();
    println!("  {}", shown.join(" "));

    // Six globals and five routines since W-302 (four and four before). The
    // gap at 1 is `संरचना संज्ञा` — types take a symbol and are named by neither
    // emitter, as `lex` established and the twin comparison confirmed from both
    // sides.
    // **14, AND I PREDICTED 8.** The miss was a declaration form my enumeration
    // did not list: `kosha.t1:20` is `सार्वजनिक गणना संज्ञाखण्ड`, an ENUM whose
    // six variants take symbols २-७. I had grepped for `चरः|वृत्तिः|संरचना` and
    // `गणना` is none of those — the third pattern of the night that was
    // well-formed and looked for the wrong words.
    //
    // THE RULE THE SYMBOL SPACE ACTUALLY FOLLOWS, which is better than the
    // declaration-index model it replaces:
    //
    //     १      संरचना संज्ञा        TYPE          not named
    //     २-७    गणना variants        VALUES        NAMED
    //     ८      गणना संज्ञाखण्ड       TYPE          not named
    //     ९      आयातः अष्टक          module ref    not named
    //     १०-१३  globals              VALUES        NAMED
    //     १४-१७  routines             VALUES        NAMED
    //
    // **VALUES ARE NAMED; TYPES AND MODULE REFERENCES ARE NOT.** That covers
    // `lex`'s two `संरचना` gaps without special-casing them, and it is a claim
    // about what the emitter needs a name FOR — a definition or a reference —
    // rather than about how declarations are counted.
    //
    // **FOURTEEN → EIGHTEEN, AND THE PIN NOW NAMES RATHER THAN NUMBERS
    // (2026-09-14).** Two 2026-09-13 landings moved this and neither re-pinned
    // it, so it was red on main for a day. (a) `ir.t1:546,549` mint two
    // MODULE-LESS globals the emitters must label — `रचनासूचकः`, the record
    // cursor, and `रचनाक्षेत्रम्`, the region it offsets into — at १००००००३ and
    // १००००००४ — ten million and change, where a module's own symbols are small
    // ordinals — and `वृद्धिवृत्तिसंज्ञा` at १००००००५ is the third of that family,
    // the growth routine `:408` now asserts. (b) the two TYPE names are now
    // in the table, which is what the model above said would never happen.
    //
    // So the assertion is on the NAMES. A bare `18` cannot tell a synthesised
    // symbol from a regression, and an id list re-taken blind is a rubber stamp:
    // these ids have renumbered twice in two days (+2 elementwise on 2026-09-13,
    // +1 again since) for reasons that were each correct. Names do not move when
    // the gather renumbers, and a name that appears or vanishes is exactly the
    // event worth failing on.
    let mut names: Vec<&str> = entries.iter().map(|(_, n)| n.as_str()).collect();
    names.sort_unstable();
    let mut want_names = vec![
        // the two types
        "संज्ञाखण्ड",
        "संज्ञा",
        // the six गणना variants
        "पाठ्यम्",
        "दत्तम्",
        "शून्यक्षेत्रम्",
        "अनिर्दिष्टम्",
        "शोधनपङ्क्तिखण्डः",
        "पाठ्यखण्डः",
        // the six globals — the two flag values arrived with W-302
        "भारस्थानम्",
        "शीर्षमानम्",
        "कार्यक्रमशीर्षमानम्",
        "यन्त्रभेदः",
        "प्लवध्वजाः",
        "सङ्कुचितप्लवध्वजाः",
        // W-363: the page the writable segment begins at
        "पृष्ठमानम्",
        // the five routines — `प्रतिबिम्बध्वजाः` arrived with W-302, and a
        // sixth, `दत्तपृष्ठाधारः`, with W-363
        "दत्तपृष्ठाधारः",
        "प्रतिबिम्बद्व्यष्टकम्",
        "प्रतिबिम्बचतुरष्टकम्",
        "प्रतिबिम्बाष्टाष्टकम्",
        "प्रतिबिम्बध्वजाः",
        "प्रतिबिम्बलेखनम्",
        // synthesised, module-less, owned by ir.t1 rather than by kosha.t1
        "रचनासूचकः",
        "रचनाक्षेत्रम्",
    ];
    want_names.sort_unstable();
    assert_eq!(
        names, want_names,
        "kosha's twenty-one declarations plus the two module-less synthesised globals"
    );
    assert_eq!(named as usize, entries.len(), "count agrees with the table");
    let mut keys: Vec<i128> = entries.iter().map(|(s, _)| *s).collect();
    keys.sort_unstable();
    // (2..=7) ∪ (10..=17)  ->  (4..=9) ∪ (12..=19)  ON 2026-09-13, the gather
    // ported to the `.t1` driver (`e58ec658`). +2 ELEMENTWISE, gaps 8,9 -> 10,11,
    // count still 14. MEASURED from this assertion's own failure — NOT computed
    // from lex's +2, though it equals it, and the reason it equals it is derived
    // rather than coincident: the gather mints a TYPE symbol per store type entry
    // first, and kosha declares exactly two types — one `संरचना` and the `गणना`
    // with six variants — which this message already names as "१ and ८ are
    // types". Two type declarations, two leading symbols, everything else +2.
    //
    // THREE POINTS ON ONE RELATION, NOT TWO MOVERS AND A CONTROL: lex declares
    // 2 types (two संरचना) and shifts +2; kosha declares 2 (one संरचना, one
    // गणना) and shifts +2; `ashtaka` declares 0 and shifts 0. lex alone could
    // not separate "shift == संरचना count" from "shift == type declarations" —
    // both predict 2 — and kosha is the case where they diverge. The control
    // does not merely fail to move; it moves by the amount the model predicts.
    // THE IDS ARE CHECKED STRUCTURALLY, NOT LISTED. Two and only two come from
    // the synthesis family, and they are the two names ir.t1 owns; the other
    // sixteen are kosha's own, distinct, and small. NOT RE-DERIVED: which
    // ordinals the gather skips. Today they are 1, 4, 11 and 12, and the model
    // in the block above — "values are named, types and module references are
    // not" — no longer explains them, since the types are named and the import
    // is not. Naming the gaps needs a reading of the gather, not a re-take of
    // the list they produce, so this assertion deliberately does not pin them.
    println!("  ids {keys:?}");
    let (synth, own): (Vec<i128>, Vec<i128>) = keys.iter().partition(|k| **k >= 10_000_000);
    assert_eq!(
        synth,
        vec![10_000_003, 10_000_004],
        "the record cursor and its region, ir.t1:546,549"
    );
    let mut distinct = own.clone();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        21,
        "kosha's own symbols are twenty-one, distinct (nineteen before W-363, sixteen before W-302)"
    );

    // **THE CROSS-MODULE QUESTION, PUT TO THE EMITTER.**
    let text = it
        .call("शृङ्खलाॱउत्सर्जनम्", vec![], 40_000_000_000)
        .expect("उत्सर्जनम् runs")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    println!("METRIC t1_kosha_emitted_octets {}", text.len());
    // THE PROBE BEHIND THE DIAGNOSIS. `ir.t1:1197-1204` sets this flag and
    // returns `मूल्याङ्कनम् ०` when a call's callee is not accepted — the call is
    // DROPPED, not lowered. If it is true here, the refusal downstream is a
    // consequence of a call the IR never built, and the gap is in lowering
    // rather than in naming.
    println!(
        "  अनिर्णीताह्वानमस्ति = {:?}   चिह्नकाङ्क = {:?}",
        it.global("अनिर्णीताह्वानमस्ति"),
        it.global("अनिर्णीताह्वानचिह्नकाङ्क")
    );
    // EVERY ROUTINE IN THE IR ARENA AND THE SYMBOL WRITTEN INTO IT. `:408`
    // raises with `रिक्तम्` as the routine and the looked-up symbol as its
    // value, and `यन्त्रचिह्नपरीक्षा` walks every routine looking each up — so a
    // routine whose `नाम` is still ० would produce exactly the observed
    // signature. This says whether one is, rather than inferring it.
    let cursor = it
        .global("वृत्तिसूचकाङ्क")
        .and_then(Value::as_int)
        .unwrap_or(-1);
    println!("  मध्यरूपॱवृत्तिसूचकाङ्क = {cursor}");
    if let Some(Value::Arena(a)) = it.global("वृत्तिकोश") {
        for (i, e) in a.borrow().iter().enumerate().take(12) {
            if let Value::Record(r) = e {
                println!("    वृत्ति[{i}] नाम = {:?}", r.borrow().get("नाम"));
            }
        }
    }

    // WHICH ROUTINE'S EMISSION REFUSED. The recorder keeps the routine label in
    // `यन्त्रनिषेधवृत्ति`; printing it says whether the refusal is in one of
    // kosha's four routines or somewhere with no routine at all.
    let who = it
        .global("यन्त्रनिषेधवृत्ति")
        .and_then(|v| v.octets())
        .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
        .unwrap_or_else(|| "<none>".into());
    println!(
        "  यन्त्रनिषेधवृत्ति = {who:?}  लक्ष्य = {:?}",
        it.global("यन्त्रनिषेधलक्ष्य")
    );
    let refused = it.global("यन्त्रनिषेधसंख्या");
    assert!(
        !text.is_empty(),
        "the emitter refused kosha.t1 — भेद {:?}, symbol {:?}, block {:?}. \
         IF THAT SYMBOL IS ABOVE ९ IT IS NOT KOSHA'S: the call site is asking for \
         a callee in अष्टक, which this source does not declare and this walk never \
         covered. That is the cross-module gap, not a defect in the walk.",
        it.global("यन्त्रनिषेधभेद"),
        refused,
        it.global("यन्त्रनिषेधपर्व"),
    );
}

/// **W-280 — `मण्डलसङ्कलनम्` MUST DISTINGUISH "COMPILED TO NOTHING" FROM
/// "REFUSED", FOR EACH EXIT SEPARATELY.**
///
/// It has six ways out and answered `रिक्तम्` from five of them, recording
/// nothing. `kosha_end_to_end.rs:838` wrote this down first — *"the whole-call
/// empty answer names nothing"* — and worked around it.
///
/// **THE INSTANCE IS `वास्तु`.** `ast.t1` and `vastu.t1` declare one module
/// between them: 45 globals, 8 types, **zero routines**. So `वृत्तयः समम् ०`
/// fires on a module that is entirely data, and that module is in `CHAIN` with
/// seven loaded importers. It is the next rung of the ladder and it is blocked
/// on exactly this.
///
/// **THIS COUNTS DISTINGUISHABLE OUTCOMES RATHER THAN ASSERTING A LITERAL.** A
/// fix that separated *emitted nothing* from the other four lumped together
/// would let `वास्तु` through and leave the defect for the next module, and an
/// assertion on one value could not see the difference.
///
/// **AND THE POSITIVE CONTROL IS A REAL REFUSAL.** A discriminator that reports
/// "distinguishable" for everything is indistinguishable from one that reports
/// it for nothing, so a source `अर्थ` genuinely refuses must land on its own
/// value — not merely somewhere.
/// **FIRST RUN, 2026-09-09 — the discriminator discriminates.**
///
/// ```text
/// t1_w280_emitted            ०    success
/// t1_w280_no_decls           १
/// t1_w280_no_routines        ३    वास्तु — compiled to nothing, legitimately
/// t1_w280_refused            २    the control, अर्थ genuinely refused it
/// t1_w280_distinct_outcomes  ४
/// ```
///
/// **`no_routines` ≠ `refused` is the whole of the row.** Before this, `वास्तु`
/// and a refused source both answered `रिक्तम्` and nothing else. No input left
/// `सङ्कलनविरामभेद` at the sentinel ६.
///
/// **AND IT DOES NOT UNBLOCK `वास्तु`.** `वृत्तयः समम् ०` still returns early;
/// emitting a routine-less module is a second capability. This makes the caller
/// able to tell WHY, not the module able to compile.
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn the_front_half_names_which_exit_it_took() {
    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
        "shrinkhala.t1",
    ]);

    let kind = |it: &mut Interpreter, src: &str, module: &str| -> i128 {
        it.call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![octets(src.as_bytes()), octets(module.as_bytes())],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम् runs");
        match it.global("सङ्कलनविरामभेद") {
            Some(Value::Int(n)) => *n,
            other => panic!("`सङ्कलनविरामभेद` is an int, not {other:?}"),
        }
    };

    // THE FOUR INPUTS I CAN ACTUALLY PRODUCE, AND THE TWO EXITS THAT ARE
    // UNDRIVEN — stated here rather than left as an apparent coverage gap,
    // because the next reader will otherwise delete them or invent a fake
    // input for them.
    //
    // **`सङ्कलननामवैषम्यभेद` (४) and `सङ्कलनानामभेद` (५) HAVE NO CONSTRUCTIBLE
    // INPUT, AND WHY IS ITSELF A FINDING.** Both are guards over the name pass:
    //
    //     शृङ्खला:389   यदि प्रविष्टयः असमम् नामानि     ४
    //     शृङ्खला:393   यदि नामानि समम् ०               ५
    //
    // **THE PARAGRAPH THAT STOOD HERE WAS FALSE, AND IT NAMED THE MECHANISM OF
    // THE BUG WHILE DECLARING IT UNREACHABLE.** It read: "`प्रविष्टयः` is the
    // emitter table's cursor DELTA across the same call. For ४ to fire, the
    // appender would have to accept fewer entries than it was asked for … That
    // is repaired, so no source reaches it today."
    //
    // Exit ४ is exactly where `मण्डलसङ्कलनम्` was refusing `वास्तु`, and it was
    // refusing `ast.t1` too. Measured:
    //
    //     ashtaka.t1  before ०   after १५  delta  १५   right BY ACCIDENT — ran first
    //     ast.t1      before १५  after ४५  delta  ३०   45 of 45 were named
    //     vastu.t1    before ४५  after  ६  delta −३९   6 of 6 were named
    //
    // The error was not the conclusion but the enumeration: it considered ONE
    // way for `प्रविष्टयः` to fall short — the appender dropping entries — and
    // not the one that was live, the measured interval CONTAINING the reset.
    // `नामसञ्चयः` opens with `यन्त्रनामारम्भः`, which zeroes the cursor, so the
    // subtraction spanned it. "No constructible input" was false for two of the
    // three corpus modules the entire time, and the margin's own words "cursor
    // DELTA" are where the answer was.
    //
    // ५ IS STILL UNDRIVEN and that part stands: a source whose routine names all
    // have zero length, which the parser does not produce.
    //
    // **AND HERE IS WHY THAT PARAGRAPH WAS BELIEVABLE: THE CORPUS CENSUS CANNOT
    // EXECUTE THIS PATH.** `paradigm_encode.rs:1014` `write_names_into_t1` calls
    // `यन्त्रनामारम्भः` and then feeds the T1 emitter's name table symbol by
    // symbol FROM RUST's `module.names`. So `paradigm_encode_runs 17` means the
    // `.t1` stages work WHEN RUST HANDS THEM THE NAME TABLE — it never calls
    // `नामसञ्चयः`. A guard on that pass's output could refuse two of three corpus
    // modules while every census ran green. The margin above was not careless; it
    // was measuring the other ladder.
    //
    // WHAT COVERS THIS PATH, STATED EXACTLY, BECAUSE "SOLE INSTRUMENT" IS TOO
    // STRONG AND A LOOSE CLAIM HERE WOULD BE THE FOURTH FALSE ONE IN THIS FILE:
    // `नामसञ्चयः` has TWO callers — this file and `kosha_end_to_end.rs:903`. But
    // that one asserts `named > 0`, and `नामसञ्चयः` returns what it ATTEMPTED,
    // which was 45 and 6 while the guard was refusing both. A zero-check cannot
    // see a mismatch — `शृङ्खला`'s own margin says so in as many words. So it
    // EXERCISES the path and cannot FALSIFY anything on it, and the rungs in this
    // file are the only assertions that could. A future reader must not take a
    // green census as coverage here; that assumption is what let this run.
    let emitted = kind(&mut it, &source("ashtaka.t1"), "अष्टक");
    let no_decls = kind(&mut it, "", "क");
    let no_routines = kind(&mut it, &source("vastu.t1"), "वास्तु");
    // POSITIVE CONTROL — a real refusal. `अज्ञातनाम` is declared nowhere, so
    // `अर्थ` refuses this source at resolve.
    let refused = kind(
        &mut it,
        "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    प्रत्यागमनम् अज्ञातनाम ।\nइति\n",
        "क",
    );

    println!("METRIC t1_w280_emitted {emitted}");
    println!("METRIC t1_w280_no_decls {no_decls}");
    println!("METRIC t1_w280_no_routines {no_routines}");
    println!("METRIC t1_w280_refused {refused}");

    // **THE SENTINEL ASSERTION, AND IT IS THE ONE THAT GENERALISES.**
    // `सङ्कलनविरामभेद` resets on entry to ६ — "no exit was recorded" — so a
    // return that forgets to set it reports a value no caller expects rather
    // than ०, which every caller reads as success. The four assertions below
    // are about four hand-chosen inputs; **this one holds for inputs nobody
    // has thought of yet**, including any future early return added by someone
    // who has not read the reset.
    //
    // It also separates UNDRIVEN from DRIVEN-BUT-UNRECORDED. The two exits
    // this test cannot construct are undriven today; if a later input ever
    // reaches them and the code is not set, this fires instead of reporting
    // success.
    let sentinel: i128 = 6;
    for (name, v) in [
        ("emitted", emitted),
        ("no_decls", no_decls),
        ("no_routines", no_routines),
        ("refused", refused),
    ] {
        assert_ne!(
            v, sentinel,
            "{name} left सङ्कलनविरामभेद at the sentinel — a return recorded no \
             exit, which is the defect W-280 exists for, one level up"
        );
    }

    let observed = [emitted, no_decls, no_routines, refused];
    let mut distinct: Vec<i128> = observed.to_vec();
    distinct.sort_unstable();
    distinct.dedup();
    println!("METRIC t1_w280_distinct_outcomes {}", distinct.len());

    // THE COUNT IS STILL THE ASSERTION, AND IT IS NOW THREE — BECAUSE THE
    // CAPABILITY LANDED, NOT BECAUSE SOMETHING REGRESSED.
    //
    // This test used to assert four distinct answers over these four inputs,
    // with `no_routines` as its own outcome. That encoded "`वास्तु` compiles to
    // nothing, legitimately" as an INVARIANT — and stopping that being true is
    // the entire purpose of the unit that changed it. `वास्तु` now emits, so
    // `no_routines` lands on `emitted` and the distinct count falls 4 → 3.
    //
    // A test whose expected value moves when a capability lands is doing its
    // job; the danger is only in changing it without saying which of the two
    // happened. It is the second: `vastu.t1` builds a real object with 6 of 6
    // globals named, and `ast.t1` 45 of 45.
    assert_eq!(
        distinct.len(),
        3,
        "three exits over four inputs — `वास्तु` now shares `emitted`, got \
         {observed:?}"
    );

    // THE CAPABILITY, ASSERTED IN THE PLACE THAT USED TO DENY IT. The old line
    // here was `assert_ne!(emitted, no_routines, "success and compiled-to-nothing
    // must differ")`. Its replacement is the exact inversion, and that inversion
    // IS the unit.
    assert_eq!(
        emitted, no_routines,
        "a routine-less module must now emit like any other — this is the \
         inversion of the assertion that stood here"
    );

    // UNCHANGED AND STILL LOAD-BEARING: a refusal must never look like either.
    assert_ne!(
        no_routines, refused,
        "a routine-less module and a refused source landed on the same value — \
         which is the whole of W-280"
    );
}

/// **MEASURING THE PREMISE BEFORE TOUCHING A GUARD — two repairs or one?**
///
/// `मण्डलसङ्कलनम्` returns empty at `यदि वृत्तयः समम् ०`, so `वास्तु` never
/// reaches the object builder and narrowing that builder alone would change
/// nothing observable. The upper guard has to go first — but it may be
/// REMOVABLE rather than narrowable, because `यदि नामानि समम् ०` two steps
/// later already catches a genuinely empty module.
///
/// **That rests on `नामानि` being non-zero for a module with NO ROUTINES**, and
/// this measures it rather than assuming it. The stages are called
/// individually, which reaches past the guard without changing it.
#[test]
#[ignore = "measurement, not a guard: reports what वास्तु's stages answer"]
fn what_vastu_answers_stage_by_stage() {
    let mut it = load(&[
        "lex.t1",
        "ast.t1",
        "parse.t1",
        "sanskrit_text.t1",
        "sanchaya.t1",
        "artha.t1",
        "vastu.t1",
        "ir.t1",
        "utsarjana.t1",
        "yantrotsarjana.t1",
        "shrinkhala.t1",
    ]);
    for (file, module) in [("ast.t1", "वास्तु"), ("vastu.t1", "वास्तु")] {
        let src = source(file);
        let decls = it
            .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 40_000_000_000)
            .expect("पठनम्")
            .as_int()
            .unwrap_or(-1);
        let ok = it
            .call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 40_000_000_000)
            .expect("निर्णयः");
        let routines = it
            .call("शृङ्खलाॱरचना", vec![Value::Int(decls)], 40_000_000_000)
            .expect("रचना")
            .as_int()
            .unwrap_or(-1);
        let named = it
            .call(
                "शृङ्खलाॱनामसञ्चयः",
                vec![octets(module.as_bytes())],
                40_000_000_000,
            )
            .expect("नामसञ्चयः")
            .as_int()
            .unwrap_or(-1);
        let globals = it
            .global("वैश्विकसञ्चयसूचकाङ्क")
            .and_then(Value::as_int)
            .unwrap_or(-1);
        println!("METRIC vastu_{file}_decls {decls}");
        println!("METRIC vastu_{file}_resolved {ok:?}");
        println!("METRIC vastu_{file}_routines {routines}");
        println!("METRIC vastu_{file}_named {named}");
        println!("METRIC vastu_{file}_ir_globals {globals}");
    }
}

/// **W-279 — A ROUTINE-LESS MODULE EMITS, AND THE OBJECT BUILDER NAMES ITS EXIT.**
///
/// Two repairs in two layers, and the order was forced: `मण्डलसङ्कलनम्`'s routine
/// guard returned empty before `पाठवस्तुरचना` was ever called, so narrowing the
/// builder alone would have changed nothing observable.
///
/// **The upper guard was REMOVED, not narrowed** — its own margin said it
/// deferred a capability rather than guarding a defect. **The lower one was
/// NARROWED, not removed** — its margin records a failure that was once live,
/// and a module of pure data shares that failure's signature on the instruction
/// count alone. It now asks whether ANYTHING was built.
///
/// # The four exits, and which this drives
///
/// ```text
/// ० सिद्ध      an object was built          DRIVEN — ashtaka, the control
/// १ वाक्य      no statements                DRIVEN — empty text
/// २ रिक्त      statements, nothing built    NOT DRIVEN — see below
/// ३ अष्टक      `वस्तुसङ्केतनम्` answered nil  TWO raise paths — see below
/// ४ नारब्ध     no exit recorded             a defect; asserted never returned
/// ```
///
/// **THIS PARAGRAPH WAS WRONG AND IS KEPT BECAUSE IT WAS LOAD-BEARING.** It read:
/// *"Exit ३ cannot be driven by any source. `लक्ष्यवस्तुसङ्केतनम्` answers `शून्यम्`
/// only when `विन्याससाम्यम्` finds the two-pass layout disagreeing."* **Refuted by
/// measurement, 2026-09-14**, on the compiled compiler through the output channel.
///
/// `लक्ष्यवस्तुसङ्केतनम्` has exactly one `प्रत्यागमनम् शून्यम्` — the E22 branch, which
/// is what made the claim look airtight — but it has **two ways to answer nil at
/// its caller**. Its return type is `दोषयुक्त अङ्कः अन्तः अ८`, and its last line
/// hands back `उत्सर्जनक्रमः`'s result; `पाठवस्तुरचना` binds that as a plain run and
/// tests `यदि अष्टकाः समम् शून्यम्`, so a FAULT-CARRYING answer sets
/// `वस्तुरचनाष्टकभेद` by the same line with no `शून्यम्` anywhere on that path.
///
/// Measured natively on the one-global exemplar rung 73 drives to `१००३`: both
/// layout passes emitted through `अष्टकॱमुद्रणम्`, **byte-identical to each other
/// and to the interpreted reference**, and `विन्याससाम्यम्`'s own answer emitted
/// beside them as **सत्यम्**. E22 did not fire. The refusal is `उत्सर्जनक्रमः`.
///
/// **AND ONLY THE E22 BRANCH WRITES `सङ्केतनदोष`.** The other route sets the exit
/// and leaves whatever that record already held — so a reader who inspected it at
/// the refusal and found `० २२ ० ०` would have read a STALE record as
/// confirmation of this paragraph. Rungs 74, 76 and 77 spent one image each
/// bisecting toward a disagreement that is not there.
///
/// The exit constant is still named `अष्टकभेद` and its margin still says
/// "encoding refused — see `सङ्केतनदोष`"; both describe ONE of its two raise
/// paths. Same shape as W-280's `नामवैषम्य` and `अनाम`, which is what the old
/// sentence was reaching for and got right about the wrong branch.
///
/// **Exit २ is the splitter defect** — statements parsed, nothing built — which
/// is what the guard's margin records and which no present source reproduces.
#[test]
#[ignore = "legacy (owner's ruling 2026-09-13: legacy runs are thrown away; the narrowed census T1_CORPUS=<names> is the landing gate and the hourly deep gate with T1_FULL_CENSUS=1 walks the corpus)"]
fn the_object_builder_names_which_exit_it_took() {
    // **THE SHIPPED MANIFEST, NOT A HAND LIST.** A first version named fourteen
    // modules and missed `ashtaka.t1`; `सङ्कलनारम्भः` reaches
    // `वाक्यविभागॱआरम्भः` which reaches `अष्टकॱअष्टकारम्भः`, and the call died
    // with "`अष्टकॱअष्टकारम्भः` is not a name in scope". **This file's own margin
    // four tests above says a hand list is exactly where a module goes missing.**
    // I wrote that warning and then did not follow it.
    let mut it = Interpreter::load(CHAIN, &spec_root()).expect("the chain loads");

    // **RESET FIRST, AS THE DRIVER DOES.** `मण्डलानिप्रतिबिम्बम्` calls
    // `सङ्कलनारम्भः` before every `पाठवस्तुरचना` — `पुनरारम्भः` in the loop and
    // `तत्परम्` for the startup. Calling the builder cold leaves `वाक्यविभाग`
    // holding the previous stage's state and it faults with
    // `no record carries a member नाम`. A routine called outside its sequence
    // is not a defect in the routine.
    let build = |it: &mut Interpreter, text: &str| -> (i128, bool) {
        it.call("शृङ्खलाॱसङ्कलनारम्भः", vec![], 8_000_000_000)
            .expect("सङ्कलनारम्भः runs");
        let obj = it
            .call(
                "शृङ्खलाॱपाठवस्तुरचना",
                vec![octets(text.as_bytes())],
                40_000_000_000,
            )
            .expect("पाठवस्तुरचना runs");
        let kind = match it.global("वस्तुरचनाविरामभेद") {
            Some(Value::Int(n)) => *n,
            other => panic!("`वस्तुरचनाविरामभेद` is an int, not {other:?}"),
        };
        (kind, !matches!(obj, Value::Nil))
    };

    // THE CONTROL FIRST — a real module's text must build a real object.
    let ashtaka_text = it
        .call(
            "शृङ्खलाॱमण्डलसङ्कलनम्",
            vec![
                octets(source("ashtaka.t1").as_bytes()),
                octets("अष्टक".as_bytes()),
            ],
            80_000_000_000,
        )
        .expect("मण्डलसङ्कलनम्")
        .octets()
        .map(|o| o.as_slice().to_vec())
        .unwrap_or_default();
    assert!(!ashtaka_text.is_empty(), "the control must produce text");
    let (built_kind, built_obj) = build(&mut it, &String::from_utf8_lossy(&ashtaka_text));

    let (empty_kind, empty_obj) = build(&mut it, "");

    // **THE ROUTINE-LESS MODULE — THE CASE THE UNIT IS ACTUALLY FOR.** A first
    // version of this test asserted only `""` against `ashtaka.t1` and passed.
    // Empty text taking the "no statements" exit is trivially true and says
    // NOTHING about a module of pure data, which is the whole capability. Both
    // files below declare module `वास्तु` and hold zero routines:
    //   ast.t1    49 declarations · 45 globals · 0 routines
    //   vastu.t1   7 declarations ·  6 globals · 0 routines
    let dataonly: Vec<(&str, i128, bool)> = ["ast.t1", "vastu.t1"]
        .iter()
        .map(|f| {
            let text = it
                .call(
                    "शृङ्खलाॱमण्डलसङ्कलनम्",
                    vec![octets(source(f).as_bytes()), octets("वास्तु".as_bytes())],
                    80_000_000_000,
                )
                .expect("मण्डलसङ्कलनम्")
                .octets()
                .map(|o| o.as_slice().to_vec())
                .unwrap_or_default();
            // **ASK THE DISCRIMINATOR, DO NOT PANIC BLIND.** A first version
            // asserted `!text.is_empty()` and died with "ast.t1 must produce
            // text" — which says the front half refused and NOT why. W-280 put
            // six exits on `मण्डलसङ्कलनम्` for this question; reading it costs
            // one call and turns a bare failure into a named one.
            if text.is_empty() {
                // **READ THE GLOBAL, DO NOT CALL IT — AND DO NOT DEFAULT.** A
                // first version called it like a routine and fell back to `-1`.
                // The call could never work (`सङ्कलनविरामभेद` is a `सार्वजनिक
                // चरः`), and the fallback turned that failure into a plausible
                // number: the run printed `-1` twice, satisfied `k != 4`, and
                // PASSED carrying no information. W-280's own reader, forty
                // lines up, panics instead — which is why it cannot do this.
                let why = match it.global("सङ्कलनविरामभेद") {
                    Some(Value::Int(n)) => n,
                    other => panic!("`सङ्कलनविरामभेद` is an int, not {other:?}"),
                };
                // **THREE NUMBERS, NOT ONE.** Exit ४ says only "the two counts
                // differ"; whether the appender lost entries or the guard counts
                // a population that was never nameable is decided by the pair.
                let g = |n: &str| match it.global(n) {
                    Some(Value::Int(v)) => *v,
                    other => panic!("`{n}` is an int, not {other:?}"),
                };
                let (tried, wrote) = (g("सङ्कलननामप्रयासाः"), g("सङ्कलननामप्रविष्टयः"));
                let stem = f.replace(".t1", "");
                println!("METRIC t1_w279_fronthalf_refused_{stem} {why}");
                println!("METRIC t1_w279_names_tried_{stem} {tried}");
                println!("METRIC t1_w279_names_wrote_{stem} {wrote}");
                println!("  {f} → NO TEXT · exit {why} · tried {tried} · wrote {wrote}");
                return (*f, -1i128, false);
            }
            let (k, o) = build(&mut it, &String::from_utf8_lossy(&text));
            (*f, k, o)
        })
        .collect();

    println!("METRIC t1_w279_built_kind {built_kind}");
    println!("METRIC t1_w279_empty_kind {empty_kind}");
    println!("  built object? {built_obj} · empty object? {empty_obj}");
    for (f, k, o) in &dataonly {
        println!("METRIC t1_w279_dataonly_kind_{} {k}", f.replace(".t1", ""));
        println!("  {f} → exit {k} · object? {o}");
    }

    // NO INPUT MAY LEAVE THE SENTINEL — the assertion that holds for inputs
    // nobody has chosen yet, including a future early return added by someone
    // who has not read the reset.
    for (n, k) in [("built", built_kind), ("empty", empty_kind)]
        .into_iter()
        .chain(dataonly.iter().map(|(f, k, _)| (*f, *k)))
    {
        assert_ne!(k, 4, "{n} left वस्तुरचनाविरामभेद at the sentinel");
    }
    assert_eq!(built_kind, 0, "a real module's text builds an object");
    assert!(built_obj, "and the object is not nil");
    assert_eq!(empty_kind, 1, "empty text parses to no statements");
    assert!(!empty_obj, "and answers nil");
    assert_ne!(built_kind, empty_kind, "the two exits must differ");

    // **THE CAPABILITY ITSELF: A ROUTINE-LESS MODULE EMITS AND BUILDS.** Both
    // files hold zero routines, and both now reach `वस्तुरचनासिद्धभेद` with a
    // real object. They pass the narrowed guard for the stated reason and not by
    // luck: `आज्ञासूचकाङ्क` is ० for a module of pure data, and it is
    // `वैश्विकसंख्यानम्` — 45 and 6 — that carries them through. An instruction
    // count alone could not have told them from a genuinely empty build.
    for (f, k, o) in &dataonly {
        assert_eq!(*k, 0, "{f} is data-only and must still build an object");
        assert!(o, "{f} built, so its object must not be nil");
    }

    // **THE ARENA ASSERTIONS — ONE CONTROL AND ONE TRIPWIRE, NOT TWO CONTROLS.**
    // This heading read "THE ARENA CONTROL" until 2026-09-12 and over-claimed for
    // the pair: the body below already distinguishes them, and the heading did
    // not. `यन्त्राधिकरणसीमा` at 160 → 160 is a control; `यन्त्रनिक्षेपाः` at
    // 0 → 0 is a tripwire that earns nothing until something is shown to move it.
    //
    // They were required of the unit in discussion and grep found them in neither
    // test file: a rule with no mechanism behind it. They are written here.
    //
    // Its second purpose is what makes it worth the lines. The name table proved
    // to reset ON ENTRY to `नामसञ्चयः`, which is what made reading the absolute
    // cursor correct. `यन्त्राधिकरणसीमा` and `यन्त्रनिक्षेपाः`
    // (`yantrotsarjana.t1:581-582`) carry their own cursors under their own
    // discipline. If either drifts across a `वास्तु` emit, the same
    // delta-across-a-reset defect is present in an arena where it would NOT go
    // negative — it would read as a plausible partial loss, which is precisely
    // the failure that hid here for `ast.t1` at +30.
    //
    // MEASURED, NOT ASSUMED: printed before asserted, because a value I have not
    // seen is not a control.
    let arena = |it: &Interpreter| {
        let r = |n: &str| match it.global(n) {
            Some(Value::Int(v)) => *v,
            other => panic!("`{n}` is an int, not {other:?}"),
        };
        (r("यन्त्राधिकरणसीमा"), r("यन्त्रनिक्षेपाः"))
    };
    let before_arena = arena(&it);
    let _ = build(&mut it, &String::from_utf8_lossy(&ashtaka_text));
    let after_arena = arena(&it);
    println!("METRIC t1_w279_arena_sima_before {}", before_arena.0);
    println!("METRIC t1_w279_arena_sima_after {}", after_arena.0);
    println!("METRIC t1_w279_arena_nikshepa_before {}", before_arena.1);
    println!("METRIC t1_w279_arena_nikshepa_after {}", after_arena.1);
    println!("  arenas {before_arena:?} → {after_arena:?}");

    // MEASURED (160, 0) → (160, 0). Both hold, and they are NOT worth the same.
    //
    // **`यन्त्राधिकरणसीमा` IS A REAL CONTROL: 160 → 160.** It carries a non-zero
    // value across the emit and does not move, so it distinguishes "reset
    // correctly" from "drifted" — the reset-on-entry pattern proven for the name
    // table does not extend to it, and it does not need to.
    //
    // **`यन्त्रनिक्षेपाः` IS NOT A CONTROL AND MUST NOT BE READ AS ONE: 0 → 0.**
    // It cannot tell "correctly unchanged" from "nothing on this path ever
    // touches it", because both give 0 → 0. It is asserted because a change from
    // 0 would be worth knowing about, but it earns nothing today and no
    // conclusion may rest on it. It becomes a control the moment something is
    // shown to move it — that positive control does not exist yet, and until it
    // does this line is a tripwire, not evidence.
    assert_eq!(
        before_arena.0, after_arena.0,
        "यन्त्राधिकरणसीमा moved across a वास्तु emit — the delta-across-a-reset \
         defect would be present here too, and here it would NOT go negative"
    );
    assert_eq!(
        before_arena.1, after_arena.1,
        "यन्त्रनिक्षेपाः moved across a वास्तु emit"
    );
}
