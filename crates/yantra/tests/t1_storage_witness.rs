//! **THE STORAGE WITNESS: a `.t1` source with a record local and an array local,
//! compiled through the `.t1` front end, RUN on `yantra`, and asserted BY VALUE.**
//!
//! # Why this file exists
//!
//! `W-283` landed a field-store arm that fired on all 400 corpus sites, closed
//! its conservation identity exactly, and emitted code that faulted. The fault —
//! `run:bad-access` on `artha.t1` and `ir.t1` — was reported by exactly ONE test
//! in the tree: `paradigm_encode.rs`, the **1,892-second census**.
//!
//! **Seventeen test files reference `Machine::load_elf`. None of the cheap ones
//! is a witness for this**, and the two that look like witnesses are not:
//!
//! * `t1_transcriptions.rs` — 3.5 s, eight `load_elf` references, and it
//!   **executes nothing**: it parses the SOURCE TEXT of `yantra-run` to read its
//!   RAM and step-budget arguments as strings (`:754`). Cheap because it runs no
//!   image, and a grep for `load_elf` cannot tell that from a real witness.
//! * `riscv64_emitter.rs` — 2 s, and it **hand-builds IR** from
//!   `Instruction::ConstInt(…)` literals. It never goes through `ir.t1`, so no
//!   lowering change can reach it.
//!
//! The cheapest test that genuinely compiles a `.t1` source through the chain
//! and runs the image is
//! `t1_whole_chain::the_t1_chain_carries_ashtaka_to_an_image_that_runs` — and
//! **`ashtaka.t1` has ZERO record-typed locals and ZERO field assignments**, so
//! it is green on a broken store arm. The sources that DO carry the construct
//! (`artha.t1` 82 sites, `ir.t1` 35) are compiled through the chain by the
//! census and by nothing else. That is why a 400-site arm landed and faulted.
//!
//! # `W-292`: THESE FIXTURES WROTE A MALFORMED STRUCT AND PASSED ANYWAY
//!
//! Every `संरचना` here opened with `आदि`. **The corpus has 60 structs and every
//! one opens with `आरभ्य`; zero use `आदि`.** So `संरचनापठनम्` never consumed the
//! opener, the field loop began on the wrong token, and `आदि` and `ॱॱ` were
//! parsed as field names — a field table with a phantom leading entry.
//!
//! **AND THE FIXTURES WERE GREEN THROUGHOUT, BEFORE AND AFTER THE CORRECTION.**
//! Measured on one tree, openers the only variable:
//!
//! ```text
//!                        आदि (malformed)   आरभ्य (correct)
//!   record   7                  ok               ok
//!   returned 13                 ok               ok
//!   nested   21                 ok               ok
//! ```
//!
//! **That is the limit of every fixture in this file and it is worth more than
//! the fixtures: a write and a read agree on whatever offset the field table
//! says, so a round-trip cannot detect a WRONG table.** These witness that a
//! store reaches storage and a load reads it back. They do not witness that a
//! field is at the offset the declaration implies — nothing here does, and
//! nothing here can, because both halves of the round trip use the same table.
//!
//! A fixture that could would have to compare against a layout computed some
//! other way, or read a field it never wrote.
//!
//! # What this file does NOT cover, stated so a green is not oversold
//!
//! **One record and one array, in one module, in one routine.** It is not the
//! corpus. It cannot see:
//!
//! * a **cross-module** struct — the type is declared in the same source;
//! * a record **returned** from a routine (38 corpus routines do; a record built
//!   in a frame dangles in every one of them);
//! * **nested** records, arrays of records, or records of arrays;
//! * any element width but the machine word — the field path multiplies an
//!   ordinal by a literal `८` with no `विस्तार` check anywhere on it, which is a
//!   known fail-open carried deliberately and filed, not fixed.
//!
//! A pass here means *these two shapes allocate, store and load back*. It does
//! not mean storage works.
//!
//! # Why the number and never the status
//!
//! The startup stub stores the entry routine's result to the SiFive finisher
//! (`riscv64::emit_startup_object`), so **an entry's return value IS the halt
//! status**. An image with no entry halts `0x5555` success — and so does an
//! image that does nothing, which is why `status == 0` cannot discriminate a
//! working back half from a degenerate one. Two corpus sources have been
//! measured reaching RUN that way. **Each fixture below returns a DIFFERENT
//! number, so the record case and the array case cannot pass for each other.**
//!
//! # Why this drives the front end and the RUST emitter
//!
//! `शृङ्खलाॱमण्डलानिप्रतिबिम्बम्` — the `.t1` driver — **names no entry, and
//! deliberately**: `shrinkhala.t1:697` records that doing so would be the second
//! thing in the tree to reference a startup-DEFINED symbol, and that
//! `paradigm_encode.rs`'s pre-link closure is **blind to the startup object**, a
//! blindness dormant only because nothing references such a symbol. So the `.t1`
//! driver cannot produce a non-zero status without waking a known defect.
//!
//! This file therefore takes the `t1_build` route: the `.t1` FRONT END through
//! [`Front`] — which is `lex.t1`, `parse.t1`, `artha.t1` and **`ir.t1`**, the
//! half under test — then the Rust emitter, whose twin agrees with the `.t1`
//! emitter octet for octet. **The lowering being witnessed is the product's.**

use sadhana::encode::Target;
use sadhana::kosha::LOAD_ADDRESS;
use sadhana::nidana::Language;
use sadhana::t1::ast::SymbolId;
use sadhana::t1::chain::Front;
use sadhana::t1::riscv64;
use sadhana::{assemble_object, link_objects, vastu};
use std::path::{Path, PathBuf};
use yantra::Machine;

/// **RAM IS SIZED FROM THE IMAGE, AND A CONSTANT HERE WAS THE SECOND HALF OF
/// THIS FILE'S REDS.** `W-295`.
///
/// This read `yantra::DEFAULT_RAM` — twenty megabytes — and every fixture that
/// ALLOCATES refused at load, before a single instruction ran:
///
/// ```text
///   the shipped loader accepts the image:
///   "segment at 0x80000000 needs 536936624 bytes and RAM is 20971520 — raise it"
/// ```
///
/// 536,936,624 is the record region (512 MiB since `e9bb8e77`, sized from a
/// MEASURED whole-corpus high water of 353,242,600 octets) plus the image. The
/// region is `॥ स्थानम् ॥` in `ॱरिक्त` — address space with no file bytes — so
/// the cost is the loader's RAM argument and nothing else.
///
/// **`yantra::ram_for` IS THE ONE STATEMENT OF THAT SIZING**, and its own margin
/// records this exact failure happening once before, to the census's runner,
/// which "loaded every image at `DEFAULT_RAM` until the region raise stopped
/// every census row at `load` … while `yantra-run` ran the same image — two
/// loaders, two answers." This file was the third loader and gave the third
/// answer. `yantra-run` and `paradigm_encode.rs:1971` both call `ram_for`; a
/// witness that does not is measuring its own constant.
///
/// It is a FUNCTION of the image and not a bigger constant deliberately: a fixed
/// number large enough for the region would go stale the next time the region
/// moves, in exactly the way this one did.
fn ram_for(elf: &[u8]) -> usize {
    yantra::ram_for(elf)
}
const BUDGET: u64 = 200_000_000;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn assemble(text: &str, name: &str) -> Vec<u8> {
    assemble_object(
        text,
        Some(name),
        Target::Uncompressed,
        false,
        Language::English,
    )
    .unwrap_or_else(|ds| {
        let named: Vec<String> = ds
            .iter()
            .take(4)
            .map(|d| {
                let line = text.lines().nth(d.line.saturating_sub(1)).unwrap_or("");
                format!("emitted line {}: `{line}` — {}", d.line, d.reason)
            })
            .collect();
        panic!(
            "सङ्केतन refused the emitted text of `{name}`:\n  {}",
            named.join("\n  ")
        );
    })
}

/// Compile one `.t1` source through the `.t1` front end, emit, link, and RUN.
/// Answers the halt, so the caller asserts the number.
fn compile_and_run(src: &str, module_name: &str, entry: &str) -> String {
    let mut front = Front::load(&spec_root()).expect("the front end loads");
    front.lex(src).expect("पदविभाग lexes the fixture");
    front.parse().expect("व्याकर parses it");
    front.resolve().expect("अर्थ resolves every name");
    front.typecheck().expect("अर्थ typechecks every statement");
    front.build_ir().expect("मध्यरूप builds IR");

    let module = front
        .module(module_name, Some(entry))
        .expect("the module reads back from मध्यरूप's arenas");

    // THE ENTRY'S LABEL, AND ITS ABSENCE IS A FAILURE HERE RATHER THAN A
    // DEFAULT. `None` gives an image whose stub halts success without calling
    // anything — which would make every assertion below pass on an empty
    // program. The `.expect` is the guard against that, not ceremony.
    let entry_label = module
        .entry
        .map(|sym| riscv64::routine_label(&module.names, sym))
        .transpose()
        .expect("the entry's label emits")
        .expect("the fixture NAMES an entry — without one the stub halts 0 and proves nothing");

    let text = riscv64::emit_module(&module).expect("the emitter accepts the module");
    // `W-284` — THE STARTUP CARRIES THE RECORD REGION, AND ONLY WHEN THE MODULE
    // ALLOCATES. It lives there and not in `emit_data` because that runs per
    // object and the linker refuses a second definition; asking for it here
    // without `module_allocates` would put a 4 MB region in every image that
    // does not use one. Passing `false` by mistake is not silent: the linker
    // answers "`रचनासूचकः` is not defined by any object", which is how this
    // line came to exist.
    let startup = riscv64::emit_startup_object_with_records(
        Some(&entry_label),
        riscv64::module_allocates(&module),
    );
    // `assemble_object` answers OCTETS; the linker takes read-back objects.
    let read = |b: &[u8], what: &str| {
        vastu::read(b).unwrap_or_else(|| panic!("the {what} object does not read back"))
    };
    let objects = [
        read(&assemble(&startup, "आरम्भ"), "startup"),
        read(&assemble(&text, module_name), "module"),
    ];
    let elf = link_objects(&objects, LOAD_ADDRESS)
        .unwrap_or_else(|es| panic!("the linker refused: {}", es.join("; ")));

    let mut m =
        Machine::load_elf(&elf, ram_for(&elf)).expect("the shipped loader accepts the image");
    let mut out: Vec<u8> = Vec::new();
    let halt = m.run(BUDGET, &mut out);
    format!("{halt:?}")
}

