//! **`W-280` — A CALL THIS ARM ACCEPTS MUST CARRY A SYMBOL, WHICHEVER SHAPE NAMED
//! THE CALLEE.**
//!
//! `ir.t1`'s `अभिव्यञ्जकरचना` reaches a cross-module callee through two trees,
//! because the corpus writes one two ways and `व्याकर` keeps the spelling:
//!
//! ```text
//!   सङ्केतनॱक्षेत्रारम्भः     ONE token   -> नाम node, split on the mark by the external arm
//!   सङ्केतन ॱ क्षेत्रारम्भः   THREE       -> नाम node, member token folded into दक्षिणसूचकाङ्क
//! ```
//!
//! The FOLDED form (W-228 (b), `parse.t1:945`) was ACCEPTED and never minted: the
//! arm wrote `स्वीकृतम् भवति सत्यम्` and left `आह्वेयसंज्ञा` at the ० of its own
//! declaration, so `आह्वानाज्ञायोजनम्` appended a Call with `संज्ञा ०`.
//! `यन्त्रनामान्वेषणम् ०` misses — ० is "no symbol", not a symbol — and
//! `यन्त्राह्वानोत्सर्जनम्` refused the whole module with `यन्त्रानामसंज्ञानिषेधभेद`.
//!
//! **MEASURED BEFORE THE FIX, over all 21 corpus sources, one fresh interpreter
//! each: 128 Calls emitted with `संज्ञा ०` — `vishlesana` 51, `vakyavibhaga` 32,
//! `sanskrit_text` 31, `samyojana` 14 — and ZERO in the other seventeen.** Those
//! four are exactly the four the 2026-09-17 hop census found at hop 0, so the
//! census and this arm are one fact and not two.
//!
//! # Why the census below is over the corpus and not over a fixture
//!
//! A fixture can only exercise the shapes whoever wrote it thought of. The
//! property — **no accepted call leaves this arm unnamed** — is about every shape
//! the corpus actually contains, and the corpus is the only statement of that.
//! The fixtures below are here for the two things a corpus census cannot do: pin
//! the DEDUPLICATION (one symbol per callee, not per site) and hold the case that
//! must still be REFUSED.
//!
//! # What this file does NOT drive
//!
//! It stops at built IR — `शृङ्खलाॱरचना` — and never emits. Driving to text is
//! `crates/yantra/tests/t1_hop0_site.rs`'s job and costs 32 s where this costs
//! about 1 s a source; running both is not redundancy, because this one names the
//! INSTRUCTION and that one names the REFUSAL, and a Call with `संज्ञा ०` that
//! some later stage happened to tolerate would pass there and fail here.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn octets(b: &[u8]) -> Value {
    Value::Octets(Octets::new(b))
}

/// Every `.t1` in the corpus directory, off DISK and NOT trimmed to a Rust-side
/// roster: a source that breaks the load is a real difference and must surface.
fn corpus_files() -> Vec<(String, String)> {
    let dir = corpus_dir();
    let mut paths: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{} is readable: {e}", dir.display()))
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "t1"))
        .collect();
    paths.sort();
    paths
        .iter()
        .map(|p| {
            let name = p
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            let text = std::fs::read_to_string(p)
                .unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()));
            (name, text)
        })
        .collect()
}

/// **THIS FILE'S OWN LOADER.** The whole corpus, because `शृङ्खला` drives every
/// stage and the interpreter's arity table is global — a subset here would make
/// the census a statement about a different program than the one that ships.
fn load(files: &[(String, String)]) -> Interpreter {
    let refs: Vec<(&str, &str)> = files
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();
    Interpreter::load(&refs, &spec_root())
        .unwrap_or_else(|e| panic!("corpus load of {} disk sources: {e:?}", refs.len()))
}

fn arena(it: &Interpreter, name: &str) -> Rc<RefCell<Vec<Value>>> {
    match it.global(name) {
        Some(Value::Arena(a)) => Rc::clone(a),
        other => panic!("`{name}` is an arena of the loaded corpus, not {other:?}"),
    }
}

