//! ADR-0019, task `D-002g` — **a T1 file reader**.
//!
//! The row's acceptance is *"one `.t1` module reads
//! `spec/mnemonics-riscv64.src.tsv`"* and a derived count *"asserted against
//! the Rust `encodings().len()`"*. Both are here.
//!
//! **Nothing in this file transcribes a table.** `B-058a` found the defect
//! class where a typed second copy agrees with the first by construction and
//! checks nothing; every number below is derived, on one side by Rust and on
//! the other by walking the embedded octets the way `crates/sadhana-t1/src/
//! encode.t1`'s own row parsers do — and the two derivations are compared.

use sadhana::encode::encodings;
use sadhana::lex::{Kind, Token, lex, lex_t1};
use sadhana::t1::anita::{self, EmbedError};
use std::path::{Path, PathBuf};

/// The spec directory, from the crate root rather than the process CWD.
fn spec_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// Lex T1 source and resolve its embeds — the whole front half of the pipeline.
fn pipeline(src: &str) -> Result<Vec<Token>, Vec<EmbedError>> {
    let tokens = lex_t1(src).expect("source must lex");
    anita::resolve(tokens, &spec_root())
}

/// The one `Kind::Str` value in a resolved stream.
fn only_string(tokens: &[Token]) -> &str {
    let mut found = None;
    for t in tokens {
        if let Kind::Str { value } = &t.kind {
            assert!(found.is_none(), "more than one string in the stream");
            found = Some(value.as_str());
        }
    }
    found.expect("no string in the stream")
}

/// Count the data rows of a derived table **in T1's own manner**.
///
/// Deliberately the shape `crates/sadhana-t1/src/encode.t1` already has, and
/// that `encode.t1:427` already explains: *"the separator octets are written as
/// numerals because T1 has no character literal either: TAB 9, LF 10, CR 13,
/// SPACE 32, `#` 35"*. It uses no `lines()`, no `split` and no `str` method T1
/// lacks — only indexing octets and comparing them to numbers, which is exactly
/// what `अष्टकान्वेषणम्` does. The four clauses mirror
/// `sadhana::encode::encodings()` one for one.
fn derive_rows(octets: &[u8]) -> usize {
    const HASH: u8 = 35;
    const TAB: u8 = 9;
    const LF: u8 = 10;

    let mut rows = 0usize;
    let mut start = 0usize;
    while start <= octets.len() {
        let mut end = start;
        while end < octets.len() && octets[end] != LF {
            end += 1;
        }
        let line = &octets[start..end];
        start = end + 1;

        if line.is_empty() || line[0] == HASH {
            continue;
        }
        // The header row: `insn` then a tab, compared as octets, since T1 has
        // no `starts_with` over text either.
        if line.len() > 4 && &line[..4] == b"insn" && line[4] == TAB {
            continue;
        }
        let fields = 1 + line.iter().filter(|&&c| c == TAB).count();
        if fields < 7 {
            continue;
        }
        rows += 1;
    }
    rows
}

// ─────────────────────────── the end-to-end proof ───────────────────────────

/// A `.t1` source names a spec file and gets its bytes. This is the row's
/// stated acceptance and the claim the whole ADR rests on.
///
/// The right-hand side is `include_str!` — the exact facility ADR-0019 says
/// this is the T1 equivalent of, and the one `crates/sadhana/src/parse.rs:25`
/// already applies to this very file. If the two ever disagree, the T1 embed is
/// not what Rust means by an embed.
#[test]
fn a_t1_source_names_a_spec_file_and_gets_its_bytes() {
    let tokens = pipeline("समावेशः आरभ्य नामकोशः समाप्तम्").expect("the embed must resolve");
    let got = only_string(&tokens);

    const RUST_SEES: &str = include_str!("../../../spec/mnemonics-riscv64.src.tsv");
    assert_eq!(
        got, RUST_SEES,
        "the T1 embed and Rust's include_str! disagree about the same file"
    );

    // Not vacuous on either side: the file is real, and it is the file named.
    assert!(
        got.len() > 4000,
        "the mnemonic registry is {} bytes, which is too small to be it",
        got.len()
    );
    assert!(
        got.contains("योगः"),
        "the embedded text does not contain the registry's first family"
    );
}

