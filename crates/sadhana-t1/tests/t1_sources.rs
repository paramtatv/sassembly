//! The T1 sources of the self-hosting assembler must lex and parse — task `D-002a`.
//!
//! # Why this test exists
//!
//! Before it, **nothing in the tree read `crates/sadhana-t1/src/*.t1` at all.**
//! `cargo test -p sadhana-t1` ran 0 tests; the crate's `lib.rs` was a comment; the
//! only reader of a `.t1` file anywhere was `src/bin/t1_parse.rs`, which no script,
//! test or CI step invoked. So a port could be written, committed and counted with
//! no mechanism able to say whether it was even well-formed text.
//!
//! It was not. `encode.t1` — the file `D-002a` was closed on — **did not lex**:
//! `tools/port_encode.py` transliterated `Option<` letter by letter and left the
//! `<`, which is outside the doc 15 repertoire, so the assembler's own lexer
//! refused the file at line 26. Ten sibling files parsed and one did not, and the
//! difference was invisible because no one was looking.
//!
//! # What this asserts, and what it does not
//!
//! It asserts what the tree can currently check: every `.t1` source lexes inside
//! the doc 15 repertoire and parses with `sadhana::t1::parse`. That parser is
//! deliberately permissive — `B-080` froze declarations and deferred expressions
//! and operators — so **passing is not evidence that a body is correct**, only
//! that it is inside the language. The stronger check arrives with the type
//! checker (`B-082`); this is the floor, not the ceiling.
//!
//! The second test is the anti-gutting one: it names the symbols `encode.rs`
//! defines and requires each to be present in `encode.t1`. Without it, deleting
//! the port would leave the first test green over an empty file.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn crate_src() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// The directory an embed's name resolves against — `spec/` at the repo root.
///
/// Derived from `CARGO_MANIFEST_DIR` rather than from the working directory,
/// because `cargo test -p sadhana-t1` and the workspace run set that differently
/// and an embed that resolves in one and not the other is a test that reports
/// the harness rather than the port.
fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("spec")
}

/// Every `.t1` source of this crate, sorted.
fn t1_sources() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(crate_src())
        .expect("crates/sadhana-t1/src exists")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    v.sort();
    v
}

#[test]
fn the_t1_port_exists_and_this_test_is_not_vacuous() {
    // A test that survives the disappearance of its own evidence proves nothing.
    let files = t1_sources();
    println!("METRIC sadhana_t1_source_files {}", files.len());
    assert!(
        files.len() >= 10,
        "only {} .t1 sources found; the port is the evidence this file rests on",
        files.len()
    );
}

#[test]
fn every_t1_source_lexes_and_parses() {
    let files = t1_sources();
    assert!(!files.is_empty(), "no .t1 sources — nothing asserted");

    let mut failures: Vec<String> = Vec::new();
    let mut declarations = 0usize;

    for f in &files {
        let name = f
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let Ok(text) = std::fs::read_to_string(f) else {
            failures.push(format!("{name}: is not UTF-8 text (see B-109)"));
            continue;
        };
        // `lex_t1`, NOT `lex` — these are T1 sources. The two differ only in
        // what a string is: `lex_t1` takes `उक्तम् … इति` as one `Kind::Str`
        // (ADR-0017) where `lex` leaves it as words, because T0's frozen
        // grammar spells a string as a phrase and `crate::parse` still tests
        // for the word `उक्तम्`.
        //
        // IT WAS `lex`, AND THAT MADE THE EMBED PASS BELOW READ A STRING'S
        // CONTENTS AS CODE. `anita::resolve` guards with
        // `!matches!(kind, Kind::Str)` and its :313 note says a string "is
        // already whole … so `उक्तम् समावेशः इति` never reaches this test as
        // the word `समावेशः`" — TRUE OF `lex_t1` AND FALSE OF WHAT IT WAS
        // BEING FED. A `.t1` file that merely MENTIONED `समावेशः` in a string
        // was refused with "must be followed by `आरभ्य`". No source had ever
        // written that literal, so the mismatch had never been exercised.
        match sadhana::lex::lex_t1(&text) {
            Err(errs) => {
                let first = errs.first().map(|e| format!("{e:?}")).unwrap_or_default();
                failures.push(format!("{name}: lex: {first}"));
            }
            // THE EMBED IS RESOLVED HERE, AND THE PARSER IS EXACTLY WHY IT
            // HAS TO BE. `समावेशः आरभ्य <name> समाप्तम्` (ADR-0019) is four
            // tokens that `anita::resolve` folds into one `Kind::Str`, and
            // `t1::parse` is permissive enough (`B-080` deferred expressions)
            // that it accepts the four UNFOLDED — measured, not assumed:
            // `cargo run -p sadhana --bin t1_parse -- .../encode.t1` reports
            // "Successfully parsed" with the resolution never run. So the
            // parser cannot tell `कोष्ठकोशः` from a name no registry carries,
            // and without this step a `.t1` source could embed a table that
            // does not exist and this file would call it well-formed.
            //
            // NOT VACUOUS, and checked the way a ratchet is: renaming the one
            // embed in `encode.t1` to `कोष्ठकोशाः` fails this test with
            // "is not a table this program may bring in", where before the
            // step it passed. `encode.t1`'s register reader (`D-002a2a`) is
            // the first `.t1` source in this tree to use an embed at all;
            // every other source passes through untouched, which is why the
            // step can run unconditionally over all fifteen.
            Ok(tokens) => match sadhana::t1::anita::resolve(tokens, &spec_root()) {
                Err(errs) => {
                    let first = errs.first().map(|e| format!("{e:?}")).unwrap_or_default();
                    failures.push(format!("{name}: embed: {first}"));
                }
                Ok(tokens) => match sadhana::t1::parse::Parser::new(&tokens).parse_program() {
                    Err(e) => failures.push(format!("{name}: parse: {e:?}")),
                    Ok(p) => declarations += p.declarations.len(),
                },
            },
        }
    }

    println!(
        "METRIC sadhana_t1_sources_parsed {}",
        files.len() - failures.len()
    );
    println!("METRIC sadhana_t1_declarations {declarations}");
    assert!(
        failures.is_empty(),
        "{} of {} T1 sources do not parse:\n  {}",
        failures.len(),
        files.len(),
        failures.join("\n  ")
    );
}

/// `W-239` — THE OCTET ARENA's Rust twin: the `Vec<u8>` a `Datum` carries
/// (`parse.rs:120`, `Datum::bytes`) and the eight operations the assembler
/// performs on it, in the spelling `ashtaka.t1` gives each. Rust keeps one
/// Vec per datum; T1 keeps ONE arena and a datum names a range of it — so
/// `&bytes[a..b]` is an operation here where in Rust it is a borrow.
const OCTET_ARENA_SYMBOLS: &[(&str, &str)] = &[
    ("Datum::bytes (Vec<u8>)", "अष्टककोश"),
    ("Vec::len", "अष्टकदैर्घ्य"),
    ("Vec::clear", "अष्टकारम्भः"),
    ("Vec::push", "अष्टकयोजनम्"),
    ("Vec::resize(len + n, 0)", "शून्याष्टकयोजनम्"),
    ("Vec::extend_from_slice", "अष्टकपाठयोजनम्"),
    ("IndexMut<usize> (and push at len)", "अष्टकस्थापनम्"),
    ("Index<usize>", "अष्टकपाठः"),
    ("&bytes[a..b]", "अष्टकखण्डः"),
];

/// The arena declares every operation of its twin — matched against the
/// DECLARATION, as the encoder's check below explains.
#[test]
fn the_octet_arena_declares_every_symbol_of_its_rust_twin() {
    let text = std::fs::read_to_string(crate_src().join("ashtaka.t1")).expect("ashtaka.t1 exists");
    let declared = |t1: &str| {
        text.contains(&format!("सार्वजनिक वृत्तिः {t1} "))
            || text.contains(&format!("सार्वजनिक चरः {t1} "))
    };
    let missing: Vec<&str> = OCTET_ARENA_SYMBOLS
        .iter()
        .filter(|(_, t1)| !declared(t1))
        .map(|(rust, _)| *rust)
        .collect();
    println!(
        "METRIC sadhana_t1_octet_arena_symbols {}",
        OCTET_ARENA_SYMBOLS.len() - missing.len()
    );
    assert!(
        missing.is_empty(),
        "ashtaka.t1 does not declare {} of {} Vec<u8> operations: {}",
        missing.len(),
        OCTET_ARENA_SYMBOLS.len(),
        missing.join(", ")
    );
    assert!(
        text.starts_with("मण्डलम् अष्टक ॥"),
        "the arena is the module `अष्टक`"
    );
}

/// `W-223` — THE SHARED DECLARATION STORE's Rust twin: what the loader
/// (`nirvahana.rs`, W-224's order-independent load) knows of every declaration,
/// read through `Interpreter::declarations`, and the operation `sanchaya.t1`
/// (घोषणासञ्चय) gives each. The loader parses heads once for all sources; the T1
/// side copies each parse's `व्याकर` arenas into arenas of its own, as text.
const STORE_SYMBOLS: &[(&str, &str)] = &[
    ("Interpreter::declarations", "सङ्ग्रहः"),
    ("ModuleDeclarations", "मण्डलप्रविष्टि"),
    ("Declaration", "प्रविष्टि"),
    ("Declaration::members", "प्राचलप्रविष्टि"),
    ("DeclarationKind::Variant", "भेदप्रविष्टिभेद"),
    ("Ty::text", "सञ्चयप्रकारलेखनम्"),
    ("by_module (find a module by name)", "मण्डलान्वेषणम्"),
    ("Module (found or added)", "मण्डलयोजनम्"),
    ("a member looked up by (module, name)", "सदस्यान्वेषणम्"),
    ("Module::imports (is it imported?)", "आयातितम्"),
    ("Declaration::kind", "प्रविष्टिभेदः"),
    ("Declaration::name", "प्रविष्टिनाम"),
    ("Declaration::ty", "प्रविष्टिप्रकारः"),
    ("Declaration::members.len", "प्रविष्टिप्राचलसंख्या"),
    ("Declaration::public", "प्रविष्टिसार्वजनिकत्वम्"),
    ("members[k].0", "प्राचलप्रविष्टिनाम"),
    ("members[k].1", "प्राचलप्रविष्टिप्रकारः"),
    ("ModuleDeclarations::name", "मण्डलनाम"),
    ("ModuleDeclarations::declarations.len", "मण्डलप्रविष्टिसंख्या"),
    ("the refusal, by kind and name", "निषेधः"),
];

#[test]
fn the_declaration_store_declares_every_symbol_of_its_rust_twin() {
    let text =
        std::fs::read_to_string(crate_src().join("sanchaya.t1")).expect("sanchaya.t1 exists");
    let declared = |t1: &str| {
        text.contains(&format!("सार्वजनिक वृत्तिः {t1} "))
            || text.contains(&format!("सार्वजनिक चरः {t1} "))
            || text.contains(&format!("सार्वजनिक संरचना {t1} "))
    };
    let missing: Vec<&str> = STORE_SYMBOLS
        .iter()
        .filter(|(_, t1)| !declared(t1))
        .map(|(rust, _)| *rust)
        .collect();
    println!(
        "METRIC sadhana_t1_declaration_store_symbols {}",
        STORE_SYMBOLS.len() - missing.len()
    );
    assert!(
        missing.is_empty(),
        "sanchaya.t1 does not declare {} of {} store operations: {}",
        missing.len(),
        STORE_SYMBOLS.len(),
        missing.join(", ")
    );
    assert!(
        text.starts_with("मण्डलम् घोषणासञ्चय ॥"),
        "the store is the module `घोषणासञ्चय`"
    );
}

/// Every symbol `crates/sadhana/src/encode.rs` defines, in the spelling the port
/// gives it. Present here means declared in `encode.t1` — a stub still counts as
/// declared, which is why the split below is stated in the source and reported by
/// `the_encoder_port_reports_how_much_of_it_is_real`.
const ENCODER_SYMBOLS: &[(&str, &str)] = &[
    ("RD", "गन्तृक्षेत्र"),
    ("RS1", "प्रथमस्रोतःक्षेत्र"),
    ("RS2", "द्वितीयस्रोतःक्षेत्र"),
    ("RS3", "तृतीयस्रोतःक्षेत्र"),
    ("MAX_RELAXATION_ROUNDS", "शिथिलनावृत्तिसीमा"),
    ("struct Slot", "संरचना अवकाश"),
    ("struct Pending", "संरचना प्रतीक्षा"),
    ("struct Encoding", "संरचना सङ्केत"),
    ("struct EncodeError", "संरचना सङ्केतनदोष"),
    ("enum Target", "गणना लक्ष्य"),
    ("is_register", "कोष्ठवाचकः"),
    ("is_immediate", "तत्कालवाचकः"),
    ("same_kind", "समानभेदः"),
    ("Slot::place", "स्थापनम्"),
    // Renamed from `समावेशः` by `B-112`: that word is in the FROZEN `keyword`
    // production of `spec/grammar-t1.ebnf` and binding it as a name was already
    // illegal. See `no_t1_source_binds_a_frozen_keyword_as_a_name` at the foot
    // of this file, which is the guard that keeps it from happening again.
    ("Slot::fits", "अन्तर्भावः"),
    ("Encoding::slot_with", "आवरणावकाशः"),
    ("Encoding::immediate", "तत्कालावकाशः"),
    ("Encoding::nth_immediate", "नियततत्कालावकाशः"),
    ("Encoding::register_count", "कोष्ठसंख्यानम्"),
    ("next_immediate", "अग्रिमतत्कालावकाशः"),
    ("words", "चतुरष्टकम्"),
    ("EncodeError::message", "सङ्केतनदोषवचनम्"),
    ("encodings", "सङ्केताः"),
    ("register", "कोष्ठाङ्कः"),
    ("domain_set", "क्षेत्रसमूहः"),
    ("split_address_part", "स्थानभागः"),
    ("value_of", "मूल्याङ्कः"),
    ("encode", "सङ्केतनम्"),
    ("encode_at", "स्थानसङ्केतनम्"),
    ("compressed_at", "सङ्कोचः"),
    ("forms_of", "रूपाणि"),
    ("encode_program", "कार्यक्रमसङ्केतनम्"),
    ("encode_program_for", "लक्ष्यसङ्केतनम्"),
    ("encode_object", "वस्तुसङ्केतनम्"),
    ("encode_object_for", "लक्ष्यवस्तुसङ्केतनम्"),
    ("layout_addresses", "स्थानविन्यासः"),
];

#[test]
fn the_encoder_port_declares_every_symbol_of_the_rust_original() {
    let text = std::fs::read_to_string(crate_src().join("encode.t1")).expect("encode.t1 exists");
    // Matched against the DECLARATION, not against the file.
    //
    // This was a bare `text.contains(t1)`, and the linker port's mutation test
    // is what exposed it: renaming a declaration left the test GREEN, because
    // the word still appeared in a prose comment three sections earlier. Every
    // routine in this file carries a `॰` comment naming its Rust original, so
    // the comments alone satisfy all 36 checks — the test could not have failed
    // for a missing routine, only for a missing MENTION of one. A port can lose
    // a routine and keep talking about it.
    //
    // Every declaration here opens with `सार्वजनिक`. Some entries in the table
    // above already carry their own keyword (`संरचना अवकाश`, `गणना लक्ष्य`);
    // the rest are bare names that are either `वृत्तिः` routines or `चरः`
    // constants. So all three shapes are tried, and a name that matches none of
    // them is not declared in this file — whatever the comments say about it.
    //
    // The trailing space matters: without it a name is a prefix of every longer
    // name beginning with it, and `अवकाश` would be satisfied by `अवकाशविस्तारः`.
    let declared = |t1: &str| {
        text.contains(&format!("सार्वजनिक {t1} "))
            || text.contains(&format!("सार्वजनिक वृत्तिः {t1} "))
            || text.contains(&format!("सार्वजनिक चरः {t1} "))
    };
    let missing: Vec<&str> = ENCODER_SYMBOLS
        .iter()
        .filter(|(_, t1)| !declared(t1))
        .map(|(rust, _)| *rust)
        .collect();
    println!(
        "METRIC sadhana_t1_encoder_symbols {}",
        ENCODER_SYMBOLS.len() - missing.len()
    );
    assert!(
        missing.is_empty(),
        "encode.t1 does not declare {} of {} encode.rs symbols: {}",
        missing.len(),
        ENCODER_SYMBOLS.len(),
        missing.join(", ")
    );
}

#[test]
fn the_encoder_port_reports_how_much_of_it_is_real() {
    // A declared routine whose whole body is `प्रत्यागमनम् उक्तम् अपूर्णम् इति ।`
    // is a stub — a label with a comment and a bare return. Counting them is the
    // only thing that stops the symbol test above from being satisfied by 36
    // stubs, which is exactly the failure that reopened `D-002a`.
    let text = std::fs::read_to_string(crate_src().join("encode.t1")).expect("encode.t1 exists");
    // `सार्वजनिक वृत्तिः`, not bare `वृत्तिः`: the word appears in this file's own
    // prose comments, and counting those inflated `real` by two.
    let functions = text.matches("सार्वजनिक वृत्तिः").count();
    // `सङ्केतनदोषवचनम्` has a real branch AND this fallback, so it is counted here
    // as a stub. Undercounting what is done is the safe direction for a row that
    // was reopened for being closed on a stub.
    let stubs = text.matches("प्रत्यागमनम् उक्तम् अपूर्णम् इति").count();
    let real = functions - stubs;
    println!("METRIC sadhana_t1_encoder_functions {functions}");
    println!("METRIC sadhana_t1_encoder_stubs {stubs}");
    println!("METRIC sadhana_t1_encoder_real {real}");
    // Ratchet. Raise it as stubs are replaced; it may never fall.
    //
    // **40 IS NOT 40/54 OF AN ENCODER.** It went 15 -> 40 in one commit, and
    // almost all of that is DECOMPOSITION: the per-row work of the four table
    // readers, the octets of the two address modifiers, the layout and
    // padding arithmetic, and the placement loop. Not one of the eight
    // routines that assemble a program moved, and this number cannot see
    // that. `the_encoder_port_reports_whether_it_encodes_anything` at the
    // foot of this file is the one that can, and it exists because of exactly
    // this jump.
    // RAISED 58 -> 59 on 2026-08-30, and this is the record the assertion
    // asks for. The routine is `मूल्याङ्कः`, Rust's `value_of`, and it was
    // the WHOLE of encoder blocker (c).
    //
    // WHICH DECISION CLEARED IT: none taken here. What cleared it was
    // `अक्षरकोश` gaining the numeral readers the same day — `अंशाः`
    // (sanskrit_text.t1:473) is `numeral::bits`'s Ok and `अंशदोषः` (:451) is
    // its Err, the `Result` split into two routines because T1 has no tuple.
    // Blocker (c) said in so many words that `sanskrit_text::numeral::bits`
    // "HAS NO T1 PORT AND IS NOT EVEN DECLARED"; it has one now, so the
    // clause that could not be written is one call, and `encode.t1` gained
    // its third `आयातः` to reach it.
    //
    // WHAT THIS NUMBER CANNOT SEE, and the reason the two assertions below
    // are unchanged: `मूल्याङ्कः` is the last routine that was blocked on
    // something OTHER than the eight. It is not one of the eight, it does
    // not encode an instruction, and `the_encoder_port_reports_whether_it_
    // encodes_anything` still reports ० program routines. One stub became a
    // body; nothing became an assembler.
    // RAISED 59 -> 60 on 2026-08-30, second raise of the day, and this is the
    // record. The routine is `सङ्केतनदोषवचनम्`, `EncodeError::message`.
    //
    // WHICH DECISION CLEARED IT: `D-002i2` added `("निदानकोशः",
    // "diagnostics.tsv")` to `TABLES` at crates/sadhana/src/t1/anita.rs:201 —
    // the fifth and sixth time a spec table on disk was unreachable for want
    // of a NAME. This routine's margin had been naming that exact row as its
    // blocker for two days, which is why it took one edit and not a
    // re-derivation: a blocker that names its row can be checked by anyone.
    //
    // The routine does NOT read spec/diagnostics.tsv. It is Rust's two lines —
    // the empty-code fallback and `Diagnostic::new(code, &args).render(lang)`
    // — over `निदान ॱ विवरणम्`, exactly as its sibling `निदान ॱ सन्देश`
    // (`ParseError::message`) is. Reading the file is `निदानपङ्क्तयः`, which
    // is another row's and still a stub; the routine is correct either way,
    // because an absent row renders as the कूट itself.
    // RAISED 60 -> 61 on 2026-08-31, and this is the record the assertion
    // asks for. The routine is `स्थानविन्यासः`, Rust's `layout_addresses`,
    // and it is the FIRST of the eight in `ENCODER_PROGRAM_ROUTINES` to get a
    // real body — so unlike the two raises above, this number and
    // `the_encoder_port_reports_whether_it_encodes_anything` move together.
    //
    // WHICH DECISION CLEARED IT: `गणना` variants became VALUES. `read_head`'s
    // `W_ENUM` arm (nirvahana.rs:1185) used to skip the block and register
    // nothing, so `असङ्कुचितम्` answered "is not a name in scope" and no
    // routine could name — let alone branch on — a `लक्ष्य` argument.
    // Variants now register as globals, zero-based by declaration position.
    //
    // AND ONE RECORDED FACT WAS FALSE. Blocker (b) and the note below both
    // said "nothing can hand a `कार्यक्रम` to the interpreter", on the
    // grounds that `सङ्कलनम्` answers a COUNT — true of `सङ्कलनम्` and of
    // nothing else. `वाक्यविभाग ॱ कार्यक्रमरचना` (vakyavibhaga.t1:595)
    // returns a whole `कार्यक्रम`, and `t1_exec_encode.rs` drives it. That is
    // the FIFTH blocker in this port found to describe the tree as it was.
    assert!(
        real >= 61,
        "only {real} of {functions} encoder routines have a real body (was 60)"
    );
    // The other direction: the remaining stubs are the WHOLE gap. LOWERED
    // 10 -> 9 -> 8 on 2026-08-30, for `मूल्याङ्कः` and `सङ्केतनदोषवचनम्`.
    //
    // THE EIGHT LEFT ARE EXACTLY `ENCODER_PROGRAM_ROUTINES` BELOW, and the
    // list of what stands in front of them is now short enough to write out.
    // Everything that was ever recorded as a missing CAPABILITY is gone:
    //
    //   * the embed — `निदानकोशः` landed at anita.rs:201;
    //   * the numeral reader — `अक्षरकोश ॱ अंशाः`/`अंशदोषः`;
    //   * `push` — six appenders in vakyavibhaga.t1;
    //   * the types — `आज्ञा` (:178) and `कार्यक्रम` (:287);
    //   * the slot parser — `विश्लेषण ॱ पङ्क्तिसङ्केतः` (vishlesana.t1:815)
    //     already turns a row of spec/encodings-riscv64.tsv into a `सङ्केत`;
    //   * the diagnostics — written above, and `निदानपङ्क्तयः` still being a
    //     stub does NOT block them: an absent registry row renders as the कूट
    //     itself, which `निदान ॱ विवरणम्`'s margin calls "a defined answer and
    //     not a fault" and `t1_exec_encode.rs` asserts.
    //
    // What is left is `encode_collecting` (encode.rs:1127) — ~250 lines, a
    // SECOND row search keyed on the family column rather than the mnemonic,
    // and one ADR-shaped question about whether the row reader moves out of
    // `विश्लेषण` (which imports `सङ्केतन`, so calling it from there is a
    // cycle). Plus the fact that nothing can hand a `कार्यक्रम` to the
    // interpreter yet — `सङ्कलनम्` answers a COUNT — so the eight cannot be
    // asserted against anything but themselves today.
    // LOWERED 8 -> 7 on 2026-08-31 for `स्थानविन्यासः`. THE SEVEN LEFT ARE
    // `ENCODER_PROGRAM_ROUTINES` MINUS `layout_addresses`, and what stands in
    // front of them is now ONE thing rather than the list above:
    // `encode_collecting` (encode.rs:1127), ~250 lines, plus the ADR about
    // where the row reader lives.
    //
    // `layout_addresses` is the one of the eight that never calls it — it
    // needs an instruction COUNT and the alignment requests, not any
    // instruction's operands — which is why it could move alone and why the
    // other seven did not follow it. Two items in the list above are struck
    // by its landing: the `कार्यक्रम` CAN be handed to the interpreter
    // (`कार्यक्रमरचना`), and a `गणना` variant IS a value.
    //
    // LOWERED 7 -> 0 ON 2026-09-04, `D-002a2`, AND THE ROW THAT SENT THE
    // WORKER SAID TWELVE. Measured before coding: ONE stub, `सङ्कोचः`, and
    // the blocker the row named for it — "a bitwise operator T1 lacks" —
    // had been closed by ADR-0032's operator section (`युक्`, `विषम`,
    // `विकल्प`, the two shifts) which `पदरचना` was already spending. What
    // `सङ्कोचः` actually waited on was its caller chain: `विन्यासावृत्तिः`
    // gained `लक्ष्यम्` and `चिह्नानि`, `उत्सर्जनक्रमः` gained `लक्ष्यम्`,
    // and `स्थानविन्यासः` rebuilds the text-label table after every round as
    // encode.rs:1814 does — WITHOUT its arity moving, which the margin had
    // predicted it would. Pinned at ० now, `ir.t1`'s way: a stub here is a
    // routine that had a body and lost it.
    assert_eq!(
        stubs, 0,
        "{stubs} encoder routines are stubs; D-002a2 left none on 2026-09-04, \
         and a stub here now means a body was removed"
    );
}

/// N-001's three ratified names, copied from the row: the row-signature
/// cache, the row-signature builder and the demand builder.
const N001_ROW_CACHE: &str = "सङ्केतसूचीप्रत्याहारकोश";
const N001_ROW_BUILDER: &str = "सङ्केतप्रत्याहारः";
const N001_DEMAND_BUILDER: &str = "आज्ञाप्रत्याहारः";
/// The routine both builders code a conversion's type text through (named by
/// the coordinator's follow-up, beside `प्रकारवर्गः`).
const N001_TYPE_CODE: &str = "प्रकारकूटः";

/// The checks the old candidate chain made, each a table or slot walk. The
/// mask compare reads none of them: what they answered is in the signatures.
const N001_OLD_CHAIN: &[&str] = &[
    "कोष्ठसंख्यानम्",
    "अवकाशसंख्या",
    "कोष्ठप्लवः",
    "कोष्ठव्यूहः",
    "कोष्ठसूचीव्यूहः",
    "कोष्ठाङ्कः",
    "आवरणावकाशः",
    "कोष्ठवाचकः",
    "तत्कालवाचकः",
    "प्रकारवर्गः",
    "समानपाठः",
    "कारकानुसारम्",
];

/// The code words of a `.t1` text: every margin (`॰` to the end of the line)
/// dropped, split on whitespace.
fn n001_code_words(text: &str) -> Vec<&str> {
    text.lines()
        .flat_map(|l| l.split('॰').next().unwrap_or("").split_whitespace())
        .collect()
}

/// The body of one public routine, from its declaration line to the first
/// `इति` at column ०.
fn n001_routine(text: &str, name: &str) -> String {
    let head = format!("सार्वजनिक वृत्तिः {name} आदाय");
    let start = text
        .find(&head)
        .unwrap_or_else(|| panic!("encode.t1 declares no `{name}`"));
    let rest = &text[start..];
    let end = rest.find("\nइति\n").expect("the routine closes");
    rest[..end].to_string()
}

/// N-001 — EVERY CANDIDATE IS DECIDED BY THE MASK COMPARE.
///
/// The selection loop in `स्थानसङ्केतनम्` asks `सङ्केतयोग्यम्` of each
/// candidate row. Before N-001 that routine was a chain of hand-written
/// checks, and the destination check walked the register table again per
/// candidate (`कोष्ठप्लवः`), half the chain's cost on ashtaka.t1. After it the
/// routine is `((S_row XOR D) AND C AND C_row) == 0` over a per-row
/// signature cached at parse time and a per-instruction demand built once
/// from the operand loop. This asserts the shape: the three ratified names
/// are declared, the match calls none of the old checks and has no branch
/// and no loop, and the second register-table walk has no call site left.
#[test]
fn every_candidate_is_decided_by_the_pratyahara_mask_compare() {
    let text = std::fs::read_to_string(crate_src().join("encode.t1")).expect("encode.t1 exists");
    let words = n001_code_words(&text);

    assert!(
        text.contains(&format!("सार्वजनिक चरः {N001_ROW_CACHE} ॱॱ")),
        "encode.t1 declares no row-signature cache `{N001_ROW_CACHE}`"
    );
    for name in [N001_ROW_BUILDER, N001_DEMAND_BUILDER] {
        assert!(
            text.contains(&format!("सार्वजनिक वृत्तिः {name} आदाय")),
            "encode.t1 declares no `{name}`"
        );
        let calls = words.iter().filter(|w| **w == name).count();
        assert_eq!(
            calls,
            2,
            "`{name}` has {} call site(s); it is built once, from one place",
            calls.saturating_sub(1)
        );
    }

    let body = n001_routine(&text, "सङ्केतयोग्यम्");
    let body_words = n001_code_words(&body);
    for old in N001_OLD_CHAIN {
        let n = body_words.iter().filter(|w| *w == old).count();
        assert_eq!(
            n, 0,
            "the match still calls the old check `{old}` ({n} site(s))"
        );
    }
    let count = |w: &str| body_words.iter().filter(|x| **x == w).count();
    // ((S_row XOR D) AND M) == 0 is written as (S_row AND M) == (D AND M) with
    // M = C AND C_row: the same compare bit for bit, and it keeps `विषम` the
    // one operator no body writes (`t1_operators.rs`, ADR-0026).
    assert_eq!(count("युक्"), 3, "the match is not three ANDs: {body}");
    assert_eq!(count("समम्"), 1, "the match is not one compare: {body}");
    assert_eq!(count("यदि"), 0, "the match branches: {body}");
    assert_eq!(count("यावत्"), 0, "the match loops: {body}");
    assert!(
        body_words.contains(&N001_ROW_CACHE),
        "the match does not read `{N001_ROW_CACHE}`"
    );

    // The destination's file is captured once per instruction now, so the
    // per-candidate register-table walk is gone from the encoder.
    let floats = words.iter().filter(|w| **w == "कोष्ठप्लवः").count();
    assert_eq!(
        floats,
        1,
        "`कोष्ठप्लवः` has {} call site(s) in encode.t1; the mask leaves none",
        floats.saturating_sub(1)
    );
    // And the selection loop is the match's only caller.
    let sites = words.iter().filter(|w| **w == "सङ्केतयोग्यम्").count();
    assert_eq!(
        sites,
        2,
        "`सङ्केतयोग्यम्` has {} call site(s), not one",
        sites - 1
    );
    // ...and that one caller is the selection loop's routine, as each
    // builder's one caller is the place its input is in hand.
    for (callee, caller) in [
        ("सङ्केतयोग्यम्", "स्थानसङ्केतनम्"),
        (N001_DEMAND_BUILDER, "स्थानसङ्केतनम्"),
        (N001_ROW_BUILDER, "सूचितसङ्केतः"),
    ] {
        let body = n001_routine(&text, caller);
        let n = n001_code_words(&body)
            .iter()
            .filter(|w| **w == callee)
            .count();
        assert_eq!(n, 1, "`{caller}` calls `{callee}` {n} time(s), not once");
    }
    println!("METRIC n001_old_chain_call_sites_in_match 0");
}

/// N-001 — THE ROW BUILDER PACKS ITS COUNTS UNSATURATED. A count wider than
/// its six-bit field would spill into the next field, and the table guard in
/// `t1_exec_encode.rs` (`n001_every_encoding_row_fits_the_signature_fields`)
/// is what makes that impossible, so the row side carries no clamp to ६३.
/// The DEMAND keeps its clamps: an instruction can be written with any count.
#[test]
fn the_row_signature_builder_does_not_saturate_its_counts() {
    let text = std::fs::read_to_string(crate_src().join("encode.t1")).expect("encode.t1 exists");
    let body = n001_routine(&text, N001_ROW_BUILDER);
    let clamps = n001_code_words(&body)
        .iter()
        .filter(|w| **w == "६३")
        .count();
    assert_eq!(
        clamps, 0,
        "`{N001_ROW_BUILDER}` still clamps {clamps} time(s) at ६३"
    );
    let demand = n001_routine(&text, N001_DEMAND_BUILDER);
    assert!(
        n001_code_words(&demand).contains(&"६३"),
        "`{N001_DEMAND_BUILDER}` lost its clamps; an instruction's counts are not bounded"
    );
}

/// N-001 — THE CONVERSION CODE IS ONE ROUTINE. Both signature builders turn a
/// conversion's type text into class × 1024 + width; the first build wrote
/// that loop twice, once per builder. The digit reader is called from one
/// routine only, and neither builder calls it.
#[test]
fn the_conversion_text_code_is_one_routine_both_builders_call() {
    let text = std::fs::read_to_string(crate_src().join("encode.t1")).expect("encode.t1 exists");
    let digits = "अक्षरकोशॱअङ्कमूल्यम्";
    let words = n001_code_words(&text);
    let sites = words.iter().filter(|w| **w == digits).count();
    assert_eq!(
        sites, 1,
        "`{digits}` has {sites} call site(s) in encode.t1, not one"
    );
    for builder in [N001_ROW_BUILDER, N001_DEMAND_BUILDER] {
        let body = n001_routine(&text, builder);
        let calls = n001_code_words(&body)
            .iter()
            .filter(|w| **w == N001_TYPE_CODE)
            .count();
        assert_eq!(
            calls, 2,
            "`{builder}` codes the pair {calls} time(s) through `{N001_TYPE_CODE}`, not twice"
        );
        assert!(
            !n001_code_words(&body).contains(&digits),
            "`{builder}` still reads the digits itself"
        );
    }
}

/// The four functions `crates/sadhana/src/samyojana.rs` defines, in the spelling
/// the port gives them — task `D-002b`.
///
/// **These are matched against the DECLARATION, not against the file.** The
/// encoder list above is checked with a bare `contains`, and a mutation test of
/// this one showed why that is not enough: renaming `सार्वजनिक वृत्तिः संस्कारः`
/// left the test green, because the word still appeared in a prose comment
/// three sections earlier. A port can lose a routine and keep talking about it.
const LINKER_FUNCTIONS: &[(&str, &str)] = &[
    ("kind_named", "भेदाङ्कः"),
    ("link", "संयोजनम्"),
    ("link_at", "स्थानसंयोजनम्"),
    ("patch", "संस्कारः"),
];

/// Every field of `Linked`, in the spelling `संयोजितम्` gives it.
///
/// Listed separately because the Rust linker's whole surface is one struct and
/// four functions, and almost all of its content is *inside* `link_at`: without
/// the fields this test would stay green over a `संयोजितम्` that had lost
/// `.debug` and `.table`. That is not hypothetical — `B-103` and `B-104` are
/// exactly that failure in the Rust original, a linked image with no `.symtab`
/// and no line table, each dropped silently.
const LINKED_FIELDS: &[(&str, &str)] = &[
    ("Linked::text", "पाठ्यम्"),
    ("Linked::data", "दत्तम्"),
    ("Linked::bss", "बीजम्"),
    ("Linked::symbols", "नामानि"),
    ("Linked::debug", "शोधनम्"),
    ("Linked::table", "सारणी"),
];

#[test]
fn the_linker_port_declares_every_symbol_of_the_rust_original() {
    let text =
        std::fs::read_to_string(crate_src().join("samyojana.t1")).expect("samyojana.t1 exists");

    let mut missing: Vec<&str> = LINKER_FUNCTIONS
        .iter()
        .filter(|(_, t1)| !text.contains(&format!("सार्वजनिक वृत्तिः {t1} ")))
        .map(|(rust, _)| *rust)
        .collect();

    // The struct, and its fields inside its own body — a field name loose in the
    // file is a parameter somewhere, not evidence the image still carries it.
    const HEAD: &str = "सार्वजनिक संरचना संयोजितम् आरभ्य";
    match text
        .split_once(HEAD)
        .and_then(|(_, rest)| rest.split_once("समाप्तम्").map(|(body, _)| body.to_string()))
    {
        None => missing.push("struct Linked"),
        Some(body) => missing.extend(
            LINKED_FIELDS
                .iter()
                .filter(|(_, t1)| !body.contains(t1))
                .map(|(rust, _)| *rust),
        ),
    }

    let total = LINKER_FUNCTIONS.len() + LINKED_FIELDS.len() + 1;
    println!("METRIC sadhana_t1_linker_symbols {}", total - missing.len());
    assert!(
        missing.is_empty(),
        "samyojana.t1 does not declare {} of {} samyojana.rs symbols: {}",
        missing.len(),
        total,
        missing.join(", ")
    );
}