fn global_int(it: &Interpreter, name: &str) -> i128 {
    it.global(name)
        .and_then(Value::as_int)
        .unwrap_or_else(|| panic!("`{name}` is a numeric global of the loaded corpus"))
}

fn field(v: &Value, key: &str) -> Value {
    match v {
        Value::Record(r) => r.borrow().get(key).cloned().unwrap_or(Value::Nil),
        _ => Value::Nil,
    }
}

/// A record field as a number. `-1` and not `0`: `०` is a MEANING here — "no
/// symbol" — and a missing field read as ० would be indistinguishable from the
/// defect this file is about.
fn field_int(v: &Value, key: &str) -> i128 {
    field(v, key).as_int().unwrap_or(-1)
}

fn octet_text(v: &Value) -> String {
    v.octets()
        .map(|o| String::from_utf8_lossy(o.as_slice()).into_owned())
        .unwrap_or_default()
}

/// Drive one source through lex, parse, resolve, typecheck and IR on a FRESH
/// interpreter, and answer it. Fresh per source because `मध्यरूप`'s arenas are
/// module-level globals reset per program: two sources on one load would have the
/// second read the first's cursor and the census would measure one source twice.
fn built_ir(files: &[(String, String)], src: &str) -> Interpreter {
    try_built_ir(files, src).unwrap_or_else(|| {
        panic!(
            "the source parsed to NO declarations, so every count taken from it would be \
             ० for a reason that has nothing to do with call symbols"
        )
    })
}

/// The same, answering `None` for a source that declares nothing at all. A
/// separate answer and not a ० count: `lib.t1` is fifteen lines of comment with no
/// `मण्डलम्` and no routine, and "declared nothing" and "declared things and built
/// no unnamed call" are different facts that must not be read off one number.
fn try_built_ir(files: &[(String, String)], src: &str) -> Option<Interpreter> {
    let mut it = load(files);
    let decls = it
        .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 80_000_000_000)
        .unwrap_or_else(|e| panic!("शृङ्खलाॱपठनम्: {e:?}"))
        .as_int()
        .unwrap_or(0);
    if decls <= 0 {
        return None;
    }
    it.call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 80_000_000_000)
        .unwrap_or_else(|e| panic!("शृङ्खलाॱनिर्णयः: {e:?}"));
    it.call("शृङ्खलाॱरचना", vec![Value::Int(decls)], 80_000_000_000)
        .unwrap_or_else(|e| panic!("शृङ्खलाॱरचना: {e:?}"));
    Some(it)
}

/// Every Call instruction built for the source in hand, as `(index, symbol)`.
/// Walked to the CURSOR and not to the arena's length: `आज्ञासूचकाङ्क` is reset per
/// program and the arena is not truncated, so a `len()` bound would read a
/// previous program's instructions — the same one-line defect `yantrotsarjana.t1`
/// documents at every walk over a shared arena.
fn calls(it: &Interpreter) -> Vec<(usize, i128)> {
    let kind = global_int(it, "आह्वानाज्ञाभेद");
    let cursor = usize::try_from(global_int(it, "आज्ञासूचकाङ्क")).unwrap_or(0);
    let insts = arena(it, "आज्ञाकोश");
    let insts = insts.borrow();
    (1..=cursor)
        .filter_map(|i| insts.get(i).map(|v| (i, v)))
        .filter(|(_, v)| field_int(v, "भेद") == kind)
        .map(|(i, v)| (i, field_int(v, "संज्ञा")))
        .collect()
}