/// The acceptance number. A table is derived from the embedded octets **in
/// T1's own manner** and compared with the count Rust derives from the same
/// file — the assertion `D-002g` names, and the one
/// `encoder_table_readers_real` will make once the follow-up row wires the four
/// bodies.
///
/// The walk below is deliberately the shape `encode.t1` already has and
/// `encode.t1:427` already explains: *"the separator octets are written as
/// numerals because T1 has no character literal either: TAB 9, LF 10, CR 13,
/// SPACE 32, `#` 35"*. It uses no `lines()`, no `split('\t')` and no `str`
/// method that T1 lacks — only indexing octets and comparing them to numbers,
/// which is exactly what `अष्टकान्वेषणम्` does.
#[test]
fn the_embedded_table_derives_the_same_row_count_rust_derives() {
    let tokens = pipeline("समावेशः आरभ्य सङ्केतकोशः समाप्तम्").expect("the embed must resolve");
    let rows = derive_rows(only_string(&tokens).as_bytes());

    assert_eq!(
        rows,
        encodings().len(),
        "the table derived from the T1 embed has {rows} rows and Rust's \
         encodings() has {}; one of the two readers is wrong, and neither is a \
         transcription of the other",
        encodings().len()
    );
    assert!(rows > 100, "derived {rows} rows, which cannot be the ISA");
}

/// The two clauses of [`derive_rows`] that today's spec file does not exercise.
///
/// Mutation testing found them: dropping the `#` skip, or accepting six-field
/// rows, changes no count against `spec/encodings-riscv64.tsv`, because that
/// file has **no commented line with seven fields and no six-field row** — and
/// dropping the same two guards from Rust's `encodings()` would not change its
/// count either, for the same reason. They are kept so the two derivations are
/// the SAME algorithm rather than two that happen to agree on one file, and
/// this test is what holds them: a table built to have both shapes.
#[test]
fn the_walk_rejects_a_commented_row_and_a_short_row() {
    let good = "a\tb\tc\td\te\tf\tg";
    let commented = format!("#{good}");
    let short = "a\tb\tc\td\te\tf";

    assert_eq!(derive_rows(good.as_bytes()), 1, "a seven-field row counts");
    assert_eq!(
        derive_rows(commented.as_bytes()),
        0,
        "a commented row with seven fields was counted"
    );
    assert_eq!(
        derive_rows(short.as_bytes()),
        0,
        "a six-field row was counted"
    );
    assert_eq!(
        derive_rows(format!("insn\t{good}").as_bytes()),
        0,
        "the header row was counted"
    );
    assert_eq!(
        derive_rows(format!("{good}\n{commented}\n{short}\n{good}").as_bytes()),
        2,
        "the four shapes together do not sum"
    );
}