#[test]
fn the_linker_port_reports_how_much_of_it_is_real() {
    // The same anti-gutting count as the encoder's, and for the same reason: the
    // symbol test above is satisfied by four stubs and an empty struct, which is
    // the failure that reopened `D-002a`. `D-002b` is DELIBERATELY NOT CLOSED on
    // this number — `संयोजनम्` and `स्थानसंयोजनम्` are still stubs, and the file
    // says what each waits on.
    let text =
        std::fs::read_to_string(crate_src().join("samyojana.t1")).expect("samyojana.t1 exists");
    let functions = text.matches("सार्वजनिक वृत्तिः").count();
    let stubs = text.matches("प्रत्यागमनम् उक्तम् अपूर्णम् इति").count();
    let real = functions - stubs;
    println!("METRIC sadhana_t1_linker_functions {functions}");
    println!("METRIC sadhana_t1_linker_stubs {stubs}");
    println!("METRIC sadhana_t1_linker_real {real}");
    // Ratchet. Raise it as stubs are replaced; it may never fall.
    assert!(
        real >= 60,
        "only {real} of {functions} linker routines have a real body (was 26)"
    );
    // The other direction: the three known stubs are the WHOLE gap. If a fourth
    // appears, a routine that had a body lost it.
    assert!(
        stubs == 0,
        "{stubs} linker routines are stubs; three are accounted for in samyojana.t1 (a), (b), (c)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The IR — task `D-002d`, `crates/sadhana/src/t1/ir.rs` ported to `ir.t1`.
//
// The file this checks REPLACED a `tools/port_encode.py` artefact whose
// content did not match `ir.rs` at all: it declared `DeviceLoad`, `Alloca`,
// `Mul`, `Div`, `Icmp` and a conditional branch, none of which exist in any
// Rust source in the tree, and it omitted the `(ValueId, Instruction)`
// pairing that does. Nothing checked it, so nothing said so.
// ─────────────────────────────────────────────────────────────────────────

/// Every symbol `crates/sadhana/src/t1/ir.rs` defines, in the spelling
/// `ir.t1` gives it.
///
/// The last five rows are not `pub` items of `ir.rs` but expressions in it:
/// the tuple-struct constructors `ValueId(n)` and `BlockId(n)`, which T1 has
/// to spell out because it has no tuple struct to call, and the three
/// `push`/`insert` sites, which T1 has to spell out because it has no
/// growable collection and appends into an arena instead. Losing any of the
/// five loses the port of a real line of `ir.rs`.
const IR_SYMBOLS: &[(&str, &str)] = &[
    ("struct ValueId", "संरचना मूल्याङ्क"),
    ("struct BlockId", "संरचना पर्वाङ्क"),
    ("enum Instruction", "संरचना आज्ञा"),
    ("enum Terminator", "संरचना अवसान"),
    ("struct Block", "संरचना पर्व"),
    ("struct Function", "संरचना वृत्ति"),
    ("Instruction::ConstInt", "ध्रुवाज्ञाभेद"),
    ("Instruction::Call", "आह्वानाज्ञाभेद"),
    ("Instruction::Add", "योगाज्ञाभेद"),
    ("Instruction::Sub", "वियोगाज्ञाभेद"),
    ("Instruction::Param", "प्राचलाज्ञाभेद"),
    // `W-245`: the eleven kinds that make a status more than zero, and the
    // six conditions of ADR-0008 as `Cmp`'s sub-kind.
    ("Instruction::Mul", "गुणनाज्ञाभेद"),
    ("Instruction::Div", "भागाज्ञाभेद"),
    ("Instruction::Rem", "शेषाज्ञाभेद"),
    ("Instruction::Shl", "वामसरणाज्ञाभेद"),
    ("Instruction::Shr", "दक्षिणसरणाज्ञाभेद"),
    ("Instruction::And", "युक्ताज्ञाभेद"),
    ("Instruction::Or", "विकल्पाज्ञाभेद"),
    ("Instruction::Xor", "वैषम्याज्ञाभेद"),
    ("Instruction::Cmp", "तुलनाज्ञाभेद"),
    ("Instruction::Load", "आहाराज्ञाभेद"),
    ("Instruction::Store", "निधानाज्ञाभेद"),
    ("CmpOp::Eq", "समतुलनाभेद"),
    ("CmpOp::Ne", "विषमतुलनाभेद"),
    ("CmpOp::Lt", "न्यूनतुलनाभेद"),
    ("CmpOp::Ge", "अन्यूनतुलनाभेद"),
    ("CmpOp::Ltu", "अचिह्नन्यूनतुलनाभेद"),
    ("CmpOp::Geu", "अचिह्नान्यूनतुलनाभेद"),
    ("Block::insts.push (Cmp)", "तुलनाज्ञायोजनम्"),
    ("Block::insts.push (Load)", "आहाराज्ञायोजनम्"),
    ("Block::insts.push (Store)", "निधानाज्ञायोजनम्"),
    ("Function::locals (the slot table)", "स्थानीयचिह्नककोश"),
    ("Function::locals.get", "स्थानीयस्थानम्"),
    ("Function::locals.entry(..).or_insert", "स्थानीयघोषणम्"),
    ("Terminator::Return", "प्रत्यागमनावसानभेद"),
    ("Terminator::Branch", "लङ्घनावसानभेद"),
    ("Terminator::Unreachable", "अगम्यावसानभेद"),
    ("Terminator::CondBranch", "शाखावसानभेद"),
    ("IrBuilder::next_val", "अग्रिममूल्याङ्क"),
    ("IrBuilder::next_block", "अग्रिमपर्वाङ्क"),
    ("IrBuilder::functions", "वृत्तिकोश"),
    ("IrBuilder::new", "आरम्भः"),
    ("IrBuilder::new_value", "नवमूल्यम्"),
    ("IrBuilder::new_block", "नवपर्व"),
    ("IrBuilder::build_program", "कार्यक्रमरचना"),
    ("IrBuilder::build_statement", "वाक्यरचना"),
    ("IrBuilder::build_expression", "अभिव्यञ्जकरचना"),
    ("ValueId(n)", "मूल्याङ्कनम्"),
    ("BlockId(n)", "पर्वाङ्कनम्"),
    ("Block::insts.push", "आज्ञायोजनम्"),
    ("Function::blocks.insert", "पर्वयोजनम्"),
    ("IrBuilder::functions.push", "वृत्तियोजनम्"),
];

/// Every field of the four structs, checked INSIDE the struct that must
/// carry it.
///
/// This is where the port's two hardest decisions are recorded, and a
/// symbol check alone would not notice either being undone. `Block::insts`
/// is a `Vec<(ValueId, Instruction)>` in Rust and a first-index/count pair
/// here; `Function::blocks` is a `HashMap<BlockId, Block>` in Rust and a
/// first-index/count pair here. If either collapses back to a single field
/// the port has silently lost the run it was standing for.
const IR_FIELDS: &[(&str, &[(&str, &str)])] = &[
    (
        "आज्ञा",
        &[
            ("Instruction tag", "भेद"),
            ("(ValueId, _) of the pair", "फलम्"),
            ("ConstInt(i64)", "ध्रुवमूल्यम्"),
            ("Call's SymbolId", "संज्ञा"),
            ("Call's args: first", "आदानारम्भ"),
            ("Call's args: count", "आदानसंख्यान"),
            ("Add/Sub left", "वाम"),
            ("Add/Sub right", "दक्षिण"),
            ("Param(usize)", "प्राचलक्रम"),
            ("Cmp's CmpOp", "उपभेद"),
            ("Load's/Store's slot", "स्थानक्रम"),
        ],
    ),
    (
        "अवसान",
        &[
            ("Terminator tag", "भेद"),
            ("Return(Option<ValueId>)", "मूल्यम्"),
            ("Branch(BlockId)", "लक्ष्यम्"),
            ("CondBranch(_, _, else BlockId)", "अन्यलक्ष्यम्"),
        ],
    ),
    (
        "पर्व",
        &[
            ("Block::id", "अङ्कन"),
            ("Block::insts: first", "आज्ञारम्भ"),
            ("Block::insts: count", "आज्ञासंख्यान"),
            ("Block::terminator", "अवसानम्"),
        ],
    ),
    (
        "वृत्ति",
        &[
            ("Function::name", "नाम"),
            ("Function::blocks: first", "पर्वारम्भ"),
            ("Function::blocks: count", "पर्वसंख्यान"),
            ("Function::entry_block", "प्रवेशपर्व"),
        ],
    ),
];

fn ir_text() -> String {
    std::fs::read_to_string(crate_src().join("ir.t1")).expect("ir.t1 exists")
}

#[test]
fn the_ir_port_declares_every_symbol_of_the_rust_original() {
    let text = ir_text();
    // Matched against the DECLARATION, never against the file, for the
    // reason written out above `the_encoder_port_declares_...`: a bare
    // `contains` is satisfied by a prose comment, and this file's header
    // names most of `ir.rs` in English and Sanskrit both.
    let declared = |t1: &str| {
        text.contains(&format!("सार्वजनिक {t1} "))
            || text.contains(&format!("सार्वजनिक वृत्तिः {t1} "))
            || text.contains(&format!("सार्वजनिक चरः {t1} "))
    };
    let mut missing: Vec<String> = IR_SYMBOLS
        .iter()
        .filter(|(_, t1)| !declared(t1))
        .map(|(rust, _)| (*rust).to_string())
        .collect();

    // Fields, inside their own struct body. Matched as `name ॱॱ ` — the
    // annotation that makes it a FIELD — rather than as a bare word, so a
    // field cannot be satisfied by a comment or by a longer name it is a
    // prefix of.
    for (name, fields) in IR_FIELDS {
        let head = format!("सार्वजनिक संरचना {name} आरभ्य");
        match text
            .split_once(&head)
            .and_then(|(_, rest)| rest.split_once("समाप्तम्").map(|(body, _)| body.to_string()))
        {
            None => missing.push(format!("struct {name}")),
            Some(body) => missing.extend(
                fields
                    .iter()
                    .filter(|(_, t1)| !body.contains(&format!("{t1} ॱॱ ")))
                    .map(|(rust, _)| (*rust).to_string()),
            ),
        }
    }

    let total = IR_SYMBOLS.len() + IR_FIELDS.iter().map(|(_, f)| f.len()).sum::<usize>();
    println!("METRIC sadhana_t1_ir_symbols {}", total - missing.len());
    assert!(
        missing.is_empty(),
        "ir.t1 does not declare {} of {} t1/ir.rs symbols: {}",
        missing.len(),
        total,
        missing.join(", ")
    );
}

#[test]
fn the_ir_port_reports_how_much_of_it_is_real() {
    // The same anti-gutting count as the encoder's and the linker's.
    let text = ir_text();
    let functions = text.matches("सार्वजनिक वृत्तिः").count();
    let stubs = text.matches("प्रत्यागमनम् उक्तम् अपूर्णम् इति").count();
    let real = functions - stubs;
    println!("METRIC sadhana_t1_ir_functions {functions}");
    println!("METRIC sadhana_t1_ir_stubs {stubs}");
    println!("METRIC sadhana_t1_ir_real {real}");

    // **TWO DIFFERENT THINGS ARE CALLED A STUB AND ONLY ONE OF THEM REACHES THE
    // DASHBOARD.** `sadhana_t1_ir_stubs` above counts PORT-ERA PLACEHOLDER
    // BODIES — `प्रत्यागमनम् उक्तम् अपूर्णम् इति`, a routine that was declared
    // before it was written. It reads 0, and 0 is TRUE: every one of them has
    // been implemented, in this file and in all nine of its siblings.
    //
    // A LOWERING STUB IS NOT THAT. `अपूर्णध्रुवम् <cause>` is a routine that RAN,
    // reached a shape it declines to lower, and planted a counted constant so
    // compilation can continue. Measured 2026-09-18: FIFTY of them, every one in
    // ir.t1 and none anywhere else in the corpus.
    //
    // So `.loop/METRICS.tsv` carried ELEVEN rows reading `*_stubs 0` and NOTHING
    // carrying the fifty. A reader of the dashboard concluded the compiler's
    // stub surface was empty. Each zero is true; the set of them is misleading,
    // which is the same defect as a count that is right about the wrong
    // population — the third instance of that shape this week.
    //
    // REPORTING ONLY, DELIBERATELY NO RATCHET. A peer session is splitting cause २७
    // into named sites right now, which MOVES this number; a ratchet here would
    // red their work for doing exactly what it is for. The number is worth
    // seeing and is not yet worth pinning.
    //
    // MARGINS STRIPPED BEFORE COUNTING. `॰` opens a comment and the margins in
    // this file quote `अपूर्णध्रुवम्` while explaining it, so an unstripped grep
    // reports raise sites that are prose. That error killed a correct hypothesis
    // once already here.
    let lowering_stubs = text
        .lines()
        .map(|l| l.split('\u{0970}').next().unwrap_or(""))
        // AND THE DECLARATION OF A RAISER IS NOT A RAISE. Both routines are
        // written `सार्वजनिक वृत्तिः अपूर्णध्रुवम् आदाय …`, so a bare token match
        // counts the two DEFINITIONS alongside the call sites and answers 47.
        //
        // THE FULL RECONCILIATION, because three agents got three numbers from
        // one file and each was internally correct:
        //
        //     50 raw token occurrences in ir.t1
        //      = 45 LIVE RAISE SITES (43 literal cause + 2 VARIABLE cause)
        //      +  3 margin mentions
        //      +  2 routine definitions
        //
        // 42 comes from filtering on `अपूर्णध्रुवम् [०-९]+`, a Devanagari
        // NUMERAL, which silently drops every raise whose cause is a variable —
        // `अपूर्णध्रुवम् नामभेदः` (:2659), `अपूर्णवाक्यम् समहेतुः` (:4054).
        // 45 is the figure to quote.
        .filter(|code| !code.contains("वृत्तिः"))
        .map(|code| code.matches("अपूर्णध्रुवम् ").count() + code.matches("अपूर्णवाक्यम् ").count())
        .sum::<usize>();
    println!("METRIC sadhana_t1_ir_lowering_stubs {lowering_stubs}");
    // Ratchet. Raise it as routines land; it may never fall.
    assert!(
        real >= 13,
        "only {real} of {functions} IR routines have a real body (was 13)"
    );
    // `ir.t1` has no stub at all, and that is a claim worth pinning rather
    // than a happy accident: every gap this port has is a MISSING ARM of a
    // routine that otherwise works, not an unwritten routine. If a stub
    // appears here, a routine that had a body lost it.
    assert_eq!(stubs, 0, "{stubs} IR routines are stubs; ir.t1 has none");
}

#[test]
fn the_ir_port_reports_which_instruction_kinds_are_actually_built() {
    // **This is the test that stops the two above from reading as "done".**
    //
    // Between them they say every symbol is declared and every routine has
    // a body, and both are true — and the port is still partial, because
    // some of `ir.rs`'s instruction kinds and terminators are declared and
    // CONSTRUCTED BY NOTHING (as of `W-198`: `Call` and `Param` of the five
    // kinds; `Unreachable` of the four terminators). A declaration count
    // cannot see that; the number below can, because it looks for the kind
    // at the call site that would emit it.
    //
    // `ir.t1` says at its foot exactly what each missing arm waits on:
    // (a) `वास्तु ॱ वाक्य` records a block as first/last indices with no
    // child list, so a block walk would double-emit nested statements;
    // (b) `व्याकर ॱ घोषणा` keeps no parameter count, so `Param` has no loop
    // bound; (c) `Call` is constructed by nothing in `ir.rs` either.
    //
    // Raise the floors as arms land. They may never fall.
    let text = ir_text();

    const KINDS: &[(&str, &str)] = &[
        ("ConstInt", "ध्रुवाज्ञाभेद"),
        ("Call", "आह्वानाज्ञाभेद"),
        ("Add", "योगाज्ञाभेद"),
        ("Sub", "वियोगाज्ञाभेद"),
        ("Param", "प्राचलाज्ञाभेद"),
        // `W-245`. The ten binary kinds share one द्विकर्म arm whose kind is a
        // VARIABLE (`आज्ञायोजनम् आरभ्य आज्ञाभेदः`), so this by-name reading sees
        // none of them built — Add and Sub included, since that arm landed. The
        // three with a wrapper of their own (`तुलनाज्ञायोजनम्`, `आहाराज्ञायोजनम्`,
        // `निधानाज्ञायोजनम्`) are read through the wrapper's own line.
        ("Mul", "गुणनाज्ञाभेद"),
        ("Div", "भागाज्ञाभेद"),
        ("Rem", "शेषाज्ञाभेद"),
        ("Shl", "वामसरणाज्ञाभेद"),
        ("Shr", "दक्षिणसरणाज्ञाभेद"),
        ("And", "युक्ताज्ञाभेद"),
        ("Or", "विकल्पाज्ञाभेद"),
        ("Xor", "वैषम्याज्ञाभेद"),
        ("Cmp", "तुलनाज्ञाभेद"),
        ("Load", "आहाराज्ञाभेद"),
        ("Store", "निधानाज्ञाभेद"),
    ];
    const TERMINATORS: &[(&str, &str)] = &[
        ("Return", "प्रत्यागमनावसानभेद"),
        ("Branch", "लङ्घनावसानभेद"),
        ("Unreachable", "अगम्यावसानभेद"),
        ("CondBranch", "शाखावसानभेद"),
    ];

    let built: Vec<&str> = KINDS
        .iter()
        .filter(|(_, t1)| text.contains(&format!("आज्ञायोजनम् आरभ्य {t1} ")))
        .map(|(rust, _)| *rust)
        .collect();
    let terminated: Vec<&str> = TERMINATORS
        .iter()
        .filter(|(_, t1)| text.contains(&format!("अवसानरचना आरभ्य {t1} ")))
        .map(|(rust, _)| *rust)
        .collect();

    println!("METRIC sadhana_t1_ir_kinds_declared {}", KINDS.len());
    println!("METRIC sadhana_t1_ir_kinds_built {}", built.len());
    println!(
        "METRIC sadhana_t1_ir_terminators_declared {}",
        TERMINATORS.len()
    );
    println!(
        "METRIC sadhana_t1_ir_terminators_built {}",
        terminated.len()
    );

    assert!(
        !built.is_empty(),
        "no instruction kind is emitted by आज्ञायोजनम्; the IR builds nothing"
    );
    assert!(
        !terminated.is_empty(),
        "no terminator is emitted by अवसानरचना; no block is ever closed"
    );
}

/// Every symbol `crates/sadhana/src/vishlesana.rs` defines, in the spelling
/// `विश्लेषण` gives it — the disassembler half of `D-002c`.
///
/// # This list is why the file it checks changed module
///
/// `vishlesana.t1` did not hold a port of `vishlesana.rs`. It held the T1
/// compiler's SEMA — `Resolver`, `Scope`, `SymbolEntry`, `TypeKind` — machine
/// transliterated as `मण्डलम् वइशलएसअन`, and none of the eight symbols below
/// appeared in it. `spec/marks.tsv`, `spec/lexicon.src.tsv` line 437 and task
/// `B-013` all name `विश्लेषणम्` the disassembler, and `samyojana.t1`'s own
/// stub note (b) waits on `विश्लेषण ॱ decode`, so the file was named for one
/// module and filled with another.
///
/// **That is the failure this table makes impossible to repeat.** A test that
/// only asked whether the file parsed was satisfied by the wrong module in it;
/// a test that asks for `उद्धरणम्` and `आज्ञादैर्घ्यम्` by name is not.
const DISASSEMBLER_SYMBOLS: &[(&str, &str)] = &[
    ("struct Decoded", "संरचना विश्लिष्टम्"),
    ("extract", "उद्धरणम्"),
    ("width_of", "आज्ञादैर्घ्यम्"),
    ("candidates", "सम्भाविनः"),
    ("decode", "विश्लेषणम्"),
    ("decode16", "सङ्कुचितविश्लेषणम्"),
    ("decode_at", "स्थानविश्लेषणम्"),
    ("reassemble", "पुनःसंयोजनम्"),
];

/// Every field of `Decoded`, in the spelling `विश्लिष्टम्` gives it.
///
/// Listed separately and checked INSIDE the struct's own body, for the reason
/// `LINKED_FIELDS` gives: a field name loose in the file is a parameter
/// somewhere, not evidence the struct still carries it.
const DECODED_FIELDS: &[(&str, &str)] = &[
    ("Decoded::insn", "आज्ञा"),
    ("Decoded::family", "कुल"),
    ("Decoded::operands", "मूल्यानि"),
];

/// The body of one `सार्वजनिक संरचना NAME आरभ्य … समाप्तम्`, or `None`.
fn struct_body(text: &str, head: &str) -> Option<String> {
    text.split_once(head)
        .and_then(|(_, rest)| rest.split_once("समाप्तम्").map(|(body, _)| body.to_string()))
}

#[test]
fn the_disassembler_port_declares_every_symbol_of_the_rust_original() {
    let text =
        std::fs::read_to_string(crate_src().join("vishlesana.t1")).expect("vishlesana.t1 exists");

    // Matched against the DECLARATION, not against the file — the rule the
    // encoder's mutation test established. Every routine here carries a `॰`
    // comment naming its Rust original, so a bare `contains` would be
    // satisfied by the comments alone and could never fail for a lost routine.
    let declared = |t1: &str| {
        text.contains(&format!("सार्वजनिक {t1} "))
            || text.contains(&format!("सार्वजनिक वृत्तिः {t1} "))
            || text.contains(&format!("सार्वजनिक चरः {t1} "))
    };
    let mut missing: Vec<&str> = DISASSEMBLER_SYMBOLS
        .iter()
        .filter(|(_, t1)| !declared(t1))
        .map(|(rust, _)| *rust)
        .collect();

    match struct_body(&text, "सार्वजनिक संरचना विश्लिष्टम् आरभ्य")
    {
        None => missing.push("struct Decoded"),
        Some(body) => missing.extend(
            DECODED_FIELDS
                .iter()
                .filter(|(_, t1)| !body.contains(t1))
                .map(|(rust, _)| *rust),
        ),
    }

    // The module line, because the whole defect this file records was the
    // WRONG MODULE under the right filename. Without this the table above is
    // satisfied by a correct disassembler that still declares itself to be
    // something else, which is exactly the state it was found in.
    if !text.starts_with("मण्डलम् विश्लेषण ") {
        missing.push("मण्डलम् विश्लेषण");
    }

    let total = DISASSEMBLER_SYMBOLS.len() + DECODED_FIELDS.len() + 1;
    println!(
        "METRIC sadhana_t1_disassembler_symbols {}",
        total - missing.len()
    );
    assert!(
        missing.is_empty(),
        "vishlesana.t1 does not declare {} of {} vishlesana.rs symbols: {}",
        missing.len(),
        total,
        missing.join(", ")
    );
}

#[test]
fn the_disassembler_port_reports_how_much_of_it_is_real() {
    // The same anti-gutting count as the encoder's and the linker's, and for
    // the same reason: the symbol test above is satisfied by eight stubs,
    // which is the failure that reopened `D-002a`.
    let text =
        std::fs::read_to_string(crate_src().join("vishlesana.t1")).expect("vishlesana.t1 exists");
    let functions = text.matches("सार्वजनिक वृत्तिः").count();
    let stubs = text.matches("प्रत्यागमनम् उक्तम् अपूर्णम् इति").count();
    let real = functions - stubs;
    println!("METRIC sadhana_t1_disassembler_functions {functions}");
    println!("METRIC sadhana_t1_disassembler_stubs {stubs}");
    println!("METRIC sadhana_t1_disassembler_real {real}");
    // Ratchet. Raise it as stubs are replaced; it may never fall.
    //
    // 12 → 33, and the row that moved it is `B-013`'s completion on
    // 2026-08-30. WHICH ROW CLEARED WHICH BLOCKER, since that is what this
    // ratchet is asked to record:
    //
    //   `सम्भाविनः`, `विश्लेषणम्`, `सङ्कुचितविश्लेषणम्`, `स्थानविश्लेषणम्`
    //   and `पुनःसंयोजनम्` — the five stubs at the foot — all said
    //   "awaits `सङ्केतन ॱ सङ्केताः`". **That blocker was stale, and its
    //   PREMISE was wrong as well as its date.** T1 has had an include since
    //   ADR-0019 and `spec/encodings-riscv64.tsv` has carried the embed name
    //   `सङ्केतकोशः` since; and nothing here ever wanted the whole table.
    //   `सङ्केताः` returns EVERY row and so does need a collection —
    //   `विश्लेषणम्` wants ONE ROW. The range idiom `सङ्केतन ॱ कोष्ठाङ्कः` and
    //   `संयोजन ॱ भेदाङ्कः` already read their tables with answers it with no
    //   collection at all, and `सङ्केतपङ्क्तिः` is that idiom here.
    //
    //   The 21 routines added with them are what reading a row costs:
    //   `खण्डसंख्या`/`खण्डारम्भः`/`खण्डसीमा` (the fields column nests three
    //   separators, so the separator is a parameter), the two field readers,
    //   `पङ्क्तिसङ्केतः`/`पङ्क्त्यवकाशः`/`न्यासरचना` (a row, a slot, a bit
    //   map), and `मूल्यसंख्या`/`न्याससंख्या` — the two that carried the
    //   finding that an arena in this language could not be empty, so ०
    //   entries and १ both reported `ॱ दैर्घ्य` १ and only entry ० told them
    //   apart. Since `W-355` a fresh arena has length ० and the length answers.
    //
    // Every one of them is EXECUTED by `tests/t1_execution.rs`, against
    // `spec/encodings-riscv64.tsv` read there in Rust — this count cannot see
    // whether a body is right, and three mutation tests there can.
    assert!(
        real >= 33,
        "only {real} of {functions} disassembler routines have a real body (was 12, raised to 33 by B-013)"
    );
    // The other direction: there are NO stubs left. `B-013` discharged all
    // five; if one comes back, a routine that had a body lost it.
    assert!(
        stubs == 0,
        "{stubs} disassembler routines are stubs; B-013 left none, and the \
         blocker they all named — `सङ्केतन ॱ सङ्केताः` — was answered by \
         reading one row instead of the whole table"
    );
}

/// Every symbol the sema defines across `crates/sadhana/src/t1/types.rs`,
/// `resolve.rs` and `typecheck.rs`, in the spelling `अर्थ` gives it — task
/// `D-002c`, doc 03 §8 phase 3.5.5.
///
/// Three Rust files, one `मण्डलम्`: T1 has no submodules, and `typecheck.rs`
/// cannot be read without `Ty` or without `Resolver`.
///
/// `Ty`'s twelve variants are `चरः` constants rather than a `गणना` because
/// three of them carry a `Box<Ty>` and three a `SymbolId`, and T1 has no
/// payload-carrying enum. That is the same flattening `ast.t1` already applies.
const SEMA_SYMBOLS: &[(&str, &str)] = &[
    ("enum Ty", "संरचना अर्थप्रकार"),
    ("Ty::Int", "पूर्णाङ्कार्थभेद"),
    ("Ty::Float", "प्लवार्थभेद"),
    ("Ty::Pointer", "स्थानार्थभेद"),
    ("Ty::Slice", "खण्डार्थभेद"),
    ("Ty::Optional", "सम्भाव्यार्थभेद"),
    ("Ty::ErrorUnion", "दोषयुक्तार्थभेद"),
    ("Ty::Struct", "संरचनार्थभेद"),
    ("Ty::Union", "संघार्थभेद"),
    ("Ty::Enum", "गणनार्थभेद"),
    ("Ty::Void", "शून्यार्थभेद"),
    ("Ty::Never", "अभावार्थभेद"),
    ("Ty::Error", "दोषार्थभेद"),
    ("struct ResolveError", "संरचना निर्णयदोष"),
    ("struct TypeError", "संरचना प्रकारदोष"),
    ("struct Resolver", "संरचना निर्णायक"),
    ("Resolver::new", "निर्णायकारम्भः"),
    ("Resolver::new_symbol", "नवसंज्ञा"),
    ("Resolver::enter_scope", "परिसरप्रवेशः"),
    ("Resolver::exit_scope", "परिसरनिर्गमः"),
    ("Resolver::declare", "घोषणम्"),
    ("Resolver::resolve_name", "नामनिर्णयः"),
    ("Resolver::resolve_program", "कार्यक्रमनिर्णयः"),
    ("Resolver::resolve_statement", "वाक्यनिर्णयः"),
    ("Resolver::resolve_expression", "अभिव्यञ्जकनिर्णयः"),
    ("TypeChecker::new", "प्रकारपरीक्षकारम्भः"),
    ("TypeChecker::typecheck_program", "कार्यक्रमप्रकारपरीक्षा"),
    ("TypeChecker::eval_ast_type", "प्रकारार्थः"),
    ("TypeChecker::typecheck_statement", "वाक्यप्रकारः"),
    ("TypeChecker::typecheck_expression", "अभिव्यञ्जकप्रकारः"),
];

/// The two fields of `Resolver` this port carries, inside its own body.
///
/// **`resolved_symbols` is not here, and since `W-274` (2026-09-05) it is not
/// in the Rust original either — it was DELETED.** The reasoning that kept it
/// out of this list stands and is why the deletion was right, so it is kept
/// rather than replaced: `resolve_expression`'s `Identifier` arm says out loud
/// "We'd store this if we had NodeIds in the AST", so that map was declared,
/// never written and never read, and requiring the port to mirror it would have
/// made this test demand a field standing for a capability neither side has.
///
/// The list is therefore COMPLETE now rather than deliberately short, which is
/// a different claim and worth spelling: if `Resolver` grows a third field, this
/// test should demand its mirror instead of assuming another exemption.
const RESOLVER_FIELDS: &[(&str, &str)] = &[
    ("Resolver::next_symbol", "अग्रिमसंज्ञा"),
    ("Resolver::scopes", "परिसराः"),
];

#[test]
fn the_sema_port_declares_every_symbol_of_the_rust_original() {
    let text = std::fs::read_to_string(crate_src().join("artha.t1")).expect("artha.t1 exists");

    let declared = |t1: &str| {
        text.contains(&format!("सार्वजनिक {t1} "))
            || text.contains(&format!("सार्वजनिक वृत्तिः {t1} "))
            || text.contains(&format!("सार्वजनिक चरः {t1} "))
    };
    let mut missing: Vec<&str> = SEMA_SYMBOLS
        .iter()
        .filter(|(_, t1)| !declared(t1))
        .map(|(rust, _)| *rust)
        .collect();

    match struct_body(&text, "सार्वजनिक संरचना निर्णायक आरभ्य")
    {
        None => missing.push("struct Resolver"),
        Some(body) => missing.extend(
            RESOLVER_FIELDS
                .iter()
                .filter(|(_, t1)| !body.contains(t1))
                .map(|(rust, _)| *rust),
        ),
    }

    if !text.starts_with("मण्डलम् अर्थ ") {
        missing.push("मण्डलम् अर्थ");
    }

    let total = SEMA_SYMBOLS.len() + RESOLVER_FIELDS.len() + 1;
    println!("METRIC sadhana_t1_sema_symbols {}", total - missing.len());
    assert!(
        missing.is_empty(),
        "artha.t1 does not declare {} of {} sema symbols: {}",
        missing.len(),
        total,
        missing.join(", ")
    );
}

#[test]
fn the_sema_port_reports_how_much_of_it_is_real() {
    // `D-002c` is DELIBERATELY NOT CLOSED on this number. Eleven of the
    // twenty-three routines are stubs and the file says what each waits on:
    // (a) growable collections, which is `push`, `pop` and `insert` and is the
    // same blocker `samyojana.t1` names as its (c); and (b) `ast.t1` having no
    // declaration node and no program node, so both entry points of the sema
    // have no tree to walk.
    let text = std::fs::read_to_string(crate_src().join("artha.t1")).expect("artha.t1 exists");
    let functions = text.matches("सार्वजनिक वृत्तिः").count();
    let stubs = text.matches("प्रत्यागमनम् उक्तम् अपूर्णम् इति").count();
    let real = functions - stubs;
    println!("METRIC sadhana_t1_sema_functions {functions}");
    println!("METRIC sadhana_t1_sema_stubs {stubs}");
    println!("METRIC sadhana_t1_sema_real {real}");
    // Ratchet. Raise it as stubs are replaced; it may never fall.
    assert!(
        real >= 19,
        "only {real} of {functions} sema routines have a real body (was 12)"
    );
    // The other direction: the eleven known stubs are the WHOLE gap.
    assert!(
        stubs <= 5,
        "{stubs} sema routines are stubs; eleven are accounted for in artha.t1 (a) and (b)"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `D-002e` — the codegen, `crates/sadhana-t1/src/utsarjana.t1`.
//
// # The file was named for one module and held a third of another
//
// `utsarjana.t1` was 28 lines of `tools/port_encode.py` output and held
// `t1/regalloc.rs`'s two types and nothing else. **Not one symbol of
// `t1/emit.rs` or `t1/x86_64.rs` was in it** — no `emit_program`, no
// `X86_64Emitter`, no `reg_name` — so two of the codegen's three Rust files
// were absent with nothing able to say so. It also invented, exactly as
// `ir.t1`'s predecessor did: `लओकएशअनएएनटरय` ("LocationEntry") appears in no
// Rust source in this tree, and its `उतसरजन` routine returned bytes, which
// nothing in the codegen does — both emitters return text.
//
// The three tables below are what makes that unrepeatable: they name every
// symbol of **all three** Rust files, so a port that covers one file and calls
// itself the codegen fails by name.
// ─────────────────────────────────────────────────────────────────────────

/// Every symbol `crates/sadhana/src/t1/regalloc.rs`, `t1/emit.rs` and
/// `t1/x86_64.rs` define, in the spelling `उत्सर्जन` gives them.
///
/// Present means DECLARED — a stub still counts, which is why
/// `the_codegen_port_reports_how_much_of_it_is_real` splits them and why
/// `the_codegen_port_reports_whether_it_emits_anything` exists at all.
const CODEGEN_SYMBOLS: &[(&str, &str)] = &[
    // regalloc.rs — the linear-scan allocator, doc 03 §8 phase 3.4.7
    ("Location::Register", "कोष्ठाधिकरणभेद"),
    ("Location::Spill", "निक्षेपाधिकरणभेद"),
    ("enum Location", "संरचना अधिकरण"),
    ("struct AllocationMap", "संरचना आवण्टनम्"),
    ("allocate_registers", "कोष्ठावण्टनम्"),
    // emit.rs — "T0 text emission, phase 3.4.8" — RETIRED AND DELETED,
    // `W-237` (2026-09-04): its text was not T0 and `सङ्केतन` refused every
    // line (research/25 §1.2). `emit_program`/`कार्यक्रमोत्सर्जनम्` and
    // `format_location`/`अधिकरणवचनम्` (with the operand tail `त्रिपदवचनम्`)
    // left this table with the files. The numeral appender STAYS: it is the
    // output buffer's, `यन्त्रोत्सर्जन ॱ यन्त्राङ्कः` writes through it, and its
    // Rust original is now `riscv64::devanagari` (`RISCV_SYMBOLS` pairs that
    // with `यन्त्राङ्कः`; this row pins the appender it calls).
    (
        "riscv64::devanagari (was emit.rs's to_devanagari_numeral)",
        "देवनागराङ्कः",
    ),
    // x86_64.rs — the AMD64 back end, phase 3.4.11
    ("struct X86_64Emitter", "संरचना यवनोत्सर्जकः"),
    ("X86_64Emitter::new", "यवनोत्सर्जकारम्भः"),
    ("X86_64Emitter::emit_function", "यवनवृत्त्युत्सर्जनम्"),
    ("X86_64Emitter::format_location", "यवनाधिकरणवचनम्"),
    ("X86_64Emitter::reg_name", "कोष्ठनाम"),
];

/// The fields of the codegen's three structs, checked INSIDE their own bodies
/// for the reason `LINKED_FIELDS` gives: a field name loose in the file is a
/// parameter somewhere, not evidence the struct still carries it.
///
/// `अधिकरण` is where `enum Location`'s payloads live. T1 has no tagged union,
/// so `Register(u8)` and `Spill(usize)` become a tag plus two fields, and
/// **the fields are the only place the payloads survive** — without them the
/// symbol check above would stay green over a `Location` that had become a
/// bare enum and lost the register number it exists to carry.
const CODEGEN_FIELDS: &[(&str, &[(&str, &str)])] = &[
    (
        "अधिकरण",
        &[
            ("Location: the tag", "भेद"),
            ("Location::Register(u8)", "कोष्ठ"),
            ("Location::Spill(usize)", "निक्षेप"),
        ],
    ),
    (
        "आवण्टनम्",
        &[
            ("AllocationMap::locations", "अधिकरणसंख्यान"),
            ("AllocationMap::num_spills", "निक्षेपसंख्यान"),
        ],
    ),
    ("यवनोत्सर्जकः", &[("X86_64Emitter::output", "निर्गमः")]),
];

fn codegen_text() -> String {
    std::fs::read_to_string(crate_src().join("utsarjana.t1")).expect("utsarjana.t1 exists")
}

/// The stub marker every port in this crate uses: a label and a bare return.
const CODEGEN_STUB: &str = "प्रत्यागमनम् उक्तम् अपूर्णम् इति";

/// The body of one `सार्वजनिक वृत्तिः NAME … इति`, or `None` when the file
/// declares no such routine.
///
/// It ends at the first `इति` in COLUMN ZERO. Every nested `यदि`/`यावत्` close
/// in these sources is indented and the stub's own `इति` is preceded by a
/// space, so the unindented one is the routine's own close and nothing else is.
/// Cutting at the next `सार्वजनिक` instead would swallow the prose footnotes
/// after the last routine in the file.
fn routine_body(text: &str, name: &str) -> Option<String> {
    let (_, rest) = text.split_once(&format!("सार्वजनिक वृत्तिः {name} "))?;
    Some(match rest.split_once("\nइति") {
        Some((body, _)) => body.to_string(),
        None => rest.to_string(),
    })
}

#[test]
fn the_codegen_port_declares_every_symbol_of_the_rust_original() {
    let text = codegen_text();
    // Matched against the DECLARATION, never against the file, for the reason
    // written out above `the_encoder_port_declares_…`: a bare `contains` is
    // satisfied by a prose comment, and this file's header names most of all
    // three Rust originals in English.
    let declared = |t1: &str| {
        text.contains(&format!("सार्वजनिक {t1} "))
            || text.contains(&format!("सार्वजनिक वृत्तिः {t1} "))
            || text.contains(&format!("सार्वजनिक चरः {t1} "))
    };
    let mut missing: Vec<String> = CODEGEN_SYMBOLS
        .iter()
        .filter(|(_, t1)| !declared(t1))
        .map(|(rust, _)| (*rust).to_string())
        .collect();

    for (name, fields) in CODEGEN_FIELDS {
        match struct_body(&text, &format!("सार्वजनिक संरचना {name} आरभ्य"))
        {
            None => missing.push(format!("struct {name}")),
            Some(body) => missing.extend(
                fields
                    .iter()
                    .filter(|(_, t1)| !body.contains(&format!("{t1} ॱॱ ")))
                    .map(|(rust, _)| (*rust).to_string()),
            ),
        }
    }

    // The module name, for the reason `artha.t1` is checked the same way: the
    // one defect of `tools/port_encode.py` that no symbol list can catch is the
    // right text under the wrong module. Doc 03 §2's own component table names
    // the codegen रूपकार, not उत्सर्जन; the file records that disagreement in
    // its header and keeps उत्सर्जन because `ir.t1` line 8 already calls it
    // that. If the rename ever happens, this line and that one move together.
    if !text.starts_with("मण्डलम् उत्सर्जन ") {
        missing.push("मण्डलम् उत्सर्जन".to_string());
    }

    let total =
        CODEGEN_SYMBOLS.len() + CODEGEN_FIELDS.iter().map(|(_, f)| f.len()).sum::<usize>() + 1;
    println!(
        "METRIC sadhana_t1_codegen_symbols {}",
        total - missing.len()
    );
    assert!(
        missing.is_empty(),
        "utsarjana.t1 does not declare {} of {} codegen symbols: {}",
        missing.len(),
        total,
        missing.join(", ")
    );
}

#[test]
fn the_codegen_port_reports_how_much_of_it_is_real() {
    // The same anti-gutting count as the encoder's, the linker's and the IR's.
    let text = codegen_text();
    let functions = text.matches("सार्वजनिक वृत्तिः").count();
    let stubs = text.matches(CODEGEN_STUB).count();
    let real = functions - stubs;
    println!("METRIC sadhana_t1_codegen_functions {functions}");
    println!("METRIC sadhana_t1_codegen_stubs {stubs}");
    println!("METRIC sadhana_t1_codegen_real {real}");
    // Ratchet. Raise it as stubs are replaced; it may never fall — except
    // by DELETION SAID OUT LOUD, which is what happened once:
    // ३६ on 2026-08-30, from २० — the last three stubs landed (see
    // `the_codegen_emitters_are_the_seven_this_file_accounts_for`)
    // and the output-buffer appenders they needed are routines of this file
    // too, so the count moved by more than three.
    // ३६ → ३२ on 2026-09-04, `W-237`: `कार्यक्रमोत्सर्जनम्`, `अधिकरणवचनम्`,
    // `त्रिपदवचनम्` — the retired T0 pair's three routines, whose text was not
    // T0 — and `टिप्पनीचिह्नयोजनम्`, the comment mark only the first of them
    // called, were deleted with `emit.rs`. Real bodies removed, not stubbed:
    // `stubs` below is still ०, which is the difference this ratchet guards.
    // Read from the failing assertion: `only 32 of 32`.
    assert!(
        real >= 32,
        "only {real} of {functions} codegen routines have a real body (was 36 before W-237 deleted four)"
    );
    // The other direction, and it is now ZERO rather than seven: every routine
    // of `regalloc.rs`, `emit.rs` and `x86_64.rs` has a body. A stub appearing
    // means a routine LOST one, which is the only thing this line still guards.
    assert_eq!(
        stubs, 0,
        "{stubs} codegen routines are stubs; there were seven, and D-002e's \
         last three landed on 2026-08-30 — a stub here now means a body was \
         removed"
    );
}

#[test]
fn the_codegen_port_reports_whether_it_emits_anything() {
    // **This is the test that stops the two above from reading as "done",**
    // and it is here because the IR port needed the same one.
    //
    // Between them the two tests above will say every symbol of all three Rust
    // files is declared and that thirty-six routines have real bodies — both
    // true — while neither can see whether one character of T0 or of AMD64
    // assembly comes out. `codegen_emitters_real` can, and it is now ७.
    //
    // **THE HISTORY OF THIS NUMBER IS THE POINT AND IS KEPT.** It was ० while
    // T1 had no string literal at all; ADR-0017 made `उक्तम् … इति` one token
    // and took it to ३; ADR-0029 decided what an AMD64 register name is and
    // took it to ४; and on 2026-08-30 the owner's newline decision plus this
    // row's reading of ADR-0029 for the AT&T punctuation took it to ७. Each
    // step is a DECISION with a document, which is why this count was pinned
    // rather than floored — see the assertion at the foot of this test.
    //
    // **AND THE COUNT STILL SAYS NOTHING ABOUT WHICH SEVEN.**
    // `the_codegen_emitters_are_the_seven_this_file_accounts_for` at the foot
    // of this file names each one with the fragment that makes it that
    // routine's real body, because a count is satisfied by any seven and a
    // commit that stubbed `देवनागराङ्कः` while adding an eighth emitter would
    // hold this number and lose the whole account.
    //
    // **WHAT A COUNT OF SEVEN DOES NOT MEAN.** The emitters run and their
    // octets are asserted in `tests/t1_execution.rs`; `कोष्ठावण्टनम्` — the
    // allocator that feeds them — still cannot be run over a real function,
    // and `the_allocator_cannot_be_run_over_a_real_function_and_this_is_the_reason`
    // there is where that is recorded. Seven emitters is not a working codegen.
    let text = codegen_text();

    // Which of `Location`'s two variants is CONSTRUCTED by anything — looked
    // for at the call site that would build one, not at the declaration.
    const KINDS: &[(&str, &str)] = &[
        ("Location::Register", "कोष्ठाधिकरणभेद"),
        ("Location::Spill", "निक्षेपाधिकरणभेद"),
    ];
    let built: Vec<&str> = KINDS
        .iter()
        .filter(|(_, t1)| text.contains(&format!("अधिकरणरचना आरभ्य {t1} ")))
        .map(|(rust, _)| *rust)
        .collect();

    // Every routine of `x86_64.rs` — the ones whose whole purpose is to
    // produce text — plus the numeral appender both back ends write through.
    // SEVEN → FIVE on 2026-09-04, `W-237`: `emit.rs`'s `format_location` and
    // `emit_program` were DELETED with their twins (their text was not T0,
    // research/25 §1.2), so `codegen_emitters_declared` fell by two and
    // `codegen_emitters_real` with it; the third `emit.rs` row,
    // `to_devanagari_numeral`, is kept under its new original.
    const EMITTERS: &[(&str, &str)] = &[
        ("riscv64::devanagari", "देवनागराङ्कः"),
        ("X86_64Emitter::new", "यवनोत्सर्जकारम्भः"),
        ("X86_64Emitter::emit_function", "यवनवृत्त्युत्सर्जनम्"),
        ("X86_64Emitter::format_location", "यवनाधिकरणवचनम्"),
        ("X86_64Emitter::reg_name", "कोष्ठनाम"),
    ];

    // A DELETED emitter would also have no body, and would otherwise be
    // indistinguishable here from a stubbed one. So absence is checked
    // separately and fails on its own.
    let undeclared: Vec<&str> = EMITTERS
        .iter()
        .filter(|(_, t1)| routine_body(&text, t1).is_none())
        .map(|(rust, _)| *rust)
        .collect();
    let emitting: Vec<&str> = EMITTERS
        .iter()
        .filter(|(_, t1)| routine_body(&text, t1).is_some_and(|b| !b.contains(CODEGEN_STUB)))
        .map(|(rust, _)| *rust)
        .collect();

    println!(
        "METRIC sadhana_t1_codegen_location_kinds_declared {}",
        KINDS.len()
    );
    println!(
        "METRIC sadhana_t1_codegen_location_kinds_built {}",
        built.len()
    );
    println!(
        "METRIC sadhana_t1_codegen_emitters_declared {}",
        EMITTERS.len()
    );
    println!("METRIC sadhana_t1_codegen_emitters_real {}", emitting.len());

    assert!(
        undeclared.is_empty(),
        "utsarjana.t1 declares no routine for {} of {} emitters: {}",
        undeclared.len(),
        EMITTERS.len(),
        undeclared.join(", ")
    );
    assert_eq!(
        built.len(),
        KINDS.len(),
        "only {} of {} Location kinds are constructed by अधिकरणरचना; \
         an allocator that never spills is not the Rust one",
        built.len(),
        KINDS.len()
    );
    // Raised ० → ३ by `F-004f2`/ADR-0017, and MEASURED rather than aimed at:
    // three is how many of the seven need a string literal and nothing else.
    //
    // It stays an `assert_eq!` and does not become a `>=`, for the reason it
    // was pinned at ० rather than floored there — each of the four that remain
    // waits on a DECISION nobody has taken, so a fourth body appearing should
    // stop the build and be explained rather than counted.
    // ३ → ४ on 2026-08-30, and this comment is the record that assertion asks
    // for. It named two ways the count could legitimately move; the SECOND
    // happened. **ADR-0029 decided what an AMD64 register name is in a script
    // with no Latin**: it is Devanagari, no escape hatch, and the names are
    // TRANSLATED BY ROLE rather than transcribed by sound — सञ्चयः for the
    // Accumulator, गणकः for the Counter — because
    // spec/registers-riscv64.tsv was already generated that way (शून्यः for
    // `zero`, स्तूपसूचकः for `sp`) and two register tables in one compiler
    // spelled by two principles is worse than either principle alone.
    //
    // The fourth body is `कोष्ठनाम`, and what unblocked it was the TABLE and
    // not an allocator: वाक्यविभाग:845 set the condition ("spec/…tsv gains a
    // Devanagari column"), spec/registers-amd64.tsv now meets it, and
    // t1_execution.rs checks the routine against THAT FILE rather than against
    // a second copy of the list.
    //
    // ४ → ७ on 2026-08-30, and this comment is the record that assertion asks
    // for. It named two ways the count could legitimately move and BOTH have
    // now happened; ADR-0017 is still untouched, which is the part worth
    // reading twice.
    //
    //   * **THE NEWLINE — the owner's decision of 2026-08-30, and ADR-0018.**
    //     No string literal gained a newline and no escape was invented. A
    //     newline is an OCTET THE EMITTER APPENDS: `उत्सर्जन` now holds one
    //     preallocated `अङ्कः अन्तः अ८` and a cursor (`निर्गमकोश`,
    //     `निर्गमसूचकाङ्क`) and appends U+000A through `यतियोजनम्`, reading the
    //     octet out of ADR-0018's own word `यतिः` rather than typing `१०`.
    //     That is exactly what ADR-0018 says an emitter should do — *"a line
    //     break is written by the emitter as a byte, not smuggled inside
    //     quotes"* — and it clears `कार्यक्रमोत्सर्जनम्` and the newline half
    //     of `यवनवृत्त्युत्सर्जनम्`. **ADR-0028's allocator was NOT needed and
    //     is not a prerequisite**: an appender that writes at a cursor owns no
    //     text twice.
    //   * **THE LATIN — ADR-0029, carried one level down from the register
    //     names to the punctuation.** ADR-0029 settled the fourteen names on
    //     2026-08-30 (that is what took the count from ३ to ४) and settled
    //     nothing about `%`, `$`, `(`, `)` or `-`, which is what still blocked
    //     `यवनाधिकरणवचनम्` and `यवनवृत्त्युत्सर्जनम्`. `utsarjana.t1`'s
    //     `HOW AT&T PUNCTUATION IS WRITTEN IN THIS REPERTOIRE` takes that
    //     decision by citation and not by coinage: `(` and `)` are doc 15
    //     §6.2's C-15-2 keywords `आरभ्य`/`समाप्तम्` and `,` is its `ऽ`;
    //     `-` is the frozen numeral production's `ऋण`; `%` and `$` are not
    //     rendered, because translated by ROLE they are disambiguators inside
    //     one namespace and the operand shapes already carry the role.
    //     `crates/sadhana-t1/tests/t1_execution.rs` asserts that not one octet
    //     of the emitted AMD64 text is Latin or ASCII punctuation.
    //
    // **This assertion stays an `assert_eq!` even at seven of seven.** It is
    // not a floor that has been reached: a count that could only rise would
    // stop saying anything the moment it did, and the reason it was pinned —
    // that a body appearing must be EXPLAINED — applies just as much to one
    // disappearing. `the_three_real_codegen_emitters_…` below is what says
    // WHICH seven and why each one could be written.
    //
    // ७ → ५ ON 2026-09-04, `W-237`, AND THIS IS THE "SAY WHICH AND WHY" THE
    // MESSAGE BELOW ASKS FOR — the one time the count FELL. Two routines
    // disappeared because they were DELETED with `emit.rs`, not stubbed:
    // `अधिकरणवचनम्` (`format_location`) and `कार्यक्रमोत्सर्जनम्` (`emit_program`),
    // the retired T0 pair whose text `सङ्केतन` refused on every line
    // (research/25 §1.2, `W-236`'s header note). Read from the failing
    // assertion: `left: 5, right: 7`. The five that remain are named in
    // `CODEGEN_EMITTERS_LANDED`, and the RISC-V emitter that reaches the
    // machine is `RISCV_EMITTERS_LANDED`'s.
    assert_eq!(
        emitting.len(),
        5,
        "the codegen emitter count moved off ५ ({}); if it fell, a routine \
         lost its body — say which and why. If a new emitter was added, name \
         the Rust routine it ports and add it to CODEGEN_EMITTERS_LANDED",
        emitting.join(", ")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The encoder's THIRD number — task `D-002a2`.
//
// `the_encoder_port_declares_every_symbol_of_the_rust_original` and
// `the_encoder_port_reports_how_much_of_it_is_real` are both above, and
// between them they now say that all 36 symbols are declared and that 40 of
// 54 routines have real bodies. Both are true. **`encode.t1` still assembles
// nothing at all** — no program, no instruction, and until this session not
// even a word.
//
// That gap is the exact failure the IR port hit (`the_ir_port_reports_which_
// instruction_kinds_are_actually_built`) and the codegen port hit
// (`the_codegen_port_reports_whether_it_emits_anything`), and it is worse
// here, because this row's ratchet moved from 15 to 40 in one commit. A
// reader looking only at that number would call `D-002a2` done.
// ─────────────────────────────────────────────────────────────────────────

fn encoder_text() -> String {
    std::fs::read_to_string(crate_src().join("encode.t1")).expect("encode.t1 exists")
}

/// The stub marker every port in this crate uses; the same string
/// `CODEGEN_STUB` names, spelled once more here so the encoder's own tests
/// do not silently follow a rename made for the codegen's.
const ENCODER_STUB: &str = "प्रत्यागमनम् उक्तम् अपूर्णम् इति";

/// The four routines that must read a DERIVED table — blocker (a).
///
/// Every one of `spec/encodings-riscv64.tsv`, `registers-riscv64.tsv`,
/// `fence-domains-riscv64.tsv` and `compression-choices.tsv` is generated by
/// asking the assembler, and `encode.t1` deliberately does not hand-copy any
/// of them: a typed second copy agrees with the first by construction and
/// checks nothing. So these stay stubs until T1 can open a file, and this
/// list is pinned at ० rather than ratcheted.
const ENCODER_TABLE_READERS: &[(&str, &str)] = &[
    ("encodings", "सङ्केताः"),
    ("register", "कोष्ठाङ्कः"),
    ("domain_set", "क्षेत्रसमूहः"),
    ("forms_of", "रूपाणि"),
];

/// The routines that take an `Instruction` or a `Program` — blocker (b).
///
/// **Neither type exists in any `.t1` file in this tree.** `parse.t1` is
/// `व्याकर`, the T1 *language* parser, and holds `घोषणा` and `व्याकरदोष`;
/// the Sassembly instruction reader `crates/sadhana/src/parse.rs` has not
/// been ported at all. Their T1 declarations take a byte slice as a
/// placeholder so the file is well formed, which is precisely why a
/// signature count cannot see the hole and this list can.
///
/// **These eight are the whole of what an assembler is for.** While every
/// one is ०, nothing in `encode.t1` turns a written instruction into an
/// image, whatever the other two tests report.
const ENCODER_PROGRAM_ROUTINES: &[(&str, &str)] = &[
    ("encode", "सङ्केतनम्"),
    ("encode_at", "स्थानसङ्केतनम्"),
    ("compressed_at", "सङ्कोचः"),
    ("encode_program", "कार्यक्रमसङ्केतनम्"),
    ("encode_program_for", "लक्ष्यसङ्केतनम्"),
    ("encode_object", "वस्तुसङ्केतनम्"),
    ("encode_object_for", "लक्ष्यवस्तुसङ्केतनम्"),
    ("layout_addresses", "स्थानविन्यासः"),
];

/// The per-row work the four table readers above would do once they have
/// text — Rust's `lines`, `split('\t')`, `parse` and `from_str_radix`.
///
/// Listed and required REAL, not merely declared. The argument `encode.t1`
/// makes for its own ratchet moving 15 → 40 is that the table readers are
/// blocked on a file reader *and on nothing else*; if one of these were to
/// vanish or become a stub that claim would be false, and the `real` count
/// alone would not say which half had gone.
const ENCODER_ROW_READERS: &[(&str, &str)] = &[
    ("lines", "पङ्क्तिसीमा"),
    (
        "filter(!starts_with('#') && !trim().is_empty())",
        "उपेक्ष्यपङ्क्तिः",
    ),
    ("f.len()", "क्षेत्रसंख्या"),
    ("f[n] start", "क्षेत्रारम्भः"),
    ("f[n] end", "क्षेत्रसीमा"),
    ("from_str_radix(_, 16)", "षोडशाङ्कमूल्यम्"),
    ("parse::<u64>()", "दशाङ्कमूल्यम्"),
];

/// The routines that build an instruction WORD.
///
/// Measured at the one place a word can begin: `सङ्केतः ॱ आकृति`, an
/// encoding's fixed bits. Everything else in the file reads a slot, places
/// one field, or measures a range — none of which is an instruction.
/// `संयोजन` reads `ॱ आकृति` in `पदपुनर्निर्माणम्`; before this session
/// `encode.t1` did not read it anywhere, so the module that exists to encode
/// was the one module in the pipeline that never started from a pattern.
const ENCODER_WORD_BUILDERS: &[(&str, &str)] = &[("encode_collecting's placement loop", "पदरचना")];

#[test]
fn the_encoder_port_reports_whether_it_encodes_anything() {
    let text = encoder_text();

    // A DELETED routine also has no body and would otherwise be
    // indistinguishable from a stubbed one, so absence is checked separately
    // and fails on its own — the same split `the_codegen_port_reports_whether_
    // it_emits_anything` makes, and for the same reason.
    let undeclared = |list: &[(&str, &str)]| -> Vec<String> {
        list.iter()
            .filter(|(_, t1)| routine_body(&text, t1).is_none())
            .map(|(rust, _)| (*rust).to_string())
            .collect()
    };
    let real = |list: &[(&str, &str)]| -> Vec<String> {
        list.iter()
            .filter(|(_, t1)| routine_body(&text, t1).is_some_and(|b| !b.contains(ENCODER_STUB)))
            .map(|(rust, _)| (*rust).to_string())
            .collect()
    };

    let missing: Vec<String> = undeclared(ENCODER_TABLE_READERS)
        .into_iter()
        .chain(undeclared(ENCODER_PROGRAM_ROUTINES))
        .chain(undeclared(ENCODER_ROW_READERS))
        .chain(undeclared(ENCODER_WORD_BUILDERS))
        .collect();

    let tables = real(ENCODER_TABLE_READERS);
    let programs = real(ENCODER_PROGRAM_ROUTINES);
    let rows = real(ENCODER_ROW_READERS);
    // Not "has a body" — has a body that starts from a PATTERN. A routine
    // could keep its name, keep a real body and stop assembling anything,
    // and that is the mutation this line exists to catch.
    let words: Vec<String> = ENCODER_WORD_BUILDERS
        .iter()
        .filter(|(_, t1)| routine_body(&text, t1).is_some_and(|b| b.contains("ॱ आकृति")))
        .map(|(rust, _)| (*rust).to_string())
        .collect();

    println!(
        "METRIC sadhana_t1_encoder_table_readers_declared {}",
        ENCODER_TABLE_READERS.len()
    );
    println!(
        "METRIC sadhana_t1_encoder_table_readers_real {}",
        tables.len()
    );
    println!(
        "METRIC sadhana_t1_encoder_program_routines_declared {}",
        ENCODER_PROGRAM_ROUTINES.len()
    );
    println!(
        "METRIC sadhana_t1_encoder_program_routines_real {}",
        programs.len()
    );
    println!(
        "METRIC sadhana_t1_encoder_row_readers_declared {}",
        ENCODER_ROW_READERS.len()
    );
    println!("METRIC sadhana_t1_encoder_row_readers_real {}", rows.len());
    println!(
        "METRIC sadhana_t1_encoder_word_builders_declared {}",
        ENCODER_WORD_BUILDERS.len()
    );
    println!(
        "METRIC sadhana_t1_encoder_word_builders_real {}",
        words.len()
    );

    assert!(
        missing.is_empty(),
        "encode.t1 declares no routine for {}: {}",
        missing.len(),
        missing.join(", ")
    );

    // The row parser is the claim that blocker (a) is a FILE READER and
    // nothing else. Every one of the seven must be real for that to be true.
    assert_eq!(
        rows.len(),
        ENCODER_ROW_READERS.len(),
        "only {} of {} per-row table readers are real; blocker (a) is then \
         more than a missing file reader and encode.t1 says otherwise",
        rows.len(),
        ENCODER_ROW_READERS.len()
    );

    // Ratchet. `पदरचना` is the first routine in this module that produces an
    // instruction, and it landed with this test. It may never fall.
    assert!(
        !words.is_empty(),
        "no routine in encode.t1 reads सङ्केतः ॱ आकृति; the encoder starts \
         from no pattern and therefore assembles no instruction"
    );

    // Pinned at ०, exactly as `codegen_emitters_real` is. These rising is
    // GOOD NEWS and the test will still fail, which is the point: raise the
    // floor here, and say in the same commit which blocker went away —
    // because (a) is a file reader T1 does not have and (b) is a whole
    // unported module, and neither should pass unremarked.
    // RAISED ० -> १ ON 2026-08-29 BY `D-002a2a`, AND HERE IS THE RECORD THIS
    // ASSERTION ASKED FOR. `register` (`कोष्ठाङ्कः`) is real. A `.t1` source
    // came to read `spec/*.tsv` by ADR-0019's embed — `समावेशः आरभ्य कोष्ठकोशः
    // समाप्तम्`, owner-ratified 2026-08-28, landed by `D-002g` — which resolves
    // the registered NAME `कोष्ठकोशः` to `spec/registers-riscv64.tsv` at
    // assembly time and yields its bytes as a string. So blocker (a) is no
    // longer a missing mechanism; the three readers still at ० are work.
    // The floor stays an `assert_eq!` rather than a `>=` for the reason it
    // always was: each one that rises should have to say in its own commit
    // which table it read and how.
    // RAISED 1 -> 2 by `D-002i2`, and this comment is the record the assertion
    // demands. The second reader is `क्षेत्रसमूहः`, the fence ordering-domain
    // table, reaching `spec/fence-domains-riscv64.tsv` through `क्षेत्रकोशः`.
    // It is the first to prove the embed is a MECHANISM and not one
    // arrangement: `कोष्ठाङ्कः` could have been a special case, two cannot.
    // It also needed the range idiom — its argument is a SET of names joined
    // by `ऽ`, so `क्षेत्रखण्डसाम्यम्` compares against a RANGE of the subject
    // rather than the whole of it, since T1 has no subslice expression.
    // RAISED 2 -> 4 on 2026-08-30, and this is the record the assertion asks
    // for. The two new readers are `सङ्केताः` (spec/encodings-riscv64.tsv) and
    // `रूपाणि` (spec/compression-choices.tsv), and HOW they came to read a
    // spec table is the whole of ADR-0031: both used to return a WHOLE TABLE —
    // `अङ्कः अन्तः सङ्केत`, and a slice of slices — and both now return the
    // ROW the caller asked for, through the range idiom, with no collection.
    //
    // ADR-0026 had already enumerated every Rust call site and found them all
    // row searches (`vishlesana.rs:182` is literally a `.find` over one row),
    // so this is a deliberate divergence from the Rust signature and not a
    // convenience. It closes encoder blocker (a) entirely.
    //
    // Note what did NOT clear it: not an allocator. T1's `push` was proven the
    // same day, so the table COULD have been built; the decision was that
    // nothing should build one so a caller can read one row of it.
    assert_eq!(
        tables.len(),
        4,
        "{} table reader(s) now have real bodies ({}); raise this floor and \
         record how a .t1 source came to read spec/*.tsv",
        tables.len(),
        tables.join(", ")
    );
    // RAISED ० -> १ ON 2026-08-31, AND HERE IS THE RECORD THIS ASSERTION
    // ASKED FOR. The routine is `layout_addresses` (`स्थानविन्यासः`).
    //
    // WHERE `कार्यक्रम` CAME FROM — which is the question this message asks,
    // and its premise, "no .t1 file in this tree declares either today", was
    // already stale when it was written. `वाक्यविभाग` declares BOTH:
    // `आज्ञा` at vakyavibhaga.t1:178 and `कार्यक्रम` at :287. What was
    // genuinely missing was a way to BUILD one, and that is
    // `कार्यक्रमरचना` (:595), which seals the runs the six appenders filled
    // and returns the record. `t1_exec_encode.rs` pushes instructions and
    // `॥ संरेखः ॥` requests through those appenders, seals them, and asserts
    // the addresses — so this routine is checked against a program and not
    // against itself, which is the failure that reopened `D-002a`.
    //
    // The OTHER half was `गणना`: `असङ्कुचितम्` was not a name in scope until
    // `read_head`'s `W_ENUM` arm (nirvahana.rs:1185) began registering
    // variants, so a routine taking a `लक्ष्य` could not be written at all.
    //
    // RAISED १ -> ३ ON 2026-08-31, AND THE BLOCKER THAT WENT AWAY WAS VOLUME.
    // The two that moved are `encode_at` (`स्थानसङ्केतनम्`) and `encode`
    // (`सङ्केतनम्`), and the first is the keystone: it is
    // `encode_collecting`, encode.rs:1127, all seventeen diagnostic sites and
    // the operand loop over kāraka rôles. Its own margin called it "THE ONLY
    // ONE OF THE EIGHT WITH NO BLOCKER BUT VOLUME", and that was accurate.
    //
    // WHAT IT COST, since the margin asked to be corrected if it was wrong:
    // the two SIGNATURE debts it recorded — `आज्ञा` as a byte-slice
    // placeholder, and Rust's `symbols` with no parameter at all — were both
    // real and both paid. The symbol table became `सङ्केतन ॱ चिह्नस्थान`, an
    // arena and a linear search, which is the shape the margin proposed.
    //
    // WHAT THIS NUMBER STILL CANNOT SEE, and the reason it is ३ and not more:
    //
    //   * `compressed_at` (`सङ्कोचः`) is NOT one of the delegations. It is a
    //     separate Rust function (encode.rs:764) with its own table —
    //     spec/compression-choices.tsv — and its own decode-and-replace
    //     method; `encode_collecting` landing does not write it.
    //   * The four program-level routines are not delegations either, though
    //     the record above called them that. `encode_program_for`
    //     (encode.rs:519) is a two-pass emit loop with a relaxation fixpoint,
    //     an E22 divergence check and an E23 layout-versus-emission check,
    //     and `encode_object_for` adds the `pending` arm. Each is a real body
    //     over `स्थानसङ्केतनम्`, not a call to it, and each needs a growable
    //     octet run this language does not have — `पाठयोगः` copies, and a
    //     text section built that way is quadratic in its own length.
    //
    // So the seven did NOT move together, and the reason is that only two of
    // them were ever the same body.
    // RAISED 3 -> 5 ON 2026-08-31, and here is which blocker went away.
    //
    // `लक्ष्यसङ्केतनम्` (encode_program_for) is written, and
    // `कार्यक्रमसङ्केतनम्` (encode_program) fell out of it as the one line it
    // always was. The note above is now HALF WRONG and is left standing so the
    // correction is visible: the two-pass body, the relaxation fixpoint, the
    // E22 and the E23 are all real, but "needs a growable octet run this
    // language does not have" was never true. An INDEXED WRITE into one
    // `अङ्कः अन्तः अ८` grows it in place — `सङ्ख्यापाठः` in the same file had
    // been doing exactly that all along. `पाठयोगः` copies and is quadratic,
    // which is a reason not to use `पाठयोगः`, not a reason the section cannot
    // be built. That is the fifth blocker this month that described the
    // mechanism it claimed was missing.
    //
    // What did NOT move: `लक्ष्यवस्तुसङ्केतनम्` still owes the `pending`
    // convention (owner-ruled 2026-08-31: module-state arena and a flag, so
    // `स्थानसङ्केतनम्` keeps its signature), and `वस्तुसङ्केतनम्` is the one
    // line waiting on it. `सङ्कोचः` is independent of all four.
    // RAISED 5 -> 7 ON 2026-09-01, and here is which blocker went away.
    //
    // `लक्ष्यवस्तुसङ्केतनम्` (encode_object_for) is written, and
    // `वस्तुसङ्केतनम्` fell out of it as one line. The blocker was the
    // `pending` convention: an OBJECT may name what it does not define and an
    // executable may not, and Rust tells those apart with
    // `Option<&mut Vec<Pending>>` threaded through. T1 has no such argument.
    // The owner ruled on 2026-08-31 that it is MODULE STATE AND A FLAG rather
    // than a third change to `स्थानसङ्केतनम्`'s signature, which two rows had
    // already settled — so `प्रतीक्षाग्रहणम्` is read at the two E05 sites and
    // an undefined name is recorded with the field left zero instead of refused.
    //
    // A SECOND DIFFERENCE THE OLD NOTE DID NOT NAME, measured from
    // encode.rs:907: the object path builds symbols from TEXT LABELS ONLY,
    // because an object's data and bss addresses belong to the linker.
    // Resolving them here would bake this object's own layout into a field
    // someone else must fill — wrong AND plausible, the E23 class.
    //
    // WHAT IS LEFT IS `सङ्कोचः` ALONE, and it is independent of all seven:
    // nothing selects a compressed form yet, so every width is Rust's own
    // `_ => 4` and `लक्ष्यम्` is read by no layout.
    // RAISED 7 -> 8 ON 2026-09-04, `D-002a2`, AND THE EIGHT ARE ALL REAL.
    //
    // `compressed_at` (`सङ्कोचः`) is written, and NO blocker went away to
    // let it — the one the row named had already gone. The row said blocker
    // (a) was "table readers that need growable collections or a bitwise
    // operator T1 lacks"; measured, `पदरचना` had been writing `विकल्प` (OR)
    // and `vishlesana.t1` `युक्`/`दक्षिणसृ` since 2026-08-30, and every
    // collection the body needs is a flat `अङ्कः अन्तः` arena. What the
    // routine waited on was its CALLER CHAIN, and that landed with it:
    // `विन्यासावृत्तिः` takes the target and the symbol table, `उत्सर्जनक्रमः`
    // takes the target and writes the low halfword, `स्थानविन्यासः` rebuilds
    // the text-label table every round (encode.rs:1814). Three helpers came
    // with the body — `अभिधानसङ्केतः` (a row by name and width, as a `सङ्केत`),
    // `सम्बन्धाङ्कः` (the relation column onto the six constants, octet by
    // octet) and `रूपसङ्कोचः` (one form tried) — plus `आज्ञाविस्तारः`, which
    // is `width_of_for` and was never in this list because Rust keeps it
    // `pub(crate)`.
    //
    // ASSERTED AGAINST TWO ORACLES in `t1_exec_encode.rs`: Rust's own
    // `compressed_at` and `layout_addresses(.., Compressed)` on the same
    // parsed instructions, and the halfwords `riscv64-elf-as` produced for
    // `crates/sadhana/tests/compressed.rs`'s six-instruction program — whose
    // forward branch compresses only on the SECOND relaxation round, which is
    // the first time this fixpoint has iterated since `B-007` built it.
    assert_eq!(
        programs.len(),
        8,
        "{} of the eight routines that assemble a program have real bodies \
         ({}); all eight landed by 2026-09-04, so a lower number here means \
         a body was removed",
        programs.len(),
        programs.join(", ")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `D-002e`, continued — WHICH emitters are real, and WHICH DECISION made each
// one writable. All seven are real as of 2026-08-30; the blocked list below is
// empty and says what emptied it.
//
// `the_codegen_port_reports_whether_it_emits_anything` COUNTS, and a count of
// seven is satisfied by any seven. **These seven are not interchangeable.**
// Each row below carries a FRAGMENT that only a real body of that routine
// contains, and each fragment is the specific thing the decision delivered:
// `उक्तम् ऋण इति` is the sign ADR-0017 could not put in a literal as `-`,
// `उक्तम् सञ्चयः इति` is ADR-0029's translate-by-role name, `यतियोजनम्` is the
// appended newline, and `पाठयोजनम् उक्तम् आरभ्य इति` is C-15-2's keyword for a
// parenthesis. So a commit that stubbed `देवनागराङ्कः` and added an eighth
// emitter, or that quietly reverted `सञ्चयः` to a transcription like `रक्स्`,
// or that put a `\n` back inside a literal, would hold the number and lose the
// entire account — and fails here by name instead.
//
// **THE HISTORY IS THE VALUE OF THIS FILE AND IS NOT DELETED WHEN A ROW
// MOVES.** Each landed row keeps the note of what it waited on, because "it
// works now" and "here is the decision that made it possible" are different
// facts and only the second survives being read a month later.
// ─────────────────────────────────────────────────────────────────────────

/// The seven emitters, each with the decision that unblocked it: the T1 name,
/// a fragment only a real body of *that* routine contains, and the Rust
/// original.
///
/// The fragment is what makes this more than a second absence check. Each one
/// is the specific thing that made the routine writable, so a body that lost it
/// fails by name rather than passing as "real".
const CODEGEN_EMITTERS_LANDED: &[(&str, &str, &str)] = &[
    // `-` is U+002D and outside the doc 15 repertoire, so Rust's `'-' => '-'`
    // arm cannot be ported. `ऋण` is the frozen `numeral` production's own sign
    // (`[ "ऋण" ] , ( binary | … )`), so this fragment is the evidence that the
    // one unportable character was ANSWERED and not quietly dropped.
    ("देवनागराङ्कः", "उक्तम् ऋण इति", "to_devanagari_numeral"),
    // `अधिकरणवचनम्` ("उक्तम् कोष्ठ इति", `emit::format_location`) WAS HERE and
    // is gone — `W-237`, 2026-09-04, with `emit.rs`: the text it spelled was
    // not T0, and the register names the machine reads are `riscv64.rs`'s
    // `register_name` / `यन्त्रकोष्ठनाम` (`RISCV_EMITTERS_LANDED`).
    // `String::new()` is the EMPTY literal, which is the whole reason this is
    // the one routine of `x86_64.rs`'s four the Latin blocker does not reach:
    // it is the only one that emits no target text.
    // ADR-0029: the AMD64 names are Devanagari with no Latin escape, and they
    // are TRANSLATED BY ROLE rather than transcribed by sound, because
    // spec/registers-riscv64.tsv was already generated that way (शून्यः for
    // `zero`). `rax` is the Accumulator, so it is `सञ्चयः` — and that literal
    // is the fragment, because a body that reverted to a transcription like
    // `रक्स्` would still be "real" by any count and must fail by name here.
    // The names themselves are checked against spec/registers-amd64.tsv, not
    // against this list, by t1_execution.rs.
    ("कोष्ठनाम", "उक्तम् सञ्चयः इति", "X86_64Emitter::reg_name"),
    (
        "यवनोत्सर्जकारम्भः",
        "निर्गमः भवति उक्तम् इति",
        "X86_64Emitter::new",
    ),
    // ── the last three, landed 2026-08-30 ──
    //
    // `कार्यक्रमोत्सर्जनम्` ("यतियोजनम्", `emit_program`) WAS HERE and is gone —
    // `W-237`, 2026-09-04. What it waited on — the owner's decision of
    // 2026-08-30 that a newline is a byte the emitter appends (ADR-0018's own
    // sentence) — is still the rule, and `यतियोजनम्` is still the call; the
    // fragment now lives in `यवनवृत्त्युत्सर्जनम्`'s row below and in every
    // line `यन्त्रोत्सर्जन` writes. The routine itself emitted a text that was
    // not T0 (research/25 §1.2), and the census that reaches the machine
    // (`crates/yantra/tests/paradigm_encode.rs`) is why it could go.
    // `format_location` returns `-{n}(%rbp)`, and `(` was the character that
    // stopped it before a register name was even reached. The fragment is the
    // ANSWER: doc 15 §6.2's C-15-2 — the binding symbol table, §3.1 having been
    // superseded when A-033 measured ० of २७ fonts for `꣼ ꣸` — assigns group
    // open and close to the keywords `आरभ्य` and `समाप्तम्`. So the paren is a
    // word this language already uses for a bracket, which is P-1 applied where
    // it was already applied, and a body that reached for `(` or `%` instead
    // would be the Latin escape ADR-0029 refused to open.
    (
        "यवनाधिकरणवचनम्",
        "पाठयोजनम् उक्तम् आरभ्य इति",
        "X86_64Emitter::format_location",
    ),
    // `emit_function` was blocked on BOTH halves — twelve lines of GNU as
    // syntax, every one ending in a newline. The fragment is the frame
    // register, because that is the part of this decision with a debt against
    // it: `स्तूपसूचकः` is spec/lexicon.src.tsv:394 read straight, but
    // spec/registers-amd64.tsv carries only the fourteen ALLOCATABLE registers
    // and neither rbp nor rsp is among them, so `आधारसूचकः` is derived in the
    // emitter and owes that table a row — see `utsarjana.t1` note (e).
    (
        "यवनवृत्त्युत्सर्जनम्",
        "उक्तम् स्तूपसूचकः इति",
        "X86_64Emitter::emit_function",
    ),
];

/// **EMPTY, as of 2026-08-30, and that is the news rather than an omission.**
///
/// It held four rows, then three; every one is now in `CODEGEN_EMITTERS_LANDED`
/// above with the decision that cleared it named beside it. What cleared them,
/// in the order they fell:
///
///   * `कोष्ठनाम` — **ADR-0029**, which decided that an AMD64 register name is
///     Devanagari with no Latin escape and is TRANSLATED BY ROLE. Not an
///     allocator and not a newline.
///   * `कार्यक्रमोत्सर्जनम्` — **the owner's decision of 2026-08-30**, that a
///     newline is a byte the emitter appends, which is ADR-0018's own rule
///     restated. **ADR-0017 is not amended** and **ADR-0028 is not a
///     prerequisite**: an appender writing at a cursor owns no text twice.
///   * `यवनाधिकरणवचनम्` and `यवनवृत्त्युत्सर्जनम्` — **both decisions
///     together**, plus this row's reading of ADR-0029 one level down from the
///     names to the punctuation, recorded in `utsarjana.t1` under
///     `HOW AT&T PUNCTUATION IS WRITTEN IN THIS REPERTOIRE`.
///
/// **Leave the list here rather than deleting it.** The loop below is what
/// makes a stub's return an EXPLAINED event rather than a silent one, and the
/// next routine of this file that stalls on a decision belongs in it.
const CODEGEN_EMITTERS_BLOCKED: &[(&str, &str)] = &[];

/// FIVE since `W-237` (2026-09-04), and the name keeps its history: the two
/// rows of the retired T0 pair left with `emit.rs` (see the struck rows in
/// `CODEGEN_EMITTERS_LANDED`), and the assertion at the foot says five.
#[test]
fn the_codegen_emitters_are_the_seven_this_file_accounts_for() {
    let text = codegen_text();
    let mut wrong: Vec<String> = Vec::new();

    for (t1, fragment, rust) in CODEGEN_EMITTERS_LANDED {
        match routine_body(&text, t1) {
            None => wrong.push(format!("{rust}: {t1} declares no routine")),
            Some(b) if b.contains(CODEGEN_STUB) => {
                wrong.push(format!("{rust}: {t1} is a stub again"));
            }
            Some(b) if !b.contains(fragment) => wrong.push(format!(
                "{rust}: {t1} has a body but does not write `{fragment}`"
            )),
            Some(_) => {}
        }
    }

    for (t1, blocker) in CODEGEN_EMITTERS_BLOCKED {
        match routine_body(&text, t1) {
            None => wrong.push(format!("{t1}: declares no routine")),
            Some(b) if !b.contains(CODEGEN_STUB) => wrong.push(format!(
                "{t1}: has a body; name the row that cleared its blocker — {blocker}"
            )),
            Some(_) => {}
        }
    }

    println!(
        "METRIC sadhana_t1_codegen_emitters_landed_named {}",
        CODEGEN_EMITTERS_LANDED.len()
    );
    println!(
        "METRIC sadhana_t1_codegen_emitters_blocked_named {}",
        CODEGEN_EMITTERS_BLOCKED.len()
    );

    // The two lists together must still be the whole of `EMITTERS`, or this
    // test would be pinning a subset while calling it the account.
    // 7 → 5 on 2026-09-04, `W-237`: two rows left with `emit.rs`.
    assert_eq!(
        CODEGEN_EMITTERS_LANDED.len() + CODEGEN_EMITTERS_BLOCKED.len(),
        5,
        "the two lists here must cover all five emitters"
    );
    assert!(
        wrong.is_empty(),
        "the codegen's emitters are not the ones this file accounts for:\n  {}",
        wrong.join("\n  ")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `W-236` — THE RISC-V EMITTER'S TWIN, `yantrotsarjana.t1` (यन्त्रोत्सर्जन).
//
// research/25 §3.3: `RISCV_SYMBOLS` names every routine of `riscv64.rs` with its
// T1 twin, and `RISCV_EMITTERS_LANDED` carries, for six of them, THE FRAGMENT
// THAT IS THE DECISION each one carries — so a body that drifted from a
// decision fails by name, where a count of routines would still say "real".
// The agreement itself — the two twins writing the same octets for the same IR
// — is `tests/t1_exec_riscv.rs`; these tables are what pins WHICH routine is
// which, and `emit.rs`/`कार्यक्रमोत्सर्जनम्` above are the retired pair whose
// text is not T0 (their headers say so as of this row; `W-237` deletes them).
// ─────────────────────────────────────────────────────────────────────────

/// Every symbol `crates/sadhana/src/t1/riscv64.rs` defines, in the spelling
/// `यन्त्रोत्सर्जन` gives it. Rust's format-string constants (`ZERO`, `RA`, `SP`,
/// `FINISHER_HI`) have no twin symbol: the T1 file writes them as the literals
/// they are, inside the lines that use them. `impl Display for Refusal` has no
/// twin either — its text is Rust `Debug` output (`BlockId(1)`), which no
/// literal of this language can hold; the refusal record carries the fields.
const RISCV_SYMBOLS: &[(&str, &str)] = &[
    ("ALLOCATABLE", "यन्त्रावण्टनीयम्"),
    ("STACK_BYTES", "यन्त्रस्तूपाष्टकाः"),
    // `W-284` — the record region, PAIRED rather than exempted. A static
    // region that exists only as a Rust `const` and as loose text on the
    // port side is scaffolding wearing the feature's name, and under the
    // standing rule that Sassembly must work without Rust that is the one
    // shape this list exists to refuse. `यन्त्रस्तूपाष्टकाः` above is the
    // precedent: the stack's size is a named constant on both sides.
    ("RECORD_REGION", "यन्त्ररचनाक्षेत्रनाम"),
    ("RECORD_CURSOR", "यन्त्ररचनासूचकनाम"),
    ("RECORD_REGION_OCTETS", "यन्त्ररचनाष्टकाः"),
    ("OWNED_LABELS", "यन्त्रस्वचिह्नमस्ति"),
    ("register_name", "यन्त्रकोष्ठनाम"),
    ("temp", "यन्त्रक्षणिकनाम"),
    ("arg", "यन्त्रार्थनाम"),
    ("devanagari", "यन्त्राङ्कः"),
    ("hex64", "यन्त्रषोडशाङ्कः"),
    ("type Names", "संरचना यन्त्रनाम"),
    ("Names::new", "यन्त्रनामारम्भः"),
    ("Names::insert", "यन्त्रनामयोजनम्"),
    ("Names::get", "यन्त्रनामान्वेषणम्"),
    ("Module::name", "यन्त्रमण्डलनाम"),
    ("Module::entry", "यन्त्रप्रवेशसंज्ञा"),
    ("Refusal::Unreachable", "यन्त्रागम्यनिषेधभेद"),
    ("Refusal::NoTerminator", "यन्त्रावसानहीननिषेधभेद"),
    ("Refusal::TargetNotInFunction", "यन्त्रबाह्यलक्ष्यनिषेधभेद"),
    ("Refusal::UnnamedSymbol", "यन्त्रानामसंज्ञानिषेधभेद"),
    ("Refusal::LabelCollision", "यन्त्रचिह्नसङ्घट्टनिषेधभेद"),
    ("Refusal::ParamAfterCall", "यन्त्राह्वानोत्तरप्राचलनिषेधभेद"),
    ("Refusal::ParamOutsideEntry", "यन्त्रप्रवेशबाह्यप्राचलनिषेधभेद"),
    ("Refusal::FrameTooLarge", "यन्त्रबृहच्चौकटनिषेधभेद"),
    ("Refusal::EntryTakesParameters", "यन्त्रप्रवेशप्राचलनिषेधभेद"),
    ("Refusal::BranchOutOfRange", "यन्त्रदूरशाखानिषेधभेद"),
    // `W-306`: the J-type's own reach. Paired and not recorded RUST_ONLY —
    // the port declares the भेद beside the other ten, in the same order.
    ("Refusal::JumpOutOfRange", "यन्त्रदूरलङ्घननिषेधभेद"),
    // `W-306c`: the narrow store's unnamed width. Paired for the same reason —
    // the port declares the भेद beside the other eleven, in the same order.
    ("Refusal::StoreWidthUnnamed", "यन्त्रानामविस्तारनिषेधभेद"),
    // `V-005`: a value read from the wrong register file. Paired for the same
    // reason — the port declares the भेद beside the other twelve, in order.
    ("Refusal::FileMismatch", "यन्त्रवर्गविरोधनिषेधभेद"),
    ("Err(Refusal)", "यन्त्रनिषेधः"),
    ("routine_label", "यन्त्रवृत्तिचिह्नम्"),
    // `global_label` is `routine_label` for the two symbols `ir.t1` mints and
    // no declaration names — the record cursor and its region (10_000_003/4).
    // ir.t1:543 rules that "the emitters own the label", so both twins spell
    // them instead of looking them up, and both keep the CALL path on
    // `routine_label` so a call into that band still refuses.
    ("global_label", "यन्त्रवैश्विकचिह्नम्"),
    ("block_label", "यन्त्रपर्वचिह्नम्"),
    ("exit_label", "यन्त्रनिर्गमचिह्नम्"),
    ("check_labels", "यन्त्रचिह्नपरीक्षा"),
    ("struct Frame", "संरचना यन्त्रचौकट"),
    ("frame_layout", "यन्त्रचौकटरचना"),
    ("Routine::line", "यन्त्राज्ञान्तः"),
    ("Routine::location", "यन्त्राधिकरणम्"),
    ("Routine::read", "यन्त्रपठनम्"),
    ("Routine::write", "यन्त्रलेखनम्"),
    ("Routine::prologue", "यन्त्रप्रस्तावना"),
    ("Routine::epilogue", "यन्त्रोपसंहारः"),
    ("lower_constant", "यन्त्रध्रुवावतरणम्"),
    // `W-254`: the pooled arm with the load dropped, so the value is the address.
    ("lower_string", "यन्त्रपाठावतरणम्"),
    ("split_hi_lo", "यन्त्रोच्चनीचविभागः"),
    ("emit_call", "यन्त्राह्वानोत्सर्जनम्"),
    ("lower_cond_branch", "यन्त्रशाखावतरणम्"),
    // `W-306`: the inversion the far-conditional relaxation leans on. Paired
    // here and not recorded as Rust-only because the `.t1` half is CONSULTED by
    // its own `यन्त्रशाखावतरणम्` — the crossed table (`न्यून → अचिह्नान्यून`) is
    // the failure this pairing is against.
    ("inverse_condition", "यन्त्रविपरीततुलनाभेदः"),
    ("emit_terminator", "यन्त्रावसानोत्सर्जनम्"),
    ("verify", "यन्त्रपरीक्षा"),
    ("emit_function", "यन्त्रवृत्त्युत्सर्जनम्"),
    ("check_branch_ranges", "यन्त्रशाखादूरपरीक्षा"),
    ("emit_startup", "यन्त्रारम्भोत्सर्जनम्"),
    // `W-243`: the startup OBJECT — the stub and its stack as one text per image.
    ("emit_startup_object", "यन्त्रारम्भमण्डलोत्सर्जनम्"),
    ("emit_data", "यन्त्रदत्तोत्सर्जनम्"),
    ("emit_module", "यन्त्रमण्डलोत्सर्जनम्"),
    ("fixture_recursive_sum", "यन्त्रयोगफलदृष्टान्तः"),
    // `W-245`: the forms that make a status more than zero.
    ("count_locals", "यन्त्रस्थानीयगणना"),
    ("branch_word", "यन्त्रशाखापदम्"),
    ("binary_verb", "यन्त्रद्विपदक्रियापदम्"),
    ("lower_compare_value", "यन्त्रतुलनावतरणम्"),
    ("fusable_compare", "यन्त्रसंयोज्यतुलना"),
    ("fixture_forms", "यन्त्ररूपदृष्टान्तः"),
    // `V-005` — THE SECOND REGISTER FILE, PAIRED. These six were RUST_ONLY rows
    // with the reason "the T1 IR has no float instruction kind, so the port has
    // no float `Location` to spell", and each said to DELETE the row and pair it
    // the day the IR gained one. `ir.t1`'s `प्लवाज्ञाभेद` (२७) and the float read
    // are that day: the port now runs the float scan, lays out the float spill
    // region and saves the `fs` registers it uses, so each has a twin.
    ("class_register_name", "यन्त्रवर्गकोष्ठनाम"),
    ("ALLOCATABLE_FLOAT_ROLE", "यन्त्रप्लवस्थिरसङ्ख्या"),
    ("allocatable", "यन्त्रवर्गावण्टनीयम्"),
    ("registers_held", "यन्त्रधृतकोष्ठाः"),
    ("store_mnemonic", "यन्त्रनिधानपदम्"),
    ("load_mnemonic", "यन्त्राहारपदम्"),
    // and the three `V-005` adds: the classifier both scans share, the op's
    // word, and the one-line lowering.
    ("value_class", "यन्त्रप्लववर्गः"),
    ("float_verb", "यन्त्रप्लवक्रियापदम्"),
    ("lower_float", "यन्त्रप्लवावतरणम्"),
    // `V-008` part 2.
    ("lower_vector", "यन्त्रव्यूहावतरणम्"),
    // `V-009` part (ii): the matrix kernel, its operand and refusal writers.
    ("matrix_kernel", "यन्त्राव्यूहकायोत्सर्जनम्"),
    ("matrix_kernel_operand", "यन्त्राव्यूहपदम्"),
    ("matrix_kernel_refusal", "यन्त्राव्यूहनिषेधः"),
    ("MATRIX_KERNEL_VERBS", "यन्त्राव्यूहक्रियापदम्"),
    ("matrix_kernel_letter", "यन्त्राव्यूहाक्षरमूल्यम्"),
    ("matrix_kernel_operand_code", "यन्त्राव्यूहपदसङ्केतः"),
    ("vector_register", "यन्त्रव्यूहनाम"),
    // `V-005` — the float calling convention: where each argument travels, the
    // same allocation over a routine's parameters, and `fa<n>`'s spelling (the
    // port's fifth register-code band).
    ("abi_slots", "यन्त्रतर्कस्थानानि"),
    ("param_locations", "यन्त्रप्राचलस्थानानि"),
    ("float_arg", "यन्त्रप्लवार्थाधारः"),
];

/// The fields of the two structs, checked inside their own bodies. `Frame::saved`
/// is `Vec<(u8, i64)>` and becomes the two parallel arenas beside the struct
/// (`यन्त्ररक्षितकोष्ठकोश`, `यन्त्ररक्षितस्थानकोश`), T1 having no tuple; the
/// struct carries its length.
const RISCV_FIELDS: &[(&str, &[(&str, &str)])] = &[
    (
        "यन्त्रनाम",
        &[
            ("Names: the key, SymbolId", "संज्ञा"),
            ("Names: the module", "मण्डल"),
            ("Names: the routine", "वृत्तिनाम"),
        ],
    ),
    (
        "यन्त्रचौकट",
        &[
            ("Frame::bytes", "अष्टकाः"),
            ("Frame::saved.len()", "रक्षितसंख्यान"),
            ("Frame::ra_offset", "पुनःस्थानस्थानम्"),
            ("Frame::num_spills", "निक्षेपसंख्यान"),
            ("Frame::num_float_spills", "प्लवनिक्षेपसंख्यान"),
            ("Frame::num_locals", "स्थानीयसंख्यान"),
        ],
    ),
];

fn riscv_text() -> String {
    std::fs::read_to_string(crate_src().join("yantrotsarjana.t1"))
        .expect("yantrotsarjana.t1 exists")
}

#[test]
fn the_riscv_emitter_port_declares_every_symbol_of_the_rust_original() {
    let text = riscv_text();
    let declared = |t1: &str| {
        text.contains(&format!("सार्वजनिक {t1} "))
            || text.contains(&format!("सार्वजनिक वृत्तिः {t1} "))
            || text.contains(&format!("सार्वजनिक चरः {t1} "))
    };
    let mut missing: Vec<String> = RISCV_SYMBOLS
        .iter()
        .filter(|(_, t1)| !declared(t1))
        .map(|(rust, _)| (*rust).to_string())
        .collect();
    for (name, fields) in RISCV_FIELDS {
        match struct_body(&text, &format!("सार्वजनिक संरचना {name} आरभ्य"))
        {
            None => missing.push(format!("struct {name}")),
            Some(body) => missing.extend(
                fields
                    .iter()
                    .filter(|(_, t1)| !body.contains(&format!("{t1} ॱॱ ")))
                    .map(|(rust, _)| (*rust).to_string()),
            ),
        }
    }
    // The module name — research/25 §3.2's derived compound, and the one W-192
    // needs: every routine carries the यन्त्र prefix because the interpreter keys
    // bare names across modules.
    if !text.starts_with("मण्डलम् यन्त्रोत्सर्जन ") {
        missing.push("मण्डलम् यन्त्रोत्सर्जन".to_string());
    }
    let unprefixed: Vec<&str> = text
        .lines()
        .filter_map(|l| {
            l.strip_prefix("सार्वजनिक वृत्तिः ")
                .or_else(|| l.strip_prefix("सार्वजनिक चरः "))
                .and_then(|r| r.split(' ').next())
        })
        .filter(|name| !name.starts_with("यन्त्र"))
        .collect();
    let total = RISCV_SYMBOLS.len() + RISCV_FIELDS.iter().map(|(_, f)| f.len()).sum::<usize>() + 1;
    println!("METRIC sadhana_t1_riscv_symbols {}", total - missing.len());
    assert!(
        missing.is_empty(),
        "yantrotsarjana.t1 does not declare {} of {} riscv64 symbols: {}",
        missing.len(),
        total,
        missing.join(", ")
    );
    assert!(
        unprefixed.is_empty(),
        "routines or globals of यन्त्रोत्सर्जन without the यन्त्र prefix (W-192 keys bare names): {unprefixed:?}"
    );
}

#[test]
fn the_riscv_emitter_port_reports_how_much_of_it_is_real() {
    let text = riscv_text();
    let functions = text.matches("सार्वजनिक वृत्तिः").count();
    let stubs = text.matches(CODEGEN_STUB).count();
    println!("METRIC sadhana_t1_riscv_functions {functions}");
    println!("METRIC sadhana_t1_riscv_stubs {stubs}");
    println!("METRIC sadhana_t1_riscv_real {}", functions - stubs);
    // Landed whole: no routine of this file was ever a stub, and a stub appearing
    // means a body was removed.
    assert_eq!(stubs, 0, "{stubs} routines of यन्त्रोत्सर्जन are stubs");
    assert!(
        functions >= 45,
        "only {functions} routines; riscv64.rs has more twins than that"
    );
}

/// Six routines, each with THE FRAGMENT THAT IS THE DECISION it carries
/// (research/25 §3.3), and the Rust original. A body that lost its fragment
/// would still count as "real" above and fails here by name.
const RISCV_EMITTERS_LANDED: &[(&str, &str, &str)] = &[
    // §2.4 THE FRAME: spill slots, saved स्थिरs, पुनःस्थानम् at the top, rounded up
    // to 16 — the psABI alignment. The fragment is the rounding.
    ("यन्त्रचौकटरचना", "विभाजनम् १६", "frame_layout"),
    // §2.5 THE CONSTANT SPLIT: hi = (c + 0x800) >> 12, the page bias that keeps lo
    // inside ±2048 and carries the +1 into hi when lo is negative (namaste.sas:22–24).
    ("यन्त्रोच्चनीचविभागः", "योगः २०४८", "split_hi_lo"),
    // §2.5 STACK ARGUMENTS: the ninth and later travel above स्तूपसूचकः, stored
    // there by the caller after one addi of 16⌈(n−8)/2⌉ — the one corpus routine
    // of arity 9 (निर्देशयोजनम्) needs exactly this line. `V-005`: the store's
    // WORD is now the value's file's (`यन्त्रनिधानपदम्` — `निधानम्` for an
    // integer, `प्लवनिधानम्` for a float past `fa7`), so the fragment is the
    // place it stores to, written right after it.
    ("यन्त्राह्वानोत्सर्जनम्", "उक्तम् स्तूपसूचकःय् इति", "emit_call"),
    // §2.5 THE SYNTHESIZED COMPARE: `bne c, zero, then` — the form for a condition
    // that is a VALUE (a call's result, a loaded बूल); kept beside the fusion.
    ("यन्त्रशाखावतरणम्", "उक्तम् विषमलङ्घनम् इति", "lower_cond_branch"),
    // `W-245` THE FUSION: a compare that ends its block and is read once by its
    // branch writes the condition's own instruction — the six ADR-0008 words,
    // करण the subject and अपादान (`त्`) the standard. The fragment is the one
    // branch word no synthesized form ever writes: not-less, `bge`.
    ("यन्त्रशाखापदम्", "उक्तम् अन्यूनलङ्घनम् इति", "branch_word"),
    // §2.2 THE LABEL SCHEME: routine label ⧺ पर्व ⧺ block id, no joiner; पर्व is
    // ir.t1's own word for the block.
    ("यन्त्रपर्वचिह्नम्", "उक्तम् पर्व इति", "block_label"),
    // §2.6 THE STARTUP: sp from auipc, NOT lui — bit 31 of 0x8000_0000+ would
    // sign-extend under lui on RV64.
    (
        "यन्त्रारम्भोत्सर्जनम्",
        "उक्तम् स्थानसापेक्षयोगः स्तूपसूचकःम् स्तूपान्तःॱउपरिन इति",
        "emit_startup",
    ),
    // §5 R6 (`W-243`) THE EXPORTED ROUTINE LABEL: every routine's label is written
    // under `॥ वैश्विकम् ॥`, because a label is local to its object (B-014), a
    // module is its own object, and a call into another module resolves only
    // through the linker's export table. The fragment is the directive.
    ("यन्त्रप्रस्तावना", "उक्तम् ॥ वैश्विकम् इति", "prologue"),
];

#[test]
fn the_riscv_emitters_carry_the_decisions_this_file_accounts_for() {
    let text = riscv_text();
    let mut wrong: Vec<String> = Vec::new();
    for (t1, fragment, rust) in RISCV_EMITTERS_LANDED {
        match routine_body(&text, t1) {
            None => wrong.push(format!("{rust}: {t1} declares no routine")),
            Some(b) if b.contains(CODEGEN_STUB) => wrong.push(format!("{rust}: {t1} is a stub")),
            Some(b) if !b.contains(fragment) => wrong.push(format!(
                "{rust}: {t1} has a body but does not write `{fragment}`"
            )),
            Some(_) => {}
        }
    }
    println!(
        "METRIC sadhana_t1_riscv_emitters_landed_named {}",
        RISCV_EMITTERS_LANDED.len()
    );
    assert_eq!(
        RISCV_EMITTERS_LANDED.len(),
        8,
        "six decisions, research/25 §3.3, W-243's seventh (§5 R6, the exported label) and W-245's eighth (the fusion of the six conditions)"
    );
    assert!(
        wrong.is_empty(),
        "the RISC-V emitter's decisions are not the ones this file accounts for:\n  {}",
        wrong.join("\n  ")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `D-002f1` — THE SASSEMBLY INSTRUCTION READER'S RECORDS, `vakyavibhaga.t1`.
//
// `crates/sadhana/src/parse.rs` is the reader that turns a Sassembly token
// stream into instructions, labels, data and diagnostics. **It had never been
// ported.** For a week the `D-002` chain called this blocker "B-109/parse.t1",
// and `parse.t1` is `व्याकर` — the T1 *language* parser, a port of
// `t1/parse.rs`, holding `घोषणा` and `व्याकरदोष`. Two files named *parse*, and
// the missing one was the assembler's.
//
// `D-002f1` is the first slice: the RECORDS and the routines that need nothing
// but them. The three readers are `D-002f3`/`f4`/`f5`, and the whole risk of
// landing records first is that a full symbol table and a full count of real
// bodies READ AS DONE while nothing reads a token. That is what the third test
// below exists to stop, and it is the same shape as `ir_kinds_built`,
// `codegen_emitters_real` and `encoder_program_routines_real`.
// ─────────────────────────────────────────────────────────────────────────

fn t0_reader_text() -> String {
    std::fs::read_to_string(crate_src().join("vakyavibhaga.t1")).expect("vakyavibhaga.t1 exists")
}

/// The stub marker every port in this crate uses, spelled once more here for
/// the reason `ENCODER_STUB` gives: this file's tests must not silently follow
/// a rename made for another module's.
const T0_READER_STUB: &str = "प्रत्यागमनम् उक्तम् अपूर्णम् इति";

/// Every symbol `crates/sadhana/src/parse.rs` defines, in the spelling
/// `वाक्यविभाग` gives it.
///
/// The rows below the types are not `pub` items of `parse.rs`. They are the
/// expressions it writes that T1 has to spell out: eleven `push` sites, because
/// T1 has no growable collection and appends into an arena; the arenas
/// themselves; `Program::default()` and the `out` that `parse` returns; and
/// `split_types`/`strip_type_class` split into index-returning halves, because
/// T1 has neither a tuple nor a sub-slice expression. Losing any of them loses
/// the port of a real line of `parse.rs`.
const T0_READER_SYMBOLS: &[(&str, &str)] = &[
    ("struct Family", "संरचना संज्ञाकुल"),
    ("struct Operand", "संरचना कारकपद"),
    ("struct Instruction", "संरचना आज्ञा"),
    ("struct Datum", "संरचना दत्तम्"),
    ("struct Label", "संरचना चिह्न"),
    ("struct DirectiveKind", "संरचना निर्देश"),
    ("struct Statement", "संरचना वाक्य"),
    ("struct Program", "संरचना कार्यक्रम"),
    ("struct ParseError", "संरचना वाक्यदोष"),
    ("Datum::addresses element", "संरचना स्थाननिर्देश"),
    ("Program::text_aligns element", "संरचना संरेख"),
    ("Vec<String> element", "संरचना पाठांश"),
    ("Width::W8", "अष्टकव्याप्तिभेद"),
    ("Width::W16", "षोडशव्याप्तिभेद"),
    ("Width::W32", "द्वात्रिंशव्याप्तिभेद"),
    ("Width::W64", "चतुःषष्टिव्याप्तिभेद"),
    ("Section::Text", "पाठकोष्ठकभेद"),
    ("Section::Data", "दत्तकोष्ठकभेद"),
    ("Section::Bss", "रिक्तकोष्ठकभेद"),
    ("StatementKind::Instruction", "आज्ञावाक्यभेद"),
    ("StatementKind::Label", "चिह्नवाक्यभेद"),
    ("StatementKind::Directive", "निर्देशवाक्यभेद"),
    ("Width::bits", "व्याप्त्यंशाः"),
    ("Width::from_digits", "अङ्कव्याप्तिः"),
    ("Width::from_type", "प्रकारव्याप्तिः"),
    ("strip_type_class: the class", "प्रकारवर्णः"),
    ("strip_type_class: the digits", "प्रकाराङ्कारम्भः"),
    ("split_types: the mnemonic", "संज्ञासीमा"),
    ("split_types: how many suffixes", "प्रकारसंख्या"),
    ("split_types: the nth suffix", "प्रकारारम्भः"),
    ("Section::from_name", "कोष्ठकनाम"),
    ("Instruction::by_role", "कारकानुसारम्"),
    ("StatementKind::as_str", "वाक्यभेदनाम"),
    ("Program::default", "आरम्भः"),
    ("parse()'s `out`", "कार्यक्रमरचना"),
    ("instructions.push", "आज्ञायोजनम्"),
    ("labels.push", "चिह्नयोजनम्"),
    ("data.push", "दत्तयोजनम्"),
    ("statements.push", "वाक्ययोजनम्"),
    ("text_aligns.push", "संरेखयोजनम्"),
    ("operands.push", "कारकपदयोजनम्"),
    ("addresses.push", "स्थाननिर्देशयोजनम्"),
    ("Vec<String> push", "पाठांशयोजनम्"),
    ("errors.push", "दोषयोजनम्"),
    ("families()'s row", "संज्ञाकुलयोजनम्"),
    ("directives()'s row", "निर्देशयोजनम्"),
    // `D-002f5`. The first eight are `apply_directive`'s first half; the last
    // six are the `sanskrit_text::numeral` calls that half writes, ported here
    // because `अक्षरकोश ॱ सङ्ख्या` is a stub and no other T1 module reads a
    // numeral at all. `globals.push` moved OFF `पाठांशयोजनम्`: a run is only a
    // run while one appender writes the arena, and `पाठांशकोश` also holds
    // every diagnostic's args, which the walk pushes BETWEEN two directives.
    ("apply_directive", "निर्देशानुष्ठानम्"),
    ("directives().find(..)", "निर्देशान्वेषणम्"),
    ("kind.status != \"live\"", "जीवन्निर्देशः"),
    ("`कोष्ठकम्` arm", "कोष्ठकनिर्देशः"),
    ("`वैश्विकम्` arm", "वैश्विकनिर्देशः"),
    ("`स्थानीयम्` arm", "स्थानीयनिर्देशः"),
    ("`संरेखः` arm", "संरेखनिर्देशः"),
    ("globals.push", "वैश्विकयोजनम्"),
    ("next_multiple_of(n) - at", "संरेखपूर्तिः"),
    ("vec![0; pad]", "शून्यपूरणम्"),
    ("numeral::digit_value", "अङ्कमूल्यम्"),
    ("numeral::split_sign: the sign", "ऋणचिह्नम्"),
    ("numeral::split_sign: the rest", "सङ्ख्यारम्भः"),
    ("numeral::classify: the radix", "सङ्ख्यामूलम्"),
    ("numeral::classify: the digits", "सङ्ख्याङ्कारम्भः"),
    ("numeral::value: whether", "सङ्ख्यादोषः"),
    ("numeral::value: what", "सङ्ख्यामानम्"),
    ("Program::globals", "वैश्विककोश"),
    ("Program::instructions", "वाक्यविभागआज्ञाकोश"),
    ("Program::labels", "चिह्नकोश"),
    ("Program::data", "दत्तकोश"),
    ("Program::statements", "वाक्यविभागवाक्यकोश"),
    ("Program::text_aligns", "संरेखकोश"),
    ("Instruction::operands", "कारकपदकोश"),
    ("Datum::addresses", "स्थाननिर्देशकोश"),
    ("Vec<String> arena", "पाठांशकोश"),
    ("parse()'s errors", "वाक्यविभागदोषकोश"),
    ("parse()'s `section`", "वर्तमानकोष्ठक"),
];

/// Every field, checked INSIDE the struct that must carry it.
///
/// This is where the port's decisions are recorded, and the symbol table above
/// cannot see any of them being undone. Four are load-bearing:
///
/// * `Instruction::operands` is a `Vec<Operand>` in Rust and a first-index/count
///   pair here. If it collapses to one field the run it stands for is gone.
/// * `Instruction::types` is a `Vec<String>` of at most TWO — one width for an
///   ordinary instruction, two for a conversion (`B-075`) — so it is two named
///   fields and a count, and `प्रकारसंख्यान` is what keeps `प्लवरूपान्तरम्ॱअ३२ॱप६४`
///   distinguishable from a bare mnemonic.
/// * `Datum::addresses` is `ADR-0013`'s deferred relocation: without
///   `स्थानारम्भ`/`स्थानसंख्यान` a datum that names a label loses the fact.
/// * `Program::bss_line` exists because a diagnostic about `ॱरिक्त` written on
///   line 40 must not blame line 1.
const T0_READER_FIELDS: &[(&str, &[(&str, &str)])] = &[
    (
        "आज्ञा",
        &[
            ("Instruction::family", "कुल"),
            ("Instruction::width", "व्याप्ति"),
            ("Instruction::types[0]", "प्रथमप्रकार"),
            ("Instruction::types[1]", "द्वितीयप्रकार"),
            ("Instruction::types.len()", "प्रकारसंख्यान"),
            ("Instruction::operands: first", "कारकारम्भ"),
            ("Instruction::operands: count", "कारकसंख्यान"),
            ("Instruction::line", "पङ्क्ति"),
        ],
    ),
    (
        "कारकपद",
        &[
            ("Operand::base", "मूलपाठ"),
            ("Operand::karaka", "कारक"),
            ("Operand::is_numeral", "सङ्ख्यात्व"),
        ],
    ),
    (
        "संज्ञाकुल",
        &[
            ("Family::key", "कूट"),
            ("Family::name", "नाम"),
            ("Family::gloss", "व्याख्या"),
            ("Family::ext", "विस्तारः"),
            ("Family::covers: first", "आवरणारम्भ"),
            ("Family::covers: count", "आवरणसंख्यान"),
        ],
    ),
    (
        "दत्तम्",
        &[
            ("Datum::section", "कोष्ठक"),
            ("Datum::bytes", "अष्टकाः"),
            ("Datum::addresses: first", "स्थानारम्भ"),
            ("Datum::addresses: count", "स्थानसंख्यान"),
            ("Datum::line", "पङ्क्ति"),
        ],
    ),
    (
        "चिह्न",
        &[
            ("Label::name", "नाम"),
            ("Label::section", "कोष्ठक"),
            ("Label::data_offset", "दत्तसरण"),
            ("Label::at", "आज्ञाक्रम"),
            ("Label::line", "पङ्क्ति"),
        ],
    ),
    (
        "वाक्य",
        &[
            ("Statement::kind", "भेद"),
            ("Statement::line", "पङ्क्ति"),
            ("Statement::name", "नाम"),
        ],
    ),
    (
        "कार्यक्रम",
        &[
            ("Program::instructions: first", "आज्ञारम्भ"),
            ("Program::instructions: count", "आज्ञासंख्यान"),
            ("Program::labels: first", "चिह्नारम्भ"),
            ("Program::labels: count", "चिह्नसंख्यान"),
            ("Program::data: first", "दत्तारम्भ"),
            ("Program::data: count", "दत्तसंख्यान"),
            ("Program::globals: first", "वैश्विकारम्भ"),
            ("Program::globals: count", "वैश्विकसंख्यान"),
            ("Program::bss", "रिक्तमान"),
            ("Program::text_aligns: first", "संरेखारम्भ"),
            ("Program::text_aligns: count", "संरेखसंख्यान"),
            ("Program::statements: first", "वाक्यारम्भ"),
            ("Program::statements: count", "वाक्यसंख्यान"),
            ("Program::bss_line", "रिक्तपङ्क्ति"),
        ],
    ),
    // `D-002f5` added the six range fields. `spec/directives.tsv` is reached
    // through the embed, and a column of a table is a RANGE IN THAT FILE —
    // T1 has no sub-slice expression, so three bare `अङ्कः अन्तः अ८` were
    // three fields the registry reader could never fill, and P13/P14 were
    // therefore unreachable however completely they were written. Losing a
    // range here puts them back out of reach.
    (
        "निर्देश",
        &[
            ("DirectiveKind::name", "नाम"),
            ("DirectiveKind::name: first", "नामारम्भ"),
            ("DirectiveKind::name: limit", "नामसीमा"),
            ("DirectiveKind::gas", "यवननाम"),
            ("DirectiveKind::gas: first", "यवनारम्भ"),
            ("DirectiveKind::gas: limit", "यवनसीमा"),
            ("DirectiveKind::status", "स्थिति"),
            ("DirectiveKind::status: first", "स्थित्यारम्भ"),
            ("DirectiveKind::status: limit", "स्थितिसीमा"),
        ],
    ),
    (
        "वाक्यदोष",
        &[
            ("ParseError::line", "पङ्क्ति"),
            ("ParseError::aksara", "अक्षर"),
            ("ParseError::code", "कूट"),
            ("ParseError::args: first", "पदार्थारम्भ"),
            ("ParseError::args: count", "पदार्थसंख्यान"),
            ("ParseError::reason", "कारणम्"),
        ],
    ),
];

#[test]
fn the_t0_reader_port_declares_every_record_of_the_rust_original() {
    let text = t0_reader_text();

    // Matched against the DECLARATION, never against the file. Every record
    // below is also NAMED IN PROSE in this file's header — which spells out
    // why there are now two `आज्ञा` in the crate — so a bare `contains` could
    // not fail for a lost record. That hole was found and fixed for
    // `samyojana.t1` this session; it is not reopened here.
    let declared = |t1: &str| {
        text.contains(&format!("सार्वजनिक {t1} "))
            || text.contains(&format!("सार्वजनिक वृत्तिः {t1} "))
            || text.contains(&format!("सार्वजनिक चरः {t1} "))
    };
    let mut missing: Vec<String> = T0_READER_SYMBOLS
        .iter()
        .filter(|(_, t1)| !declared(t1))
        .map(|(rust, _)| (*rust).to_string())
        .collect();

    // Fields, inside their own struct body, matched as `name ॱॱ ` — the
    // annotation that makes it a FIELD — so a field cannot be satisfied by a
    // comment or by a longer name it is a prefix of.
    for (name, fields) in T0_READER_FIELDS {
        match struct_body(&text, &format!("सार्वजनिक संरचना {name} आरभ्य"))
        {
            None => missing.push(format!("struct {name}")),
            Some(body) => missing.extend(
                fields
                    .iter()
                    .filter(|(_, t1)| !body.contains(&format!("{t1} ॱॱ ")))
                    .map(|(rust, _)| (*rust).to_string()),
            ),
        }
    }

    // The module line. The defect `vishlesana.t1` recorded — the right filename
    // over the wrong module — is exactly the defect that hid this port's
    // absence, so this file states which module it is and the test reads it.
    if !text.starts_with("मण्डलम् वाक्यविभाग ") {
        missing.push("मण्डलम् वाक्यविभाग".to_string());
    }

    let total =
        T0_READER_SYMBOLS.len() + T0_READER_FIELDS.iter().map(|(_, f)| f.len()).sum::<usize>();
    println!(
        "METRIC sadhana_t1_t0_reader_symbols {}",
        total - missing.len()
    );
    assert!(
        missing.is_empty(),
        "vakyavibhaga.t1 does not declare {} of {} parse.rs symbols: {}",
        missing.len(),
        total,
        missing.join(", ")
    );
}

/// The one routine of this port that CANNOT be written, and what it waits on.
///
/// `StatementKind::as_str` returns `"instruction"`, `"label"`, `"directive"` —
/// the words `spec/parse-shape-t0.tsv` is keyed on, and they are LATIN. Doc 15's
/// repertoire refuses a Latin letter and ADR-0017 says a string literal is not a
/// licence to smuggle one back in. It is required to be a STUB here for the
/// reason `CODEGEN_EMITTERS_BLOCKED` gives: a body appearing means a decision
/// was taken (a Devanagari column in the shape file, or a shape oracle that
/// compares `भेद` numbers), and that decision should have a row and not arrive
/// unremarked inside a port.
/// EMPTY SINCE 2026-08-30, and the record the assertion demands is that NO ROW
/// CLEARED IT — the blocker's premise was wrong when it was written.
///
/// It read: *"Latin: parse-shape-t0.tsv's own words are
/// instruction/label/directive"*, and asked for that file to gain a Devanagari
/// column or for the shape oracle to compare `भेद` numbers instead of words.
/// Both halves are about the ORACLE'S COMPARISON, not about `वाक्यभेदनाम`. And
/// `spec/parse-shape-t0.tsv` is a GENERATED golden of 9502 rows recording one
/// parse per line, so a column there would have been a per-row copy of a
/// three-valued fact.
///
/// THE THREE NAMES WERE ALREADY IN THE FILE THAT DECLARED THE STUB, twenty
/// lines above it: `आज्ञावाक्यभेद`, `चिह्नवाक्यभेद` and `निर्देशवाक्यभेद` at
/// `vakyavibhaga.t1:116`–`:118`, and `चिह्न` is attested as `label` in
/// `spec/lexicon.tsv`. The vocabulary was settled before the blocker was
/// written; what was missing was somebody reading it.
///
/// This is the eighth stale blocker found in this crate in one day. Kept as an
/// empty list rather than deleted, so the next one has somewhere to be
/// recorded and so this note survives.
const T0_READER_BLOCKED: &[(&str, &str)] = &[];

#[test]
fn the_t0_reader_port_reports_how_much_of_it_is_real() {
    let text = t0_reader_text();
    let functions = text.matches("सार्वजनिक वृत्तिः").count();
    let stubs = text.matches(T0_READER_STUB).count();
    let real = functions - stubs;
    println!("METRIC sadhana_t1_t0_reader_functions {functions}");
    println!("METRIC sadhana_t1_t0_reader_stubs {stubs}");
    println!("METRIC sadhana_t1_t0_reader_real {real}");

    // Ratchet. Raise it as routines land; it may never fall.
    // 94 -> 92 on 2026-09-04, W-239: NOT A FALL — the two stash routines
    // (`अष्टकयोजनम्`, `शून्याष्टकयोजनम्`) were LIFTED out of this file into
    // `ashtaka.t1`, where the octet arena's NINE routines stand and are counted
    // by `t1_ashtaka.rs`; 92 + 9 = 101 real routines across the two files.
    assert!(
        real >= 92,
        "only {real} of {functions} reader routines have a real body (94 before \
         W-239 lifted two into ashtaka.t1; was 93, \
         raised from 36 by D-002f5's nineteen — the four no-byte directive \
         arms, the applier, the registry lookup and its status test, the P16 \
         arity check and its operand count, the padding pair, and the six-part \
         port of sanskrit_text::numeral that this row had to write because \
         अक्षरकोश ॱ सङ्ख्या is a stub. Before that, 36, raised from 23 by \
         D-002f3's thirteen)"
    );

    // And the stub must be the ONE that is blocked, by name. A count alone is
    // satisfied by any one stub, so stubbing `कारकानुसारम्` and writing
    // `वाक्यभेदनाम` in Latin would hold the number and lose the account.
    for (t1, blocker) in T0_READER_BLOCKED {
        match routine_body(&text, t1) {
            None => panic!("{t1}: declares no routine"),
            Some(b) if !b.contains(T0_READER_STUB) => {
                panic!("{t1}: has a body; name the row that cleared its blocker — {blocker}")
            }
            Some(_) => {}
        }
    }
    assert_eq!(
        stubs,
        T0_READER_BLOCKED.len(),
        "{stubs} routines are stubs and only {} is blocked; a stub here that is \
         not `वाक्यभेदनाम` is a routine that had a body and lost it",
        T0_READER_BLOCKED.len()
    );
}

/// The two registry readers of `D-002f2`, ASSERTED BY NAME — and the reason
/// this test exists is written in that row's own note.
///
/// `निर्देशकोशपठनम्` and `संज्ञाकुलपठनम्` were written, mutation-tested in a
/// scratch worktree, and then SILENTLY REVERTED when a routine-level union took
/// `vakyavibhaga.t1` wholesale from three worktrees that had never carried them.
/// Nothing failed, because **absent code fails no test**: until this test, no
/// assertion in the tree named either routine, and the only thing that noticed
/// anything at all was `every_t1_source_lexes_and_parses` complaining about the
/// table name they had taken with them. A `grep -c` for that name then found the
/// COMMENT describing it and reported it present. So each of the four things
/// below fails on its own, and none of them is a `contains` on a bare name:
///
/// 1. each routine reaches its table THROUGH ADR-0019's embed, by the name the
///    registry actually spells;
/// 2. that name RESOLVES — `anita::table_path` answers with the file, so the
///    name is not merely typed in a `.t1` file that nothing compiles;
/// 3. the row each builds goes INTO ITS ARENA, via its `योजनम्` half;
/// 4. the header filter each uses discriminates the header AND NOTHING ELSE,
///    checked against the real file rather than against the reader's opinion.
///
/// Point 4 is not decoration. `संज्ञाकुलपठनम्` shipped testing the first octet
/// of field 0 against `१०२` — ASCII `f`, the first letter of the header word
/// `family` — and so dropped `fence`, `fadd`, `fmv`, `fencei` and twenty more:
/// **24 of the 74 mnemonic families, silently, because they begin with the same
/// letter the header does.** A filter keyed on a LETTER cannot tell a header
/// from a row; one keyed on the SCRIPT can, since doc 15's repertoire refuses a
/// Latin letter in Devanagari data and `२२४` is the UTF-8 lead octet of every
/// character in U+0900..U+097F.
#[test]
fn the_t0_reader_reads_the_two_registry_tables() {
    let text = t0_reader_text();

    // (routine, embedded table name, the file it must resolve to, the arena
    //  half it must push through, the 0-based field whose script says "row").
    let readers: &[(&str, &str, &str, &str, usize)] = &[
        (
            "निर्देशकोशपठनम्",
            "निर्देशकोशः",
            "directives.tsv",
            "निर्देशयोजनम्",
            0,
        ),
        (
            "संज्ञाकुलपठनम्",
            "नामकोशः",
            "mnemonics-riscv64.src.tsv",
            "संज्ञाकुलयोजनम्",
            1,
        ),
    ];

    for (routine, table, file, append, script_field) in readers {
        let body = routine_body(&text, routine)
            .unwrap_or_else(|| panic!("{routine} is declared in vakyavibhaga.t1"));

        assert!(
            body.contains(&format!("पदविभागॱसमावेशपाठः उक्तम् {table} इति")),
            "{routine} has a body but does not open {table} through the store; a \
             registry built from anything else is a hand-copied second copy of a \
             derived file. The spelling changed on 2026-09-14: ADR-0019's embed \
             substitutes at LEX time of the source being compiled, so the table a \
             COMPILER needs was resolved when that compiler's own source was \
             compiled — empty for every compiled compiler, which is why the native \
             assembler read 0 mnemonics. The condition this guard protects is \
             unchanged: the table is reached by NAME from the store the host (or a \
             generated module) fills from spec/, never hand-copied"
        );
        assert_eq!(
            sadhana::t1::anita::table_path(table),
            Some(*file),
            "`{table}` is spelled in vakyavibhaga.t1 but has no row in \
             anita.rs's TABLES, so the embed has nothing to resolve and \
             {routine} is unspellable rather than merely wrong — this entry \
             has gone missing once already"
        );
        assert!(
            body.contains(append),
            "{routine} reads {file} and pushes nothing; its arena stays empty \
             and every lookup against it keeps answering ०"
        );
        assert!(
            body.contains("समम् २२४"),
            "{routine}'s header filter does not test the SCRIPT of a field. A \
             filter keyed on one Latin LETTER cannot tell {file}'s header from \
             a row that starts with the same letter, which is how 24 of the \
             mnemonic families were silently dropped"
        );

        // The file, read the way the reader reads it: `#` and blank lines
        // dropped, then five fields or more, then the script test.
        let table_text = std::fs::read_to_string(spec_root().join(file))
            .unwrap_or_else(|_| panic!("spec/{file} exists"));
        let rows: Vec<&str> = table_text
            .lines()
            .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
            .filter(|l| l.split('\t').count() >= 5)
            .collect();
        let kept = rows
            .iter()
            .filter(|l| {
                l.split('\t')
                    .nth(*script_field)
                    .and_then(|f| f.as_bytes().first().copied())
                    == Some(224)
            })
            .count();

        // NOT VACUOUS, and this is the half that proves the script test works:
        // the header must SURVIVE the two filters above it, or the script test
        // is discriminating nothing and could be deleted without a failure.
        let header = rows.first().expect("the table has a header line");
        assert!(
            header
                .split('\t')
                .nth(*script_field)
                .and_then(|f| f.as_bytes().first().copied())
                != Some(224),
            "spec/{file}'s header is now Devanagari in field {script_field}, so \
             the script test discriminates nothing and {routine} would take the \
             header as a row"
        );
        assert_eq!(
            kept,
            rows.len() - 1,
            "{routine} keeps {kept} of spec/{file}'s {} non-header rows. Every \
             row this drops is a registry entry that silently does not exist.",
            rows.len() - 1
        );

        println!(
            "METRIC sadhana_t1_t0_reader_{}_rows {kept}",
            table_path_metric(file)
        );
    }
}

/// The metric name for a table, since `println!` cannot take a `.` in an ident
/// and the file names carry two.
fn table_path_metric(file: &str) -> String {
    file.replace(['.', '-'], "_")
}

/// The three statement kinds `parse.rs` records, and they are measured AT THE
/// APPEND SITE — `वाक्ययोजनम् आरभ्य <kind>` — not at the declaration.
///
/// `वाक्ययोजनम्` takes the kind as its FIRST parameter for exactly this reason,
/// which is the same reason `मध्यरूप ॱ आज्ञायोजनम्` does and the only thing that
/// makes `ir_kinds_built` able to see anything.
const T0_READER_STATEMENT_KINDS: &[(&str, &str)] = &[
    ("StatementKind::Instruction", "आज्ञावाक्यभेद"),
    ("StatementKind::Label", "चिह्नवाक्यभेद"),
    ("StatementKind::Directive", "निर्देशवाक्यभेद"),
];

#[test]
fn the_t0_reader_reports_whether_it_reads_any_token() {
    // **This is the test that stops the two above from reading as "done".**
    //
    // Between them they say every record of `parse.rs` is declared and every
    // routine but the one blocked one has a real body, and both are true — and
    // NOTHING IN THE FILE READS A TOKEN. Four ports today were caught by
    // exactly this gap between *declared* and *does the thing*, which is why
    // `ir.t1` added `ir_kinds_built`, `utsarjana.t1` `codegen_emitters_real`
    // and `encode.t1` `encoder_program_routines_real`. This is the reader's
    // number, and it is the one that must move for `D-002f3`..`f6` to be real.
    let text = t0_reader_text();

    // A ROUTINE THAT TAKES A TOKEN, matched on the DECLARATION LINE and not on
    // the file. `पदविभागॱचिह्नक` appears twice in this file's prose — it is the
    // type the port says out loud it does not yet read, and the note about the
    // T1 token carrying no operand payload names it again. A bare `contains`
    // would report a reader that reads nothing as reading tokens.
    let consumers: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("सार्वजनिक वृत्तिः") && l.contains("ॱॱ पदविभागॱचिह्नक"))
        .collect();

    let read: Vec<&str> = T0_READER_STATEMENT_KINDS
        .iter()
        .filter(|(_, t1)| text.contains(&format!("वाक्ययोजनम् आरभ्य {t1} ")))
        .map(|(rust, _)| *rust)
        .collect();

    println!(
        "METRIC sadhana_t1_t0_reader_statement_kinds_declared {}",
        T0_READER_STATEMENT_KINDS.len()
    );
    println!(
        "METRIC sadhana_t1_t0_reader_statement_kinds_read {}",
        read.len()
    );
    println!(
        "METRIC sadhana_t1_t0_reader_token_consumers {}",
        consumers.len()
    );

    // Not vacuous: the file must still NAME the token type it does not read, or
    // the zero below would be a spelling accident rather than a measurement.
    assert!(
        text.contains("पदविभागॱचिह्नक"),
        "vakyavibhaga.t1 no longer names पदविभागॱचिह्नक at all; the two counts \
         above would then be zero for the wrong reason"
    );

    // WAS PINNED AT ०, AND THE ROW THAT MOVED IT IS `D-002f3` — THE
    // STATEMENT SPLITTER. `वाक्यविभाजनम्` is `parse.rs:485-612`'s
    // `for tok in tokens`: it walks `पदविभागॱचिह्नककोश`, splits on `।`, pairs
    // `॥` as opener and closer per ADR-0012 without letting an unpaired one
    // swallow the directive after it, ends a label at its `ॱॱ`, and then runs
    // the row's three post-checks — L02 at `वैश्विकचिह्नपरीक्षा`, P25 at
    // `अनावृतनिर्देशपरीक्षा` and P02 at `अन्तिमवाक्यपरीक्षा`. Its input arena
    // stopped being empty this morning, when `D-002f10` landed `lex.t1`.
    //
    // **NEITHER NUMBER PROVES THE SPLITTER SPLITS.** A declaration count and
    // three static call sites read exactly the same over an arena that is
    // never walked, which is the ratchet question this row was told to ask;
    // the walk is measured by `the_statement_splitter_walks_the_token_arena`
    // at the foot of this file. Raise these two as `D-002f4` (the sentence
    // reader) and `D-002f5`/`f6` (the directive reader) land.
    assert!(
        consumers.len() >= 6,
        "only {} routine(s) in vakyavibhaga.t1 take a पदविभागॱचिह्नक (was 6, \
         raised by D-002f5's एकपदपरीक्षा; 5 before that, raised by D-002f3); \
         a consumer that was there has been lost",
        consumers.len()
    );
    assert_eq!(
        read.len(),
        T0_READER_STATEMENT_KINDS.len(),
        "only {} of the {} statement kinds are appended by a reader ({}); \
         D-002f3 appends all three from वाक्यविभाजनम्",
        read.len(),
        T0_READER_STATEMENT_KINDS.len(),
        read.join(", ")
    );

    // REPORTED, NOT PINNED, AND ABOUT ANOTHER FILE.
    //
    // THIS PARAGRAPH USED TO SAY the reader had nothing to read — that
    // `पदविभाग ॱ पदविभाग` walked the source counting akṣaras and wrote NOTHING
    // into `चिह्नककोश`. **`D-002f10` made that false on 2026-08-28**: `lex.t1`
    // went 121 -> 502 lines, `चिह्नककोश` is written, and
    // `sadhana_t1_t0_lexer_token_writes` is non-zero.
    //
    // The correction is recorded rather than the sentence quietly replaced,
    // because the reason it was true is worth keeping: every routine signed
    // `शब्द ॱॱ अङ्कः अन्तः अ८` was UNCALLABLE by a lexer holding only the
    // source and two offsets, T1 having no slice constructor — so a body that
    // could not be written had been written as `प्रत्यागमनम् ०`. That is why
    // the number was zero, and it is not the same thing as the routine being
    // unfinished.
    //
    // Still printed and not asserted on, because it is another file's row.
    // What IS asserted is that the arena is still declared, since otherwise
    // the number would mean nothing either way.
    let lexer = std::fs::read_to_string(crate_src().join("lex.t1")).expect("lex.t1 exists");
    assert!(
        lexer.contains("सार्वजनिक चरः चिह्नककोश "),
        "lex.t1 no longer declares चिह्नककोश; the reader's input arena is gone"
    );
    println!(
        "METRIC sadhana_t1_t0_lexer_token_writes {}",
        lexer.matches("चिह्नककोश अङ्कः").count()
    );
}

// ══════════════════════════════════════════════════════════════════════
// `D-002f10` — THE T0 LEXER PORT, AND WHETHER IT WRITES A TOKEN.
//
// `crates/sadhana-t1/src/lex.t1` is the port of `crates/sadhana/src/lex.rs`
// `lex`. Until `D-002f10` it declared `चिह्नककोश` and never wrote to it, so
// `D-002f3`/`f4`/`f6` would each have read an empty arena.
//
// **THE METRIC DIRECTLY ABOVE IS NOT ENOUGH AND THIS SECTION EXISTS BECAUSE OF
// IT.** `sadhana_t1_t0_lexer_token_writes` counts `lexer.matches("चिह्नककोश
// अङ्कः")`, and that phrase is equally a READ (`चिह्नककोश अङ्कः क अन्तः` on
// the right of a `भवति`) and a line of PROSE — a bare `contains` was shown this
// morning to report 2 from prose alone. So the smallest number that would read
// as done while the lexer still emitted nothing is **1**: one appender holding
// one store, called by nobody. That is the shape four ports were caught in
// today, and it is what the three counts below are built to refuse.
//
// The floors are therefore on three different things, and no one of them can be
// satisfied by faking another:
//   · `..._token_stores` — the STORE form only, `कोश अङ्कः सूचकाङ्क अन्तः भवति`.
//     A read cannot match it; `भवति` after `अन्तः` is what makes it a write.
//   · `..._append_sites` — how many times the walker CALLS the appender. A
//     store inside a routine nothing calls is still nothing written.
//   · `..._kinds_produced` — how many of the eight `भेद` the port can actually
//     emit, counted at the RETURN AND ASSIGNMENT sites by whole-line match. A
//     declaration (`सार्वजनिक चरः दण्डभेद ...`) is not a production and neither
//     is a comment, so the eight declarations at the head of the file cannot
//     inflate this to eight.
// ══════════════════════════════════════════════════════════════════════

fn t0_lexer_text() -> String {
    std::fs::read_to_string(crate_src().join("lex.t1")).expect("lex.t1 exists")
}

/// The code lines of a `.t1` file: prose is `॰`-led and the lexer drops it.
fn t1_code_lines(text: &str) -> Vec<&str> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('॰'))
        .collect()
}

/// A number as Devanagari digits — U+0966..U+096F.
///
/// Derived, never typed: the octet triples the port compares against are
/// checked below by encoding the SIGN ITSELF, so a transposed digit in
/// `lex.t1` fails here instead of silently classifying nothing.
fn devanagari(n: u8) -> String {
    n.to_string()
        .chars()
        .map(|c| char::from_u32(0x0966 + (c as u32 - '0' as u32)).expect("digit"))
        .collect()
}

/// Every `भेद` the token type declares, with the `lex.rs` variant it ports.
///
/// `Kind::Operand` HAS NO ROW because T1 has no operand kind: `vakyavibhaga.t1`
/// settled that an operand lexes as `पदभेद` and the reader calls `विभज` for the
/// role and takes the base by length. That is the landed shape, so the mapping
/// below sends `Kind::Word` and `Kind::Operand` to the same `भेद`.
const T0_LEXER_KINDS: &[(&str, &str)] = &[
    ("Kind::Word", "पदभेद"),
    ("Kind::Numeral", "सङ्ख्याभेद"),
    ("Kind::Str", "शब्दभेद"),
    ("Kind::Danda", "दण्डभेद"),
    ("Kind::DoubleDanda", "युग्मदण्डभेद"),
    ("Kind::LabelMark", "चिह्नभेद"),
    ("Kind::MemberMark", "अंशचिह्नभेद"),
    ("Kind::Separator", "विच्छेदकभेद"),
];

/// The two `भेद` this port CANNOT produce, and the named thing each waits on.
///
/// Same discipline as `CODEGEN_EMITTERS_BLOCKED` and `T0_READER_BLOCKED`: a
/// count alone is held by any two absences, so the blocked ones are named. A
/// body appearing for either means a decision was taken, and it should have a
/// row rather than arrive unremarked inside a port.
const T0_LEXER_BLOCKED_KINDS: &[(&str, &str)] = &[(
    "सङ्ख्याभेद",
    "not produced BY DESIGN, and the reason changed 2026-09-01. It used to \
         read \"अक्षरकोशॱसङ्ख्या (is_numeral) is a stub\"; that routine is FULLY \
         IMPLEMENTED at sanskrit_text.t1:21 and had been for some time. The \
         lexer still does not classify numerals because T0 वाक्यविभाग REQUIRES \
         an operand to arrive as पदभेद and takes its role from विभज — lex.t1 \
         blocker (d). One WALK serves both languages through two entries \
         (पदविभाग for T1, वाक्यपदविभाग for T0, ADR-0017 and W-366 (b)) and \
         neither makes a numeral kind, so व्याकर decides numeral-ness by TEXT \
         instead, asking अक्षरकोशॱसङ्ख्या",
)];

// `शब्दभेद` WAS IN THIS LIST AND THE ENTRY WAS FACTUALLY WRONG — removed
// 2026-10-03, and NOT because a row cleared its blocker. `lex.t1` produced it
// all along, at TWO sites, and this file could not see either: both were the
// one-line form `यदि पदपाठम् समम् उक्तम् विवरम् इति आदि भेद भवति शब्दभेद । इति`,
// and `t0_lexer_kind_sites` compared WHOLE LINES. So the assertion below read
// `0` and passed while the thing it forbids was happening twice.
//
// SAS-011(3) (`ebfe686c`) only reformatted the arm — it moved the `उक्तम्`
// lookback inside the match, which put the assignment on a line of its own. The
// sound count went 2 -> 1, i.e. that commit REDUCED the production sites, and
// this test reported it as newly produced. Measured both ways on both trees:
// before 0 by whole-line / 2 by occurrence, after 1 / 1, with the other six
// भेद at 1 / 1 unchanged and `सङ्ख्याभेद` genuinely absent at 0 / 0.
//
// So the number was never bumped to 1: that would have re-pinned a matcher
// that cannot see an inline assignment. The matcher was fixed instead.
//
// WHAT IS NOT SETTLED, and it is a claim about the LANGUAGE rather than this
// test: the removed entry asserted "Kind::Str is lex_t1's, not lex's —
// ADR-0017". `lex.t1` demonstrably produces `शब्दभेद`, and the corpus reaches
// the fixpoint, so either that intent is stale or it needs a guard somewhere
// that can actually observe it. A whole-line text scan never could. Filed as
// W-366 rather than decided here.
//
// DECIDED BY ADR-0017 ITSELF, and BUILT as W-366 (b) on 2026-10-04: the intent
// stood (":206, T0 is not changed"), so the walk now takes Rust's `Strings`
// flag and has two entries. `शब्दभेद` stays PRODUCED in this file — by the T1
// entry — and the claim "never on T0 text" is enforced at RUN time, where a
// text scan cannot: `w366a_sas_lexes_no_string_token.rs` lexes through the T0
// entry `पदविभागॱवाक्यपदविभाग` and holds its T1 control through `पदविभाग`.

/// A `भेद` is PRODUCED where it is returned or assigned, and nowhere else.
///
/// COUNTED PER OCCURRENCE AND NOT PER WHOLE LINE — corrected 2026-10-03. The
/// previous version compared whole lines, and `.t1` writes a short arm inline:
/// `यदि <test> आदि भेद भवति शब्दभेद । इति` is one line carrying one production
/// site, and whole-line equality scored it ZERO. That is how the `शब्दभेद`
/// entry above stayed green over two live sites. Prose cannot count either way
/// — `t1_code_lines` drops `॰`-led lines before this sees them.
///
/// THE LEADING SPACE IS LOAD-BEARING, not decoration: `भेद` ends many names in
/// this corpus, so a bare substring would also match `अन्यभेद भवति शब्दभेद ।`
/// and credit a different variable's assignment to this one. A site is the
/// phrase at the start of a line or preceded by a space.
///
/// THE KIND'S NUMBER IS A THIRD SPELLING — added 2026-10-03 (W-366 a). The
/// embed collapse writes `समावेशचिह्नकम् ॱ भेद भवति ३ ।`: a production of
/// `शब्दभेद` by its declared VALUE, which neither named phrase can see. So the
/// assignment form is also matched with the number `text` itself declares for
/// `kind` (`सार्वजनिक चरः <kind> ॱॱ न६४ भवति <N> ।`), read and never typed. The
/// RETURN form gets no numeric twin on purpose: `विभज` returns kāraka roles
/// `प्रत्यागमनम् १ ।`..`प्रत्यागमनम् ५ ।` in the same digits, and counting
/// those would credit `सङ्ख्याभेद` (२) with production sites it does not have.
fn t0_lexer_kind_sites(text: &str, kind: &str) -> usize {
    let declared = format!("सार्वजनिक चरः {kind} ॱॱ न६४ भवति ");
    let mut phrases = vec![format!("प्रत्यागमनम् {kind} ।"), format!("भेद भवति {kind} ।")];
    if let Some(n) = t1_code_lines(text)
        .into_iter()
        .find_map(|l| l.strip_prefix(declared.as_str()))
        .and_then(|rest| rest.split_whitespace().next())
    {
        phrases.push(format!("भेद भवति {n} ।"));
    }
    t1_code_lines(text)
        .into_iter()
        .map(|l| {
            phrases
                .iter()
                .map(|p| {
                    let inner = l.matches(&format!(" {p}")).count();
                    inner + usize::from(l.starts_with(p.as_str()))
                })
                .sum::<usize>()
        })
        .sum()
}

#[test]
fn the_t0_lexer_port_actually_writes_tokens() {
    let text = t0_lexer_text();
    let lines = t1_code_lines(&text);

    // A WRITE, and only a write. `भवति` after `अन्तः` is the store; the same
    // phrase without it is the read that the looser metric above cannot tell
    // apart.
    let stores = lines
        .iter()
        .filter(|l| l.starts_with("चिह्नककोश अङ्कः चिह्नकसूचकाङ्क अन्तः भवति "))
        .count();

    // The walker CALLING the appender. The declaration line begins
    // `सार्वजनिक वृत्तिः`, so it cannot be mistaken for a call.
    let append_sites = lines
        .iter()
        .filter(|l| l.starts_with("चिह्नकयोजनम् "))
        .count();

    let produced: Vec<&str> = T0_LEXER_KINDS
        .iter()
        .filter(|(_, t1)| t0_lexer_kind_sites(&text, t1) > 0)
        .map(|(rust, _)| *rust)
        .collect();

    println!("METRIC sadhana_t1_t0_lexer_token_stores {stores}");
    println!("METRIC sadhana_t1_t0_lexer_append_sites {append_sites}");
    println!(
        "METRIC sadhana_t1_t0_lexer_kinds_declared {}",
        T0_LEXER_KINDS.len()
    );
    println!(
        "METRIC sadhana_t1_t0_lexer_kinds_produced {}",
        produced.len()
    );
    println!(
        "METRIC sadhana_t1_t0_lexer_kinds_blocked {}",
        T0_LEXER_BLOCKED_KINDS.len()
    );

    // Not vacuous: the arena and its index must both still be declared, or
    // every number above would be zero for a spelling reason.
    assert!(
        text.contains("सार्वजनिक चरः चिह्नककोश ") && text.contains("सार्वजनिक चरः चिह्नकसूचकाङ्क "),
        "lex.t1 no longer declares the arena or its index; the counts above \
         would then be zero for the wrong reason"
    );

    // Ratchets. Raise them as the lexer grows; they may never fall.
    assert!(
        stores >= 1,
        "lex.t1 stores into चिह्नककोश {stores} times (was 1); the arena is \
         written by the `कोश अङ्कः सूचकाङ्क अन्तः भवति` form and nothing else"
    );
    assert!(
        append_sites >= 2,
        "only {append_sites} call site(s) reach चिह्नकयोजनम् (was 2 — the head \
         of a word and the sign peeled off its tail); an appender nothing \
         calls writes nothing"
    );
    assert!(
        produced.len() >= 6,
        "only {} of the {} भेद are produced ({}); the other {} must be \
         T0_LEXER_BLOCKED_KINDS and named",
        produced.len(),
        T0_LEXER_KINDS.len(),
        produced.join(", "),
        T0_LEXER_KINDS.len() - produced.len()
    );

    // And the unproduced ones must be exactly the blocked ones, BY NAME. A
    // count alone is satisfied by any two absences, so a `दण्डभेद` that lost
    // its return while `सङ्ख्याभेद` gained one would hold the number.
    for (kind, blocker) in T0_LEXER_BLOCKED_KINDS {
        assert_eq!(
            t0_lexer_kind_sites(&text, kind),
            0,
            "{kind} is produced now; name the row that cleared its blocker — {blocker}"
        );
    }
    assert_eq!(
        produced.len() + T0_LEXER_BLOCKED_KINDS.len(),
        T0_LEXER_KINDS.len(),
        "{} भेद are neither produced nor named as blocked; one of them had a \
         production site and lost it",
        T0_LEXER_KINDS.len() - produced.len() - T0_LEXER_BLOCKED_KINDS.len()
    );
}

/// W-366 (a): the matcher SEES the embed collapse's literal-number site.
///
/// Before the numeric phrase, `lex.t1`'s `शब्दभेद` read 1 site (the layout-word
/// arm) while the routine had two; the second assigns the kind by its value.
/// Shown both ways on the site itself: the line ALONE, with no declaration to
/// read the number from, scores 0 — which is what the named phrases score — and
/// the same line beside the declaration scores 1.
#[test]
fn the_t0_lexer_kind_matcher_sees_a_kind_assigned_by_its_number() {
    let text = t0_lexer_text();
    let kind = "शब्दभेद";
    let prefix = format!("सार्वजनिक चरः {kind} ॱॱ न६४ भवति ");
    let decl = t1_code_lines(&text)
        .into_iter()
        .find(|l| l.starts_with(prefix.as_str()))
        .expect("lex.t1 declares शब्दभेद");
    let n = decl[prefix.len()..]
        .split_whitespace()
        .next()
        .expect("the declaration has a value");
    let numeric = format!(" भेद भवति {n} ।");
    let sites: Vec<&str> = t1_code_lines(&text)
        .into_iter()
        .filter(|l| l.ends_with(numeric.as_str()))
        .collect();
    assert!(
        !sites.is_empty(),
        "lex.t1 no longer assigns शब्दभेद by its number ({n}); if the embed collapse \
         now names the kind, this test has served and can go"
    );
    for site in &sites {
        assert_eq!(
            t0_lexer_kind_sites(site, kind),
            0,
            "without the declaration the site is invisible — the old matcher's reading"
        );
        assert_eq!(
            t0_lexer_kind_sites(&format!("{decl}\n{site}"), kind),
            1,
            "beside the declaration the numeric site is counted: {site}"
        );
    }
    let all = t0_lexer_kind_sites(&text, kind);
    println!(
        "METRIC sadhana_t1_t0_lexer_str_sites {all} (numeric {})",
        sites.len()
    );
    assert!(
        all == sites.len() + 1,
        "lex.t1's शब्दभेद sites are the layout-word arm plus {} numeric; counted {all}",
        sites.len()
    );
}

/// The sign each `भेद` is spelled with, for the octet check below.
///
/// `ॱॱ` is absent on purpose: it is `ॱ` twice and the port compares it as two
/// three-octet runs, so checking `ॱ` checks both.
const T0_LEXER_SIGNS: &[(&str, &str)] = &[
    ("।", "दण्डभेद"),
    ("॥", "युग्मदण्डभेद"),
    ("ॱ", "अंशचिह्नभेद"),
    ("ऽ", "विच्छेदकभेद"),
];

#[test]
fn the_t0_lexer_compares_the_octets_the_signs_actually_have() {
    // T1 has no character literal, so every sign in `lex.t1` is written as
    // three decimal octets in Devanagari digits. THAT IS A TRANSCRIPTION, and a
    // transposed digit would classify nothing while every count above held. So
    // the triples are DERIVED HERE from the sign itself and looked for in the
    // file, rather than typed a second time.
    let text = t0_lexer_text();
    let lines = t1_code_lines(&text);

    for (sign, kind) in T0_LEXER_SIGNS {
        let b = sign.as_bytes();
        assert_eq!(b.len(), 3, "{sign} is not three octets");
        let triple = format!(
            "{} {} {}",
            devanagari(b[0]),
            devanagari(b[1]),
            devanagari(b[2])
        );
        // Every line that compares this sign's octets at आरम्भः. `ॱ` has two
        // such lines — its own arm and the first half of `ॱॱ` — so the test
        // asks whether ONE of them classifies, not whether the first does.
        let compared: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, l)| l.contains(&triple) && l.contains("त्र्यष्टकसाम्यम् मूल आरम्भः"))
            .map(|(i, _)| i)
            .collect();
        assert!(
            !compared.is_empty(),
            "lex.t1 never compares आरम्भः against `{triple}`, the octets of {sign}"
        );
        // A triple compared and then dropped classifies nothing, so one of
        // those arms must return that sign's own भेद on the very next line.
        let returns = format!("प्रत्यागमनम् {kind} ।");
        assert!(
            compared.iter().any(|at| lines[at + 1] == returns),
            "{sign} ({triple}) is compared on {} line(s) and no arm returns {kind}",
            compared.len()
        );
    }

    // lex.rs:295 — `line.split('॰').next()`, where the comment mark ends the
    // line. ADR-0017 moved it after string recognition; for T0 it is unchanged,
    // and it is the one cut this port must make before splitting words.
    let mark = "॰".as_bytes();
    let mark_triple = format!(
        "{} {} {}",
        devanagari(mark[0]),
        devanagari(mark[1]),
        devanagari(mark[2])
    );
    assert!(
        text.contains(&mark_triple),
        "lex.t1 never compares against `{mark_triple}`, the octets of ॰ — \
         lex.rs:295's comment strip is not ported"
    );
    println!(
        "METRIC sadhana_t1_t0_lexer_signs_checked {}",
        T0_LEXER_SIGNS.len()
    );
}

/// The `भेद` a `lex.rs` token kind ports to. `Kind::Operand` joins `Kind::Word`
/// for the reason `T0_LEXER_KINDS` gives.
fn t1_bheda_of(kind: &sadhana::lex::Kind) -> &'static str {
    use sadhana::lex::Kind;
    match kind {
        Kind::Word | Kind::Operand { .. } => "पदभेद",
        Kind::Numeral => "सङ्ख्याभेद",
        Kind::Str { .. } => "शब्दभेद",
        Kind::Danda => "दण्डभेद",
        Kind::DoubleDanda => "युग्मदण्डभेद",
        Kind::LabelMark => "चिह्नभेद",
        Kind::MemberMark => "अंशचिह्नभेद",
        Kind::Separator => "विच्छेदकभेद",
    }
}

#[test]
fn the_worked_example_of_doc_02_needs_only_kinds_this_port_can_emit() {
    // `D-002f10`'s acceptance: `योगः कम् खन गन ।` produces four `चिह्नक`.
    //
    // **IT PRODUCES FIVE, AND THE FOUR ARE THE WORDS.** The expectation is not
    // hand-written here — it is taken through `sadhana::lex::lex`, the Rust
    // original this port ports, whose own test asserts `k.len() == 5` with
    // `k[4] == Kind::Danda`. The daṇḍa is a token. Both numbers are asserted
    // below so the row's wording and the original's behaviour are both on the
    // record and neither is quietly rounded to the other.
    //
    // Nothing executes a `.t1` body in this tree — `t1::parse::parse_program`
    // descends into one only when `आदि` follows the routine name directly, and
    // every routine has `आदाय` or `ददाति` there instead — so this is a
    // CORRESPONDENCE proof, not a run: every kind the example needs is a kind
    // the port has a production site for. It is stated as that and not as more.
    let tokens = sadhana::lex::lex("योगः कम् खन गन ।").expect("the doc 02 example lexes");
    let text = t0_lexer_text();

    let words = tokens
        .iter()
        .filter(|t| t1_bheda_of(&t.kind) == "पदभेद")
        .count();

    println!("METRIC sadhana_t1_t0_lexer_example_tokens {}", tokens.len());
    println!("METRIC sadhana_t1_t0_lexer_example_words {words}");

    assert_eq!(
        words, 4,
        "the example is योगः कम् खन गन — four words, of which three carry a \
         kāraka sigil and lex as पदभेद with the rest"
    );
    assert_eq!(
        tokens.len(),
        5,
        "four words AND the daṇḍa; `D-002f10` says four चिह्नक and the fifth \
         is `।`, which lex.rs emits as Kind::Danda"
    );
    assert_eq!(t1_bheda_of(&tokens[4].kind), "दण्डभेद");

    // Every kind the example needs must have a production site in the port.
    for (i, t) in tokens.iter().enumerate() {
        let bheda = t1_bheda_of(&t.kind);
        assert!(
            t0_lexer_kind_sites(&text, bheda) > 0,
            "token {i} of the acceptance example (`{}`) is {bheda}, and lex.t1 \
             has no site that returns or assigns it",
            t.text
        );
    }
}

// ══════════════════════════════════════════════════════════════════════
// `D-002f8` — THE DIAGNOSTIC PATH, `nidana.t1`.
//
// `vakyavibhaga.t1` (`D-002f1`) already holds `वाक्यदोष` and `दोषकोश`: the
// error RECORD and its arena. A record is not a diagnostic. What `D-002f8`
// adds is the two halves that make one — rendering a code through `निदान` in
// a chosen language (`ParseError::message`, `parse.rs:329`), and the
// did-you-mean heuristic (`distance`/`nearest`, `:375-412`).
//
// # Why this row was mostly a segmenter
//
// `distance` IS AKṢARA-LEVEL AND MUST STAY SO. `क्ष` is ONE akṣara — क plus
// virāma plus ष, three code points — so a code-point distance calls `क्ष` and
// `क` three edits apart when they are one, and suppresses exactly the
// suggestion worth making. `parse.rs:376` spends its first two lines on
// `sanskrit_text::aksharas`, and when this row ran **no `.t1` module in this
// crate had it**: `sanskrit_text.t1`'s `अक्षराणि` was a stub returning
// `अपूर्णम्`, and that file belonged to another worker. So the segmenter had to
// be written here, and it is the bulk of the port.
//
// **THAT IS NO LONGER TRUE AND THE HANDOVER IS NOW POSSIBLE.** `अक्षराणि` is
// written — the general UAX #29 reader, over the three `spec/*.tsv` tables,
// conformant on all 766 cases of the UCD's `GraphemeBreakTest.txt`. Taking the
// segmenter out of `nidana.t1` is an edit to that file and to the ratchet below
// that counts its real bodies, so it is named here and left to the row that
// owns it — the same shape of handover `sanskrit_text.t1`'s numeral section
// records for `वाक्यविभाग`'s seven readers.
//
// # What is ported and what is a separate row
//
// `crates/sanskrit-text/src/segment.rs` computes the UAX #29 **extended**
// grapheme cluster in full — GB1–GB999, Hangul jamo, regional-indicator
// parity, ExtPict ZWJ — over `GRAPHEME_BREAK` (1386 rows), `INCB` (473) and
// `EXTENDED_PICTOGRAPHIC` (156) in `tables.rs`. **That general port is its own
// row**: 2015 table rows plus four state machines is larger than any file in
// this crate, and none of it is reachable from a mnemonic.
//
// What `nidana.t1` ports is the DEVANAGARI RESTRICTION, and over this domain it
// is not an approximation but the same function. Inside U+0900–U+097F only four
// rules can fire — GB9, GB9a, GB9c, GB999 — and the ranges are read off
// `tables.rs`. The note here used to record a measurement: "the ported
// segmenter and `sanskrit_text::aksharas` agree on the SLICES, not merely the
// counts, for all 74 registry names of `spec/mnemonics-riscv64.src.tsv` plus
// the four acceptance probes — 81 names, 0 mismatches."
//
// **THAT MEASUREMENT DOES NOT REPRODUCE, AND THE REASON IS A REAL DEFECT.**
// `निदान ॱ सङ्केताङ्कः` (`nidana.t1:153`) writes its UTF-8 arithmetic as one
// `आरभ्य`/`समाप्तम्` nest whose brackets are UNBALANCED — the three-अष्टक
// return opens five groups and closes six, the four-अष्टक return opens seven
// and closes nine. The parser stops at the extra `समाप्तम्` instead of
// refusing it, so the LAST term is silently dropped and every code point is
// rounded down to a multiple of ६४ : executed, that routine answers U+0900 for
// अ (U+0905), U+0940 for the virāma (U+094D) and U+1F000 for U+1F600.
// `निदान ॱ अक्षरविभागः` therefore answers 15 for `अक्षर` — the whole word as
// ONE akṣara — so the Devanagari segmenter is not currently segmenting
// anything, and `अक्षरदूरम्` above it is comparing whole words.
//
// The measurement was recorded before `.t1` bodies executed (`D-002j`), which
// is how it could be both written down and false. It is corrected rather than
// deleted, because the number was cited elsewhere. Fixing `nidana.t1` is
// outside the row that found this; `sanskrit_text.t1`'s `सङ्केतमूल्यम्` carries
// the reproduction in its margin and deliberately does NOT copy the idiom —
// every group there is balanced and every partial sum is a named local.
//
// # The ratchet
//
// A full symbol table and a full count of real bodies would BOTH read 100%
// while the port silently computed a CODE-POINT distance and rendered nothing
// — which is the exact defect this row exists to prevent. So the third test
// below measures the two things that cannot be faked:
//
//   * which UAX #29 break rules the segmenter actually RETURNS, and whether
//     GB9c — the conjunct rule, the one that makes `क्ष` a single akṣara — is
//     among them; and
//   * whether `विवरणम्` actually substitutes an argument into a template.
//
// Both are matched with COMMENTS STRIPPED and at the statement site, never a
// bare `contains`. That is not a precaution in the abstract: `विच्छेदनियमः`
// appears SIX times in `nidana.t1` and only ONE of them is a return — the
// other five are prose explaining the rule. A bare `contains` would report a
// code-point segmenter as applying every rule in UAX #29.
// ══════════════════════════════════════════════════════════════════════

fn diagnostic_text() -> String {
    std::fs::read_to_string(crate_src().join("nidana.t1")).expect("nidana.t1 exists")
}

/// The stub marker, spelled once more here for the reason `ENCODER_STUB` gives.
const DIAGNOSTIC_STUB: &str = "प्रत्यागमनम् उक्तम् अपूर्णम् इति";

/// `nidana.t1` with every `॰` comment removed.
///
/// **The instrument the ratchet below depends on.** Every rule name, every
/// routine name and the word `अक्षर` itself are written repeatedly in this
/// port's prose, because the port explains what it is doing. Measuring against
/// the raw file would count those explanations as behaviour.
fn diagnostic_code_only(text: &str) -> String {
    text.lines()
        .map(|l| match l.find('॰') {
            Some(i) => &l[..i],
            None => l,
        })
        .collect::<Vec<&str>>()
        .join("\n")
}

/// Every symbol `nidana.rs`, `parse.rs` and `segment.rs` define that this port
/// is responsible for, in the spelling `निदान` gives it.
///
/// The rows below the ported functions are not `pub` items of any of the three.
/// They are expressions those files write that T1 has to spell out: a `&str`
/// comparison, a `char` decoded from a `&str`, `Vec::push`, `String::push_str`,
/// and the `format!("{{{n}}}")`/`replace` pair — none of which T1 has.
const DIAGNOSTIC_SYMBOLS: &[(&str, &str)] = &[
    // nidana.rs
    ("Language::Sanskrit", "संस्कृतभाषा"),
    ("Language::English", "आङ्ग्लभाषा"),
    ("struct Diagnostic", "संरचना निदानम्"),
    ("struct Row", "संरचना निदानपङ्क्ति"),
    ("rows", "निदानपङ्क्तयः"),
    ("row", "निदानपङ्क्तिः"),
    ("Diagnostic::render", "विवरणम्"),
    // parse.rs — the row's own three
    ("ParseError::message", "सन्देश"),
    ("distance", "अक्षरदूरम्"),
    ("nearest", "आसन्नतमम्"),
    // segment.rs — the akṣara layer this row had to port to have a distance
    ("aksharas", "अक्षरविभागः"),
    ("aksharas_count", "अक्षरसंख्या"),
    ("gcb", "विस्तारवर्गीकरणम्"),
    ("incb", "अन्तर्वर्गीकरणम्"),
    ("Cursor::new", "दर्शकारम्भः"),
    ("Cursor::push", "दर्शकवर्धनम्"),
    ("Cursor::breaks_before", "विच्छेदः"),
    // what T1 has to spell out
    ("UTF-8 sequence width", "अष्टकसंख्या"),
    ("char at a byte offset", "सङ्केताङ्कः"),
    ("aksharas().collect() — a", "प्रथमविभजनम्"),
    ("aksharas().collect() — b", "द्वितीयविभजनम्"),
    ("a[i] != b[j]", "अक्षरतुल्यम्"),
    ("nearest's relative threshold", "दूरसीमा"),
    ("&str == &str, for a code", "अष्टकतुल्यम्"),
    ("rows(): the per-row push", "निदानपङ्क्तियोजनम्"),
    ("String::new, for the message", "विवरणारम्भः"),
    ("String::push", "विवरणयोजनम्"),
    ("format!(\"{{{n}}}\")", "पदार्थाङ्कः"),
    ("str::replace's substitution", "पदार्थलेखनम्"),
];

/// The fields of the two records, checked inside their own struct body.
const DIAGNOSTIC_FIELDS: &[(&str, &[(&str, &str)])] = &[
    (
        "निदानम्",
        &[
            ("Diagnostic::code", "कूट"),
            ("Diagnostic::args: first", "पदार्थारम्भ"),
            ("Diagnostic::args: count", "पदार्थसंख्यान"),
        ],
    ),
    (
        "निदानपङ्क्ति",
        &[
            ("Row::code", "कूट"),
            ("Row::term", "पदम्"),
            ("Row::devanagari", "संस्कृतम्"),
            ("Row::english", "आङ्ग्लम्"),
        ],
    ),
];

#[test]
fn the_diagnostic_port_declares_every_symbol_of_the_rust_original() {
    let text = diagnostic_text();

    // Matched against the DECLARATION, never against the file. Every name
    // below is ALSO written in this port's prose — it explains each one — so a
    // bare `contains` could not fail for a lost routine.
    let declared = |t1: &str| {
        text.contains(&format!("सार्वजनिक {t1} "))
            || text.contains(&format!("सार्वजनिक वृत्तिः {t1} "))
            || text.contains(&format!("सार्वजनिक चरः {t1} "))
            // a routine taking no argument is followed by `ददाति`, not a space
            || text.contains(&format!("सार्वजनिक वृत्तिः {t1} ददाति"))
    };
    let mut missing: Vec<String> = DIAGNOSTIC_SYMBOLS
        .iter()
        .filter(|(_, t1)| !declared(t1))
        .map(|(rust, _)| (*rust).to_string())
        .collect();

    // Fields, inside their own struct body, matched as `name ॱॱ ` — the
    // annotation that makes it a FIELD — so a field cannot be satisfied by a
    // comment or by a longer name it is a prefix of.
    for (name, fields) in DIAGNOSTIC_FIELDS {
        match struct_body(&text, &format!("सार्वजनिक संरचना {name} आरभ्य"))
        {
            None => missing.push(format!("struct {name}")),
            Some(body) => missing.extend(
                fields
                    .iter()
                    .filter(|(_, t1)| !body.contains(&format!("{t1} ॱॱ ")))
                    .map(|(rust, _)| (*rust).to_string()),
            ),
        }
    }

    // The module line. `vishlesana.t1` recorded the defect of the right
    // filename over the wrong module; this file states which module it is.
    if !text.starts_with("मण्डलम् निदान ") {
        missing.push("मण्डलम् निदान".to_string());
    }

    let total = DIAGNOSTIC_SYMBOLS.len()
        + DIAGNOSTIC_FIELDS
            .iter()
            .map(|(_, f)| f.len())
            .sum::<usize>();
    println!(
        "METRIC sadhana_t1_diagnostic_symbols {}",
        total - missing.len()
    );
    assert!(
        missing.is_empty(),
        "nidana.t1 does not declare {} of {} symbols: {}",
        missing.len(),
        total,
        missing.join(", ")
    );
}

/// The one routine of this port that CANNOT be written, and what it waits on.
///
/// **NOTHING IS BLOCKED IN THIS PORT ANY MORE, AND THE LIST IS EMPTY BY
/// DESIGN.**
///
/// `निदानपङ्क्तयः` was the one entry. Its recorded blocker read "no include and
/// no file reader: spec/diagnostics.tsv cannot be reached from T1" — true when
/// it was written, and stale twice over by `D-002i2`: ADR-0019's `समावेशः`
/// gave T1 an include, and `("निदानकोशः", "diagnostics.tsv")` — `anita.rs:201`
/// — gave that include a name for this file. `निदानपङ्क्तियोजनम्`, the per-row
/// work, was already written, so the routine is the loop over it that the note
/// said it would be, and nothing else in `nidana.t1` moved.
///
/// **WHAT THE EMPTY ARENA WAS COSTING.** `निदानपङ्क्तिः` searches
/// `निदानपङ्क्तिकोश` and answered ० for every code while nothing had filled it,
/// so `विवरणम्` had no template and every diagnostic in the tree rendered as
/// its bare code. Eight routines downstream read as blocked on their own
/// account and were blocked on this one.
///
/// The list stays here, empty, rather than being deleted: the loop below is
/// what refuses a stub that is not accounted for, and a port that regains one
/// should have to name it.
const DIAGNOSTIC_BLOCKED: &[(&str, &str)] = &[];

#[test]
fn the_diagnostic_port_reports_how_much_of_it_is_real() {
    let text = diagnostic_text();
    let functions = text.matches("सार्वजनिक वृत्तिः").count();
    let stubs = text.matches(DIAGNOSTIC_STUB).count();
    let real = functions - stubs;
    println!("METRIC sadhana_t1_diagnostic_functions {functions}");
    println!("METRIC sadhana_t1_diagnostic_stubs {stubs}");
    println!("METRIC sadhana_t1_diagnostic_real {real}");

    // Ratchet. Raise it as routines land; it may never fall.
    //
    // RAISED TO 26 FROM 24 BY `D-002i2`. The two are `निदानपङ्क्तयः` — whose
    // blocker `anita.rs:201`'s `("निदानकोशः", "diagnostics.tsv")` cleared, so
    // that the registry arena is filled for the first time — and
    // `शीर्षपङ्क्तिः`, which refuses the `code term devanagari english` header
    // row. The header needs refusing HERE and nowhere else in the crate:
    // `कोष्ठपङ्क्तिः` and the other readers search for a name no header
    // carries and so refuse it by accident, while this routine files every row
    // it walks and would file the header as registry row १.
    assert!(
        real >= 26,
        "only {real} of {functions} diagnostic routines have a real body (was 26, \
         raised from 24 by D-002i2 when anita.rs's TABLES learned निदानकोशः)"
    );

    // And every stub must be one that is blocked, by name. A count alone is
    // satisfied by any stub, so stubbing `अक्षरदूरम्` and writing
    // `निदानपङ्क्तयः` would hold the number and lose the account.
    for (t1, blocker) in DIAGNOSTIC_BLOCKED {
        match routine_body(&text, t1) {
            None => panic!("{t1}: declares no routine"),
            Some(b) if !b.contains(DIAGNOSTIC_STUB) => {
                panic!("{t1}: has a body; name the row that cleared its blocker — {blocker}")
            }
            Some(_) => {}
        }
    }
    assert_eq!(
        stubs,
        DIAGNOSTIC_BLOCKED.len(),
        "{stubs} routines are stubs and {} are blocked; nothing in this port is \
         blocked any more, so a stub here is a routine that had a body and lost it",
        DIAGNOSTIC_BLOCKED.len()
    );
}

/// The four UAX #29 rules that can fire inside Devanagari, and the ONE that
/// makes this port akṣara-level rather than code-point-level.
///
/// GB9c is the conjunct rule: `क` + virāma + `ष` stays one cluster. Without it
/// `क्ष` is three akṣaras, `distance("क्ष", "क")` is 3 instead of 1, and the
/// suggestion this row exists to make is suppressed.
const DIAGNOSTIC_BREAK_RULES: &[(&str, &str)] = &[
    ("GB9 — Extend attaches", "योजकनियमः"),
    ("GB9a — SpacingMark attaches", "मात्रानियमः"),
    ("GB9c — the conjunct rule", "संयोगनियमः"),
    ("GB999 — otherwise, break", "विच्छेदनियमः"),
];

#[test]
fn the_diagnostic_port_reports_whether_it_segments_or_renders_anything() {
    // **This is the test that stops the two above from reading as "done".**
    //
    // Between them they say every symbol of `nidana.rs`, `parse.rs` and
    // `segment.rs` is declared and every routine but the blocked one has a real
    // body — and BOTH WOULD STILL HOLD over a port that computed a code-point
    // distance and rendered nothing. Five ports today were caught by exactly
    // this gap, which is why `ir.t1` added `ir_kinds_built`, `utsarjana.t1`
    // `codegen_emitters_real`, `encode.t1` `encoder_program_routines_real` and
    // `vakyavibhaga.t1` `t0_reader_token_consumers`. This is the diagnostic
    // path's number.
    let text = diagnostic_text();
    let code = diagnostic_code_only(&text);

    // A rule the segmenter ACTUALLY RETURNS, at the return site, with comments
    // stripped. `विच्छेदनियमः` alone appears 6 times in the file and only 1 of
    // them is a return; the other 5 are prose about the rule.
    let applied: Vec<&str> = DIAGNOSTIC_BREAK_RULES
        .iter()
        .filter(|(_, t1)| code.contains(&format!("प्रत्यागमनम् {t1} ।")))
        .map(|(rust, _)| *rust)
        .collect();

    // Does the renderer actually substitute? `विवरणम्` must CALL the
    // substitution, not merely declare it — a render that copies the template
    // through renders a message with `{0}` still in it, which is the failure
    // `rendering_substitutes_every_argument_in_both_languages` pins in Rust.
    let renders = routine_body(&code, "विवरणम्")
        .map(|b| b.matches("पदार्थलेखनम्").count())
        .unwrap_or(0);

    // And does the distance actually walk akṣaras? `अक्षरदूरम्` must reach the
    // segmenter, or it is a code-point Levenshtein with a Sanskrit name.
    let distance_segments = routine_body(&code, "अक्षरदूरम्")
        .map(|b| b.matches("विभजनम्").count())
        .unwrap_or(0);

    println!(
        "METRIC sadhana_t1_diagnostic_break_rules_declared {}",
        DIAGNOSTIC_BREAK_RULES.len()
    );
    println!(
        "METRIC sadhana_t1_diagnostic_break_rules_applied {}",
        applied.len()
    );
    println!("METRIC sadhana_t1_diagnostic_render_substitutions {renders}");
    println!("METRIC sadhana_t1_diagnostic_distance_segmentations {distance_segments}");

    // Not vacuous: stripping comments must have left the file, not emptied it.
    assert!(
        code.contains("सार्वजनिक वृत्तिः अक्षरविभागः "),
        "comment stripping ate the code; every count above would be zero for \
         the wrong reason"
    );

    // ALL FOUR RULES, and GB9c BY NAME. A code-point segmenter returns GB999
    // and nothing else, so this is the assertion that cannot be satisfied by a
    // port that lost the akṣara.
    assert_eq!(
        applied.len(),
        DIAGNOSTIC_BREAK_RULES.len(),
        "only {} of {} UAX #29 rules are returned by the segmenter ({}); a port \
         that applies fewer is not akṣara-level",
        applied.len(),
        DIAGNOSTIC_BREAK_RULES.len(),
        applied.join(", ")
    );
    assert!(
        code.contains("प्रत्यागमनम् संयोगनियमः ।"),
        "the segmenter never returns GB9c — the conjunct rule. Without it क्ष \
         is three akṣaras, distance(क्ष, क) is 3 rather than 1, and D-002f8's \
         whole point is lost"
    );

    // The renderer substitutes, and the distance segments. Both are floors.
    assert!(
        renders >= 1,
        "विवरणम् never calls पदार्थलेखनम्; nothing is substituted and no \
         diagnostic is actually rendered"
    );
    assert_eq!(
        distance_segments, 2,
        "अक्षरदूरम् reaches the segmenter {distance_segments} time(s), not 2; \
         both operands must be split into akṣaras or the distance is \
         code-point-level"
    );
}

/// The acceptance the row states, asserted where it can actually RUN.
///
/// `.t1` sources are parsed, not executed, so this crate cannot evaluate
/// `आसन्नतमम्`. The behaviour is pinned by `crates/sadhana/src/parse.rs`'s own
/// tests — `an_unknown_mnemonic_is_an_error_with_a_suggestion`,
/// `an_unrelated_name_gets_no_suggestion`, `a_near_miss_still_gets_its_
/// suggestion` and `distance_counts_aksaras_not_code_points`. What this test
/// pins is that the PORT still carries the two decisions those tests encode,
/// so neither can be quietly dropped from the T1 side.
#[test]
fn the_diagnostic_port_keeps_the_two_decisions_the_row_records() {
    let code = diagnostic_code_only(&diagnostic_text());

    // 1. THE THRESHOLD IS RELATIVE. A fixed 2 offered व्यत्ययः for क्षत्रियः.
    //    `दूरसीमा` must divide by three and floor at one; a literal comparison
    //    against a constant distance would be the defect returning.
    //    THE SPELLING MOVED, NOT THE DECISION — 2026-09-02. This asserted
    //    `भागः ३`, and `भागः` is a RETIRED operator: ADR-0028 froze
    //    `विभाजनम्` for divide and `nidana.t1:636` was the corpus's last site
    //    still writing the old word. It went unnoticed because a name the
    //    grammar does not know simply parsed as one more name — and once
    //    juxtaposed application landed (W-187) it parsed as a CALL,
    //    `दैर्घ्यम्(भागः, ३)`, which is how `अर्थ` finally reported it as an
    //    undeclared `भागः`. The threshold is still relative; only the operator
    //    is spelled as the frozen grammar spells it.
    let threshold = routine_body(&code, "दूरसीमा").expect("दूरसीमा is declared");
    assert!(
        threshold.contains("विभाजनम् ३"),
        "दूरसीमा no longer divides by three; the threshold has stopped being \
         relative and an unrelated suggestion is worse than none"
    );
    assert!(
        !threshold.contains("भागः"),
        "`दूरसीमा` writes `भागः` again. ADR-0028 retired it for `विभाजनम्`, and \
         a retired operator does not fail loudly — it parses as a NAME and \
         then as a CALL, which is a wrong program that looks like a right one"
    );
    assert!(
        threshold.contains("प्रत्यागमनम् १ ।"),
        "दूरसीमा no longer floors at one; a pure ratio rejects योगः for योग, \
         which is precisely the typo worth catching"
    );
    assert!(
        threshold.contains("अक्षरसंख्या"),
        "दूरसीमा measures something other than akṣaras; the threshold is a \
         third of the LONGER NAME IN AKṢARAS"
    );

    // 2. THE VIRĀMA IS THE LINKER. This single range test is what GB9c fires
    //    on, and U+094D is २३८१. Lose it and every conjunct splits.
    let incb = routine_body(&code, "अन्तर्वर्गीकरणम्").expect("अन्तर्वर्गीकरणम् is declared");
    assert!(
        incb.contains("समम् २३८१"),
        "अन्तर्वर्गीकरणम् no longer recognises U+094D (२३८१) as the Linker; \
         GB9c can never fire and क्ष becomes three akṣaras"
    );
}

// ══════════════════════════════════════════════════════════════════════
// `D-002f3` — THE STATEMENT SPLITTER, AND WHETHER IT ACTUALLY WALKS.
//
// **THE RATCHET QUESTION, asked of the row's own acceptance.**
// `the_t0_reader_reports_whether_it_reads_any_token` now reads
// `token_consumers 5` and `statement_kinds_read 3`, off the `०` it was
// pinned at. Both of those numbers WOULD STILL READ EXACTLY THE SAME if the
// splitter split nothing:
//
//   · `t0_reader_token_consumers` counts routines whose DECLARATION takes a
//     `पदविभागॱचिह्नक`. Five routines can each take a token and do real work
//     on the token they are handed while NOTHING EVER WALKS `चिह्नककोश`. A
//     splitter that is never entered splits nothing, and a count of
//     declarations cannot tell the two apart — which is the same shape as
//     `symbols-declared 100%` on a module that does nothing, the failure
//     that caught six ports today.
//   · `t0_reader_statement_kinds_read` counts three STATIC call sites. Text
//     matches wherever it sits, including inside a routine no caller ever
//     reaches and over an arena never read.
//
// So the answer to "what number here would read as done while the splitter
// still splits nothing" is BOTH OF THE ACCEPTANCE NUMBERS, and what neither
// of them measures is THE WALK. This test measures the walk, in the two
// places it cannot be faked, plus the three post-checks the row names.
//
// NOTHING BELOW IS A BARE `contains` ON A NAME. `vakyavibhaga.t1`'s own
// mutation M9 showed a bare `contains` reporting 2 consumers FROM PROSE
// ALONE with the file otherwise unchanged, so every match here is an
// EXPRESSION (an index, a comparison) or a CALL SITE on a code line, and
// `t1_code_lines` drops the `॰`-led prose before any of them is counted.
// ══════════════════════════════════════════════════════════════════════

/// The three delimiters the splitter must tell apart, with the `lex.rs` kind.
///
/// `ॱॱ` is `चिह्नभेद` and NOT `विच्छेदकभेद`: `पदविभागॱविरामचिह्न` classifies
/// the six-octet `ॱॱ` as `चिह्नभेद`, and `विच्छेदकभेद` is `ऽ` — `Kind::Separator`,
/// which `parse.rs` has no arm for. Getting that pair backwards is the one
/// substitution that would leave every label unread and every `ऽ` eating a
/// statement, so the mapping is written out here rather than inferred.
const T0_READER_DELIMITERS: &[(&str, &str)] = &[
    ("Kind::Danda — `।` ends a sentence", "दण्डभेद"),
    ("Kind::DoubleDanda — `॥` pairs, ADR-0012", "युग्मदण्डभेद"),
    ("Kind::LabelMark — `ॱॱ` ends a label", "चिह्नभेद"),
];

/// The three post-checks `D-002f3` names, each of which must be DECLARED AND
/// CALLED. Declared-but-never-called is precisely `D-002f1`'s shape: every
/// record and every appender existed and no caller reached any of them.
const T0_READER_POST_CHECKS: &[(&str, &str)] = &[
    (
        "L02 — a `वैश्विकम्` naming no label, checked HERE not at link time (B-096)",
        "वैश्विकचिह्नपरीक्षा",
    ),
    (
        "P25 — an unclosed directive, reported at the mark that OPENED it",
        "अनावृतनिर्देशपरीक्षा",
    ),
    (
        "P02 — a trailing statement with no daṇḍa",
        "अन्तिमवाक्यपरीक्षा",
    ),
];

#[test]
fn the_statement_splitter_walks_the_token_arena() {
    let text = t0_reader_text();
    let code = t1_code_lines(&text);

    // 1. INDEXING THE INPUT ARENA. `पदविभागॱचिह्नककोश अङ्कः` is an index
    //    EXPRESSION — the arena, the index keyword — and not a mention of the
    //    arena's name, which this file's prose also makes.
    let arena_reads = code
        .iter()
        .filter(|l| l.contains("पदविभागॱचिह्नककोश अङ्कः"))
        .count();

    // 2. DISPATCHING ON THE DELIMITERS, matched as a COMPARISON.
    let dispatched: Vec<&str> = T0_READER_DELIMITERS
        .iter()
        .filter(|(_, t1)| {
            code.iter()
                .any(|l| l.contains(&format!("समम् पदविभागॱ{t1}")))
        })
        .map(|(rust, _)| *rust)
        .collect();

    // 3. THE POST-CHECKS, declared AND called. The call site is a code line
    //    that is not the declaration.
    let wired: Vec<&str> = T0_READER_POST_CHECKS
        .iter()
        .filter(|(_, t1)| {
            let declared = code
                .iter()
                .any(|l| l.starts_with(&format!("सार्वजनिक वृत्तिः {t1} ")));
            let called = code
                .iter()
                .any(|l| !l.starts_with("सार्वजनिक वृत्तिः") && l.contains(*t1));
            declared && called
        })
        .map(|(rust, _)| *rust)
        .collect();

    println!("METRIC sadhana_t1_t0_reader_arena_reads {arena_reads}");
    println!(
        "METRIC sadhana_t1_t0_reader_delimiters_dispatched {}",
        dispatched.len()
    );
    println!(
        "METRIC sadhana_t1_t0_reader_post_checks_wired {}",
        wired.len()
    );

    // Not vacuous: the arena must still be spelled the way the lexer spells
    // it, or a rename would drive every number above to zero and read as a
    // regression that never happened.
    assert!(
        t0_lexer_text().contains("सार्वजनिक चरः चिह्नककोश "),
        "lex.t1 no longer declares चिह्नककोश under that name; the counts \
         above would be zero for a spelling reason and not a real one"
    );

    // THE WALK ITSELF: a loop, in the splitter, over the lexer's arena. Each
    // of the three is checked INSIDE `वाक्यविभाजनम्`'s own body, so moving
    // the index expression into a routine nothing calls does not hold it.
    let walk = routine_body(&text, "वाक्यविभाजनम्")
        .expect("वाक्यविभाजनम् — the statement splitter — is declared");
    assert!(
        walk.contains("यावत्"),
        "वाक्यविभाजनम् has no यावत्: a splitter that does not loop reads one \
         token at most and splits nothing"
    );
    assert!(
        walk.contains("पदविभागॱचिह्नककोश अङ्कः"),
        "वाक्यविभाजनम् never indexes पदविभागॱचिह्नककोश: whatever else the \
         file declares, the input arena is not being read"
    );
    assert!(
        walk.contains("पदविभागॱचिह्नकसूचकाङ्क"),
        "वाक्यविभाजनम् does not bound its walk by पदविभागॱचिह्नकसूचकाङ्क, so \
         it does not walk the tokens the lexer actually wrote"
    );

    // ADR-0012'S OPENER, PINNED AS AN EXPRESSION — AND IT IS HERE BECAUSE A
    // MUTATION SURVIVED WITHOUT IT. Replacing `उद्घाटकः भवति सूचकाङ्कः` with
    // `उद्घाटकः भवति ०` leaves every count on this test and on the acceptance
    // test EXACTLY WHERE THEY ARE — same routines, same declarations, same
    // call sites, same index expressions — while a `॥` no longer records
    // where it opened. P25 then can never fire, and an unpaired `॥` swallows
    // the directive after it, which is the one thing ADR-0012 and this row
    // both forbid by name. `उद्घाटकः` holds an arena INDEX rather than a flag
    // for exactly this reason: a flag would know that a directive is open and
    // not know where it began, and the diagnostic must land on the OPENING
    // mark. This is the same discipline `अन्तर्वर्गीकरणम्`'s `समम् २३८१` is
    // pinned with — a count cannot see a load-bearing value, so the value is
    // named.
    assert!(
        walk.contains("उद्घाटकः भवति सूचकाङ्कः"),
        "वाक्यविभाजनम् no longer records WHERE a ॥ opened, so P25 cannot be \
         raised at the opening mark and an unpaired ॥ swallows what follows it"
    );

    // Ratchets. Each may be raised and may never fall.
    assert!(
        arena_reads >= 16,
        "only {arena_reads} index expressions into पदविभागॱचिह्नककोश (was 16, \
         raised from 8 by D-002f5, whose four arms each take the head token \
         and its operand out of the arena); the splitter and the directive \
         reader together read the arena in fewer places than they did"
    );
    assert_eq!(
        dispatched.len(),
        T0_READER_DELIMITERS.len(),
        "the splitter compares only {} of the {} delimiter kinds ({}); a \
         reader that cannot tell । from ॥ from ॱॱ splits nothing whatever \
         else it declares",
        dispatched.len(),
        T0_READER_DELIMITERS.len(),
        dispatched.join(", ")
    );
    assert_eq!(
        wired.len(),
        T0_READER_POST_CHECKS.len(),
        "only {} of the {} post-checks are declared AND called ({})",
        wired.len(),
        T0_READER_POST_CHECKS.len(),
        wired.join(", ")
    );
}

// ══════════════════════════════════════════════════════════════════════
// `D-002f5` — THE DIRECTIVES THAT EMIT NO BYTES, AND WHETHER THEY DO ANYTHING.
//
// **THE RATCHET QUESTION, asked of this row's own acceptance.**
// `the_t0_reader_port_reports_how_much_of_it_is_real` moved 36 → 55 and
// `..._token_consumers` 5 → 6. NEITHER NUMBER CAN SEE A DIRECTIVE BEING
// APPLIED. Nineteen routines can each have a real body, take a token, and
// index an arena while `निर्देशानुष्ठानम्` is never called from the walk — in
// which case `॥ कोष्ठकम् ॱदत्त ॥` still leaves `वर्तमानकोष्ठक` on `ॱपाठ`,
// `वैश्विकसंख्यानम्` is still `०`, and every count above reads exactly as it
// reads now. That is the shape six ports were caught in, and it is the one
// this row's own predecessor (`D-002f3`) had to add a second test for.
//
// So the smallest thing that would read as green while the directive reader
// did nothing is **nineteen unreached routines**, and what is measured below
// is therefore the WIRING and the EFFECTS:
//
//   · the walk calls the applier, and the applier dispatches to all four arms;
//   · each arm's EFFECT is present as an expression — the section assignment,
//     the globals push, the `संरेखयोजनम्` request, the `रिक्तमानम्` advance;
//   · the numeral reader is `सङ्ख्यामानम्` (a MAGNITUDE) at the `संरेखः` site,
//     which is `W-075`, and the guards that make a boundary a boundary;
//   · the acceptance itself: an already-aligned `ॱदत्त` offset emits NOTHING.
//
// Nothing here is a bare `contains` on a name. `vakyavibhaga.t1`'s prose names
// every routine it declares — mutation M9 on that file showed a bare
// `contains` reporting two consumers FROM PROSE ALONE — so every match is an
// expression or a call site on a code line, after `t1_code_lines` has dropped
// the `॰`-led prose.
// ══════════════════════════════════════════════════════════════════════

/// The four arms `apply_directive`'s first half has, each of which must be
/// DECLARED and DISPATCHED TO from `निर्देशानुष्ठानम्`'s own body.
///
/// Declared-but-never-dispatched is the exact failure this section exists for:
/// `parse.rs` reaches its arms through `match head.text.as_str()`, and a T1
/// port that declares the four and matches none of them is a reader that
/// accepts `॥ कोष्ठकम् ॱदत्त ॥` and changes no section.
const T0_READER_DIRECTIVE_ARMS: &[(&str, &str)] = &[
    ("कोष्ठकम् — switch section (P15/P16)", "कोष्ठकनिर्देशः"),
    ("वैश्विकम् — export a name (P16)", "वैश्विकनिर्देशः"),
    ("स्थानीयम् — the default, and it says so", "स्थानीयनिर्देशः"),
    ("संरेखः — a BOUNDARY, not a count (W-071)", "संरेखनिर्देशः"),
];

/// The seven diagnostics this row raises, matched AT THE RAISE SITE.
///
/// A code is an octet run here (`कूटरचना` builds each one; doc 15 refuses the
/// Latin letters `P` and `L`), so a declaration and an initialiser both mention
/// the name. What is counted is the name appearing as an ARGUMENT to a push —
/// `दोषचिह्नकयोजनम् आरभ्य <code>` — which neither the declaration nor
/// `कूटरचना`'s `अङ्कः … अन्तः भवति` lines can satisfy.
const T0_READER_DIRECTIVE_CODES: &[(&str, &str)] = &[
    ("P13 — not a directive at all", "अज्ञातनिर्देशकूटः"),
    ("P14 — named by doc 02, not implemented", "असमर्थितनिर्देशकूटः"),
    ("P15 — not a section", "अज्ञातकोष्ठककूटः"),
    ("P16 — needs at least one value", "अपूर्णनिर्देशकूटः"),
    ("P19 — not a numeral / not a boundary", "असङ्ख्यकूटः"),
    ("P29 — does not fit 64 bits (W-075)", "अतिप्रवाहकूटः"),
    ("P30 — a count carries no sign (W-075)", "सचिह्नगणनाकूटः"),
];

#[test]
fn the_directive_reader_applies_the_directives_that_emit_no_bytes() {
    let text = t0_reader_text();
    let code = t1_code_lines(&text);

    // 1. THE WALK CALLS THE APPLIER. Checked inside `वाक्यविभाजनम्`'s own
    //    body, so moving the call into a routine nothing reaches does not
    //    satisfy it. `parse.rs` calls `apply_directive` from exactly one
    //    place — the `॥` arm that CLOSES a directive — and this is that.
    let walk = routine_body(&text, "वाक्यविभाजनम्")
        .expect("वाक्यविभाजनम् — the statement splitter — is declared");
    assert!(
        walk.contains("निर्देशानुष्ठानम् आरभ्य वाक्यारम्भः"),
        "वाक्यविभाजनम् never calls निर्देशानुष्ठानम्: every routine D-002f5 \
         declares can be real and unreached, and `॥ कोष्ठकम् ॱदत्त ॥` then \
         changes no section"
    );

    // 2. THE APPLIER DISPATCHES TO ALL FOUR ARMS, from its own body.
    let apply = routine_body(&text, "निर्देशानुष्ठानम्")
        .expect("निर्देशानुष्ठानम् — apply_directive's first half — is declared");
    let dispatched: Vec<&str> = T0_READER_DIRECTIVE_ARMS
        .iter()
        .filter(|(_, t1)| {
            code.iter()
                .any(|l| l.starts_with(&format!("सार्वजनिक वृत्तिः {t1} ")))
                && apply.contains(&format!("{t1} आरभ्य वाक्यारम्भः"))
        })
        .map(|(rust, _)| *rust)
        .collect();

    // 3. THE REGISTRY IS ASKED FIRST AND IT REFUSES. P13/P14 are the point of
    //    the row's first sentence: a directive doc 02 named and nothing
    //    implements is REFUSED, not ignored. The lookup must be a real walk of
    //    `निर्देशकोश` bounded by its own index, not a constant.
    let lookup = routine_body(&text, "निर्देशान्वेषणम्")
        .expect("निर्देशान्वेषणम् — directives().find(..) — is declared");
    assert!(
        lookup.contains("यावत्") && lookup.contains("निर्देशकोश अङ्कः"),
        "निर्देशान्वेषणम् does not walk निर्देशकोश; P13 would then fire for \
         every directive or for none, whichever the constant says"
    );
    assert!(
        lookup.contains("निर्देशसूचकाङ्क"),
        "निर्देशान्वेषणम् does not bound its walk by निर्देशसूचकाङ्क, so it \
         does not read the rows the registry reader will actually write"
    );

    // 4. THE DIAGNOSTICS, AT THE RAISE SITE.
    let raised: Vec<&str> = T0_READER_DIRECTIVE_CODES
        .iter()
        .filter(|(_, t1)| {
            code.iter()
                .any(|l| l.contains(&format!("दोषचिह्नकयोजनम् आरभ्य {t1} ")))
        })
        .map(|(rust, _)| *rust)
        .collect();

    println!(
        "METRIC sadhana_t1_t0_reader_directive_arms_dispatched {}",
        dispatched.len()
    );
    println!(
        "METRIC sadhana_t1_t0_reader_directive_codes_raised {}",
        raised.len()
    );

    assert_eq!(
        dispatched.len(),
        T0_READER_DIRECTIVE_ARMS.len(),
        "only {} of the {} no-byte directive arms are declared AND dispatched \
         to from निर्देशानुष्ठानम् ({})",
        dispatched.len(),
        T0_READER_DIRECTIVE_ARMS.len(),
        dispatched.join(", ")
    );
    assert_eq!(
        raised.len(),
        T0_READER_DIRECTIVE_CODES.len(),
        "only {} of the {} diagnostics this row owns are raised at a push site \
         ({}); a code that is declared and never pushed is a refusal that \
         cannot happen",
        raised.len(),
        T0_READER_DIRECTIVE_CODES.len(),
        raised.join(", ")
    );

    // 5. EACH ARM'S EFFECT, AS AN EXPRESSION IN ITS OWN BODY. A dispatch that
    //    reaches a routine which changes nothing is the same silence with more
    //    call frames, so the four effects are named one by one.
    let section_arm = routine_body(&text, "कोष्ठकनिर्देशः").expect("कोष्ठकनिर्देशः is declared");
    assert!(
        section_arm.contains("वर्तमानकोष्ठक भवति भेदः"),
        "कोष्ठकनिर्देशः never assigns वर्तमानकोष्ठक: `॥ कोष्ठकम् ॱदत्त ॥` \
         parses, refuses nothing, and leaves every following statement in ॱपाठ"
    );
    assert!(
        section_arm.contains("कोष्ठकनाम आरभ्य"),
        "कोष्ठकनिर्देशः does not call कोष्ठकनाम, so a mistyped section name \
         is not refused by P15 — it is defaulted, which is the failure \
         Section::from_name returning Option exists to prevent"
    );

    let global_arm = routine_body(&text, "वैश्विकनिर्देशः").expect("वैश्विकनिर्देशः is declared");
    assert!(
        global_arm.contains("वैश्विकयोजनम् आरभ्य"),
        "वैश्विकनिर्देशः never pushes a global, so वैश्विकसंख्यानम् stays ० \
         and वैश्विकचिह्नपरीक्षा's L02 loop still has nothing to read"
    );

    // 6. `संरेखः`, AND THE THREE SECTIONS IT ACTS DIFFERENTLY IN.
    let align = routine_body(&text, "संरेखनिर्देशः").expect("संरेखनिर्देशः is declared");

    // ॱपाठ — THE ROW'S OWN ACCEPTANCE, FIRST HALF. Alignment in a text
    // section cannot be resolved while parsing, for the same reason a
    // branch's width cannot: relaxation has not run. So the REQUEST is
    // recorded and स्थानविन्यासः satisfies it (`W-071`).
    assert!(
        align.contains("संरेखयोजनम् आरभ्य वाक्यविभागआज्ञासूचकाङ्क"),
        "the ॱपाठ arm of संरेखनिर्देशः does not call संरेखयोजनम् at the \
         current instruction index; Program::text_aligns then stays empty and \
         `॥ संरेखः १६ ॥` in ॱपाठ is silently dropped — W-071"
    );

    // ॱदत्त — THE ROW'S OWN ACCEPTANCE, SECOND HALF: an already-aligned
    // offset emits NOTHING. Pinned as the guard EXPRESSION and not as a
    // count, because deleting `यदि पूर्तिः समम् ०` leaves every number on
    // this test where it is while `॥ संरेखः १६ ॥` at offset zero starts
    // pushing an empty Datum — which is `संरेखः` behaving like `स्थानम्`,
    // the exact confusion W-071 records.
    assert!(
        align.contains("संरेखपूर्तिः आरभ्य सरणम्"),
        "the ॱदत्त arm does not compute the padding from the current data \
         offset, so it cannot know whether any is needed"
    );
    assert!(
        align.contains("यदि पूर्तिः समम् ० आदि"),
        "संरेखनिर्देशः has no already-aligned guard: `॥ संरेखः १६ ॥` at an \
         offset already divisible by sixteen would push a Datum, and संरेखः \
         would be स्थानम् — the row's acceptance is exactly that it emits \
         NOTHING there"
    );

    // ॱरिक्त — `B-108`. It advances the location counter and nothing else,
    // which is precisely and only what स्थानम् does in a NOBITS section.
    assert!(
        align.contains("रिक्तमानम् भवति"),
        "संरेखनिर्देशः never advances रिक्तमानम्, so ॱरिक्त either refuses \
         संरेखः or ignores it — B-108 is the row that settled it does neither"
    );

    // W-075: A BOUNDARY IS A MAGNITUDE, so it is read with `सङ्ख्यामानम्`.
    // Read as a bit pattern instead, `ऋण१६` is `ffff…f0` — not a power of
    // two, so it would be refused for a reason that is not the one it broke,
    // and P30 would never be the answer to a signed boundary.
    assert!(
        align.contains("सङ्ख्यामानम् आरभ्य"),
        "संरेखनिर्देशः does not read its operand with सङ्ख्यामानम्; a boundary \
         is a magnitude and reading it as bits is W-075"
    );
    assert!(
        align.contains("सीमाङ्कः न्यूनम् २"),
        "संरेखनिर्देशः does not refuse ० and १: zero asks for a multiple of \
         nothing and one is satisfied by every address, so both are statements \
         that do nothing and read like statements that do something"
    );
    assert!(
        align.contains("सीमाङ्कः युक् आरभ्य सीमाङ्कः वियोगः १ समाप्तम्"),
        "संरेखनिर्देशः has no power-of-two guard, so `॥ संरेखः ६ ॥` is \
         accepted and the padding arithmetic below it means nothing"
    );

    // 7. NOT VACUOUS. Every assertion above is a match on this one file, so a
    //    rename of the applier would drive them all to a uniform failure that
    //    looks like a regression. The applier's own declaration is checked
    //    separately, on a code line.
    assert!(
        code.iter()
            .any(|l| l.starts_with("सार्वजनिक वृत्तिः निर्देशानुष्ठानम् ")),
        "vakyavibhaga.t1 no longer declares निर्देशानुष्ठानम्; every match \
         above would fail for a spelling reason and not a real one"
    );
}

// ═══════════════════ A FROZEN KEYWORD IS NOT A NAME ══════════════════════════
//
// `spec/grammar-t1.ebnf`'s `keyword` production is FROZEN, and it says of its
// own contents: *"Each is an ordinary Sanskrit word, so each is also a possible
// identifier — the reader must prefer the keyword, which is why they are
// enumerated here rather than recognised by shape."*
//
// That sentence has a consequence nothing in this tree was checking: **a `.t1`
// file that BINDS one of those words as a name is already illegal**, and stays
// invisible only because no T1 front end enforces the keyword list yet. When one
// arrives, every such name breaks at once, in files nobody is looking at.
//
// The collision that produced this test was `समावेशः` (`include`), declared as
// `सार्वजनिक वृत्तिः समावेशः` — `Slot::fits` — in `encode.t1`. ADR-0019 found it
// while choosing a word for the embed, reported it, and routed around it. The
// owner ruled on 2026-08-28 that the word be FREED instead, and asked for this
// guard, *"the deliverable that outlasts the rename"*. Writing it immediately
// found four more in the same file (`गणना` as a loop counter, four times, and
// `दोषः` as a parameter) and five in three files this row may not edit. Nine of
// the eleven were unknown to anyone before the list was read against the corpus.
//
// # The keyword list is READ FROM THE GRAMMAR, never transcribed
//
// `frozen_keywords` parses `spec/grammar-t1.ebnf` itself. A hardcoded copy would
// be `B-058a`'s defect exactly — a hand-typed second copy that agrees with the
// first by construction — and would go stale the next time an ADR amends the
// production, which has now happened three times since the freeze. Amend the
// grammar and this test changes what it checks, with no edit here.
//
// # What counts as USING a keyword as an identifier, and why it is BINDINGS
//
// T1's phrase structure is only partly frozen: `statement` is a **deferred
// non-terminal** that "is not defined anywhere yet" (`expression` was one too
// until ADR-0032 froze the operators on 2026-08-30). So there is no
// parser that can say of an arbitrary occurrence whether it stands in an
// identifier position, and a test that guessed would be unsound in the direction
// that matters — `प्रत्यागमनम् सत्यम् ।` returns the keyword `true` and is
// perfectly legal.
//
// What IS frozen, and what this test uses, are the two positions where a word is
// unambiguously a NAME BEING BOUND:
//
//   1. the word after a declaring keyword — `चरः X`, `सार्वजनिक वृत्तिः X`,
//      `संरचना X`, `गणना X`, and the rest of `DECLARING_KEYWORDS`;
//   2. the word before the annotation mark `ॱॱ` (ADR-0003's colon role) — the
//      shape every parameter, every struct field and every typed local has.
//
// This is deliberately the SOUND half rather than the complete one. It cannot
// see a keyword used as a bare reference — but a reference resolves to a
// binding, so a corpus with no illegal binding has no illegal reference either,
// and the position that introduces the error is the position this test guards.
// A front end (`B-080`) is what will one day check the other half; until then
// the hole is named here rather than left to be discovered.

/// The repository root. This crate's manifest sits two levels below it, and the
/// grammar and the sibling crates' `.t1` sources are both outside this crate.
fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Drop every `(* … *)` comment, keeping newlines so line numbers survive.
///
/// This runs BEFORE any quoted word is looked for, and it has to: the keyword
/// production's own ADR-0019 note contains the string `"textual inclusion"`,
/// spread over two lines, and a naive scan for `"…"` would read `textual` as a
/// thirty-third keyword. ISO EBNF comments nest, so the depth is counted.
fn strip_ebnf_comments(src: &str) -> String {
    let chars: Vec<char> = src.chars().collect();
    let mut out = String::with_capacity(src.len());
    let mut depth = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '(' && chars.get(i + 1) == Some(&'*') {
            depth += 1;
            i += 2;
            continue;
        }
        if depth > 0 && chars[i] == '*' && chars.get(i + 1) == Some(&')') {
            depth -= 1;
            i += 2;
            continue;
        }
        if depth == 0 || chars[i] == '\n' {
            out.push(chars[i]);
        }
        i += 1;
    }
    out
}

/// Every word of `spec/grammar-t1.ebnf`'s frozen `keyword` production, in the
/// order the grammar lists them, read from the grammar.
fn frozen_keywords() -> Vec<String> {
    let path = repo_root().join("spec/grammar-t1.ebnf");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("spec/grammar-t1.ebnf must be readable: {e}"));
    let bare = strip_ebnf_comments(&src);

    // The production runs from the line that opens it to the first `;` outside a
    // comment — EBNF's production terminator.
    let mut body = String::new();
    let mut inside = false;
    for line in bare.lines() {
        if !inside {
            if !line
                .strip_prefix("keyword")
                .is_some_and(|rest| rest.starts_with(char::is_whitespace))
            {
                continue;
            }
            inside = true;
        }
        match line.find(';') {
            Some(end) => {
                body.push_str(&line[..end]);
                break;
            }
            None => {
                body.push_str(line);
                body.push('\n');
            }
        }
    }
    assert!(
        inside,
        "spec/grammar-t1.ebnf has no `keyword` production; this test reads the \
         keyword list from the grammar and has nothing to read"
    );

    // Odd fields of a split on `"` are the quoted terminals.
    body.split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// The keywords that INTRODUCE a name, so that the word after one is a binding.
///
/// This subset is not in the frozen file — declarations are phrase structure and
/// `spec/grammar-t1.ebnf` freezes only three constructs of it — so it is written
/// here. It cannot drift silently all the same: every word below is asserted to
/// be in the production this test reads, so renaming one in the grammar fails
/// this file loudly instead of quietly scanning for a word that no longer exists.
const DECLARING_KEYWORDS: &[&str] = &[
    "वृत्तिः",  // fn
    "चरः",    // var
    "ध्रुवः",   // const
    "संरचना",  // struct
    "संघः",    // union
    "गणना",   // enum
    "प्रकारः", // type
];

/// ADR-0003's colon role: `name ॱॱ type`, in a parameter, a field or a local.
const ANNOTATION_MARK: &str = "ॱॱ";

/// Every `.t1` source in the tree, from every crate, sorted.
///
/// Deliberately NOT [`t1_sources`], which reads this crate's `src/` alone:
/// `crates/textapp/src/text/*.t1` are T1 sources too and the grammar binds them
/// exactly as much.
fn every_t1_source_in_the_tree() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|x| x == "t1") {
                out.push(p);
            }
        }
    }
    let mut v = Vec::new();
    walk(&repo_root().join("crates"), &mut v);
    v.sort();
    v
}

/// The CODE words of a `.t1` source — those outside a comment and outside a
/// string literal — each with its 1-based line.
///
/// The two exclusions are the grammar's own, in the grammar's own order. A
/// string is taken FIRST, because ADR-0017 put the string ahead of the comment
/// strip in the lexer and that is why `॰` inside a literal is ordinary text; a
/// doubled `इति` is the word itself (ADR-0011) and does not close. A string does
/// not cross a newline, so one line is the whole scope either needs.
///
/// Without this, `encode.t1`'s prose — which names `समावेशः` three times in `॰`
/// comments — would be reported as three more violations, and the mutation that
/// proves this test works would be indistinguishable from a comment.
fn t1_code_words(text: &str) -> Vec<(String, usize)> {
    const STRING_OPEN: &str = "उक्तम्";
    const STRING_CLOSE: &str = "इति";
    const COMMENT_MARK: char = '॰';

    let mut out = Vec::new();
    for (n, line) in text.lines().enumerate() {
        let words: Vec<&str> = line.split_whitespace().collect();
        let mut i = 0usize;
        while i < words.len() {
            if words[i] == STRING_OPEN {
                i += 1;
                while i < words.len() {
                    if words[i] == STRING_CLOSE {
                        if words.get(i + 1) == Some(&STRING_CLOSE) {
                            i += 2; // the doubled close is the word itself
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                continue;
            }
            if words[i].starts_with(COMMENT_MARK) {
                break;
            }
            out.push((words[i].to_string(), n + 1));
            i += 1;
        }
    }
    out
}

/// Every place a `.t1` source BINDS a frozen keyword as a name: the line, the
/// word, and which of the two positions it stood in.
fn keyword_bindings(text: &str, keywords: &[String]) -> Vec<(usize, String, &'static str)> {
    let words = t1_code_words(text);
    let mut hits = Vec::new();
    for (i, (word, line)) in words.iter().enumerate() {
        if !keywords.iter().any(|k| k == word) {
            continue;
        }
        if i > 0 && DECLARING_KEYWORDS.contains(&words[i - 1].0.as_str()) {
            hits.push((*line, word.clone(), "declared as a name"));
        } else if words.get(i + 1).is_some_and(|(w, _)| w == ANNOTATION_MARK) {
            hits.push((*line, word.clone(), "bound as a name before `ॱॱ`"));
        }
    }
    hits
}

/// The keyword bindings this row FOUND, REPORTED and did not fix, because the
/// files are owned by other workers and `B-112`'s ruling scopes this one to
/// `crates/sadhana-t1/src/encode.t1`.
///
/// Every one is `गणना` (`enum`) used as a loop counter — the same idiom the same
/// four sites in `encode.t1` used, and fixed here to `संख्यानम्`, which is what
/// `spec/lexicon.src.tsv:352` already glosses `count` and what the rest of the
/// tree already spells a count with (`वैश्विकसंख्यानम्`, `पदार्थसंख्यानम्`,
/// `कोष्ठसंख्यानम्`). The fix is mechanical and someone who owns those files
/// should take it.
///
/// **The count is EXACT, not a ceiling.** A `<=` would let a sixth appear inside
/// a file that already has some, which is the one thing this test exists to
/// prevent. If you fix one, lower the number in the same commit; the failure
/// message says so.
/// **EMPTIED 2026-08-28, THE SAME DAY IT WAS WRITTEN.** All five bindings were
/// renamed to `संख्यानम्` by the supervisor, who owns every lane and could edit
/// the three files the row that wrote this list could not. `अक्षरगणना` and the
/// other compounds were left alone — the rename matched on Devanagari word
/// boundaries, not substrings, which matters because `गणना` sits inside them.
///
/// The list is kept rather than deleted, because the shape is the point: a
/// worker who finds a collision in a file they do not own records it HERE with
/// an exact count, and the next worker who can edit that file lowers the number
/// in the same commit. That is what just happened, and the guard enforced it —
/// it went RED the moment the bindings were fixed, because the count is an
/// EQUALITY and 0 != 1. A ceiling would have stayed silently green.
const KEYWORD_BINDINGS_IN_FILES_THIS_ROW_MAY_NOT_EDIT: &[(&str, &str, usize)] = &[];

#[test]
fn the_frozen_keyword_list_is_read_from_the_grammar_and_this_test_is_not_vacuous() {
    let keywords = frozen_keywords();
    println!("METRIC sadhana_t1_frozen_keywords {}", keywords.len());

    // A guard whose list came back empty passes over everything and asserts
    // nothing — the exact shape of failure a grammar-derived list invites.
    assert!(
        keywords.len() >= 30,
        "only {} keywords parsed out of spec/grammar-t1.ebnf's `keyword` \
         production ({:?}); the parse is broken and every check below is vacuous",
        keywords.len(),
        keywords
    );

    // The comment stripper is load-bearing: `"textual inclusion"` sits inside
    // the production's own ADR note. If it ever stops running, that phrase
    // arrives as a keyword and this says so by name.
    for w in &keywords {
        assert!(
            w.chars().all(|c| !c.is_ascii_alphabetic()),
            "`{w}` was read out of the `keyword` production as a terminal, but it \
             holds Latin letters — a quoted phrase inside an EBNF comment leaked \
             through strip_ebnf_comments"
        );
    }

    // The two words this ruling moved, pinned in both directions. `समावेशः` is
    // the embed opener (ADR-0019 as ratified) and `आनीतम्` was withdrawn from
    // the list when the owner freed `समावेशः` instead.
    assert!(
        keywords.iter().any(|k| k == "समावेशः"),
        "`समावेशः` is not in the frozen keyword production"
    );
    assert!(
        !keywords.iter().any(|k| k == "आनीतम्"),
        "`आनीतम्` is back in the keyword production; ADR-0019 as ratified uses \
         `समावेशः` for the embed and adds no word"
    );

    // Every declaring keyword this file scans for must be a keyword the grammar
    // still has, or the scan below looks for words that are not there and passes
    // for a spelling reason.
    for d in DECLARING_KEYWORDS {
        assert!(
            keywords.iter().any(|k| k == d),
            "DECLARING_KEYWORDS names `{d}`, which is no longer in the frozen \
             `keyword` production; the binding scan would silently stop seeing \
             every declaration it introduces"
        );
    }
}

/// **The guard.** No `.t1` source in the tree binds a word from the frozen
/// `keyword` production as a name.
///
/// This is the deliverable `B-112` asked for and the one that outlasts the
/// rename it came from: the next collision fails here, by name and by line,
/// instead of waiting for a front end that does not exist to reject a file
/// nobody is reading.
#[test]
fn no_t1_source_binds_a_frozen_keyword_as_a_name() {
    let keywords = frozen_keywords();
    let files = every_t1_source_in_the_tree();
    let root = repo_root();

    assert!(
        files.len() >= 15,
        "only {} .t1 sources found under crates/; this guard has almost nothing \
         to guard",
        files.len()
    );

    let mut reported: Vec<String> = Vec::new();
    let mut allowed_seen: Vec<(String, String, usize)> = Vec::new();
    let mut total = 0usize;

    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
        // A path relative to the repo root, with `/` on every platform, so the
        // allowlist below reads the way a person would write it.
        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .components()
            .map(|c| c.as_os_str().to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("/");

        for (line, word, why) in keyword_bindings(&text, &keywords) {
            total += 1;
            let budget = KEYWORD_BINDINGS_IN_FILES_THIS_ROW_MAY_NOT_EDIT
                .iter()
                .find(|(f, w, _)| *f == rel && *w == word);
            match budget {
                Some(_) => {
                    let slot = allowed_seen
                        .iter_mut()
                        .find(|(f, w, _)| *f == rel && *w == word);
                    match slot {
                        Some((_, _, n)) => *n += 1,
                        None => allowed_seen.push((rel.clone(), word.clone(), 1)),
                    }
                }
                None => reported.push(format!(
                    "{rel}:{line}: `{word}` is a FROZEN KEYWORD of \
                     spec/grammar-t1.ebnf and is {why} here"
                )),
            }
        }
    }

    println!(
        "METRIC sadhana_t1_files_scanned_for_keyword_bindings {}",
        files.len()
    );
    println!("METRIC sadhana_t1_keyword_bindings {total}");
    println!(
        "METRIC sadhana_t1_keyword_bindings_unfixable {}",
        allowed_seen.iter().map(|(_, _, n)| n).sum::<usize>()
    );

    assert!(
        reported.is_empty(),
        "{} .t1 binding(s) use a word the FROZEN `keyword` production of \
         spec/grammar-t1.ebnf reserves. That grammar says a keyword \"is also a \
         possible identifier — the reader must prefer the keyword\", so each of \
         these is already illegal and is unnoticed only because no T1 front end \
         enforces the list yet (B-112). Rename the identifier — the keyword \
         cannot move, the production is frozen and changing it is an ADR:\n  {}",
        reported.len(),
        reported.join("\n  ")
    );

    // The other direction: the known-unfixable set is EXACT. A new binding
    // inside a file that already has one must not hide behind its neighbour,
    // and a binding that gets FIXED must be struck from the list rather than
    // leaving a budget for the next one.
    for (file, word, expected) in KEYWORD_BINDINGS_IN_FILES_THIS_ROW_MAY_NOT_EDIT {
        let found = allowed_seen
            .iter()
            .find(|(f, w, _)| f == file && w == word)
            .map_or(0, |(_, _, n)| *n);
        assert_eq!(
            found, *expected,
            "{file} binds `{word}` {found} time(s); \
             KEYWORD_BINDINGS_IN_FILES_THIS_ROW_MAY_NOT_EDIT records {expected}. \
             If you FIXED one, lower the number here in the same commit. If you \
             ADDED one, do not — `{word}` is a frozen keyword and binding it as \
             a name is already illegal."
        );
    }
}
/// Every quoted terminal anywhere in `spec/grammar-t1.ebnf`, comments removed.
fn grammar_terminals() -> Vec<String> {
    let path = repo_root().join("spec/grammar-t1.ebnf");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("spec/grammar-t1.ebnf must be readable: {e}"));
    strip_ebnf_comments(&src)
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect()
}

/// One row of `spec/directives.tsv`, by its Sassembly word.
fn directive_row(word: &str) -> Option<String> {
    let path = repo_root().join("spec/directives.tsv");
    let src = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("spec/directives.tsv must be readable: {e}"));
    src.lines()
        .find(|l| l.split('\t').next() == Some(word))
        .map(str::to_string)
}

/// The words that carry a T0 DIRECTIVE sense and a T1 sense at once.
///
/// `(word, its .include-style spelling, whether the T1 sense is a FROZEN grammar
/// terminal)`.
///
/// **EVERY FLAG IS NOW `true`, AND THE ONE THAT CHANGED IS THE POINT.** `समम्`
/// carried `false` because its T1 sense is the equality comparison — an
/// OPERATOR — and `spec/grammar-t1.ebnf` said in as many words that operators
/// and their precedence were *"still not frozen"*, so the only place its T1
/// sense could be checked was the `.t1` corpus. ADR-0032 (`D-002h`, 2026-08-30)
/// froze the operators from that same corpus, and this test FAILED ON THAT
/// CHANGE before it landed, by name, which is what the flag was for. The entry
/// is revisited rather than the assertion loosened: `समम्` is now checked the
/// strong way, against the grammar, like the other five.
const WORDS_THAT_SERVE_BOTH_TIERS: &[(&str, &str, bool)] = &[
    ("स्थानम्", ".space", true),    // T1: `pointer_type`
    ("समम्", ".equ", true),        // T1: the equality comparison — ADR-0032
    ("समावेशः", ".include", true), // T1: the embed — ADR-0019, owner-ratified
    ("यदि", ".if", true),         // T1: the `if` keyword
    ("अन्यथा", ".else", true),     // T1: the `else` keyword
    ("अन्तः", ".endif", true),     // T1: the index close
];

/// ADR-0019's shared word, **measured rather than asserted**.
///
/// The draft of ADR-0019 treated `समावेशः` meaning two things — `.include`,
/// *textual inclusion*, in T0, and an EMBED in T1 — as a hazard, and spent a new
/// keyword (`आनीतम्`) avoiding it. The owner ruled the other way on 2026-08-28,
/// and the evidence is in `spec/directives.tsv` itself: of the six directives in
/// that file's `:32`–`:39` span, **five already carried a T1 sense too**, and
/// `समावेशः` was the only one that did not. A rule forbidding this collision
/// would have to unpick `यदि` and `अन्यथा` from the frozen `keyword` production.
///
/// This test exists because that is the load-bearing claim of the ratified ADR,
/// and a claim stated only in prose rots. If someone deletes `.if` from the
/// directive set, or freezes an operator, this says so.
#[test]
fn a_word_that_serves_both_tiers_is_the_rule_and_not_the_exception() {
    let terminals = grammar_terminals();
    let corpus: Vec<String> = every_t1_source_in_the_tree()
        .iter()
        .filter_map(|p| std::fs::read_to_string(p).ok())
        .collect();

    for (word, directive, frozen_in_t1) in WORDS_THAT_SERVE_BOTH_TIERS {
        let row = directive_row(word)
            .unwrap_or_else(|| panic!("`{word}` is no longer a row of spec/directives.tsv"));
        assert!(
            row.contains(directive),
            "spec/directives.tsv no longer spells `{word}` as `{directive}`: {row}"
        );

        if *frozen_in_t1 {
            assert!(
                terminals.iter().any(|t| t == word),
                "`{word}` is a T0 directive but is no longer a terminal of \
                 spec/grammar-t1.ebnf, so it no longer serves both tiers"
            );
        } else {
            // Reachable only if a future entry is added with the flag `false`.
            // No word carries it today (ADR-0032 froze the last one, `समम्`),
            // and the branch is KEPT rather than deleted because the situation
            // it handles — a T1 sense that lives in the corpus and not in the
            // grammar — is exactly what `statement` and `type` still are.
            assert!(
                !terminals.iter().any(|t| t == word),
                "`{word}` is now a frozen terminal of grammar-t1.ebnf, so this \
                 entry should be checked against the grammar instead"
            );
            assert!(
                corpus
                    .iter()
                    .any(|src| t1_code_words(src).iter().any(|(w, _)| w == word)),
                "`{word}` is a T0 directive and its T1 sense is an operator, but \
                 no .t1 source writes it"
            );
        }
    }

    println!(
        "METRIC sadhana_words_serving_both_tiers {}",
        WORDS_THAT_SERVE_BOTH_TIERS.len()
    );

    // Not vacuous: `समावेशः` is in the list and is the one this ruling moved.
    assert!(
        WORDS_THAT_SERVE_BOTH_TIERS
            .iter()
            .any(|(w, _, _)| *w == "समावेशः"),
        "the word ADR-0019 shares is not in the table that justifies sharing it"
    );
}

// ══════════════════════════════════════════════════════════════════════
// `D-002i` — `sanskrit_text.t1`, THE MODULE THAT OWNS `numeral.rs`.
//
// # Why this section is new rather than a raised number
//
// THERE WAS NO RATCHET ON THIS FILE AT ALL. Every other port in this suite has
// one — the encoder, the linker, the IR, the disassembler, the sema, the
// codegen, the T0 reader, the diagnostic path — and `sanskrit_text.t1` had
// none, because when they were written all seven of its routines were stubs and
// a floor of zero asserts nothing. So `D-002i` turning five of them into bodies
// tripped NOTHING, which is a ratchet's own failure mode running in reverse:
// the port could be reverted tomorrow and every test in this crate would stay
// green. That is what happened to `निर्देशकोशपठनम्` and `संज्ञाकुलपठनम्` —
// written, mutation-tested, then SILENTLY REVERTED — which is the incident
// `the_t0_reader_reads_the_two_registry_tables` exists for.
//
// The floor below is set at what `D-002i` reached, and the two stubs that
// remain are named with their blockers, in the shape `T0_READER_BLOCKED` and
// `DIAGNOSTIC_BLOCKED` already use.
//
// # What cleared the blocker on the five that landed
//
// NOTHING EXTERNAL. The file's header called the four numeral readers stubs
// "exactly as the three above are" and named no blocker for them, because there
// was none — what was missing was an oracle that could tell a correct body from
// a wrong one, and `D-002j`'s `nirvahana` is what supplied it. The five bodies
// are RUN in `tests/t1_execution.rs` and every answer is compared to
// `u64::from_str_radix`'s over 241 built tokens, with five mutations that must
// all fail it.
//
// `सङ्ख्या` was not blocked either. It is `validate`, which asks a THIRD
// question and is not `मानदोषः समम् ०`: `ऋण१` is a well-formed numeral that the
// magnitude reader refuses, and a literal too large is well-formed WRITING that
// both value readers refuse.
//
// # This is the TEXT half and `t1_execution.rs` is the BEHAVIOUR half
//
// Neither is redundant. A count of real bodies cannot see a body that computes
// the wrong thing — the 2026-08-29 measurement in `t1_execution.rs`'s head is
// exactly that, one inverted comparison leaving all 34 tests green. And running
// the five cannot see the OTHER two quietly acquiring bodies that no row
// authorised. The two tests below are the second thing; the execution suite is
// the first.
// ══════════════════════════════════════════════════════════════════════

fn sanskrit_text_text() -> String {
    std::fs::read_to_string(crate_src().join("sanskrit_text.t1")).expect("sanskrit_text.t1 exists")
}

/// The stub marker, spelled once more here for the reason `ENCODER_STUB` gives.
const SANSKRIT_TEXT_STUB: &str = "प्रत्यागमनम् उक्तम् अपूर्णम् इति";

/// Every symbol of the Rust original this module is the T1 face of, and the
/// routine that answers it.
///
/// `value` and `bits` each appear TWICE. Rust returns `Result<u64,
/// NumeralError>` and T1 has no tuple, so the fault and the answer are separate
/// questions — the same split `सङ्केतन` made for `split_types` and `व्याकर` for
/// `split_type_class`, and the file's own header gives the reason at length.
const SANSKRIT_TEXT_SYMBOLS: &[(&str, &str)] = &[
    ("numeral::digit_value", "अङ्कमूल्यम्"),
    ("numeral::split_sign: the sign", "ऋणचिह्नम्"),
    ("numeral::split_sign: the rest", "सङ्ख्यारम्भः"),
    ("numeral::classify: the radix", "सङ्ख्यामूलम्"),
    ("numeral::classify: the digits", "सङ्ख्याङ्कारम्भः"),
    ("numeral::is_numeral", "सङ्ख्या"),
    ("numeral::value: whether", "मानदोषः"),
    ("numeral::value: what", "मानम्"),
    ("numeral::bits: whether", "अंशदोषः"),
    ("numeral::bits: what", "अंशाः"),
    ("phonology::is_consonant", "व्यञ्जन"),
    ("segment::aksharas", "अक्षराणि"),
];

/// The ONE that cannot be written, and what it waits on.
///
/// **`व्यञ्जन` WAS THE OTHER, AND IT IS WRITTEN.** It was the fourth routine in
/// this port blocked by a missing name in `anita.rs`'s `TABLES` — that file
/// records `संस्कारकोशः`, `निर्देशकोशः` and `जालकोशः` before it — and one row
/// there cleared it: `("शिवसूत्रकोशः", "shiva-sutras.tsv")`, `anita.rs:197`,
/// added by `D-002i2`. `sanskrit_text.t1` now derives हल् by walking the sūtras
/// through `समावेशः`, and `tests/t1_exec_aksara.rs` runs it against
/// `spec/shiva-sutras.tsv` read independently in Rust: all 33 consonants true,
/// all 9 vowels of अच् false, and ऩ ऱ ळ ऴ — inside U+0915..U+0939 and NOT in
/// हल् — false, which is the assertion a range test could not pass.
///
/// **AND `अक्षराणि` IS WRITTEN, SO THIS LIST IS NOW EMPTY.**
///
/// It was the last stub in the module and it was blocked twice. Both halves are
/// answered, and they were answered by different kinds of thing:
///
///   * HALF (1) WAS DATA. "The UAX #29 tables have no `spec/*.tsv` at all" was
///     true when it was written and is not now: `spec/grapheme-break.tsv`
///     (1386 ranges), `spec/incb.tsv` (473) and
///     `spec/extended-pictographic.tsv` (156) are generated from the same
///     pinned UCD as `tables.rs`, asserted equal to it code point by code point
///     by `sanskrit-text`'s `grapheme_break_spec`, and named in `anita.rs`'s
///     `TABLES` as `अक्षरभेदकोशः`, `संयोगभेदकोशः` and `चित्राक्षरकोशः`.
///   * HALF (2) WAS A DECISION AND THE ROW TOOK IT, under ADR-0031. The
///     declared return `अङ्कः अन्तः अङ्कः अन्तः अ८` was a slice of slices no
///     idiom here can build; `अक्षराणि` now answers a COUNT over
///     `अक्षरारम्भकोश`/`अक्षरदैर्घ्यकोश`. The enumeration ADR-0031 requires is
///     in the routine's margin and names the only two consumers that keep a
///     substring — `parse.rs:376` (`a[i-1] != b[j-1]`, served by
///     `अक्षरसाम्यम्`) and `lex.rs:438` (`clusters[last]` into
///     `Karaka::from_sigil`, served by a start and a length). The precedent is
///     `निदान`'s own `अक्षरविभागः`/`अक्षरसंख्या`/`अक्षरतुल्यम्`.
///
/// The evidence is `tests/t1_exec_aksara.rs`: all **766** cases of the UCD's
/// `GraphemeBreakTest.txt` segment identically, and emptying `spec/incb.tsv`
/// breaks 16 of them while emptying `spec/extended-pictographic.tsv` breaks 3 —
/// so each table is measured to be load-bearing rather than merely embedded.
///
/// Leave it empty. A stub reappearing in this module now fails the count below
/// with no row to hide behind, which is what the list is for.
const SANSKRIT_TEXT_BLOCKED: &[(&str, &str)] = &[];

#[test]
fn the_sanskrit_text_port_declares_every_symbol_of_the_rust_original() {
    let text = sanskrit_text_text();
    let mut missing = Vec::new();
    for (rust, t1) in SANSKRIT_TEXT_SYMBOLS {
        if routine_body(&text, t1).is_none() {
            missing.push(format!("{rust} -> {t1}"));
        }
    }
    println!(
        "METRIC sadhana_t1_sanskrit_text_symbols {}",
        SANSKRIT_TEXT_SYMBOLS.len() - missing.len()
    );
    assert!(
        missing.is_empty(),
        "sanskrit_text.t1 does not declare {} of {} numeral.rs/phonology.rs symbols: {}",
        missing.len(),
        SANSKRIT_TEXT_SYMBOLS.len(),
        missing.join(", ")
    );
}

#[test]
fn the_sanskrit_text_port_reports_how_much_of_it_is_real() {
    let text = sanskrit_text_text();
    let functions = text.matches("सार्वजनिक वृत्तिः").count();
    let stubs = text.matches(SANSKRIT_TEXT_STUB).count();
    let real = functions - stubs;
    println!("METRIC sadhana_t1_sanskrit_text_functions {functions}");
    println!("METRIC sadhana_t1_sanskrit_text_stubs {stubs}");
    println!("METRIC sadhana_t1_sanskrit_text_real {real}");

    // RATCHET, SET AT 13 BY `D-002i` — the first floor this file has ever had,
    // raised from no floor rather than from a number. Before that row the module
    // was seven declarations and SEVEN stubs. The thirteen are:
    //
    //   * the five the row was asked for — `सङ्ख्या`, `मानदोषः`, `मानम्`,
    //     `अंशदोषः`, `अंशाः`;
    //   * the five halves those are built out of, for the same no-tuple reason
    //     the header gives — `अङ्कमूल्यम्`, `ऋणचिह्नम्`, `सङ्ख्यारम्भः`,
    //     `सङ्ख्यामूलम्`, `सङ्ख्याङ्कारम्भः`; and
    //   * three that are this module's own and have no symbol in `numeral.rs` —
    //     `सङ्केताष्टकम्`, `मूलपाठसङ्केतः` and `उपसर्गसाम्यम्`, which are how the
    //     radix prefixes are DERIVED instead of listed as octets. They are not
    //     in `SANSKRIT_TEXT_SYMBOLS` for exactly that reason, which is why that
    //     table has twelve rows and this count is thirteen.
    //
    // Raise it as routines land; it may never fall.
    //
    // RAISED TO 18 FROM 13 BY `D-002i2`, AND WHAT CLEARED THE BLOCKER WAS ONE
    // ROW IN A TABLE THIS FILE DOES NOT OWN. `व्यञ्जन` was stubbed because
    // `spec/shiva-sutras.tsv` — the file `phonology.rs:281`'s
    // `char_set("ह", "ल्")` derives हल् from — had NO NAME in
    // `crates/sadhana/src/t1/anita.rs`'s `TABLES`, so `समावेशः` could not reach
    // it. `("शिवसूत्रकोशः", "shiva-sutras.tsv")` is now `anita.rs:197` and the
    // routine followed the same day.
    //
    // The five are `व्यञ्जन` itself and the four the derivation is built out of,
    // none of which has a symbol in `phonology.rs`:
    //
    //   * `हकारः` and `लकारचिह्नम्` — the two SOUNDS that NAME the pratyāhāra,
    //     `char_set`'s own two arguments, each compared as the UTF-8 octets of
    //     its code point because R-15-1 admits no literal that would spell it;
    //   * `इत्संज्ञा` — Rust's `f[4] == "it"`, the anubandha column, which is
    //     what makes हल् skip the markers it passes rather than stop at them;
    //   * `वर्णोपक्रमः` — the prefix walk, because `is_consonant` asks after
    //     `aksara.chars().next()` and क्ष is three code points whose first is क .
    //
    // THE 33 ARE STILL NOT LISTED ANYWHERE IN THE PORT, and that is the point:
    // `the_sanskrit_text_port_derives_hal_rather_than_listing_it` below refuses
    // a body that names them, and `tests/t1_exec_aksara.rs` grades the routine
    // against `spec/shiva-sutras.tsv` read independently in Rust.
    //
    // RAISED TO 37 FROM 18, AND THE MODULE NOW HAS NO STUB AT ALL. `अक्षराणि`
    // was the last one; writing it took nineteen routines, and they are not
    // padding — UAX #29 needs a UTF-8 decoder (`अष्टकविस्तारः`,
    // `सङ्केतमूल्यम्`), three table readers (`भेदपूरणम्`, `संयोगपूरणम्`,
    // `चित्रपूरणम्`) with the two column readers that tell the UCD's property
    // spellings apart (`भेदनामवर्गः`, `संयोगनामवर्गः`), three binary searches
    // over the filled arenas (`अक्षरभेदः`, `संयोगभेदः`, `चित्राक्षरम्`), a
    // five-field cursor (`अक्षरदर्शकादिः`, `अक्षरदर्शकवर्धनम्`), the rule
    // decision and its break predicate (`विच्छेदनिर्णयः`, `विच्छेदप्रश्नः`),
    // the per-position boundary (`अक्षरसीमा`), `अक्षराणि` itself, and the three
    // that serve the consumers ADR-0031 made this row enumerate — `अक्षरादिः`,
    // `अक्षरमानम्` and `अक्षरसाम्यम्`.
    //
    // Raise it as routines land; it may never fall.
    assert!(
        real >= 37,
        "only {real} of {functions} अक्षरकोश routines have a real body (was 37, \
         raised from 18 by the row that wrote अक्षराणि; before that 18, raised \
         from 13 by D-002i2 when anita.rs's TABLES learned शिवसूत्रकोशः)"
    );

    // And any stub must be one that is blocked, BY NAME. A count alone is
    // satisfied by any stub, so stubbing `मानम्` and writing a hand-listed
    // हल् would hold the number and lose the account. The list is EMPTY now,
    // so this loop is vacuous and the `assert_eq!` below is what carries the
    // rule: no stub may appear in this module without a row naming it.
    for (t1, blocker) in SANSKRIT_TEXT_BLOCKED {
        match routine_body(&text, t1) {
            None => panic!("{t1}: declares no routine"),
            Some(b) if !b.contains(SANSKRIT_TEXT_STUB) => {
                panic!("{t1}: has a body; name the row that cleared its blocker — {blocker}")
            }
            Some(_) => {}
        }
    }
    assert_eq!(
        stubs,
        SANSKRIT_TEXT_BLOCKED.len(),
        "{stubs} routines are stubs and only {} are blocked; a stub here that is \
         not अक्षराणि is a routine that had a body and lost it",
        SANSKRIT_TEXT_BLOCKED.len()
    );
}

/// **हल् must be DERIVED from the sūtras and never listed.**
///
/// This is the ratchet that makes the raise above mean something. `व्यञ्जन`
/// would be trivial to write as 33 comparisons, and it would pass every
/// behavioural test in `tests/t1_exec_aksara.rs` on the day it was written — a
/// second transcription of an AUTHORED table that agrees with the original
/// exactly until someone edits one of them. `spec/shiva-sutras.tsv` says in its
/// own head that it "cannot be derived from anything: no oracle emits it and no
/// measurement produces it", which is precisely why a copy of it inside a port
/// is unrecoverable rather than merely redundant.
///
/// So: the routine must reach the file, and it must not spell the set out.
#[test]
fn the_sanskrit_text_port_derives_hal_rather_than_listing_it() {
    let text = sanskrit_text_text();
    // Comments stripped, for the reason `the_sanskrit_text_port_derives_the_
    // radix_prefixes_…` gives below: this file's head names the consonants that
    // are NOT in हल् (ऩ ऱ ळ ऴ) while explaining why a range test fails, and a
    // bare `contains` over the whole text would read prose as code.
    let code: String = text
        .lines()
        .map(|l| l.split('॰').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        code.contains("पदविभागॱसमावेशपाठः उक्तम् शिवसूत्रकोशः इति"),
        "व्यञ्जन must reach spec/shiva-sutras.tsv through the store; without \
         that line there is nothing to derive हल् FROM. The spelling changed on \
         2026-09-14 from ADR-0019's embed to a run-time lookup by name, because \
         the embed resolves when the READING source is compiled and so was empty \
         in every compiled compiler"
    );

    // THE 33, AS A LIST. Every consonant of हल् appearing as a Devanagari
    // literal in the CODE would be the hand-copied table. Two or three are
    // unavoidable in identifiers — व्यञ्जन itself is spelled with them — so
    // this counts how many of the 33 appear as a standalone token, which is
    // what a comparison against a literal would look like.
    let hal = [
        "ह", "य", "व", "र", "ल", "ञ", "म", "ङ", "ण", "न", "झ", "भ", "घ", "ढ", "ध", "ज", "ब", "ग",
        "ड", "द", "ख", "फ", "छ", "ठ", "थ", "च", "ट", "त", "क", "प", "श", "ष", "स",
    ];
    let listed: Vec<&str> = hal
        .iter()
        .filter(|c| code.split_whitespace().any(|t| t == **c))
        .copied()
        .collect();
    assert!(
        listed.is_empty(),
        "{} of the 33 consonants are written as standalone tokens in \
         sanskrit_text.t1 ({listed:?}) — हल् is being LISTED, not derived; the \
         set belongs to spec/shiva-sutras.tsv and nowhere else",
        listed.len()
    );
}

/// The port must not become a SECOND octet transcription of `numeral.rs`.
///
/// `vakyavibhaga.t1` carries the same reader and spends 45 lines writing the
/// three radix prefixes out as literal octets into three globals an initialiser
/// has to remember to fill. This module DERIVES them — five code points per
/// prefix, and `सङ्केताष्टकम्` computes the UTF-8 — so there is no octet table
/// here to drift and no initialiser to forget. Asserted the way a ratchet is:
/// the distinguishing code points must be present, the UTF-8 arithmetic must be
/// present, and no prefix arena may be filled octet by octet.
#[test]
fn the_sanskrit_text_port_derives_the_radix_prefixes_rather_than_listing_them() {
    let text = sanskrit_text_text();
    // Comments stripped, for the reason `diagnostic_code_only` gives: the head
    // of that file discusses the prefixes at length and a bare `contains` would
    // read the prose as the code.
    let code: String = text
        .lines()
        .map(|l| l.split('॰').next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");

    // U+0966 ० — the code point all three prefixes open with.
    assert!(
        code.contains("२४०६"),
        "the prefixes' shared first code point (U+0966 = २४०६) is not in the code"
    );
    // And the three that tell ०द्वि, ०अष्ट and ०षोड् apart at their SECOND
    // akṣara: द U+0926, अ U+0905, ष U+0937. A reader that stopped before this
    // one would classify every `०…` as the same radix.
    for (word, cp) in [
        ("द U+0926", "२३४२"),
        ("अ U+0905", "२३०९"),
        ("ष U+0937", "२३५९"),
    ] {
        assert!(
            code.contains(cp),
            "{word} ({cp}) is not in the code; the three prefixes cannot be told apart"
        );
    }

    // The derivation is real: १२८ is the UTF-8 continuation byte and ६३ its
    // payload mask, and only a routine that COMPUTES octets has a use for
    // either. A file that listed them would carry neither.
    assert!(
        code.contains("१२८") && code.contains("६३"),
        "no UTF-8 arithmetic (१२८, ६३) in the code — the octets are being listed"
    );

    // NOT VACUOUS, and this is the half that fails if the port is ever
    // rewritten the way वाक्यविभाग writes it: filling a prefix arena takes one
    // indexed store per octet, and forty-five of those is what this refuses.
    let stores = code.matches("मूलपाठः अङ्कः").count();
    assert_eq!(
        stores, 0,
        "{stores} indexed store(s) into a radix-prefix arena; this module derives \
         its prefixes and वाक्यविभाग is the one that lists them"
    );
}

/// TWENTY-FIVE PUBLIC NAMES ARE DECLARED IN MORE THAN ONE MODULE, and the
/// interpreter cannot tell them apart.
///
/// `Interpreter::global` keys the BARE name, never the module-qualified one. So
/// loading two modules that both export `अङ्कमूल्यम्` leaves one shadowing the
/// other, and the loser's callers get a routine with the wrong arity.
///
/// # This is not theoretical — it is what stops the chain at typecheck
///
/// `measure_corpus_ir` loads `मध्यरूप` beside `अक्षरकोश` because `ir.t1` needs
/// it, and TWELVE of fifteen files then fail to PARSE with
/// `sanskrit_text.t1:21: सङ्ख्या: अङ्कमूल्यम् …` — `अक्षरकोश`'s two-argument
/// `अङ्कमूल्यम्` (`sanskrit_text.t1:1282`) resolving to `मध्यरूप`'s
/// one-argument one (`ir.t1:291`). The same corpus parses 15/15 under
/// `measure_corpus_resolve`, whose loader happens not to include `ir.t1`.
///
/// So lex/parse/resolve/typecheck reading 15/15 is true *of the loaders those
/// censuses use*, and IR construction is the first stage that must load two
/// colliding modules at once. Any real compiler loads all fifteen.
///
/// # A ratchet, not a target
///
/// The count may only fall. The fix is either renaming across the corpus or
/// teaching the interpreter to key module-qualified names — a runtime change,
/// and the language already has the modules to key by. Until then this stops
/// number twenty-six from arriving unnoticed.
#[test]
fn public_names_are_not_declared_in_two_modules() {
    use std::collections::BTreeMap;
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut owner: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .expect("corpus is readable")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    files.sort();
    for path in &files {
        let stem = path.file_name().unwrap().to_string_lossy().into_owned();
        let text = std::fs::read_to_string(path).expect("source is readable");
        for line in text.lines() {
            let code = line.split('॰').next().unwrap_or("").trim();
            let mut w = code.split(' ');
            if w.next() != Some("सार्वजनिक") {
                continue;
            }
            // `वृत्तिः`/`चरः`/`संरचना`/`गणना` then the name.
            let kind = w.next().unwrap_or("");
            if !matches!(kind, "वृत्तिः" | "चरः" | "संरचना" | "गणना")
            {
                continue;
            }
            if let Some(name) = w.next().filter(|n| !n.is_empty() && *n != "ॱॱ") {
                let e = owner.entry(name.to_string()).or_default();
                if !e.contains(&stem) {
                    e.push(stem.clone());
                }
            }
        }
    }
    let clashes: Vec<_> = owner.iter().filter(|(_, m)| m.len() > 1).collect();
    let shown: Vec<String> = clashes
        .iter()
        .take(6)
        .map(|(n, m)| format!("{n} in {}", m.join(", ")))
        .collect();
    assert!(
        clashes.len() <= 25,
        "public names declared in two or more modules rose to {} (was 25). \
         `Interpreter::global` keys the BARE name, so a new one silently \
         shadows an existing routine and its callers get the wrong arity. \
         First few: {shown:?}",
        clashes.len()
    );
}

// ─────────────────────────────────────────────────────────────────────────
// The unparser — `W-215`, `crates/sadhana/src/t1/unparse.rs` and its T1 twin
// `unparse.t1` (`मुद्रण`). The inverse of the parser over the same AST
// (research/22 §4.4 Rule X4).
//
// THE TWINS DIVERGE IN WHAT THEY CAN SPELL, and the table says where: Rust's
// `ast.rs` keeps `Declaration::Module` but drops a struct's fields
// (`TypeDecl { name, is_struct }`), so `unparse` refuses a type declaration;
// `व्याकर` keeps the module name as parser state (`मण्डलनामसूचकाङ्क`) and,
// since this row, a struct's fields and an enum's variant run, so `मुद्रण`
// prints them. Both refuse a node they cannot spell by name.
// ─────────────────────────────────────────────────────────────────────────

/// Every symbol `crates/sadhana/src/t1/unparse.rs` defines, in the spelling
/// `unparse.t1` gives it.
const UNPARSER_SYMBOLS: &[(&str, &str)] = &[
    ("unparse", "स्रोतलेखनम्"),
    ("unparse_declaration", "घोषणालेखनम्"),
    ("unparse_statement", "वाक्यलेखनम्"),
    ("unparse_expression", "अभिव्यञ्जकलेखनम्"),
    ("unparse_type", "प्रकारलेखनम्"),
    ("enum Unprintable", "अमुद्रणीयकारणम्"),
];

fn unparse_text() -> String {
    std::fs::read_to_string(crate_src().join("unparse.t1")).expect("unparse.t1 exists")
}

#[test]
fn the_unparser_declares_every_symbol_of_its_rust_twin() {
    let text = unparse_text();
    let missing: Vec<String> = UNPARSER_SYMBOLS
        .iter()
        .filter(|(_, t1)| {
            !text.contains(&format!("वृत्तिः {t1}")) && !text.contains(&format!("चरः {t1}"))
        })
        .map(|(rust, _)| (*rust).to_string())
        .collect();
    println!(
        "METRIC sadhana_t1_unparser_symbols {}",
        UNPARSER_SYMBOLS.len() - missing.len()
    );
    assert!(
        missing.is_empty(),
        "unparse.t1 does not declare {} of {} t1/unparse.rs symbols: {}",
        missing.len(),
        UNPARSER_SYMBOLS.len(),
        missing.join(", ")
    );
}

// ─────────────────────────────────────────────────────────────────────────
// `W-255` — THE RESERVED WORDS, ASKED OF THE PARSER RATHER THAN LISTED.
// ─────────────────────────────────────────────────────────────────────────

/// The candidate set: every quoted terminal of the grammar that is written in
/// Devanagari, deduplicated and sorted.
///
/// Built on the [`grammar_terminals`] this file already had — which reads every
/// terminal ANYWHERE in `spec/grammar-t1.ebnf`, not the `keyword` production
/// alone. The wider reader was already here; nothing was asking it about names.
fn devanagari_terminals() -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for t in grammar_terminals() {
        if t.chars().any(|c| ('\u{0900}'..='\u{097f}').contains(&c))
            && !t.chars().any(|c| c.is_ascii_alphabetic())
            && !out.contains(&t)
        {
            out.push(t);
        }
    }
    out.sort();
    out
}

/// The parser, loaded once: the lexer, the AST, the parser and its text helper.
fn reserved_word_parser() -> sadhana::t1::nirvahana::Interpreter {
    let names = ["lex.t1", "ast.t1", "parse.t1", "sanskrit_text.t1"];
    let texts: Vec<(String, String)> = names
        .iter()
        .map(|n| {
            let p = crate_src().join(n);
            (
                (*n).to_string(),
                std::fs::read_to_string(&p)
                    .unwrap_or_else(|e| panic!("{} is readable: {e}", p.display())),
            )
        })
        .collect();
    let refs: Vec<(&str, &str)> = texts
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    sadhana::t1::nirvahana::Interpreter::load(&refs, &spec_root())
        .unwrap_or_else(|e| panic!("the T1 parser loads: {e:?}"))
}

/// Whether `व्याकर` reads `program` with no refusal — the T1 parser's own
/// verdict, which is the one the chain acts on.
fn t1_parses(it: &mut sadhana::t1::nirvahana::Interpreter, program: &str) -> bool {
    use sadhana::t1::nirvahana::{Octets, Value};
    let Ok(toks) = it.call(
        "पदविभागॱपदविभाग",
        vec![Value::Octets(Octets::new(program.as_bytes()))],
        2_000_000_000,
    ) else {
        return false;
    };
    match it.call("व्याकरॱकार्यक्रमपठनम्", vec![toks], 4_000_000_000)
    {
        Ok(v) => {
            v.as_int().unwrap_or(0) > 0 && it.global("दोषसूचकाङ्क").and_then(Value::as_int) == Some(0)
        }
        Err(_) => false,
    }
}

/// THE RESERVED SET, DERIVED FROM THE PARSER. Every grammar terminal that
/// `व्याकर` will NOT accept as a bound name — asked, not asserted.
///
/// # Why this is asked and not written down
///
/// A hand-kept list is a second reference and it drifts from the first; that
/// drift IS the defect this guards. Reading the `keyword` production was a
/// first attempt at deriving it, and `W-255` measured what that derivation
/// actually catches: of the 32 words in that production the parser accepts 31
/// as a local name, and of the seven words it REFUSES, six are not in the
/// production at all. The old list was not merely narrower than the reserved
/// set — it was very nearly disjoint from it.
///
/// So the question is put to the parser instead, in the two positions a name
/// is bound in: a local (`चरः न ॱॱ न६४`) and a parameter (`आदाय न ॱॱ न६४`).
/// A word refused in either is reserved. The set then tracks the grammar AND
/// the parser with nothing in between to go stale.
///
/// It costs one interpreter image and about three seconds for the whole
/// grammar, which is why this is a plain `#[test]` and not an `#[ignore]`d one.
fn reserved_by_the_parser(it: &mut sadhana::t1::nirvahana::Interpreter) -> Vec<String> {
    devanagari_terminals()
        .into_iter()
        .filter(|w| {
            let local = format!(
                "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः {w} ॱॱ न६४ भवति ० ।\n    प्रत्यागमनम् {w} ।\nइति\n"
            );
            let param = format!(
                "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग आदाय {w} ॱॱ न६४ ददाति न६४ आदि\n    प्रत्यागमनम् {w} ।\nइति\n"
            );
            !t1_parses(it, &local) || !t1_parses(it, &param)
        })
        .collect()
}

/// The reserved set is what the row said it was, and the gap it names is real.
///
/// This is the measurement `W-255` was opened for, kept as an assertion so the
/// next person does not have to take it on trust.
#[test]
fn the_reserved_set_is_asked_of_the_parser_and_the_keyword_production_missed_it() {
    let mut it = reserved_word_parser();
    let reserved = reserved_by_the_parser(&mut it);
    let keywords = frozen_keywords();
    println!(
        "METRIC sadhana_t1_devanagari_terminals {}",
        devanagari_terminals().len()
    );
    println!(
        "METRIC sadhana_t1_reserved_by_the_parser {}",
        reserved.len()
    );
    for w in &reserved {
        println!("  reserved: {w}");
    }

    assert!(
        !reserved.is_empty(),
        "the parser accepted every grammar terminal as a name; either the probe \
         no longer parses anything or the image is wrong, and the guard below \
         would pass over everything"
    );

    // THE WORD THAT DREW BLOOD. A peer bound `अङ्कः` — the index opener,
    // `spec/grammar-t1.ebnf`'s `index`, `slice_type` and `array_type` all open
    // with it — the old guard passed it, and a source stopped parsing.
    assert!(
        reserved.iter().any(|w| w == "अङ्कः"),
        "`अङ्कः` is not in the parser-derived reserved set; it opens three \
         productions and a binding of it does not parse, so either the probe \
         has stopped exercising the parser or the parser has changed"
    );
    assert!(
        !keywords.iter().any(|k| k == "अङ्कः"),
        "`अङ्कः` is now in the `keyword` production. The gap this test records \
         has closed at the grammar; say so here rather than leaving a test that \
         claims a hole that no longer exists"
    );

    // And the other direction, which is the half that keeps the set honest: a
    // word the parser DOES accept as a name is not reserved, however keyword-ish
    // it looks. `पाठ` is the text type and 36 bindings in the corpus use it.
    assert!(
        !reserved.iter().any(|w| w == "पाठ"),
        "`पाठ` is reserved by this measurement, and 36 bindings in the corpus \
         name it; the probe is refusing programs for some other reason"
    );
}

/// **The guard `W-255` adds.** No `.t1` source binds a word the parser will not
/// accept as a name.
///
/// The older guard beside this one reads the `keyword` production and stays —
/// it carries ADR facts and a style rule — but it is not what stops a source
/// from parsing, and this is.
#[test]
fn no_t1_source_binds_a_word_the_parser_reserves() {
    let mut it = reserved_word_parser();
    let reserved = reserved_by_the_parser(&mut it);
    let files = every_t1_source_in_the_tree();
    assert!(
        files.len() >= 15,
        "only {} .t1 sources found; this guard has almost nothing to guard",
        files.len()
    );

    let mut found: Vec<String> = Vec::new();
    for path in &files {
        let text = std::fs::read_to_string(path)
            .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
        for (line, word, why) in keyword_bindings(&text, &reserved) {
            found.push(format!(
                "{}:{line}: `{word}` is RESERVED — the parser refuses it as a \
                 name — and is {why} here",
                path.display()
            ));
        }
    }
    assert!(
        found.is_empty(),
        "{} binding(s) of a word the parser reserves:\n  {}",
        found.len(),
        found.join("\n  ")
    );
}

/// THE FAIL-FIRST CONTROL: the case that passed before this row and refuses now.
///
/// `अङ्कः` bound as a local, scanned with the OLD list (the `keyword`
/// production) and with the NEW one (the parser's own refusals). The old list
/// finds nothing; the new one names the line. Without this the widening could
/// be a no-op and every test above would still be green.
///
/// # What actually breaks, measured here rather than assumed
///
/// The BINDING alone parses. `चरः अङ्कः ॱॱ न६४ भवति ० ।` is read without
/// complaint, because after `चरः` the parser is taking a name and takes this
/// one. It is the READ that fails: `प्रत्यागमनम् अङ्कः ।` puts the word where a
/// value belongs, the parser starts an index expression and then wants the
/// `अन्तः` that closes it. So the word is not refused where it is written down
/// — it is refused everywhere it is used afterwards, which is why the author
/// spends the hour on the wrong line. The guard is on the binding because that
/// is the one place the name appears once and can be renamed.
#[test]
fn the_binding_that_broke_a_source_passes_the_old_list_and_fails_the_new_one() {
    let binding_only = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः अङ्कः ॱॱ न६४ भवति ० ।\n    प्रत्यागमनम् ० ।\nइति\n";
    let binding_and_use = "मण्डलम् क ॥\nसार्वजनिक वृत्तिः ग ददाति न६४ आदि\n    चरः अङ्कः ॱॱ न६४ भवति ० ।\n    प्रत्यागमनम् अङ्कः ।\nइति\n";

    let old = keyword_bindings(binding_and_use, &frozen_keywords());
    assert!(
        old.is_empty(),
        "the `keyword` production now catches `अङ्कः`; this control records that \
         it did not, and the row it belongs to is finished: {old:?}"
    );

    let mut it = reserved_word_parser();
    let new = keyword_bindings(binding_and_use, &reserved_by_the_parser(&mut it));
    assert_eq!(
        new.len(),
        1,
        "the new guard names exactly the one binding: {new:?}"
    );
    assert_eq!(new[0].0, 3, "and names its line");
    assert_eq!(new[0].1, "अङ्कः");

    // The two halves of the mechanism, both measured: the binding is read, the
    // use is not.
    assert!(
        t1_parses(&mut it, binding_only),
        "`चरः अङ्कः ॱॱ न६४` alone stopped parsing; the refusal has moved to the \
         binding and the comment above is now wrong"
    );
    assert!(
        !t1_parses(&mut it, binding_and_use),
        "reading `अङ्कः` back parses after all; the reserved set would then be \
         guarding something that costs nothing"
    );
}

// ── the Rust emitter's population, discovered rather than listed ───────────

/// Rust items `riscv64.rs` declares that the port deliberately does NOT carry,
/// each with the reason.
///
/// EXACT, not a ceiling — the whole finding is that an unlisted symbol is an
/// unchecked one, so a `<=` here would rebuild the hole one level up.
const RUST_ONLY: &[(&str, &str)] = &[
    // `W-285` — THE JUSTIFICATION THESE THREE CARRIED WAS MEASURED FALSE, AND
    // THE ENTRIES ARE RE-POINTED RATHER THAN DELETED. The pairing guard was
    // right to demand a pair; it was the REASON that was wrong, and a deleted
    // entry cannot be told from one that was never owed.
    //
    // WHAT IT SAID, AND WHY IT WAS WRONG: it claimed the two halves reaching
    // the bool differently was "the point and not a defect" — this side SCANS
    // a finished module, the port RECORDED that it emitted one — and that
    // "these two disagreeing is exactly what the twin octet test would catch".
    //
    // THEY DID NOT DISAGREE. Both asked whether THIS MODULE allocates, and the
    // startup is linked into an IMAGE of several module objects. A module that
    // allocates nothing, linked beside one that does, emitted no region and
    // left the other's reference unresolved: `sanskrit_text`, `shrinkhala` and
    // `unparse` failed to link on `e9bd47ec`, each naming `रचनाक्षेत्रम्` and
    // `रचनासूचकः`. **The twin AGREED on all three** — 456,527 · 93,802 ·
    // 234,376 octets — because agreeing is what two halves do when they share a
    // scope error.
    //
    //   A TWIN TEST CANNOT CATCH AN ERROR BOTH TWINS MAKE. Twins check ROUTE;
    //   a scope error is invisible to every twin comparison by construction.
    //   The margin asserting otherwise is why nobody looked.
    //
    // THE REMEDY WAS AN ARGUMENT, NOT A BETTER PREDICATE. No emitter can derive
    // what will be linked beside it, so whoever assembles the image must pass
    // it in. `यन्त्रारम्भोत्सर्जनम्` now takes `रचनामस्ति ॱॱ बूल` exactly as
    // `emit_startup_with_records` takes `records`, and the port's
    // `यन्त्ररचनामस्ति` is a PER-MODULE fact the assembler ORs across the
    // modules it links — a scope anyone can state, which is what it lacked.
    (
        "emit_startup_with_records",
        "a Rust-only WRAPPER, and only that: since `W-285` the port's \
         यन्त्रारम्भोत्सर्जनम् takes the same flag as a parameter. This side \
         needs two wrappers because `emit_startup` has callers that must keep \
         their old signatures; the port changed its one signature instead",
    ),
    (
        "emit_module_and_relaxations",
        "`W-332`'s census entry point. It is emit_module's OWN body with the \
         relaxed-routine labels still attached, and emit_module is now the \
         wrapper that drops them — so this is a wrapper count that differs, \
         exactly as emit_startup_object_with_records below is, and not a second \
         emitter. The port's यन्त्रमण्डलोत्सर्जनम् needs no variant because it \
         has no census to return: its relaxation is STATE in the routine \
         emitter (`W-306`) rather than an `Err` value, so a `.t1` counter is a \
         global on that side and not a widened result. NOT WRITTEN THIS CYCLE \
         and the asymmetry is deliberate: `W-332`'s acceptance is a count of \
         what the RUST emitter did to the corpus, and `t1_twin_agreement` \
         already pins that the two halves emit the same octets, so a second \
         census would re-measure agreement rather than add a reading",
    ),
    (
        "emit_startup_object_with_records",
        "same — and the port needs no VARIANT because since `W-285` its \
         यन्त्रारम्भमण्डलोत्सर्जनम् takes the flag directly and passes it \
         through. It is a wrapper count that differs, not the interface",
    ),
    (
        "module_allocates",
        "this side SCANS a module; the port's यन्त्ररचनाप्रश्नः reports what its \
         emitter RECORDED. NOT \"the same answer by the other route\" — that \
         claim was measured false in `W-285`: both answer about ONE MODULE, and \
         the startup is linked into an IMAGE, so three sources that allocate \
         nothing failed to link beside one that does. Each CALLER must OR it \
         across the objects it is about to link; neither predicate can",
    ),
    (
        "ZERO",
        "a register NAME; the port writes `शून्यः` inline where Rust names a const",
    ),
    // The two symbol ids ir.t1 addresses the record region through (run growth
    // and slices, 2026-09-13): declared in ir.t1 as `रचनासूचकसंज्ञा` /
    // `रचनाक्षेत्रसंज्ञा`; the port's module_allocates twin READS them as
    // `मध्यरूपॱ…` and declares nothing — the Rust consts twin ir.t1, not the port.
    (
        "RECORD_CURSOR_SYMBOL",
        "ir.t1's `रचनासूचकसंज्ञा`; the port reads `मध्यरूपॱरचनासूचकसंज्ञा`",
    ),
    (
        "RECORD_REGION_SYMBOL",
        "ir.t1's `रचनाक्षेत्रसंज्ञा`; the port reads `मध्यरूपॱरचनाक्षेत्रसंज्ञा`",
    ),
    ("RA", "a register NAME; the port writes `पुनःस्थानम्` inline"),
    (
        "FLOAT_MOVE",
        "`V-005`: a mnemonic WORD, `fsgnj.d`'s; the port writes `प्लवचिह्नारोपणम्` inline, as it writes `योगः` and `निधानम्`",
    ),
    ("SP", "a register NAME; the port writes `स्तूपसूचकः` inline"),
    (
        "VECTOR_SET_LENGTH",
        "`V-008` part 2: a mnemonic WORD, `vsetvli`'s; the port writes it inline, as it writes `योगः`",
    ),
    (
        "VECTOR_REGISTER_STEM",
        "`V-008` part 2: the vector register stem; the port writes it inline in its `vector_register` twin",
    ),
    (
        "VECTOR_TYPE_E64_M8",
        "`V-008` part 2: vtype 219 as a numeral; the port writes `२१९` inline",
    ),
    (
        "MATRIX_KERNEL",
        "`V-009` (ii): the kernel's readable rows, test-only; the compact table encodes them",
    ),
    (
        "MATRIX_KERNEL_TABLE",
        "`V-009` (ii): the compact table; the port writes the same letters as a literal in `यन्त्राव्यूहकायोत्सर्जनम्`",
    ),
    (
        "MATRIX_KERNEL_OPERANDS",
        "`V-009` (ii): the operand list; the port writes the same letters as a literal in `यन्त्राव्यूहपदसङ्केतः`",
    ),
    (
        "matrix_kernel_rows",
        "`V-009` (ii): the table decoded into rows; the port decodes each row inside `यन्त्राव्यूहकायोत्सर्जनम्`'s loop",
    ),
    (
        "MATRIX_PRODUCT_SYMBOL",
        "`V-009` (ii): the product kernel's symbol; the port reads `ir.t1`'s `मध्यरूपॱआव्यूहवृत्तिसंज्ञा`",
    ),
    (
        "MATRIX_TRANSPOSE_SYMBOL",
        "`V-009` (ii): the transpose kernel's symbol; the port reads `ir.t1`'s `मध्यरूपॱव्युत्क्रमवृत्तिसंज्ञा`",
    ),
    (
        "VECTOR_LABEL_INFIX",
        "`V-008` part 2: the expansion's label infix; the port writes `खण्ड` inline, and only Rust's branch-range census reads it",
    ),
    (
        "FINISHER_HI",
        "the finisher's high half as a Devanagari literal; written inline by यन्त्रप्रवेशोत्सर्जनम्",
    ),
    (
        "Refusal",
        "the enum ITSELF has no T1 type — its ten VARIANTS are mirrored as \
         `यन्त्र…निषेधभेद` constants and each is paired in RISCV_SYMBOLS",
    ),
    (
        "Module",
        "the IR module as `riscv64` receives it; the port reads मध्यरूप's arenas \
         directly and never materialises the struct",
    ),
    (
        "Routine",
        "same — a Rust-side view over arenas the port walks in place",
    ),
    (
        "Frame",
        "carried as `यन्त्रचौकट` and checked FIELD BY FIELD in RISCV_FIELDS \
         rather than by name here",
    ),
    (
        "Names",
        "a type ALIAS for `HashMap<SymbolId, (String, String)>`, so its \
         `new`/`insert`/`get` are the map's rather than the emitter's — the \
         three `Names::` rows pair the PORT's routines against those \
         operations, and the alias itself has nothing to carry",
    ), // `W-381` — first seen when the reader learned `const fn` (it had read
    // `pub const fn fail_word` as a `const` named `fn`).
    (
        "fail_word",
        "its twin is NOT in this port: the FAIL-form word `code << 16 | 0x3333` \
         is built by ir.t1's `समापकविफलशब्दः` (module मध्यरूप), because both \
         of ir.t1's refusal stores take it from the IR; the port's one other \
         use, the vector refusal, CALLS that routine as `मध्यरूपॱसमापकविफलशब्दः`. A \
         यन्त्र-prefixed copy here would be a second statement of the word. \
         Pinned by the_fail_word_twin_is_ir_t1s_routine_and_the_port_calls_it",
    ),
];

/// Every module-level NAMED DECLARATION `riscv64.rs` makes — `fn`, `struct`,
/// `const`, `enum`, `type`, `static` — as `(kind, name)`, at column zero so a
/// function nested inside another is not mistaken for port surface.
///
/// THIS IS THE AUTHORITATIVE POPULATION for the guard below, and its scope is
/// stated rather than implied: **`impl` blocks and `mod` are module-level and
/// are NOT discovered.** `riscv64.rs` has three `impl`s and one `mod tests`.
/// The test module is not port surface; the `impl`s hold the methods that
/// appear in `RISCV_SYMBOLS` as QUALIFIED rows — `Routine::…`, `Module::…` —
/// which this guard does not cover and which the commit records as residue.
///
/// An earlier draft of this margin said "every module-level item", which
/// overclaimed by four in a guard whose subject is overclaiming.
fn riscv_rust_items() -> Vec<(String, String)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../crates/sadhana/src/t1/riscv64.rs");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{} must be readable: {e}", path.display()));
    text.lines().filter_map(riscv_rust_item).collect()
}