/// The `(module, routine)` pairs minted for this program, in mint order.
fn minted(it: &Interpreter) -> Vec<(String, String)> {
    let cursor = usize::try_from(global_int(it, "बाह्यसंज्ञासूचकाङ्क")).unwrap_or(0);
    let modules = arena(it, "बाह्यमण्डलकोश");
    let routines = arena(it, "बाह्यवृत्तिकोश");
    let (modules, routines) = (modules.borrow(), routines.borrow());
    (1..=cursor)
        .map(|i| {
            (
                octet_text(modules.get(i).unwrap_or(&Value::Nil)),
                octet_text(routines.get(i).unwrap_or(&Value::Nil)),
            )
        })
        .collect()
}

/// **THE क्षेत्र-SHAPED CALLEES OF A BUILT SOURCE**, deduplicated to one entry per
/// callee node and answered as `(node, base kind, base symbol)`.
///
/// This exists so the census below has THREE answers where it had two. "Zero
/// Calls with `संज्ञा ०`" is consistent with two different worlds: no source
/// reaches `ir.t1`'s `क्षेत्र` arm at all, or sources reach it and every one is
/// REFUSED there. Those want different next moves, and a count of unnamed calls
/// cannot tell them apart — an instrument with two states where the truth has
/// three hides its own breakage.
///
/// The walk follows `वाम` through the CURRIED `आह्वान` chain the way `ir.t1` does:
/// `f a b` is `आह्वान(आह्वान(f, a), b)`, ONE call with two arguments, so the
/// callee is the first node down that spine which is not itself an `आह्वान`.
/// Deduplicated by that node, or a two-argument call would count twice.
fn kshetra_callees(it: &Interpreter) -> Vec<(usize, i128, i128)> {
    let naam = global_int(it, "नामाभिव्यञ्जकभेद");
    let kshetra = global_int(it, "क्षेत्राभिव्यञ्जकभेद");
    let ahvana = global_int(it, "आह्वानाभिव्यञ्जकभेद");
    let cursor = usize::try_from(global_int(it, "अभिव्यञ्जकसूचकाङ्क")).unwrap_or(0);
    let nodes = arena(it, "अभिव्यञ्जककोश");
    let nodes = nodes.borrow();
    // `-1` for a missing node and not `0`: ० is a live index meaning NO CHILD, and
    // a read past the cursor must not be reported as one.
    let kind = |i: usize| nodes.get(i).map(|v| field_int(v, "भेद")).unwrap_or(-1);
    let left = |i: usize| {
        usize::try_from(nodes.get(i).map(|v| field_int(v, "वामसूचकाङ्क")).unwrap_or(0)).unwrap_or(0)
    };
    let mut found: Vec<(usize, i128, i128)> = Vec::new();
    for i in 1..=cursor {
        if kind(i) != ahvana {
            continue;
        }
        let mut callee = left(i);
        while kind(callee) == ahvana {
            callee = left(callee);
        }
        if kind(callee) != kshetra {
            continue;
        }
        let base = left(callee);
        let symbol = if kind(base) == naam {
            field_int(nodes.get(base).unwrap_or(&Value::Nil), "संज्ञासूचकाङ्क")
        } else {
            -1
        };
        if !found.iter().any(|(n, _, _)| *n == callee) {
            found.push((callee, kind(base), symbol));
        }
    }
    found
}

