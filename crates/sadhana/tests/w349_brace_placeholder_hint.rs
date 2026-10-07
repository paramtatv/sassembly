//! W-349 — a stray ASCII brace or bracket is, in practice, an unsubstituted
//! template placeholder, and the repertoire diagnostic should say so.
//!
//! The lexer's message was *"`{` is outside the doc 15 repertoire (7 more in this
//! word)"*: true, and not the diagnosis. The reporter's cause was a generator's
//! f-string split during an edit, and it cost a 268 s build to locate.
//!
//! WHAT MUST NOT CHANGE, and each is asserted here rather than assumed:
//!   * the word is still REFUSED — the hint is added, the refusal is kept
//!     (`a-margin-mark-one-glyph-off-is-live-code`: the repertoire check is
//!     load-bearing);
//!   * the sentence `outside the doc 15 repertoire` survives verbatim, because
//!     six test files filter on it;
//!   * the count stays the LAST clause, `(N more in this word)`, and the hint
//!     carries no ` (` of its own, because `sas_reachability.rs` recovers `N` by
//!     `split_once(" (")` then `strip_suffix(" more in this word)")`;
//!   * a violation that is NOT a brace or bracket reads exactly as before.

use sadhana::lex::{LexError, lex, lex_t1};

const SENTENCE: &str = "outside the doc 15 repertoire";
const HINT: &str = "placeholder";

fn errors(src: &str, t1: bool) -> Vec<LexError> {
    let r = if t1 { lex_t1(src) } else { lex(src) };
    r.err()
        .unwrap_or_default()
        .into_iter()
        .filter(|e| e.reason.contains(SENTENCE))
        .collect()
}

/// The same recovery `sas_reachability.rs::aksaras_in` performs.
fn count_from(e: &LexError) -> usize {
    let Some(rest) = e.reason.split_once(" (") else {
        return 1;
    };
    let Some(n) = rest.1.strip_suffix(" more in this word)") else {
        return 1;
    };
    1 + n.parse::<usize>().unwrap_or(0)
}

#[test]
fn a_brace_placeholder_is_refused_and_named_as_generator_output() {
    // The reporter's shape: an f-string placeholder that reached the file.
    for t1 in [false, true] {
        let es = errors("चरः {name} भवति ० ।\n", t1);
        assert_eq!(
            es.len(),
            1,
            "one diagnostic per offending word (t1 = {t1}): {es:?}"
        );
        let e = &es[0];
        assert_eq!(
            e.aksara, "{",
            "the first offending character is still the one named"
        );
        assert!(
            e.reason.contains(HINT),
            "the diagnostic does not say the brace is likely an unsubstituted \
             placeholder (t1 = {t1}): {}",
            e.reason
        );
        // `{name}` is six ASCII characters, all outside the repertoire.
        assert_eq!(
            count_from(e),
            6,
            "the count is no longer recoverable the way sas_reachability.rs \
             recovers it: {}",
            e.reason
        );
    }
}

#[test]
fn each_brace_and_bracket_draws_the_hint_alone_and_with_no_count() {
    for ch in ['{', '}', '[', ']'] {
        let es = errors(&format!("चरः {ch} भवति ० ।\n"), true);
        assert_eq!(es.len(), 1, "`{ch}`: {es:?}");
        assert!(
            es[0].reason.contains(HINT),
            "`{ch}` drew no hint: {}",
            es[0].reason
        );
        assert!(
            !es[0].reason.contains("more in this word"),
            "`{ch}` alone has nothing more in its word: {}",
            es[0].reason
        );
        assert_eq!(count_from(&es[0]), 1);
    }
}

#[test]
fn a_brace_later_in_the_word_still_draws_the_hint() {
    // `x{0}`: the first violation is the letter, the cause is still the brace.
    let es = errors("चरः x{0} भवति ० ।\n", true);
    assert_eq!(es.len(), 1, "{es:?}");
    assert_eq!(es[0].aksara, "x");
    assert!(es[0].reason.contains(HINT), "{}", es[0].reason);
}

#[test]
fn a_violation_with_no_brace_reads_exactly_as_it_did() {
    // CONTROL. The hint must not attach to every repertoire violation: a Latin
    // letter is the near-miss glyph case, and its message is unchanged.
    let es = errors("चरः R भवति ० ।\n", true);
    assert_eq!(es.len(), 1, "{es:?}");
    assert_eq!(es[0].reason, "`R` is outside the doc 15 repertoire");
    let es = errors("चरः abc भवति ० ।\n", true);
    assert_eq!(es.len(), 1, "{es:?}");
    assert_eq!(
        es[0].reason,
        "`a` is outside the doc 15 repertoire (2 more in this word)"
    );
}