/// One column-zero line of `riscv64.rs` as `(kind, name)`, or `None`.
fn riscv_rust_item(line: &str) -> Option<(String, String)> {
    // Column zero only, and `pub` is not required: the port covers private
    // helpers too — `temp` and `arg` are both in RISCV_SYMBOLS and neither
    // is `pub`.
    if line.starts_with(' ') || line.starts_with('\t') {
        return None;
    }
    // `W-381` — A RESTRICTED `pub` IS STILL AN ITEM. Only `pub ` was stripped,
    // so `pub(crate) fn x` matched no kind and was SKIPPED, the silent miss
    // this guard exists to refuse. None exists in riscv64.rs today (grep,
    // 2026-10-06); the form is read so the first one is seen.
    let rest = match line.strip_prefix("pub(") {
        Some(after) => after.split_once(") ").map_or(line, |(_, r)| r),
        None => line.strip_prefix("pub ").unwrap_or(line),
    };
    // `static ` ADDED BY `W-332`, AND IT WAS A REAL BLIND SPOT. The kind
    // list was the population this guard can see, and a `static` at column
    // zero was outside it — so the emitter could grow process state with no
    // twin and nothing would say so. `W-264`'s own margin predicted this
    // shape ("a guard cannot find its own blind spot") and the miss was
    // found the way it says such things are found: by needing to add a
    // `static` and asking whether the guard would notice. It discovered
    // ZERO existing items when added, so this widened the question without
    // moving any answer.
    //
    // THE ONE STATIC IT EVER CAUGHT HAS SINCE BEEN REMOVED, and the kind
    // stays. `RELAXED_ROUTINES` was a process-global relaxation counter;
    // `emit_module_and_relaxations` returns the labels per call instead, so
    // there is no emitter static left for this arm to find. The arm is kept
    // because its subject is the POPULATION the guard can see, not any
    // member of it: removing it would restore the blind spot the moment the
    // next `static` is written, and a guard narrowed because its one catch
    // was fixed is a guard that only ever fires once.
    // `W-381` — A QUALIFIED `fn` IS A `fn`. `pub const fn fail_word` was read
    // as a `const` named `fn`: the item was still reported (the gate went red
    // on it), but under the wrong kind and name, so no RISCV_SYMBOLS or
    // RUST_ONLY row could ever answer it. The qualifiers a module-level `fn`
    // can carry are stripped first, so the name read is the function's.
    let mut rest = rest;
    for qualifier in ["const ", "unsafe ", "async ", "extern \"C\" "] {
        if let Some(after) = rest.strip_prefix(qualifier)
            && (after.starts_with("fn ")
                || after.starts_with("unsafe fn ")
                || after.starts_with("extern \"C\" fn "))
        {
            rest = after;
        }
    }
    for kind in ["fn ", "struct ", "const ", "enum ", "type ", "static "] {
        let Some(after) = rest.strip_prefix(kind) else {
            continue;
        };
        let name: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        return (!name.is_empty()).then(|| (kind.trim().to_string(), name));
    }
    None
}

