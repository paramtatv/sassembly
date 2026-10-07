//! **SAS-011 fix (3) — THE TOKENISER'S CLASS ORDER.** `lex.t1`'s word arm
//! allocated a fresh byte run for EVERY word in order to recognise the two
//! ADR-0018 layout words, and a second one for the token before it.
//!
//! # What it cost, and why a length test fixes it
//!
//! `पदविभाग`'s word arm used to open with
//! `खण्डपाठः मूल पदारम्भः शिरोन्तः` — `खण्डपाठः` allocates a run and copies the
//! word octet by octet in the interpreter — and then, for any word that is not
//! the first, `चिह्नकपाठः` on the PREVIOUS token to ask whether it was `उक्तम्`.
//! Two allocations and two full copies per word, to recognise `विवरम्`
//! (18 octets) and `यतिः` (12 octets) and nothing else.
//!
//! `lex.t1` ITSELF ALREADY MAKES THIS ARGUMENT, two screens further down, at
//! the `समावेशः` loop: *"`चिह्नकपाठः` ALLOCATES a fresh run per call. Calling
//! it on every token to find four is twenty thousand allocations per source —
//! the bloat the owner named. `समावेशः` is seven aksaras, २१ octets; test the
//! LENGTH from the token's own bounds first, and copy only a token that could
//! match."* The word arm is the same shape and had not had the same treatment.
//! So the fix is ORDER, not arithmetic: ask the length — which the arm already
//! holds as two offsets and needs no allocation for — before copying anything.
//!
//! # The three things this file asserts, because two of them can hide
//!
//! 1. **THE FIGURE.** `खण्डपाठः` is called far fewer times than there are word
//!    tokens. Before the reorder the two counts were EQUAL; a reorder that
//!    silently stopped gating would show up here and nowhere else.
//! 2. **THE WIDTHS ARE PINNED.** `१८` and `१२` are literals in `lex.t1` and a
//!    respelling of either layout word would turn the arm off without failing
//!    anything. Asserted against the words themselves.
//! 3. **THE CASE THAT MUST STILL BE REFUSED.** `उक्तम् विवरम् इति` is a
//!    STRING whose content is the word, not a layout word — the 2026-09-14
//!    rung-level defect this arm exists to avoid (the compiled lexer emitted
//!    `०` where the interpreter emits `३२`). A length gate that skipped the
//!    `उक्तम्` lookback would classify it `शब्दभेद` again, and the figure above
//!    would still look right.
//!
//! The loader is this file's own, per the convention: a census keyed to a
//! shared loader's counts is not this census.

use sadhana::t1::nirvahana::{Interpreter, Octets, Value};
use std::collections::HashMap;
use std::path::Path;

/// `पदभेद` — Kind::Word.
const K_WORD: i128 = 1;
/// `शब्दभेद` — Kind::Str, what a bare layout word becomes.
const K_STR: i128 = 3;

const FUEL_LEX: u64 = 2_000_000_000;

/// Steps per word token when lexing `parse.t1` BEFORE fix (3) — every word paid
/// a `खण्डपाठः` allocation and copy, and every word but the first a
/// `चिह्नकपाठः` on its predecessor. MEASURED on this tree by running the same
/// text through `HEAD`'s `lex.t1` and this one: 4,616 words both times,
/// 13,183,077 steps then and 10,933,599 now.
const STEPS_PER_WORD_BEFORE: u64 = 2_855;
/// The ceiling fix (3) has to hold — above the 2,368 measured and well below
/// the 2,855 that preceded it, so an unrelated change to the lexer has room and
/// losing the length pre-test does not.
const STEPS_PER_WORD_CEILING: u64 = 2_500;

fn source(name: &str) -> String {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("{} is readable: {e}", p.display()))
}

/// `lex.t1` imports nothing (its own header says so and there is no `आयातः` in
/// it), so the lexer alone is the whole chain this file needs.
fn load_lexer() -> Interpreter {
    let text = source("lex.t1");
    Interpreter::load(&[("lex.t1", text.as_str())], &repo_spec())
        .unwrap_or_else(|e| panic!("lex.t1 load: {e:?}"))
}