/// **A RECORD WHOSE FIELDS ARE NOT ALL SCALARS — RE-FOUNDED, AND THE AXIS IT
/// NAMED WAS THE WRONG ONE.** `W-292`.
///
/// This test was written to ask whether a record still addresses correctly when
/// one of its fields is an aggregate. It failed with `BadAccess { addr: 0 }`,
/// and the failure was read as an answer: nesting defeats the field chain.
///
/// **IT WAS NOT THE NESTING.** `t1_execution`'s
/// `which_field_cause_refuses_a_nested_record` varied the two axes separately
/// and measured the field arm's cause arena directly:
///
/// ```text
///   scalar struct, SECOND field    clean, 8 instructions
///   nested struct, SECOND field    clean, 8 instructions   ← the run field itself
///   scalar struct, FIRST  field    cause ३९ ×2
///   nested struct, FIRST  field    cause ३९ ×2
/// ```
///
/// **A record with an aggregate member addresses its fields exactly as well as
/// an all-scalar one** — the nested variant reads the RUN field and lowers
/// clean. What refuses is **the first field of any struct**, and every probe
/// this test had held that constant: all three read `प्रथमम्`. **A fixture that
/// varies one axis while accidentally pinning a second measures the second.**
///
/// # What the cause arena says, and it is a parse defect
///
/// Cause ३९ is `क्षेत्रमिलितम् समम् असत्यम्` — the field name is not in the
/// struct. It is not: for `संरचना धारकः` with fields `प्रथमम्` and `द्वितीयम्`,
/// the declaration's range `प्राचलादि=1 … प्राचलान्त=3` holds **three** entries
/// for **two** fields, and they name tokens 6, 8 and 11 — `आदि`, `ॱॱ` and
/// `द्वितीयम्`. **`प्रथमम्` is named by no entry at all.** The field table is
/// malformed; the lowering that reads it is correct to refuse.
///
/// **REFOUNDED 2026-09-13 — EVERY SENTENCE OF THE PARAGRAPH ABOVE HAS EXPIRED,
/// AND IT WAS STILL GENERATING WORK.** It is kept rather than deleted because the
/// belief history is the point, but read it as dated, not as current:
///
/// - `parse.t1`'s struct loop calls `प्राचलयोजनम्` **exactly once per field**
///   (`:1327`), and that routine appends one entry and answers its slot
///   (`:268-275`). One entry per field, not three for two. This landed in
///   `W-215` on **2026-09-04**, which is BEFORE the paragraph was written — so
///   the reading was never right against this tree, rather than having decayed.
/// - `प्रथमम्` IS named by an entry. `:272` below already recorded it
///   round-tripping at `a7a836fa`, in the same commit as the paragraph above.
///   **The file contradicted itself and nobody read the two halves together.**
/// - The offset is **8**, measured, not 16.
///
/// # The second finding: measured, and it is closed
///
/// Measured at `5c52a405` by reading the emitted immediates rather than inferring
/// them — `प्रथमम्` at **०** and `द्वितीयम्` at **८**, with the two halts of a
/// two-live-record fixture agreeing (`Some(7)`, no neighbour corruption, and
/// `Some(11)` for `५ + ६` read back separately). That measurement is now an
/// ASSERTION in
/// `the_two_fields_of_a_record_sit_at_absolute_offsets_zero_and_eight`.
///
/// **WHY IT WAS NEVER OBSERVED TO CLOSE:** the evidence lived in a probe that
/// asserted nothing, so the only thing that changed when the defect was fixed was
/// a number in a log nobody reads. A recorded-but-unasserted defect cannot
/// announce its own repair — and a stale paragraph is not inert, it is a source
/// of briefs. This one produced a unit brief predicting offset 16 and warning of
/// a trap that cannot occur.
///
/// **AND THE PIN IS ABSOLUTE FOR A REASON:** `off(k+1) − off(k) == 8` holds at
/// `(0, 8)` and equally at `(8, 16)`, so a difference-only test cannot see a
/// shifted origin. A fixture that writes and reads ONE field agrees with itself
/// at the wrong address and passes; every other storage witness in this file has
/// that shape.
///
/// # What this test asserts now
///
/// The question it was written for, asked on the axis that actually isolates
/// it: a struct with an aggregate member, reading a field that the table does
/// reach. **`२१` — a fourth number**, so no single bug satisfies this and the
/// other three fixtures at once.
///
/// WHAT IT DOES NOT COVER: the first field, which is recorded above and belongs
/// to whichever row repairs the field table; and whether the wrong offset
/// corrupts a neighbouring record, which needs a fixture with two live records
/// and is the measurement that would turn the offset finding into a run failure.
#[test]
fn a_record_with_an_aggregate_field_still_addresses_its_scalars() {
    // THE AGGREGATE MEMBER IS FIRST AND THE SCALAR READ IS SECOND — the shape
    // that isolates nesting, now that the first-field refusal is known to be a
    // separate defect.
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "\n",
        "संरचना धारकः आरभ्य\n",
        "    सूची ॱॱ अङ्कः अन्तः न६४ ऽ\n",
        "    द्वितीयम् ॱॱ न६४\n",
        "समाप्तम् ।\n",
        "\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः धा ॱॱ धारकः भवति ० ।\n",
        "    धा ॱ द्वितीयम् भवति २१ ।\n",
        "    प्रत्यागमनम् धा ॱ द्वितीयम् ।\n",
        "इति\n",
    );
    let halt = compile_and_run(src, "परीक्षा", "मुख्यम्");
    println!("METRIC t1_storage_witness_nested_halt {halt}");

    // THE FIRST-FIELD REFUSAL, RECORDED AND NOT ASSERTED — **AND IT NO LONGER
    // REFUSES.** Same struct, same program, one token different, so a reader can
    // see both outcomes side by side without this test turning red over a defect
    // it did not find.
    //
    // WHEN THIS WAS WRITTEN the probe below returned a fault and the assertion
    // further down called the first field "separately known to be unreachable —
    // cause ३९, a malformed field table". Measured at `a7a836fa` it returns
    // `Some(17)`: the fixture writes the FIRST field and reads it back. The
    // premise expired and nobody was watching, because a probe that asserts
    // nothing cannot go green — **the only thing that changes when a recorded
    // defect is fixed is a number in a log nobody reads.**
    //
    // KEPT RATHER THAN DELETED, and still unasserted. It is the cheapest witness
    // that the first field is reachable, and deleting it would lose the record
    // that it once was not. Whoever asserts it should first check cause ३९ in the
    // census — this fixture round-tripping says nothing about the cause's count.
    let first = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "संरचना धारकः आरभ्य\n    प्रथमम् ॱॱ न६४ ऽ\n    द्वितीयम् ॱॱ न६४\nसमाप्तम् ।\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः धा ॱॱ धारकः भवति ० ।\n",
        "    धा ॱ प्रथमम् भवति १७ ।\n",
        "    प्रत्यागमनम् धा ॱ प्रथमम् ।\n",
        "इति\n",
    );
    println!(
        "METRIC t1_storage_witness_probe_first_field {}",
        compile_and_run(first, "परीक्षा", "मुख्यम्")
    );

    assert!(
        halt.contains("Some(21)"),
        "a record with an aggregate field must still allocate and address its \
         scalar fields; the machine halted {halt}. The size is `८ × field count` \
         and a run field counts as one word (a pointer, ADR-0026). This fixture \
         reads the SECOND field because the first was unreachable when it was \
         written — cause ३९, a malformed field table. That is no longer true: \
         the probe above round-trips the first field at `a7a836fa`. The choice \
         of the second field is now historical, not necessary."
    );
}