/// **THE CENSUS.** Not one corpus source may build a Call this arm accepted and
/// could not name. Before `W-280` this stood at 128 across four sources.
#[test]
fn no_corpus_source_builds_a_call_with_no_symbol() {
    let files = corpus_files();
    let mut unnamed: Vec<(String, usize)> = Vec::new();
    let mut declareless: Vec<String> = Vec::new();
    let mut total_calls = 0usize;
    let mut total_kshetra = 0usize;
    for (name, src) in &files {
        let Some(it) = try_built_ir(&files, src) else {
            // NOT A SKIP — A NAMED OUTCOME. A source that declares nothing builds no
            // calls, and that is only benign if it also WRITES nothing: a real source
            // that had silently stopped parsing would otherwise leave the census here
            // reading "0 unnamed" and looking like a pass.
            println!("  {name:20} declares nothing — comment only, no call to census");
            assert!(
                !src.contains("वृत्तिः"),
                "{name} parsed to NO declarations and yet writes `वृत्तिः` — it did not \
                 decline to declare, it failed to parse, and every count in this census \
                 would be ० for that reason instead of this file's"
            );
            declareless.push(name.clone());
            continue;
        };
        let built = calls(&it);
        let zero = built.iter().filter(|(_, s)| *s == 0).count();
        let shaped = kshetra_callees(&it);
        total_calls += built.len();
        total_kshetra += shaped.len();
        println!(
            "  {name:20} calls {:>5}  unnamed {zero:>4}  minted {:>4}  क्षेत्र-callees {:>3} {:?}",
            built.len(),
            minted(&it).len(),
            shaped.len(),
            shaped.iter().take(4).collect::<Vec<_>>()
        );
        if zero > 0 {
            unnamed.push((name.clone(), zero));
        }
    }
    println!("METRIC w280_corpus_calls_built {total_calls}");
    // REPORT-ONLY, per the owner ruling of 2026-09-13: this number encodes nothing
    // about correctness and a source that legitimately starts writing the shape
    // must not red a landing. It is here to keep the two worlds above apart —
    // `0` means nothing reaches `ir.t1`'s `क्षेत्र` arm, and any other number means
    // sources reach it and are being refused there, which is a different fact even
    // when `w280_corpus_calls_unnamed` is ० in both.
    println!("METRIC w280_corpus_kshetra_callees {total_kshetra}");
    if total_kshetra > 0 {
        println!(
            "NOTE a corpus source now writes a क्षेत्र-shaped callee ({total_kshetra}); \
             `ir.t1`'s क्षेत्र arm refuses every one, so those calls are ABSENT from the \
             counts above rather than unnamed in them"
        );
    }
    println!(
        "METRIC w280_corpus_sources_censused {}",
        files.len() - declareless.len()
    );
    println!(
        "METRIC w280_corpus_sources_declareless {}",
        declareless.len()
    );
    println!(
        "METRIC w280_corpus_calls_unnamed {}",
        unnamed.iter().map(|(_, n)| n).sum::<usize>()
    );
    assert!(
        unnamed.is_empty(),
        "these sources build Calls with `संज्ञा ०`: {unnamed:?}. `यन्त्रनामान्वेषणम् ०` \
         misses at every one of them, so `यन्त्राह्वानोत्सर्जनम्` refuses the module — \
         which is precisely the hop-0 refusal `W-280` removed."
    );
}

/// A source `व` that calls one imported routine BOTH WAYS. Everything the two
/// shapes must agree about is here and nowhere else: same module, same routine,
/// two spellings.
const BOTH_SPELLINGS: &str = concat!(
    "मण्डलम् व ॥\n",
    "आयातः सङ्केतन ।\n",
    "सार्वजनिक वृत्तिः ग आदाय क ॱॱ न६४ ददाति न६४ आदि\n",
    "    चरः एकम् ॱॱ न६४ भवति सङ्केतनॱक्षेत्रसंख्या क ।\n",
    "    चरः द्वितीयम् ॱॱ न६४ भवति सङ्केतन ॱ क्षेत्रसंख्या क ।\n",
    "    प्रत्यागमनम् एकम् योगः द्वितीयम् ।\n",
    "इति\n",
);