/// `W-381` — the declaration reader names a QUALIFIED `fn` by its own name:
/// `pub const fn fail_word` is the `fn` `fail_word`, never a `const` named `fn`.
#[test]
fn the_riscv_item_reader_names_a_const_fn_by_its_name() {
    let item = |l: &str| riscv_rust_item(l).map(|(k, n)| format!("{k} {n}"));
    assert_eq!(
        item("pub const fn fail_word(code: u64) -> u64 {").as_deref(),
        Some("fn fail_word")
    );
    assert_eq!(item("const fn helper() {}").as_deref(), Some("fn helper"));
    assert_eq!(item("pub unsafe fn raw() {}").as_deref(), Some("fn raw"));
    assert_eq!(
        item("pub const FINISHER_HI: &str = \"\";").as_deref(),
        Some("const FINISHER_HI")
    );
    assert_eq!(
        item("pub fn emit_module(m: &Module) {").as_deref(),
        Some("fn emit_module")
    );
    assert_eq!(
        item("pub(crate) fn scoped() {}").as_deref(),
        Some("fn scoped")
    );
    assert_eq!(
        item("pub(super) const fn up() {}").as_deref(),
        Some("fn up")
    );
    assert_eq!(item("async fn later() {}").as_deref(), Some("fn later"));
    assert_eq!(
        item("extern \"C\" fn c_abi() {}").as_deref(),
        Some("fn c_abi")
    );
    // A method is not a module-level item: `impl` blocks are indented, and the
    // `Routine::…`/`Frame::…` rows stay hand-listed (the margin above).
    assert_eq!(item("    const fn indented() {}"), None);
    assert_eq!(item("impl Frame {"), None);
}