/// **TWO LIVE RECORDS — THE MEASUREMENT THIS FILE NAMED AS MISSING, TAKEN.**
///
/// The header of `a_record_with_an_aggregate_field_still_addresses_its_scalars`
/// closes by naming what it does not cover: *"whether the wrong offset corrupts a
/// neighbouring record, which needs a fixture with two live records and is the
/// measurement that would turn the offset finding into a run failure."* This is
/// that fixture.
///
/// # Why a one-record fixture cannot answer it
///
/// If `द्वितीयम्` were emitted at offset **16** in a two-field record that is
/// **16 octets** long, a fixture that writes that field and reads it back
/// **agrees with itself at the wrong address** and passes. Every storage witness
/// in this file has that shape. Only a SECOND live record placed where the
/// overflow lands can tell the two apart, and only if the second record is
/// written FIRST — otherwise the overflow happens before there is anything to
/// corrupt and the read still answers correctly.
///
/// So the order is load-bearing: write `ब ॱ प्रथमम्`, THEN write `अ ॱ द्वितीयम्`,
/// then read `ब ॱ प्रथमम्` back. A correct table leaves `७`; an offset of 16
/// writes `९९` over the neighbour and the read answers `९९`.
///
/// # Registered before the run
///
/// **PREDICTED `Some(7)` — no corruption** — against the brief this unit arrived
/// with, which predicts 16. The brief derives from this file's own `:219`
/// paragraph (*"three entries for two fields … `प्रथमम्` is named by no entry at
/// all"*), and that paragraph's premise **expired**: `:272` records that the
/// first field round-trips at `a7a836fa`, and `parse.t1`'s struct loop appends
/// exactly ONE entry per field through `प्राचलयोजनम्` (`:1327`), a shape that
/// landed in `W-215` on 2026-09-04. If the table holds one entry per field then
/// the ordinal is already a rank among fields and the offset is already `8`.
///
/// **The falsifier is the whole point of writing the number down first:** an
/// answer of `Some(99)` means the table really is malformed, the brief is right,
/// and the trap it names — that fixing the ordinal alone moves the error from
/// `+8` to `−8` — is live.
// NOT `#[ignore]`d, DELIBERATELY. This asserts, and an ignored assertion guards
// nothing — a whole-crate run does not reach it, so the defect it pins could
// reopen without any gate going red. That is precisely how the paragraph this
// test refounds went stale: the evidence lived in an unasserted probe, and the
// only thing that changed when the defect was fixed was a number in a log.
#[test]
fn the_two_fields_of_a_record_sit_at_absolute_offsets_zero_and_eight() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "संरचना धारकः आरभ्य\n    प्रथमम् ॱॱ न६४ ऽ\n    द्वितीयम् ॱॱ न६४\nसमाप्तम् ।\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः अ ॱॱ धारकः भवति ० ।\n",
        "    चरः ब ॱॱ धारकः भवति ० ।\n",
        "    ब ॱ प्रथमम् भवति ७ ।\n",
        "    अ ॱ द्वितीयम् भवति ९९ ।\n",
        "    प्रत्यागमनम् ब ॱ प्रथमम् ।\n",
        "इति\n",
    );
    println!(
        "METRIC t1_storage_witness_probe_neighbour_corruption {}",
        compile_and_run(src, "परीक्षा", "मुख्यम्")
    );

    // BOTH FIELDS OF ONE RECORD, READ BACK SEPARATELY. If the two fields alias —
    // the failure a single-field fixture cannot see either — the sum is wrong
    // while each individual read still looks right.
    let both = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "संरचना धारकः आरभ्य\n    प्रथमम् ॱॱ न६४ ऽ\n    द्वितीयम् ॱॱ न६४\nसमाप्तम् ।\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः अ ॱॱ धारकः भवति ० ।\n",
        "    अ ॱ प्रथमम् भवति ५ ।\n",
        "    अ ॱ द्वितीयम् भवति ६ ।\n",
        "    प्रत्यागमनम् अ ॱ प्रथमम् योगः अ ॱ द्वितीयम् ।\n",
        "इति\n",
    );
    println!(
        "METRIC t1_storage_witness_probe_both_fields_sum {}",
        compile_and_run(both, "परीक्षा", "मुख्यम्")
    );

    // **THE ABSOLUTE OFFSET, READ OFF THE EMITTED TEXT.** The two halts above are
    // BEHAVIOURAL and a behavioural pass cannot distinguish `(0, 8)` from
    // `(0, 16)` — both round-trip, and neither aliases. `off(k+1) − off(k) == 8`
    // holds at `(0, 8)` AND at `(8, 16)`, so a difference test is decoration.
    // This reads the immediates the emitter actually wrote.
    let mut front = Front::load(&spec_root()).expect("the front end loads");
    front.lex(both).expect("lex");
    front.parse().expect("parse");
    front.resolve().expect("resolve");
    front.typecheck().expect("typecheck");
    front.build_ir().expect("build_ir");
    let module = front
        .module("परीक्षा", Some("मुख्यम्"))
        .expect("module reads back");
    let text = riscv64::emit_module(&module).expect("the emitter emits");
    // NOT FILTERED BY GUESSED MNEMONIC. The first version of this looked for
    // lines beginning `sd `/`ld ` and printed EMPTY — which is what a wrong
    // pattern and a module with no stores produce alike. The emitted text is
    // Devanagari: a field read is `आहारः <dst>म् <base>त् <imm>न ।`, and the
    // reads off the STACK pointer are frame slots, not fields.
    let field_offsets: Vec<String> = text
        .lines()
        .map(str::trim)
        .filter(|l| l.starts_with("आहारः") && !l.contains("स्तूपसूचकःत्"))
        .filter_map(|l| {
            l.split_whitespace()
                .nth(3)
                .map(|t| t.trim_end_matches('न').to_string())
        })
        .collect();
    println!(
        "METRIC t1_storage_witness_probe_field_offsets {}",
        field_offsets.join(",")
    );

    // **THE ABSOLUTE PIN, AND IT IS ABSOLUTE ON PURPOSE.** `off(k+1) − off(k) == 8`
    // holds at `(0, 8)` and equally at `(8, 16)`, so a difference-only test cannot
    // see a shifted origin and is decoration. These are the two offsets themselves.
    //
    // MEASURED, NOT PREDICTED: `०` and `८`. The brief this unit arrived with
    // predicted `१६` for the second field and said pinning `8` would land a red
    // today. It does not — **the defect it describes is already closed**, and this
    // assertion is what stops it from reopening unobserved. See the refounded
    // paragraph in `a_record_with_an_aggregate_field_still_addresses_its_scalars`.
    // THE SET, NOT THE MULTISET, AND THE REASON IS A CORRECTION TO THIS TEST'S
    // FIRST VERSION. It asserted the sequence `[०, ८]` and measured `[०, ०, ८]`
    // — **and the extra entry was mine, not the emitter's.** The bump allocator
    // reads its cursor with `आहारः क्षणिक१म् क्षणिक०त् ०न`, which is an indexed
    // load off a temporary and not a field access at all. A filter that admits it
    // is a population error, so the fix belongs in the claim rather than in the
    // subject. The discriminating content is which offsets appear, and `१६` must
    // not be among them.
    let mut distinct = field_offsets.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(
        distinct,
        vec!["०".to_string(), "८".to_string()],
        "the two fields of a two-field record must be addressed at ABSOLUTE \
         offsets 0 and 8. Got {field_offsets:?}. An offset of १६ for the second \
         field is the `W-292` defect: an ordinal counted in TOKEN space rather \
         than FIELD space. A one-record fixture cannot catch it — it agrees with \
         itself at the wrong address — which is why this asserts the emitted \
         immediates while the two halts above assert the behaviour. Note that १६ \
         DOES legitimately appear elsewhere in this module as the record's SIZE \
         (`योगः क्षणिक२म् क्षणिक१न १६न`, two fields × ८), so a bare grep for १६ \
         over the emitted text would fire on a correct tree."
    );
}

/// **A RECORD RETURNED FROM A ROUTINE — THE CASE THE BUMP REGION WAS CHOSEN
/// FOR, AND IT WAS UNTESTED UNTIL NOW.** `W-291`.
///
/// `W-279` rejected a frame for records with a measured reason: **38 corpus
/// routines RETURN a struct type, so a record built in a frame dangles in every
/// one of them.** That was the argument for a static bump region — and nothing
/// in the tree exercised it. **The motivating case for a design decision is the
/// one case most worth testing, and it is the one that gets skipped**, because
/// the simplest fixture never crosses a call boundary.
///
/// THE FIXTURE CROSSES ONE: `रचयति` builds the record, writes a field and
/// RETURNS it; `मुख्यम्` receives it and reads the field back. **Under a frame
/// the callee's frame is dead by the time the caller reads** — the value would
/// be whatever the stack now holds, plausibly zero, plausibly the right answer
/// by luck. Under the bump region it survives.
///
/// **`१३` and not `७` or `९`**: the other two fixtures return those, and a
/// single bug that made all three agree would be hidden by a shared expected
/// value. Three shapes, three numbers.
///
/// WHAT IT STILL DOES NOT COVER: one module. A record crossing a MODULE
/// boundary is a different question and needs a two-source fixture this
/// harness cannot yet build.
#[test]
fn a_record_returned_from_a_routine_survives_the_call() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "\n",
        "संरचना बिन्दुः आरभ्य\n",
        "    प्रथमम् ॱॱ न६४ ऽ\n",
        "    द्वितीयम् ॱॱ न६४\n",
        "समाप्तम् ।\n",
        "\n",
        "सार्वजनिक वृत्तिः रचयति ददाति बिन्दुः आदि\n",
        "    चरः स्थानम् ॱॱ बिन्दुः भवति ० ।\n",
        "    स्थानम् ॱ द्वितीयम् भवति १३ ।\n",
        "    प्रत्यागमनम् स्थानम् ।\n",
        "इति\n",
        "\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः लब्धम् ॱॱ बिन्दुः भवति रचयति ।\n",
        "    प्रत्यागमनम् लब्धम् ॱ द्वितीयम् ।\n",
        "इति\n",
    );
    let halt = compile_and_run(src, "परीक्षा", "मुख्यम्");
    println!("METRIC t1_storage_witness_returned_halt {halt}");
    assert!(
        halt.contains("Some(13)"),
        "a record built in a callee and RETURNED must still read 13 in the caller; the \
         machine halted {halt}. This is the case the bump region was chosen for — 38 corpus \
         routines return a struct type — so a failure here says the storage does not outlive \
         the frame that made it, which is the whole argument `W-279` measured."
    );
}