/// **ONE SYMBOL PER CALLEE, NOT PER CALL SITE — AND NOT PER SPELLING.** The two
/// forms above name the same routine, so they must carry the SAME symbol and the
/// mint arena must hold ONE entry. Two entries would link correctly and still be
/// wrong: `chain.rs:658` deduplicates by `(module, routine)`, and a `.t1` side
/// that minted per spelling would drift from the twin by exactly the number of
/// folded sites — invisible to every count that does not compare the two.
#[test]
fn the_two_spellings_of_one_callee_share_one_symbol() {
    let files = corpus_files();
    let it = built_ir(&files, BOTH_SPELLINGS);
    let built = calls(&it);
    let pairs = minted(&it);
    println!("  both spellings: calls {built:?}  minted {pairs:?}");

    assert_eq!(
        built.len(),
        2,
        "the source writes exactly two calls and the arm built {}: the fixture is no \
         longer exercising the two shapes and the agreement below would be vacuous",
        built.len()
    );
    assert!(
        built.iter().all(|(_, s)| *s > 0),
        "a spelling went unnamed: {built:?}"
    );
    assert_eq!(
        built[0].1, built[1].1,
        "`सङ्केतनॱक्षेत्रसंख्या` and `सङ्केतन ॱ क्षेत्रसंख्या` are the same callee and got \
         symbols {} and {} — two numbers for one routine, which links twice and \
         diverges from `chain.rs`'s per-callee numbering.",
        built[0].1, built[1].1
    );
    assert_eq!(
        pairs,
        vec![("सङ्केतन".to_string(), "क्षेत्रसंख्या".to_string())],
        "the mint arena must hold exactly the one pair both spellings name"
    );
}

/// **THE CASE THAT MUST STILL BE REFUSED.** A BARE callee the resolver did not
/// name — no module prefix, no folded member — reaches neither minter, and must
/// leave `स्वीकृतम्` false so the arm refuses it as `अनिर्णीताह्वान` and appends NO
/// instruction. Without this, `W-280` could have been "mint something for every
/// callee", which names an unresolved call with a synthetic symbol and turns a
/// diagnosable refusal into a link error a long way away.
///
/// The two sources differ in ONE token — the callee's name — so that what is
/// being measured is the resolution and not the shape of the program.
#[test]
fn an_unresolved_bare_callee_is_still_refused() {
    let files = corpus_files();
    let resolved = concat!(
        "मण्डलम् व ॥\n",
        "वृत्तिः ख आदाय क ॱॱ न६४ ददाति न६४ आदि\n",
        "    प्रत्यागमनम् क ।\n",
        "इति\n",
        "सार्वजनिक वृत्तिः ग ददाति न६४ आदि\n",
        "    प्रत्यागमनम् ख ७ ।\n",
        "इति\n",
    );
    let unresolved = resolved.replace("प्रत्यागमनम् ख ७ ।", "प्रत्यागमनम् घ ७ ।");
    assert_ne!(unresolved, resolved, "the fixture pair must differ");

    let named = calls(&built_ir(&files, resolved));
    let bare = calls(&built_ir(&files, &unresolved));
    println!("  resolved {named:?}   unresolved {bare:?}");

    assert_eq!(
        named.len(),
        1,
        "the control must build exactly the one call it writes, and built {named:?} — \
         so the comparison below would be between two things neither of which is the \
         subject"
    );
    assert!(
        named[0].1 > 0,
        "the resolved callee itself went unnamed ({named:?}), so the pair says nothing \
         about resolution"
    );
    assert!(
        bare.is_empty(),
        "an UNRESOLVED bare callee built {bare:?}. It must build nothing at all: the arm \
         records `अनिर्णीताह्वान` and returns before appending, and a Call with `संज्ञा ०` \
         here would be refused two modules away with no name to report."
    );
}