fn repo_spec() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// `(kind, text)` for every token the arena holds, index for index from ०;
/// slot ० is never written and reports as `None`.
fn tokens(it: &Interpreter) -> Vec<Option<(i128, String)>> {
    match it.global("चिह्नककोश") {
        Some(Value::Arena(a)) => a
            .borrow()
            .iter()
            .map(|v| match v {
                Value::Record(r) => {
                    let r: HashMap<String, Value> = r.borrow().clone();
                    let kind = r.get("भेद").and_then(Value::as_int).unwrap_or(0);
                    let text = match r.get("पाठ") {
                        Some(Value::Octets(o)) => {
                            let lo = r.get("अष्टक").and_then(Value::as_int).unwrap_or(0) as usize;
                            let hi = r.get("पाठसीमा").and_then(Value::as_int).unwrap_or(0) as usize;
                            let s = o.as_slice();
                            if lo <= hi && hi <= s.len() {
                                String::from_utf8_lossy(&s[lo..hi]).into_owned()
                            } else {
                                String::new()
                            }
                        }
                        _ => String::new(),
                    };
                    Some((kind, text))
                }
                _ => None,
            })
            .collect(),
        other => panic!("global `चिह्नककोश` is {other:?}, not an arena — THIS CENSUS is broken"),
    }
}

fn lex(it: &mut Interpreter, src: &str) -> Vec<Option<(i128, String)>> {
    it.call(
        "पदविभागॱपदविभाग",
        vec![Value::Octets(Octets::new(src.as_bytes()))],
        FUEL_LEX,
    )
    .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"));
    tokens(it)
}

/// Lex `src` and answer `(tokens, steps)` — steps being what the budget lost,
/// which is the only step reading available without `T1_CALLS` (whose
/// `std::env::set_var` this crate's `-D unsafe-code` forbids a test to make).
fn lex_counting(it: &mut Interpreter, src: &str) -> (Vec<Option<(i128, String)>>, u64) {
    let before = FUEL_LEX;
    it.call(
        "पदविभागॱपदविभाग",
        vec![Value::Octets(Octets::new(src.as_bytes()))],
        before,
    )
    .unwrap_or_else(|e| panic!("पदविभाग runs: {e:?}"));
    let spent = before.saturating_sub(it.fuel_remaining());
    (tokens(it), spent)
}

/// The kind the lexer gave the token whose text is `word`, in a source that
/// holds it exactly once. Panics rather than defaulting: a missing token is a
/// broken fixture, not a classification.
fn kind_of(toks: &[Option<(i128, String)>], word: &str) -> i128 {
    let hits: Vec<i128> = toks
        .iter()
        .flatten()
        .filter(|(_, t)| t == word)
        .map(|(k, _)| *k)
        .collect();
    assert_eq!(
        hits.len(),
        1,
        "the fixture must hold `{word}` exactly once; it lexed to {} tokens",
        hits.len()
    );
    hits[0]
}