/// **A RECORD LOCAL, A FIELD WRITE, AND THE VALUE READ BACK — the exact shape
/// that faulted.** `चरः स्थानम् ॱॱ बिन्दुः भवति ०` declares; `स्थानम् ॱ
/// द्वितीयम् भवति ७` writes; the return reads it back.
///
/// **`७` and not `०`**: an image that allocates nothing, stores nothing and
/// returns a constant would halt ० and satisfy a status check. Seven is a number
/// only a real store-then-load can produce.
#[test]
fn a_record_local_takes_a_field_write_and_reads_it_back() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "\n",
        "संरचना बिन्दुः आरभ्य\n",
        "    प्रथमम् ॱॱ न६४ ऽ\n",
        "    द्वितीयम् ॱॱ न६४\n",
        "समाप्तम् ।\n",
        "\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः स्थानम् ॱॱ बिन्दुः भवति ० ।\n",
        "    स्थानम् ॱ द्वितीयम् भवति ७ ।\n",
        "    प्रत्यागमनम् स्थानम् ॱ द्वितीयम् ।\n",
        "इति\n",
    );
    let halt = compile_and_run(src, "परीक्षा", "मुख्यम्");
    println!("METRIC t1_storage_witness_record_halt {halt}");
    assert!(
        halt.contains("Some(7)"),
        "a record local must take a field write and read 7 back; the machine halted {halt}. \
         A fault here is the store landing on storage that was never allocated — the base is \
         ० and the offset is added to it."
    );
}

/// **AN ARRAY LOCAL, AN ELEMENT WRITE, AND THE VALUE READ BACK.** The index
/// twin of the record case, and **`९` so the two cannot pass for each other**:
/// if a single bug made both return the same number, one expected value would
/// hide it.
#[test]
fn an_array_local_takes_an_element_write_and_reads_it_back() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः पङ्क्तिः ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n",
        "    पङ्क्तिः अङ्कः १ अन्तः भवति ९ ।\n",
        "    प्रत्यागमनम् पङ्क्तिः अङ्कः १ अन्तः ।\n",
        "इति\n",
    );
    let halt = compile_and_run(src, "परीक्षा", "मुख्यम्");
    println!("METRIC t1_storage_witness_array_halt {halt}");
    assert!(
        halt.contains("Some(9)"),
        "an array local must take an element write and read 9 back; the machine halted {halt}. \
         Nine and not seven on purpose: the record fixture returns seven, so one bug cannot \
         satisfy both."
    );
}

/// **A THROWAWAY PROBE — the `भवति <call>` shape, which every `भवति ०` count is
/// blind to.** Not an assertion yet; it records two halts and names what each
/// value would mean.
///
/// `artha.t1:688`'s entry opens `चरः अष्टकप्रकार ॱॱ अर्थप्रकार भवति दोषार्थः ।` —
/// a record local initialised from a CALL to a struct-returning routine, not from
/// `भवति ०`. `ir.t1:2457`'s struct arm allocates and raises shape ३० BEFORE the
/// initialiser arm at `:2464` is reached, so the call may be built and discarded.
///
/// The two fixtures separate the questions. `carried` never writes the field in
/// the caller, so its answer is the initialiser's: **५ means the call was
/// honoured, ० means the struct arm allocated a fresh record and dropped it.**
/// `written` is artha's shape exactly — call-init, then a field write and read —
/// and **२१ means it works, a fault means this is the defect.**
/// # OUT OF SCOPE FOR `tools/sassembly-model.py` — NOT PENDING
///
/// **The model covers STORAGE: what a declaration is given, and what address an
/// access computes.** This fixture's record local is initialised from a CALL
/// rather than from `भवति ०`, so what it holds is decided by the call
/// convention — who writes the returned record, and where — and that is not a
/// storage rule.
///
/// **The model must NOT be extended to predict it.** An abstention asks for a
/// measurement; a guess gets quoted. A model that reaches past its subject is
/// how the false "defect 3" came to be written, and that retraction is recorded
/// in `predict()`'s own body.
///
/// So this is "not this instrument's question" rather than "pending", which
/// moves the score's DENOMINATOR honestly instead of its numerator dishonestly.
#[test]
fn probe_a_record_local_initialised_from_a_call() {
    let carried = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "संरचना धारकः आरभ्य\n    प्रथमम् ॱॱ न६४ ऽ\n    द्वितीयम् ॱॱ न६४\nसमाप्तम् ।\n",
        "वृत्तिः विषम् ददाति धारकः आदि\n",
        "    चरः फलम् ॱॱ धारकः भवति ० ।\n",
        "    फलम् ॱ द्वितीयम् भवति ५ ।\n",
        "    प्रत्यागमनम् फलम् ।\n",
        "इति\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः धा ॱॱ धारकः भवति विषम् ।\n",
        "    प्रत्यागमनम् धा ॱ द्वितीयम् ।\n",
        "इति\n",
    );
    println!(
        "METRIC t1_storage_witness_probe_call_init_carried {}",
        compile_and_run(carried, "परीक्षा", "मुख्यम्")
    );

    let written = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "संरचना धारकः आरभ्य\n    प्रथमम् ॱॱ न६४ ऽ\n    द्वितीयम् ॱॱ न६४\nसमाप्तम् ।\n",
        "वृत्तिः विषम् ददाति धारकः आदि\n",
        "    चरः फलम् ॱॱ धारकः भवति ० ।\n",
        "    फलम् ॱ द्वितीयम् भवति ५ ।\n",
        "    प्रत्यागमनम् फलम् ।\n",
        "इति\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः धा ॱॱ धारकः भवति विषम् ।\n",
        "    धा ॱ द्वितीयम् भवति २१ ।\n",
        "    प्रत्यागमनम् धा ॱ द्वितीयम् ।\n",
        "इति\n",
    );
    println!(
        "METRIC t1_storage_witness_probe_call_init_written {}",
        compile_and_run(written, "परीक्षा", "मुख्यम्")
    );
}

/// **`artha.t1`'s ENTRY, SHAPE FOR SHAPE, IN ONE MODULE — AND IT RUNS CLEAN.**
///
/// `artha.t1:688` (`पाठप्रकारार्थः`) holds four shapes no other fixture here has,
/// all at once: two record locals live together, writes at offset ० AND offset ८,
/// a record passed AS AN ARGUMENT (`अर्थप्रकारयोजनम् अष्टकप्रकार`, :695), and a
/// struct RETURNED from the entry. In the corpus that routine faults at
/// `addr: 8` — the `विस्तार` field, the second of `अर्थप्रकार`'s five.
///
/// **Here it answers `Some(8)`.** So the fault is NOT in the shape, and this is a
/// recorded NEGATIVE: whatever breaks artha is something a single-module fixture
/// cannot express. The header above already names the likeliest candidate as a
/// gap — a CROSS-MODULE struct — and artha's image is three module objects where
/// every fixture in this file is one.
///
/// **WHY IT IS KEPT UNASSERTED, which the margin above now has a rule for:** an
/// unasserted probe cannot go green, so it will never announce that artha started
/// working. It is here to stop the next reader re-deriving a dead mechanism, not
/// to guard anything. Assert it only alongside a fixture that FAULTS, so the pair
/// can tell a fix from a coincidence.
/// # OUT OF SCOPE — AND IT IS A RECORDED NEGATIVE, WHICH IS A RESULT
///
/// **This one is not awaiting a prediction; it has already answered.** The shape
/// reproduces clean in a single module while the corpus routine faults at
/// `addr: 8`. **So the fault is NOT in the shape**, and whatever breaks `artha`
/// cannot be expressed by a single-module fixture at all.
///
/// A negative that names what it rules out is worth more than a pending probe:
/// it removed the entry's shape from the candidate list and left the
/// cross-module case as the survivor. **Promoting it to an assertion would pin a
/// clean run that was never in doubt** — the value is in the elimination, not in
/// the green.
#[test]
fn probe_artha_entry_shape() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "संरचना धारकः आरभ्य\n",
        "    प्रथमम् ॱॱ न६४ ऽ\n",
        "    द्वितीयम् ॱॱ न६४ ऽ\n",
        "    तृतीयम् ॱॱ न६४\n",
        "समाप्तम् ।\n",
        "वृत्तिः विषम् ददाति धारकः आदि\n",
        "    चरः फलम् ॱॱ धारकः भवति ० ।\n",
        "    फलम् ॱ द्वितीयम् भवति ५ ।\n",
        "    प्रत्यागमनम् फलम् ।\n",
        "इति\n",
        "वृत्तिः योजनम् आदाय ध ॱॱ धारकः ददाति न६४ आदि\n",
        "    प्रत्यागमनम् ध ॱ द्वितीयम् ।\n",
        "इति\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः क ॱॱ धारकः भवति विषम् ।\n",
        "    क ॱ प्रथमम् भवति १ ।\n",
        "    क ॱ द्वितीयम् भवति ८ ।\n",
        "    चरः फ ॱॱ धारकः भवति विषम् ।\n",
        "    फ ॱ प्रथमम् भवति २ ।\n",
        "    फ ॱ तृतीयम् भवति योजनम् क ।\n",
        "    प्रत्यागमनम् फ ॱ तृतीयम् ।\n",
        "इति\n",
    );
    println!(
        "METRIC t1_storage_witness_probe_artha_entry {}",
        compile_and_run(src, "परीक्षा", "मुख्यम्")
    );
}