/// Every name in the table resolves to a file that is actually there. Without
/// this, four of the five entries are an unchecked promise to the follow-up
/// row.
#[test]
fn every_table_name_resolves_to_a_file_that_exists() {
    let names = anita::table_names();
    // 8 -> 10 on 2026-08-30. This assertion exists so a table cannot be added
    // unreviewed, so here is the review: `शिवसूत्रकोशः` -> shiva-sutras.tsv
    // unblocks `अक्षरकोश ॱ व्यञ्जन` (HAL is derived from that file and the 33
    // consonants are not a contiguous range, so nothing arithmetic answers
    // it); `निदानकोशः` -> diagnostics.tsv unblocks
    // `सङ्केतन ॱ सङ्केतनदोषवचनम्`, whose row walk was already written and had
    // nothing to walk. Both files already existed in `spec/`; only the NAME
    // was missing, which is now the fifth and sixth time that has blocked a
    // routine — see the margin in `anita.rs`.
    // 11 -> 13 on 2026-08-30. The review: `संयोगभेदकोशः` -> incb.tsv and
    // `चित्राक्षरकोशः` -> extended-pictographic.tsv, the two UAX #29 tables
    // `अक्षराणि` still lacked after grapheme-break.tsv cleared one third of
    // its blocker. Both generated from the PINNED UCD by tools/gen-incb.py and
    // asserted identical to it code point by code point.
    assert_eq!(names.len(), 13, "the reviewed surface changed size");

    // `भेदाङ्कः` ENCODES "NOT FOUND" AS 0, WHICH IS ONLY SOUND WHILE 0 IS ABSENT.
    //
    // T1 has no `Option`, so `samyojana.t1`'s port of `reloc_number` returns 0 for
    // a name it does not find. The Rust original returns `Option<u32>` and can
    // distinguish. That collapse is safe TODAY because the table's numbers start
    // at 1 — `R_RISCV_NONE` (0) is not listed, and the string "none" appears
    // nowhere in the file.
    //
    // It is an INVARIANT OF THE DATA, not of the code, so nothing in the port
    // would notice if it stopped holding: adding a row numbered 0 would make
    // `भेदाङ्कः` report "not found" for a real relocation, silently and forever.
    // This asserts the invariant where it can be seen.
    {
        let table = std::fs::read_to_string(spec_root().join("relocations-riscv64.tsv"))
            .expect("relocations-riscv64.tsv is readable");
        for line in table.lines().skip(1) {
            if line.starts_with('#') || line.trim().is_empty() {
                continue;
            }
            let number = line.split('\t').nth(1).unwrap_or("").trim();
            assert_ne!(
                number, "0",
                "a relocation numbered 0 was added: `भेदाङ्कः` in samyojana.t1 \
                 encodes not-found as 0 and would now hide this row. Give T1 a \
                 real optional, or renumber."
            );
        }
    }
    for name in names {
        let src = format!("समावेशः आरभ्य {name} समाप्तम्");
        let tokens = pipeline(&src).unwrap_or_else(|e| panic!("`{name}` did not resolve: {e:?}"));
        assert!(
            !only_string(&tokens).is_empty(),
            "`{name}` resolved to an empty file"
        );
    }
}

// ───────────────── the content is a string, and that is the point ────────────

/// ADR-0019's rule 3, and the third application of ADR-0017's reason: an embed
/// carries the string token's kind, so every reader of a string reads one with
/// no second rule.
///
/// The test states it the way the claim is actually made — one matcher that
/// knows only `Kind::Str` picks up all three of a literal, a layout word and an
/// embed. A fourth kind would fail this and would have restarted the count
/// ADR-0017 closed.
#[test]
fn an_embed_carries_the_string_kind_that_a_literal_and_a_layout_word_carry() {
    let tokens =
        pipeline("उक्तम् क इति यतिः समावेशः आरभ्य नामकोशः समाप्तम्").expect("the embed must resolve");

    let strings: Vec<&String> = tokens
        .iter()
        .filter_map(|t| match &t.kind {
            Kind::Str { value } => Some(value),
            _ => None,
        })
        .collect();

    assert_eq!(
        strings.len(),
        3,
        "one matcher that knows only Kind::Str must see the literal, the \
         layout word and the embed; it saw {}",
        strings.len()
    );
    assert_eq!(strings[0], "क", "the literal");
    assert_eq!(strings[1], "\n", "ADR-0018's यतिः");
    assert!(strings[2].contains("योगः"), "the embed");
}

/// ADR-0017 established that `Token::text` carries the source and the value
/// carries the meaning, so a diagnostic can quote what was typed. An embed
/// keeps that split rather than losing the phrase it came from.
#[test]
fn the_embed_token_keeps_the_source_phrase_it_came_from() {
    let tokens = pipeline("समावेशः आरभ्य नामकोशः समाप्तम्").expect("must resolve");
    let t = tokens
        .iter()
        .find(|t| matches!(t.kind, Kind::Str { .. }))
        .unwrap();
    assert_eq!(t.text, "समावेशः आरभ्य नामकोशः समाप्तम्");
    assert_eq!(t.line, 1);
}

// ───────────── a name, not a path — the repertoire is not weakened ──────────