/// `W-381` — `fail_word`'s RUST_ONLY reason, checked rather than asserted: ir.t1
/// declares the routine, the port calls it, and the arithmetic is the FAIL form.
#[test]
fn the_fail_word_twin_is_ir_t1s_routine_and_the_port_calls_it() {
    let src = |f: &str| {
        let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(f);
        std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
    };
    let ir = src("ir.t1");
    let start = ir
        .find("सार्वजनिक वृत्तिः समापकविफलशब्दः आदाय")
        .expect("ir.t1 declares the FAIL-word routine");
    let body = &ir[start..start + ir[start..].find("\nइति").expect("its body ends")];
    assert!(
        body.contains("गुणनम् ६५५३६") && body.contains("योगः १३१०७"),
        "the routine is no longer code × 65536 + 0x3333:\n{body}"
    );
    assert!(
        riscv_text().contains("मध्यरूपॱसमापकविफलशब्दः"),
        "the port no longer calls ir.t1's routine for its FAIL word"
    );
    assert_eq!(sadhana::t1::riscv64::fail_word(0x355), 0x0355_3333);
}

/// ॥ EVERY SYMBOL THE RUST EMITTER DECLARES IS PAIRED OR RECORDED ॥ `W-264`.
///
/// `the_riscv_emitter_port_declares_every_symbol_of_the_rust_original` above
/// iterates `RISCV_SYMBOLS` and asks whether each LISTED Rust symbol has a T1
/// counterpart. **A Rust function nobody listed is invisible to it**, so the
/// emitter can grow ahead of the port in silence — which is the one thing that
/// guard exists to prevent. Its name promises the population that EXISTS; it
/// checks the population someone REMEMBERED.
///
/// This one goes the other way: it DISCOVERS `riscv64.rs`'s module-level items
/// and requires each to be either paired in `RISCV_SYMBOLS` or recorded in
/// `RUST_ONLY` with a reason. **Adding a `pub fn` to the emitter now fails by
/// name.**
///
/// THE TWO TOGETHER ARE DERIVED-AGAINST-DERIVED ACROSS TWO LANGUAGES: this half
/// derives the Rust population, the half above derives the T1 declarations, and
/// `RISCV_SYMBOLS` is the pairing between them rather than the authority over
/// either. A pairing cannot be derived — no rule turns `emit_function` into
/// `यन्त्रवृत्त्युत्सर्जनम्` — but it can be forced to cover both populations,
/// which is what makes the list checkable instead of trusted.
///
/// WHAT THIS DOES NOT COVER, said rather than left for a reader to discover:
/// `RISCV_SYMBOLS` also carries QUALIFIED rows — `Refusal::NoTerminator`,
/// `Module::entry`, `Names::get` — which are enum variants and methods, not
/// module-level items. Those remain hand-listed and unverified against the Rust
/// side, as do `RISCV_FIELDS`'s struct fields. **One green here does not mean
/// both are discovered.**
#[test]
fn every_symbol_the_rust_emitter_declares_is_paired_or_recorded() {
    let items = riscv_rust_items();
    assert!(
        items.len() > 30,
        "only {} module-level items found in riscv64.rs — the reader has stopped \
         reaching the file and would pass over anything",
        items.len()
    );

    let paired: BTreeSet<&str> = RISCV_SYMBOLS.iter().map(|(rust, _)| *rust).collect();
    let recorded: BTreeSet<&str> = RUST_ONLY.iter().map(|(rust, _)| *rust).collect();

    let unchecked: Vec<String> = items
        .iter()
        .filter(|(_, name)| !paired.contains(name.as_str()) && !recorded.contains(name.as_str()))
        .map(|(kind, name)| format!("{kind} {name}"))
        .collect();
    assert!(
        unchecked.is_empty(),
        "riscv64.rs declares {} symbol(s) that NOTHING checks against the port — \
         pair each in RISCV_SYMBOLS with its `यन्त्र…` counterpart, or record it \
         in RUST_ONLY with the reason it has none:\n  {}",
        unchecked.len(),
        unchecked.join("\n  ")
    );

    let names: BTreeSet<&str> = items.iter().map(|(_, n)| n.as_str()).collect();
    let stale: Vec<&str> = RUST_ONLY
        .iter()
        .map(|(rust, _)| *rust)
        .filter(|r| !names.contains(r))
        .collect();
    assert!(
        stale.is_empty(),
        "RUST_ONLY records symbols riscv64.rs no longer declares — drop the \
         rows:\n  {}",
        stale.join("\n  ")
    );

    println!("METRIC riscv_rust_module_items {}", items.len());
    println!("METRIC riscv_pairs_listed {}", RISCV_SYMBOLS.len());
    println!("METRIC riscv_rust_only {}", RUST_ONLY.len());
}