/// **THROWAWAY — A CROSS-MODULE RECORD, THE ONE GAP THE HEADER NAMES.**
///
/// Every other fixture here is one module. `artha.t1`'s image is three objects,
/// and after its entry's shape reproduced clean, a cross-module struct is the
/// surviving candidate. Two sources compiled through ONE `Front` so the second
/// sees the first's declarations, then three objects linked by construction —
/// no closure walk, because the dependency is known rather than discovered.
///
/// Answers a `Result` rather than panicking: if `Front` will not accumulate two
/// modules, the stage that refuses is the finding.
///
/// **IT REFUSED, AND THE STAGE IS THE ANSWER:**
///
/// ```text
///   B resolve: `अन्यत्ॱविषम्` at line 3 has no declaration
/// ```
///
/// **`Front` IS PER-SOURCE BY CONSTRUCTION.** `chain.rs:265` opens every
/// `resolve()` with `अर्थॱनिर्णायकारम्भः` — resolver START — and overwrites
/// `self.resolver`; `typecheck()` at `:307` zeroes the checker "FOR THIS SOURCE".
/// So the second module is resolved against a resolver that has never seen the
/// first, and no ordering of the calls changes that.
///
/// **THIS DOES NOT MEAN THE COMPILER CANNOT DO CROSS-MODULE** — the corpus plainly
/// does, and `chain.rs:606` is how: cross-module names are minted into a SYNTHETIC
/// band from `2_000_000`, keyed by `(module, name)`, and answered by the LINKER
/// rather than by the resolver. That is why a two-object link with no closure
/// leaves `अर्थसंज्ञा२०००००१…` unresolved.
///
/// So this fixture asks its question the wrong way round and is kept as the record
/// of that, not as a gap in the compiler. **A cross-module witness has to supply
/// the closure, which is the census's job and not this file's** — and until one
/// exists, the header's cross-module bullet is UNTESTABLE here rather than merely
/// untested.
/// # OUT OF SCOPE — THE NAMES ARE ANSWERED BY THE LINKER, NOT THE RESOLVER
///
/// **Cross-module names are minted into a SYNTHETIC BAND** — `chain.rs:606`,
/// `let mut next_synth = 2_000_000usize` — and resolved when objects are linked,
/// not when a module is compiled. The storage model predicts what a declaration
/// is given and what address an access computes **within one compilation**; a
/// symbol that does not exist until link time is outside that entirely.
///
/// **Do not add a linker stage to the model to make this predictable.** The
/// model's subject is storage, and a model that reaches past its subject
/// produces predictions that read exactly like the ones it is entitled to
/// make.
#[test]
fn probe_a_cross_module_record() {
    let src_a = concat!(
        "मण्डलम् अन्यत् ॥\n",
        "सार्वजनिक संरचना धारकः आरभ्य\n",
        "    प्रथमम् ॱॱ न६४ ऽ\n",
        "    द्वितीयम् ॱॱ न६४\n",
        "समाप्तम् ।\n",
        "सार्वजनिक वृत्तिः विषम् ददाति धारकः आदि\n",
        "    चरः फलम् ॱॱ धारकः भवति ० ।\n",
        "    फलम् ॱ द्वितीयम् भवति ५ ।\n",
        "    प्रत्यागमनम् फलम् ।\n",
        "इति\n",
    );
    let src_b = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः धा ॱॱ अन्यत्ॱधारकः भवति अन्यत्ॱविषम् ।\n",
        "    धा ॱ द्वितीयम् भवति २१ ।\n",
        "    प्रत्यागमनम् धा ॱ द्वितीयम् ।\n",
        "इति\n",
    );

    let run = || -> Result<String, String> {
        let mut front = Front::load(&spec_root()).map_err(|e| format!("front: {e}"))?;
        for (which, src) in [("A", src_a), ("B", src_b)] {
            front.lex(src).map_err(|e| format!("{which} lex: {e}"))?;
            front.parse().map_err(|e| format!("{which} parse: {e}"))?;
            front
                .resolve()
                .map_err(|e| format!("{which} resolve: {e}"))?;
            front
                .typecheck()
                .map_err(|e| format!("{which} typecheck: {e}"))?;
            front.build_ir().map_err(|e| format!("{which} ir: {e}"))?;
        }
        let a = front
            .module("अन्यत्", None)
            .map_err(|e| format!("module A: {e}"))?;
        let b = front
            .module("परीक्षा", Some("मुख्यम्"))
            .map_err(|e| format!("module B: {e}"))?;
        let entry_label = b
            .entry
            .map(|s| riscv64::routine_label(&b.names, s))
            .transpose()
            .map_err(|r| format!("entry label: {r:?}"))?
            .ok_or_else(|| "module B names no entry".to_string())?;
        let text_a = riscv64::emit_module(&a).map_err(|r| format!("emit A: {r:?}"))?;
        let text_b = riscv64::emit_module(&b).map_err(|r| format!("emit B: {r:?}"))?;
        let records = riscv64::module_allocates(&a) || riscv64::module_allocates(&b);
        let startup = riscv64::emit_startup_object_with_records(Some(&entry_label), records);
        let read = |bytes: &[u8], what: &str| {
            vastu::read(bytes).ok_or_else(|| format!("the {what} object does not read back"))
        };
        let objects = [
            read(&assemble(&startup, "आरम्भ"), "startup")?,
            read(&assemble(&text_b, "परीक्षा"), "B")?,
            read(&assemble(&text_a, "अन्यत्"), "A")?,
        ];
        let elf = link_objects(&objects, LOAD_ADDRESS)
            .map_err(|es| format!("link: {}", es.join("; ")))?;
        let mut m = Machine::load_elf(&elf, ram_for(&elf)).map_err(|e| format!("load: {e}"))?;
        let mut out: Vec<u8> = Vec::new();
        Ok(format!(
            "{:?} (records={records}, 3 objects)",
            m.run(BUDGET, &mut out)
        ))
    };
    match run() {
        Ok(s) => println!("METRIC t1_storage_witness_probe_cross_module {s}"),
        Err(e) => println!("METRIC t1_storage_witness_probe_cross_module STOPPED {e}"),
    }
}