/// The reason ADR-0019 resolves a NAME. A path is Latin, and the lexer refuses
/// Latin — over a bare word and, since ADR-0017, over the whole of a string
/// literal too. This test is the evidence that the repertoire was not widened
/// to make the row work: it is still refusing exactly what it refused before.
#[test]
fn a_path_cannot_be_written_because_the_repertoire_still_refuses_it() {
    let bare = lex_t1("समावेशः आरभ्य spec/mnemonics-riscv64.src.tsv समाप्तम्");
    let errs = bare.expect_err("a Latin path must not lex");
    assert!(
        errs.iter().any(|e| e.reason.contains("repertoire")),
        "a Latin path was refused for some reason other than the repertoire: {errs:?}"
    );

    // And inside a string literal, which is where a path syntax would have put
    // it. `lex.rs` records that a string "is not a hole in R-15-1"; this holds
    // it to that.
    let quoted = lex_t1("समावेशः आरभ्य उक्तम् spec/x.tsv इति समाप्तम्");
    let errs = quoted.expect_err("a Latin path inside a string must not lex either");
    assert!(
        errs.iter().any(|e| e.reason.contains("repertoire")),
        "a string is a hole in the repertoire: {errs:?}"
    );
}

/// Path traversal is not refused — it is unspellable. There is no path, so
/// there is nowhere to put a `..`. The nearest thing a program CAN write is an
/// unknown name, and that is a refusal.
#[test]
fn a_traversal_is_unspellable_rather_than_refused() {
    // Written entirely in the repertoire, so it lexes; it simply names nothing.
    let e = pipeline("समावेशः आरभ्य पूर्वम् समाप्तम्").expect_err("must be refused");
    assert_eq!(e.len(), 1);
    assert!(
        e[0].reason
            .contains("is not a table this program may bring in"),
        "{}",
        e[0].reason
    );
    // The refusal is the user's whole map of the surface.
    for name in anita::table_names() {
        assert!(
            e[0].reason.contains(name),
            "the refusal does not name `{name}`, so it is not a map of what exists"
        );
    }
}

// ────────────────────────── malformed embeds ────────────────────────────────

#[test]
fn an_embed_without_its_group_open_is_refused() {
    let e = pipeline("समावेशः नामकोशः समाप्तम्").expect_err("must be refused");
    assert!(
        e[0].reason.contains("must be followed by `आरभ्य`"),
        "{}",
        e[0].reason
    );
}

#[test]
fn an_embed_without_its_group_close_is_refused() {
    let e = pipeline("समावेशः आरभ्य नामकोशः ।").expect_err("must be refused");
    assert!(e[0].reason.contains("must be closed by"), "{}", e[0].reason);
}

#[test]
fn an_embed_that_names_nothing_is_refused() {
    let e = pipeline("समावेशः आरभ्य समाप्तम्").expect_err("must be refused");
    assert!(e[0].reason.contains("names no table"), "{}", e[0].reason);
}

/// One bad embed must not hide the next: `lex` returns every error it found and
/// so does this. A pass that stopped at the first would make fixing a file an
/// n-round game.
#[test]
fn two_bad_embeds_produce_two_diagnostics() {
    let e = pipeline("समावेशः आरभ्य कखग समाप्तम् समावेशः आरभ्य घङच समाप्तम्").expect_err("must be refused");
    assert_eq!(e.len(), 2, "got {e:?}");
    assert!(e[0].reason.contains("कखग"));
    assert!(e[1].reason.contains("घङच"));
}

// ───────────────────────── what must NOT change ─────────────────────────────

/// ADR-0018's rule 2 applied to this construct: inside `उक्तम् … इति`, `समावेशः`
/// is its own akṣaras of text and nothing is brought in. There is no lead-in
/// character, no lexer state, and no position at which one spelling means two
/// things — which is what ADR-0011 refused when it killed `॰` as an escape.
///
/// This matters more now than it did to the draft. The word is shared with T0's
/// `.include` directive, so *"one spelling, two meanings"* is already true of it
/// ACROSS the tiers; what must stay false is two meanings WITHIN one tier at one
/// position. A string literal is that position, and this is what holds it.
#[test]
fn an_embed_word_inside_a_string_is_its_own_letters() {
    let tokens = pipeline("उक्तम् समावेशः आरभ्य नामकोशः समाप्तम् इति").expect("must resolve");
    assert_eq!(
        only_string(&tokens),
        "समावेशः आरभ्य नामकोशः समाप्तम्",
        "a string literal was decoded as an embed"
    );
}