/// **THE CASE `W-280` ASKED FOR, AND IT IS NOT A MINT.**
///
/// `ir.t1`'s `क्षेत्र` arm was left ACCEPTING-WITHOUT-MINTING on 2026-09-17 with its
/// margin saying, in as many words, *"THE CASE COMES FIRST — `w280-folded-callee.rs`'s
/// census is where it belongs"*. This is that case, and the census above is the
/// measurement that decided which repair it gets.
///
/// # Why refusal and not a symbol
///
/// The arm was written for `मण्डल ॱ वृत्तिः`, a spaced qualified callee. `W-228` (b)
/// took that shape away from it: `parse.t1:945` folds a spaced qualifier whose head
/// names an IMPORTED module into the head's own नाम node, so the import never
/// becomes a `क्षेत्र` at all. Row C below is that statement's control — it is the
/// spaced form, and it must still build ONE NAMED call through the नाम arm.
///
/// What is left reaching the accept is a base the fold DECLINED: a record, a
/// parameter, a local. T1 has no function values, so none of them names a routine,
/// and the old accept appended a Call with `संज्ञा ०` that `यन्त्राह्वानोत्सर्जनम्`
/// reported as `यन्त्रानामसंज्ञानिषेधभेद` over the WHOLE MODULE — a link error a long
/// way from its cause. Minting instead would have been worse in the same direction:
/// a synthetic `बाह्यसंज्ञा` for a record's field names a label no module exports.
///
/// # THREE ROWS, BECAUSE TWO WOULD REPORT THE MIDDLE ONE WRONG
///
/// ```text
///   A  र ॱ क ७        field applied   builds NO call   अनिर्णीताह्वान  रचना ०
///   B  र ॱ क          field READ      builds NO call   no refusal      रचना १
///   C  सङ्केतन ॱ क्षेत्रसंख्या क   import call   builds ONE NAMED call   no refusal   रचना १
/// ```
///
/// A and B build the same count of Calls — ZERO — and mean opposite things. A count
/// alone cannot separate "refused at the arm" from "was never a call"; the refusal
/// flag can, which is why every row reads it.
#[test]
fn a_kshetra_shaped_callee_is_refused_rather_than_built_unnamed() {
    let files = corpus_files();

    // THIS TEST'S OWN DRIVER, and not `built_ir` above. The shared one discards
    // `शृङ्खलाॱरचना`'s answer, and that answer is half of what row A asserts: a
    // refused build answers ० where a clean one answers the declaration count.
    let drive = |src: &str| -> (Interpreter, i128, bool, i128) {
        let mut it = load(&files);
        let decls = it
            .call("शृङ्खलाॱपठनम्", vec![octets(src.as_bytes())], 80_000_000_000)
            .unwrap_or_else(|e| panic!("शृङ्खलाॱपठनम्: {e:?}"))
            .as_int()
            .unwrap_or(0);
        assert!(decls > 0, "the fixture parsed to NO declarations: {src}");
        it.call("शृङ्खलाॱनिर्णयः", vec![Value::Int(decls)], 80_000_000_000)
            .unwrap_or_else(|e| panic!("शृङ्खलाॱनिर्णयः: {e:?}"));
        let built = it
            .call("शृङ्खलाॱरचना", vec![Value::Int(decls)], 80_000_000_000)
            .unwrap_or_else(|e| panic!("शृङ्खलाॱरचना: {e:?}"))
            .as_int()
            .unwrap_or(-1);
        let refused = matches!(it.global("अनिर्णीताह्वानमस्ति"), Some(Value::Bool(true)));
        let at = it
            .global("अनिर्णीताह्वानचिह्नकाङ्क")
            .and_then(Value::as_int)
            .unwrap_or(-1);
        (it, built, refused, at)
    };

    // The three sources differ only in the LAST statement, so what is measured is
    // the callee's shape and not the shape of the program around it.
    let head = concat!(
        "मण्डलम् व ॥\n",
        "आयातः सङ्केतन ।\n",
        "संरचना स आरभ्य\n",
        "  क ॱॱ न६४\n",
        "समाप्तम् ।\n",
        "सार्वजनिक वृत्तिः ग आदाय र ॱॱ स ददाति न६४ आदि\n",
    );
    let a = format!("{head}    प्रत्यागमनम् र ॱ क ७ ।\nइति\n");
    let b = format!("{head}    प्रत्यागमनम् र ॱ क ।\nइति\n");
    let c = format!("{head}    प्रत्यागमनम् सङ्केतन ॱ क्षेत्रसंख्या ७ ।\nइति\n");

    // ── A — the case. The callee IS a क्षेत्र whose base the resolver placed, which
    //       is exactly the state the old accept read, and it must build nothing.
    let (it_a, built_a, refused_a, at_a) = drive(&a);
    let shaped_a = kshetra_callees(&it_a);
    let calls_a = calls(&it_a);
    let tokens_a = global_int(&it_a, "चिह्नकदैर्घ्य");
    println!(
        "  A  built {built_a}  calls {calls_a:?}  refused {refused_a} at {at_a}/{tokens_a}  क्षेत्र {shaped_a:?}"
    );
    assert_eq!(
        shaped_a.len(),
        1,
        "the fixture must put exactly one क्षेत्र-shaped callee in front of the arm and \
         put {shaped_a:?} — every assertion below would be about a shape that is not \
         the subject"
    );
    assert!(
        shaped_a[0].2 > 0,
        "the क्षेत्र's base carries symbol {} — the old accept was guarded by \
         `आधारः ॱ संज्ञासूचकाङ्क अधिकम् ०`, so a base at ० would not have reached it and \
         this row would pass for the wrong reason",
        shaped_a[0].2
    );
    assert!(
        calls_a.is_empty(),
        "a क्षेत्र-shaped callee built {calls_a:?}. Before 2026-09-18 this was exactly \
         one Call with `संज्ञा ०`, refused two stages away as \
         `यन्त्रानामसंज्ञानिषेधभेद` over the whole module."
    );
    assert!(
        refused_a,
        "no Call was built AND no refusal was recorded — which is the `अपूर्णध्रुवम् १४` \
         stub swallowing the call in silence, not the arm refusing it"
    );
    assert!(
        at_a > 0 && at_a <= tokens_a,
        "the refusal records token {at_a}, outside the source's {tokens_a} tokens: it \
         names no site and reports no better than the module-wide link error it replaced"
    );
    assert_eq!(
        built_a, 0,
        "a refused build must answer ०, and answered {built_a}"
    );

    // ── B — the control that makes A's ZERO mean something. One token shorter: the
    //       same field, READ instead of applied. It is not a call, so it builds none
    //       and must raise NO refusal.
    let (it_b, built_b, refused_b, _) = drive(&b);
    let calls_b = calls(&it_b);
    println!("  B  built {built_b}  calls {calls_b:?}  refused {refused_b}");
    assert!(
        calls_b.is_empty(),
        "a plain field READ built calls {calls_b:?} — it is not an application at all"
    );
    assert!(
        !refused_b,
        "the arm refused a plain field READ, so row A's refusal says nothing about \
         callees and everything about fields"
    );
    assert_eq!(
        built_b, 1,
        "the field-read source must BUILD, or row A's ० is the fixture failing rather \
         than the arm refusing"
    );

    // ── C — the control that proves the repair did not take the shape the arm was
    //       written for. `सङ्केतन ॱ क्षेत्रसंख्या` is the spaced qualified import:
    //       `parse.t1:945` folds it into a नाम, so it must never become a क्षेत्र,
    //       and it must carry a real symbol.
    let (it_c, built_c, refused_c, _) = drive(&c);
    let calls_c = calls(&it_c);
    let shaped_c = kshetra_callees(&it_c);
    println!("  C  built {built_c}  calls {calls_c:?}  refused {refused_c}  क्षेत्र {shaped_c:?}");
    assert!(
        shaped_c.is_empty(),
        "the spaced qualified IMPORT reached the क्षेत्र arm as {shaped_c:?}. The fold at \
         `parse.t1:945` is what keeps that shape out of it, and with the fold gone the \
         refusal above would start eating real cross-module calls."
    );
    assert_eq!(
        calls_c.len(),
        1,
        "the import call must build exactly one Call and built {calls_c:?}"
    );
    assert!(
        calls_c[0].1 > 0,
        "the spaced qualified import went unnamed ({calls_c:?}) — that is `W-280`'s own \
         defect back again"
    );
    assert!(!refused_c, "the import call was refused");
    assert_eq!(built_c, 1, "the import source must build");
}