/// **THE SHAPE AT `+152` — A GLOBAL ARRAY *OF RECORDS*, INITIALISED `०`.**
///
/// `+152` in artha's image falls in `अर्थप्रकारयोजनम्` (artha.t1:318), measured by
/// containment over its object's `.text` symbols and robust for any startup base
/// in `[56, 152)`. It is four lines, and the store at `:320` is an ELEMENT store
/// into a global array whose element type is a RECORD:
///
/// ```text
///   सार्वजनिक चरः अर्थप्रकारकोश ॱॱ अङ्कः अन्तः अर्थप्रकार भवति ० ।   :315
///   अर्थप्रकारसूचकाङ्क भवति अर्थप्रकारसूचकाङ्क योगः १ ।              :319  index -> 1
///   अर्थप्रकारकोश अङ्कः अर्थप्रकारसूचकाङ्क अन्तः भवति प्रकार ।        :320
/// ```
///
/// A null base with element 1 at width 8 gives `0 + 1×8 = 8`, which is artha's
/// `addr: 8` exactly. **The `न६४` variant below is the control**: if both fault,
/// "of records" is not the discriminator and must not be reported as one.
#[test]
fn probe_a_global_array_element_store() {
    let of_records = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "संरचना धारकः आरभ्य\n    प्रथमम् ॱॱ न६४ ऽ\n    द्वितीयम् ॱॱ न६४\nसमाप्तम् ।\n",
        "सार्वजनिक चरः सूचकाङ्कः ॱॱ न६४ भवति ० ।\n",
        "सार्वजनिक चरः कोशः ॱॱ अङ्कः अन्तः धारकः भवति ० ।\n",
        "वृत्तिः योजनम् आदाय प्रकार ॱॱ धारकः ददाति न६४ आदि\n",
        "    सूचकाङ्कः भवति सूचकाङ्कः योगः १ ।\n",
        "    कोशः अङ्कः सूचकाङ्कः अन्तः भवति प्रकार ।\n",
        "    प्रत्यागमनम् सूचकाङ्कः ।\n",
        "इति\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः धा ॱॱ धारकः भवति ० ।\n",
        "    धा ॱ द्वितीयम् भवति ७ ।\n",
        "    प्रत्यागमनम् योजनम् धा ।\n",
        "इति\n",
    );
    let rec_halt = compile_and_run(of_records, "परीक्षा", "मुख्यम्");
    println!("METRIC t1_storage_witness_probe_global_array_of_records {rec_halt}");

    let of_words = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "सार्वजनिक चरः सूचकाङ्कः ॱॱ न६४ भवति ० ।\n",
        "सार्वजनिक चरः कोशः ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    सूचकाङ्कः भवति सूचकाङ्कः योगः १ ।\n",
        "    कोशः अङ्कः सूचकाङ्कः अन्तः भवति ७ ।\n",
        "    प्रत्यागमनम् सूचकाङ्कः ।\n",
        "इति\n",
    );
    let word_halt = compile_and_run(of_words, "परीक्षा", "मुख्यम्");
    println!("METRIC t1_storage_witness_probe_global_array_of_words {word_halt}");

    // ASSERTED AS A REPRODUCTION, WHICH MEANS **A RED HERE IS GOOD NEWS**: it says
    // a slice declared `भवति ०` has stopped having a null base, and whoever made
    // that true should promote these two to assert the VALUE instead.
    //
    // AND IT IS DELIBERATE THAT THIS AND `paradigm_encode`'s PIN BOTH MOVE ON THE
    // SAME FIX — the pair is a cross-check, not a duplicated count. The pin says
    // which SOURCES reach the machine; these fixtures say which SHAPE faults. If
    // these go red and the pin does not move, the fix works on a fixture and not
    // on the corpus. If the pin moves and these stay red, a source recovered for
    // some other reason and the mechanism is still live. **Only both moving
    // together means what the pin alone would be assumed to mean**, and no single
    // place could tell you which of the three happened.
    //
    // Asserted rather than merely printed on purpose. An unasserted probe cannot
    // go green, so it reports a defect forever and never reports its repair —
    // the one-way ratchet the first-field margin above records. A reproduction
    // is exactly the case that earns an assertion, because its partner condition
    // (the fix) is what makes it fire.
    // **PROMOTED FROM FAULT TO VALUE — `W-293` LANDED AND THE RED WAS THE GOOD
    // NEWS.** Until this edit both halts were `BadAccess { addr: 8 }`, `० + १ × ८`
    // through a null base, and this asserted exactly that so a fix would turn it
    // red. It did. The margin above told the next reader to promote the fixture to
    // assert the VALUE, and this is that promotion — kept rather than rewritten so
    // the ratchet's two positions are both visible.
    //
    // `१` is the index the fixture returns, not the element: both routines answer
    // `सूचकाङ्कः` after storing through the base. The record and the `न६४` variant
    // both answering it is what says the element TYPE is irrelevant — which was
    // the point of running two.
    assert!(
        rec_halt.contains("Some(1)") && word_halt.contains("Some(1)"),
        "a global array must allocate its storage and round-trip an element write; \
         halts were {rec_halt} and {word_halt}. Before `W-293` both faulted at \
         addr 8 — `० + १ × ८` through a null base, the same pair the census \
         reports for `artha.t1` at pc 2147483800. A fault here means a global \
         array has lost its storage again; `--placements` records why three of \
         the four candidate fixes cannot give it any."
    );
}

/// **THE LOCAL-SLICE HALF: `अङ्कः अन्तः अ८ भवति ०`, element store at index ०.**
///
/// `encode.t1:3432`'s `प्रकारविभाजकः` — which holds encode's faulting offset — is
/// three lines: a LOCAL octet slice declared `भवति ०`, then element stores at ०,
/// १, २. Index ० at width 1 predicts `addr: 0`, which is encode's addr.
///
/// **IT WAS WRITTEN AS A CONTROL FOR SCOPE AND IT IS NOT ONE — CORRECTED HERE
/// RATHER THAN REWORDED, BECAUSE THE WRONG READING LANDED.** This fixture faults,
/// and I read that as showing "global" was not the discriminator, concluding that
/// any slice declared `भवति ०` has a null base. `probe_slice_scope_against_element_type`
/// below is the grid that should have come first:
///
/// ```text
///                LOCAL              GLOBAL
///   न६४          works              BadAccess addr 8
///   अ८           BadAccess addr 0   BadAccess addr 0
///   record       works              BadAccess addr 8
/// ```
///
/// **Globals fault for every element type; locally only `अ८` does.** So "global"
/// IS a discriminator, and this fixture is an instance of a SECOND, INDEPENDENT
/// defect that presents the same symptom — a bad access at a small address.
///
/// > **A control must share the CAUSE, not merely the SYMPTOM.** Where two defects
/// > produce one symptom, an instance of the second is not a counterexample to
/// > the first.
///
/// And the refutation was already in this file: `an_array_local_takes_an_element_write_and_reads_it_back`
/// is a LOCAL `न६४` slice declared `भवति ०` storing element १, green throughout.
/// A claim contradicted by a passing test in the same file.
///
/// What this fixture DOES witness, which is worth keeping: a local `अ८` slice has
/// no storage, and the index does not enter the address — idx ० and idx १ both
/// fault at 0. It is `encode.t1`'s `प्रकारविभाजकः` and `samyojana.t1`'s
/// `संस्कारोपपदम्`, which are local `अ८` slices, not globals.
#[test]
fn probe_a_local_slice_element_store() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः पाठ्यम् ॱॱ अङ्कः अन्तः अ८ भवति ० ।\n",
        "    पाठ्यम् अङ्कः ० अन्तः भवति ७ ।\n",
        "    प्रत्यागमनम् पाठ्यम् अङ्कः ० अन्तः ।\n",
        "इति\n",
    );
    let halt = compile_and_run(src, "परीक्षा", "मुख्यम्");
    println!("METRIC t1_storage_witness_probe_local_slice_store {halt}");
    // **PROMOTED `W-294`, EXACTLY AS THE OLD ASSERTION'S OWN MESSAGE INSTRUCTED:
    // "A red here means the defect is fixed — promote this to assert the value."**
    // It went red the moment the narrow lowering landed, which is the whole
    // reason a recorded reproduction is asserted rather than printed.
    //
    // AND THE FIXTURE HAD TO CHANGE TO DESERVE THE PROMOTION. It used to end
    // `प्रत्यागमनम् ०` — a LITERAL — so it could not tell a store that worked from
    // a store that had been elided; both answer `Some(0)`. It now READS THE
    // ELEMENT BACK, so the value is evidence. Measured `Some(7)`.
    assert!(
        halt.contains("Some(7)"),
        "a LOCAL slice of `अ८` must store element ० and read it back; the \
         machine halted {halt}. The element is one octet, so this exercises \
         `आहारःॱअ८` and a stride of १ — a stride of ८ would read past the \
         element and a bare `आहारः` would read eight octets out of one. \
         Before `W-294` this faulted at addr 0 because `ir.t1` refused the \
         narrow width outright. THE STORAGE WAS THERE ALL ALONG and the \
         lowering was what refused — this fixture's old margin said a local \
         `अ८` slice HAS NO STORAGE, and that was the wrong diagnosis of a real \
         symptom: the sibling `न६४` fixture allocated and ran throughout."
    );
}

/// **A NARROW INDEX THROUGH A PARAMETER — THE SHAPE THE CORPUS ACTUALLY HAS.**
///
/// Both fixtures below declare their run with `चरः`: a LOCAL. That is the shape
/// that was easiest to write and it is **not** the shape the corpus mostly uses.
/// Measured by `sansos-30` while sizing an unrelated change: of the `ॱ दैर्घ्य`
/// reads in the corpus, **143 have a PARAMETER base and 38 a true local**. A run
/// reaches a routine as an argument far more often than it is declared in one.
///
/// The provenance differs and the lowering does not care — the base is a VALUE
/// wherever it came from, and `base + idx × width` is the same arithmetic. **That
/// is exactly the kind of "should be fine" this file exists to stop me asserting.**
/// A parameter's pointer was produced by the CALLER, so it crosses a call boundary
/// between the allocation and the access, and nothing else in this file tests a
/// narrow access across one.
#[test]
fn a_narrow_run_indexes_through_a_parameter() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "वृत्तिः पठ् आदाय मूल ॱॱ अङ्कः अन्तः अ८ ददाति न६४ आदि\n",
        "    प्रत्यागमनम् मूल अङ्कः १ अन्तः ।\n",
        "इति\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः प ॱॱ अङ्कः अन्तः अ८ भवति ० ।\n",
        "    प अङ्कः ० अन्तः भवति ४ ।\n",
        "    प अङ्कः १ अन्तः भवति ९ ।\n",
        "    प्रत्यागमनम् पठ् प ।\n",
        "इति\n",
    );
    let halt = compile_and_run(src, "परीक्षा", "मुख्यम्");
    println!("METRIC t1_storage_witness_narrow_param_index {halt}");
    // ELEMENT १, NOT ०, AND WRITTEN IN THE CALLER — element ० would round-trip at
    // a base the callee got wrong by any constant, since `base + 0` hides an
    // offset error. Reading element १ across the call is what makes the base and
    // the stride both load-bearing.
    assert!(
        halt.contains("Some(9)"),
        "a narrow run passed as a PARAMETER must index at stride 1 in the callee; \
         the machine halted {halt}. The caller writes ४ at element ० and ९ at \
         element १, the callee reads element १. This is the corpus's dominant \
         shape — a run reaches a routine as an argument far more often than it \
         is declared in one — and every other narrow fixture here declares its \
         run with `चरः` in the same routine that indexes it."
    );
}