/// `resolve` is public and takes any `Vec<Token>`, not only one `lex_t1` built,
/// so the rule *"a string's value is data and is never re-read as source"* has
/// to hold for a stream assembled by hand too.
///
/// Through the lexer this is unreachable — a `Kind::Str` token's text is either
/// the whole `उक्तम् … इति` literal or a layout word, never a bare `समावेशः` —
/// which mutation testing confirmed by finding the guard unkillable from
/// source. This test reaches it the way a caller could: directly. Without the
/// guard, a program could smuggle an embed through a value.
#[test]
fn a_string_token_is_never_re_read_as_an_embed_even_when_handed_in_directly() {
    let str_tok = Token {
        kind: Kind::Str {
            value: anita::EMBED_WORD.to_string(),
        },
        text: anita::EMBED_WORD.to_string(),
        byte: 0,
        aksara: 0,
        line: 1,
    };
    let mut tokens = vec![str_tok];
    tokens.extend(lex_t1("आरभ्य नामकोशः समाप्तम्").expect("must lex"));

    let out = anita::resolve(tokens, &spec_root()).expect("must resolve to nothing");
    assert_eq!(
        out.len(),
        4,
        "a Kind::Str whose text is the embed word was re-read as source"
    );
    let Kind::Str { value } = &out[0].kind else {
        panic!("the string token did not survive")
    };
    assert_eq!(value, anita::EMBED_WORD, "the value was replaced by a file");
}

/// Content is data, not source. A `समावेशः` that arrives inside an embedded
/// file is not a nested embed. ADR-0019 states this so it is a decision rather
/// than an oversight; this holds it.
#[test]
fn embedded_content_is_not_rescanned_for_a_nested_embed() {
    let tokens = pipeline("समावेशः आरभ्य नामकोशः समाप्तम्").expect("must resolve");
    assert_eq!(
        tokens.len(),
        1,
        "resolving one embed produced {} tokens; content was rescanned",
        tokens.len()
    );
}

/// Everything that is not an embed comes through byte-identical. A pass that
/// rewrote anything else would be changing the language, not extending it.
#[test]
fn tokens_outside_an_embed_are_passed_through_untouched() {
    let src = "वृत्तिः क आदि प्रत्यागमनम् १२ । इति ॥";
    let before = lex_t1(src).expect("must lex");
    let after = anita::resolve(before.clone(), &spec_root()).expect("no embeds to resolve");
    assert_eq!(before, after, "a stream with no embed was altered");
}

/// T0 is unchanged, to the byte — the same claim ADR-0017 and ADR-0018 each
/// make and each guard. `lex` never sees an embed, so no `.sas` file changes
/// meaning, and `spec/grammar-t0.ebnf` stays frozen.
///
/// **This test carries far more weight than it did before the owner's ruling.**
/// While the embed word was `आनीतम्`, a word T0 had no use for, this only said
/// that `lex` does not run a pass it was never given. Now the word is `समावेशः`,
/// which T0 ALSO has — as the `.include` directive of `spec/directives.tsv:36` —
/// and so this is the mechanical form of the distinction ADR-0019 draws: the
/// SAME source text means a directive to one tier and an embed to the other, and
/// the tier is what decides. The `॥ … ॥` around it is the T0 directive wrapper
/// (ADR-0012), which is the second discriminator, present here on purpose.
#[test]
fn t0_never_produces_an_embed() {
    // The `उक्तम् … इति` is here on purpose: without it this test would pass
    // even if `lex` were flipped to take the string token, and would then be
    // claiming more than it checks.
    let src = "॥ अष्टकाः उक्तम् क इति समावेशः आरभ्य नामकोशः समाप्तम् ॥";
    let t0 = lex(src).expect("T0 must lex this as ordinary words");
    assert!(
        !t0.iter().any(|t| matches!(t.kind, Kind::Str { .. })),
        "T0 produced a string token"
    );
    assert!(
        t0.iter().any(|t| t.text == "समावेशः"),
        "T0 must still see the word itself"
    );
    // ADR-0017's claim that `lex` is untouched to the byte, restated here so
    // that this suite alone notices if T0 is moved onto the string token.
    // Each stays a token of its own. Were `lex` moved onto the string token,
    // the three would fuse into one whose text is the whole literal, and every
    // one of these would fail. (The kind is deliberately not asserted:
    // `उक्तम्` ends in `म्`, so ADR-0004 makes it an Operand, not a Word.)
    for w in ["उक्तम्", "क", "इति"] {
        assert!(
            t0.iter().any(|t| t.text == w),
            "T0 must still read `{w}` as a token of its own"
        );
    }
}