// ══════════════════ `D-002i2` — THE TABLE READERS, BOTH DIRECTIONS ══════════════════

/// Every table name a stretch of `.t1` code asks the embed store for, in source
/// order.
///
/// **Comments are stripped first, and that is the whole instrument.**
/// `shrinkhala.t1:790` writes `पदविभागॱसमावेशपाठः` inside a `॰` comment while
/// reading no table at all, so a matcher over raw text reports a reader that is
/// not there — which is not a hypothetical. `anita.rs`'s own margin records
/// `निर्देशकोशः` going missing from `TABLES` while *"a `grep -c` looking for the
/// name found the COMMENT above and reported it present"*, and two live readers
/// sat with no table to resolve against. [`diagnostic_code_only`] is reused
/// rather than re-written: it applies `lex.rs:295`'s rule — `॰` ends the line —
/// and a second stripper is a second thing to get wrong.
///
/// The call is matched by its SHAPE, `<x>समावेशपाठः उक्तम् <name> इति`, over
/// whitespace-separated words, so it spans a line break the way the parser does.
/// The suffix match admits an unqualified call from inside `पदविभाग` itself
/// without a second pattern, and it does not match the routine's own declaration
/// at `lex.t1:236`, where the next word is `आदाय`.
fn embed_reader_names(text: &str) -> Vec<String> {
    let code = diagnostic_code_only(text);
    let words: Vec<&str> = code.split_whitespace().collect();
    let mut out = Vec::new();
    for (i, w) in words.iter().enumerate() {
        if !w.ends_with("समावेशपाठः") {
            continue;
        }
        if words.get(i + 1) != Some(&"उक्तम्") || words.get(i + 3) != Some(&"इति")
        {
            continue;
        }
        if let Some(name) = words.get(i + 2) {
            out.push((*name).to_string());
        }
    }
    out
}