/// SAS-011 fix (3): the word arm asks the LENGTH before it copies, so
/// `खण्डपाठः` runs on the few words that could be a layout word and not on all
/// of them — and the classification, including the one case it must refuse, is
/// unchanged.
#[test]
fn the_word_arm_copies_only_a_word_that_could_be_a_layout_word() {
    // (2) THE WIDTHS, PINNED. `lex.t1` writes १८ and १२ as literals.
    assert_eq!("विवरम्".len(), 18, "ADR-0018's SPACE is 18 octets");
    assert_eq!("यतिः".len(), 12, "ADR-0018's LINE FEED is 12 octets");

    let mut it = load_lexer();

    // (1) THE FIGURE, over a real source. `parse.t1` AND NOT `lex.t1`: the
    // figure is a before/after on one text, and the text must not be the file
    // the change edits — lexing `lex.t1` made the word count move by 52 between
    // the two readings purely because the source had grown.
    let src = source("parse.t1");
    let (toks, steps) = lex_counting(&mut it, &src);
    let words = toks.iter().flatten().filter(|(k, _)| *k == K_WORD).count();
    assert!(
        words > 1_500,
        "parse.t1 must lex to over 1,500 words for the ratio to mean anything; got {words}"
    );
    let per_word = steps / words as u64;
    println!("parse.t1: {words} words, {steps} steps, {per_word} steps per word");
    assert!(
        per_word <= STEPS_PER_WORD_CEILING,
        "the lexer must spend at most {STEPS_PER_WORD_CEILING} steps per word token: \
         {steps} steps over {words} words is {per_word}. Before SAS-011 fix (3) it was \
         {STEPS_PER_WORD_BEFORE} — every word paid `खण्डपाठः`, an allocation and an \
         octet-by-octet copy, so a reading back at that level means the length gate is gone"
    );

    // (3) CLASSIFICATION, unchanged — and the case that must still be refused.
    //
    // One fixture, five words: the two layout words bare, the same space word
    // inside `उक्तम् … इति`, and two decoys of exactly the gated widths.

    let fixture = "मण्डलम् क ॥\n\
                   सार्वजनिक वृत्तिः ग ददाति न६४ आदि\n\
                   चरः अ ॱॱ अङ्कः अन्तः अ८ भवति विवरम् ।\n\
                   चरः ब ॱॱ अङ्कः अन्तः अ८ भवति यतिः ।\n\
                   चरः स ॱॱ अङ्कः अन्तः अ८ भवति उक्तम् विवरम् इति ।\n\
                   चरः द ॱॱ न६४ भवति कमलानि ।\n\
                   चरः फ ॱॱ न६४ भवति कमले ।\n\
                   प्रत्यागमनम् ० ।\n\
                   इति\n";
    assert_eq!("कमलानि".len(), 18, "the 18-octet decoy is 18 octets");
    assert_eq!("कमले".len(), 12, "the 12-octet decoy is 12 octets");

    let f = lex(&mut it, fixture);
    // `विवरम्` appears TWICE in the fixture — bare and quoted — so it is read
    // positionally rather than by text: the token after `उक्तम्` is the quoted
    // one and the other is bare.
    let flat: Vec<(i128, String)> = f.iter().flatten().cloned().collect();
    let quoted = flat
        .windows(2)
        .find(|w| w[0].1 == "उक्तम्")
        .map(|w| w[1].clone())
        .expect("the fixture holds `उक्तम् विवरम् इति`");
    assert_eq!(
        quoted.1, "विवरम्",
        "the token after `उक्तम्` is the quoted layout word"
    );
    assert_eq!(
        quoted.0, K_WORD,
        "`उक्तम् विवरम् इति` is a STRING whose content is the WORD — the quoted \
         word must stay पदभेद. Classifying it शब्दभेद is the 2026-09-14 defect: \
         ir.t1 folds it to one octet and the compiled lexer emits ० where the \
         interpreter emits ३२"
    );
    let bare: Vec<i128> = flat
        .iter()
        .enumerate()
        .filter(|(i, (_, t))| t == "विवरम्" && !(*i > 0 && flat[i - 1].1 == "उक्तम्"))
        .map(|(_, (k, _))| *k)
        .collect();
    assert_eq!(bare, vec![K_STR], "a BARE `विवरम्` is शब्दभेद");
    assert_eq!(kind_of(&f, "यतिः"), K_STR, "a BARE `यतिः` is शब्दभेद");
    assert_eq!(
        kind_of(&f, "कमलानि"),
        K_WORD,
        "an 18-octet word that is not `विवरम्` is an ordinary word — the gate is \
         a length PRE-test, not the classification"
    );
    assert_eq!(
        kind_of(&f, "कमले"),
        K_WORD,
        "a 12-octet word that is not `यतिः` is an ordinary word"
    );
}