/// **The owner's ruling of 2026-08-28, pinned in both directions.**
///
/// This test is the descendant of `samavesha_is_not_the_embed_word_and_brings_
/// nothing_in`, and it now asserts the opposite of what that one did. The
/// history is the point, so it is written down rather than quietly dropped.
///
/// The DRAFT of ADR-0019 refused `समावेशः` for two reasons — it is `.include`,
/// *textual inclusion*, in `spec/directives.tsv:36`, and it was a function name
/// in `crates/sadhana-t1/src/encode.t1` (`Slot::fits`) — and proposed `आनीतम्`.
/// The owner refused the new word. The occupancy was a **defect**, not a claim
/// on the word: `समावेशः` is in the FROZEN `keyword` production, which says a
/// keyword *"is also a possible identifier — the reader must prefer the
/// keyword"*, so that declaration was already illegal and merely unenforced.
/// `B-112` renamed it to `अन्तर्भावः` and added
/// `no_t1_source_binds_a_frozen_keyword_as_a_name` so the next one fails loudly.
///
/// It is kept as a test rather than deleted because the reason it existed has
/// not gone away: a later worker reaching for either word should meet a failing
/// test and an ADR, not a silent change.
#[test]
fn samavesha_is_the_embed_word_and_anitam_is_not_a_word_of_this_language() {
    assert_eq!(
        anita::EMBED_WORD,
        "समावेशः",
        "ADR-0019 as ratified opens an embed with the keyword the language \
         already had, and adds none"
    );

    // And the word the draft proposed brings nothing in. It is an ordinary
    // identifier now — four words that resolve to no file and pass through.
    let tokens = pipeline("आनीतम् आरभ्य नामकोशः समाप्तम्").expect("must resolve to nothing");
    assert!(
        !tokens.iter().any(|t| matches!(t.kind, Kind::Str { .. })),
        "`आनीतम्` brought a file in; the owner refused it and ADR-0019 as \
         ratified adds no keyword at all"
    );
    assert_eq!(
        tokens.len(),
        4,
        "the four words must pass through untouched, not be consumed"
    );
}

/// **T0's `.include` row is not edited, reclassified or deleted.**
///
/// ADR-0019 rule 5 says T0 is unchanged to the byte. The one way this ruling
/// could have quietly broken that is from the other end — retiring the directive
/// so that the word would have a single sense again and the ADR's distinction
/// would have nothing to distinguish. It was not retired, and this says so.
#[test]
fn the_t0_include_directive_survives_the_t1_embed_unchanged() {
    let tsv = std::fs::read_to_string(spec_root().join("directives.tsv"))
        .expect("spec/directives.tsv exists");
    let row = tsv
        .lines()
        .find(|l| l.split('\t').next() == Some("समावेशः"))
        .expect("spec/directives.tsv still carries समावेशः");

    assert!(row.contains(".include"), "{row}");
    assert!(row.contains("textual inclusion"), "{row}");
    assert!(
        row.contains("planned"),
        "the .include directive changed status; ADR-0019 neither implements nor \
         reserves it: {row}"
    );

    // The T1 sense of the same word, asserted in the same breath, so that the
    // two senses are held to coexist rather than assumed to.
    assert_eq!(anita::EMBED_WORD, "समावेशः");
}

/// The name table is a map, not a set of aliases: two names must not denote one
/// file, and no name may be blank.
#[test]
fn the_reviewed_surface_has_no_duplicate_name_or_file() {
    let names = anita::table_names();
    let mut files: Vec<&str> = names
        .iter()
        .map(|n| anita::table_path(n).unwrap())
        .collect();
    let mut sorted = names.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), names.len(), "a name is declared twice");
    files.sort_unstable();
    files.dedup();
    assert_eq!(files.len(), names.len(), "two names denote one file");
    assert!(anita::table_path("").is_none(), "the empty name resolves");
}