/// **THE OTHER WIDTH THE CORPUS ACTUALLY USES — `अ३२`, TWENTY SITES.**
///
/// `W-294` lowered narrow element access by reading the width instead of
/// refusing it. The fixture above measures `अ८` (609 corpus sites). **This one
/// exists because the width-4 arm would otherwise ship unexecuted**: a census of
/// the corpus's slice element types gives `अ८` 609, `अ३२` **20**, `अ६४`/`न६४` 77,
/// and `अ१६` **zero**. So of the three arms the change adds, one is measured by
/// the fixture above, one is measured here, and one is reachable by no source in
/// the tree.
///
/// **A LOWERING ARM WITH NO FIXTURE IS A CLAIM.** Twenty sites is not a corner
/// case, and `अ३२` is the width where a wrong stride is least visible — four
/// octets still lands inside a word-aligned allocation, so an error reads a
/// neighbouring element rather than faulting. `अ८` faults loudly when wrong;
/// this one would not.
#[test]
fn a_thirty_two_bit_slice_element_round_trips() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः सूची ॱॱ अङ्कः अन्तः अ३२ भवति ० ।\n",
        "    सूची अङ्कः ० अन्तः भवति ५ ।\n",
        "    सूची अङ्कः १ अन्तः भवति ६ ।\n",
        "    प्रत्यागमनम् सूची अङ्कः ० अन्तः योगः सूची अङ्कः १ अन्तः ।\n",
        "इति\n",
    );
    let halt = compile_and_run(src, "परीक्षा", "मुख्यम्");
    println!("METRIC t1_storage_witness_a32_round_trip {halt}");
    // TWO ELEMENTS, NOT ONE, AND THE SUM RATHER THAN EITHER — a single element
    // round-trips at any stride, including a wrong one, because the write and
    // the read share it. Element १ at stride 4 is `base+4`; at the old stride of
    // 8 it would be `base+8`, and both are inside the allocation. Only writing
    // BOTH and summing can tell those apart: a stride error makes element १
    // overlap or miss element ०, and ५+६ stops being ११.
    assert!(
        halt.contains("Some(11)"),
        "a local `अङ्कः अन्तः अ३२` must hold TWO distinct elements at stride 4 \
         and sum them to ११; the machine halted {halt}. This is the width the \
         corpus uses 20 times and the one where a wrong stride is quietest — \
         four octets still lands inside a word-aligned allocation, so a stride \
         error reads a neighbour instead of faulting."
    );
}

/// **THE STRIDE OF A STRUCT-ELEMENT ARRAY — does element १ overlap element ०?**
///
/// The field path multiplies an ordinal by a literal `८` with no `विस्तार` check
/// on it. For an array whose element is a RECORD that is only right if the
/// element is one word: `बिन्दुः` is two, so element १ wants `+16`, and a stride
/// of 8 would put `element १ field क` exactly on `element ० field ख`.
///
/// **The subject was established before this was written**: 46 corpus arrays have
/// a struct element type, and their accesses are 12 at literal ०, 3 at a literal
/// nonzero, and **298 by a VARIABLE index** — so the corpus does index past
/// element ०. The defect is masked rather than unreachable: a slice declared
/// `भवति ०` faults on its FIRST access, so no run gets far enough to observe a
/// wrong stride. An array as a record FIELD is the one shape that allocates.
///
/// ५ would mean the stride is the struct's size and the question is RETIRED.
/// ७ would mean element १ aliased element ०.
///
/// **NEITHER: IT ANSWERS `BadAccess { addr: 0 }`, SO THE STRIDE IS STILL
/// UNMEASURED AND THIS FIXTURE IS BLOCKED — on a THIRD defect, not on the two in
/// the grid.** `probe_slice_scope_against_element_type` shows a LOCAL array of
/// records taking a WHOLE-ELEMENT store at index १ without faulting, so that
/// array's base is valid. This fixture's first access is `अङ्कः ० अन्तः ॱ ख` —
/// element THEN field, a compound of `AddrOfIndex` and `AddrOfField` — and it
/// faults at addr 0 on index ०, where a valid base would give `base + 8`.
///
/// **So compound index-then-field addressing produces a null base of its own**,
/// and the stride cannot be observed through it: reaching element १'s field
/// requires the compound access that fails at element ०.
///
/// WHAT UNBLOCKS THIS: a fix to compound index-then-field addressing. NOT the
/// global-storage defect and NOT the local-`अ८` defect — this array is local and
/// its elements are records, which the grid says is the one allocating cell.
///
/// The SUBJECT is established, so this is worth keeping rather than deleting: 46
/// corpus arrays have a struct element type, and their accesses are 12 at literal
/// ०, 3 at a literal nonzero and **298 by a VARIABLE index**. The corpus indexes
/// past element ०, so the stride is masked behind these defects rather than
/// unreachable — a latent defect with a name and a caller, not a hypothetical.
#[test]
fn probe_the_stride_of_a_struct_element_array() {
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "संरचना बिन्दुः आरभ्य\n    क ॱॱ न६४ ऽ\n    ख ॱॱ न६४\nसमाप्तम् ।\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः पङ्क्तिः ॱॱ अङ्कः अन्तः बिन्दुः भवति ० ।\n",
        "    पङ्क्तिः अङ्कः ० अन्तः ॱ ख भवति ५ ।\n",
        "    पङ्क्तिः अङ्कः १ अन्तः ॱ क भवति ७ ।\n",
        "    प्रत्यागमनम् पङ्क्तिः अङ्कः ० अन्तः ॱ ख ।\n",
        "इति\n",
    );
    println!(
        "METRIC t1_storage_witness_probe_struct_array_stride {}",
        compile_and_run(src, "परीक्षा", "मुख्यम्")
    );
}

/// **THE 2×2 THAT MY OWN LANDED CLAIM NEEDED AND DID NOT HAVE.**
///
/// `an_array_local_takes_an_element_write_and_reads_it_back` above is a LOCAL
/// slice of `न६४` declared `भवति ०`, stores element १, and answers `Some(9)`.
/// My local-slice probe is a LOCAL slice of `अ८` declared `भवति ०`, stores
/// element ०, and FAULTS. Both are local, both `भवति ०` — so element type and
/// index were confounded and the claim "a slice declared `भवति ०` has a null
/// base" was too wide. This separates the two axes.
#[test]
fn probe_slice_element_type_against_index() {
    let cases: [(&str, &str, &str); 4] = [
        ("n64_idx0", "न६४", "०"),
        ("n64_idx1", "न६४", "१"),
        ("a8_idx0", "अ८", "०"),
        ("a8_idx1", "अ८", "१"),
    ];
    for (tag, ty, idx) in cases {
        let src = format!(
            "मण्डलम् परीक्षा ॥\n\
             सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
             \u{20}   चरः पङ्क्तिः ॱॱ अङ्कः अन्तः {ty} भवति ० ।\n\
             \u{20}   पङ्क्तिः अङ्कः {idx} अन्तः भवति ९ ।\n\
             \u{20}   प्रत्यागमनम् पङ्क्तिः अङ्कः {idx} अन्तः ।\n\
             इति\n"
        );
        println!(
            "METRIC t1_storage_witness_probe_slice_{tag} {}",
            compile_and_run(&src, "परीक्षा", "मुख्यम्")
        );
    }
}