/// Every embed-store read in the port: the source it is written in, and the
/// table it names.
///
/// **`sarani.t1` is excluded, and the exclusion is what keeps this measurable.**
/// That file is GENERATED by `tools/mkspectables.py` and carries every table's
/// name because it IS the store — it names all thirteen whatever the rest of the
/// port does, so counting it would make both directions below pass over a port
/// with no reader in it.
fn embed_readers() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for path in t1_sources() {
        let file = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default()
            .to_string();
        if file == "sarani.t1" {
            continue;
        }
        let text = std::fs::read_to_string(&path).expect("a .t1 source reads");
        for name in embed_reader_names(&text) {
            out.push((file.clone(), name));
        }
    }
    out
}

/// **`D-002i2`: every table the embed reaches is read, and every read names a
/// table.**
///
/// The row asks for the table readers the embed now reaches, and the tree
/// answers it: measured here, thirteen names in `anita.rs`'s `TABLES` and thirty
/// reads across six sources, none unread and none unknown. What was missing was
/// not a reader — it was anything that would say so if one went away. This test
/// is that, and it runs in both directions because each refuses a different
/// defect:
///
/// **A table with no reader** is the state `TABLES` documents as impossible —
/// *"every entry is here because a named reader in the tree is blocked on it,
/// not because the file exists"* — and it has happened twice. `निर्देशकोशः` was
/// written, verified, then lost to a routine-level union that took
/// `vakyavibhaga.t1` wholesale from worktrees which never carried it. A count
/// alone would not have noticed: the surviving sources still read twelve tables.
///
/// **A read naming no table** is the direction nothing else covers, and it fails
/// SILENTLY. `lex.t1:238`'s `समावेशपाठः` walks `समावेशनामकोश` and, on a name the
/// store does not hold, `प्रत्यागमनम् ०` — the nil word. It does not refuse, and
/// `.t1` has no raise; the caller then walks an empty table and reports zero rows
/// as an answer. The embed's own resolver cannot catch it either, because that
/// resolver reads `समावेशः आरभ्य <name> समाप्तम्` at LEX time while this is a
/// run-time lookup with a string argument, which is why the port reaches its own
/// tables this way at all (`lex.t1:227`: compiled natively, nothing filled the
/// store and the assembler had no mnemonics). So a misspelt name here is a
/// reader that returns nothing forever, and this assertion is the only thing
/// that would say the word is wrong.
#[test]
fn every_table_the_embed_reaches_is_read_by_the_port_and_every_read_names_a_table() {
    let names = sadhana::t1::anita::table_names();
    let readers = embed_readers();
    let sources: BTreeSet<&str> = readers.iter().map(|(f, _)| f.as_str()).collect();
    println!("METRIC t1_embed_tables {}", names.len());
    println!("METRIC t1_embed_reads {}", readers.len());
    println!("METRIC t1_embed_reading_sources {}", sources.len());

    // THE INSTRUMENT IS CONTROLLED BEFORE IT IS BELIEVED, both ways round. The
    // comment case is the one that actually misreported a reader once, and the
    // live case is what says the matcher can still find one at all — a stripper
    // that returned "" would pass the negative control and every direction
    // below would then be vacuous.
    let probe = names.first().expect("TABLES is not empty");
    let commented = format!("॰ पदविभागॱसमावेशपाठः उक्तम् {probe} इति");
    assert!(
        embed_reader_names(&commented).is_empty(),
        "the matcher counted a read written inside a ॰ comment; \
         shrinkhala.t1:790 is exactly that line and reads no table"
    );
    let live = format!("चरः पाठ्यम् भवति पदविभागॱसमावेशपाठः उक्तम् {probe} इति ।");
    assert_eq!(
        embed_reader_names(&live),
        vec![(*probe).to_string()],
        "the matcher no longer finds a read it is given verbatim"
    );

    // Non-vacuity of the measurement itself: the port is the evidence.
    assert!(
        names.len() >= 13,
        "only {} tables in anita.rs's TABLES; thirteen stood there for D-002i2",
        names.len()
    );
    assert!(
        readers.len() >= names.len() && sources.len() >= 4,
        "{} reads across {} sources cannot cover {} tables",
        readers.len(),
        sources.len(),
        names.len()
    );

    // A — every table has a reader.
    let read: BTreeSet<&str> = readers.iter().map(|(_, t)| t.as_str()).collect();
    let unread: Vec<&str> = names
        .iter()
        .copied()
        .filter(|n| !read.contains(n))
        .collect();
    assert!(
        unread.is_empty(),
        "{} table(s) in anita.rs's TABLES have no reader in the port: {}\n\
         Either a routine lost its read — निर्देशकोशः did, to a routine-level \
         union — or the entry was added for a reader nobody wrote.",
        unread.len(),
        unread.join(", ")
    );

    // B — every read names a table. A miss returns ० and nothing refuses.
    let known: BTreeSet<&str> = names.iter().copied().collect();
    let unknown: Vec<String> = readers
        .iter()
        .filter(|(_, t)| !known.contains(t.as_str()))
        .map(|(f, t)| format!("{f}: {t}"))
        .collect();
    assert!(
        unknown.is_empty(),
        "{} read(s) name a table anita.rs's TABLES does not hold:\n  {}\n\
         समावेशपाठः answers ० for a name the store lacks and does not refuse, \
         so this reader walks an empty table forever.",
        unknown.len(),
        unknown.join("\n  ")
    );
}