/// **SCOPE × ELEMENT TYPE, the full grid.** The 2×2 above showed element type
/// matters for a LOCAL slice; this adds the global column and a record element,
/// because a claim about "slices" needs every cell it generalises over.
/// # THE TWO RECORD-ELEMENT CELLS ARE BLOCKED MEASUREMENTS, NOT MODEL GAPS
///
/// `local_rec` and `global_rec` return `UNKNOWN` from `predict()`, because the
/// width check `elem not in ELEMENT_WIDTH` runs BEFORE any defect branch and a
/// record element type has no measured width. **The reason it has none is the
/// part that matters: THE RECORD STRIDE CANNOT BE MEASURED TODAY.**
///
/// The only access that would reveal a stride is compound index-then-field —
/// `पङ्क्तिः अङ्कः १ अन्तः ॱ ख` — and that is DEFECT 3, which faults at 0 before
/// the stride is ever expressed. **So the abstention is not "the model lacks a
/// width"; it is "the width is unmeasurable until defect 3 is fixed", and defect
/// 3 is the named blocker.**
///
/// Adding a width for `बिन्दुः` would make these two cells predictable and the
/// prediction would rest on nothing measured. Two abstentions with a named
/// blocker are worth more than two guesses that look like the four cells beside
/// them — and those four ARE entitled to their predictions, which is exactly why
/// a guess here would be indistinguishable from them.
///
/// **The other four cells stand, and one of them corrected the model**:
/// `global अ८ idx १` was predicted `addr 1` from `predict()` and the machine
/// answered `addr 0`. An index sweep settled it — global न६४ gives 0, 8, 16, 24
/// while global अ८ gives 0, 0, 0, 0 from a DIFFERENT pc — so defect 2 is about
/// the `अ८` access path, is scope-INDEPENDENT, and DOMINATES defect 1. Fixed at
/// `d9a1c082`.
#[test]
fn probe_slice_scope_against_element_type() {
    let struct_decl = "संरचना बिन्दुः आरभ्य\n    क ॱॱ न६४ ऽ\n    ख ॱॱ न६४\nसमाप्तम् ।\n";
    // (tag, element type, needs the struct declared, global?)
    let cases: [(&str, &str, bool, bool); 6] = [
        ("local_n64", "न६४", false, false),
        ("local_a8", "अ८", false, false),
        ("local_rec", "बिन्दुः", true, false),
        ("global_n64", "न६४", false, true),
        ("global_a8", "अ८", false, true),
        ("global_rec", "बिन्दुः", true, true),
    ];
    for (tag, ty, needs_struct, global) in cases {
        let decl = if needs_struct { struct_decl } else { "" };
        let src = if global {
            format!(
                "मण्डलम् परीक्षा ॥\n{decl}\
                 सार्वजनिक चरः पङ्क्तिः ॱॱ अङ्कः अन्तः {ty} भवति ० ।\n\
                 सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
                 \u{20}   पङ्क्तिः अङ्कः १ अन्तः भवति ९ ।\n\
                 \u{20}   प्रत्यागमनम् ० ।\n\
                 इति\n"
            )
        } else {
            format!(
                "मण्डलम् परीक्षा ॥\n{decl}\
                 सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n\
                 \u{20}   चरः पङ्क्तिः ॱॱ अङ्कः अन्तः {ty} भवति ० ।\n\
                 \u{20}   पङ्क्तिः अङ्कः १ अन्तः भवति ९ ।\n\
                 \u{20}   प्रत्यागमनम् ० ।\n\
                 इति\n"
            )
        };
        println!(
            "METRIC t1_storage_witness_probe_grid_{tag} {}",
            compile_and_run(&src, "परीक्षा", "मुख्यम्")
        );
    }
}

/// **THE THREE NAMES NO DECLARATION SPELLS — AND THE REFUSAL THAT MUST SURVIVE
/// NAMING THEM.** `W-295`.
///
/// Every fixture above is a round trip: it compiles, links, runs and asserts a
/// number. **None of them can say WHY a link failed**, and the repair this test
/// guards was found by reading two link errors that a halt assertion reports
/// only as "the linker refused". So this one asserts the NAME TABLE directly,
/// before emit, and it is the falsifier for the repair rather than a second
/// witness for the storage model.
///
/// # The three, and where each comes from
///
/// `ir.t1` mints three symbols that no declaration in any source names, so the
/// loops in `chain.rs::module` that read `वृत्तिनामचिह्नककोश` and
/// `वैश्विकनामकोश` cannot reach any of them — **there is no arena entry to
/// miss.** Each has to be spelled by the driver:
///
/// ```text
///   १००००००३  रचनासूचकः     module-less   ir.t1:546   shrinkhala.t1:473
///   १००००००४  रचनाक्षेत्रम्   module-less   ir.t1:549   shrinkhala.t1:474
///   १००००००५  खण्डवृद्धिः     this module   ir.t1:560   shrinkhala.t1:338
/// ```
///
/// The `.t1` driver has spelled all three since `W-284`/`W-285`. The Rust twin
/// spelled none, and the two failures stacked in one pipeline: the first two
/// refused at EMIT (`UnnamedSymbol { symbol: SymbolId(10000003) }`) and the
/// third at LINK (`` `परीक्षासंज्ञा१००००००५` is not defined by any object ``),
/// so fixing the first merely uncovered the second. Seven fixtures above were
/// red on the pair and stayed red on the third.
///
/// **MODULE-LESS IS A CLAIM WITH TEETH.** `routine_label` is `module ⧺ name`, so
/// an empty module half is the difference between the label `रचनासूचकः` — which
/// the startup object defines, once per image — and `परीक्षारचनासूचकः`, which
/// nothing defines. The growth routine is the opposite: it is per-module and
/// MUST carry the module, since two linked modules each define their own.
/// Asserting the pairs would not catch a swap; asserting the LABELS does.
///
/// # The case that must still be REFUSED
///
/// The repair inserts names for symbols it can identify. It must not become a
/// blanket "name anything unknown", because `UnnamedSymbol` is what turns an IR
/// referencing a symbol from nowhere into a refusal instead of a label that
/// links against the wrong thing — the exact failure `W-279` recorded, where a
/// resolved symbol was attributed to the wrong module and produced
/// `परीक्षासंज्ञा४` against `अन्यत्संज्ञा४`. So this asserts that a symbol
/// nothing minted STILL refuses, and refuses by that name.
#[test]
fn the_emitters_own_three_names_and_still_refuse_a_fourth() {
    // ITS OWN LOADER, not `compile_and_run`'s: that helper emits, links and
    // runs, and every one of those stages is downstream of the table under
    // test. Sharing it would make this test unable to fail before them.
    let src = concat!(
        "मण्डलम् परीक्षा ॥\n",
        "\n",
        "संरचना धारकः आरभ्य\n",
        "    प्रथमम् ॱॱ न६४ ऽ\n",
        "    द्वितीयम् ॱॱ न६४\n",
        "समाप्तम् ।\n",
        "\n",
        "सार्वजनिक वृत्तिः मुख्यम् ददाति न६४ आदि\n",
        "    चरः वस्तु ॱॱ धारकः भवति ० ।\n",
        "    वस्तु ॱ प्रथमम् भवति ५ ।\n",
        "    चरः पङ्क्तिः ॱॱ अङ्कः अन्तः न६४ भवति ० ।\n",
        "    पङ्क्तिः अङ्कः १ अन्तः भवति २ ।\n",
        "    प्रत्यागमनम् वस्तु ॱ प्रथमम् योगः पङ्क्तिः अङ्कः १ अन्तः ।\n",
        "इति\n",
    );

    let mut front = Front::load(&spec_root()).expect("the front end loads");
    front.lex(src).expect("पदविभाग lexes the fixture");
    front.parse().expect("व्याकर parses it");
    front.resolve().expect("अर्थ resolves every name");
    front.typecheck().expect("अर्थ typechecks every statement");
    front.build_ir().expect("मध्यरूप builds IR");
    let module = front
        .module("परीक्षा", Some("मुख्यम्"))
        .expect("the module reads back from मध्यरूप's arenas");

    // THE LABEL AND NOT THE PAIR, for the reason the margin gives: a module
    // half that leaked into the first two is invisible to a pair comparison
    // written from the same wrong assumption.
    let label = |sym: SymbolId| {
        riscv64::routine_label(&module.names, sym)
            .unwrap_or_else(|e| panic!("symbol {sym:?} has no label: {e}"))
    };
    assert_eq!(
        label(riscv64::RECORD_CURSOR_SYMBOL),
        "रचनासूचकः",
        "the record cursor is MODULE-LESS: the startup object defines `रचनासूचकः` \
         once per image, so a module half here labels a reference nothing defines"
    );
    assert_eq!(
        label(riscv64::RECORD_REGION_SYMBOL),
        "रचनाक्षेत्रम्",
        "the record region is module-less for the same reason as its cursor"
    );

    // THE GROWTH ROUTINE, BY SEARCH AND NOT BY ITS NUMBER, so this still holds
    // the day `ir.t1` renumbers `वृद्धिवृत्तिसंज्ञा`: what must be true is that
    // SOME symbol carries the pair, and that `chain.rs` read it from the arena
    // rather than inventing `वृत्तिN` and `1_000_000 + i`.
    let growth: Vec<&SymbolId> = module
        .names
        .iter()
        .filter(|(_, (_, n))| n == "खण्डवृद्धिः")
        .map(|(s, _)| s)
        .collect();
    assert_eq!(
        growth.len(),
        1,
        "a source with an indexed store synthesises EXACTLY ONE `खण्डवृद्धिः`; \
         the table holds {}: {:?}",
        growth.len(),
        growth
    );
    assert_eq!(
        label(*growth[0]),
        "परीक्षाखण्डवृद्धिः",
        "the growth routine is PER-MODULE and carries the module, unlike the \
         cursor and the region — two linked modules each define their own, and \
         a module-less label here is the linker's `defined by more than one object`"
    );
    assert!(
        module.functions.iter().any(|f| f.name == *growth[0]),
        "the symbol `खण्डवृद्धिः` is named by must be the one its DEFINITION \
         carries: the call sites `ir.t1:3938` emits hold the arena's `नाम`, and \
         overwriting it here is what left `परीक्षासंज्ञा१००००००५` undefined"
    );

    // AND THE REFUSAL, which is the half of this test that can fail on a repair
    // that works. `9_999_999` is below `ir.t1`'s minted range and above the
    // resolver's, so nothing mints it and nothing may name it.
    let stranger = SymbolId(9_999_999);
    let refused = riscv64::routine_label(&module.names, stranger);
    assert!(
        matches!(refused, Err(riscv64::Refusal::UnnamedSymbol { symbol }) if symbol == stranger),
        "a symbol nothing minted must STILL refuse by name — a blanket fallback \
         here would label it and link it against whatever answered; got {refused:?}"
    );
}
